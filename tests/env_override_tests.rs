mod common;

use assert_cmd::Command;
use httpmock::prelude::*;
use serde_json::json;
use std::fs;

fn isolated(tmp: &tempfile::TempDir) -> Command {
    Command::from_std(common::isolated_command(tmp.path()))
}

#[test]
fn api_base_env_redirects_v1_calls() {
    let server = MockServer::start();
    let tags = server.mock(|when, then| {
        when.method(GET)
            .path("/open/v1/tag")
            .header("Authorization", "Bearer env-token");
        then.status(200).json_body(json!([{"name": "work"}]));
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
fn blank_access_token_env_falls_back_to_config() {
    let server = MockServer::start();
    let tags = server.mock(|when, then| {
        when.method(GET)
            .path("/open/v1/tag")
            .header("Authorization", "Bearer file-token");
        then.status(200).json_body(json!([]));
    });
    let tmp = tempfile::tempdir().unwrap();
    common::write_config(
        tmp.path(),
        &json!({"access_token": "file-token", "refresh_token": null}),
    );
    isolated(&tmp)
        .env("TICKTICK_ACCESS_TOKEN", "  ")
        .env("TICKTICK_CLIENT_ID", "cid")
        .env("TICKTICK_CLIENT_SECRET", "sec")
        .env("TICKTICK_API_BASE", server.url("/open/v1"))
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
        then.status(200).json_body(json!({"inboxId": "inbox1"}));
    });
    let tmp = tempfile::tempdir().unwrap();
    common::write_config(
        tmp.path(),
        &json!({"access_token": null, "v2_session_token": "stale-file-cookie"}),
    );
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
        then.status(200).json_body(json!({}));
    });
    let tmp = tempfile::tempdir().unwrap();
    common::write_config(
        tmp.path(),
        &json!({"access_token": null, "v2_session_token": "file-cookie"}),
    );
    isolated(&tmp)
        .env("TICKTICK_V2_BASE", server.url("/api/v2"))
        .arg("sync")
        .assert()
        .success();
    sync.assert();
}

fn read_config(tmp: &tempfile::TempDir) -> serde_json::Value {
    let config = fs::read_to_string(tmp.path().join("ticktick-cli/config.json")).unwrap();
    serde_json::from_str(&config).unwrap()
}

/// `env_token` false: the token comes from config.json.
fn saved_account_timezone(env_token: bool, tz_args: &[&str], echoed_tz: &str) -> Option<String> {
    saved_over(env_token, None, tz_args, echoed_tz)
}

/// What `task add` leaves in config.json's account_timezone, starting from `stored`.
fn saved_over(
    env_token: bool,
    stored: Option<&str>,
    tz_args: &[&str],
    echoed_tz: &str,
) -> Option<String> {
    let server = MockServer::start();
    let create = server.mock(|when, then| {
        when.method(POST).path("/open/v1/task");
        then.status(200).json_body(json!({
            "id": "t1",
            "title": "x",
            "projectId": "p1",
            "timeZone": echoed_tz,
        }));
    });
    let tmp = tempfile::tempdir().unwrap();
    common::write_config(
        tmp.path(),
        &json!({"access_token": "file-token", "account_timezone": stored}),
    );
    let mut cmd = isolated(&tmp);
    if env_token {
        cmd.env("TICKTICK_ACCESS_TOKEN", "env-token");
    }
    cmd.env("TICKTICK_API_BASE", server.url("/open/v1/"))
        .args(["task", "add", "x", "--due", "2027-03-01T10:00"])
        .args(tz_args)
        .assert()
        .success();
    create.assert();
    read_config(&tmp)["account_timezone"]
        .as_str()
        .map(str::to_string)
}

#[test]
fn explicit_tz_is_not_cached_as_the_account_timezone() {
    assert_eq!(
        saved_account_timezone(false, &["--tz", "Europe/London"], "Europe/London"),
        None
    );
    assert_eq!(
        saved_account_timezone(false, &[], "America/Chicago").as_deref(),
        Some("America/Chicago")
    );
}

