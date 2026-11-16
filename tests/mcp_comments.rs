//! End-to-end tests for the comment tools; cases follow Python
//! `tests/test_tools/test_comments.py`.

mod common;

use common::McpBuilder;
use httpmock::prelude::*;
use serde_json::{Value, json};

const V1: &str = "/open/v1";

fn mock_projects(server: &MockServer) {
    server.mock(|when, then| {
        when.method(GET).path(format!("{V1}/project"));
        then.status(200)
            .json_body(json!([{"id": "p1", "name": "Work"}]));
    });
}

#[test]
fn list_comments() {
    let server = MockServer::start();
    mock_projects(&server);
    let list = server.mock(|when, then| {
        when.method(GET)
            .path(format!("{V1}/project/p1/task/t1/comments"));
        then.status(200).json_body(json!([
            {"id": "c1", "userId": 1, "title": "first", "createdTime": 1, "modifiedTime": 1},
            {"id": "c2", "userId": 1, "title": "second", "createdTime": 2, "modifiedTime": 2},
        ]));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool("list_comments", json!({"task_id": "t1", "project": "work"}))
        .ok();
    list.assert();
    let titles: Vec<Value> = res
        .json()
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["title"].clone())
        .collect();
    assert_eq!(titles, vec![json!("first"), json!("second")]);
}

#[test]
fn list_comments_empty_and_null_bodies() {
    let server = MockServer::start();
    mock_projects(&server);
    let mut list = server.mock(|when, then| {
        when.method(GET)
            .path(format!("{V1}/project/p1/task/t1/comments"));
        then.status(200).json_body(json!([]));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool("list_comments", json!({"task_id": "t1", "project": "Work"}))
        .ok();
    assert_eq!(res.json(), json!([]));
    list.delete();
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("{V1}/project/p1/task/t1/comments"));
        then.status(200);
    });
    let res = c
        .call_tool("list_comments", json!({"task_id": "t1", "project": "Work"}))
        .ok();
    assert_eq!(res.json(), json!([]));
}

#[test]
fn add_comment_sends_title() {
    let server = MockServer::start();
    mock_projects(&server);
    let add = server.mock(|when, then| {
        when.method(POST)
            .path(format!("{V1}/project/p1/task/t1/comment"))
            .json_body(json!({"title": "hello"}));
        then.status(200)
            .json_body(json!({"id": "c3", "title": "hello"}));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool(
            "add_comment",
            json!({"task_id": "t1", "project": "Work", "text": "hello"}),
        )
        .ok();
    add.assert();
    assert_eq!(res.json()["id"], "c3");
}

#[test]
fn delete_comment() {
    let server = MockServer::start();
    mock_projects(&server);
    let del = server.mock(|when, then| {
        when.method(DELETE)
            .path(format!("{V1}/project/p1/task/t1/comment/c1"));
        then.status(200);
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool(
            "delete_comment",
            json!({"task_id": "t1", "project": "Work", "comment_id": "c1"}),
        )
        .ok();
    del.assert();
    assert_eq!(res.text, "Comment c1 deleted");
}
