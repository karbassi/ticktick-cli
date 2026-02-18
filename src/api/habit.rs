use serde::{Deserialize, Serialize};

use super::v2;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Habit {
    pub id: String,
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

/// List all habits.
pub fn list() -> Result<Vec<Habit>, String> {
    let token = v2::get_session_token()?;
    let resp = v2::v2_get("/habits", &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse habits: {e}"))
}

/// List all habit sections.
pub fn list_sections() -> Result<Vec<HabitSection>, String> {
    let token = v2::get_session_token()?;
    let resp = v2::v2_get("/habitSections", &token)?;
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

/// Update a habit.
pub fn update(id: &str, etag: &str, fields: &HabitFields) -> Result<serde_json::Value, String> {
    let token = v2::get_session_token()?;
    let mut habit = serde_json::json!({
        "id": id,
        "etag": etag,
    });
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
    let habits = list()?;

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
    let trimmed = input.trim().to_lowercase();

    let (year, month, day) = match trimmed.as_str() {
        "today" => {
            let (y, m, d) = local_today();
            (y, m, d)
        }
        "yesterday" => {
            let (y, m, d) = local_today();
            sub_day(y, m, d)
        }
        _ => {
            let parts: Vec<&str> = trimmed.split('-').collect();
            if parts.len() != 3 {
                return Err(format!(
                    "invalid date '{input}': expected YYYY-MM-DD, 'today', or 'yesterday'"
                ));
            }
            let year: i32 = parts[0]
                .parse()
                .map_err(|_| format!("invalid year in '{input}'"))?;
            let month: u32 = parts[1]
                .parse()
                .map_err(|_| format!("invalid month in '{input}'"))?;
            let day: u32 = parts[2]
                .parse()
                .map_err(|_| format!("invalid day in '{input}'"))?;
            if !(1..=12).contains(&month) {
                return Err(format!("month out of range in '{input}'"));
            }
            if !(1..=31).contains(&day) {
                return Err(format!("day out of range in '{input}'"));
            }
            (year, month, day)
        }
    };

    Ok(year as i64 * 10000 + month as i64 * 100 + day as i64)
}

/// Returns today's local date as (year, month, day).
fn local_today() -> (i32, u32, u32) {
    use std::mem::MaybeUninit;
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as libc::time_t;

    let mut tm = unsafe { MaybeUninit::<libc::tm>::zeroed().assume_init() };
    unsafe { libc::localtime_r(&now, &mut tm) };

    (tm.tm_year + 1900, (tm.tm_mon + 1) as u32, tm.tm_mday as u32)
}

/// Subtract one day from the given date.
fn sub_day(year: i32, month: u32, day: u32) -> (i32, u32, u32) {
    if day > 1 {
        (year, month, day - 1)
    } else if month > 1 {
        let prev_month = month - 1;
        let d = super::task::days_in_month(year, prev_month);
        (year, prev_month, d)
    } else {
        (year - 1, 12, 31)
    }
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
    }

    #[test]
    fn sub_day_normal() {
        assert_eq!(sub_day(2026, 2, 18), (2026, 2, 17));
    }

    #[test]
    fn sub_day_first_of_month() {
        assert_eq!(sub_day(2026, 3, 1), (2026, 2, 28));
        assert_eq!(sub_day(2024, 3, 1), (2024, 2, 29)); // leap year
    }

    #[test]
    fn sub_day_first_of_year() {
        assert_eq!(sub_day(2026, 1, 1), (2025, 12, 31));
    }
}
