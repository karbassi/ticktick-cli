//! Cases follow the Python server's `test_tasks.py` and friends.
//!
//! The child runs with `TZ=America/Chicago`, so 2027-03-01 is `-0600`.

mod common;

use common::{ACCESS_TOKEN, HEX_PROJECT, McpBuilder, McpClient, SESSION_TOKEN};
use httpmock::prelude::*;
use serde_json::{Value, json};

const V1: &str = "/open/v1";
const V2: &str = "/api/v2";

fn v1(path: &str) -> String {
    format!("{V1}{path}")
}

fn v2(path: &str) -> String {
    format!("{V2}{path}")
}

fn start(server: &MockServer) -> McpClient {
    McpBuilder::new(server).start()
}

fn mock_projects(server: &MockServer, projects: Value) -> httpmock::Mock<'_> {
    server.mock(|when, then| {
        when.method(GET)
            .path(v1("/project"))
            .header("Authorization", format!("Bearer {ACCESS_TOKEN}"));
        then.status(200).json_body(projects);
    })
}

fn mock_create(server: &MockServer, body: Value) -> httpmock::Mock<'_> {
    let mut reply = body.clone();
    reply["id"] = json!("t1");
    server.mock(|when, then| {
        when.method(POST)
            .path(v1("/task"))
            .header("Authorization", format!("Bearer {ACCESS_TOKEN}"))
            .json_body(body);
        then.status(200).json_body(reply);
    })
}

#[test]
fn add_task_timed_gets_at_due_time_reminder() {
    let server = MockServer::start();
    let create = mock_create(
        &server,
        json!({
            "title": "x",
            "projectId": HEX_PROJECT,
            "dueDate": "2027-03-01T10:00:00.000-0600",
            "isAllDay": false,
            "reminders": ["TRIGGER:PT0S"],
        }),
    );
    let mut c = start(&server);
    let res = c
        .call_tool(
            "add_task",
            json!({"title": "x", "project": HEX_PROJECT, "due": "2027-03-01T10:00"}),
        )
        .ok();
    create.assert();
    assert_eq!(res.json()["id"], "t1");
    assert_eq!(res.json()["reminders"], json!(["TRIGGER:PT0S"]));
}

#[test]
fn add_task_all_day_gets_9am_reminder() {
    let server = MockServer::start();
    let create = mock_create(
        &server,
        json!({
            "title": "x",
            "dueDate": "2027-03-01T00:00:00.000-0600",
            "isAllDay": true,
            "reminders": ["TRIGGER:P0DT9H0M0S"],
        }),
    );
    let mut c = start(&server);
    c.call_tool("add_task", json!({"title": "x", "due": "2027-03-01"}))
        .ok();
    create.assert();
}

#[test]
fn add_task_all_day_flag_overrides_default() {
    let server = MockServer::start();
    let create = mock_create(
        &server,
        json!({
            "title": "x",
            "dueDate": "2027-03-01T00:00:00.000-0600",
            "isAllDay": false,
            "reminders": ["TRIGGER:PT0S"],
        }),
    );
    let mut c = start(&server);
    c.call_tool(
        "add_task",
        json!({"title": "x", "due": "2027-03-01", "all_day": false}),
    )
    .ok();
    create.assert();
}

#[test]
fn add_task_all_day_flag_applies_to_start_only_task() {
    let server = MockServer::start();
    let create = mock_create(
        &server,
        json!({
            "title": "x",
            "startDate": "2027-03-01T10:00:00.000-0600",
            "isAllDay": true,
            "reminders": ["TRIGGER:P0DT9H0M0S"],
        }),
    );
    let mut c = start(&server);
    c.call_tool(
        "add_task",
        json!({"title": "x", "start": "2027-03-01T10:00", "all_day": true}),
    )
    .ok();
    create.assert();
}

#[test]
fn add_task_empty_reminders_means_none_and_timezone_is_sent() {
    let server = MockServer::start();
    let create = mock_create(
        &server,
        json!({
            "title": "x",
            "dueDate": "2027-03-01T10:00:00.000-0500",
            "isAllDay": false,
            "timeZone": "America/New_York",
            "reminders": [],
        }),
    );
    let mut c = start(&server);
    c.call_tool(
        "add_task",
        json!({
            "title": "x",
            "due": "2027-03-01T10:00",
            "timezone": "America/New_York",
            "reminders": [],
        }),
    )
    .ok();
    create.assert();
}

#[test]
fn add_task_without_date_sends_no_reminders_and_skips_empty_fields() {
    let server = MockServer::start();
    let create = mock_create(
        &server,
        json!({
            "title": "x",
            "priority": 5,
            "content": "notes",
            "items": [{"title": "a", "status": 0}, {"title": "b", "status": 0}],
        }),
    );
    let mut c = start(&server);
    c.call_tool(
        "add_task",
        json!({
            "title": "x",
            "priority": "High",
            "tags": [],
            "content": "notes",
            "items": ["a", "b"],
        }),
    )
    .ok();
    create.assert();
}

