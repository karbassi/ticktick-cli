//! MCP tool modules, one per Python `tools/<module>.py`. Each module defines a
//! `#[tool_router(router = <module>_router, vis = "pub(crate)")]` impl block on
//! `TickTickServer`; [`router`] adds them together. See `src/mcp/README.md`.

mod projects;
mod sync;
mod tags;
mod tasks;

use rmcp::handler::server::router::tool::ToolRouter;

use super::TickTickServer;

/// Every tool the server exposes.
pub fn router() -> ToolRouter<TickTickServer> {
    TickTickServer::tasks_router()
        + TickTickServer::projects_router()
        + TickTickServer::tags_router()
        + TickTickServer::sync_router()
}
