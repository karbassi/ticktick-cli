mod common;

use common::{
    HttpResponse, HttpServer, http_post, http_post_with, initialize_request, isolated_command,
};
use httpmock::prelude::*;
use serde_json::json;
use std::time::Duration;

#[test]
fn http_initialize_and_list_tools() {
    let server = HttpServer::start(&[], &[]);
    let init = http_post(server.port, &server.host(), None, &initialize_request(1));
    assert_eq!(init.status, 200, "{init:?}");
    let session = init
        .header("mcp-session-id")
        .expect("session id header")
        .to_string();
    let result = init.json();
    assert_eq!(result["id"], 1);
    assert_eq!(result["result"]["serverInfo"]["name"], "TickTick");

    let ack = http_post(
        server.port,
        &server.host(),
        Some(&session),
        &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    );
    assert_eq!(ack.status, 202, "{ack:?}");

    let list = http_post(
        server.port,
        &server.host(),
        Some(&session),
        &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
    );
    assert_eq!(list.status, 200, "{list:?}");
    let tools = list.json()["result"]["tools"].as_array().unwrap().clone();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"add_task"), "{names:?}");
    assert!(names.contains(&"list_projects"), "{names:?}");
}

fn status(server: &HttpServer, host: &str, origin: Option<&str>) -> u16 {
    let headers: Vec<(&str, &str)> = origin.map(|o| ("Origin", o)).into_iter().collect();
    http_post_with(server.port, host, None, &headers, &initialize_request(1)).status
}

#[test]
fn http_rejects_disallowed_host_and_origin() {
    let server = HttpServer::start(&[], &[]);
    // FastMCP answers a bad Host with 421 and a bad Origin with 403.
    assert_eq!(status(&server, "evil.example", None), 421);
    assert_eq!(status(&server, "evil.example:8502", None), 421);
    assert_eq!(
        status(&server, &server.host(), Some("https://evil.example")),
        403
    );
    assert_eq!(
        status(&server, "127.0.0.1", Some("http://evil.localhost")),
        403
    );
}

#[test]
fn http_loopback_hosts_and_origins_on_any_port() {
    let server = HttpServer::start(&[], &[]);
    // A port-forward or proxy may present another port.
    assert_eq!(status(&server, "localhost:9000", None), 200);
    assert_eq!(status(&server, "127.0.0.1", None), 200);
    assert_eq!(status(&server, "[::1]:9000", None), 200);
    assert_eq!(
        status(&server, &server.host(), Some("http://localhost:9999")),
        200
    );
    assert_eq!(
        status(&server, &server.host(), Some("http://[::1]:9999")),
        200
    );
    // Python's `urlsplit(...).hostname` survives a malformed port.
    assert_eq!(
        status(&server, &server.host(), Some("http://localhost:1:2")),
        200
    );
}

#[test]
fn http_skips_an_empty_origin() {
    let server = HttpServer::start(&[], &[]);
    assert_eq!(status(&server, "evil.example", Some("")), 421);
    assert_eq!(status(&server, &server.host(), Some("")), 200);
}