#[test]
fn add_task_duration_from_due_sets_start_and_end() {
    let server = MockServer::start();
    let create = mock_create(
        &server,
        json!({
            "title": "x",
            "startDate": "2027-03-01T10:00:00.000-0600",
            "dueDate": "2027-03-01T11:30:00.000-0600",
            "isAllDay": false,
            "reminders": ["TRIGGER:PT0S"],
        }),
    );
    let mut c = start(&server);
    c.call_tool(
        "add_task",
        json!({"title": "x", "due": "2027-03-01T10:00", "duration": "1h30m"}),
    )
    .ok();
    create.assert();
}

#[test]
fn add_task_duration_past_year_9999_is_an_error() {
    let server = MockServer::start();
    let create = server.mock(|when, then| {
        when.method(POST).path(v1("/task"));
        then.status(200).json_body(json!({"id": "t1"}));
    });
    let mut c = start(&server);
    let res = c.call_tool(
        "add_task",
        json!({"title": "x", "due": "2027-03-01T10:00", "duration": "80000000h"}),
    );
    assert!(res.is_error);
    assert_eq!(res.text, "date value out of range");
    create.assert_calls(0);
}

#[test]
fn add_task_column_resolves_by_name() {
    let server = MockServer::start();
    let columns = server.mock(|when, then| {
        when.method(GET)
            .path(v1(&format!("/project/{HEX_PROJECT}/column")));
        then.status(200).json_body(json!([
            {"id": "c1", "name": "Doing"},
            {"id": "c2", "name": "Done"},
        ]));
    });
    let create = mock_create(
        &server,
        json!({"title": "x", "projectId": HEX_PROJECT, "columnId": "c1"}),
    );
    let mut c = start(&server);
    c.call_tool(
        "add_task",
        json!({"title": "x", "project": HEX_PROJECT, "column": "doing"}),
    )
    .ok();
    columns.assert();
    create.assert();

    let res = c.call_tool("add_task", json!({"title": "x", "column": "doing"}));
    assert!(res.is_error);
    assert_eq!(res.text, "column requires project");
}

#[test]
fn add_task_rejects_bad_input_as_tool_errors() {
    let server = MockServer::start();
    let mut c = start(&server);
    let res = c.call_tool("add_task", json!({"title": "x", "priority": "urgent"}));
    assert!(res.is_error);
    assert_eq!(
        res.text,
        "Invalid priority 'urgent'. Use: none, low, medium, high"
    );
    let res = c.call_tool("add_task", json!({"title": "x", "due": "next week"}));
    assert!(res.is_error, "{}", res.text);
    let res = c.call_tool(
        "add_task",
        json!({"title": "x", "due": "2027-03-01", "duration": "1h"}),
    );
    assert!(res.is_error);
    assert_eq!(
        res.text,
        "Duration requires a date with a time component (use YYYY-MM-DDTHH:MM)"
    );
    // Unknown parameters are rejected, like the Python schema
    // (additionalProperties: false).
    let res = c.call_tool("add_task", json!({"title": "x", "bogus": 1}));
    assert!(res.is_error, "{}", res.text);
}

#[test]
fn add_task_rejects_timezone_paths() {
    let server = MockServer::start();
    let create = server.mock(|when, then| {
        when.method(POST).path(v1("/task"));
        then.status(200).json_body(json!({"id": "t1"}));
    });
    let mut c = start(&server);
    for tz in ["/etc/hosts", "../../etc/hosts", "America/../Europe/London"] {
        let res = c.call_tool(
            "add_task",
            json!({"title": "x", "due": "2027-03-01T10:00", "timezone": tz}),
        );
        assert!(res.is_error, "{tz}: {}", res.text);
        assert_eq!(res.text, format!("Unknown timezone '{tz}'"));
    }
    create.assert_calls(0);
}

#[test]
fn add_and_edit_task_send_the_canonical_timezone_name() {
    let server = MockServer::start();
    let create = mock_create(
        &server,
        json!({
            "title": "x",
            "dueDate": "2027-03-01T10:00:00.000-0500",
            "isAllDay": false,
            "timeZone": "America/New_York",
            "reminders": ["TRIGGER:PT0S"],
        }),
    );
    let edit = server.mock(|when, then| {
        when.method(POST).path(v1("/task/t1")).json_body(json!({
            "taskId": "t1",
            "projectId": HEX_PROJECT,
            "timeZone": "Europe/London",
        }));
        then.status(200).json_body(json!({"id": "t1"}));
    });
    let mut c = start(&server);
    c.call_tool(
        "add_task",
        json!({"title": "x", "due": "2027-03-01T10:00", "timezone": "america/new_york"}),
    )
    .ok();
    c.call_tool(
        "edit_task",
        json!({"task_id": "t1", "project": HEX_PROJECT, "timezone": "EUROPE/LONDON"}),
    )
    .ok();
    create.assert();
    edit.assert();
}

