use assert_cmd::{Command, cargo::cargo_bin_cmd};
use httpmock::prelude::*;
use std::fs;

fn isolated(tmp: &tempfile::TempDir) -> Command {
    let mut c = cargo_bin_cmd!("ticktick-cli");
    c.current_dir(tmp.path())
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env_remove("TICKTICK_ACCESS_TOKEN")
        .env_remove("TICKTICK_V2_SESSION_TOKEN")
        .env_remove("TICKTICK_API_BASE")
        .env_remove("TICKTICK_V2_BASE");
    c
}

#[test]
fn api_base_env_redirects_v1_calls() {
    let server = MockServer::start();
    let tags = server.mock(|when, then| {
        when.method(GET)
            .path("/open/v1/tag")
            .header("Authorization", "Bearer env-token");
        then.status(200)
            .json_body(serde_json::json!([{"name": "work"}]));
    });
    let tmp = tempfile::tempdir().unwrap();
    isolated(&tmp)
        .env("TICKTICK_ACCESS_TOKEN", "env-token")
        .env("TICKTICK_API_BASE", server.url("/open/v1/"))
        .args(["tag", "list"])
        .assert()
        .success();
    tags.assert();
}

#[test]
fn v2_session_token_env_wins_over_config() {
    let server = MockServer::start();
    let sync = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/batch/check/0")
            .header("Cookie", "t=env-cookie");
        then.status(200)
            .json_body(serde_json::json!({"inboxId": "inbox1"}));
    });
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("ticktick-cli");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("config.json"),
        r#"{"access_token":null,"refresh_token":null,"v2_session_token":"stale-file-cookie"}"#,
    )
    .unwrap();
    isolated(&tmp)
        .env("TICKTICK_V2_SESSION_TOKEN", "env-cookie")
        .env("TICKTICK_V2_BASE", server.url("/api/v2"))
        .arg("sync")
        .assert()
        .success();
    sync.assert();
}

#[test]
fn v2_session_token_falls_back_to_config() {
    let server = MockServer::start();
    let sync = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/batch/check/0")
            .header("Cookie", "t=file-cookie");
        then.status(200).json_body(serde_json::json!({}));
    });
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("ticktick-cli");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("config.json"),
        r#"{"access_token":null,"refresh_token":null,"v2_session_token":"file-cookie"}"#,
    )
    .unwrap();
    isolated(&tmp)
        .env("TICKTICK_V2_BASE", server.url("/api/v2"))
        .arg("sync")
        .assert()
        .success();
    sync.assert();
}

fn saved_account_timezone(tz_args: &[&str], echoed_tz: &str) -> Option<String> {
    let server = MockServer::start();
    let create = server.mock(|when, then| {
        when.method(POST).path("/open/v1/task");
        then.status(200).json_body(serde_json::json!({
            "id": "t1",
            "title": "x",
            "projectId": "p1",
            "timeZone": echoed_tz,
        }));
    });
    let tmp = tempfile::tempdir().unwrap();
    isolated(&tmp)
        .env("TICKTICK_ACCESS_TOKEN", "env-token")
        .env("TICKTICK_API_BASE", server.url("/open/v1/"))
        .args(["task", "add", "x", "--due", "2027-03-01T10:00"])
        .args(tz_args)
        .assert()
        .success();
    create.assert();
    let config = fs::read_to_string(tmp.path().join("ticktick-cli/config.json")).ok()?;
    let config: serde_json::Value = serde_json::from_str(&config).unwrap();
    config["account_timezone"].as_str().map(str::to_string)
}

#[test]
fn explicit_tz_is_not_cached_as_the_account_timezone() {
    assert_eq!(
        saved_account_timezone(&["--tz", "Europe/London"], "Europe/London"),
        None
    );
    assert_eq!(
        saved_account_timezone(&[], "America/Chicago").as_deref(),
        Some("America/Chicago")
    );
}

fn assign_stderr(status: u16) -> String {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/open/v1/task/assign");
        then.status(status).body("{}");
    });
    let tmp = tempfile::tempdir().unwrap();
    let out = isolated(&tmp)
        .env("TICKTICK_ACCESS_TOKEN", "env-token")
        .env("TICKTICK_API_BASE", server.url("/open/v1/"))
        .args([
            "task",
            "assign",
            "aaaaaaaaaaaaaaaaaaaaaaaa",
            "t1",
            "-a",
            "x",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    String::from_utf8(out.stderr).unwrap()
}

#[test]
fn assign_adds_the_membership_hint_only_for_membership_codes() {
    for status in [403, 500] {
        let err = assign_stderr(status);
        assert!(err.contains("must be a member"), "{status}: {err}");
    }
    let err = assign_stderr(404);
    assert!(err.contains("404"), "{err}");
    assert!(!err.contains("must be a member"), "{err}");
}
