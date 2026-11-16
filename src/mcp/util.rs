//! Mirrors the Python server's `client.py`, `resolve.py` and `dates.py` so tool
//! output and errors match it.

use std::io::Read;
use std::sync::{Mutex, OnceLock};

use jiff::tz::TimeZone;
use rmcp::ErrorData;
use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::Value;

use crate::api;
use crate::config;

/// Failures are `isError` tool results, never protocol errors, so the model sees the message.
pub type ToolResult = Result<CallToolResult, ErrorData>;

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

/// Panics also become `isError` results.
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

pub async fn blocking<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("internal error: {e}"))?
}

/// JSON is serialized compactly, like FastMCP.
pub fn success(out: Output) -> CallToolResult {
    let text = match out {
        Output::Json(v) => v.to_string(),
        Output::Text(s) => s,
    };
    CallToolResult::success(vec![ContentBlock::text(text)])
}

pub fn tool_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.into())])
}

// Raw JSON like Python, not the typed `api::*` structs, which drop fields.

/// Not `into_string`: it caps bodies at 10 MiB, which large accounts exceed.
fn parse(resp: ureq::Response) -> Result<Value, String> {
    let mut body = Vec::new();
    resp.into_reader()
        .read_to_end(&mut body)
        .map_err(|e| format!("failed to read response: {e}"))?;
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&body).map_err(|e| format!("failed to parse response: {e}"))
}

fn token() -> Result<String, String> {
    config::get_access_token()
}

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

pub fn v1_post_empty(path: &str) -> Result<(), String> {
    api::post_empty(path, &token()?).map(|_| ())
}

pub fn v1_put(path: &str, body: &Value) -> Result<Value, String> {
    parse(api::put(path, &token()?, body)?)
}

pub fn v1_delete(path: &str) -> Result<(), String> {
    api::delete(path, &token()?).map(|_| ())
}

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

pub fn as_list(v: Value) -> Vec<Value> {
    match v {
        Value::Array(items) => items,
        _ => Vec::new(),
    }
}

/// Earlier writes stay applied on failure, so the error says how many succeeded.
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
pub fn looks_like_id(query: &str) -> bool {
    query.len() >= 20 && query.chars().all(|c| c.is_ascii_hexdigit())
}

fn field<'a>(item: &'a Value, key: &str) -> &'a str {
    item.get(key).and_then(Value::as_str).unwrap_or("")
}

fn find_item<'a>(query: &str, items: &'a [Value], entity: &str) -> Result<&'a Value, String> {
    find_item_with(
        query,
        items,
        |query, closest| {
            let mut msg = format!("No {entity} found matching '{query}'");
            if let Some(name) = closest {
                msg.push_str(&format!(". Did you mean '{name}'?"));
            }
            msg
        },
        |query, names| {
            format!(
                "Multiple {entity}s match '{query}': {}. Use a more specific name or the full ID.",
                names.join(", ")
            )
        },
    )
}

/// Exact, then unique-substring name match; callers supply the error texts.
/// `not_found` gets the closest name when it is within edit distance 3.
fn find_item_with<'a>(
    query: &str,
    items: &'a [Value],
    not_found: impl Fn(&str, Option<&str>) -> String,
    ambiguous: impl Fn(&str, &[&str]) -> String,
) -> Result<&'a Value, String> {
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
            let closest = items
                .iter()
                .map(|i| {
                    let name = field(i, "name");
                    (name, strsim::levenshtein(&search, &name.to_lowercase()))
                })
                .min_by_key(|(_, d)| *d)
                .filter(|(_, d)| *d <= 3)
                .map(|(name, _)| name);
            Err(not_found(query, closest))
        }
        _ => {
            let names: Vec<&str> = matches.iter().map(|m| field(m, "name")).collect();
            Err(ambiguous(query, &names))
        }
    }
}

/// `resolve_name` with caller-supplied error texts.
pub fn resolve_name_with(
    query: &str,
    items: &[Value],
    not_found: impl Fn(&str, Option<&str>) -> String,
    ambiguous: impl Fn(&str, &[&str]) -> String,
) -> Result<String, String> {
    if looks_like_id(query) {
        return Ok(query.to_string());
    }
    find_item_with(query, items, not_found, ambiguous).map(|i| field(i, "id").to_string())
}