#[test]
fn etc_unknown_timezone_is_rejected() {
    let server = MockServer::start();
    let create = server.mock(|when, then| {
        when.method(POST).path(v1("/task"));
        then.status(200).json_body(json!({"id": "t1"}));
    });
    let mut c = start(&server);
    let res = c.call_tool(
        "add_task",
        json!({"title": "x", "due": "2027-03-01T10:00", "timezone": "Etc/Unknown"}),
    );
    assert!(res.is_error);
    assert_eq!(res.text, "Unknown timezone 'Etc/Unknown'");
    create.assert_calls(0);
}

/// "today" at midnight in `tz`, as the API string.
fn today_midnight_in(tz: &str) -> String {
    let zone = jiff::tz::TimeZone::get(tz).unwrap();
    jiff::Timestamp::now()
        .to_zoned_datetime(zone.clone())
        .date()
        .to_zoned_datetime(zone)
        .unwrap()
        .strftime("%Y-%m-%dT%H:%M:%S.000%z")
        .to_string()
}

#[test]
fn add_task_today_uses_ticktick_timezone_for_date_and_offset() {
    // Pago Pago and Kiritimati are 25h apart, so their dates always differ.
    for (system, env) in [
        ("UTC", "America/Chicago"),
        ("Pacific/Pago_Pago", "Pacific/Kiritimati"),
    ] {
        let server = MockServer::start();
        let create = mock_create(
            &server,
            json!({
                "title": "x",
                "dueDate": today_midnight_in(env),
                "isAllDay": true,
                "reminders": ["TRIGGER:P0DT9H0M0S"],
            }),
        );
        let mut c = McpBuilder::new(&server)
            .env("TZ", system)
            .env("TICKTICK_TIMEZONE", env)
            .start();
        c.call_tool("add_task", json!({"title": "x", "due": "today"}))
            .ok();
        create.assert();
    }
}

#[test]
fn add_task_ignores_config_account_timezone() {
    let server = MockServer::start();
    let create = mock_create(
        &server,
        json!({
            "title": "x",
            "dueDate": "2027-03-01T10:00:00.000-0600",
            "isAllDay": false,
            "reminders": ["TRIGGER:PT0S"],
        }),
    );
    let mut c = McpBuilder::new(&server)
        .config(json!({"account_timezone": "Asia/Tokyo"}))
        .start();
    c.call_tool("add_task", json!({"title": "x", "due": "2027-03-01T10:00"}))
        .ok();
    create.assert();
}

#[test]
fn invalid_ticktick_timezone_errors_unless_timezone_is_given() {
    let server = MockServer::start();
    let create = mock_create(
        &server,
        json!({
            "title": "x",
            "dueDate": "2027-03-01T10:00:00.000-0500",
            "isAllDay": false,
            "timeZone": "America/New_York",
            "reminders": ["TRIGGER:PT0S"],
        }),
    );
    let mut c = McpBuilder::new(&server)
        .env("TICKTICK_TIMEZONE", "Not/AZone")
        .start();
    let res = c.call_tool("add_task", json!({"title": "x", "due": "2027-03-01T10:00"}));
    assert!(res.is_error);
    assert_eq!(res.text, "TICKTICK_TIMEZONE: Unknown timezone 'Not/AZone'");
    create.assert_calls(0);

    c.call_tool(
        "add_task",
        json!({"title": "x", "due": "2027-03-01T10:00", "timezone": "America/New_York"}),
    )
    .ok();
    create.assert();
}

#[test]
fn edit_task_sends_only_passed_fields() {
    let server = MockServer::start();
    mock_projects(&server, json!([{"id": "p1", "name": "Work"}]));
    let title_only = server.mock(|when, then| {
        when.method(POST)
            .path(v1("/task/t1"))
            .json_body(json!({"taskId": "t1", "projectId": "p1", "title": "y"}));
        then.status(200)
            .json_body(json!({"id": "t1", "title": "y"}));
    });
    let clear = server.mock(|when, then| {
        when.method(POST).path(v1("/task/t1")).json_body(json!({
            "taskId": "t1",
            "projectId": "p1",
            "dueDate": null,
            "priority": 0,
            "reminders": [],
        }));
        then.status(200).json_body(json!({"id": "t1"}));
    });
    // A due date sets isAllDay but adds no default reminder on edit.
    let dated = server.mock(|when, then| {
        when.method(POST).path(v1("/task/t1")).json_body(json!({
            "taskId": "t1",
            "projectId": "p1",
            "dueDate": "2027-03-01T10:00:00.000-0600",
            "isAllDay": false,
        }));
        then.status(200).json_body(json!({"id": "t1"}));
    });
    let mut c = start(&server);
    let res = c
        .call_tool(
            "edit_task",
            json!({"task_id": "t1", "project": "Work", "title": "y"}),
        )
        .ok();
    assert_eq!(res.json(), json!({"id": "t1", "title": "y"}));
    c.call_tool(
        "edit_task",
        json!({
            "task_id": "t1",
            "project": "work",
            "clear_due": true,
            "due": "2027-03-01",
            "priority": "none",
            "reminders": [],
        }),
    )
    .ok();
    c.call_tool(
        "edit_task",
        json!({"task_id": "t1", "project": "Work", "due": "2027-03-01T10:00"}),
    )
    .ok();
    title_only.assert();
    clear.assert();
    dated.assert();
}

