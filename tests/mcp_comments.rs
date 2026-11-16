mod common;

use common::{McpBuilder, V1, mock_projects};
use httpmock::prelude::*;
use serde_json::json;

#[test]
fn list_comments_resolves_project_and_null_body_is_empty_list() {
    let server = MockServer::start();
    mock_projects(&server, json!([{"id": "p1", "name": "Work"}]));
    let list = server.mock(|when, then| {
        when.method(GET)
            .path(format!("{V1}/project/p1/task/t1/comments"));
        then.status(200);
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool("list_comments", json!({"task_id": "t1", "project": "work"}))
        .ok();
    list.assert();
    assert_eq!(res.json(), json!([]));
}

#[test]
fn add_comment_sends_text_as_title() {
    let server = MockServer::start();
    mock_projects(&server, json!([{"id": "p1", "name": "Work"}]));
    let add = server.mock(|when, then| {
        when.method(POST)
            .path(format!("{V1}/project/p1/task/t1/comment"))
            .json_body(json!({"title": "hello"}));
        then.status(200).json_body(json!({"id": "c3"}));
    });
    let mut c = McpBuilder::new(&server).start();
    c.call_tool(
        "add_comment",
        json!({"task_id": "t1", "project": "Work", "text": "hello"}),
    )
    .ok();
    add.assert();
}

#[test]
fn delete_comment_resolves_project_and_confirms() {
    let server = MockServer::start();
    mock_projects(&server, json!([{"id": "p1", "name": "Work"}]));
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
