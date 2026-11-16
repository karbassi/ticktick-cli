//! Kanban column tools (Python `tools/columns.py`): list_columns, add_column,
//! edit_column. Column name resolution is `util::resolve_column_id`.
//! Stub: fill in the tools and remove them from `PENDING_TOOLS`.

use rmcp::tool_router;

use crate::mcp::TickTickServer;

#[tool_router(router = columns_router, vis = "pub(crate)", allow_empty)]
impl TickTickServer {}