#[test]
fn complete_task_single_uses_single_endpoint() {
    let server = MockServer::start();
    let single = server.mock(|when, then| {
        when.method(POST)
            .path(v1(&format!("/project/{HEX_PROJECT}/task/t1/complete")));
        then.status(200);
    });
    let mut c = start(&server);
    let res = c
        .call_tool(
            "complete_task",
            json!({"task_id": "t1", "project": HEX_PROJECT}),
        )
        .ok();
    single.assert();
    assert_eq!(res.text, "Task t1 completed");
}

#[test]
fn complete_task_batch_dedupes_chunks_and_reports_failures() {
    let server = MockServer::start();
    let ids: Vec<String> = (0..120).map(|i| format!("t{i}")).collect();
    let chunk_mock = |chunk: &[String], skip: Option<&str>| {
        let done: Vec<&String> = chunk
            .iter()
            .filter(|id| Some(id.as_str()) != skip)
            .collect();
        server.mock(|when, then| {
            when.method(POST)
                .path(v1("/task/completeTasks"))
                .json_body(json!({"projectId": HEX_PROJECT, "taskIds": chunk}));
            then.status(200).json_body(json!(done));
        })
    };
    let first = chunk_mock(&ids[..50], Some("t7"));
    let second = chunk_mock(&ids[50..100], None);
    let third = chunk_mock(&ids[100..], None);

    let mut c = start(&server);
    // task_id comes first; the duplicate t0 in task_ids is dropped.
    let mut task_ids = ids[1..].to_vec();
    task_ids.push("t0".to_string());
    let res = c
        .call_tool(
            "complete_task",
            json!({"project": HEX_PROJECT, "task_id": "t0", "task_ids": task_ids}),
        )
        .ok();
    first.assert();
    second.assert();
    third.assert();
    assert_eq!(res.text, "Completed 119 of 120 tasks; not completed: t7");
}

#[test]
fn complete_task_chunk_failure_reports_it_and_continues() {
    let server = MockServer::start();
    let ids: Vec<String> = (0..120).map(|i| format!("t{i}")).collect();
    let first = server.mock(|when, then| {
        when.method(POST)
            .path(v1("/task/completeTasks"))
            .json_body(json!({"projectId": HEX_PROJECT, "taskIds": &ids[..50]}));
        then.status(200).json_body(json!(
            ids[..50]
                .iter()
                .filter(|id| *id != "t7")
                .collect::<Vec<_>>()
        ));
    });
    let second = server.mock(|when, then| {
        when.method(POST)
            .path(v1("/task/completeTasks"))
            .json_body(json!({"projectId": HEX_PROJECT, "taskIds": &ids[50..100]}));
        then.status(500).body("boom");
    });
    let third = server.mock(|when, then| {
        when.method(POST)
            .path(v1("/task/completeTasks"))
            .json_body(json!({"projectId": HEX_PROJECT, "taskIds": &ids[100..]}));
        then.status(200).json_body(json!(&ids[100..]));
    });
    let mut c = start(&server);
    let res = c.call_tool(
        "complete_task",
        json!({"project": HEX_PROJECT, "task_ids": ids}),
    );
    first.assert();
    second.assert();
    third.assert();
    assert!(!res.is_error);
    let not_completed: Vec<String> = std::iter::once("t7".to_string())
        .chain(ids[50..100].iter().cloned())
        .collect();
    let prefix = format!(
        "Completed 69 of 120 tasks; not completed: {}; chunk 2 failed: ",
        not_completed.join(", ")
    );
    assert!(res.text.starts_with(&prefix), "{}", res.text);
    assert!(res.text.contains("500"), "{}", res.text);
}

#[test]
fn complete_task_is_an_error_when_nothing_completed() {
    let server = MockServer::start();
    let batch = server.mock(|when, then| {
        when.method(POST).path(v1("/task/completeTasks"));
        then.status(200).json_body(json!([]));
    });
    let mut c = start(&server);
    let res = c.call_tool(
        "complete_task",
        json!({"project": HEX_PROJECT, "task_ids": ["a", "b"]}),
    );
    batch.assert();
    assert!(res.is_error);
    assert_eq!(res.text, "Completed 0 of 2 tasks; not completed: a, b");
}

#[test]
fn complete_task_requires_an_id() {
    let server = MockServer::start();
    let mut c = start(&server);
    let res = c.call_tool("complete_task", json!({"project": HEX_PROJECT}));
    assert!(res.is_error);
    assert_eq!(res.text, "Provide task_id or task_ids");
}

