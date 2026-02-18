use crate::config;
use serde::Serialize;

const V2_BASE_URL: &str = "https://api.ticktick.com/api/v2";

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

/// Build the x-device header JSON string.
pub fn x_device_header(device_id: &str) -> String {
    format!(
        r#"{{"platform":"web","os":"macOS 10.15.7","device":"Chrome 130.0.0.0","name":"","version":6490,"id":"{device_id}","channel":"website","campaign":"","websocket":""}}"#
    )
}

/// Sign in to the TickTick v2 API and return the session token.
fn signon(username: &str, password: &str) -> Result<String, String> {
    let device_id = config::get_or_create_device_id();
    let url = format!("{V2_BASE_URL}/user/signon?wc=true&remember=true");

    if crate::output::verbose() >= 1 {
        eprintln!("> POST {url}");
    }

    let body = serde_json::json!({
        "username": username,
        "password": password,
    });

    let resp = ureq::post(&url)
        .set("User-Agent", USER_AGENT)
        .set("x-device", &x_device_header(&device_id))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| format!("v2 sign-in failed: {e}"))?;

    let json: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("failed to parse v2 sign-in response: {e}"))?;

    let token = json["token"]
        .as_str()
        .ok_or_else(|| "v2 sign-in response missing token".to_string())?;

    config::save_v2_session_token(token)?;
    eprintln!("v2 session authenticated");

    Ok(token.to_string())
}

/// Get a v2 session token, using cached token or signing in.
pub fn get_session_token() -> Result<String, String> {
    if let Some(token) = config::get_v2_session_token() {
        return Ok(token);
    }
    let (username, password) = config::get_v2_credentials()?;
    signon(&username, &password)
}

/// Clear cached token and re-sign in.
fn refresh_session() -> Result<String, String> {
    let _ = config::clear_v2_session_token();
    let (username, password) = config::get_v2_credentials()?;
    signon(&username, &password)
}

/// Make a POST request to the v2 API with session auth.
/// Retries once on 401 by refreshing the session.
fn v2_post(
    endpoint: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<ureq::Response, String> {
    let device_id = config::get_or_create_device_id();
    let url = format!("{V2_BASE_URL}{endpoint}");

    if crate::output::verbose() >= 1 {
        eprintln!("> POST {url}");
    }

    let result = ureq::post(&url)
        .set("User-Agent", USER_AGENT)
        .set("x-device", &x_device_header(&device_id))
        .set("Cookie", &format!("t={token}"))
        .set("Content-Type", "application/json")
        .send_json(body.clone());

    match result {
        Ok(resp) => Ok(resp),
        Err(ureq::Error::Status(401, _)) => {
            let new_token = refresh_session()?;
            ureq::post(&url)
                .set("User-Agent", USER_AGENT)
                .set("x-device", &x_device_header(&device_id))
                .set("Cookie", &format!("t={new_token}"))
                .set("Content-Type", "application/json")
                .send_json(body.clone())
                .map_err(|e| format!("v2 API request failed after re-auth: {e}"))
        }
        Err(e) => Err(format!("v2 API request failed: {e}")),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMove {
    pub task_id: String,
    pub from_project_id: String,
    pub to_project_id: String,
}

/// Move tasks between projects using the v2 batch endpoint.
pub fn move_tasks(moves: &[TaskMove]) -> Result<(), String> {
    let token = get_session_token()?;
    let body = serde_json::to_value(moves)
        .map_err(|e| format!("failed to serialize move request: {e}"))?;
    v2_post("/batch/taskProject", &token, &body)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_device_header_contains_device_id() {
        let header = x_device_header("6490abcdef1234567890");
        let parsed: serde_json::Value =
            serde_json::from_str(&header).expect("x-device header should be valid JSON");
        assert_eq!(parsed["id"], "6490abcdef1234567890");
        assert_eq!(parsed["platform"], "web");
        assert_eq!(parsed["version"], 6490);
    }

    #[test]
    fn x_device_header_is_valid_json() {
        let header = x_device_header("test123");
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&header);
        assert!(parsed.is_ok(), "x-device header should be valid JSON");
    }
}
