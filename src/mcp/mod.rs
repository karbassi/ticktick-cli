//! `ticktick-cli mcp`: an MCP server exposing TickTick as tools and resources.
//!
//! It is a drop-in replacement for the Python `ticktick-mcp` server: tool names,
//! parameter names, required sets and JSON types match it (see
//! `tests/fixtures/mcp-contract.json` and `tests/mcp_contract.rs`). See
//! `src/mcp/README.md` for how to add a tool module.

mod tools;
pub mod util;

use std::net::SocketAddr;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{
    Implementation, ListResourcesResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ReadResourceResult, Resource, ResourceContents, ServerCapabilities,
    ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, tool_handler};
use serde_json::Value;

/// The MCP server. Tools live in `tools/<module>.rs`, one router per module.
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

/// `(uri, name, description)` of each resource.
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

/// Fetch a resource's JSON (blocking).
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

// ---------------------------------------------------------------------------
// Transports
// ---------------------------------------------------------------------------

/// Hosts the HTTP transport may bind. The endpoint has no auth, so anything
/// else would hand the TickTick account to the network.
const LOOPBACK_HOSTS: &[&str] = &["127.0.0.1", "localhost", "::1"];

/// Options for `ticktick-cli mcp`; `None` falls back to env, then the default.
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

/// Resolve the transport: flags win over `TICKTICK_MCP_TRANSPORT`,
/// `TICKTICK_MCP_HOST` and `TICKTICK_MCP_PORT`.
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

/// A JSON array of strings from an env var (FastMCP's `FASTMCP_HTTP_ALLOWED_*`).
fn env_json_list(name: &str) -> Result<Vec<String>, String> {
    match env_var(name) {
        None => Ok(Vec::new()),
        Some(raw) => serde_json::from_str::<Vec<String>>(&raw)
            .map_err(|e| format!("{name} must be a JSON array of strings: {e}")),
    }
}

/// Run the server until the client disconnects (stdio) or the process is
/// stopped (HTTP). Logs go to stderr only.
pub fn run(opts: McpOptions) -> Result<(), String> {
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
    // Port 0 asks the OS for a free port; the allow-lists use the real one.
    let port = bound.port();

    // DNS-rebinding protection: only loopback Host headers on our port, plus
    // the extra hosts (e.g. the reverse proxy's name) from the env.
    let mut hosts = vec![
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        format!("[::1]:{port}"),
    ];
    hosts.extend(env_json_list("FASTMCP_HTTP_ALLOWED_HOSTS")?);
    let mut origins = vec![
        format!("http://127.0.0.1:{port}"),
        format!("http://localhost:{port}"),
    ];
    origins.extend(env_json_list("FASTMCP_HTTP_ALLOWED_ORIGINS")?);

    let config = StreamableHttpServerConfig::default()
        .with_allowed_hosts(hosts)
        .with_allowed_origins(origins);
    let service = StreamableHttpService::new(
        || Ok(TickTickServer::new()),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let app = axum::Router::new().nest_service("/mcp", service);

    eprintln!("ticktick-cli MCP server listening on http://{bound}/mcp");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|e| format!("MCP HTTP server failed: {e}"))
}