#[test]
fn http_same_origin_honors_forwarded_proto_from_a_loopback_proxy() {
    let server = HttpServer::start(&[], &[("FASTMCP_HTTP_ALLOWED_HOSTS", r#"["mcp.ali.fyi"]"#)]);
    let with = |origin: &str, proto: Option<&str>| {
        let mut headers = vec![("Origin", origin)];
        headers.extend(proto.map(|p| ("X-Forwarded-Proto", p)));
        http_post_with(
            server.port,
            "mcp.ali.fyi",
            None,
            &headers,
            &initialize_request(1),
        )
        .status
    };
    assert_eq!(with("https://mcp.ali.fyi", Some("https")), 200);
    assert_eq!(with("https://mcp.ali.fyi", None), 403);
    assert_eq!(with("http://mcp.ali.fyi", Some("https")), 403);
    assert_eq!(with("https://mcp.ali.fyi", Some("HTTPS")), 403);
}

#[test]
fn http_allows_hosts_and_origins_from_env() {
    let server = HttpServer::start(
        &[],
        &[
            (
                "FASTMCP_HTTP_ALLOWED_HOSTS",
                r#"["mcp.ali.fyi:443", "*.ts.net"]"#,
            ),
            ("FASTMCP_HTTP_ALLOWED_ORIGINS", r#"["https://*.ts.net"]"#),
        ],
    );
    let proxied = http_post(server.port, "mcp.ali.fyi", None, &initialize_request(1));
    assert_eq!(proxied.status, 200, "{proxied:?}");
    assert_eq!(proxied.json()["result"]["serverInfo"]["name"], "TickTick");
    assert_eq!(status(&server, "MCP.ali.fyi:8443", None), 200);
    assert_eq!(
        status(&server, "box.tail.ts.net", Some("https://box.tail.ts.net")),
        200
    );
    // Same-origin needs no allow-list entry.
    assert_eq!(
        status(&server, "mcp.ali.fyi", Some("http://mcp.ali.fyi")),
        200
    );
    assert_eq!(
        status(&server, "mcp.ali.fyi", Some("https://other.example")),
        403
    );
    assert_eq!(status(&server, "other.ali.fyi", None), 421);
}

#[test]
fn sigterm_and_sigint_shut_down_with_an_sse_stream_open() {
    for sig in [libc::SIGTERM, libc::SIGINT] {
        let mut server = HttpServer::start(&[], &[]);
        let session = server.session();
        let _sse = server.open_sse(&session);
        server.signal(sig);
        // Under the 5s drain backstop, so the open stream must be cancelled.
        let status = server.wait_exit(Duration::from_secs(3));
        assert!(
            status.is_some_and(|s| s.success()),
            "signal {sig}: {status:?}"
        );
    }
}

/// An HTTP server whose `GET /project` answers after `delay`, with a
/// `list_projects` call to it in flight.
fn slow_call(
    api: &MockServer,
    delay: Duration,
) -> (HttpServer, std::thread::JoinHandle<HttpResponse>) {
    api.mock(|when, then| {
        when.method(GET).path("/open/v1/project");
        then.status(200)
            .json_body(json!([{"id": "p1", "name": "Work"}]))
            .delay(delay);
    });
    let server = HttpServer::start(&[], &[("TICKTICK_API_BASE", &api.url("/open/v1"))]);
    let session = server.session();
    let (port, host) = (server.port, server.host());
    let call = std::thread::spawn(move || {
        let req = json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call",
                         "params": {"name": "list_projects", "arguments": {}}});
        http_post(port, &host, Some(&session), &req)
    });
    std::thread::sleep(Duration::from_millis(400));
    (server, call)
}

#[test]
fn sigterm_lets_an_in_flight_call_finish_then_exits() {
    let api = MockServer::start();
    let (mut server, call) = slow_call(&api, Duration::from_millis(1500));
    server.signal(libc::SIGTERM);
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        std::net::TcpStream::connect(("127.0.0.1", server.port)).is_err(),
        "still accepting connections"
    );
    let resp = call.join().unwrap();
    assert_eq!(resp.status, 200, "{resp:?}");
    let result = &resp.json()["result"];
    assert_eq!(result["isError"], false, "{resp:?}");
    assert_eq!(
        result["content"][0]["text"],
        r#"[{"id":"p1","name":"Work"}]"#
    );
    let status = server.wait_exit(Duration::from_secs(2));
    assert!(status.is_some_and(|s| s.success()), "{status:?}");
}

#[test]
fn a_stuck_call_does_not_hold_shutdown_past_the_grace_period() {
    let api = MockServer::start();
    let (mut server, _call) = slow_call(&api, Duration::from_secs(30));
    server.signal(libc::SIGTERM);
    // 5s grace, plus the backstops.
    let status = server.wait_exit(Duration::from_secs(8));
    assert!(status.is_some(), "still running");
}

#[test]
fn a_second_signal_exits_at_once() {
    use std::os::unix::process::ExitStatusExt;
    let api = MockServer::start();
    let (mut server, _call) = slow_call(&api, Duration::from_secs(30));
    server.signal(libc::SIGTERM);
    std::thread::sleep(Duration::from_millis(300));
    server.signal(libc::SIGINT);
    let status = server
        .wait_exit(Duration::from_secs(1))
        .expect("still running");
    assert_eq!(status.code(), Some(130), "{status:?}");
    assert_eq!(status.signal(), None);
}

#[test]
fn a_signal_right_after_the_address_line_shuts_down_cleanly() {
    for sig in [libc::SIGINT, libc::SIGTERM] {
        let mut server = HttpServer::start(&[], &[]);
        server.signal(sig);
        let status = server.wait_exit(Duration::from_secs(3));
        assert!(
            status.is_some_and(|s| s.success()),
            "signal {sig}: {status:?}"
        );
    }
}

#[test]
fn idle_session_expires_after_the_env_timeout() {
    let server = HttpServer::start(&[], &[("TICKTICK_MCP_SESSION_TIMEOUT", "1")]);
    let session = server.session();
    let list = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}});
    let live = http_post(server.port, &server.host(), Some(&session), &list);
    assert_eq!(live.status, 200, "{live:?}");
    std::thread::sleep(Duration::from_millis(2500));
    let gone = http_post(server.port, &server.host(), Some(&session), &list);
    assert_eq!(gone.status, 404, "{gone:?}");
}

