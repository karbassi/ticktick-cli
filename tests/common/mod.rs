//! Spawned servers run isolated (temp dir as cwd, `HOME` and `XDG_CONFIG_HOME`)
//! so no real `.env` or config.json leaks in. `TZ` is `America/Chicago`.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use httpmock::MockServer;
use serde_json::{Value, json};

pub const ACCESS_TOKEN: &str = "test-access-token";
pub const SESSION_TOKEN: &str = "test-session-token";
/// A 24-char hex project ID: resolved without a `GET /project` lookup.
pub const HEX_PROJECT: &str = "aaaaaaaaaaaaaaaaaaaaaaaa";

const TIMEOUT: Duration = Duration::from_secs(30);

pub fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ticktick-cli")
}

pub fn isolated_command(dir: &Path) -> Command {
    let mut cmd = Command::new(bin());
    cmd.current_dir(dir)
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env("TZ", "America/Chicago");
    for var in [
        "TICKTICK_ACCESS_TOKEN",
        "TICKTICK_V2_SESSION_TOKEN",
        "TICKTICK_CLIENT_ID",
        "TICKTICK_CLIENT_SECRET",
        "TICKTICK_API_BASE",
        "TICKTICK_V2_BASE",
        "TICKTICK_MS_BASE",
        "TICKTICK_OAUTH_TOKEN_URL",
        "TICKTICK_MCP_TRANSPORT",
        "TICKTICK_MCP_HOST",
        "TICKTICK_MCP_PORT",
        "FASTMCP_HTTP_ALLOWED_HOSTS",
        "FASTMCP_HTTP_ALLOWED_ORIGINS",
    ] {
        cmd.env_remove(var);
    }
    cmd
}

pub fn write_config(dir: &Path, config: &Value) {
    let cfg_dir = dir.join("ticktick-cli");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(cfg_dir.join("config.json"), config.to_string()).unwrap();
}

pub struct McpBuilder {
    dir: tempfile::TempDir,
    envs: Vec<(String, String)>,
    args: Vec<String>,
}

impl McpBuilder {
    pub fn new(server: &MockServer) -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
            envs: vec![
                ("TICKTICK_ACCESS_TOKEN".into(), ACCESS_TOKEN.into()),
                ("TICKTICK_V2_SESSION_TOKEN".into(), SESSION_TOKEN.into()),
                ("TICKTICK_API_BASE".into(), server.url("/open/v1")),
                ("TICKTICK_V2_BASE".into(), server.url("/api/v2")),
                ("TICKTICK_MS_BASE".into(), server.url("/ms")),
            ],
            args: vec!["mcp".into()],
        }
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.envs.retain(|(k, _)| k != key);
        self.envs.push((key.into(), value.into()));
        self
    }

    pub fn config(self, config: Value) -> Self {
        write_config(self.dir.path(), &config);
        self
    }

    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn start(self) -> McpClient {
        let mut client = self.spawn();
        client.initialize();
        client
    }

    pub fn spawn(self) -> McpClient {
        let mut cmd = isolated_command(self.dir.path());
        cmd.args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().expect("spawn ticktick-cli mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        McpClient {
            child,
            stdin: Some(stdin),
            lines: rx,
            next_id: 1,
            _dir: self.dir,
        }
    }
}

#[derive(Debug)]
pub struct ToolCall {
    pub is_error: bool,
    pub text: String,
    pub raw: Value,
}

impl ToolCall {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.text)
            .unwrap_or_else(|e| panic!("tool output is not JSON ({e}): {}", self.text))
    }

    pub fn ok(self) -> Self {
        assert!(!self.is_error, "tool call failed: {}", self.text);
        self
    }
}

pub struct McpClient {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    next_id: u64,
    _dir: tempfile::TempDir,
}

