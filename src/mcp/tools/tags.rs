//! Tag tools (Python `tools/tags.py`).

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::tag::TagFields;
use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, run};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TagNamesParams {
    /// List of tag names.
    pub names: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameTagParams {
    /// Current tag name.
    pub old_name: String,
    /// New tag name.
    pub new_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditTagParams {
    /// The tag name to edit.
    pub name: String,
    /// New hex color code (e.g. "#FF0000").
    #[serde(default)]
    pub color: Option<String>,
    /// Set a parent tag name (for hierarchical tags).
    #[serde(default)]
    pub parent: Option<String>,
    /// Set to true to remove the parent tag.
    #[serde(default)]
    pub clear_parent: bool,
    /// New sort order integer.
    #[serde(default)]
    pub sort_order: Option<i64>,
    /// Sort type (e.g. "project", "dueDate").
    #[serde(default)]
    pub sort_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeTagsParams {
    /// Tag name to merge from (will be deleted).
    pub source: String,
    /// Tag name to merge into (will be kept).
    pub target: String,
}

#[tool_router(router = tags_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "List all tags.\n\nReturns all tags with their names, colors, parent relationships, and sort settings.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_tags(&self) -> ToolResult {
        run(|| util::v1_get("/tag")).await
    }

    #[tool(
        description = "Create one or more new tags.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn add_tags(&self, Parameters(p): Parameters<TagNamesParams>) -> ToolResult {
        run(move || {
            let created = p
                .names
                .iter()
                .map(|n| util::v1_post("/tag", &json!({ "label": n, "name": n })))
                .collect::<Result<Vec<Value>, String>>()?;
            Ok(Value::Array(created))
        })
        .await
    }

    #[tool(
        description = "Delete one or more tags by name.\n\nTasks keep existing, but the tag is removed from every task that had it.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn delete_tags(&self, Parameters(p): Parameters<TagNamesParams>) -> ToolResult {
        run(move || {
            for name in &p.names {
                util::v1_delete_json("/tag/delete?scope=0", &json!({ "name": name }))?;
            }
            Ok(format!("Deleted {} tag(s)", p.names.len()))
        })
        .await
    }

    #[tool(
        description = "Rename a tag.\n\nAll tasks with the old tag name will be updated to use the new name.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn rename_tag(&self, Parameters(p): Parameters<RenameTagParams>) -> ToolResult {
        run(move || {
            util::v1_put(
                "/tag/rename?scope=0",
                &json!({ "name": p.old_name, "newName": p.new_name }),
            )?;
            Ok(format!("Tag '{}' renamed to '{}'", p.old_name, p.new_name))
        })
        .await
    }

    #[tool(
        description = "Update a tag's properties.\n\nOnly provided fields are changed.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn edit_tag(&self, Parameters(p): Parameters<EditTagParams>) -> ToolResult {
        run(move || {
            let parent = if p.clear_parent {
                Some(None)
            } else {
                p.parent.map(Some)
            };
            let fields = TagFields {
                color: p.color,
                parent,
                sort_order: p.sort_order,
                sort_type: p.sort_type,
            };
            let mut tag = json!({ "name": p.name });
            fields.apply_to(&mut tag);
            util::v1_put("/tag", &tag)
        })
        .await
    }

    #[tool(
        description = "Merge a source tag into a target tag.\n\nAll tasks tagged with the source tag will be re-tagged with the target tag. The source tag is deleted. Requires the v2 session token.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn merge_tags(&self, Parameters(p): Parameters<MergeTagsParams>) -> ToolResult {
        run(move || {
            util::v2_put(
                "/tag/merge",
                &json!({ "name": p.source, "newName": p.target }),
            )?;
            Ok(format!("Tag '{}' merged into '{}'", p.source, p.target))
        })
        .await
    }
}
