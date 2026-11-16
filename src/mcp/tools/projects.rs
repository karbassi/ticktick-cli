//! Python `tools/projects.py`.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::api::project::ProjectFields;
use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, nonempty, resolve_folder_id, resolve_project_id, run};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectParams {
    /// Project name or ID. Supports fuzzy matching.
    pub project: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddProjectParams {
    /// Project name (required).
    pub name: String,
    /// Hex color code (e.g. "#FF0000").
    #[serde(default)]
    pub color: Option<String>,
    /// View mode: "list", "kanban", "timeline".
    #[serde(default)]
    pub view_mode: Option<String>,
    /// Project kind (e.g. "TASK", "NOTE").
    #[serde(default)]
    pub kind: Option<String>,
    /// Folder name or ID to place the project in.
    #[serde(default)]
    pub folder: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditProjectParams {
    /// Project name or ID to edit.
    pub project: String,
    /// New project name.
    #[serde(default)]
    pub name: Option<String>,
    /// New hex color code.
    #[serde(default)]
    pub color: Option<String>,
    /// New view mode: "list", "kanban", "timeline".
    #[serde(default)]
    pub view_mode: Option<String>,
    /// Move project to this folder (name or ID).
    #[serde(default)]
    pub folder: Option<String>,
    /// Set to true to remove the project from its folder.
    #[serde(default)]
    pub remove_folder: bool,
}

#[tool_router(router = projects_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "List all projects (task lists) in TickTick.\n\nReturns all projects including their IDs, names, colors, and folder assignments. Does not include the Inbox: use \"inbox\" as the project name in task tools to interact with the default inbox.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_projects(&self) -> ToolResult {
        run(|| util::v1_get("/project")).await
    }

    #[tool(
        description = "Get a single project by name or ID.\n\nSupports fuzzy name matching. Use \"inbox\" to get the inbox project.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_project(&self, Parameters(p): Parameters<ProjectParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            util::v1_get(&format!("/project/{pid}"))
        })
        .await
    }

    #[tool(
        description = "List the members of a project.\n\nReturns username, displayName, and self (true for you). A project that is not shared lists only you. Use a member's username with assign_task.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_project_members(&self, Parameters(p): Parameters<ProjectParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            util::v1_get(&format!("/project/{pid}/members"))
        })
        .await
    }

    #[tool(
        description = "Create a new project (task list).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn add_project(&self, Parameters(p): Parameters<AddProjectParams>) -> ToolResult {
        run(move || {
            let group_id = match nonempty(p.folder) {
                Some(folder) => Some(Some(resolve_folder_id(&folder)?.0)),
                None => None,
            };
            let fields = ProjectFields {
                name: Some(p.name),
                color: nonempty(p.color),
                view_mode: nonempty(p.view_mode),
                kind: nonempty(p.kind),
                group_id,
            };
            let mut body = json!({});
            fields.apply_to(&mut body);
            util::v1_post("/project", &body)
        })
        .await
    }

    #[tool(
        description = "Update an existing project's properties.\n\nOnly provided fields are changed.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn edit_project(&self, Parameters(p): Parameters<EditProjectParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            let group_id = if p.remove_folder {
                Some(None)
            } else {
                match nonempty(p.folder) {
                    Some(folder) => Some(Some(resolve_folder_id(&folder)?.0)),
                    None => None,
                }
            };
            let fields = ProjectFields {
                name: p.name,
                color: p.color,
                view_mode: p.view_mode,
                kind: None,
                group_id,
            };
            let mut body = json!({ "id": pid });
            fields.apply_to(&mut body);
            util::v1_post(&format!("/project/{pid}"), &body)
        })
        .await
    }

    #[tool(
        description = "Permanently delete a project and all its tasks.\n\nThis action cannot be undone.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn delete_project(&self, Parameters(p): Parameters<ProjectParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            util::v1_delete(&format!("/project/{pid}"))?;
            Ok(format!("Project {} deleted", p.project))
        })
        .await
    }
}