#[test]
fn list_tasks_by_fuzzy_project_name() {
    let server = MockServer::start();
    let projects = mock_projects(
        &server,
        json!([{"id": "p1", "name": "Work"}, {"id": "p2", "name": "Personal"}]),
    );
    let tasks = json!([{"id": "t1", "title": "Task 1", "items": [{"title": "sub"}]}]);
    let data = server.mock(|when, then| {
        when.method(GET).path(v1("/project/p2/data"));
        then.status(200)
            .json_body(json!({"project": {"id": "p2"}, "tasks": tasks}));
    });
    let mut c = start(&server);
    let res = c.call_tool("list_tasks", json!({"project": "pers"})).ok();
    projects.assert();
    data.assert();
    assert_eq!(res.json(), tasks);
}

#[test]
fn list_tasks_all_projects_skips_failures_and_includes_inbox() {
    let server = MockServer::start();
    mock_projects(
        &server,
        json!([{"id": "p1", "name": "Work"}, {"id": "p2", "name": "Broken"}]),
    );
    server.mock(|when, then| {
        when.method(GET).path(v1("/project/p1/data"));
        then.status(200).json_body(json!({"tasks": [{"id": "a"}]}));
    });
    server.mock(|when, then| {
        when.method(GET).path(v1("/project/p2/data"));
        then.status(500);
    });
    // Inbox discovery: create a probe task, read its projectId, delete it.
    let probe = server.mock(|when, then| {
        when.method(POST)
            .path(v1("/task"))
            .json_body(json!({"title": "__inbox_probe__"}));
        then.status(200)
            .json_body(json!({"id": "probe", "title": "__inbox_probe__", "projectId": "inbox123"}));
    });
    let cleanup = server.mock(|when, then| {
        when.method(DELETE).path(v1("/project/inbox123/task/probe"));
        then.status(200);
    });
    server.mock(|when, then| {
        when.method(GET).path(v1("/project/inbox123/data"));
        then.status(200).json_body(json!({"tasks": [{"id": "b"}]}));
    });
    let mut c = start(&server);
    let res = c.call_tool("list_tasks", json!({})).ok();
    assert_eq!(res.json(), json!([{"id": "a"}, {"id": "b"}]));
    probe.assert();
    cleanup.assert();

    // The inbox ID is cached for the rest of the process.
    let res = c.call_tool("list_tasks", json!({"project": "inbox"})).ok();
    assert_eq!(res.json(), json!([{"id": "b"}]));
    probe.assert_calls(1);
}

#[test]
fn list_tasks_all_projects_keeps_project_order_when_fetched_concurrently() {
    let server = MockServer::start();
    let ids: Vec<String> = (0..10).map(|i| format!("p{i}")).collect();
    mock_projects(
        &server,
        json!(ids.iter().map(|id| json!({"id": id})).collect::<Vec<_>>()),
    );
    // Earlier projects answer slower, so completion order is the reverse of project order.
    for (i, id) in ids.iter().enumerate() {
        server.mock(|when, then| {
            when.method(GET).path(v1(&format!("/project/{id}/data")));
            then.status(200)
                .delay(std::time::Duration::from_millis(20 * (10 - i as u64)))
                .json_body(json!({"tasks": [{"id": format!("t{i}")}]}));
        });
    }
    server.mock(|when, then| {
        when.method(POST).path(v1("/task"));
        then.status(200)
            .json_body(json!({"id": "probe", "title": "__inbox_probe__", "projectId": "inbox123"}));
    });
    server.mock(|when, then| {
        when.method(DELETE).path(v1("/project/inbox123/task/probe"));
        then.status(200);
    });
    server.mock(|when, then| {
        when.method(GET).path(v1("/project/inbox123/data"));
        then.status(200)
            .json_body(json!({"tasks": [{"id": "inbox"}]}));
    });
    let mut c = start(&server);
    let started = std::time::Instant::now();
    let res = c.call_tool("list_tasks", json!({})).ok();
    // Serial fetches would take 1.1s of delays; two waves of parallel ones about 0.24s.
    assert!(started.elapsed() < std::time::Duration::from_millis(900));
    let mut want: Vec<Value> = (0..10).map(|i| json!({"id": format!("t{i}")})).collect();
    want.push(json!({"id": "inbox"}));
    assert_eq!(res.json(), Value::Array(want));
}

#[test]
fn move_task_by_names_fetches_projects_once() {
    let server = MockServer::start();
    let projects = mock_projects(
        &server,
        json!([{"id": "p1", "name": "Work"}, {"id": "p2", "name": "Home"}]),
    );
    let mv = server.mock(|when, then| {
        when.method(POST).path(v1("/task/move")).json_body(json!([{
            "taskId": "t1",
            "fromProjectId": "p1",
            "toProjectId": "p2",
        }]));
        then.status(200).json_body(json!([{"id": "t1"}]));
    });
    let mut c = start(&server);
    c.call_tool(
        "move_task",
        json!({"task_id": "t1", "from_project": "work", "to_project": "home"}),
    )
    .ok();
    mv.assert();
    projects.assert_calls(1);
}

