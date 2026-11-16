//! Ported from Python `test_focus.py`.

mod common;

use common::{ACCESS_TOKEN, McpBuilder};
use httpmock::prelude::*;
use serde_json::{Value, json};

fn focus_chunk<'a>(
    server: &'a MockServer,
    from: &str,
    to: &str,
    focus_type: u8,
    records: Value,
) -> httpmock::Mock<'a> {
    server.mock(|when, then| {
        when.method(GET)
            .path("/open/v1/focus")
            .header("Authorization", format!("Bearer {ACCESS_TOKEN}"))
            .query_param("from", from)
            .query_param("to", to)
            .query_param("type", focus_type.to_string());
        then.status(200).json_body(records);
    })
}

fn mock_save<'a>(server: &'a MockServer, body: Value) -> httpmock::Mock<'a> {
    server.mock(|when, then| {
        when.method(POST)
            .path("/open/v1/focus")
            .header("Authorization", format!("Bearer {ACCESS_TOKEN}"))
            .json_body(body);
        then.status(200).json_body(json!({"id": "f1"}));
    })
}

#[test]
fn log_queries_both_types_in_30_day_chunks() {
    let server = MockServer::start();
    // 2026-01-01 .. 2026-02-14 is 45 days: 2 chunks x 2 types.
    let ends = [
        (
            "2026-01-01T00:00:00.000+0000",
            "2026-01-31T00:00:00.000+0000",
        ),
        (
            "2026-01-31T00:00:00.000+0000",
            "2026-02-14T23:59:59.999+0000",
        ),
    ];
    let mut mocks = Vec::new();
    for (from, to) in ends {
        for focus_type in [0u8, 1] {
            // Each type returns one record, whose id repeats in both chunks.
            let record = json!([{
                "id": format!("f{focus_type}"),
                "startTime": format!("2026-0{}-01", focus_type + 1),
            }]);
            mocks.push(focus_chunk(&server, from, to, focus_type, record));
        }
    }
    let v2_pomodoros = server.mock(|when, then| {
        when.path_includes("/pomodoros");
        then.status(200).json_body(json!([]));
    });

    let mut mcp = McpBuilder::new(&server).start();
    let res = mcp
        .call_tool(
            "focus_log",
            json!({"from_date": "2026-01-01", "to_date": "2026-02-14"}),
        )
        .ok();

    for m in &mocks {
        m.assert();
    }
    v2_pomodoros.assert_calls(0);
    let ids: Vec<Value> = res
        .json()
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].clone())
        .collect();
    assert_eq!(ids, vec![json!("f0"), json!("f1")]);
}

#[test]
fn log_fetches_both_types_concurrently() {
    let server = MockServer::start();
    for focus_type in ["0", "1"] {
        server.mock(|when, then| {
            when.method(GET)
                .path("/open/v1/focus")
                .query_param("type", focus_type);
            then.status(200)
                .json_body(json!([]))
                .delay(std::time::Duration::from_millis(600));
        });
    }
    let mut mcp = McpBuilder::new(&server).start();
    let started = std::time::Instant::now();
    mcp.call_tool(
        "focus_log",
        json!({"from_date": "2026-01-01", "to_date": "2026-01-01"}),
    )
    .ok();
    // Serial fetches would take 1.2s.
    assert!(started.elapsed() < std::time::Duration::from_millis(1100));
}

#[test]
fn log_keeps_records_without_id() {
    let server = MockServer::start();
    let focus = server.mock(|when, then| {
        when.method(GET).path("/open/v1/focus");
        then.status(200)
            .json_body(json!([{"startTime": "2026-01-02", "note": "x"}]));
    });
    let mut mcp = McpBuilder::new(&server).start();
    let res = mcp
        .call_tool(
            "focus_log",
            json!({"from_date": "2026-01-01", "to_date": "2026-01-02"}),
        )
        .ok();
    focus.assert_calls(2);
    let notes: Vec<Value> = res
        .json()
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["note"].clone())
        .collect();
    assert_eq!(notes, vec![json!("x"), json!("x")]);
}