pub fn resolve_name(query: &str, items: &[Value], entity: &str) -> Result<String, String> {
    if looks_like_id(query) {
        return Ok(query.to_string());
    }
    find_item(query, items, entity).map(|i| field(i, "id").to_string())
}

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

/// Discovered by creating and deleting a probe task, so cached per process.
/// Discovery is serialized so concurrent callers don't each create a probe.
pub fn inbox_id() -> Result<String, String> {
    static INBOX: OnceLock<String> = OnceLock::new();
    static DISCOVERING: Mutex<()> = Mutex::new(());
    if let Some(id) = INBOX.get() {
        return Ok(id.clone());
    }
    let _discovering = DISCOVERING.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(id) = INBOX.get() {
        return Ok(id.clone());
    }
    let id = api::project::get_inbox_id()
        .map_err(|e| format!("Could not discover inbox project ID: {e}"))?;
    Ok(INBOX.get_or_init(|| id).clone())
}

/// The ID for `project` when it needs no `/project` lookup.
fn direct_project_id(project: &str) -> Result<Option<String>, String> {
    if project.eq_ignore_ascii_case("inbox") {
        return inbox_id().map(Some);
    }
    if let Some(rest) = project.strip_prefix("inbox")
        && !rest.is_empty()
        && rest.chars().all(|c| c.is_ascii_digit())
    {
        return Ok(Some(project.to_string()));
    }
    Ok(looks_like_id(project).then(|| project.to_string()))
}

pub fn resolve_project_id(project: &str) -> Result<String, String> {
    match direct_project_id(project)? {
        Some(id) => Ok(id),
        None => resolve_name(project, &as_list(v1_get("/project")?), "project"),
    }
}

/// Like [`resolve_project_id`], but `/project` is fetched at most once.
pub fn resolve_project_ids<S: AsRef<str>>(projects: &[S]) -> Result<Vec<String>, String> {
    let mut all: Option<Vec<Value>> = None;
    let mut ids = Vec::with_capacity(projects.len());
    for project in projects {
        let project = project.as_ref();
        let id = match direct_project_id(project)? {
            Some(id) => id,
            None => {
                if all.is_none() {
                    all = Some(as_list(v1_get("/project")?));
                }
                resolve_name(project, all.as_deref().unwrap_or_default(), "project")?
            }
        };
        ids.push(id);
    }
    Ok(ids)
}

pub fn resolve_column_id(project_id: &str, column: &str) -> Result<String, String> {
    let columns = as_list(v1_get(&format!("/project/{project_id}/column"))?);
    resolve_name(column, &columns, "column")
}

pub fn resolve_folder_id(folder: &str) -> Result<(String, String), String> {
    let groups = as_list(v1_get("/project/group")?);
    resolve_name_with_etag(folder, &groups, "folder")
}

pub use api::task::{ParsedDateTime, parse_datetime_in, parse_duration};

/// The canonical IANA name. Rejects paths too, not just unknown names.
pub fn check_timezone(tz: &str) -> Result<String, String> {
    api::task::canonical_timezone(tz)
}

/// `TICKTICK_TIMEZONE` (IANA name) if set, else the system zone like Python.
pub fn local_zone() -> Result<TimeZone, String> {
    match std::env::var("TICKTICK_TIMEZONE") {
        Ok(name) if !name.is_empty() => {
            api::task::time_zone(Some(&name)).map_err(|e| format!("TICKTICK_TIMEZONE: {e}"))
        }
        _ => Ok(api::task::system_zone()),
    }
}

/// The caller's `timezone`, else [`local_zone`].
pub fn zone(tz: Option<&str>) -> Result<TimeZone, String> {
    match tz {
        Some(tz) => api::task::time_zone(Some(tz)),
        None => local_zone(),
    }
}

/// Parses and formats in one zone, so "today" and the offset agree.
pub fn api_date(input: &str, zone: &TimeZone) -> Result<String, String> {
    parse_datetime_in(input, zone)?.to_api_string_in(zone)
}

/// Python checks with `if value:`, which treats "" like None.
pub fn nonempty(s: Option<String>) -> Option<String> {
    s.filter(|s| !s.is_empty())
}

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
        assert_eq!(
            check_timezone("america/CHICAGO").unwrap(),
            "America/Chicago"
        );
        assert_eq!(check_timezone("utc").unwrap(), "UTC");
        for bad in [
            "Mars/Olympus",
            "Etc/Unknown",
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
