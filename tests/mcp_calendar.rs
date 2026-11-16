//! End-to-end tests for `list_calendars` and `list_events`; cases follow Python
//! `tests/test_tools/test_calendar.py`.

mod common;

use common::{McpBuilder, SESSION_TOKEN};
use httpmock::prelude::*;
use serde_json::json;

#[test]
fn list_calendars() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/calendar/third/accounts")
            .header_includes("Cookie", SESSION_TOKEN);
        then.status(200).json_body(json!([
            {"id": "cal1", "type": "google", "name": "Google Calendar"},
            {"id": "cal2", "type": "outlook", "name": "Outlook"},
        ]));
    });
    let mut c = McpBuilder::new(&server).start();
    let data = c.call_tool("list_calendars", json!({})).ok().json();
    m.assert();
    assert_eq!(data.as_array().unwrap().len(), 2);
    assert_eq!(data[0]["type"], "google");
}

#[test]
fn list_events() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v2/calendar/bind/events/all")
            .json_body(json!({
                "begin": "2026-02-22T00:00:00.000+0000",
                "end": "2026-02-22T23:59:59.999+0000",
            }));
        then.status(200).json_body(json!([{
            "id": "ev1",
            "title": "Standup",
            "startDate": "2026-02-22T09:00:00.000+0000",
            "endDate": "2026-02-22T09:30:00.000+0000",
        }]));
    });
    let mut c = McpBuilder::new(&server).start();
    let data = c
        .call_tool(
            "list_events",
            json!({"begin": "2026-02-22T00:00:00.000+0000", "end": "2026-02-22T23:59:59.999+0000"}),
        )
        .ok()
        .json();
    m.assert();
    assert_eq!(data.as_array().unwrap().len(), 1);
    assert_eq!(data[0]["title"], "Standup");
}

#[test]
fn list_events_swallows_errors_into_empty_list() {
    let server = MockServer::start();
    server.mock(|when, then| {
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
    assert_eq!(res.json(), json!([]));
}
