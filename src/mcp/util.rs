//! Helpers shared by every MCP tool module: running blocking API calls, turning
//! results into MCP tool results, raw JSON API access, name resolution, dates
//! and priorities. Behavior mirrors the Python server's `client.py`,
//! `resolve.py` and `dates.py` so tool output and errors match it.

use std::sync::Mutex;

use rmcp::ErrorData;
use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::Value;

use crate::api;
use crate::config;

/// What every tool returns. API and validation failures are tool results with
/// `isError: true` (never protocol errors), so the model sees the message.
pub type ToolResult = Result<CallToolResult, ErrorData>;

/// A tool's successful output: JSON data, or a plain confirmation message.
pub enum Output {
    Json(Value),
    Text(String),
}

impl From<Value> for Output {
    fn from(v: Value) -> Self {
        Output::Json(v)
    }
}

impl From<String> for Output {
    fn from(s: String) -> Self {
        Output::Text(s)
    }
}

/// Run a blocking (ureq) tool body on the blocking thread pool and convert its
/// outcome into a tool result. `Err(msg)` and panics both become `isError`
/// results; nothing escapes as a protocol error.
pub async fn run<F, O>(f: F) -> ToolResult
where
    F: FnOnce() -> Result<O, String> + Send + 'static,
    O: Into<Output> + Send + 'static,
{
    Ok(match tokio::task::spawn_blocking(f).await {
        Ok(Ok(out)) => success(out.into()),
        Ok(Err(msg)) => tool_error(msg),
        Err(e) => tool_error(format!("internal error: {e}")),
    })
}

/// Run a blocking closure that produces a value (used by resources).
pub async fn blocking<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("internal error: {e}"))?
}

/// A successful tool result. JSON is serialized compactly, like FastMCP does.
pub fn success(out: Output) -> CallToolResult {
    let text = match out {
        Output::Json(v) => v.to_string(),
        Output::Text(s) => s,
    };
    CallToolResult::success(vec![ContentBlock::text(text)])
}

/// A tool error result (`isError: true`) carrying `msg`.
pub fn tool_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.into())])
}

// ---------------------------------------------------------------------------
// Raw JSON API access. Python returns the API's JSON untouched, so tools use
// these instead of the typed `api::*` helpers (which drop fields, and some of
// which print to stdout, which would corrupt the stdio transport).
// ---------------------------------------------------------------------------

/// Parse a response body as JSON; an empty body is `null`.
fn parse(resp: ureq::Response) -> Result<Value, String> {
    let text = resp
        .into_string()
        .map_err(|e| format!("failed to read response: {e}"))?;
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|e| format!("failed to parse response: {e}"))
}

fn token() -> Result<String, String> {
    config::get_access_token()
}

/// The v2 session cookie (`TICKTICK_V2_SESSION_TOKEN`, else config.json).
pub fn session_token() -> Result<String, String> {
    config::get_v2_session_token().ok_or_else(|| {
        "v2 session token not set. Set TICKTICK_V2_SESSION_TOKEN from the 't' cookie in your browser."
            .to_string()
    })
}

pub fn v1_get(path: &str) -> Result<Value, String> {
    parse(api::get(path, &token()?)?)
}

pub fn v1_post(path: &str, body: &Value) -> Result<Value, String> {
    parse(api::post(path, &token()?, body)?)
}

/// POST without a body (e.g. the single-task complete endpoint).
pub fn v1_post_empty(path: &str) -> Result<(), String> {
    api::post_empty(path, &token()?).map(|_| ())
}

pub fn v1_put(path: &str, body: &Value) -> Result<Value, String> {
    parse(api::put(path, &token()?, body)?)
}

pub fn v1_delete(path: &str) -> Result<(), String> {
    api::delete(path, &token()?).map(|_| ())
}

/// DELETE with a JSON body (e.g. `/tag/delete`).
pub fn v1_delete_json(path: &str, body: &Value) -> Result<(), String> {
    api::delete_json(path, &token()?, body).map(|_| ())
}

pub fn v2_get(path: &str) -> Result<Value, String> {
    parse(api::v2::v2_get(path, &session_token()?)?)
}

pub fn v2_post(path: &str, body: &Value) -> Result<Value, String> {
    parse(api::v2::v2_post(path, &session_token()?, body)?)
}

pub fn v2_put(path: &str, body: &Value) -> Result<Value, String> {
    parse(api::v2::v2_put(path, &session_token()?, body)?)
}

/// A JSON array response as a vector (`null` becomes empty).
pub fn as_list(v: Value) -> Vec<Value> {
    match v {
        Value::Array(items) => items,
        _ => Vec::new(),
    }
}

