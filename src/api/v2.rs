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

/// Get the cached v2 session token.
///
/// The session token is obtained by copying the `t` cookie from
/// a logged-in TickTick web session and setting `v2_session_token`
/// in config.json.
pub fn get_session_token() -> Result<String, String> {
    config::get_v2_session_token().ok_or_else(|| {
        "v2 session token not set\n\n  \
         hint: Log in at ticktick.com, copy the 't' cookie from DevTools \
         (Network tab → any request to api.ticktick.com → Cookie header), \
         then add \"v2_session_token\": \"<token>\" to ~/.config/ticktick-cli/config.json"
            .to_string()
    })
}

// ---------------------------------------------------------------------------
// v2 HTTP helpers
// ---------------------------------------------------------------------------

/// Format a v2 API error, with a helpful hint on 401.
fn v2_error(e: &ureq::Error) -> String {
    if matches!(e, ureq::Error::Status(401, _)) {
        "v2 session expired\n\n  \
         hint: Log in at ticktick.com, copy the 't' cookie from DevTools \
         (Network tab → any request to api.ticktick.com → Cookie header), \
         then update \"v2_session_token\" in ~/.config/ticktick-cli/config.json"
            .to_string()
    } else {
        format!("v2 API request failed: {e}")
    }
}

/// Make a POST request to the v2 API with session auth.
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

    ureq::post(&url)
        .set("User-Agent", USER_AGENT)
        .set("x-device", &x_device_header(&device_id))
        .set("Cookie", &format!("t={token}"))
        .set("Content-Type", "application/json")
        .send_json(body.clone())
        .map_err(|e| v2_error(&e))
}

/// Make a GET request to the v2 API with session auth.
pub fn v2_get(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    let device_id = config::get_or_create_device_id();
    let url = format!("{V2_BASE_URL}{endpoint}");

    if crate::output::verbose() >= 1 {
        eprintln!("> GET {url}");
    }

    ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("x-device", &x_device_header(&device_id))
        .set("Cookie", &format!("t={token}"))
        .call()
        .map_err(|e| v2_error(&e))
}

/// Make a DELETE request to the v2 API with session auth.
pub fn v2_delete(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    let device_id = config::get_or_create_device_id();
    let url = format!("{V2_BASE_URL}{endpoint}");

    if crate::output::verbose() >= 1 {
        eprintln!("> DELETE {url}");
    }

    ureq::delete(&url)
        .set("User-Agent", USER_AGENT)
        .set("x-device", &x_device_header(&device_id))
        .set("Cookie", &format!("t={token}"))
        .call()
        .map_err(|e| v2_error(&e))
}

/// Make a PUT request to the v2 API with session auth.
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

    ureq::put(&url)
        .set("User-Agent", USER_AGENT)
        .set("x-device", &x_device_header(&device_id))
        .set("Cookie", &format!("t={token}"))
        .set("Content-Type", "application/json")
        .send_json(body.clone())
        .map_err(|e| v2_error(&e))
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
/// `limit` caps the number of results.
///
/// Note: the `from`/`to` date query parameters are intentionally omitted —
/// they cause HTTP 500 errors on the v2 session API.
pub fn list_completed_in_all(limit: u32) -> Result<Vec<serde_json::Value>, String> {
    let token = get_session_token()?;
    let endpoint = format!("/project/all/completedInAll/?limit={limit}");
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
// User profile / settings
// ---------------------------------------------------------------------------

/// Get the user's profile.
pub fn get_profile() -> Result<serde_json::Value, String> {
    let token = get_session_token()?;
    let resp = v2_get("/user/profile", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse profile response: {e}"))
}

/// Get the user's status (subscription, points, etc.).
pub fn get_status() -> Result<serde_json::Value, String> {
    let token = get_session_token()?;
    let resp = v2_get("/user/status", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse status response: {e}"))
}

/// Get user preference settings.
pub fn get_settings() -> Result<serde_json::Value, String> {
    let token = get_session_token()?;
    let resp = v2_get("/user/preferences/settings?includeWeb=true", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse settings response: {e}"))
}

// ---------------------------------------------------------------------------
// Trash
// ---------------------------------------------------------------------------

/// List tasks in the trash.
pub fn list_trash() -> Result<Vec<serde_json::Value>, String> {
    let token = get_session_token()?;
    let resp = v2_get("/project/all/trash/page", &token)?;
    let data: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("failed to parse trash response: {e}"))?;
    let tasks = data
        .get("tasks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(tasks)
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
