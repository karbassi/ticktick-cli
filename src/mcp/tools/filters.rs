//! Saved filter tools (Python `tools/filters.py`): list_filters, add_filter,
//! edit_filter, delete_filters. Filters come from the v2 `batch/check` state.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, resolve_name_with_etag, run};

/// The `filters` list of the v2 account state (`null` or missing is empty).
fn batch_filters() -> Result<Vec<Value>, String> {
    let data = util::v2_get("/batch/check/0")?;
    Ok(util::as_list(
        data.get("filters").cloned().unwrap_or(Value::Null),
    ))
}

/// Resolve a filter name or ID to `(id, etag)`.
fn resolve_filter(name_or_id: &str) -> Result<(String, String), String> {
    resolve_name_with_etag(name_or_id, &batch_filters()?, "filter")
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddFilterParams {
    /// Filter name (required).
    pub name: String,
    /// JSON rule definition string (e.g. '{"and":[],"type":0}').
    #[serde(default)]
    pub rule: Option<String>,
    /// Sort type for filter results (e.g. "dueDate", "priority").
    #[serde(default)]
    pub sort_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditFilterParams {
    /// Current filter name or ID. Supports fuzzy matching.
    pub filter_name: String,
    /// New filter name.
    #[serde(default)]
    pub name: Option<String>,
    /// New JSON rule definition string.
    #[serde(default)]
    pub rule: Option<String>,
    /// New sort type.
    #[serde(default)]
    pub sort_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteFiltersParams {
    /// List of filter names or IDs to delete. Supports fuzzy matching.
    pub filters: Vec<String>,
}

#[tool_router(router = filters_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "List all saved filters.\n\nReturns all custom filters with their IDs, names, rules, and sort settings. Requires v2 session token.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_filters(&self) -> ToolResult {
        run(|| Ok(Value::Array(batch_filters()?))).await
    }

    #[tool(
        description = "Create a new saved filter.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn add_filter(&self, Parameters(p): Parameters<AddFilterParams>) -> ToolResult {
        run(move || {
            let mut f = json!({ "name": p.name });
            if let Some(rule) = p.rule {
                f["rule"] = json!(rule);
            }
            if let Some(sort_type) = p.sort_type {
                f["sortType"] = json!(sort_type);
            }
            util::v2_post("/batch/filter", &json!({ "add": [f] }))
        })
        .await
    }

    #[tool(
        description = "Update an existing filter.\n\nOnly provided fields are changed.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn edit_filter(&self, Parameters(p): Parameters<EditFilterParams>) -> ToolResult {
        run(move || {
            let (fid, etag) = resolve_filter(&p.filter_name)?;
            let mut update = json!({ "id": fid, "etag": etag });
            if let Some(name) = p.name {
                update["name"] = json!(name);
            }
            if let Some(rule) = p.rule {
                update["rule"] = json!(rule);
            }
            if let Some(sort_type) = p.sort_type {
                update["sortType"] = json!(sort_type);
            }
            util::v2_post("/batch/filter", &json!({ "update": [update] }))
        })
        .await
    }

    #[tool(
        description = "Delete one or more saved filters.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn delete_filters(&self, Parameters(p): Parameters<DeleteFiltersParams>) -> ToolResult {
        run(move || {
            let ids = p
                .filters
                .iter()
                .map(|f| resolve_filter(f).map(|(id, _)| id))
                .collect::<Result<Vec<String>, String>>()?;
            util::v2_post("/batch/filter", &json!({ "delete": ids }))?;
            Ok(format!("Deleted {} filter(s)", ids.len()))
        })
        .await
    }
}
