pub mod auth;
pub mod calendar;
pub mod column;
pub mod comment;
pub mod countdown;
pub mod filter;
pub mod focus;
pub mod habit;
pub mod project;
pub mod project_group;
pub mod tag;
pub mod task;
pub mod v2;

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use crate::config;

const DEFAULT_BASE_URL: &str = "https://api.ticktick.com/open/v1";

const CONNECT_TIMEOUT_SECS: u64 = 10;
const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// ureq's default of 1 makes parallel requests to one host reconnect.
const IDLE_CONNECTIONS_PER_HOST: usize = 8;

/// `TICKTICK_HTTP_TIMEOUT_SECS` (0 or unparsable: the default); it also caps
/// the connect timeout.
fn timeout_secs() -> u64 {
    std::env::var("TICKTICK_HTTP_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .filter(|&s| s > 0)
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
}

/// Every request goes through this agent so none can stall forever. The
/// overall timeout also caps a response that trickles in under the read timeout.
pub fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let secs = timeout_secs();
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(secs))
            .timeout_connect(Duration::from_secs(secs.min(CONNECT_TIMEOUT_SECS)))
            .timeout_read(Duration::from_secs(secs))
            .timeout_write(Duration::from_secs(secs))
            .max_idle_connections_per_host(IDLE_CONNECTIONS_PER_HOST)
            .build()
    })
}

/// Overridable so tests can use a mock server.
pub fn env_url(var: &str, default: &str) -> String {
    match std::env::var(var) {
        Ok(v) if !v.trim().is_empty() => v.trim().trim_end_matches('/').to_string(),
        _ => default.to_string(),
    }
}

pub fn base_url() -> String {
    env_url("TICKTICK_API_BASE", DEFAULT_BASE_URL)
}

/// A fresh access token after `failed` got a 401.
fn try_refresh_token(failed: &str) -> Option<String> {
    // Concurrent 401s refresh once. `get_access_token` never takes this lock.
    static REFRESHING: Mutex<()> = Mutex::new(());
    let _refreshing = REFRESHING.lock().unwrap_or_else(|e| e.into_inner());
    // Another request, or a login in another process, already replaced it.
    let current = config::get_access_token().ok()?;
    if current != failed {
        return Some(current);
    }

    let plan = config::plan_refresh()?;
    // Fails when another process already rotated the refresh token, but that
    // process may have saved its new tokens in the meantime.
    let Ok((access, rotated)) = auth::refresh_token(&plan.refresh) else {
        return config::get_access_token().ok().filter(|t| t != failed);
    };
    let refresh = rotated.unwrap_or_else(|| plan.refresh.clone());
    let token = config::store_refresh(plan, access, refresh)?;
    eprintln!("Token refreshed automatically");
    Some(token)
}

/// Send a v1 request, refreshing the token and retrying once on 401. Without a
/// body the request has no Content-Type.
fn send(
    method: &str,
    endpoint: &str,
    token: &str,
    body: Option<&serde_json::Value>,
) -> Result<ureq::Response, String> {
    let url = format!("{}{endpoint}", base_url());
    if crate::output::verbose() >= 1 {
        eprintln!("> {method} {url}");
    }
    let call = |t: &str| {
        let req = agent()
            .request(method, &url)
            .set("Authorization", &format!("Bearer {t}"));
        match body {
            Some(b) => req
                .set("Content-Type", "application/json")
                .send_json(b.clone()),
            None => req.call(),
        }
        .map_err(Box::new)
    };
    match call(token) {
        Ok(resp) => Ok(resp),
        Err(e) if matches!(*e, ureq::Error::Status(401, _)) => match try_refresh_token(token) {
            Some(new_token) => call(&new_token).map_err(|e| format!("API request failed: {e}")),
            None => Err(config::expired_token_error()),
        },
        Err(e) => Err(format!("API request failed: {e}")),
    }
}

pub fn get(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    send("GET", endpoint, token, None)
}

pub fn post(
    endpoint: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<ureq::Response, String> {
    send("POST", endpoint, token, Some(body))
}

pub fn post_empty(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    send("POST", endpoint, token, None)
}

pub fn put(
    endpoint: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<ureq::Response, String> {
    send("PUT", endpoint, token, Some(body))
}

/// DELETE with a JSON body (e.g. `/tag/delete`).
pub fn delete_json(
    endpoint: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<ureq::Response, String> {
    send("DELETE", endpoint, token, Some(body))
}

pub fn delete(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    send("DELETE", endpoint, token, None)
}