#[test]
fn log_sorts_by_start_and_keeps_last_duplicate() {
    let server = MockServer::start();
    let type0 = server.mock(|when, then| {
        when.method(GET)
            .path("/open/v1/focus")
            .query_param("type", "0");
        then.status(200).json_body(json!([
            {"id": "b", "startTime": "2026-01-03", "v": "old"},
            {"id": "a", "startTime": "2026-01-01"},
        ]));
    });
    let type1 = server.mock(|when, then| {
        when.method(GET)
            .path("/open/v1/focus")
            .query_param("type", "1");
        then.status(200)
            .json_body(json!([{"id": "b", "startTime": "2026-01-03", "v": "new"}]));
    });
    let mut mcp = McpBuilder::new(&server).start();
    let res = mcp
        .call_tool(
            "focus_log",
            json!({"from_date": "2026-01-01", "to_date": "2026-01-02"}),
        )
        .ok();
    type0.assert();
    type1.assert();
    assert_eq!(
        res.json(),
        json!([
            {"id": "a", "startTime": "2026-01-01"},
            {"id": "b", "startTime": "2026-01-03", "v": "new"},
        ])
    );
}

#[test]
fn log_accepts_today_and_yesterday() {
    let server = MockServer::start();
    let focus = server.mock(|when, then| {
        when.method(GET).path("/open/v1/focus");
        then.status(200).json_body(json!([]));
    });
    let mut mcp = McpBuilder::new(&server).start();
    let res = mcp
        .call_tool(
            "focus_log",
            json!({"from_date": " Yesterday ", "to_date": "TODAY"}),
        )
        .ok();
    focus.assert_calls(2);
    assert_eq!(res.json(), json!([]));
}

#[test]
fn log_rejects_invalid_dates() {
    let server = MockServer::start();
    let mut mcp = McpBuilder::new(&server).start();
    let res = mcp.call_tool(
        "focus_log",
        json!({"from_date": "01/02/2026", "to_date": "2026-01-02"}),
    );
    assert!(res.is_error);
    assert_eq!(res.text, "Invalid date '01/02/2026'");

    let res = mcp.call_tool(
        "focus_log",
        json!({"from_date": "2026-01-01", "to_date": "2026-02-30"}),
    );
    assert!(res.is_error);
    assert_eq!(res.text, "day is out of range for month");
}

#[test]
fn log_with_reversed_range_is_empty() {
    let server = MockServer::start();
    let mut mcp = McpBuilder::new(&server).start();
    let res = mcp
        .call_tool(
            "focus_log",
            json!({"from_date": "2026-02-02", "to_date": "2026-01-01"}),
        )
        .ok();
    assert_eq!(res.json(), json!([]));
}

#[test]
fn save_basic() {
    let server = MockServer::start();
    let save = mock_save(
        &server,
        json!({
            "type": 0,
            "startTime": "2026-02-22T14:00:00.000+0000",
            "endTime": "2026-02-22T14:25:00.000+0000",
            "status": 1,
            "pauseDuration": 0,
            "note": "",
        }),
    );
    let mut mcp = McpBuilder::new(&server).start();
    mcp.call_tool(
        "focus_save",
        json!({"duration_minutes": 25, "end": "2026-02-22T14:25:00+00:00"}),
    )
    .ok();
    save.assert();
}

