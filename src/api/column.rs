use serde::{Deserialize, Serialize};

use crate::config;

/// A kanban column (called a "section" in some TickTick clients).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Column {
    pub id: String,
    #[serde(default)]
    pub project_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub sort_order: Option<i64>,
}

#[derive(Default)]
pub struct ColumnFields {
    pub name: Option<String>,
    pub sort_order: Option<i64>,
    pub color: Option<String>,
}

impl ColumnFields {
    pub fn to_body(&self) -> serde_json::Value {
        let mut body = serde_json::json!({});
        if let Some(ref n) = self.name {
            body["name"] = serde_json::Value::String(n.clone());
        }
        if let Some(s) = self.sort_order {
            body["sortOrder"] = serde_json::Value::Number(s.into());
        }
        if let Some(ref c) = self.color {
            body["color"] = serde_json::Value::String(c.clone());
        }
        body
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.sort_order.is_none() && self.color.is_none()
    }
}

/// List the kanban columns of a project.
pub fn list(project_id: &str) -> Result<Vec<Column>, String> {
    let token = config::get_access_token()?;
    let resp = super::get(&format!("/project/{project_id}/column"), &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse columns: {e}"))
}

/// Create a column in a project.
pub fn create(project_id: &str, fields: &ColumnFields) -> Result<serde_json::Value, String> {
    let token = config::get_access_token()?;
    let resp = super::post(
        &format!("/project/{project_id}/column"),
        &token,
        &fields.to_body(),
    )?;
    resp.into_json()
        .map_err(|e| format!("failed to parse column create response: {e}"))
}

/// Update a column. The official API has no delete endpoint (DELETE returns 405).
pub fn update(
    project_id: &str,
    column_id: &str,
    fields: &ColumnFields,
) -> Result<serde_json::Value, String> {
    let token = config::get_access_token()?;
    let resp = super::post(
        &format!("/project/{project_id}/column/{column_id}"),
        &token,
        &fields.to_body(),
    )?;
    resp.into_json()
        .map_err(|e| format!("failed to parse column update response: {e}"))
}

/// Resolve a column name or ID within a project to its ID.
pub fn resolve_id(project_id: &str, name_or_id: &str) -> Result<String, String> {
    let columns = list(project_id)?;
    match_column(&columns, name_or_id)
}

/// Pure matching logic: exact ID, exact name, then unique substring (all case-insensitive).
pub fn match_column(columns: &[Column], name_or_id: &str) -> Result<String, String> {
    if let Some(c) = columns.iter().find(|c| c.id == name_or_id) {
        return Ok(c.id.clone());
    }

    let search = name_or_id.to_lowercase();

    if let Some(c) = columns.iter().find(|c| c.name.to_lowercase() == search) {
        return Ok(c.id.clone());
    }

    let matches: Vec<_> = columns
        .iter()
        .filter(|c| c.name.to_lowercase().contains(&search))
        .collect();

    match matches.len() {
        0 => {
            let mut msg = format!("no column found matching '{name_or_id}'");
            let closest = columns
                .iter()
                .map(|c| {
                    (
                        c.name.as_str(),
                        strsim::levenshtein(&search, &c.name.to_lowercase()),
                    )
                })
                .min_by_key(|(_, d)| *d);
            if let Some((name, dist)) = closest
                && dist <= 3
            {
                msg.push_str(&format!("\n\n  Did you mean '{name}'?"));
            }
            msg.push_str(
                "\n\n  hint: Run 'ticktick-cli project column list <project>' to see columns",
            );
            Err(msg)
        }
        1 => Ok(matches[0].id.clone()),
        _ => {
            let names: Vec<_> = matches.iter().map(|c| c.name.as_str()).collect();
            Err(format!(
                "multiple columns match '{name_or_id}': {}\n\n  hint: Use a more specific name or the full column ID",
                names.join(", ")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cols() -> Vec<Column> {
        serde_json::from_str(
            r##"[
              {"id":"c1","projectId":"p","name":"To do","color":null,"sortOrder":0},
              {"id":"c2","projectId":"p","name":"Doing","color":"#FF0000","sortOrder":1},
              {"id":"c3","projectId":"p","name":"Done","sortOrder":2}
            ]"##,
        )
        .unwrap()
    }

    #[test]
    fn deserialize_column() {
        let c = &cols()[1];
        assert_eq!(c.id, "c2");
        assert_eq!(c.color.as_deref(), Some("#FF0000"));
        assert_eq!(c.sort_order, Some(1));
        assert!(cols()[0].color.is_none());
    }

    #[test]
    fn body_only_includes_set_fields() {
        let f = ColumnFields {
            name: Some("Review".into()),
            ..Default::default()
        };
        assert_eq!(f.to_body(), serde_json::json!({"name": "Review"}));

        let f = ColumnFields {
            name: Some("Review".into()),
            sort_order: Some(5),
            color: Some("#00FF00".into()),
        };
        assert_eq!(
            f.to_body(),
            serde_json::json!({"name": "Review", "sortOrder": 5, "color": "#00FF00"})
        );
        assert!(ColumnFields::default().is_empty());
        assert!(!f.is_empty());
    }

    #[test]
    fn match_by_id_exact_and_partial() {
        let c = cols();
        assert_eq!(match_column(&c, "c3").unwrap(), "c3");
        assert_eq!(match_column(&c, "doing").unwrap(), "c2");
        assert_eq!(match_column(&c, "to").unwrap(), "c1");
    }

    #[test]
    fn match_exact_beats_substring() {
        // "Done" is a substring of nothing else, but "do" is ambiguous
        let c = cols();
        assert_eq!(match_column(&c, "Done").unwrap(), "c3");
        let err = match_column(&c, "do").unwrap_err();
        assert!(err.contains("multiple columns match"), "{err}");
    }

    #[test]
    fn match_missing_suggests() {
        let err = match_column(&cols(), "Dne").unwrap_err();
        assert!(err.contains("no column found"), "{err}");
        assert!(err.contains("Did you mean 'Done'?"), "{err}");
    }
}
