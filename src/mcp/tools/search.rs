//! Task search tools (Python `tools/search.py`): search_tasks, filter_tasks.
//! Stub: fill in the tools and remove them from `PENDING_TOOLS`.

use rmcp::tool_router;

use crate::mcp::TickTickServer;

#[tool_router(router = search_router, vis = "pub(crate)", allow_empty)]
impl TickTickServer {}
