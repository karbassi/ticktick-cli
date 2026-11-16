//! End-to-end tests for `search_tasks` and `filter_tasks`; cases follow Python
//! `tests/test_tools/test_search.py`. The server runs with `TZ=America/Chicago`
//! (January and February are `-0600`).

mod common;

use common::McpBuilder;
use httpmock::prelude::*;
use serde_json::json;

const V1: &str = "/open/v1";

fn mock_projects(server: &MockServer) {
    server.mock(|when, then| {
        when.method(GET).path(format!("{V1}/project"));
        then.status(200)
            .json_body(json!([{"id": "p1", "name": "Work"}]));
    });
}

#[test]
fn search_keyword_only() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path(format!("{V1}/task/search"))
            .json_body(json!({"keywords": "quokka"}));
        then.status(200)
            .json_body(json!([{"id": "t1", "title": "Quokka"}]));
    });
    let mut c = McpBuilder::new(&server).start();
    c.call_tool("search_tasks", json!({"query": "quokka"})).ok();
    m.assert();
}

#[test]
fn search_all_options() {
    let server = MockServer::start();
    mock_projects(&server);
    let m = server.mock(|when, then| {
        when.method(POST)
            .path(format!("{V1}/task/search"))
            .json_body(json!({
                "keywords": "plan",
                "projectIds": ["p1"],
                "tags": ["a"],
                "status": [0, 2],
                "dueFrom": "2030-01-01T00:00:00.000-0600",
                "dueTo": "2030-02-01T09:30:00.000-0600",
            }));
        then.status(200).json_body(json!([]));
    });
    let mut c = McpBuilder::new(&server).start();
    c.call_tool(
        "search_tasks",
        json!({
            "query": "plan",
            "projects": ["Work"],
            "tags": ["a"],
            "status": ["active", "completed"],
            "due_from": "2030-01-01",
            "due_to": "2030-02-01T09:30",
        }),
    )
    .ok();
    m.assert();
}

#[test]
fn search_empty_response_is_empty_list() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path(format!("{V1}/task/search"));
        then.status(200);
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool("search_tasks", json!({"query": "nothing"}))
        .ok();
    assert_eq!(res.json(), json!([]));
}

#[test]
fn search_empty_lists_are_omitted() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path(format!("{V1}/task/search"))
            .json_body(json!({"keywords": "x"}));
        then.status(200).json_body(json!([]));
    });
    let mut c = McpBuilder::new(&server).start();
    c.call_tool(
        "search_tasks",
        json!({"query": "x", "projects": [], "tags": [], "status": []}),
    )
    .ok();
    m.assert();
}

#[test]
fn search_bad_status_rejected() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path(format!("{V1}/task/search"));
        then.status(200).json_body(json!([]));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool("search_tasks", json!({"query": "x", "status": ["done"]}));
    assert!(res.is_error);
    assert!(res.text.contains("Invalid status 'done'"), "{}", res.text);
    m.assert_calls(0);
}

#[test]
fn filter_all_options() {
    let server = MockServer::start();
    mock_projects(&server);
    let m = server.mock(|when, then| {
        when.method(POST)
            .path(format!("{V1}/task/filter"))
            .json_body(json!({
                "projectIds": ["p1"],
                "startDate": "2030-01-01T00:00:00.000-0600",
                "endDate": "2030-02-01T00:00:00.000-0600",
                "priority": [5, 3],
                "tag": ["a", "b"],
                "status": [-1],
                "kind": ["CHECKLIST", "TASK"],
            }));
        then.status(200).json_body(json!([{"id": "t1"}]));
    });
    let mut c = McpBuilder::new(&server).start();
    c.call_tool(
        "filter_tasks",
        json!({
            "projects": ["Work"],
            "start_date": "2030-01-01",
            "end_date": "2030-02-01",
            "priority": ["high", "medium"],
            "tags": ["a", "b"],
            "status": ["abandoned"],
            "kind": ["checklist", "text"],
        }),
    )
    .ok();
    m.assert();
}

#[test]
fn filter_no_criteria_sends_empty_body() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path(format!("{V1}/task/filter"))
            .json_body(json!({}));
        then.status(200).json_body(json!([]));
    });
    let mut c = McpBuilder::new(&server).start();
    c.call_tool("filter_tasks", json!({})).ok();
    m.assert();
}

#[test]
fn filter_bad_priority_and_kind_rejected() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path(format!("{V1}/task/filter"));
        then.status(200).json_body(json!([]));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool("filter_tasks", json!({"priority": ["urgent"]}));
    assert!(res.is_error);
    assert!(
        res.text.contains("Invalid priority 'urgent'"),
        "{}",
        res.text
    );
    let res = c.call_tool("filter_tasks", json!({"kind": ["todo"]}));
    assert!(res.is_error);
    assert!(res.text.contains("Invalid kind 'todo'"), "{}", res.text);
    m.assert_calls(0);
}
