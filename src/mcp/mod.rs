//! Drop-in replacement for the Python `ticktick-mcp` server (contract:
//! `tests/mcp_contract.rs`). See `src/mcp/README.md` for adding a tool module.

mod coerce;
mod guard;
mod tools;
pub mod util;

use std::future::IntoFuture;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, Implementation, ListResourcesResult,
    PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult,
    Resource, ResourceContents, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, tool_handler};
use serde_json::Value;
use tokio::signal::unix::{SignalKind, signal};

#[derive(Clone)]
pub struct TickTickServer {
    tool_router: ToolRouter<Self>,
}

impl Default for TickTickServer {
    fn default() -> Self {
        Self::new()
    }
}

impl TickTickServer {
    pub fn new() -> Self {
        Self {
            tool_router: tools::router(),
        }
    }
}

const RESOURCES: &[(&str, &str, &str)] = &[
    (
        "ticktick://projects",
        "projects",
        "All projects (task lists) with their IDs, names, colors, and folder assignments.",
    ),
    (
        "ticktick://tags",
        "tags",
        "All tags with their names, colors, and hierarchical relationships.",
    ),
    (
        "ticktick://settings",
        "settings",
        "User preference settings including web-specific options.",
    ),
    (
        "ticktick://profile",
        "profile",
        "User profile and subscription status merged into one view.",
    ),
];

fn read_resource_json(uri: &str) -> Result<Value, String> {
    match uri {
        "ticktick://projects" => util::v1_get("/project"),
        "ticktick://tags" => util::v1_get("/tag"),
        "ticktick://settings" => util::v2_get("/user/preferences/settings?includeWeb=true"),
        "ticktick://profile" => {
            let mut profile = util::v2_get("/user/profile")?;
            let status = util::v2_get("/user/status")?;
            match profile.as_object_mut() {
                Some(obj) => {
                    obj.insert("status".to_string(), status);
                }
                None => profile = serde_json::json!({ "status": status }),
            }
            Ok(profile)
        }
        _ => Err(format!("Unknown resource: {uri}")),
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for TickTickServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("TickTick", env!("CARGO_PKG_VERSION")))
        .with_instructions(
            "TickTick tasks, projects, tags and more. Projects can be named fuzzily; \
             use \"inbox\" for the default inbox.",
        )
    }

    async fn call_tool(
        &self,
        mut request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if let (Some(tool), Some(args)) = (
            self.tool_router.get(&request.name),
            request.arguments.as_mut(),
        ) {
            coerce::coerce_arguments(&tool.input_schema, args);
        }
        self.tool_router
            .call(ToolCallContext::new(self, request, context))
            .await
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let resources = RESOURCES
            .iter()
            .map(|(uri, name, description)| {
                Resource::new(*uri, *name)
                    .with_description(*description)
                    .with_mime_type("text/plain")
            })
            .collect();
        Ok(ListResourcesResult::with_all_items(resources))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let uri = request.uri;
        if !RESOURCES.iter().any(|(u, _, _)| *u == uri) {
            return Err(ErrorData::resource_not_found(
                format!("Unknown resource: {uri}"),
                None,
            ));
        }
        let key = uri.clone();
        let value = util::blocking(move || read_resource_json(&key))
            .await
            .map_err(|e| ErrorData::internal_error(e, None))?;
        Ok(ReadResourceResult::new(vec![ResourceContents::text(value.to_string(), uri)]).into())
    }
}

/// The endpoint has no auth, so only loopback may be bound.
const LOOPBACK_HOSTS: &[&str] = &["127.0.0.1", "localhost", "::1"];

pub struct McpOptions {
    pub http: bool,
    pub host: Option<String>,
    pub port: Option<u16>,
}

enum Transport {
    Stdio,
    Http { host: String, port: u16 },
}

fn env_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn transport(opts: McpOptions) -> Result<Transport, String> {
    let http = if opts.http {
        true
    } else {
        match env_var("TICKTICK_MCP_TRANSPORT").as_deref() {
            None | Some("stdio") => false,
            Some("http") => true,
            Some(other) => {
                return Err(format!(
                    "TICKTICK_MCP_TRANSPORT must be 'stdio' or 'http', got '{other}'"
                ));
            }
        }
    };
    if !http {
        return Ok(Transport::Stdio);
    }
    let host = opts
        .host
        .or_else(|| env_var("TICKTICK_MCP_HOST"))
        .unwrap_or_else(|| "127.0.0.1".to_string());
    if !LOOPBACK_HOSTS.contains(&host.as_str()) {
        return Err(format!(
            "MCP host must be a loopback address (127.0.0.1, localhost, ::1), got '{host}'\n\n  \
             hint: the HTTP endpoint has no auth; put an authenticating proxy in front for remote use"
        ));
    }
    let port = match opts.port {
        Some(p) => p,
        None => match env_var("TICKTICK_MCP_PORT") {
            Some(p) => p
                .parse()
                .map_err(|_| format!("TICKTICK_MCP_PORT must be a port number, got '{p}'"))?,
            None => 8502,
        },
    };
    Ok(Transport::Http { host, port })
}

