//! Token source, refresh and config.json handling, end to end.

mod common;

use common::{ACCESS_TOKEN, McpBuilder};
use httpmock::prelude::*;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn v1(path: &str) -> String {
    format!("/open/v1{path}")
}

/// A config dir the test can read and rewrite while the server runs.
struct ConfigDir(tempfile::TempDir);

impl ConfigDir {
    fn new(config: &Value) -> Self {
        let dir = Self(tempfile::tempdir().unwrap());
        fs::create_dir_all(dir.0.path().join("ticktick-cli")).unwrap();
        dir.write(config);
        dir
    }

    fn file(&self) -> PathBuf {
        self.0.path().join("ticktick-cli/config.json")
    }

    fn write(&self, config: &Value) {
        fs::write(self.file(), config.to_string()).unwrap();
    }

    fn read(&self) -> Value {
        serde_json::from_str(&fs::read_to_string(self.file()).unwrap()).unwrap()
    }

    fn path(&self) -> &str {
        self.0.path().to_str().unwrap()
    }
}

/// Tokens come from config.json: the builder's env token is blanked out.
fn config_tokens(server: &MockServer, cfg: &ConfigDir) -> McpBuilder {
    McpBuilder::new(server)
        .env("TICKTICK_ACCESS_TOKEN", "")
        .env("TICKTICK_CLIENT_ID", "cid")
        .env("TICKTICK_CLIENT_SECRET", "sec")
        .env("TICKTICK_OAUTH_TOKEN_URL", &server.url("/oauth/token"))
        .env("XDG_CONFIG_HOME", cfg.path())
}

fn projects_with<'a>(server: &'a MockServer, token: &str, status: u16) -> httpmock::Mock<'a> {
    let token = token.to_string();
    server.mock(move |when, then| {
        when.method(GET)
            .path(v1("/project"))
            .header("Authorization", format!("Bearer {token}"));
        then.status(status).json_body(json!([]));
    })
}

fn refresh_with<'a>(server: &'a MockServer, used: &str, delay: Duration) -> httpmock::Mock<'a> {
    let body = format!("grant_type=refresh_token&refresh_token={used}");
    server.mock(move |when, then| {
        when.method(POST).path("/oauth/token").body(body);
        then.status(200)
            .delay(delay)
            .json_body(json!({"access_token": "a1", "refresh_token": "r2"}));
    })
}

#[test]
fn config_tokens_are_reread_after_a_refresh() {
    let server = MockServer::start();
    let cfg = ConfigDir::new(&json!({"access_token": "a0", "refresh_token": "r1"}));
    let stale = projects_with(&server, "a0", 401);
    let refresh = refresh_with(&server, "r1", Duration::ZERO);
    let refreshed = projects_with(&server, "a1", 200);
    let relogged = projects_with(&server, "login2", 200);
    let mut c = config_tokens(&server, &cfg).start();

    c.call_tool("list_projects", json!({})).ok();
    assert_eq!(
        cfg.read(),
        json!({"access_token": "a1", "refresh_token": "r2"})
    );

    // `ticktick-cli login` in another process.
    cfg.write(&json!({"access_token": "login2", "refresh_token": "r9"}));
    c.call_tool("list_projects", json!({})).ok();

    stale.assert_calls(1);
    refresh.assert_calls(1);
    refreshed.assert_calls(1);
    relogged.assert_calls(1);
}

#[test]
fn refresh_does_not_overwrite_a_login_made_while_it_ran() {
    let server = MockServer::start();
    let cfg = ConfigDir::new(&json!({"access_token": "a0", "refresh_token": "r1"}));
    projects_with(&server, "a0", 401);
    let refresh = refresh_with(&server, "r1", Duration::from_secs(2));
    let refreshed = projects_with(&server, "a1", 200);
    let relogged = projects_with(&server, "login2", 200);
    let mut c = config_tokens(&server, &cfg).start();

    let login = json!({"access_token": "login2", "refresh_token": "r9"});
    let file = cfg.file();
    let concurrent_login = {
        let login = login.to_string();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            fs::write(file, login).unwrap();
        })
    };
    c.call_tool("list_projects", json!({})).ok();
    concurrent_login.join().unwrap();

    refresh.assert_calls(1);
    refreshed.assert_calls(0);
    relogged.assert_calls(1);
    assert_eq!(cfg.read(), login);
}

