//! Python `tools/columns.py`.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::api::column::ColumnFields;
use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, resolve_column_id, resolve_project_id, run};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListColumnsParams {
    /// Project name or ID. Supports fuzzy matching.
    pub project: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddColumnParams {
    /// Project name or ID. Supports fuzzy matching.
    pub project: String,
    /// Column name (required).
    pub name: String,
    /// Position among the columns; lower comes first.
    #[serde(default)]
    pub sort_order: Option<i64>,
    /// Column color as a hex string, e.g. "#FF0000".
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditColumnParams {
    /// Project name or ID. Supports fuzzy matching.
    pub project: String,
    /// Column name or ID. Supports fuzzy matching.
    pub column: String,
    /// New column name.
    #[serde(default)]
    pub name: Option<String>,
    /// New position among the columns; lower comes first.
    #[serde(default)]
    pub sort_order: Option<i64>,
    /// New color as a hex string, e.g. "#FF0000".
    #[serde(default)]
    pub color: Option<String>,
}

#[tool_router(router = columns_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "List the kanban columns of a project.\n\nReturns column IDs, names, colors, and sort orders. A project only has columns when it uses the kanban view.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_columns(&self, Parameters(p): Parameters<ListColumnsParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            let columns = util::v1_get(&format!("/project/{pid}/column"))?;
            Ok(Value::Array(util::as_list(columns)))
        })
        .await
    }

    #[tool(
        description = "Create a kanban column in a project.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn add_column(&self, Parameters(p): Parameters<AddColumnParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            let fields = ColumnFields {
                name: Some(p.name),
                sort_order: p.sort_order,
                color: p.color,
            };
            util::v1_post(&format!("/project/{pid}/column"), &fields.to_body())
        })
        .await
    }

    #[tool(
        description = "Rename, reorder, or recolor a kanban column.\n\nOnly provided fields are changed. The official API has no endpoint to delete a column, so there is no delete_column tool.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn edit_column(&self, Parameters(p): Parameters<EditColumnParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            let cid = resolve_column_id(&pid, &p.column)?;
            let fields = ColumnFields {
                name: p.name,
                sort_order: p.sort_order,
                color: p.color,
            };
            if fields.is_empty() {
                return Err("Provide at least one of name, sort_order, color".to_string());
            }
            util::v1_post(&format!("/project/{pid}/column/{cid}"), &fields.to_body())
        })
        .await
    }
}