/// Apply `op` to each item in order, stopping at the first failure. Earlier
/// writes stay applied, so the error says how many succeeded, e.g.
/// "Deleted 1 of 3 tag(s); 'b' failed: ...".
pub fn each<T, R>(
    items: &[T],
    verb: &str,
    noun: &str,
    label: impl Fn(&T) -> String,
    mut op: impl FnMut(&T) -> Result<R, String>,
) -> Result<Vec<R>, String> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        match op(item) {
            Ok(r) => out.push(r),
            Err(e) => {
                return Err(format!(
                    "{verb} {} of {} {noun}; '{}' failed: {e}",
                    out.len(),
                    items.len(),
                    label(item)
                ));
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Name resolution (Python `resolve.py`)
// ---------------------------------------------------------------------------

/// True for a hex string of 20+ characters, which is passed through as an ID.
pub fn looks_like_id(query: &str) -> bool {
    query.len() >= 20 && query.chars().all(|c| c.is_ascii_hexdigit())
}

fn field<'a>(item: &'a Value, key: &str) -> &'a str {
    item.get(key).and_then(Value::as_str).unwrap_or("")
}

/// Find the item `query` names. Order: exact name (case-insensitive), then a
/// single substring match. Several matches, or none, is an error; with none,
/// the closest name within edit distance 3 is suggested.
fn find_item<'a>(query: &str, items: &'a [Value], entity: &str) -> Result<&'a Value, String> {
    let search = query.to_lowercase();
    if let Some(item) = items
        .iter()
        .find(|i| field(i, "name").to_lowercase() == search)
    {
        return Ok(item);
    }
    let matches: Vec<&Value> = items
        .iter()
        .filter(|i| field(i, "name").to_lowercase().contains(&search))
        .collect();
    match matches.len() {
        1 => Ok(matches[0]),
        0 => {
            let mut msg = format!("No {entity} found matching '{query}'");
            let closest = items
                .iter()
                .map(|i| {
                    let name = field(i, "name");
                    (name, strsim::levenshtein(&search, &name.to_lowercase()))
                })
                .min_by_key(|(_, d)| *d);
            if let Some((name, dist)) = closest
                && dist <= 3
            {
                msg.push_str(&format!(". Did you mean '{name}'?"));
            }
            Err(msg)
        }
        _ => {
            let names: Vec<&str> = matches.iter().map(|m| field(m, "name")).collect();
            Err(format!(
                "Multiple {entity}s match '{query}': {}. Use a more specific name or the full ID.",
                names.join(", ")
            ))
        }
    }
}

/// Resolve a name or ID among `items` (JSON objects with `id` and `name`).
/// A hex ID of 20+ characters is returned as-is.
pub fn resolve_name(query: &str, items: &[Value], entity: &str) -> Result<String, String> {
    if looks_like_id(query) {
        return Ok(query.to_string());
    }
    find_item(query, items, entity).map(|i| field(i, "id").to_string())
}

/// Like [`resolve_name`] but also returns the item's `etag` ("" if unknown).
pub fn resolve_name_with_etag(
    query: &str,
    items: &[Value],
    entity: &str,
) -> Result<(String, String), String> {
    if looks_like_id(query) {
        let etag = items
            .iter()
            .find(|i| field(i, "id") == query)
            .map(|i| field(i, "etag").to_string())
            .unwrap_or_default();
        return Ok((query.to_string(), etag));
    }
    find_item(query, items, entity)
        .map(|i| (field(i, "id").to_string(), field(i, "etag").to_string()))
}

/// Inbox project ID, discovered once per process (and cached in config.json
/// when it is writable) by creating and deleting a probe task.
pub fn inbox_id() -> Result<String, String> {
    static INBOX: Mutex<Option<String>> = Mutex::new(None);
    let mut cached = INBOX.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(id) = cached.as_ref() {
        return Ok(id.clone());
    }
    let id = api::project::get_inbox_id()
        .map_err(|e| format!("Could not discover inbox project ID: {e}"))?;
    *cached = Some(id.clone());
    Ok(id)
}

/// Resolve a project name or ID. "inbox" means the default inbox; inbox IDs
/// (`inbox<digits>`) and hex IDs pass through; names match fuzzily.
pub fn resolve_project_id(project: &str) -> Result<String, String> {
    if project.eq_ignore_ascii_case("inbox") {
        return inbox_id();
    }
    if let Some(rest) = project.strip_prefix("inbox")
        && !rest.is_empty()
        && rest.chars().all(|c| c.is_ascii_digit())
    {
        return Ok(project.to_string());
    }
    if looks_like_id(project) {
        return Ok(project.to_string());
    }
    let projects = as_list(v1_get("/project")?);
    resolve_name(project, &projects, "project")
}

