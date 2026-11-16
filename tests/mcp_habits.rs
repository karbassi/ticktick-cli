//! Cases follow Python `test_habits.py`.

mod common;

use common::{McpBuilder, McpClient, SESSION_TOKEN, V1, V2, start};
use httpmock::prelude::*;
use serde_json::{Value, json};

fn habits() -> Value {
    json!([
        {"id": "h1", "name": "Exercise", "etag": "e1", "type": "Boolean", "goal": 1, "status": 0},
        {"id": "h2", "name": "Read", "etag": "e2", "type": "Numeric", "goal": 30, "unit": "pages"},
    ])
}

fn sections() -> Value {
    json!([
        {"id": "s1", "name": "Morning", "sortOrder": 0},
        {"id": "s2", "name": "Evening", "sortOrder": 1},
    ])
}

fn mock_habits(server: &MockServer) -> httpmock::Mock<'_> {
    server.mock(|when, then| {
        when.method(GET)
            .path(format!("{V2}/habits"))
            .header("Cookie", format!("t={SESSION_TOKEN}"));
        then.status(200).json_body(habits());
    })
}

fn mock_sections(server: &MockServer) -> httpmock::Mock<'_> {
    server.mock(|when, then| {
        when.method(GET).path(format!("{V1}/habit/sections"));
        then.status(200).json_body(sections());
    })
}

fn mock_post<'a>(
    server: &'a MockServer,
    path: &str,
    body: Value,
    reply: Value,
) -> httpmock::Mock<'a> {
    server.mock(|when, then| {
        when.method(POST)
            .path(format!("{V2}{path}"))
            .json_body(body);
        then.status(200).json_body(reply);
    })
}

#[test]
fn add_boolean_defaults() {
    let server = MockServer::start();
    let post = mock_post(
        &server,
        "/habits/batch",
        json!({"add": [{"name": "Meditate", "type": "Boolean"}]}),
        json!({"id2etag": {"h_new": "etag_new"}}),
    );
    start(&server)
        .call_tool("add_habit", json!({"name": "Meditate"}))
        .ok();
    post.assert();
}

#[test]
fn add_numeric_with_all_fields_resolves_section() {
    let server = MockServer::start();
    let secs = mock_sections(&server);
    let post = mock_post(
        &server,
        "/habits/batch",
        json!({"add": [{
            "name": "Water", "type": "Numeric", "goal": 8.0, "unit": "glasses",
            "sectionId": "s1", "repeatRule": "RRULE:FREQ=DAILY", "color": "#00FF00",
        }]}),
        json!({"id2etag": {"h_new": "etag_new"}}),
    );
    start(&server)
        .call_tool(
            "add_habit",
            json!({
                "name": "Water", "habit_type": "Numeric", "goal": 8, "unit": "glasses",
                "section": "morning", "repeat_rule": "RRULE:FREQ=DAILY", "color": "#00FF00",
            }),
        )
        .ok();
    secs.assert();
    post.assert();
}

#[test]
fn add_habit_empty_section_is_ignored() {
    let server = MockServer::start();
    let post = mock_post(
        &server,
        "/habits/batch",
        json!({"add": [{"name": "X", "type": "Boolean"}]}),
        json!({}),
    );
    start(&server)
        .call_tool("add_habit", json!({"name": "X", "section": ""}))
        .ok();
    post.assert();
}

#[test]
fn add_habit_unknown_section_errors() {
    let server = MockServer::start();
    mock_sections(&server);
    let res = start(&server).call_tool("add_habit", json!({"name": "X", "section": "Nope"}));
    assert!(res.is_error);
    assert_eq!(res.text, "No habit section found matching 'Nope'");
}

#[test]
fn edit_habit_keeps_unchanged_fields() {
    let server = MockServer::start();
    mock_habits(&server);
    let post = mock_post(
        &server,
        "/habits/batch",
        json!({"update": [{"id": "h1", "name": "Workout", "etag": "e1", "type": "Boolean", "goal": 1, "status": 0}]}),
        json!({"id2etag": {"h1": "e1_new"}}),
    );
    start(&server)
        .call_tool(
            "edit_habit",
            json!({"habit": "exercise", "name": "Workout"}),
        )
        .ok();
    post.assert();
}

