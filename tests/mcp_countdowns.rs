//! End-to-end tests for `list_countdowns`; cases follow Python
//! `tests/test_tools/test_countdowns.py`.

mod common;

use common::McpBuilder;
use httpmock::prelude::*;
use serde_json::json;

#[test]
fn list_uses_v1_only() {
    let server = MockServer::start();
    let v1 = server.mock(|when, then| {
        when.method(GET).path("/open/v1/countdown");
        then.status(200).json_body(json!([
            {"id": "c1", "name": "Birthday", "date": 19860506},
            {"id": "c2", "name": "Anniversary", "date": 20180630},
        ]));
    });
    let v2 = server.mock(|when, then| {
        when.method(GET).path("/api/v2/batch/check/0");
        then.status(200).json_body(json!({}));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool("list_countdowns", json!({})).ok();
    v1.assert();
    v2.assert_hits(0);
    let data = res.json();
    assert_eq!(data[0]["name"], "Birthday");
    assert_eq!(data[1]["name"], "Anniversary");
    assert_eq!(data[0]["date"], 19860506);
}

#[test]
fn list_empty() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/open/v1/countdown");
        then.status(200).json_body(json!([]));
    });
    let mut c = McpBuilder::new(&server).start();
    assert_eq!(
        c.call_tool("list_countdowns", json!({})).ok().json(),
        json!([])
    );
}
