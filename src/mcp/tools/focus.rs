//! Focus/pomodoro tools (Python `tools/focus.py`): focus_status, focus_stats,
//! focus_log, focus_timeline, focus_save. The ms API base is
//! `TICKTICK_MS_BASE` (tests mock it at `/ms`).
//! Stub: fill in the tools and remove them from `PENDING_TOOLS`.

use rmcp::tool_router;

use crate::mcp::TickTickServer;

#[tool_router(router = focus_router, vis = "pub(crate)", allow_empty)]
impl TickTickServer {}
