//! The Python tests only exercise the client; these assert the request bodies
//! built in `filters.py`.

mod common;

use common::{McpBuilder, SESSION_TOKEN};
use httpmock::prelude::*;
use serde_json::{Value, json};

fn batch() -> Value {
    json!({
        "filters": [
            {"id": "f1", "name": "High Priority", "etag": "e1", "rule": "{\"and\":[],\"type\":0}"},
            {"id": "f2", "name": "Due Today", "etag": "e2"},
        ],
        "projectGroups": [],
    })
}

fn mock_batch<'a>(server: &'a MockServer, body: Value) -> httpmock::Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/batch/check/0")
            .header("Cookie", format!("t={SESSION_TOKEN}"));
        then.status(200).json_body(body);
    })
}

fn mock_filter_post<'a>(server: &'a MockServer, body: Value) -> httpmock::Mock<'a> {
    server.mock(|when, then| {
        when.method(POST)
            .path("/api/v2/batch/filter")
            .json_body(body);
        then.status(200)
            .json_body(json!({"id2etag": {"f1": "new"}}));
    })
}

#[test]
fn response_over_10_mib_is_read_whole() {
    let server = MockServer::start();
    let rule = "x".repeat(11 * 1024 * 1024);
    mock_batch(
        &server,
        json!({"filters": [{"id": "f1", "name": "Big", "rule": rule}]}),
    );
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool("list_filters", json!({})).ok();
    assert_eq!(
        res.json()[0]["rule"].as_str().map(str::len),
        Some(rule.len())
    );
}

#[test]
fn list_filters_missing_is_empty() {
    let server = MockServer::start();
    mock_batch(&server, json!({"filters": null}));
    let mut c = McpBuilder::new(&server).start();
    assert_eq!(
        c.call_tool("list_filters", json!({})).ok().json(),
        json!([])
    );
}

#[test]
fn add_filter_minimal() {
    let server = MockServer::start();
    let add = mock_filter_post(&server, json!({"add": [{"name": "Overdue"}]}));
    let mut c = McpBuilder::new(&server).start();
    c.call_tool("add_filter", json!({"name": "Overdue"})).ok();
    add.assert();
}

#[test]
fn add_filter_all_fields() {
    let server = MockServer::start();
    let add = mock_filter_post(
        &server,
        json!({"add": [{"name": "Overdue", "rule": "{\"and\":[],\"type\":1}", "sortType": "dueDate"}]}),
    );
    let mut c = McpBuilder::new(&server).start();
    c.call_tool(
        "add_filter",
        json!({"name": "Overdue", "rule": "{\"and\":[],\"type\":1}", "sort_type": "dueDate"}),
    )
    .ok();
    add.assert();
}

#[test]
fn edit_filter_resolves_name_and_sends_etag() {
    let server = MockServer::start();
    mock_batch(&server, batch());
    let edit = mock_filter_post(
        &server,
        json!({"update": [{"id": "f1", "etag": "e1", "name": "Very High Priority"}]}),
    );
    let mut c = McpBuilder::new(&server).start();
    c.call_tool(
        "edit_filter",
        json!({"filter_name": "high priority", "name": "Very High Priority"}),
    )
    .ok();
    edit.assert();
}

#[test]
fn edit_filter_unknown() {
    let server = MockServer::start();
    mock_batch(&server, batch());
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool("edit_filter", json!({"filter_name": "zzz", "name": "x"}));
    assert!(res.is_error);
    assert!(res.text.contains("No filter found"), "{}", res.text);
}

#[test]
fn delete_filters_resolves_names() {
    let server = MockServer::start();
    mock_batch(&server, batch());
    let del = mock_filter_post(&server, json!({"delete": ["f1", "f2"]}));
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool(
            "delete_filters",
            json!({"filters": ["High Priority", "today"]}),
        )
        .ok();
    del.assert();
    assert_eq!(res.text, "Deleted 2 filter(s)");
}
