//! Folder (project group) tools (Python `tools/folders.py`): list_folders,
//! add_folder, delete_folders, rename_folder. Folder name resolution is
//! `util::resolve_folder_id` (returns the etag too).
//! Stub: fill in the tools and remove them from `PENDING_TOOLS`.

use rmcp::tool_router;

use crate::mcp::TickTickServer;

#[tool_router(router = folders_router, vis = "pub(crate)", allow_empty)]
impl TickTickServer {}
