//! v2 batch updates replace the whole record, so edits must resend the fields
//! they don't change.

mod common;

use httpmock::prelude::*;
use serde_json::json;

fn run(server: &MockServer, args: &[&str]) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    common::isolated_command(dir.path())
        .args(args)
        .env("TICKTICK_ACCESS_TOKEN", common::ACCESS_TOKEN)
        .env("TICKTICK_API_BASE", server.url("/open/v1"))
        .env("TICKTICK_V2_BASE", server.url("/api/v2"))
        .env("TICKTICK_V2_SESSION_TOKEN", common::SESSION_TOKEN)
        .output()
        .unwrap()
}

#[test]
fn habit_edit_keeps_unchanged_fields() {
    let server = MockServer::start();
    let list = server.mock(|when, then| {
        when.method(GET).path("/api/v2/habits");
        then.status(200).json_body(json!([{
            "id": "h1", "name": "Water", "etag": "e1", "color": "#1166CC",
            "type": "Real", "goal": 5.0, "unit": "cups",
        }]));
    });
    let update = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v2/habits/batch")
            .json_body(json!({"update": [{
                "id": "h1", "name": "Water", "etag": "e1", "color": "#1166CC",
                "type": "Real", "goal": 6.0, "unit": "cups",
            }]}));
        then.status(200)
            .json_body(json!({"id2etag": {"h1": "e2"}, "id2error": {}}));
    });
    let out = run(&server, &["habit", "edit", "Water", "--goal", "6"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    update.assert();
    list.assert_calls(1);
}

#[test]
fn habit_archive_fetches_the_list_once() {
    let server = MockServer::start();
    let list = server.mock(|when, then| {
        when.method(GET).path("/api/v2/habits");
        then.status(200).json_body(json!([
            {"id": "h1", "name": "Water", "etag": "e1", "goal": 5.0},
            {"id": "h2", "name": "Read", "etag": "e2", "goal": 1.0},
        ]));
    });
    let h1 = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v2/habits/batch")
            .json_body(json!({"update": [
                {"id": "h1", "name": "Water", "etag": "e1", "goal": 5.0, "status": 1},
            ]}));
        then.status(200)
            .json_body(json!({"id2etag": {}, "id2error": {}}));
    });
    let h2 = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v2/habits/batch")
            .json_body(json!({"update": [
                {"id": "h2", "name": "Read", "etag": "e2", "goal": 1.0, "status": 1},
            ]}));
        then.status(200)
            .json_body(json!({"id2etag": {}, "id2error": {}}));
    });
    let out = run(&server, &["habit", "archive", "Water", "Read"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    h1.assert();
    h2.assert();
    list.assert_calls(1);
}

#[test]
fn habit_edit_by_unknown_id_sends_nothing() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/api/v2/habits");
        then.status(200)
            .json_body(json!([{"id": "h1", "name": "Water", "etag": "e1"}]));
    });
    let update = server.mock(|when, then| {
        when.method(POST).path("/api/v2/habits/batch");
        then.status(200).json_body(json!({}));
    });
    let out = run(
        &server,
        &["habit", "edit", "aaaaaaaaaaaaaaaaaaaaaaaa", "--goal", "6"],
    );
    assert!(!out.status.success());
    update.assert_calls(0);
}

#[test]
fn filter_edit_keeps_unchanged_fields() {
    let server = MockServer::start();
    let check = server.mock(|when, then| {
        when.method(GET).path("/api/v2/batch/check/0");
        then.status(200).json_body(json!({"filters": [{
            "id": "f1", "name": "Urgent", "etag": "e1", "rule": "{\"and\":[]}", "sortType": "dueDate",
        }]}));
    });
    let update = server.mock(|when, then| {
        when.method(POST).path("/api/v2/batch/filter").json_body(json!({"update": [{
            "id": "f1", "name": "Urgent", "etag": "e1", "rule": "{\"and\":[]}", "sortType": "priority",
        }]}));
        then.status(200).json_body(json!({"id2etag": {"f1": "e2"}, "id2error": {}}));
    });
    let out = run(
        &server,
        &["filter", "edit", "Urgent", "--sort-type", "priority"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    update.assert();
    check.assert_calls(1);
}

#[test]
fn lists_survive_a_record_with_a_null_name() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/api/v2/batch/check/0");
        then.status(200)
            .json_body(json!({"filters": [{"id": "f1", "name": null, "rule": null}]}));
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/v2/habits");
        then.status(200)
            .json_body(json!([{"id": "h1", "name": null}]));
    });
    for args in [["filter", "list"], ["habit", "list"]] {
        let out = run(&server, &args);
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
}

#[test]
fn habit_list_accepts_a_numeric_target_start_date() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/api/v2/habits");
        then.status(200)
            .json_body(json!([{"id": "h1", "name": "Read", "targetStartDate": 20190525}]));
    });
    let out = run(&server, &["habit", "list"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let habits: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(habits[0]["targetStartDate"], 20190525);
}

#[test]
fn project_get_inbox_needs_no_project_record() {
    let server = MockServer::start();
    let any = server.mock(|when, then| {
        when.any_request();
        then.status(404);
    });
    let out = run(&server, &["project", "get", "inbox123"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let project: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(project["id"], "inbox123");
    assert_eq!(project["name"], "Inbox");
    any.assert_calls(0);
}
