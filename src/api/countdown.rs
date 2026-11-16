use serde::{Deserialize, Serialize};

use crate::config;

/// A countdown (birthday, anniversary, event). The official API is read-only
/// for countdowns: only `GET /open/v1/countdown` exists. Fields not listed
/// here are kept in `extra` so output is lossless.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Countdown {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// Target date as a YYYYMMDD integer.
    #[serde(default)]
    pub date: Option<i64>,
    #[serde(default)]
    pub repeat_flag: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// List all countdowns.
pub fn list() -> Result<Vec<Countdown>, String> {
    let token = config::get_access_token()?;
    let resp = super::get("/countdown", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse countdowns: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_deserialize_keeps_extra_fields() {
        let json = r#"[{"id":"c1","name":"My Birthday","date":19860506,"repeatFlag":"RRULE:FREQ=YEARLY;INTERVAL=1;BYMONTH=5;BYMONTHDAY=6","type":2,"color":"7BC4FA","reminders":["TRIGGER:P0DT9H0M0S"]}]"#;
        let list: Vec<Countdown> = serde_json::from_str(json).unwrap();
        assert_eq!(list.len(), 1);
        let c = &list[0];
        assert_eq!(c.id, "c1");
        assert_eq!(c.name.as_deref(), Some("My Birthday"));
        assert_eq!(c.date, Some(19860506));
        assert!(c.repeat_flag.as_deref().unwrap().starts_with("RRULE:"));
        assert_eq!(c.extra["color"], "7BC4FA");

        let out = serde_json::to_value(c).unwrap();
        assert_eq!(out["repeatFlag"], c.repeat_flag.as_deref().unwrap());
        assert_eq!(out["reminders"][0], "TRIGGER:P0DT9H0M0S");
    }
}
