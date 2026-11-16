//! Search tools (Python `tools/search.py`): server-side task search and filter.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::task::{TaskFilter, TaskSearch};
use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, api_date, priority_value, resolve_project_ids, run};

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

/// Python `_project_ids`. Empty lists are left out of the request body.
fn project_ids(projects: Option<Vec<String>>) -> Result<Vec<String>, String> {
    match projects {
        Some(list) if !list.is_empty() => resolve_project_ids(&list),
        _ => Ok(Vec::new()),
    }
}

fn mapped<T>(
    items: Option<Vec<String>>,
    lookup: impl Fn(&str) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    items
        .unwrap_or_default()
        .iter()
        .map(|s| lookup(s))
        .collect()
}

fn statuses(status: Option<Vec<String>>) -> Result<Vec<i32>, String> {
    mapped(status, |s| match s.to_lowercase().as_str() {
        "active" => Ok(0),
        "completed" => Ok(2),
        "abandoned" => Ok(-1),
        _ => Err(format!(
            "Invalid status '{s}'. Use: active, completed, abandoned"
        )),
    })
}

fn priorities(priority: Option<Vec<String>>) -> Result<Vec<i32>, String> {
    mapped(priority, |p| {
        priority_value(p)
            .map_err(|_| format!("Invalid priority '{p}'. Use: none, low, medium, high"))
    })
}

fn kinds(kind: Option<Vec<String>>) -> Result<Vec<String>, String> {
    mapped(kind, |k| match k.to_lowercase().as_str() {
        "text" | "task" => Ok("TASK".to_string()),
        "note" => Ok("NOTE".to_string()),
        "checklist" => Ok("CHECKLIST".to_string()),
        _ => Err(format!("Invalid kind '{k}'. Use: text, note, checklist")),
    })
}

/// Python `_date`.
fn date(value: Option<String>) -> Result<Option<String>, String> {
    match util::nonempty(value) {
        Some(v) => Ok(Some(api_date(&v, &util::local_zone()?)?)),
        None => Ok(None),
    }
}

fn post(endpoint: &str, body: &impl serde::Serialize) -> Result<Value, String> {
    let body = serde_json::to_value(body).map_err(|e| e.to_string())?;
    util::v1_post(endpoint, &body).map(or_empty)
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
            let search = TaskSearch {
                keywords: p.query,
                project_ids: project_ids(p.projects)?,
                tags: p.tags.unwrap_or_default(),
                status: statuses(p.status)?,
                due_from: date(p.due_from)?,
                due_to: date(p.due_to)?,
            };
            post("/task/search", &search)
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
            let filter = TaskFilter {
                project_ids: project_ids(p.projects)?,
                start_date: date(p.start_date)?,
                end_date: date(p.end_date)?,
                priority: priorities(p.priority)?,
                tag: p.tags.unwrap_or_default(),
                status: statuses(p.status)?,
                kind: kinds(p.kind)?,
            };
            post("/task/filter", &filter)
        })
        .await
    }
}