/// Resolve a kanban column name or ID within a project.
pub fn resolve_column_id(project_id: &str, column: &str) -> Result<String, String> {
    let columns = as_list(v1_get(&format!("/project/{project_id}/column"))?);
    resolve_name(column, &columns, "column")
}

/// Resolve a folder (project group) name or ID to `(id, etag)`.
pub fn resolve_folder_id(folder: &str) -> Result<(String, String), String> {
    let groups = as_list(v1_get("/project/group")?);
    resolve_name_with_etag(folder, &groups, "folder")
}

// ---------------------------------------------------------------------------
// Dates (Python `dates.py`)
// ---------------------------------------------------------------------------

pub use api::task::{ParsedDateTime, parse_datetime, parse_duration};

/// Reject a name that is not a tz database zone (paths included).
pub fn check_timezone(tz: &str) -> Result<(), String> {
    api::task::time_zone(Some(tz)).map(|_| ())
}

/// Format a parsed date for the API with a UTC offset. The offset comes from
/// `tz` when given, else the stored TickTick account timezone, else the system
/// timezone (`TZ` / `/etc/localtime`).
pub fn api_date(date: &ParsedDateTime, tz: Option<&str>) -> Result<String, String> {
    match tz {
        Some(tz) => date.to_api_string(Some(tz)),
        None => match config::get_account_timezone() {
            Some(account) if check_timezone(&account).is_ok() => date.to_api_string(Some(&account)),
            _ => date.to_api_string(None),
        },
    }
}

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

/// `Some(s)` only for a non-empty string. Python checks many optional strings
/// with `if value:`, which treats "" like None.
pub fn nonempty(s: Option<String>) -> Option<String> {
    s.filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// Priority
// ---------------------------------------------------------------------------

/// Map a priority name (none, low, medium, high) to TickTick's number.
pub fn priority_value(priority: &str) -> Result<i32, String> {
    match priority.to_lowercase().as_str() {
        "none" => Ok(0),
        "low" => Ok(1),
        "medium" => Ok(3),
        "high" => Ok(5),
        _ => Err(format!(
            "Invalid priority '{priority}'. Use: none, low, medium, high"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn projects() -> Vec<Value> {
        vec![
            json!({"id": "p1", "name": "Work"}),
            json!({"id": "p2", "name": "Personal"}),
            json!({"id": "p3", "name": "Work Archive"}),
        ]
    }

    #[test]
    fn exact_match_wins_over_substring() {
        assert_eq!(resolve_name("work", &projects(), "project").unwrap(), "p1");
    }

    #[test]
    fn single_substring_match() {
        assert_eq!(resolve_name("pers", &projects(), "project").unwrap(), "p2");
    }

    #[test]
    fn ambiguous_substring_is_an_error() {
        let err = resolve_name("wor", &projects(), "project").unwrap_err();
        assert_eq!(
            err,
            "Multiple projects match 'wor': Work, Work Archive. Use a more specific name or the full ID."
        );
    }

    #[test]
    fn no_match_suggests_close_name() {
        let err = resolve_name("Persnal", &projects(), "project").unwrap_err();
        assert_eq!(
            err,
            "No project found matching 'Persnal'. Did you mean 'Personal'?"
        );
        let err = resolve_name("zzzzzzzz", &projects(), "project").unwrap_err();
        assert_eq!(err, "No project found matching 'zzzzzzzz'");
    }

    #[test]
    fn hex_id_passes_through() {
        let id = "aaaaaaaaaaaaaaaaaaaaaaaa";
        assert_eq!(resolve_name(id, &[], "project").unwrap(), id);
    }

    #[test]
    fn etag_variant_returns_etag() {
        let groups = vec![json!({"id": "g1", "name": "Home", "etag": "e1"})];
        assert_eq!(
            resolve_name_with_etag("home", &groups, "folder").unwrap(),
            ("g1".to_string(), "e1".to_string())
        );
    }

    #[test]
    fn priorities() {
        assert_eq!(priority_value("High").unwrap(), 5);
        assert_eq!(priority_value("none").unwrap(), 0);
        assert!(priority_value("urgent").is_err());
    }

    #[test]
    fn timezone_check() {
        assert!(check_timezone("America/Chicago").is_ok());
        assert!(check_timezone("UTC").is_ok());
        assert!(check_timezone("Asia/Kolkata").is_ok());
        for bad in [
            "Mars/Olympus",
            "../etc/passwd",
            "/etc/hosts",
            "/usr/share/zoneinfo/America/Chicago",
            "America/../Europe/London",
            "",
        ] {
            assert_eq!(
                check_timezone(bad),
                Err(format!("Unknown timezone '{bad}'"))
            );
        }
    }
}
