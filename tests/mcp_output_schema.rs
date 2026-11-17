//! FastMCP parity for `outputSchema` and `structuredContent`. The structured
//! result of every call is checked in `common::McpClient::call_tool`.

mod common;

use common::{HEX_PROJECT, start, v1, v2};
use httpmock::prelude::*;
use serde_json::{Value, json};

#[test]
fn tools_list_declares_the_fastmcp_output_schemas() {
    let server = MockServer::start();
    let mut c = start(&server);
    let schemas = common::output_schemas();
    let tools = c.list_tools();
    for tool in &tools {
        let name = tool["name"].as_str().unwrap();
        let expected = schemas.get(name).cloned().unwrap_or(Value::Null);
        assert_eq!(tool["outputSchema"], expected, "{name}");
    }
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for name in schemas.keys() {
        assert!(
            names.contains(&name.as_str()),
            "schema for unlisted tool {name}"
        );
    }
}

fn get(server: &MockServer, path: String, body: Value) {
    server.mock(|when, then| {
        when.method(GET).path(path);
        then.status(200).json_body(body);
    });
}

#[test]
fn list_tools_return_wrapped_lists() {
    let server = MockServer::start();
    get(&server, v1("/tag"), json!([{"name": "work"}]));
    get(
        &server,
        v1("/project/group"),
        json!([{"id": "g1", "name": "Home"}]),
    );
    get(&server, v1("/countdown"), json!([{"id": "c1"}]));
    get(
        &server,
        v2("/habits"),
        json!([{"id": "h1", "name": "Read"}]),
    );
    let mut c = start(&server);
    for tool in [
        "list_tags",
        "list_folders",
        "list_countdowns",
        "list_habits",
    ] {
        c.call_tool(tool, json!({})).ok();
    }
}

#[test]
fn delete_task_wraps_its_message() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(DELETE)
            .path(v1(&format!("/project/{HEX_PROJECT}/task/t1")));
        then.status(200);
    });
    let mut c = start(&server);
    let res = c
        .call_tool(
            "delete_task",
            json!({"project": HEX_PROJECT, "task_id": "t1"}),
        )
        .ok();
    assert_eq!(res.text, "Task t1 deleted");
}

#[test]
fn output_that_breaks_the_schema_is_a_tool_error() {
    let server = MockServer::start();
    get(&server, v1("/tag"), json!({"unexpected": "object"}));
    let mut c = start(&server);
    let res = c.call_tool("list_tags", json!({}));
    assert!(res.is_error, "{}", res.text);
    assert!(
        res.text.contains("does not match its output schema"),
        "{}",
        res.text
    );
}

#[test]
fn tools_without_a_schema_send_no_structured_content() {
    let server = MockServer::start();
    let mut c = start(&server);
    // Checked by call_tool: an unknown tool's error has no structuredContent.
    assert!(c.call_tool("no_such_tool", json!({})).is_error);
}
