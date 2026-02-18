use crate::config;
use serde::{Deserialize, Serialize};

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

// ---------------------------------------------------------------------------
// v2 HTTP helpers
// ---------------------------------------------------------------------------

/// Make a POST request to the v2 API with session auth.
/// Retries once on 401 by refreshing the session.
pub fn v2_post(
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

/// Make a GET request to the v2 API with session auth.
/// Retries once on 401 by refreshing the session.
pub fn v2_get(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    let device_id = config::get_or_create_device_id();
    let url = format!("{V2_BASE_URL}{endpoint}");

    if crate::output::verbose() >= 1 {
        eprintln!("> GET {url}");
    }

    let result = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("x-device", &x_device_header(&device_id))
        .set("Cookie", &format!("t={token}"))
        .call();

    match result {
        Ok(resp) => Ok(resp),
        Err(ureq::Error::Status(401, _)) => {
            let new_token = refresh_session()?;
            ureq::get(&url)
                .set("User-Agent", USER_AGENT)
                .set("x-device", &x_device_header(&device_id))
                .set("Cookie", &format!("t={new_token}"))
                .call()
                .map_err(|e| format!("v2 API request failed after re-auth: {e}"))
        }
        Err(e) => Err(format!("v2 API request failed: {e}")),
    }
}

/// Make a DELETE request to the v2 API with session auth.
/// Retries once on 401 by refreshing the session.
pub fn v2_delete(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    let device_id = config::get_or_create_device_id();
    let url = format!("{V2_BASE_URL}{endpoint}");

    if crate::output::verbose() >= 1 {
        eprintln!("> DELETE {url}");
    }

    let result = ureq::delete(&url)
        .set("User-Agent", USER_AGENT)
        .set("x-device", &x_device_header(&device_id))
        .set("Cookie", &format!("t={token}"))
        .call();

    match result {
        Ok(resp) => Ok(resp),
        Err(ureq::Error::Status(401, _)) => {
            let new_token = refresh_session()?;
            ureq::delete(&url)
                .set("User-Agent", USER_AGENT)
                .set("x-device", &x_device_header(&device_id))
                .set("Cookie", &format!("t={new_token}"))
                .call()
                .map_err(|e| format!("v2 API request failed after re-auth: {e}"))
        }
        Err(e) => Err(format!("v2 API request failed: {e}")),
    }
}

/// Make a PUT request to the v2 API with session auth.
/// Retries once on 401 by refreshing the session.
pub fn v2_put(
    endpoint: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<ureq::Response, String> {
    let device_id = config::get_or_create_device_id();
    let url = format!("{V2_BASE_URL}{endpoint}");

    if crate::output::verbose() >= 1 {
        eprintln!("> PUT {url}");
    }

    let result = ureq::put(&url)
        .set("User-Agent", USER_AGENT)
        .set("x-device", &x_device_header(&device_id))
        .set("Cookie", &format!("t={token}"))
        .set("Content-Type", "application/json")
        .send_json(body.clone());

    match result {
        Ok(resp) => Ok(resp),
        Err(ureq::Error::Status(401, _)) => {
            let new_token = refresh_session()?;
            ureq::put(&url)
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

/// Minimal percent-encoding for URL query parameters.
/// Encodes spaces as `%20` and other unsafe characters.
pub fn url_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char)
            }
            _ => {
                encoded.push('%');
                encoded.push(
                    char::from_digit((b >> 4) as u32, 16)
                        .unwrap()
                        .to_ascii_uppercase(),
                );
                encoded.push(
                    char::from_digit((b & 0xf) as u32, 16)
                        .unwrap()
                        .to_ascii_uppercase(),
                );
            }
        }
    }
    encoded
}

// ---------------------------------------------------------------------------
// Batch check (full account state sync)
// ---------------------------------------------------------------------------

/// Fetch the full account state from the v2 batch/check endpoint.
pub fn batch_check() -> Result<serde_json::Value, String> {
    let token = get_session_token()?;
    let resp = v2_get("/batch/check/0", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse batch check response: {e}"))
}

// ---------------------------------------------------------------------------
// Task move
// ---------------------------------------------------------------------------

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
// Completed tasks
// ---------------------------------------------------------------------------

/// List completed tasks across all projects.
///
/// `from` and `to` are ISO date strings (e.g. "2026-01-18+00:00:00").
/// `limit` caps the number of results.
pub fn list_completed_in_all(
    from: &str,
    to: &str,
    limit: u32,
) -> Result<Vec<serde_json::Value>, String> {
    let token = get_session_token()?;
    let from_enc = url_encode(from);
    let to_enc = url_encode(to);
    let endpoint =
        format!("/project/all/completedInAll/?from={from_enc}&to={to_enc}&limit={limit}");
    let resp = v2_get(&endpoint, &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse completed tasks: {e}"))
}

/// List completed tasks for a specific project.
pub fn list_completed_by_project(project_id: &str) -> Result<Vec<serde_json::Value>, String> {
    let token = get_session_token()?;
    let endpoint = format!("/project/{project_id}/completed");
    let resp = v2_get(&endpoint, &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse completed tasks: {e}"))
}

// ---------------------------------------------------------------------------
// Subtask parent batch
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskParent {
    pub parent_id: String,
    pub project_id: String,
    pub task_id: String,
}

/// Set parent-child relationships for tasks using the v2 batch endpoint.
pub fn set_task_parents(parents: &[TaskParent]) -> Result<(), String> {
    let token = get_session_token()?;
    let body = serde_json::to_value(parents)
        .map_err(|e| format!("failed to serialize parent request: {e}"))?;
    v2_post("/batch/taskParent", &token, &body)?;
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

    #[test]
    fn url_encode_spaces() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("no-change"), "no-change");
        assert_eq!(url_encode("a b c"), "a%20b%20c");
    }

    #[test]
    fn url_encode_special_chars() {
        assert_eq!(url_encode("a+b"), "a%2Bb");
        assert_eq!(url_encode("foo@bar"), "foo%40bar");
        assert_eq!(
            url_encode("2026-01-18+00:00:00"),
            "2026-01-18%2B00%3A00%3A00"
        );
    }

    #[test]
    fn url_encode_preserves_unreserved() {
        assert_eq!(url_encode("abc-123_def.ghi~"), "abc-123_def.ghi~");
    }

    #[test]
    fn task_parent_serialization() {
        let parent = TaskParent {
            parent_id: "parent123".to_string(),
            project_id: "proj456".to_string(),
            task_id: "task789".to_string(),
        };
        let json = serde_json::to_value(&parent).unwrap();
        assert_eq!(json["parentId"], "parent123");
        assert_eq!(json["projectId"], "proj456");
        assert_eq!(json["taskId"], "task789");
    }
}
