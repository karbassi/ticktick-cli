use serde::{Deserialize, Serialize};

use super::v2;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Filter {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub rule: Option<String>,
    #[serde(default)]
    pub sort_order: Option<i64>,
    #[serde(default)]
    pub sort_type: Option<String>,
    #[serde(default)]
    pub etag: Option<String>,
}

/// Fields that can be set/updated on a filter.
#[derive(Default, Clone)]
pub struct FilterFields {
    pub name: Option<String>,
    pub rule: Option<String>,
    pub sort_type: Option<String>,
}

impl FilterFields {
    pub fn apply_to(&self, body: &mut serde_json::Value) {
        if let Some(ref n) = self.name {
            body["name"] = serde_json::Value::String(n.clone());
        }
        if let Some(ref r) = self.rule {
            body["rule"] = serde_json::Value::String(r.clone());
        }
        if let Some(ref s) = self.sort_type {
            body["sortType"] = serde_json::Value::String(s.clone());
        }
    }
}

/// List all filters (from batch_check).
pub fn list() -> Result<Vec<Filter>, String> {
    let data = v2::batch_check()?;
    let filters = data
        .get("filters")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    serde_json::from_value(serde_json::Value::Array(filters))
        .map_err(|e| format!("failed to parse filters: {e}"))
}

/// Create a filter.
pub fn create(fields: &FilterFields) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let mut filter = serde_json::json!({});
    fields.apply_to(&mut filter);
    let body = serde_json::json!({ "add": [filter] });
    let resp = v2::v2_post("/batch/filter", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse filter create response: {e}"))
}

/// Update a filter.
pub fn update(id: &str, etag: &str, fields: &FilterFields) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let mut filter = serde_json::json!({
        "id": id,
        "etag": etag,
    });
    fields.apply_to(&mut filter);
    let body = serde_json::json!({ "update": [filter] });
    let resp = v2::v2_post("/batch/filter", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse filter update response: {e}"))
}

/// Delete filters by IDs.
pub fn delete(ids: &[String]) -> Result<(), String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({ "delete": ids });
    v2::v2_post("/batch/filter", &token, &body)?;
    Ok(())
}

/// Resolve a filter name or ID to (id, etag).
pub fn resolve_id(name_or_id: &str) -> Result<(String, String), String> {
    let filters = list()?;

    // If it looks like an ID (long hex string), find by ID
    if name_or_id.len() >= 20
        && name_or_id.chars().all(|c| c.is_ascii_hexdigit())
        && let Some(f) = filters.iter().find(|f| f.id == name_or_id)
    {
        let etag = f.etag.clone().unwrap_or_default();
        return Ok((f.id.clone(), etag));
    }

    let search = name_or_id.to_lowercase();

    // Exact match (case-insensitive)
    if let Some(f) = filters.iter().find(|f| f.name.to_lowercase() == search) {
        let etag = f.etag.clone().unwrap_or_default();
        return Ok((f.id.clone(), etag));
    }

    // Contains match
    let matches: Vec<_> = filters
        .iter()
        .filter(|f| f.name.to_lowercase().contains(&search))
        .collect();

    match matches.len() {
        0 => {
            let mut msg = format!("no filter found matching '{name_or_id}'");

            let closest = filters
                .iter()
                .map(|f| {
                    (
                        f.name.as_str(),
                        strsim::levenshtein(&search, &f.name.to_lowercase()),
                    )
                })
                .min_by_key(|(_, d)| *d);

            if let Some((name, dist)) = closest
                && dist <= 3
            {
                msg.push_str(&format!("\n\n  Did you mean '{name}'?"));
            }

            msg.push_str("\n\n  hint: Run 'ticktick-cli filter list' to see available filters");
            Err(msg)
        }
        1 => {
            let etag = matches[0].etag.clone().unwrap_or_default();
            Ok((matches[0].id.clone(), etag))
        }
        _ => {
            let names: Vec<_> = matches.iter().map(|f| f.name.as_str()).collect();
            Err(format!(
                "multiple filters match '{name_or_id}': {}\n\n  hint: Use a more specific name or the full filter ID",
                names.join(", ")
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_deserialize() {
        let json = r#"{"id":"f1","name":"High Priority","rule":"{\"and\":[],\"type\":0}","sortOrder":100,"sortType":"dueDate","etag":"etag1"}"#;
        let filter: Filter = serde_json::from_str(json).unwrap();
        assert_eq!(filter.id, "f1");
        assert_eq!(filter.name, "High Priority");
        assert_eq!(filter.rule.as_deref(), Some("{\"and\":[],\"type\":0}"));
        assert_eq!(filter.sort_order, Some(100));
        assert_eq!(filter.sort_type.as_deref(), Some("dueDate"));
        assert_eq!(filter.etag.as_deref(), Some("etag1"));
    }

    #[test]
    fn filter_deserialize_minimal() {
        let json = r#"{"id":"f2","name":"Simple"}"#;
        let filter: Filter = serde_json::from_str(json).unwrap();
        assert_eq!(filter.id, "f2");
        assert_eq!(filter.name, "Simple");
        assert!(filter.rule.is_none());
        assert!(filter.sort_order.is_none());
        assert!(filter.etag.is_none());
    }

    #[test]
    fn filter_fields_apply_to() {
        let fields = FilterFields {
            name: Some("Test".to_string()),
            rule: Some("{\"and\":[],\"type\":0}".to_string()),
            sort_type: Some("priority".to_string()),
        };
        let mut body = serde_json::json!({});
        fields.apply_to(&mut body);
        assert_eq!(body["name"], "Test");
        assert_eq!(body["rule"], "{\"and\":[],\"type\":0}");
        assert_eq!(body["sortType"], "priority");
    }

    #[test]
    fn filter_fields_partial() {
        let fields = FilterFields {
            name: Some("Only Name".to_string()),
            ..Default::default()
        };
        let mut body = serde_json::json!({});
        fields.apply_to(&mut body);
        assert_eq!(body["name"], "Only Name");
        assert!(body.get("rule").is_none());
        assert!(body.get("sortType").is_none());
    }
}
