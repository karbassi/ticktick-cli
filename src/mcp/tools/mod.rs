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

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use serde_json::{Map, Value};

use super::TickTickServer;

/// FastMCP's `outputSchema` per tool, generated from the Python return
/// annotations. Clients that cached the Python tool list expect it, and a
/// tool with one must return `structuredContent` (see `structured`).
const OUTPUT_SCHEMAS: &str = include_str!("../output_schemas.json");

pub fn output_schemas() -> Map<String, Value> {
    serde_json::from_str(OUTPUT_SCHEMAS).expect("output_schemas.json is a JSON object")
}

pub fn router() -> ToolRouter<TickTickServer> {
    let mut router = routes();
    for (name, schema) in output_schemas() {
        let Value::Object(schema) = schema else {
            panic!("output schema for {name} is not an object");
        };
        let route = router
            .map
            .get_mut(name.as_str())
            .unwrap_or_else(|| panic!("output schema for unknown tool {name}"));
        route.attr.output_schema = Some(Arc::new(schema));
    }
    router
}

/// The `structuredContent` FastMCP sends for a successful call: the JSON
/// value itself for an object schema, or `{"result": value}` when the
/// schema is marked `x-fastmcp-wrap-result` (lists and strings). `None`
/// when the text doesn't fit the schema.
pub fn structured(schema: &Map<String, Value>, text: &str) -> Option<Value> {
    if schema.get("x-fastmcp-wrap-result") == Some(&Value::Bool(true)) {
        let result = &schema["properties"]["result"];
        let value = if result["type"] == "string" {
            Value::String(text.to_string())
        } else {
            let v: Value = serde_json::from_str(text).ok()?;
            if result["type"] == "array" && !v.is_array() {
                return None;
            }
            v
        };
        return Some(serde_json::json!({ "result": value }));
    }
    let v: Value = serde_json::from_str(text).ok()?;
    v.is_object().then_some(v)
}

fn routes() -> ToolRouter<TickTickServer> {
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