#[test]
fn edit_habit_by_unknown_id_errors_without_sending() {
    let server = MockServer::start();
    mock_habits(&server);
    let post = server.mock(|when, then| {
        when.method(POST).path(format!("{V2}/habits/batch"));
        then.status(200).json_body(json!({}));
    });
    let r = start(&server).call_tool(
        "edit_habit",
        json!({"habit": "aaaaaaaaaaaaaaaaaaaaaaaa", "name": "X"}),
    );
    assert!(r.is_error, "{}", r.text);
    assert_eq!(r.text, "No habit found with ID 'aaaaaaaaaaaaaaaaaaaaaaaa'");
    post.assert_calls(0);
}

#[test]
fn edit_habit_all_fields() {
    let server = MockServer::start();
    mock_habits(&server);
    mock_sections(&server);
    let post = mock_post(
        &server,
        "/habits/batch",
        json!({"update": [{
            "id": "h2", "etag": "e2", "type": "Numeric", "name": "Reading", "goal": 40.0, "unit": "pp",
            "sectionId": "s2", "repeatRule": "R", "color": "#111111",
        }]}),
        json!({}),
    );
    start(&server)
        .call_tool(
            "edit_habit",
            json!({
                "habit": "Read", "name": "Reading", "goal": 40, "unit": "pp",
                "section": "Evening", "repeat_rule": "R", "color": "#111111",
            }),
        )
        .ok();
    post.assert();
}

#[test]
fn edit_habit_no_match_suggests() {
    let server = MockServer::start();
    mock_habits(&server);
    let res = start(&server).call_tool("edit_habit", json!({"habit": "Excercise"}));
    assert!(res.is_error);
    assert!(res.text.contains("No habit found matching 'Excercise'"));
    assert!(res.text.contains("Did you mean 'Exercise'?"));
}

#[test]
fn delete_habits_resolves_each_with_one_fetch_and_confirms() {
    let server = MockServer::start();
    let list = mock_habits(&server);
    let post = mock_post(
        &server,
        "/habits/batch",
        json!({"delete": ["h1", "h2"]}),
        json!({"ok": true}),
    );
    let res = start(&server)
        .call_tool("delete_habits", json!({"habits": ["Exercise", "read"]}))
        .ok();
    list.assert_calls(1);
    post.assert();
    assert_eq!(res.text, "Deleted 2 habit(s)");
}

#[test]
fn habit_ids_skip_the_fetch_unless_the_etag_is_needed() {
    let id = "0123456789abcdef01234567";
    let server = MockServer::start();
    let list = mock_habits(&server);
    let delete = mock_post(&server, "/habits/batch", json!({"delete": [id]}), json!({}));
    let mut c = start(&server);
    c.call_tool("delete_habits", json!({"habits": [id]})).ok();
    list.assert_calls(0);
    delete.assert();
    // Archive needs the full record, and an ID absent from the list has none.
    let r = c.call_tool("archive_habits", json!({"habits": [id]}));
    assert!(r.is_error, "{}", r.text);
    list.assert_calls(1);
}

#[test]
fn checkin_specific_date() {
    let server = MockServer::start();
    mock_habits(&server);
    let post = mock_post(
        &server,
        "/habitCheckins/batch",
        json!({"add": [{"habitId": "h1", "checkinStamp": 20260222, "value": 1.0, "status": 0}]}),
        json!({"id2etag": {"c1": "ce1"}}),
    );
    start(&server)
        .call_tool(
            "checkin_habit",
            json!({"habit": "Exercise", "date": "2026-02-22"}),
        )
        .ok();
    post.assert();
}

#[test]
fn checkin_numeric_value() {
    let server = MockServer::start();
    mock_habits(&server);
    let post = mock_post(
        &server,
        "/habitCheckins/batch",
        json!({"add": [{"habitId": "h2", "checkinStamp": 20260101, "value": 12.5, "status": 0}]}),
        json!({}),
    );
    start(&server)
        .call_tool(
            "checkin_habit",
            json!({"habit": "Read", "date": "2026-01-01", "value": 12.5}),
        )
        .ok();
    post.assert();
}