#[test]
fn account_timezone_is_saved_canonical_and_never_empty() {
    assert_eq!(saved_account_timezone(false, &[], ""), None);
    assert_eq!(
        saved_account_timezone(false, &[], "america/chicago").as_deref(),
        Some("America/Chicago")
    );
}

#[test]
fn invalid_stored_account_timezone_is_redetected() {
    for stored in ["", "Mars/Olympus"] {
        assert_eq!(
            saved_over(false, Some(stored), &[], "America/Chicago").as_deref(),
            Some("America/Chicago"),
            "{stored:?}"
        );
    }
}

#[test]
fn env_token_never_caches_the_account_timezone() {
    assert_eq!(saved_account_timezone(true, &[], "America/Chicago"), None);
}

/// The `timeZone` a non-interactive `task add --dry-run` would send.
fn dry_run_timezone(env_token: bool, account_tz: &str) -> serde_json::Value {
    let tmp = tempfile::tempdir().unwrap();
    common::write_config(
        tmp.path(),
        &json!({"access_token": "file-token", "account_timezone": account_tz}),
    );
    let mut cmd = isolated(&tmp);
    if env_token {
        cmd.env("TICKTICK_ACCESS_TOKEN", "env-token");
    }
    let out = cmd
        .args(["task", "add", "x", "--due", "2027-03-01T10:00", "--dry-run"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    body["timeZone"].clone()
}

#[test]
fn env_token_ignores_the_config_account_timezone() {
    // Differs from the pinned local TZ, so config.json's tokens send the local one.
    assert_eq!(dry_run_timezone(false, "Europe/London"), "America/Chicago");
    assert_eq!(
        dry_run_timezone(true, "Europe/London"),
        serde_json::Value::Null
    );
}

#[test]
fn env_token_never_uses_the_config_v2_cookie() {
    let server = MockServer::start();
    let sync = server.mock(|when, then| {
        when.method(GET).path("/api/v2/batch/check/0");
        then.status(200).json_body(json!({}));
    });
    let tmp = tempfile::tempdir().unwrap();
    common::write_config(
        tmp.path(),
        &json!({"access_token": "file-token", "v2_session_token": "file-cookie"}),
    );
    let out = isolated(&tmp)
        .env("TICKTICK_ACCESS_TOKEN", "env-token")
        .env("TICKTICK_V2_BASE", server.url("/api/v2"))
        .arg("sync")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("v2 session token not set"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    sync.assert_calls(0);
}

#[test]
fn dotenv_access_token_takes_its_v2_cookie_from_dotenv() {
    let server = MockServer::start();
    let sync = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/batch/check/0")
            .header("Cookie", "t=dotenv-cookie");
        then.status(200).json_body(json!({}));
    });
    let tmp = tempfile::tempdir().unwrap();
    common::write_config(
        tmp.path(),
        &json!({"access_token": "file-token", "v2_session_token": "file-cookie"}),
    );
    fs::write(
        tmp.path().join(".env"),
        "TICKTICK_ACCESS_TOKEN=dotenv-token\nTICKTICK_V2_SESSION_TOKEN=dotenv-cookie\n",
    )
    .unwrap();
    isolated(&tmp)
        .env("TICKTICK_V2_BASE", server.url("/api/v2"))
        .arg("sync")
        .assert()
        .success();
    sync.assert();
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

fn task_list(
    tmp: &tempfile::TempDir,
    server: &MockServer,
    token: &str,
    args: &[&str],
) -> Vec<String> {
    let out = isolated(tmp)
        .env("TICKTICK_ACCESS_TOKEN", token)
        .env("TICKTICK_API_BASE", server.url("/open/v1/"))
        .args(["task", "list"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tasks: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    tasks
        .iter()
        .map(|t| t["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn env_token_inbox_is_probed_only_when_asked_for_and_cached_across_processes() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/open/v1/project");
        then.status(200).json_body(json!([]));
    });
    let probe = server.mock(|when, then| {
        when.method(POST).path("/open/v1/task");
        then.status(200)
            .json_body(json!({"id": "probe", "title": "p", "projectId": "inbox9"}));
    });
    let cleanup = server.mock(|when, then| {
        when.method(DELETE)
            .path("/open/v1/project/inbox9/task/probe");
        then.status(200);
    });
    let inbox = server.mock(|when, then| {
        when.method(GET).path("/open/v1/project/inbox9/data");
        then.status(200).json_body(
            json!({"tasks": [{"id": "t1", "title": "in inbox", "projectId": "inbox9"}]}),
        );
    });
    let tmp = tempfile::tempdir().unwrap();

    assert!(task_list(&tmp, &server, "env-token", &[]).is_empty());
    probe.assert_calls(0);

    assert_eq!(task_list(&tmp, &server, "env-token", &["inbox"]), ["t1"]);
    assert_eq!(task_list(&tmp, &server, "env-token", &["inbox"]), ["t1"]);
    assert_eq!(task_list(&tmp, &server, "env-token", &[]), ["t1"]);
    probe.assert_calls(1);
    cleanup.assert_calls(1);

    // Another account's token doesn't get this inbox.
    assert!(task_list(&tmp, &server, "other-token", &[]).is_empty());
    inbox.assert_calls(3);
}

#[test]
fn dry_run_never_probes_for_the_inbox() {
    let server = MockServer::start();
    let any = server.mock(|when, then| {
        when.any_request();
        then.status(500);
    });
    let tmp = tempfile::tempdir().unwrap();
    let out = isolated(&tmp)
        .env("TICKTICK_ACCESS_TOKEN", "env-token")
        .env("TICKTICK_API_BASE", server.url("/open/v1/"))
        .args(["task", "add", "x", "-p", "Inbox", "--dry-run"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["projectId"], "inbox");
    any.assert_calls(0);
    assert!(!tmp.path().join("ticktick-cli/config.json").exists());
}

#[test]
fn failed_probe_cleanup_names_the_leftover_task_and_keeps_the_id() {
    let server = MockServer::start();
    let probe = server.mock(|when, then| {
        when.method(POST).path("/open/v1/task");
        then.status(200)
            .json_body(json!({"id": "probe7", "title": "p", "projectId": "inbox9"}));
    });
    server.mock(|when, then| {
        when.method(DELETE)
            .path("/open/v1/project/inbox9/task/probe7");
        then.status(500);
    });
    server.mock(|when, then| {
        when.method(GET).path("/open/v1/project/inbox9/data");
        then.status(200).json_body(json!({"tasks": []}));
    });
    let tmp = tempfile::tempdir().unwrap();
    let out = isolated(&tmp)
        .env("TICKTICK_ACCESS_TOKEN", "env-token")
        .env("TICKTICK_API_BASE", server.url("/open/v1/"))
        .args(["task", "list", "inbox"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("probe7"), "{err}");

    assert!(task_list(&tmp, &server, "env-token", &["inbox"]).is_empty());
    probe.assert_calls(1);
}

#[test]
fn expired_dotenv_access_token_refreshes_with_dotenv_refresh_token() {
    let server = MockServer::start();
    let stale = server.mock(|when, then| {
        when.method(GET)
            .path("/open/v1/tag")
            .header("Authorization", "Bearer dotenv-token");
        then.status(401);
    });
    let refresh = server.mock(|when, then| {
        when.method(POST)
            .path("/oauth/token")
            .header("Authorization", "Basic Y2lkOnNlYw==")
            .body("grant_type=refresh_token&refresh_token=dotenv-refresh");
        then.status(200)
            .json_body(json!({"access_token": "fresh", "refresh_token": "r2"}));
    });
    let fresh = server.mock(|when, then| {
        when.method(GET)
            .path("/open/v1/tag")
            .header("Authorization", "Bearer fresh");
        then.status(200).json_body(json!([]));
    });
    let tmp = tempfile::tempdir().unwrap();
    common::write_config(
        tmp.path(),
        &json!({"access_token": "file-token", "refresh_token": "file-refresh"}),
    );
    fs::write(
        tmp.path().join(".env"),
        "TICKTICK_ACCESS_TOKEN=dotenv-token\nTICKTICK_REFRESH_TOKEN=dotenv-refresh\n",
    )
    .unwrap();
    isolated(&tmp)
        // Already in the env, which must not stop the .env file being read.
        .env("TICKTICK_CLIENT_ID", "cid")
        .env("TICKTICK_CLIENT_SECRET", "sec")
        .env("TICKTICK_API_BASE", server.url("/open/v1"))
        .env("TICKTICK_OAUTH_TOKEN_URL", server.url("/oauth/token"))
        .args(["tag", "list"])
        .assert()
        .success();
    stale.assert();
    refresh.assert();
    fresh.assert();
    assert_eq!(
        read_config(&tmp),
        json!({"access_token": "file-token", "refresh_token": "file-refresh"})
    );
}

fn expired_token_stderr(dotenv: &str) -> String {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/open/v1/tag");
        then.status(401);
    });
    server.mock(|when, then| {
        when.method(POST).path("/oauth/token");
        then.status(400);
    });
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join(".env"), dotenv).unwrap();
    let out = isolated(&tmp)
        .env("TICKTICK_API_BASE", server.url("/open/v1"))
        .env("TICKTICK_OAUTH_TOKEN_URL", server.url("/oauth/token"))
        .args(["tag", "list"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    String::from_utf8(out.stderr).unwrap()
}

#[test]
fn expired_env_token_hint_names_the_env_vars_not_login() {
    let err = expired_token_stderr("TICKTICK_ACCESS_TOKEN=a\n");
    assert!(err.contains("Set TICKTICK_REFRESH_TOKEN"), "{err}");
    assert!(!err.contains("login"), "{err}");

    let err = expired_token_stderr(
        "TICKTICK_ACCESS_TOKEN=a\nTICKTICK_REFRESH_TOKEN=r\n\
         TICKTICK_CLIENT_ID=cid\nTICKTICK_CLIENT_SECRET=sec\n",
    );
    assert!(err.contains("could not refresh it"), "{err}");
    assert!(!err.contains("login"), "{err}");
}

#[test]
fn login_clears_the_previous_accounts_inbox_and_timezone() {
    use std::io::Write;
    use std::time::{Duration, Instant};

    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST)
            .path("/oauth/token")
            .body_includes("grant_type=authorization_code&code=c1&");
        then.status(200)
            .json_body(json!({"access_token": "new", "refresh_token": "new-r"}));
    });
    let tmp = tempfile::tempdir().unwrap();
    common::write_config(
        tmp.path(),
        &json!({
            "access_token": "old",
            "refresh_token": "old-r",
            "inbox_project_id": "inboxOld",
            "account_timezone": "Europe/London",
            "v2_device_id": "dev1",
        }),
    );
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut login = common::isolated_command(tmp.path())
        .env("TICKTICK_CLIENT_ID", "cid")
        .env("TICKTICK_CLIENT_SECRET", "sec")
        .env("TICKTICK_OAUTH_TOKEN_URL", server.url("/oauth/token"))
        .env("TICKTICK_OAUTH_PORT", port.to_string())
        .arg("login")
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    let started = Instant::now();
    let mut callback = loop {
        match std::net::TcpStream::connect(("127.0.0.1", port)) {
            Ok(s) => break s,
            Err(_) if started.elapsed() < Duration::from_secs(10) => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Err(e) => panic!("login never listened: {e}"),
        }
    };
    callback
        .write_all(b"GET /callback?code=c1&state=x HTTP/1.1\r\n\r\n")
        .unwrap();
    assert!(login.wait().unwrap().success());

    assert_eq!(
        read_config(&tmp),
        json!({"access_token": "new", "refresh_token": "new-r", "v2_device_id": "dev1"})
    );
}
