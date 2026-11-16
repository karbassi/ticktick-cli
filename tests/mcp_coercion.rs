//! FastMCP's lax pydantic accepted `"5"` or `25.0` for an int, `true` for an
//! int, and `"true"`, `"yes"` or `1` for a bool, so callers send them.

mod common;

use common::{McpBuilder, SESSION_TOKEN};
use httpmock::prelude::*;
use serde_json::json;

#[test]
fn integral_float_for_an_int() {
    let server = MockServer::start();
    let save = server.mock(|when, then| {
        when.method(POST)
            .path("/open/v1/focus")
            .json_body_includes(r#"{"startTime": "2026-02-22T14:00:00.000+0000"}"#);
        then.status(200).json_body(json!({"id": "f1"}));
    });
    let mut c = McpBuilder::new(&server).start();
    c.call_tool(
        "focus_save",
        json!({"duration_minutes": 25.0, "end": "2026-02-22T14:25:00+00:00"}),
    )
    .ok();
    save.assert();
}

#[test]
fn numeric_string_for_an_int() {
    let server = MockServer::start();
    let completed = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/project/all/completedInAll/")
            .query_param("limit", "5")
            .header("Cookie", format!("t={SESSION_TOKEN}"));
        then.status(200).json_body(json!([]));
    });
    let mut c = McpBuilder::new(&server).start();
    c.call_tool("list_tasks", json!({"status": "completed", "limit": "5"}))
        .ok();
    completed.assert();
}

#[test]
fn string_for_a_bool() {
    let server = MockServer::start();
    let edit = server.mock(|when, then| {
        when.method(PUT)
            .path("/open/v1/tag")
            .json_body(json!({"name": "a", "parent": ""}));
        then.status(200).json_body(json!({"name": "a"}));
    });
    let mut c = McpBuilder::new(&server).start();
    c.call_tool(
        "edit_tag",
        json!({"name": "a", "parent": "p", "clear_parent": "true"}),
    )
    .ok();
    edit.assert();
}

#[test]
fn yes_and_one_for_a_bool() {
    for flag in [json!("yes"), json!(1)] {
        let server = MockServer::start();
        let edit = server.mock(|when, then| {
            when.method(PUT)
                .path("/open/v1/tag")
                .json_body(json!({"name": "a", "parent": ""}));
            then.status(200).json_body(json!({"name": "a"}));
        });
        let mut c = McpBuilder::new(&server).start();
        c.call_tool(
            "edit_tag",
            json!({"name": "a", "parent": "p", "clear_parent": flag}),
        )
        .ok();
        edit.assert();
    }
}

#[test]
fn bool_for_an_int() {
    let server = MockServer::start();
    let completed = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/project/all/completedInAll/")
            .query_param("limit", "1");
        then.status(200).json_body(json!([]));
    });
    let mut c = McpBuilder::new(&server).start();
    c.call_tool("list_tasks", json!({"status": "completed", "limit": true}))
        .ok();
    completed.assert();
}

#[test]
fn unconvertible_value_is_still_rejected() {
    let server = MockServer::start();
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool("list_tasks", json!({"status": "completed", "limit": "abc"}));
    assert!(res.is_error);
    assert!(
        res.text
            .contains(r#"invalid type: string "abc", expected i64"#),
        "{}",
        res.text
    );
}