#[test]
fn checkin_today_default_uses_yyyymmdd_stamp() {
    let server = MockServer::start();
    mock_habits(&server);
    let post = server.mock(|when, then| {
        when.method(POST)
            .path(format!("{V2}/habitCheckins/batch"))
            .body_includes("\"checkinStamp\":20");
        then.status(200).json_body(json!({}));
    });
    start(&server)
        .call_tool("checkin_habit", json!({"habit": "Exercise"}))
        .ok();
    post.assert();
}

/// Pago Pago and Kiritimati are 25h apart, so their dates always differ.
fn kiritimati_today_stamp() -> i64 {
    let zone = jiff::tz::TimeZone::get("Pacific/Kiritimati").unwrap();
    let today = jiff::Timestamp::now().to_zoned_datetime(zone).date();
    i64::from(today.year()) * 10000 + i64::from(today.month()) * 100 + i64::from(today.day())
}

fn start_pago_pago_vs_kiritimati(server: &MockServer) -> McpClient {
    McpBuilder::new(server)
        .env("TZ", "Pacific/Pago_Pago")
        .env("TICKTICK_TIMEZONE", "Pacific/Kiritimati")
        .start()
}

#[test]
fn checkin_today_is_taken_in_ticktick_timezone() {
    let stamp = kiritimati_today_stamp();
    let server = MockServer::start();
    mock_habits(&server);
    let post = mock_post(
        &server,
        "/habitCheckins/batch",
        json!({"add": [{"habitId": "h1", "checkinStamp": stamp, "value": 1.0, "status": 0}]}),
        json!({}),
    );
    start_pago_pago_vs_kiritimati(&server)
        .call_tool(
            "checkin_habit",
            json!({"habit": "Exercise", "date": "today"}),
        )
        .ok();
    post.assert();
}

#[test]
fn habit_log_today_is_taken_in_ticktick_timezone() {
    let stamp = kiritimati_today_stamp();
    let server = MockServer::start();
    mock_habits(&server);
    let post = mock_post(
        &server,
        "/habitCheckins/query",
        json!({"habitIds": ["h1"], "afterStamp": stamp}),
        json!({}),
    );
    start_pago_pago_vs_kiritimati(&server)
        .call_tool(
            "habit_log",
            json!({"habits": ["Exercise"], "after": "today"}),
        )
        .ok();
    post.assert();
}

#[test]
fn checkin_invalid_date_errors() {
    let server = MockServer::start();
    mock_habits(&server);
    let mut c = start(&server);
    let res = c.call_tool(
        "checkin_habit",
        json!({"habit": "Exercise", "date": "next week"}),
    );
    assert!(res.is_error);
    assert_eq!(
        res.text,
        "Invalid date 'next week': expected YYYY-MM-DD, 'today', or 'yesterday'"
    );
    let res = c.call_tool(
        "checkin_habit",
        json!({"habit": "Exercise", "date": "2026-13-01"}),
    );
    assert_eq!(res.text, "Month out of range in '2026-13-01'");
    let res = c.call_tool(
        "checkin_habit",
        json!({"habit": "Exercise", "date": "2026-aa-01"}),
    );
    assert_eq!(
        res.text,
        "Invalid date '2026-aa-01': non-numeric components"
    );
}

#[test]
fn checkin_impossible_date_errors_without_request() {
    let server = MockServer::start();
    mock_habits(&server);
    let post = server.mock(|when, then| {
        when.method(POST).path(format!("{V2}/habitCheckins/batch"));
        then.status(200).json_body(json!({}));
    });
    let res = start(&server).call_tool(
        "checkin_habit",
        json!({"habit": "Exercise", "date": "2026-02-30"}),
    );
    assert!(res.is_error);
    assert_eq!(res.text, "Invalid date '2026-02-30': no such calendar day");
    post.assert_calls(0);
}

