pub mod auth;
pub mod project;
pub mod task;

use crate::config;

const BASE_URL: &str = "https://api.ticktick.com/open/v1";

fn is_unauthorized(err: &ureq::Error) -> bool {
    matches!(err, ureq::Error::Status(401, _))
}

fn try_refresh_token() -> Option<String> {
    let cfg = config::load();
    let refresh = cfg.refresh_token.as_ref()?;

    match auth::refresh_token(refresh) {
        Ok((new_access, new_refresh)) => {
            let mut cfg = config::load();
            cfg.access_token = Some(new_access.clone());
            if let Some(r) = new_refresh {
                cfg.refresh_token = Some(r);
            }
            let _ = config::save(&cfg);
            eprintln!("Token refreshed automatically");
            Some(new_access)
        }
        Err(_) => None,
    }
}

pub fn get(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    if crate::output::verbose() >= 1 {
        eprintln!("> GET {BASE_URL}{endpoint}");
    }
    let result = ureq::get(&format!("{BASE_URL}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token() {
                ureq::get(&format!("{BASE_URL}{endpoint}"))
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
    if crate::output::verbose() >= 1 {
        eprintln!("> POST {BASE_URL}{endpoint}");
    }
    let result = ureq::post(&format!("{BASE_URL}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(body.clone());

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token() {
                ureq::post(&format!("{BASE_URL}{endpoint}"))
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
    if crate::output::verbose() >= 1 {
        eprintln!("> POST {BASE_URL}{endpoint}");
    }
    let result = ureq::post(&format!("{BASE_URL}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token() {
                ureq::post(&format!("{BASE_URL}{endpoint}"))
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

pub fn delete(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    if crate::output::verbose() >= 1 {
        eprintln!("> DELETE {BASE_URL}{endpoint}");
    }
    let result = ureq::delete(&format!("{BASE_URL}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call();

    match result {
        Ok(resp) => Ok(resp),
        Err(e) if is_unauthorized(&e) => {
            if let Some(new_token) = try_refresh_token() {
                ureq::delete(&format!("{BASE_URL}{endpoint}"))
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