impl McpClient {
    pub fn send(&mut self, msg: &Value) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        writeln!(stdin, "{msg}").unwrap();
        stdin.flush().unwrap();
    }

    /// Every stdout line must be JSON-RPC.
    pub fn recv(&mut self) -> Value {
        let line = self
            .lines
            .recv_timeout(TIMEOUT)
            .expect("timed out waiting for the MCP server");
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("non-JSON on stdout ({e}): {line}"))
    }

    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let msg = self.recv();
            if msg.get("id") == Some(&json!(id)) {
                return msg;
            }
        }
    }

    pub fn result(&mut self, method: &str, params: Value) -> Value {
        let resp = self.request(method, params);
        assert!(resp.get("error").is_none(), "{method} failed: {resp}");
        resp["result"].clone()
    }

    pub fn initialize(&mut self) -> Value {
        let result = self.result(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "ticktick-cli-tests", "version": "0"}
            }),
        );
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        result
    }

    pub fn list_tools(&mut self) -> Vec<Value> {
        let result = self.result("tools/list", json!({}));
        result["tools"].as_array().unwrap().clone()
    }

    pub fn call_tool(&mut self, name: &str, args: Value) -> ToolCall {
        let result = self.result("tools/call", json!({"name": name, "arguments": args}));
        let text = result["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter_map(|b| b["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();
        ToolCall {
            is_error: result["isError"].as_bool().unwrap_or(false),
            text,
            raw: result,
        }
    }

    pub fn list_resources(&mut self) -> Vec<Value> {
        let result = self.result("resources/list", json!({}));
        result["resources"].as_array().unwrap().clone()
    }

    pub fn read_resource(&mut self, uri: &str) -> Value {
        let result = self.result("resources/read", json!({"uri": uri}));
        let text = result["contents"][0]["text"].as_str().unwrap();
        serde_json::from_str(text).unwrap()
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct HttpServer {
    child: Child,
    pub port: u16,
    _dir: tempfile::TempDir,
}

impl HttpServer {
    pub fn start(args: &[&str], envs: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut cmd = isolated_command(dir.path());
        cmd.args(["mcp", "--http", "--port", "0"])
            .args(args)
            .env("TICKTICK_ACCESS_TOKEN", ACCESS_TOKEN)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        for (k, v) in envs {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().expect("spawn ticktick-cli mcp --http");
        let stderr = child.stderr.take().unwrap();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let port = loop {
            let line = rx
                .recv_timeout(TIMEOUT)
                .expect("HTTP server did not report its address");
            if let Some(rest) = line.split("http://127.0.0.1:").nth(1) {
                break rest.trim_end_matches("/mcp").parse().unwrap();
            }
        };
        Self {
            child,
            port,
            _dir: dir,
        }
    }

    pub fn host(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn initialize_request(id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "ticktick-cli-tests", "version": "0"}
        }
    })
}

#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl HttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn json(&self) -> Value {
        if let Ok(v) = serde_json::from_str(&self.body) {
            return v;
        }
        self.body
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .filter_map(|d| serde_json::from_str(d.trim()).ok())
            .next()
            .unwrap_or_else(|| panic!("no JSON in response body: {}", self.body))
    }
}

/// Raw socket so the `Host` header can be anything.
pub fn http_post(
    port: u16,
    host_header: &str,
    session: Option<&str>,
    body: &Value,
) -> HttpResponse {
    http_post_with(port, host_header, session, &[], body)
}

pub fn http_post_with(
    port: u16,
    host_header: &str,
    session: Option<&str>,
    extra_headers: &[(&str, &str)],
    body: &Value,
) -> HttpResponse {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.set_read_timeout(Some(TIMEOUT)).unwrap();
    let body = body.to_string();
    let mut req = format!(
        "POST /mcp HTTP/1.1\r\nHost: {host_header}\r\nContent-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nContent-Length: {}\r\n\
         MCP-Protocol-Version: 2025-06-18\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(s) = session {
        req.push_str(&format!("Mcp-Session-Id: {s}\r\n"));
    }
    for (k, v) in extra_headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(&body);
    stream.write_all(req.as_bytes()).unwrap();

    // SSE streams may stay open, so also stop at a complete response event.
    let mut raw = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                raw.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&raw);
                if text.contains("\"jsonrpc\"") && text.contains("\n\n") && sse_done(&text) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    parse_http(&String::from_utf8_lossy(&raw))
}

fn sse_done(text: &str) -> bool {
    text.lines().any(|l| {
        l.strip_prefix("data:")
            .and_then(|d| serde_json::from_str::<Value>(d.trim()).ok())
            .is_some_and(|v| v.get("result").is_some() || v.get("error").is_some())
    })
}

fn parse_http(raw: &str) -> HttpResponse {
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((raw, ""));
    let mut lines = head.lines();
    let status_line = lines.next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();
    let chunked = headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("transfer-encoding") && v.eq_ignore_ascii_case("chunked")
    });
    let body = if chunked {
        dechunk(body)
    } else {
        body.to_string()
    };
    HttpResponse {
        status,
        headers,
        body,
    }
}

fn dechunk(mut body: &str) -> String {
    let mut out = String::new();
    while let Some((size, rest)) = body.split_once("\r\n") {
        let Ok(n) = usize::from_str_radix(size.trim(), 16) else {
            break;
        };
        if n == 0 {
            break;
        }
        let n = n.min(rest.len());
        out.push_str(&rest[..n]);
        body = rest[n..].strip_prefix("\r\n").unwrap_or(&rest[n..]);
    }
    out
}