#[test]
fn list_tasks_completed_uses_v2() {
    let server = MockServer::start();
    let completed = server.mock(|when, then| {
        when.method(GET)
            .path(v2("/project/all/completedInAll/"))
            .query_param("limit", "50")
            .header("Cookie", format!("t={SESSION_TOKEN}"));
        then.status(200)
            .json_body(json!([{"id": "t2", "title": "Done", "status": 2}]));
    });
    let mut c = start(&server);
    let res = c
        .call_tool("list_tasks", json!({"status": "completed", "limit": 50}))
        .ok();
    completed.assert();
    assert_eq!(res.json()[0]["status"], 2);
}

#[test]
fn project_resolution_errors_match_python() {
    let server = MockServer::start();
    mock_projects(
        &server,
        json!([
            {"id": "p1", "name": "Work"},
            {"id": "p2", "name": "Personal"},
            {"id": "p3", "name": "Work Archive"},
        ]),
    );
    let mut c = start(&server);
    let res = c.call_tool("get_project", json!({"project": "Persnal"}));
    assert!(res.is_error);
    assert_eq!(
        res.text,
        "No project found matching 'Persnal'. Did you mean 'Personal'?"
    );
    let res = c.call_tool("get_project", json!({"project": "wor"}));
    assert!(res.is_error);
    assert_eq!(
        res.text,
        "Multiple projects match 'wor': Work, Work Archive. Use a more specific name or the full ID."
    );
}

#[test]
fn move_task_posts_v1_move() {
    let server = MockServer::start();
    let mv = server.mock(|when, then| {
        when.method(POST).path(v1("/task/move")).json_body(json!([{
            "taskId": "t1",
            "fromProjectId": "inbox123",
            "toProjectId": HEX_PROJECT,
        }]));
        then.status(200).json_body(json!([{"id": "t1"}]));
    });
    let mut c = start(&server);
    let res = c
        .call_tool(
            "move_task",
            json!({"task_id": "t1", "from_project": "inbox123", "to_project": HEX_PROJECT}),
        )
        .ok();
    mv.assert();
    assert_eq!(res.text, format!("Task t1 moved to {HEX_PROJECT}"));
}

#[test]
fn subtask_and_unparent() {
    let server = MockServer::start();
    let parent = server.mock(|when, then| {
        when.method(POST)
            .path(v2("/batch/taskParent"))
            .header("Cookie", format!("t={SESSION_TOKEN}"))
            .json_body(json!([{"taskId": "t2", "parentId": "t1", "projectId": "inbox1"}]));
        then.status(200).json_body(json!({"ok": true}));
    });
    let unparent = server.mock(|when, then| {
        when.method(POST)
            .path(v1("/task/t2"))
            .json_body(json!({"taskId": "t2", "projectId": "inbox1", "parentId": ""}));
        then.status(200).json_body(json!({"id": "t2"}));
    });
    let mut c = start(&server);
    let res = c
        .call_tool(
            "set_subtask",
            json!({"task_id": "t2", "parent_id": "t1", "project": "inbox1"}),
        )
        .ok();
    assert_eq!(res.text, "Task t2 is now a subtask of t1");
    c.call_tool(
        "unparent_task",
        json!({"task_id": "t2", "project": "inbox1"}),
    )
    .ok();
    parent.assert();
    unparent.assert();
}

#[test]
fn assign_failure_is_a_tool_error_with_hint() {
    let server = MockServer::start();
    let assign = server.mock(|when, then| {
        when.method(POST)
            .path(v1("/task/assign"))
            .json_body(json!({"projectId": "inbox1", "taskId": "t1", "assigneeUsername": "x"}));
        then.status(500).json_body(json!({}));
    });
    let mut c = start(&server);
    let res = c.call_tool(
        "assign_task",
        json!({"task_id": "t1", "project": "inbox1", "assignee": "x"}),
    );
    assign.assert();
    assert!(res.is_error);
    assert_eq!(
        res.text,
        "Assign failed (500). The assignee must be a member of a shared project; check list_project_members."
    );
}

#[test]
fn assign_not_found_passes_the_api_error_through() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path(v1("/task/assign"));
        then.status(404).body("task not found");
    });
    let mut c = start(&server);
    let res = c.call_tool(
        "assign_task",
        json!({"task_id": "t1", "project": "inbox1", "assignee": "x"}),
    );
    assert!(res.is_error);
    assert!(res.text.contains("404"), "{}", res.text);
    assert!(!res.text.contains("must be a member"), "{}", res.text);
}

#[test]
fn api_error_surfaces_as_is_error_not_protocol_error() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path(v1(&format!("/project/{HEX_PROJECT}/task/t1")));
        then.status(404).body("not found");
    });
    let mut c = start(&server);
    let res = c.call_tool("get_task", json!({"task_id": "t1", "project": HEX_PROJECT}));
    assert!(res.is_error);
    assert!(res.text.contains("404"), "{}", res.text);
    let res = c.call_tool("complete_task", json!({"project": HEX_PROJECT}));
    assert!(res.is_error);
}

