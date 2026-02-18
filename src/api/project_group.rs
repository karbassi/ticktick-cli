use serde::{Deserialize, Serialize};

use super::v2;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ProjectGroup {
    pub id: String,
    #[serde(default)]
    pub etag: Option<String>,
    pub name: String,
    #[serde(default)]
    pub show_all: Option<bool>,
    #[serde(default)]
    pub sort_order: Option<i64>,
    #[serde(default)]
    pub sort_type: Option<String>,
    #[serde(default)]
    pub view_mode: Option<String>,
}

/// List all project groups by extracting from batch/check response.
pub fn list() -> Result<Vec<ProjectGroup>, String> {
    let data = v2::batch_check()?;
    let groups = data
        .get("projectGroups")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));
    serde_json::from_value(groups).map_err(|e| format!("failed to parse project groups: {e}"))
}

/// Create a project group (folder).
pub fn create(name: &str) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({
        "add": [{
            "name": name,
            "listType": "group",
        }]
    });
    let resp = v2::v2_post("/batch/projectGroup", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse project group create response: {e}"))
}

/// Delete project groups by IDs.
pub fn delete(ids: &[String]) -> Result<(), String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({ "delete": ids });
    v2::v2_post("/batch/projectGroup", &token, &body)?;
    Ok(())
}

/// Rename a project group.
pub fn rename(id: &str, etag: &str, new_name: &str) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({
        "update": [{
            "id": id,
            "etag": etag,
            "name": new_name,
            "listType": "group",
        }]
    });
    let resp = v2::v2_post("/batch/projectGroup", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse project group rename response: {e}"))
}

/// Resolve a folder name or ID to (id, etag).
/// Uses fuzzy matching (case-insensitive, partial match).
pub fn resolve_id(name_or_id: &str) -> Result<(String, String), String> {
    let groups = list()?;

    // If it looks like an ID (long hex string), find by ID
    if name_or_id.len() >= 20
        && name_or_id.chars().all(|c| c.is_ascii_hexdigit())
        && let Some(g) = groups.iter().find(|g| g.id == name_or_id)
    {
        let etag = g.etag.clone().unwrap_or_default();
        return Ok((g.id.clone(), etag));
    }

    let search = name_or_id.to_lowercase();

    // Exact match (case-insensitive)
    if let Some(g) = groups.iter().find(|g| g.name.to_lowercase() == search) {
        let etag = g.etag.clone().unwrap_or_default();
        return Ok((g.id.clone(), etag));
    }

    // Contains match
    let matches: Vec<_> = groups
        .iter()
        .filter(|g| g.name.to_lowercase().contains(&search))
        .collect();

    match matches.len() {
        0 => {
            let mut msg = format!("no folder found matching '{name_or_id}'");

            let closest = groups
                .iter()
                .map(|g| {
                    (
                        g.name.as_str(),
                        strsim::levenshtein(&search, &g.name.to_lowercase()),
                    )
                })
                .min_by_key(|(_, d)| *d);

            if let Some((name, dist)) = closest
                && dist <= 3
            {
                msg.push_str(&format!("\n\n  Did you mean '{name}'?"));
            }

            msg.push_str("\n\n  hint: Run 'ticktick-cli folder list' to see available folders");
            Err(msg)
        }
        1 => {
            let etag = matches[0].etag.clone().unwrap_or_default();
            Ok((matches[0].id.clone(), etag))
        }
        _ => {
            let names: Vec<_> = matches.iter().map(|g| g.name.as_str()).collect();
            Err(format!(
                "multiple folders match '{name_or_id}': {}\n\n  hint: Use a more specific name or the full folder ID",
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
    fn project_group_deserialize() {
        let json = r#"{"id":"abc123","etag":"etag1","name":"Work","showAll":true,"sortOrder":0,"sortType":"project","viewMode":"list"}"#;
        let group: ProjectGroup = serde_json::from_str(json).unwrap();
        assert_eq!(group.id, "abc123");
        assert_eq!(group.etag.as_deref(), Some("etag1"));
        assert_eq!(group.name, "Work");
        assert_eq!(group.show_all, Some(true));
        assert_eq!(group.sort_order, Some(0));
        assert_eq!(group.view_mode.as_deref(), Some("list"));
    }

    #[test]
    fn project_group_deserialize_minimal() {
        let json = r#"{"id":"abc123","name":"Minimal"}"#;
        let group: ProjectGroup = serde_json::from_str(json).unwrap();
        assert_eq!(group.id, "abc123");
        assert_eq!(group.name, "Minimal");
        assert!(group.etag.is_none());
        assert!(group.show_all.is_none());
        assert!(group.sort_order.is_none());
    }
}
