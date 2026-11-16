//! Search tools (Python `tools/search.py`): server-side task search and filter.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::mcp::TickTickServer;
use crate::mcp::util::{
    self, ToolResult, api_date, parse_datetime, priority_value, resolve_project_id, run,
};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchTasksParams {
    /// Keyword(s) to search for.
    pub query: String,
    /// Only search these projects (names or IDs; "inbox" works).
    #[serde(default)]
    pub projects: Option<Vec<String>>,
    /// Only tasks with these tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Statuses to include: "active", "completed", "abandoned".
    #[serde(default)]
    pub status: Option<Vec<String>>,
    /// Earliest due date: "today", "tomorrow", "YYYY-MM-DD", or "YYYY-MM-DDTHH:MM".
    #[serde(default)]
    pub due_from: Option<String>,
    /// Latest due date, same formats.
    #[serde(default)]
    pub due_to: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FilterTasksParams {
    /// Only these projects (names or IDs; "inbox" works).
    #[serde(default)]
    pub projects: Option<Vec<String>>,
    /// Earliest task start date: "today", "tomorrow", "YYYY-MM-DD",
    /// or "YYYY-MM-DDTHH:MM".
    #[serde(default)]
    pub start_date: Option<String>,
    /// Latest task start date, same formats.
    #[serde(default)]
    pub end_date: Option<String>,
    /// Priorities to include: "none", "low", "medium", "high".
    #[serde(default)]
    pub priority: Option<Vec<String>>,
    /// Only tasks with these tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Statuses to include: "active", "completed", "abandoned".
    #[serde(default)]
    pub status: Option<Vec<String>>,
    /// Task kinds to include: "text", "note", "checklist".
    #[serde(default)]
    pub kind: Option<Vec<String>>,
}

/// Python `_project_ids`: None or empty means no filter.
fn project_ids(projects: Option<Vec<String>>) -> Result<Option<Value>, String> {
    match projects {
        Some(list) if !list.is_empty() => {
            let ids = list
                .iter()
                .map(|p| resolve_project_id(p).map(Value::String))
                .collect::<Result<Vec<Value>, String>>()?;
            Ok(Some(Value::Array(ids)))
        }
        _ => Ok(None),
    }
}

/// Map each name through `lookup`; None or empty means no filter.
fn mapped(
    items: Option<Vec<String>>,
    lookup: impl Fn(&str) -> Result<Value, String>,
) -> Result<Option<Value>, String> {
    match items {
        Some(list) if !list.is_empty() => {
            let out = list
                .iter()
                .map(|s| lookup(s))
                .collect::<Result<Vec<Value>, String>>()?;
            Ok(Some(Value::Array(out)))
        }
        _ => Ok(None),
    }
}

fn statuses(status: Option<Vec<String>>) -> Result<Option<Value>, String> {
    mapped(status, |s| match s.to_lowercase().as_str() {
        "active" => Ok(json!(0)),
        "completed" => Ok(json!(2)),
        "abandoned" => Ok(json!(-1)),
        _ => Err(format!(
            "Invalid status '{s}'. Use: active, completed, abandoned"
        )),
    })
}

fn priorities(priority: Option<Vec<String>>) -> Result<Option<Value>, String> {
    mapped(priority, |p| {
        priority_value(p)
            .map(|v| json!(v))
            .map_err(|_| format!("Invalid priority '{p}'. Use: none, low, medium, high"))
    })
}

fn kinds(kind: Option<Vec<String>>) -> Result<Option<Value>, String> {
    mapped(kind, |k| match k.to_lowercase().as_str() {
        "text" | "task" => Ok(json!("TASK")),
        "note" => Ok(json!("NOTE")),
        "checklist" => Ok(json!("CHECKLIST")),
        _ => Err(format!("Invalid kind '{k}'. Use: text, note, checklist")),
    })
}

/// Python `_date`: empty or missing means no bound.
fn date(value: Option<String>) -> Result<Option<Value>, String> {
    match util::nonempty(value) {
        Some(v) => Ok(Some(json!(api_date(&parse_datetime(&v)?, None)))),
        None => Ok(None),
    }
}

/// Python `tags or None`.
fn tag_list(tags: Option<Vec<String>>) -> Option<Value> {
    tags.filter(|t| !t.is_empty()).map(|t| json!(t))
}

fn put(body: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(v) = value {
        body.insert(key.to_string(), v);
    }
}

/// Python `... or []`: an empty or null response is an empty list.
fn or_empty(v: Value) -> Value {
    if v.is_null() { json!([]) } else { v }
}

#[tool_router(router = search_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "Search tasks by keyword, server-side.\n\nMatches whole words in task titles and content, case-insensitively (\"quokka\" finds \"Quokka plan\", \"quok\" does not). Faster than listing every project and scanning. For criteria without a keyword (priority, date range, kind), use filter_tasks.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn search_tasks(&self, Parameters(p): Parameters<SearchTasksParams>) -> ToolResult {
        run(move || {
            let mut body = Map::new();
            body.insert("keywords".into(), json!(p.query));
            put(&mut body, "projectIds", project_ids(p.projects)?);
            put(&mut body, "tags", tag_list(p.tags));
            put(&mut body, "status", statuses(p.status)?);
            put(&mut body, "dueFrom", date(p.due_from)?);
            put(&mut body, "dueTo", date(p.due_to)?);
            util::v1_post("/task/search", &Value::Object(body)).map(or_empty)
        })
        .await
    }

    #[tool(
        description = "Filter tasks by project, date range, priority, tags, status, or kind, server-side.\n\nAll criteria are optional and combine. No keyword matching; use search_tasks for that. Not related to the saved filters (list_filters and friends).",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn filter_tasks(&self, Parameters(p): Parameters<FilterTasksParams>) -> ToolResult {
        run(move || {
            let mut body = Map::new();
            put(&mut body, "projectIds", project_ids(p.projects)?);
            put(&mut body, "startDate", date(p.start_date)?);
            put(&mut body, "endDate", date(p.end_date)?);
            put(&mut body, "priority", priorities(p.priority)?);
            put(&mut body, "tag", tag_list(p.tags));
            put(&mut body, "status", statuses(p.status)?);
            put(&mut body, "kind", kinds(p.kind)?);
            util::v1_post("/task/filter", &Value::Object(body)).map(or_empty)
        })
        .await
    }
}