#[test]
fn stalled_request_times_out_as_a_tool_error() {
    let server = MockServer::start();
    let stalled = server.mock(|when, then| {
        when.method(GET)
            .path(v1(&format!("/project/{HEX_PROJECT}/data")));
        then.status(200)
            .delay(std::time::Duration::from_secs(10))
            .json_body(json!({"tasks": []}));
    });
    let mut c = McpBuilder::new(&server)
        .env("TICKTICK_HTTP_TIMEOUT_SECS", "1")
        .start();
    let started = std::time::Instant::now();
    let res = c.call_tool("list_tasks", json!({"project": HEX_PROJECT}));
    let elapsed = started.elapsed();
    stalled.assert();
    assert!(res.is_error, "{}", res.text);
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "took {elapsed:?}"
    );
}

#[test]
fn refreshed_token_replaces_the_stale_env_token_for_later_calls() {
    let server = MockServer::start();
    let stale = server.mock(|when, then| {
        when.method(GET)
            .path(v1("/project"))
            .header("Authorization", format!("Bearer {ACCESS_TOKEN}"));
        then.status(401);
    });
    let refresh = server.mock(|when, then| {
        when.method(POST)
            .path("/oauth/token")
            .header("Authorization", "Basic Y2lkOnNlYw==")
            .body("grant_type=refresh_token&refresh_token=r1");
        then.status(200)
            .json_body(json!({"access_token": "fresh", "refresh_token": "r2"}));
    });
    let fresh = server.mock(|when, then| {
        when.method(GET)
            .path(v1("/project"))
            .header("Authorization", "Bearer fresh");
        then.status(200).json_body(json!([]));
    });
    // An unwritable config dir: the refresh must not depend on config.json.
    let mut c = McpBuilder::new(&server)
        .env("TICKTICK_REFRESH_TOKEN", "r1")
        .env("TICKTICK_CLIENT_ID", "cid")
        .env("TICKTICK_CLIENT_SECRET", "sec")
        .env("TICKTICK_OAUTH_TOKEN_URL", &server.url("/oauth/token"))
        .env("XDG_CONFIG_HOME", "/dev/null/config")
        .start();
    c.call_tool("list_projects", json!({})).ok();
    c.call_tool("list_projects", json!({})).ok();
    stale.assert_calls(1);
    refresh.assert_calls(1);
    fresh.assert_calls(2);
}

#[test]
fn v2_tool_uses_env_session_token_over_config() {
    let server = MockServer::start();
    let trash = server.mock(|when, then| {
        when.method(GET)
            .path(v2("/project/all/trash/page"))
            .header("Cookie", format!("t={SESSION_TOKEN}"))
            .header_exists("x-device");
        then.status(200)
            .json_body(json!({"tasks": [{"id": "t1", "title": "Trashed"}]}));
    });
    let mut c = McpBuilder::new(&server)
        .config(json!({
            "access_token": null,
            "refresh_token": null,
            "v2_session_token": "stale-file-cookie",
        }))
        .start();
    let res = c.call_tool("list_trash", json!({})).ok();
    trash.assert();
    assert_eq!(res.json(), json!([{"id": "t1", "title": "Trashed"}]));
}

#[test]
fn v2_tool_without_session_token_is_a_tool_error() {
    let server = MockServer::start();
    let mut c = McpBuilder::new(&server)
        .env("TICKTICK_V2_SESSION_TOKEN", "")
        .start();
    let res = c.call_tool("sync_account", json!({}));
    assert!(res.is_error);
    assert!(
        res.text.contains("TICKTICK_V2_SESSION_TOKEN"),
        "{}",
        res.text
    );
}

#[test]
fn project_tools() {
    let server = MockServer::start();
    mock_projects(&server, json!([{"id": "p1", "name": "Work"}]));
    server.mock(|when, then| {
        when.method(GET).path(v1("/project/group"));
        then.status(200)
            .json_body(json!([{"id": "g1", "name": "Home", "etag": "e1"}]));
    });
    let add = server.mock(|when, then| {
        when.method(POST).path(v1("/project")).json_body(json!({
            "name": "New",
            "viewMode": "kanban",
            "groupId": "g1",
        }));
        then.status(200)
            .json_body(json!({"id": "p9", "name": "New"}));
    });
    let edit = server.mock(|when, then| {
        when.method(POST)
            .path(v1("/project/p1"))
            .json_body(json!({"id": "p1", "name": "Job", "groupId": "NONE"}));
        then.status(200)
            .json_body(json!({"id": "p1", "name": "Job"}));
    });
    let members = server.mock(|when, then| {
        when.method(GET).path(v1("/project/p1/members"));
        then.status(200)
            .json_body(json!([{"username": "me", "self": true}]));
    });
    let delete = server.mock(|when, then| {
        when.method(DELETE).path(v1("/project/p1"));
        then.status(200);
    });
    let mut c = start(&server);
    let res = c
        .call_tool(
            "add_project",
            json!({"name": "New", "view_mode": "kanban", "folder": "home"}),
        )
        .ok();
    assert_eq!(res.json()["id"], "p9");
    c.call_tool(
        "edit_project",
        json!({"project": "work", "name": "Job", "folder": "home", "remove_folder": true}),
    )
    .ok();
    let res = c
        .call_tool("list_project_members", json!({"project": "Work"}))
        .ok();
    assert_eq!(res.json()[0]["username"], "me");
    let res = c
        .call_tool("delete_project", json!({"project": "work"}))
        .ok();
    assert_eq!(res.text, "Project work deleted");
    add.assert();
    edit.assert();
    members.assert();
    delete.assert();
}

