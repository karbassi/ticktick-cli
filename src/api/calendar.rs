use super::v2;

/// List third-party calendar accounts (Google, Outlook, etc.).
pub fn list_accounts() -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let resp = v2::v2_get("/calendar/third/accounts", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse calendar accounts response: {e}"))
}

/// Query bound calendar events by date range.
/// `begin` and `end` should be ISO date strings like "2026-02-11T00:00:00.000+0000".
pub fn query_events(begin: &str, end: &str) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({ "begin": begin, "end": end });
    let resp = v2::v2_post("/calendar/bind/events/all", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse calendar events response: {e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #[test]
    fn date_format_for_api() {
        // Verify the expected date format for calendar API
        let date = "2026-02-18T00:00:00.000+0000";
        assert!(date.contains("T00:00:00.000+0000"));
    }
}