#[test]
fn save_with_task() {
    let server = MockServer::start();
    let save = mock_save(
        &server,
        json!({
            "type": 0,
            "startTime": "2026-02-22T14:00:00.000+0000",
            "endTime": "2026-02-22T14:25:00.000+0000",
            "status": 1,
            "pauseDuration": 0,
            "note": "Deep work session",
            "taskId": "task1",
            "tasks": [{
                "taskId": "task1",
                "startTime": "2026-02-22T14:00:00.000+0000",
                "endTime": "2026-02-22T14:25:00.000+0000",
                "projectId": "proj1",
            }],
        }),
    );
    let mut mcp = McpBuilder::new(&server).start();
    mcp.call_tool(
        "focus_save",
        json!({
            "duration_minutes": 25,
            "end": "2026-02-22T14:25:00+00:00",
            "note": "Deep work session",
            "task_id": "task1",
            "project_id": "proj1",
        }),
    )
    .ok();
    save.assert();
}

#[test]
fn save_with_project_only() {
    let server = MockServer::start();
    // No top-level taskId, and no taskId in the task entry.
    let save = mock_save(
        &server,
        json!({
            "type": 0,
            "startTime": "2026-02-22T14:00:00.000+0000",
            "endTime": "2026-02-22T14:25:00.000+0000",
            "status": 1,
            "pauseDuration": 0,
            "note": "",
            "tasks": [{
                "startTime": "2026-02-22T14:00:00.000+0000",
                "endTime": "2026-02-22T14:25:00.000+0000",
                "projectId": "proj1",
            }],
        }),
    );
    let mut mcp = McpBuilder::new(&server).start();
    mcp.call_tool(
        "focus_save",
        json!({
            "duration_minutes": 25,
            "end": "2026-02-22T14:25:00+00:00",
            "project_id": "proj1",
        }),
    )
    .ok();
    save.assert();
}

#[test]
fn save_empty_task_and_project_are_ignored() {
    let server = MockServer::start();
    let save = mock_save(
        &server,
        json!({
            "type": 0,
            "startTime": "2026-02-22T14:00:00.000+0000",
            "endTime": "2026-02-22T14:25:00.000+0000",
            "status": 1,
            "pauseDuration": 0,
            "note": "",
        }),
    );
    let mut mcp = McpBuilder::new(&server).start();
    mcp.call_tool(
        "focus_save",
        json!({
            "duration_minutes": 25,
            "end": "2026-02-22T14:25:00Z",
            "task_id": "",
            "project_id": null,
        }),
    )
    .ok();
    save.assert();
}

#[test]
fn save_naive_end_is_local_time() {
    let server = MockServer::start();
    // TZ is America/Chicago (CST, -06:00 in February): 08:25 local = 14:25Z.
    let save = mock_save(
        &server,
        json!({
            "type": 0,
            "startTime": "2026-02-22T14:00:00.000+0000",
            "endTime": "2026-02-22T14:25:00.000+0000",
            "status": 1,
            "pauseDuration": 0,
            "note": "",
        }),
    );
    let mut mcp = McpBuilder::new(&server).start();
    mcp.call_tool(
        "focus_save",
        json!({"duration_minutes": 25, "end": "2026-02-22T08:25:00"}),
    )
    .ok();
    save.assert();
}

fn naive_save_end(config: Value, env: Option<&str>, end: &str) {
    let server = MockServer::start();
    let save = mock_save(
        &server,
        json!({
            "type": 0,
            "startTime": "2026-02-22T14:00:00.000+0000",
            "endTime": "2026-02-22T14:25:00.000+0000",
            "status": 1,
            "pauseDuration": 0,
            "note": "",
        }),
    );
    let mut mcp = McpBuilder::new(&server).config(config);
    if let Some(tz) = env {
        mcp = mcp.env("TICKTICK_TIMEZONE", tz);
    }
    mcp.start()
        .call_tool("focus_save", json!({"duration_minutes": 25, "end": end}))
        .ok();
    save.assert();
}

#[test]
fn save_naive_end_follows_ticktick_timezone() {
    // Asia/Tokyo is +09:00, so 23:25 there is 14:25Z (the system zone would give 05:25Z).
    naive_save_end(json!({}), Some("Asia/Tokyo"), "2026-02-22T23:25:00");
}