/// Same env format as FastMCP's `FASTMCP_HTTP_ALLOWED_*`.
fn env_json_list(name: &str) -> Result<Vec<String>, String> {
    match env_var(name) {
        None => Ok(Vec::new()),
        Some(raw) => serde_json::from_str::<Vec<String>>(&raw)
            .map_err(|e| format!("{name} must be a JSON array of strings: {e}")),
    }
}

/// Idle time before a session is dropped. rmcp defaults to 300s; the Python
/// server allowed 1800s.
fn session_timeout() -> Result<Duration, String> {
    match env_var("TICKTICK_MCP_SESSION_TIMEOUT") {
        None => Ok(Duration::from_secs(1800)),
        Some(s) => s
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0)
            .map(Duration::from_secs)
            .ok_or_else(|| {
                format!(
                    "TICKTICK_MCP_SESSION_TIMEOUT must be a positive number of seconds, got '{s}'"
                )
            }),
    }
}

/// Logs go to stderr only: stdout carries the stdio transport.
pub fn run(opts: McpOptions) -> Result<(), String> {
    // `main` restores SIGPIPE's default for the CLI; a long-running server must
    // not die when a peer resets mid-write.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
    let transport = transport(opts)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("failed to start async runtime: {e}"))?;
    runtime.block_on(async move {
        match transport {
            Transport::Stdio => serve_stdio().await,
            Transport::Http { host, port } => serve_http(&host, port).await,
        }
    })
}

async fn serve_stdio() -> Result<(), String> {
    let service = TickTickServer::new()
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| format!("MCP stdio startup failed: {e}"))?;
    service
        .waiting()
        .await
        .map_err(|e| format!("MCP server stopped: {e}"))?;
    Ok(())
}

async fn serve_http(host: &str, port: u16) -> Result<(), String> {
    let ip = if host == "localhost" {
        "127.0.0.1"
    } else {
        host
    };
    let addr: SocketAddr = format!("{ip}:{port}")
        .parse()
        .or_else(|_| format!("[{ip}]:{port}").parse())
        .map_err(|e| format!("invalid address {host}:{port}: {e}"))?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("failed to bind {addr}: {e}"))?;
    let bound = listener
        .local_addr()
        .map_err(|e| format!("failed to read bound address: {e}"))?;
    // DNS-rebinding protection; env hosts cover e.g. a reverse proxy's name.
    let guard = guard::Guard::new(
        env_json_list("FASTMCP_HTTP_ALLOWED_HOSTS")?,
        env_json_list("FASTMCP_HTTP_ALLOWED_ORIGINS")?,
    );
    let config = StreamableHttpServerConfig::default()
        .disable_allowed_hosts()
        .disable_allowed_origins();
    let cancel = config.cancellation_token.clone();
    let mut sessions = LocalSessionManager::default();
    sessions.session_config.keep_alive = Some(session_timeout()?);
    let server = TickTickServer::new();
    let service =
        StreamableHttpService::new(move || Ok(server.clone()), Arc::new(sessions), config);
    let app = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(axum::middleware::from_fn(move |req, next| {
            guard.clone().check(req, next)
        }));

    let mut sigterm = signal(SignalKind::terminate())
        .map_err(|e| format!("failed to listen for SIGTERM: {e}"))?;
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let shutdown = async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
        // Open SSE streams would otherwise hold the graceful drain forever.
        cancel.cancel();
        let _ = stop_tx.send(());
    };

    eprintln!("ticktick-cli MCP server listening on http://{bound}/mcp");
    let serve = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .into_future();
    tokio::pin!(serve);
    tokio::select! {
        res = &mut serve => return res.map_err(|e| format!("MCP HTTP server failed: {e}")),
        _ = stop_rx => {}
    }
    match tokio::time::timeout(SHUTDOWN_GRACE, serve).await {
        Ok(res) => res.map_err(|e| format!("MCP HTTP server failed: {e}")),
        Err(_) => Ok(()),
    }
}

/// How long in-flight requests get to finish after SIGINT/SIGTERM.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);
