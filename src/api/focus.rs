use super::v2;

const MS_BASE_URL: &str = "https://ms.ticktick.com";
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

// ---------------------------------------------------------------------------
// Read endpoints (api.ticktick.com/api/v2)
// ---------------------------------------------------------------------------

/// Get current timer state.
pub fn get_timer_status() -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let resp = v2::v2_get("/timer", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse timer status: {e}"))
}

/// Get focus/pomodoro statistics (today/total).
pub fn get_stats() -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let resp = v2::v2_get("/pomodoros/statistics/generalForDesktop", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse focus stats: {e}"))
}

/// Get focus session log for a date range.
/// `from` and `to` are epoch milliseconds.
pub fn get_log(from: i64, to: i64) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let resp = v2::v2_get(&format!("/pomodoros?from={from}&to={to}"), &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse focus log: {e}"))
}

/// Get full focus session timeline.
pub fn get_timeline() -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let resp = v2::v2_get("/pomodoros/timeline", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse focus timeline: {e}"))
}

// ---------------------------------------------------------------------------
// Control endpoint (ms.ticktick.com)
// ---------------------------------------------------------------------------

/// Send focus operations to the control endpoint.
/// Returns the full response including current session state.
pub fn focus_op(ops: &[serde_json::Value]) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let device_id = crate::config::get_or_create_device_id();
    let x_device = v2::x_device_header(&device_id);

    // Use current time as lastPoint
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let body = serde_json::json!({
        "lastPoint": now_ms,
        "opList": ops,
    });

    if crate::output::verbose() >= 1 {
        eprintln!("> POST {MS_BASE_URL}/focus/batch/focusOp");
    }

    let resp = ureq::post(&format!("{MS_BASE_URL}/focus/batch/focusOp"))
        .set("User-Agent", USER_AGENT)
        .set("x-device", &x_device)
        .set("Cookie", &format!("t={token}"))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| format!("focus op request failed: {e}"))?;

    resp.into_json()
        .map_err(|e| format!("failed to parse focus op response: {e}"))
}

/// Generate a unique operation/session ID (hex string).
pub fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    // Use time-based hex + simple random suffix
    let seed = ms as u64 ^ 0xDEAD_BEEF_CAFE_BABE;
    format!("{ms:x}{seed:08x}")
}

/// Get the current time as an ISO string for focus ops.
pub fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        % 1000;
    // Convert epoch seconds to date/time components
    let days = secs / 86400;
    let day_secs = secs % 86400;
    let hour = day_secs / 3600;
    let minute = (day_secs % 3600) / 60;
    let second = day_secs % 60;

    // Civil days algorithm from Howard Hinnant
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!(
        "{y:04}-{m:02}-{d:02}T{hour:02}:{minute:02}:{second:02}.{ms:03}+0000",
        y = y,
        m = m,
        d = d,
        hour = hour,
        minute = minute,
        second = second,
        ms = ms,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_id_is_hex() {
        let id = generate_id();
        assert!(!id.is_empty());
        assert!(
            id.chars().all(|c| c.is_ascii_hexdigit()),
            "ID should be hex: {id}"
        );
    }

    #[test]
    fn now_iso_format() {
        let iso = now_iso();
        assert!(iso.contains("T"), "Should contain T separator: {iso}");
        assert!(iso.ends_with("+0000"), "Should end with +0000: {iso}");
    }
}
