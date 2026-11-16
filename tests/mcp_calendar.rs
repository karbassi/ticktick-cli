//! `list_events` swallows API errors into an empty list, as Python does.

mod common;

use common::McpBuilder;
use httpmock::prelude::*;
use serde_json::json;

#[test]
fn list_events_swallows_errors_into_empty_list() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/api/v2/calendar/bind/events/all");
        then.status(500).body("boom");
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool(
            "list_events",
            json!({"begin": "2026-02-22T00:00:00.000+0000", "end": "2026-02-22T23:59:59.999+0000"}),
        )
        .ok();
    m.assert();
    assert_eq!(res.json(), json!([]));
}
