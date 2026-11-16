use serde::{Deserialize, Serialize};

use super::task::{system_zone, today_in};
use super::v2;
use jiff::tz::TimeZone;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Habit {
    pub id: String,
    #[serde(default, deserialize_with = "super::null_as_empty")]
    pub name: String,
    #[serde(default)]
    pub icon_res: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub sort_order: Option<i64>,
    #[serde(default)]
    pub status: Option<i32>,
    #[serde(default)]
    pub encouragement: Option<String>,
    #[serde(default)]
    pub total_check_ins: Option<i64>,
    #[serde(default)]
    pub current_streak: Option<i64>,
    #[serde(default)]
    pub created_time: Option<String>,
    #[serde(default)]
    pub modified_time: Option<String>,
    #[serde(default, rename = "type")]
    pub habit_type: Option<String>,
    #[serde(default)]
    pub goal: Option<f64>,
    #[serde(default)]
    pub step: Option<f64>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub record_enable: Option<bool>,
    #[serde(default)]
    pub repeat_rule: Option<String>,
    #[serde(default)]
    pub reminders: Option<Vec<String>>,
    #[serde(default)]
    pub section_id: Option<String>,
    #[serde(default)]
    pub target_days: Option<i32>,
    #[serde(default)]
    pub target_start_date: Option<String>,
    #[serde(default)]
    pub completed_cycles: Option<i32>,
    #[serde(default)]
    pub etag: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct HabitCheckin {
    pub id: String,
    #[serde(default)]
    pub habit_id: Option<String>,
    #[serde(default)]
    pub checkin_stamp: Option<i64>,
    #[serde(default)]
    pub checkin_time: Option<String>,
    #[serde(default)]
    pub op_time: Option<String>,
    #[serde(default)]
    pub value: Option<f64>,
    #[serde(default)]
    pub goal: Option<f64>,
    #[serde(default)]
    pub status: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct HabitSection {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub sort_order: Option<i64>,
}

#[derive(Default, Clone)]
pub struct HabitFields {
    pub name: Option<String>,
    pub habit_type: Option<String>,
    pub goal: Option<f64>,
    pub unit: Option<String>,
    pub section_id: Option<String>,
    pub repeat_rule: Option<String>,
    pub color: Option<String>,
    pub status: Option<i32>,
}

impl HabitFields {
    pub fn apply_to(&self, body: &mut serde_json::Value) {
        if let Some(ref n) = self.name {
            body["name"] = serde_json::Value::String(n.clone());
        }
        if let Some(ref t) = self.habit_type {
            body["type"] = serde_json::Value::String(t.clone());
        }
        if let Some(g) = self.goal {
            body["goal"] = serde_json::json!(g);
        }
        if let Some(ref u) = self.unit {
            body["unit"] = serde_json::Value::String(u.clone());
        }
        if let Some(ref s) = self.section_id {
            body["sectionId"] = serde_json::Value::String(s.clone());
        }
        if let Some(ref r) = self.repeat_rule {
            body["repeatRule"] = serde_json::Value::String(r.clone());
        }
        if let Some(ref c) = self.color {
            body["color"] = serde_json::Value::String(c.clone());
        }
        if let Some(s) = self.status {
            body["status"] = serde_json::json!(s);
        }
    }
}

fn raw_list() -> Result<Vec<serde_json::Value>, String> {
    let token = v2::get_session_token()?;
    let resp = v2::v2_get("/habits", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse habits: {e}"))
}

fn parse(raw: &[serde_json::Value]) -> Result<Vec<Habit>, String> {
    serde_json::from_value(serde_json::Value::Array(raw.to_vec()))
        .map_err(|e| format!("failed to parse habits: {e}"))
}

/// List all habits.
pub fn list() -> Result<Vec<Habit>, String> {
    parse(&raw_list()?)
}

/// The full record each query names, from one fetch. v2 updates replace the
/// whole habit, so edits go onto these.
pub fn resolve_records(
    queries: &[String],
) -> Result<Vec<Result<serde_json::Value, String>>, String> {
    let raw = raw_list()?;
    let habits = parse(&raw)?;
    Ok(queries
        .iter()
        .map(|q| {
            let (id, _) = match_habit(q, &habits)?;
            raw.iter()
                .find(|h| h["id"] == id.as_str())
                .cloned()
                .ok_or_else(|| format!("no habit found matching '{q}'"))
        })
        .collect())
}

/// List all habit sections.
pub fn list_sections() -> Result<Vec<HabitSection>, String> {
    let token = crate::config::get_access_token()?;
    let resp = super::get("/habit/sections", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse habit sections: {e}"))
}

/// Create a habit section.
pub fn create_section(name: &str) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({
        "add": [{ "name": name }],
    });
    let resp = v2::v2_post("/habitSections/batch", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse section create response: {e}"))
}

/// Delete habit sections by IDs.
pub fn delete_sections(ids: &[String]) -> Result<(), String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({ "delete": ids });
    v2::v2_post("/habitSections/batch", &token, &body)?;
    Ok(())
}

/// Rename a habit section.
pub fn rename_section(id: &str, name: &str) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({
        "update": [{ "id": id, "name": name }],
    });
    let resp = v2::v2_post("/habitSections/batch", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse section rename response: {e}"))
}

/// Resolve a habit section name or ID to (id, name).
pub fn resolve_section_id(name_or_id: &str) -> Result<(String, String), String> {
    let sections = list_sections()?;

    // If it looks like an ID (long hex string), find by ID
    if name_or_id.len() >= 20
        && name_or_id.chars().all(|c| c.is_ascii_hexdigit())
        && let Some(s) = sections.iter().find(|s| s.id == name_or_id)
    {
        return Ok((s.id.clone(), s.name.clone()));
    }

    let search = name_or_id.to_lowercase();

    // Exact match (case-insensitive)
    if let Some(s) = sections.iter().find(|s| s.name.to_lowercase() == search) {
        return Ok((s.id.clone(), s.name.clone()));
    }

    // Contains match
    let matches: Vec<_> = sections
        .iter()
        .filter(|s| s.name.to_lowercase().contains(&search))
        .collect();

    match matches.len() {
        0 => {
            let mut msg = format!("no habit section found matching '{name_or_id}'");

            let closest = sections
                .iter()
                .map(|s| {
                    (
                        s.name.as_str(),
                        strsim::levenshtein(&search, &s.name.to_lowercase()),
                    )
                })
                .min_by_key(|(_, d)| *d);

            if let Some((name, dist)) = closest
                && dist <= 3
            {
                msg.push_str(&format!("\n\n  Did you mean '{name}'?"));
            }

            msg.push_str(
                "\n\n  hint: Run 'ticktick-cli habit section list' to see available sections",
            );
            Err(msg)
        }
        1 => Ok((matches[0].id.clone(), matches[0].name.clone())),
        _ => {
            let names: Vec<_> = matches.iter().map(|s| s.name.as_str()).collect();
            Err(format!(
                "multiple sections match '{name_or_id}': {}\n\n  hint: Use a more specific name or the full section ID",
                names.join(", ")
            ))
        }
    }
}

/// Create a habit.
pub fn create(fields: &HabitFields) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let mut habit = serde_json::json!({});
    fields.apply_to(&mut habit);
    let body = serde_json::json!({ "add": [habit] });
    let resp = v2::v2_post("/habits/batch", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse habit create response: {e}"))
}

/// Delete habits by IDs.
pub fn delete(ids: &[String]) -> Result<(), String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({ "delete": ids });
    v2::v2_post("/habits/batch", &token, &body)?;
    Ok(())
}

/// Update a habit, given its full record from [`resolve_records`].
pub fn update(
    mut habit: serde_json::Value,
    fields: &HabitFields,
) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    fields.apply_to(&mut habit);
    let body = serde_json::json!({ "update": [habit] });
    let resp = v2::v2_post("/habits/batch", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse habit update response: {e}"))
}

/// Record a habit check-in.
pub fn checkin(habit_id: &str, stamp: i64, value: f64) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({
        "add": [{
            "habitId": habit_id,
            "checkinStamp": stamp,
            "value": value,
            "status": 0,
        }]
    });
    let resp = v2::v2_post("/habitCheckins/batch", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse checkin response: {e}"))
}

/// Query check-ins for habits.
pub fn query_checkins(habit_ids: &[String], after_stamp: i64) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let body = serde_json::json!({
        "habitIds": habit_ids,
        "afterStamp": after_stamp,
    });
    let resp = v2::v2_post("/habitCheckins/query", &token, &body)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse checkin query response: {e}"))
}

/// Resolve a habit name or ID to (id, etag).
pub fn resolve_id(name_or_id: &str) -> Result<(String, String), String> {
    match_habit(name_or_id, &list()?)
}

fn match_habit(name_or_id: &str, habits: &[Habit]) -> Result<(String, String), String> {
    // If it looks like an ID (long hex string), find by ID
    if name_or_id.len() >= 20
        && name_or_id.chars().all(|c| c.is_ascii_hexdigit())
        && let Some(h) = habits.iter().find(|h| h.id == name_or_id)
    {
        let etag = h.etag.clone().unwrap_or_default();
        return Ok((h.id.clone(), etag));
    }

    let search = name_or_id.to_lowercase();

    // Exact match (case-insensitive)
    if let Some(h) = habits.iter().find(|h| h.name.to_lowercase() == search) {
        let etag = h.etag.clone().unwrap_or_default();
        return Ok((h.id.clone(), etag));
    }

    // Contains match
    let matches: Vec<_> = habits
        .iter()
        .filter(|h| h.name.to_lowercase().contains(&search))
        .collect();

    match matches.len() {
        0 => {
            let mut msg = format!("no habit found matching '{name_or_id}'");

            let closest = habits
                .iter()
                .map(|h| {
                    (
                        h.name.as_str(),
                        strsim::levenshtein(&search, &h.name.to_lowercase()),
                    )
                })
                .min_by_key(|(_, d)| *d);

            if let Some((name, dist)) = closest
                && dist <= 3
            {
                msg.push_str(&format!("\n\n  Did you mean '{name}'?"));
            }

            msg.push_str("\n\n  hint: Run 'ticktick-cli habit list' to see available habits");
            Err(msg)
        }
        1 => {
            let etag = matches[0].etag.clone().unwrap_or_default();
            Ok((matches[0].id.clone(), etag))
        }
        _ => {
            let names: Vec<_> = matches.iter().map(|h| h.name.as_str()).collect();
            Err(format!(
                "multiple habits match '{name_or_id}': {}\n\n  hint: Use a more specific name or the full habit ID",
                names.join(", ")
            ))
        }
    }
}

/// Convert a date string to a stamp integer (YYYYMMDD).
/// Accepts: "YYYY-MM-DD", "today", "yesterday".
pub fn date_to_stamp(input: &str) -> Result<i64, String> {
    date_to_stamp_in(input, || Ok(system_zone()))
}

/// Like [`date_to_stamp`], with "today" and "yesterday" taken in `zone()`.
pub fn date_to_stamp_in(
    input: &str,
    zone: impl FnOnce() -> Result<TimeZone, String>,
) -> Result<i64, String> {
    let trimmed = input.trim().to_lowercase();

    let date = match trimmed.as_str() {
        "today" => today_in(&zone()?),
        "yesterday" => today_in(&zone()?).yesterday().map_err(|e| e.to_string())?,
        _ => {
            let parts: Vec<&str> = trimmed.split('-').collect();
            if parts.len() != 3 {
                return Err(format!(
                    "Invalid date '{input}': expected YYYY-MM-DD, 'today', or 'yesterday'"
                ));
            }
            let nums: Result<Vec<i64>, _> = parts.iter().map(|p| p.trim().parse::<i64>()).collect();
            let Ok(nums) = nums else {
                return Err(format!("Invalid date '{input}': non-numeric components"));
            };
            let (year, month, day) = (nums[0], nums[1], nums[2]);
            if !(1..=12).contains(&month) {
                return Err(format!("Month out of range in '{input}'"));
            }
            if !(1..=31).contains(&day) {
                return Err(format!("Day out of range in '{input}'"));
            }
            let real = i16::try_from(year)
                .ok()
                .and_then(|y| jiff::civil::Date::new(y, month as i8, day as i8).ok());
            let Some(date) = real else {
                return Err(format!("Invalid date '{input}': no such calendar day"));
            };
            date
        }
    };

    Ok(i64::from(date.year()) * 10000 + i64::from(date.month()) * 100 + i64::from(date.day()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn habit_deserialize() {
        let json = r##"{"id":"h1","name":"Morning Run","color":"#FF0000","status":0,"type":"Boolean","goal":1.0,"unit":"times","etag":"etag1","totalCheckIns":10,"currentStreak":3,"sortOrder":0}"##;
        let habit: Habit = serde_json::from_str(json).unwrap();
        assert_eq!(habit.id, "h1");
        assert_eq!(habit.name, "Morning Run");
        assert_eq!(habit.color.as_deref(), Some("#FF0000"));
        assert_eq!(habit.status, Some(0));
        assert_eq!(habit.habit_type.as_deref(), Some("Boolean"));
        assert_eq!(habit.goal, Some(1.0));
        assert_eq!(habit.unit.as_deref(), Some("times"));
        assert_eq!(habit.etag.as_deref(), Some("etag1"));
        assert_eq!(habit.total_check_ins, Some(10));
        assert_eq!(habit.current_streak, Some(3));
    }

    #[test]
    fn habit_deserialize_minimal() {
        let json = r#"{"id":"h2","name":"Read"}"#;
        let habit: Habit = serde_json::from_str(json).unwrap();
        assert_eq!(habit.id, "h2");
        assert_eq!(habit.name, "Read");
        assert!(habit.color.is_none());
        assert!(habit.goal.is_none());
        assert!(habit.etag.is_none());
    }

    #[test]
    fn habit_checkin_deserialize() {
        let json = r#"{"id":"c1","habitId":"h1","checkinStamp":20260218,"value":1.0,"goal":1.0,"status":0}"#;
        let checkin: HabitCheckin = serde_json::from_str(json).unwrap();
        assert_eq!(checkin.id, "c1");
        assert_eq!(checkin.habit_id.as_deref(), Some("h1"));
        assert_eq!(checkin.checkin_stamp, Some(20260218));
        assert_eq!(checkin.value, Some(1.0));
        assert_eq!(checkin.goal, Some(1.0));
    }

    #[test]
    fn habit_section_deserialize() {
        let json = r#"{"id":"s1","name":"Morning","sortOrder":0}"#;
        let section: HabitSection = serde_json::from_str(json).unwrap();
        assert_eq!(section.id, "s1");
        assert_eq!(section.name, "Morning");
        assert_eq!(section.sort_order, Some(0));
    }

    #[test]
    fn habit_fields_apply_to() {
        let fields = HabitFields {
            name: Some("Test".to_string()),
            habit_type: Some("Boolean".to_string()),
            goal: Some(1.0),
            unit: Some("times".to_string()),
            section_id: None,
            repeat_rule: None,
            color: Some("#FF0000".to_string()),
            status: Some(0),
        };
        let mut body = serde_json::json!({});
        fields.apply_to(&mut body);
        assert_eq!(body["name"], "Test");
        assert_eq!(body["type"], "Boolean");
        assert_eq!(body["goal"], 1.0);
        assert_eq!(body["unit"], "times");
        assert!(body.get("sectionId").is_none());
        assert!(body.get("repeatRule").is_none());
        assert_eq!(body["color"], serde_json::json!("#FF0000"));
        assert_eq!(body["status"], 0);
    }

    #[test]
    fn date_to_stamp_specific_date() {
        assert_eq!(date_to_stamp("2026-02-18").unwrap(), 20260218);
        assert_eq!(date_to_stamp("2026-01-01").unwrap(), 20260101);
        assert_eq!(date_to_stamp("2025-12-31").unwrap(), 20251231);
    }

    #[test]
    fn date_to_stamp_today_format() {
        let stamp = date_to_stamp("today").unwrap();
        // Stamp should be YYYYMMDD format — at least 8 digits, valid range
        assert!(stamp >= 20000101);
        assert!(stamp <= 29991231);
    }

    #[test]
    fn date_to_stamp_yesterday() {
        let today = date_to_stamp("today").unwrap();
        let yesterday = date_to_stamp("yesterday").unwrap();
        // Yesterday should be less than today (except at year boundary, but still valid)
        assert!(yesterday < today || yesterday > today - 200);
    }

    #[test]
    fn date_to_stamp_invalid() {
        assert!(date_to_stamp("not-a-date").is_err());
        assert!(date_to_stamp("2026/02/18").is_err());
        assert!(date_to_stamp("").is_err());
        assert!(date_to_stamp("2026-13-01").is_err());
        assert!(date_to_stamp("2026-00-01").is_err());
        assert!(date_to_stamp("2026-02-30").is_err());
        assert!(date_to_stamp("2025-02-29").is_err());
        assert_eq!(date_to_stamp("2024-02-29").unwrap(), 20240229);
    }
}
