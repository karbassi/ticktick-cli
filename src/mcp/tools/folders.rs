//! Python `tools/folders.py`.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, resolve_folder_id, run};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddFolderParams {
    /// Folder name (required).
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteFoldersParams {
    /// List of folder names or IDs to delete. Supports fuzzy matching.
    pub folders: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameFolderParams {
    /// Current folder name or ID. Supports fuzzy matching.
    pub folder: String,
    /// New folder name.
    pub new_name: String,
}

#[tool_router(router = folders_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "List all project folders (groups).\n\nFolders organize projects into groups. Returns folder IDs, names, and sort settings.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_folders(&self) -> ToolResult {
        run(|| Ok(Value::Array(util::as_list(util::v1_get("/project/group")?)))).await
    }

    #[tool(
        description = "Create a new project folder.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn add_folder(&self, Parameters(p): Parameters<AddFolderParams>) -> ToolResult {
        run(move || util::v1_post("/project/group", &json!({ "name": p.name }))).await
    }

    #[tool(
        description = "Delete one or more folders.\n\nProjects inside deleted folders are not deleted; they become ungrouped.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn delete_folders(&self, Parameters(p): Parameters<DeleteFoldersParams>) -> ToolResult {
        run(move || {
            let targets = p
                .folders
                .iter()
                .map(|f| resolve_folder_id(f).map(|(id, _)| (f.clone(), id)))
                .collect::<Result<Vec<(String, String)>, String>>()?;
            util::each(
                &targets,
                "Deleted",
                "folder(s)",
                |(name, _)| name.clone(),
                |(_, id)| util::v1_delete(&format!("/project/group/{id}")),
            )?;
            Ok(format!("Deleted {} folder(s)", targets.len()))
        })
        .await
    }

    #[tool(
        description = "Rename a project folder.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn rename_folder(&self, Parameters(p): Parameters<RenameFolderParams>) -> ToolResult {
        run(move || {
            let (id, _) = resolve_folder_id(&p.folder)?;
            util::v1_post(
                &format!("/project/group/{id}"),
                &json!({ "name": p.new_name }),
            )
        })
        .await
    }
}