#[test]
fn bad_session_timeout_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let out = isolated_command(dir.path())
        .args(["mcp", "--http", "--port", "0"])
        .env("TICKTICK_MCP_SESSION_TIMEOUT", "soon")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("TICKTICK_MCP_SESSION_TIMEOUT"), "{stderr}");
}

#[test]
fn http_transport_and_port_from_env() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = isolated_command(dir.path());
    cmd.args(["mcp"])
        .env("TICKTICK_MCP_TRANSPORT", "http")
        .env("TICKTICK_MCP_HOST", "127.0.0.1")
        .env("TICKTICK_MCP_PORT", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let mut stderr = std::io::BufReader::new(child.stderr.take().unwrap());
    let mut line = String::new();
    std::io::BufRead::read_line(&mut stderr, &mut line).unwrap();
    let _ = child.kill();
    let _ = child.wait();
    assert!(line.contains("listening on http://127.0.0.1:"), "{line}");
}

#[test]
fn non_loopback_host_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let out = isolated_command(dir.path())
        .args(["mcp", "--http", "--host", "0.0.0.0", "--port", "0"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("loopback"), "{stderr}");

    let out = isolated_command(dir.path())
        .args(["mcp"])
        .env("TICKTICK_MCP_TRANSPORT", "http")
        .env("TICKTICK_MCP_HOST", "192.168.1.10")
        .env("TICKTICK_MCP_PORT", "0")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("loopback"));

    let out = isolated_command(dir.path())
        .args(["mcp"])
        .env("TICKTICK_MCP_TRANSPORT", "carrier-pigeon")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("TICKTICK_MCP_TRANSPORT"));
}

#[test]
fn host_or_port_without_http_is_refused_not_ignored() {
    let dir = tempfile::tempdir().unwrap();
    for args in [["mcp", "--host", "0.0.0.0"], ["mcp", "--port", "9000"]] {
        let out = isolated_command(dir.path())
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(!out.status.success(), "{args:?} served stdio");
        assert!(out.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("--http"), "{stderr}");
    }
}

#[test]
fn port_flag_applies_when_the_env_picks_http() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = isolated_command(dir.path())
        .args(["mcp", "--port", "0"])
        .env("TICKTICK_MCP_TRANSPORT", "http")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = std::io::BufReader::new(child.stderr.take().unwrap());
    let mut line = String::new();
    std::io::BufRead::read_line(&mut stderr, &mut line).unwrap();
    let _ = child.kill();
    let _ = child.wait();
    assert!(line.contains("listening on http://127.0.0.1:"), "{line}");
}
