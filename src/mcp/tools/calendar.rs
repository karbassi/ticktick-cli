//! Python `tools/calendar.py`; its `sync_account` is in `sync.rs`.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, run};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListEventsParams {
    /// Start datetime in ISO format (e.g. "2026-02-11T00:00:00.000+0000").
    pub begin: String,
    /// End datetime in ISO format (e.g. "2026-02-18T23:59:59.999+0000").
    pub end: String,
}

#[tool_router(router = calendar_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "List third-party calendar accounts connected to TickTick.\n\nReturns connected calendar accounts (Google Calendar, Outlook, etc.) with their sync status. Requires v2 session token.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_calendars(&self) -> ToolResult {
        run(|| util::v2_get("/calendar/third/accounts")).await
    }

    #[tool(
        description = "Query calendar events for a date range.\n\nReturns events from all connected calendars within the specified date range.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_events(&self, Parameters(p): Parameters<ListEventsParams>) -> ToolResult {
        run(move || -> Result<Value, String> {
            // Python swallows every error here and returns an empty list.
            Ok(util::v2_post(
                "/calendar/bind/events/all",
                &json!({ "begin": p.begin, "end": p.end }),
            )
            .unwrap_or_else(|_| json!([])))
        })
        .await
    }
}
