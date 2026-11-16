//! Python `tools/countdowns.py`. Read-only: the official API has no write endpoints.

use rmcp::{tool, tool_router};
use serde_json::json;

use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, run};

#[tool_router(router = countdowns_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "List all countdowns (birthdays, anniversaries, events).\n\nRead-only: the official API has no create, update, or delete for countdowns. Each countdown has an id, name, date (YYYYMMDD integer), repeatFlag, reminders, and display settings.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_countdowns(&self) -> ToolResult {
        run(|| {
            let v = util::v1_get("/countdown")?;
            Ok(if v.is_null() { json!([]) } else { v })
        })
        .await
    }
}
