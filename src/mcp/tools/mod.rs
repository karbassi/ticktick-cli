//! One module per Python `tools/<module>.py`. See `src/mcp/README.md`.

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