#[test]
fn tag_batch_failure_reports_what_succeeded_and_stops() {
    let server = MockServer::start();
    let create = |name: &'static str, status: u16| {
        server.mock(move |when, then| {
            when.method(POST)
                .path(v1("/tag"))
                .json_body(json!({"label": name, "name": name}));
            then.status(status).json_body(json!({"name": name}));
        })
    };
    let (a, b, z) = (create("a", 200), create("b", 500), create("z", 200));
    let delete = |name: &'static str, status: u16| {
        server.mock(move |when, then| {
            when.method(DELETE)
                .path(v1("/tag/delete"))
                .json_body(json!({"name": name}));
            then.status(status);
        })
    };
    let (da, db, dz) = (delete("a", 200), delete("b", 500), delete("z", 200));
    let mut c = start(&server);

    let res = c.call_tool("add_tags", json!({"names": ["a", "b", "z"]}));
    assert!(res.is_error);
    assert!(
        res.text.starts_with("Created 1 of 3 tag(s); 'b' failed: "),
        "{}",
        res.text
    );
    let res = c.call_tool("delete_tags", json!({"names": ["a", "b", "z"]}));
    assert!(res.is_error);
    assert!(
        res.text.starts_with("Deleted 1 of 3 tag(s); 'b' failed: "),
        "{}",
        res.text
    );
    for m in [&a, &b, &da, &db] {
        m.assert();
    }
    z.assert_calls(0);
    dz.assert_calls(0);
}

#[test]
fn tag_tools() {
    let server = MockServer::start();
    let create = |name: &'static str| {
        server.mock(move |when, then| {
            when.method(POST)
                .path(v1("/tag"))
                .json_body(json!({"label": name, "name": name}));
            then.status(200).json_body(json!({"name": name}));
        })
    };
    let (a, b) = (create("a"), create("b"));
    let delete = server.mock(|when, then| {
        when.method(DELETE)
            .path(v1("/tag/delete"))
            .query_param("scope", "0")
            .json_body(json!({"name": "old"}));
        then.status(200);
    });
    let rename = server.mock(|when, then| {
        when.method(PUT)
            .path(v1("/tag/rename"))
            .query_param("scope", "0")
            .json_body(json!({"name": "a", "newName": "z"}));
        then.status(200);
    });
    let edit = server.mock(|when, then| {
        when.method(PUT)
            .path(v1("/tag"))
            .json_body(json!({"name": "a", "color": "#FF0000", "parent": "", "sortOrder": 3}));
        then.status(200)
            .json_body(json!({"name": "a", "color": "#FF0000"}));
    });
    let merge = server.mock(|when, then| {
        when.method(PUT)
            .path(v2("/tag/merge"))
            .header("Cookie", format!("t={SESSION_TOKEN}"))
            .json_body(json!({"name": "a", "newName": "b"}));
        then.status(200);
    });
    let mut c = start(&server);
    let res = c.call_tool("add_tags", json!({"names": ["a", "b"]})).ok();
    assert_eq!(res.json(), json!([{"name": "a"}, {"name": "b"}]));
    let res = c.call_tool("delete_tags", json!({"names": ["old"]})).ok();
    assert_eq!(res.text, "Deleted 1 tag(s)");
    let res = c
        .call_tool("rename_tag", json!({"old_name": "a", "new_name": "z"}))
        .ok();
    assert_eq!(res.text, "Tag 'a' renamed to 'z'");
    c.call_tool(
        "edit_tag",
        json!({"name": "a", "color": "#FF0000", "parent": "p", "clear_parent": true, "sort_order": 3}),
    )
    .ok();
    let res = c
        .call_tool("merge_tags", json!({"source": "a", "target": "b"}))
        .ok();
    assert_eq!(res.text, "Tag 'a' merged into 'b'");
    for m in [a, b, delete, rename, edit, merge] {
        m.assert();
    }
}

#[test]
fn profile_resource_merges_status() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path(v2("/user/profile"))
            .header("Cookie", format!("t={SESSION_TOKEN}"));
        then.status(200).json_body(json!({"username": "me"}));
    });
    server.mock(|when, then| {
        when.method(GET).path(v2("/user/status"));
        then.status(200).json_body(json!({"pro": true}));
    });
    let mut c = start(&server);
    assert_eq!(
        c.read_resource("ticktick://profile"),
        json!({"username": "me", "status": {"pro": true}})
    );
    let resp = c.request("resources/read", json!({"uri": "ticktick://nope"}));
    assert!(resp.get("error").is_some(), "{resp}");
}
