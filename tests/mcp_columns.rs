//! End-to-end tests for the column tools, ported from the Python server's
//! `test_columns.py`.

mod common;

use common::{HEX_PROJECT, McpBuilder};
use httpmock::prelude::*;
use serde_json::{Value, json};

const C1: &str = "bbbbbbbbbbbbbbbbbbbbbbbb";
const C2: &str = "cccccccccccccccccccccccc";

fn columns() -> Value {
    json!([
        {"id": C1, "projectId": HEX_PROJECT, "name": "To do", "color": null, "sortOrder": 0},
        {"id": C2, "projectId": HEX_PROJECT, "name": "Doing", "color": null, "sortOrder": 1},
    ])
}

fn mock_columns(server: &MockServer) -> httpmock::Mock<'_> {
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("/open/v1/project/{HEX_PROJECT}/column"));
        then.status(200).json_body(columns());
    })
}

#[test]
fn list_columns_returns_columns() {
    let server = MockServer::start();
    let list = mock_columns(&server);
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool("list_columns", json!({"project": HEX_PROJECT}))
        .ok();
    list.assert();
    assert_eq!(res.json(), columns());
}

#[test]
fn list_columns_resolves_project_name() {
    let server = MockServer::start();
    let projects = server.mock(|when, then| {
        when.method(GET).path("/open/v1/project");
        then.status(200)
            .json_body(json!([{"id": HEX_PROJECT, "name": "Board"}]));
    });
    let list = mock_columns(&server);
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool("list_columns", json!({"project": "Board"}))
        .ok();
    projects.assert();
    list.assert();
    assert_eq!(res.json()[1]["name"], "Doing");
}

#[test]
fn add_column_minimal() {
    let server = MockServer::start();
    let add = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/open/v1/project/{HEX_PROJECT}/column"))
            .json_body(json!({"name": "Done"}));
        then.status(200)
            .json_body(json!({"id": "x", "name": "Done"}));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool(
            "add_column",
            json!({"project": HEX_PROJECT, "name": "Done"}),
        )
        .ok();
    add.assert();
    assert_eq!(res.json()["id"], "x");
}

#[test]
fn add_column_all_fields() {
    let server = MockServer::start();
    let add = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/open/v1/project/{HEX_PROJECT}/column"))
            .json_body(json!({"name": "Done", "sortOrder": 3, "color": "#FF0000"}));
        then.status(200)
            .json_body(json!({"id": "x", "name": "Done"}));
    });
    let mut c = McpBuilder::new(&server).start();
    c.call_tool(
        "add_column",
        json!({"project": HEX_PROJECT, "name": "Done", "sort_order": 3, "color": "#FF0000"}),
    )
    .ok();
    add.assert();
}

#[test]
fn edit_column_resolves_name() {
    let server = MockServer::start();
    let list = mock_columns(&server);
    let edit = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/open/v1/project/{HEX_PROJECT}/column/{C2}"))
            .json_body(json!({"name": "In progress"}));
        then.status(200)
            .json_body(json!({"id": C2, "name": "In progress"}));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool(
            "edit_column",
            json!({"project": HEX_PROJECT, "column": "doing", "name": "In progress"}),
        )
        .ok();
    list.assert();
    edit.assert();
    assert_eq!(res.json()["name"], "In progress");
}

#[test]
fn edit_column_needs_a_field() {
    let server = MockServer::start();
    let _list = mock_columns(&server);
    let edit = server.mock(|when, then| {
        when.method(POST);
        then.status(200).json_body(json!({}));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool(
        "edit_column",
        json!({"project": HEX_PROJECT, "column": "doing"}),
    );
    assert!(res.is_error);
    assert!(res.text.contains("at least one"), "{}", res.text);
    edit.assert_calls(0);
}

#[test]
fn edit_column_unknown_column() {
    let server = MockServer::start();
    let _list = mock_columns(&server);
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool(
        "edit_column",
        json!({"project": HEX_PROJECT, "column": "zzz", "name": "x"}),
    );
    assert!(res.is_error);
    assert!(res.text.contains("No column found"), "{}", res.text);
}
