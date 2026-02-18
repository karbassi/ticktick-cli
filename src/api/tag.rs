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

/// Fields that can be updated on a tag.
#[derive(Default, Clone)]
pub struct TagFields {
    pub color: Option<String>,
    pub parent: Option<Option<String>>, // Some(Some(name)) = set, Some(None) = clear
    pub sort_order: Option<i64>,
    pub sort_type: Option<String>,
}

impl TagFields {
    pub fn apply_to(&self, body: &mut serde_json::Value) {
        if let Some(ref c) = self.color {
            body["color"] = serde_json::Value::String(c.clone());
        }
        if let Some(ref p) = self.parent {
            match p {
                Some(name) => body["parent"] = serde_json::Value::String(name.clone()),
                None => body["parent"] = serde_json::Value::String(String::new()),
            }
        }
        if let Some(o) = self.sort_order {
            body["sortOrder"] = serde_json::json!(o);
        }
        if let Some(ref s) = self.sort_type {
            body["sortType"] = serde_json::Value::String(s.clone());
        }
    }
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

/// Update a tag's properties.
pub fn update(name: &str, fields: &TagFields) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let mut tag = serde_json::json!({ "name": name });
    fields.apply_to(&mut tag);
    let body = serde_json::json!({ "update": [tag] });
    let resp = v2::v2_post("/batch/tag", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse tag update response: {e}"))
}

/// Merge a source tag into a target tag.
/// All tasks tagged with `source` are re-tagged with `target`, and `source` is deleted.
pub fn merge(source: &str, target: &str) -> Result<(), String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({ "name": source, "newName": target });
    v2::v2_put("/tag/merge", &token, &body)?;
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

    #[test]
    fn tag_fields_apply_to() {
        let fields = TagFields {
            color: Some("#FF0000".to_string()),
            parent: Some(Some("parent-tag".to_string())),
            sort_order: Some(100),
            sort_type: Some("dueDate".to_string()),
        };
        let mut body = serde_json::json!({ "name": "test" });
        fields.apply_to(&mut body);
        assert_eq!(body["color"], "#FF0000");
        assert_eq!(body["parent"], "parent-tag");
        assert_eq!(body["sortOrder"], 100);
        assert_eq!(body["sortType"], "dueDate");
    }

    #[test]
    fn tag_fields_clear_parent() {
        let fields = TagFields {
            parent: Some(None), // clear parent
            ..Default::default()
        };
        let mut body = serde_json::json!({ "name": "test" });
        fields.apply_to(&mut body);
        assert_eq!(body["parent"], "");
        assert!(body.get("color").is_none());
    }
}
