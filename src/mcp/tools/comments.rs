//! Comment tools (Python `tools/comments.py`).

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, resolve_project_id, run};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListCommentsParams {
    /// The task ID.
    pub task_id: String,
    /// The project name or ID containing the task.
    pub project: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddCommentParams {
    /// The task ID.
    pub task_id: String,
    /// The project name or ID containing the task.
    pub project: String,
    /// The comment text.
    pub text: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteCommentParams {
    /// The task ID.
    pub task_id: String,
    /// The project name or ID containing the task.
    pub project: String,
    /// The comment ID (from list_comments).
    pub comment_id: String,
}

#[tool_router(router = comments_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "List the comments on a task.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_comments(&self, Parameters(p): Parameters<ListCommentsParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            let v = util::v1_get(&format!("/project/{pid}/task/{}/comments", p.task_id))?;
            Ok(if v.is_null() { json!([]) } else { v })
        })
        .await
    }

    #[tool(
        description = "Add a comment to a task.\n\nComments cannot be edited through the official API. To change one, delete it and add a new one.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn add_comment(&self, Parameters(p): Parameters<AddCommentParams>) -> ToolResult {
        run(move || -> Result<Value, String> {
            let pid = resolve_project_id(&p.project)?;
            util::v1_post(
                &format!("/project/{pid}/task/{}/comment", p.task_id),
                &json!({ "title": p.text }),
            )
        })
        .await
    }

    #[tool(
        description = "Delete a comment from a task.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn delete_comment(&self, Parameters(p): Parameters<DeleteCommentParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            util::v1_delete(&format!(
                "/project/{pid}/task/{}/comment/{}",
                p.task_id, p.comment_id
            ))?;
            Ok(format!("Comment {} deleted", p.comment_id))
        })
        .await
    }
}
