use serde::{Deserialize, Serialize};

use super::v2;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Tag {
    pub name: String,
    #[serde(default)]
    pub raw_name: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub sort_order: Option<i64>,
    #[serde(default)]
    pub sort_type: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub etag: Option<String>,
    #[serde(default)]
    pub parent: Option<String>,
}

/// List all tags.
pub fn list() -> Result<Vec<Tag>, String> {
    let token = v2::get_session_token()?;
    let resp = v2::v2_get("/tags", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse tags: {e}"))
}

/// Create one or more tags via the batch endpoint.
pub fn create(names: &[String]) -> Result<Vec<Tag>, String> {
    let token = v2::get_session_token()?;
    let add: Vec<serde_json::Value> = names
        .iter()
        .map(|n| serde_json::json!({ "label": n, "name": n }))
        .collect();
    let body = serde_json::json!({ "add": add });
    let resp = v2::v2_post("/batch/tag", &token, &body)?;
    // The batch endpoint returns the full list of tags after the operation.
    resp.into_json()
        .map_err(|e| format!("failed to parse tag create response: {e}"))
}

/// Delete a tag by its label.
pub fn delete(name: &str) -> Result<(), String> {
    let token = v2::get_session_token()?;
    let encoded = v2::url_encode(name);
    v2::v2_delete(&format!("/tag?name={encoded}"), &token)?;
    Ok(())
}

/// Rename a tag.
pub fn rename(old: &str, new: &str) -> Result<(), String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({ "name": old, "newName": new });
    v2::v2_put("/tag/rename", &token, &body)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_deserialize() {
        let json = r##"{"name":"work","rawName":"Work","label":"Work","sortOrder":0,"sortType":"project","color":"#ff0000","etag":"abc123"}"##;
        let tag: Tag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, "work");
        assert_eq!(tag.raw_name.as_deref(), Some("Work"));
        assert_eq!(tag.label.as_deref(), Some("Work"));
        assert_eq!(tag.color.as_deref(), Some("#ff0000"));
        assert_eq!(tag.etag.as_deref(), Some("abc123"));
    }

    #[test]
    fn tag_deserialize_minimal() {
        let json = r#"{"name":"minimal"}"#;
        let tag: Tag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, "minimal");
        assert!(tag.color.is_none());
        assert!(tag.parent.is_none());
    }
}
