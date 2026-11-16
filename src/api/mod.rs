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

use crate::config::{self, RefreshedTokens};

const DEFAULT_BASE_URL: &str = "https://api.ticktick.com/open/v1";

/// Read a base URL from `var`, falling back to `default`. Tests point these at a
/// mock server; a trailing slash is dropped so endpoints can start with `/`.
pub fn env_url(var: &str, default: &str) -> String {
    match std::env::var(var) {
        Ok(v) if !v.trim().is_empty() => v.trim().trim_end_matches('/').to_string(),
        _ => default.to_string(),
    }
}

/// Base URL of the official Open API (`TICKTICK_API_BASE` overrides it).
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
    let mut refreshed = config::refreshed_tokens();
    let (refresh, persist) = match refreshed.as_ref() {
        // Another request already refreshed.
        Some(r) if r.access != failed => return Some(r.access.clone()),
        Some(RefreshedTokens {
            refresh: Some(t),
            persist,
            ..
        }) => (t.clone(), *persist),
        _ => config::get_refresh_token()?,
    };

    let (access, rotated) = auth::refresh_token(&refresh).ok()?;
    let refresh = rotated.unwrap_or(refresh);
    if persist {
        let mut cfg = config::load();
        cfg.access_token = Some(access.clone());
        cfg.refresh_token = Some(refresh.clone());
        let _ = config::save(&cfg);
    }
    *refreshed = Some(RefreshedTokens {
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
    let result = ureq::get(&format!("{base}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token(token) {
                ureq::get(&format!("{base}{endpoint}"))
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
    let result = ureq::post(&format!("{base}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(body.clone());

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token(token) {
                ureq::post(&format!("{base}{endpoint}"))
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
    let result = ureq::post(&format!("{base}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token(token) {
                ureq::post(&format!("{base}{endpoint}"))
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
        ureq::request(method, &format!("{base}{endpoint}"))
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
    let result = ureq::delete(&format!("{base}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token(token) {
                ureq::delete(&format!("{base}{endpoint}"))
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