#[test]
fn env_access_token_never_refreshes_with_config_refresh_token() {
    let server = MockServer::start();
    let cfg = ConfigDir::new(&json!({"access_token": "other", "refresh_token": "file-r"}));
    let stale = projects_with(&server, ACCESS_TOKEN, 401);
    let refresh = server.mock(|when, then| {
        when.method(POST).path("/oauth/token");
        then.status(200)
            .json_body(json!({"access_token": "a1", "refresh_token": "r2"}));
    });
    let mut c = McpBuilder::new(&server)
        .env("TICKTICK_CLIENT_ID", "cid")
        .env("TICKTICK_CLIENT_SECRET", "sec")
        .env("TICKTICK_OAUTH_TOKEN_URL", &server.url("/oauth/token"))
        .env("XDG_CONFIG_HOME", cfg.path())
        .start();

    let res = c.call_tool("list_projects", json!({}));

    assert!(res.is_error, "{}", res.text);
    stale.assert_calls(1);
    refresh.assert_calls(0);
}

#[test]
fn env_access_token_probes_the_inbox_instead_of_using_config() {
    let server = MockServer::start();
    let original = r#"{"access_token":"other","refresh_token":null,"inbox_project_id":"inbox999"}"#;
    let cfg = ConfigDir::new(&json!({}));
    fs::write(cfg.file(), original).unwrap();
    let probe = server.mock(|when, then| {
        when.method(POST)
            .path(v1("/task"))
            .header("Authorization", format!("Bearer {ACCESS_TOKEN}"))
            .json_body(json!({"title": "__inbox_probe__"}));
        then.status(200)
            .json_body(json!({"id": "probe", "title": "__inbox_probe__", "projectId": "inbox123"}));
    });
    let cleanup = server.mock(|when, then| {
        when.method(DELETE).path(v1("/project/inbox123/task/probe"));
        then.status(200);
    });
    let inbox = server.mock(|when, then| {
        when.method(GET).path(v1("/project/inbox123/data"));
        then.status(200).json_body(json!({"tasks": []}));
    });
    let other_account = server.mock(|when, then| {
        when.method(GET).path(v1("/project/inbox999/data"));
        then.status(200).json_body(json!({"tasks": []}));
    });
    let mut c = McpBuilder::new(&server)
        .env("XDG_CONFIG_HOME", cfg.path())
        .start();

    c.call_tool("list_tasks", json!({"project": "inbox"})).ok();
    c.call_tool("list_tasks", json!({"project": "inbox"})).ok();

    probe.assert_calls(1);
    cleanup.assert_calls(1);
    inbox.assert_calls(2);
    other_account.assert_calls(0);
    assert_eq!(fs::read_to_string(cfg.file()).unwrap(), original);
}

#[test]
fn stalled_token_refresh_times_out() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path(v1("/project"));
        then.status(401);
    });
    let refresh = server.mock(|when, then| {
        when.method(POST).path("/oauth/token");
        then.status(200)
            .delay(Duration::from_secs(10))
            .json_body(json!({"access_token": "fresh"}));
    });
    let mut c = McpBuilder::new(&server)
        .env("TICKTICK_REFRESH_TOKEN", "r1")
        .env("TICKTICK_CLIENT_ID", "cid")
        .env("TICKTICK_CLIENT_SECRET", "sec")
        .env("TICKTICK_OAUTH_TOKEN_URL", &server.url("/oauth/token"))
        .env("TICKTICK_HTTP_TIMEOUT_SECS", "1")
        .start();
    let started = Instant::now();
    let res = c.call_tool("list_projects", json!({}));
    let elapsed = started.elapsed();
    refresh.assert();
    assert!(res.is_error, "{}", res.text);
    assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");
}
