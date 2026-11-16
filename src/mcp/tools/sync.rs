//! Full account sync (Python `sync_account`, which lives in `tools/calendar.py`).

use rmcp::{tool, tool_router};

use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, run};

#[tool_router(router = sync_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "Perform a full account sync.\n\nFetches the complete account state including all projects, tasks, tags, filters, folders, and settings. Useful for getting a comprehensive view of the account or forcing a refresh. Requires the v2 session token.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn sync_account(&self) -> ToolResult {
        run(|| util::v2_get("/batch/check/0")).await
    }
}
