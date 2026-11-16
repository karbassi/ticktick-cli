//! Calendar tools (Python `tools/calendar.py`): list_calendars, list_events.
//! `sync_account` from that file is already in `sync.rs`; don't add it here.
//! Stub: fill in the tools and remove them from `PENDING_TOOLS`.

use rmcp::tool_router;

use crate::mcp::TickTickServer;

#[tool_router(router = calendar_router, vis = "pub(crate)", allow_empty)]
impl TickTickServer {}
