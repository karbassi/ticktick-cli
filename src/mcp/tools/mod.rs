//! MCP tool modules, one per Python `tools/<module>.py`. Each module defines a
//! `#[tool_router(router = <module>_router, vis = "pub(crate)")]` impl block on
//! `TickTickServer`; [`router`] adds them together. Every module is already
//! declared and wired below (unported ones are empty stubs), so filling one in
//! touches only its own file. See `src/mcp/README.md`.

mod calendar;
mod columns;
mod comments;
mod countdowns;
mod filters;
mod focus;
mod folders;
mod habits;
mod projects;
mod search;
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
        + TickTickServer::calendar_router()
        + TickTickServer::columns_router()
        + TickTickServer::comments_router()
        + TickTickServer::countdowns_router()
        + TickTickServer::filters_router()
        + TickTickServer::focus_router()
        + TickTickServer::folders_router()
        + TickTickServer::habits_router()
        + TickTickServer::search_router()
}
