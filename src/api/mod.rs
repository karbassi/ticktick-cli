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

use crate::config::{self, RefreshedTokens};

const DEFAULT_BASE_URL: &str = "https://api.ticktick.com/open/v1";

const CONNECT_TIMEOUT_SECS: u64 = 10;
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// `TICKTICK_HTTP_TIMEOUT_SECS` (0 or unparsable: the default); it also caps
/// the connect timeout.
fn timeout_secs() -> u64 {
    std::env::var("TICKTICK_HTTP_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .filter(|&s| s > 0)
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
}

/// Every request goes through this agent so none can stall forever.
pub fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let secs = timeout_secs();
        ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(secs.min(CONNECT_TIMEOUT_SECS)))
            .timeout_read(Duration::from_secs(secs))
            .timeout_write(Duration::from_secs(secs))
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

fn is_unauthorized(err: &ureq::Error) -> bool {
    matches!(err, ureq::Error::Status(401, _))
}

/// A fresh access token after `failed` got a 401. The result is kept in memory
/// for the rest of the process; config.json is updated only when its refresh
/// token was used, and a failed write is ignored.
fn try_refresh_token(failed: &str) -> Option<String> {
    // Concurrent 401s refresh once. `get_access_token` never takes this lock.
    static REFRESHING: Mutex<()> = Mutex::new(());
    let _refreshing = REFRESHING.lock().unwrap_or_else(|e| e.into_inner());
    let (refresh, persist) = match config::refreshed_tokens() {
        // Another request already refreshed.
        Some(r) if r.access != failed => return Some(r.access),
        Some(RefreshedTokens {
            refresh: Some(t),
            persist,
            ..
        }) => (t, persist),
        _ => config::get_refresh_token()?,
    };

    let (access, rotated) = auth::refresh_token(&refresh).ok()?;
    let refresh = rotated.unwrap_or(refresh);
    if persist {
        let _ = config::update(|cfg| {
            cfg.access_token = Some(access.clone());
            cfg.refresh_token = Some(refresh.clone());
        });
    }
    config::set_refreshed_tokens(RefreshedTokens {
        access: access.clone(),
        refresh: Some(refresh),
        persist,
    });
    eprintln!("Token refreshed automatically");
    Some(access)
}

pub fn get(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    let base = base_url();
    if crate::output::verbose() >= 1 {
        eprintln!("> GET {base}{endpoint}");
    }
    let result = agent()
        .get(&format!("{base}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token(token) {
                agent()
                    .get(&format!("{base}{endpoint}"))
                    .set("Authorization", &format!("Bearer {new_token}"))
                    .call()
                    .map_err(|e| format!("API request failed: {e}"))
            } else {
                Err(
                    "token expired\n\n  hint: Run 'ticktick-cli login' to re-authenticate"
                        .to_string(),
                )
            }
        }
        Err(e) => Err(format!("API request failed: {e}")),
    }
}

pub fn post(
    endpoint: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<ureq::Response, String> {
    let base = base_url();
    if crate::output::verbose() >= 1 {
        eprintln!("> POST {base}{endpoint}");
    }
    let result = agent()
        .post(&format!("{base}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(body.clone());

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token(token) {
                agent()
                    .post(&format!("{base}{endpoint}"))
                    .set("Authorization", &format!("Bearer {new_token}"))
                    .set("Content-Type", "application/json")
                    .send_json(body.clone())
                    .map_err(|e| format!("API request failed: {e}"))
            } else {
                Err(
                    "token expired\n\n  hint: Run 'ticktick-cli login' to re-authenticate"
                        .to_string(),
                )
            }
        }
        Err(e) => Err(format!("API request failed: {e}")),
    }
}

pub fn post_empty(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    let base = base_url();
    if crate::output::verbose() >= 1 {
        eprintln!("> POST {base}{endpoint}");
    }
    let result = agent()
        .post(&format!("{base}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token(token) {
                agent()
                    .post(&format!("{base}{endpoint}"))
                    .set("Authorization", &format!("Bearer {new_token}"))
                    .call()
                    .map_err(|e| format!("API request failed: {e}"))
            } else {
                Err(
                    "token expired\n\n  hint: Run 'ticktick-cli login' to re-authenticate"
                        .to_string(),
                )
            }
        }
        Err(e) => Err(format!("API request failed: {e}")),
    }
}

/// Send a JSON body with any method, refreshing the token once on 401.
fn send_json(
    method: &str,
    endpoint: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<ureq::Response, String> {
    let base = base_url();
    if crate::output::verbose() >= 1 {
        eprintln!("> {method} {base}{endpoint}");
    }
    let call = |t: &str| {
        agent()
            .request(method, &format!("{base}{endpoint}"))
            .set("Authorization", &format!("Bearer {t}"))
            .set("Content-Type", "application/json")
            .send_json(body.clone())
            .map_err(Box::new)
    };
    match call(token) {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => match try_refresh_token(token) {
            Some(new_token) => call(&new_token).map_err(|e| format!("API request failed: {e}")),
            None => Err(
                "token expired\n\n  hint: Run 'ticktick-cli login' to re-authenticate".to_string(),
            ),
        },
        Err(e) => Err(format!("API request failed: {e}")),
    }
}

pub fn put(
    endpoint: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<ureq::Response, String> {
    send_json("PUT", endpoint, token, body)
}

/// DELETE with a JSON body (e.g. `/tag/delete`).
pub fn delete_json(
    endpoint: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<ureq::Response, String> {
    send_json("DELETE", endpoint, token, body)
}

pub fn delete(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    let base = base_url();
    if crate::output::verbose() >= 1 {
        eprintln!("> DELETE {base}{endpoint}");
    }
    let result = agent()
        .delete(&format!("{base}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token(token) {
                agent()
                    .delete(&format!("{base}{endpoint}"))
                    .set("Authorization", &format!("Bearer {new_token}"))
                    .call()
                    .map_err(|e| format!("API request failed: {e}"))
            } else {
                Err(
                    "token expired\n\n  hint: Run 'ticktick-cli login' to re-authenticate"
                        .to_string(),
                )
            }
        }
        Err(e) => Err(format!("API request failed: {e}")),
    }
}
