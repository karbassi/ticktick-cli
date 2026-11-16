//! Ported from Python `test_folders.py`.

mod common;

use common::McpBuilder;
use httpmock::prelude::*;
use serde_json::{Value, json};

fn groups() -> Value {
    json!([
        {"id": "g1", "name": "Home", "sortOrder": 0, "showAll": true},
        {"id": "g2", "name": "Work", "sortOrder": 1, "showAll": true},
    ])
}

fn mock_groups(server: &MockServer) -> httpmock::Mock<'_> {
    server.mock(|when, then| {
        when.method(GET).path("/open/v1/project/group");
        then.status(200).json_body(groups());
    })
}

#[test]
fn list_folders_uses_v1() {
    let server = MockServer::start();
    let list = mock_groups(&server);
    let v2 = server.mock(|when, then| {
        when.method(GET).path("/api/v2/batch/check/0");
        then.status(200).json_body(json!({}));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool("list_folders", json!({})).ok();
    list.assert();
    v2.assert_calls(0);
    assert_eq!(res.json(), groups());
}

#[test]
fn add_folder() {
    let server = MockServer::start();
    let add = server.mock(|when, then| {
        when.method(POST)
            .path("/open/v1/project/group")
            .json_body(json!({"name": "Errands"}));
        then.status(200)
            .json_body(json!({"id": "g3", "name": "Errands"}));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool("add_folder", json!({"name": "Errands"})).ok();
    add.assert();
    assert_eq!(res.json()["id"], "g3");
}

#[test]
fn delete_folders_resolves_names() {
    let server = MockServer::start();
    let _list = mock_groups(&server);
    let g1 = server.mock(|when, then| {
        when.method(DELETE).path("/open/v1/project/group/g1");
        then.status(200);
    });
    let g2 = server.mock(|when, then| {
        when.method(DELETE).path("/open/v1/project/group/g2");
        then.status(200);
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool("delete_folders", json!({"folders": ["home", "Work"]}))
        .ok();
    g1.assert();
    g2.assert();
    assert_eq!(res.text, "Deleted 2 folder(s)");
}

#[test]
fn delete_folders_unknown_name_deletes_nothing() {
    let server = MockServer::start();
    let _list = mock_groups(&server);
    let del = server.mock(|when, then| {
        when.method(DELETE);
        then.status(200);
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool("delete_folders", json!({"folders": ["home", "zzz"]}));
    assert!(res.is_error);
    assert!(res.text.contains("No folder found"), "{}", res.text);
    del.assert_calls(0);
}

#[test]
fn delete_folders_failure_reports_what_was_deleted() {
    let server = MockServer::start();
    let _list = mock_groups(&server);
    let g1 = server.mock(|when, then| {
        when.method(DELETE).path("/open/v1/project/group/g1");
        then.status(200);
    });
    let g2 = server.mock(|when, then| {
        when.method(DELETE).path("/open/v1/project/group/g2");
        then.status(500).body("boom");
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c.call_tool("delete_folders", json!({"folders": ["home", "Work"]}));
    g1.assert();
    g2.assert();
    assert!(res.is_error);
    assert!(
        res.text
            .starts_with("Deleted 1 of 2 folder(s); 'Work' failed: "),
        "{}",
        res.text
    );
}

#[test]
fn rename_folder() {
    let server = MockServer::start();
    let _list = mock_groups(&server);
    let rename = server.mock(|when, then| {
        when.method(POST)
            .path("/open/v1/project/group/g2")
            .json_body(json!({"name": "Job"}));
        then.status(200)
            .json_body(json!({"id": "g2", "name": "Job"}));
    });
    let mut c = McpBuilder::new(&server).start();
    let res = c
        .call_tool(
            "rename_folder",
            json!({"folder": "Work", "new_name": "Job"}),
        )
        .ok();
    rename.assert();
    assert_eq!(res.json()["name"], "Job");
}
