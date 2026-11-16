//! Task comment tools (Python `tools/comments.py`): list_comments,
//! add_comment, delete_comment.
//! Stub: fill in the tools and remove them from `PENDING_TOOLS`.

use rmcp::tool_router;

use crate::mcp::TickTickServer;

#[tool_router(router = comments_router, vis = "pub(crate)", allow_empty)]
impl TickTickServer {}
