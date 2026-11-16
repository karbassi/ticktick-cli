//! Countdown tools (Python `tools/countdowns.py`): list_countdowns.
//! Stub: fill in the tools and remove them from `PENDING_TOOLS`.

use rmcp::tool_router;

use crate::mcp::TickTickServer;

#[tool_router(router = countdowns_router, vis = "pub(crate)", allow_empty)]
impl TickTickServer {}
