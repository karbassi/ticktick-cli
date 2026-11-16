//! Saved filter tools (Python `tools/filters.py`): list_filters, add_filter,
//! edit_filter, delete_filters.
//! Stub: fill in the tools and remove them from `PENDING_TOOLS`.

use rmcp::tool_router;

use crate::mcp::TickTickServer;

#[tool_router(router = filters_router, vis = "pub(crate)", allow_empty)]
impl TickTickServer {}
