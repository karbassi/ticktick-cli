//! `list_events` reports API errors, unlike the Python server, which returns an
//! empty list. The other calendar tools are pure pass-through and have no e2e test.

mod common;

use common::McpBuilder;
use httpmock::prelude::*;
use serde_json::json;

const RANGE: (&str, &str) = (
    "2026-02-22T00:00:00.000+0000",
    "2026-02-22T23:59:59.999+0000",
);

#[test]
fn list_events_reports_api_errors() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/api/v2/calendar/bind/events/all");
        then.status(500).body("boom");
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool("list_events", json!({"begin": RANGE.0, "end": RANGE.1}));
    m.assert();
    assert!(res.is_error);
    assert!(res.text.contains("500"), "{}", res.text);
}

#[test]
fn list_events_without_session_reports_the_hint() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/api/v2/calendar/bind/events/all");
        then.status(200).json_body(json!([]));
    });
    let mut c = McpBuilder::new(&server)
        .env("TICKTICK_V2_SESSION_TOKEN", "")
        .start();
    let res = c.call_tool("list_events", json!({"begin": RANGE.0, "end": RANGE.1}));
    m.assert_calls(0);
    assert!(res.is_error);
    assert!(
        res.text.starts_with("v2 session token not set"),
        "{}",
        res.text
    );
}
