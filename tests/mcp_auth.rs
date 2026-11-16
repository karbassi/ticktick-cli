//! Token source, refresh and config.json handling, end to end.

mod common;

use common::McpBuilder;
use httpmock::prelude::*;
use serde_json::json;
use std::time::{Duration, Instant};

fn v1(path: &str) -> String {
    format!("/open/v1{path}")
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