#[test]
fn save_naive_end_ignores_config_account_timezone() {
    naive_save_end(
        json!({"account_timezone": "Asia/Tokyo"}),
        None,
        "2026-02-22T08:25:00",
    );
}

#[test]
fn save_naive_end_with_invalid_ticktick_timezone_is_an_error() {
    let server = MockServer::start();
    let save = server.mock(|when, then| {
        when.any_request();
        then.status(200).json_body(json!({}));
    });
    let mut mcp = McpBuilder::new(&server)
        .env("TICKTICK_TIMEZONE", "Not/AZone")
        .start();
    let res = mcp.call_tool(
        "focus_save",
        json!({"duration_minutes": 25, "end": "2026-02-22T08:25:00"}),
    );
    assert!(res.is_error);
    assert_eq!(res.text, "TICKTICK_TIMEZONE: Unknown timezone 'Not/AZone'");
    save.assert_calls(0);
}

#[test]
fn save_offset_and_fraction_are_converted_to_utc() {
    let server = MockServer::start();
    let save = mock_save(
        &server,
        json!({
            "type": 0,
            "startTime": "2026-02-22T14:29:00.123+0000",
            "endTime": "2026-02-22T14:30:00.123+0000",
            "status": 1,
            "pauseDuration": 0,
            "note": "",
        }),
    );
    let mut mcp = McpBuilder::new(&server).start();
    mcp.call_tool(
        "focus_save",
        json!({"duration_minutes": 1, "end": "2026-02-22 09:30:00.123456-05:00"}),
    )
    .ok();
    save.assert();
}

#[test]
fn save_rejects_short_duration() {
    let server = MockServer::start();
    let save = server.mock(|when, then| {
        when.method(POST).path("/open/v1/focus");
        then.status(200).json_body(json!({}));
    });
    let mut mcp = McpBuilder::new(&server).start();
    let res = mcp.call_tool("focus_save", json!({"duration_minutes": 0}));
    assert!(res.is_error);
    assert_eq!(res.text, "duration_minutes must be >= 1");
    save.assert_calls(0);
}

#[test]
fn save_rejects_invalid_end() {
    let server = MockServer::start();
    let save = server.mock(|when, then| {
        when.method(POST).path("/open/v1/focus");
        then.status(200).json_body(json!({}));
    });
    let mut mcp = McpBuilder::new(&server).start();
    for end in ["soon", "2026-02-22T25:00:00", "2026-13-01"] {
        let res = mcp.call_tool("focus_save", json!({"duration_minutes": 5, "end": end}));
        assert!(res.is_error);
        assert_eq!(
            res.text,
            format!(
                "Invalid end time '{end}'. Use 'now' or ISO format like '2026-02-22T14:30:00'."
            )
        );
    }
    save.assert_calls(0);
}

#[test]
fn log_today_is_taken_in_ticktick_timezone() {
    // Pago Pago and Kiritimati are 25h apart, so their dates always differ.
    let zone = jiff::tz::TimeZone::get("Pacific/Kiritimati").unwrap();
    let today = jiff::Timestamp::now().to_zoned_datetime(zone).date();
    let server = MockServer::start();
    let mut mocks = Vec::new();
    for focus_type in [0u8, 1] {
        mocks.push(focus_chunk(
            &server,
            &format!("{today}T00:00:00.000+0000"),
            &format!("{today}T23:59:59.999+0000"),
            focus_type,
            json!([]),
        ));
    }
    let mut mcp = McpBuilder::new(&server)
        .env("TZ", "Pacific/Pago_Pago")
        .env("TICKTICK_TIMEZONE", "Pacific/Kiritimati")
        .start();
    mcp.call_tool(
        "focus_log",
        json!({"from_date": "today", "to_date": "today"}),
    )
    .ok();
    for m in &mocks {
        m.assert();
    }
}