#[test]
fn habit_log_query() {
    let server = MockServer::start();
    mock_habits(&server);
    let post = mock_post(
        &server,
        "/habitCheckins/query",
        json!({"habitIds": ["h1", "h2"], "afterStamp": 20260201}),
        json!({"checkins": [{"habitId": "h1", "value": 1}]}),
    );
    start(&server)
        .call_tool(
            "habit_log",
            json!({"habits": ["Exercise", "Read"], "after": "2026-02-01"}),
        )
        .ok();
    post.assert();
}

#[test]
fn archive_habits_sets_status_1_and_keeps_fields() {
    let server = MockServer::start();
    mock_habits(&server);
    let post = mock_post(
        &server,
        "/habits/batch",
        json!({"update": [
            {"id": "h1", "name": "Exercise", "etag": "e1", "type": "Boolean", "goal": 1, "status": 1},
            {"id": "h2", "name": "Read", "etag": "e2", "type": "Numeric", "goal": 30, "unit": "pages", "status": 1},
        ]}),
        json!({"id2etag": {"h1": "e1_arch"}}),
    );
    start(&server)
        .call_tool("archive_habits", json!({"habits": ["Exercise", "Read"]}))
        .ok();
    post.assert();
}

#[test]
fn sections_delete_resolves_with_one_fetch_and_confirms() {
    let server = MockServer::start();
    let list = mock_sections(&server);
    let post = mock_post(
        &server,
        "/habitSections/batch",
        json!({"delete": ["s1", "s2"]}),
        json!({"ok": true}),
    );
    let res = start(&server)
        .call_tool(
            "manage_habit_sections",
            json!({"action": "delete", "sections": ["Morning", "eve"]}),
        )
        .ok();
    list.assert_calls(1);
    post.assert();
    assert_eq!(res.text, "Deleted 2 section(s)");
}

#[test]
fn sections_rename_resolves_via_v1() {
    let server = MockServer::start();
    mock_sections(&server);
    let post = mock_post(
        &server,
        "/habitSections/batch",
        json!({"update": [{"id": "s2", "name": "Night"}]}),
        json!({"id2etag": {}}),
    );
    start(&server)
        .call_tool(
            "manage_habit_sections",
            json!({"action": "rename", "section": "evening", "new_name": "Night"}),
        )
        .ok();
    post.assert();
}

#[test]
fn sections_hex_id_passes_through_after_fetching_sections() {
    let server = MockServer::start();
    let secs = mock_sections(&server);
    let post = mock_post(
        &server,
        "/habitSections/batch",
        json!({"update": [{"id": "0123456789abcdef01234567", "name": "N"}]}),
        json!({}),
    );
    start(&server)
        .call_tool(
            "manage_habit_sections",
            json!({"action": "rename", "section": "0123456789abcdef01234567", "new_name": "N"}),
        )
        .ok();
    secs.assert();
    post.assert();
}

#[test]
fn sections_ambiguous_match_errors() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path(format!("{V1}/habit/sections"));
        then.status(200).json_body(json!([
            {"id": "s1", "name": "Morning A"},
            {"id": "s2", "name": "Morning B"},
        ]));
    });
    let res = start(&server).call_tool(
        "manage_habit_sections",
        json!({"action": "rename", "section": "morn", "new_name": "N"}),
    );
    assert!(res.is_error);
    assert_eq!(
        res.text,
        "Multiple sections match 'morn': Morning A, Morning B"
    );
}

#[test]
fn sections_validation_errors() {
    let server = MockServer::start();
    let mut c = start(&server);
    let cases = [
        (
            json!({"action": "add"}),
            "'name' is required for action='add'",
        ),
        (
            json!({"action": "delete"}),
            "'sections' is required for action='delete'",
        ),
        (
            json!({"action": "delete", "sections": []}),
            "'sections' is required for action='delete'",
        ),
        (
            json!({"action": "rename", "new_name": "x"}),
            "'section' is required for action='rename'",
        ),
        (
            json!({"action": "rename", "section": "x"}),
            "'new_name' is required for action='rename'",
        ),
        (
            json!({"action": "bogus"}),
            "Invalid action 'bogus'. Use: list, add, delete, rename",
        ),
    ];
    for (args, msg) in cases {
        let res = c.call_tool("manage_habit_sections", args);
        assert!(res.is_error);
        assert_eq!(res.text, msg);
    }
}
