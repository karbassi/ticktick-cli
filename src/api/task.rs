use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

unsafe extern "C" {
    fn tzset();
}

/// Serializes access to the TZ environment variable.
/// `utc_offset_for_tz` temporarily mutates TZ, so concurrent callers
/// (e.g. parallel tests) must be serialized.
static TZ_LOCK: Mutex<()> = Mutex::new(());

#[derive(Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub title: String,
    pub status: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_all_day: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub completed_time: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_all_day: Option<bool>,
    #[serde(default)]
    pub time_zone: Option<String>,
}

pub fn get_by_id(token: &str, project_id: &str, task_id: &str) -> Result<Task, String> {
    let resp = super::get(&format!("/project/{project_id}/task/{task_id}"), token)?;

    resp.into_json()
        .map_err(|e| format!("failed to parse task: {e}"))
}

pub fn list_by_project(token: &str, project_id: Option<&str>) -> Result<Vec<Task>, String> {
    if let Some(pid) = project_id {
        let resp = super::get(&format!("/project/{pid}/data"), token)?;
        let data: ProjectData = resp
            .into_json()
            .map_err(|e| format!("failed to parse project data: {e}"))?;
        Ok(data.tasks)
    } else {
        let projects = super::project::get_all()?;
        let mut all_tasks = Vec::new();
        for project in &projects {
            if let Ok(resp) = super::get(&format!("/project/{}/data", project.id), token)
                && let Ok(data) = resp.into_json::<ProjectData>()
            {
                all_tasks.extend(data.tasks);
            }
        }
        Ok(all_tasks)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectData {
    #[serde(default)]
    tasks: Vec<Task>,
}

// ---------------------------------------------------------------------------
// Date/time helpers
// ---------------------------------------------------------------------------

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 => 31,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 0,
    }
}

/// Adds one day to the given date, handling month and year overflow.
pub fn add_days(year: i32, month: u32, day: u32) -> (i32, u32, u32) {
    let dim = days_in_month(year, month);
    if day < dim {
        (year, month, day + 1)
    } else if month < 12 {
        (year, month + 1, 1)
    } else {
        (year + 1, 1, 1)
    }
}

// ---------------------------------------------------------------------------
// Local timezone helpers (via libc)
// ---------------------------------------------------------------------------

/// Raw mktime + tm_gmtoff computation. Caller must hold TZ_LOCK if the
/// TZ env var might be in a modified state.
fn compute_utc_offset(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> i64 {
    use std::mem::MaybeUninit;

    let mut tm = unsafe { MaybeUninit::<libc::tm>::zeroed().assume_init() };
    tm.tm_year = year - 1900;
    tm.tm_mon = month as i32 - 1;
    tm.tm_mday = day as i32;
    tm.tm_hour = hour as i32;
    tm.tm_min = minute as i32;
    tm.tm_sec = 0;
    tm.tm_isdst = -1; // let mktime figure out DST

    unsafe { libc::mktime(&mut tm) };
    tm.tm_gmtoff
}

/// Returns the UTC offset in seconds for the given date/time in the
/// system's local timezone. Acquires TZ_LOCK to guard against concurrent
/// modifications from `utc_offset_for_tz`.
fn local_utc_offset_secs(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> i64 {
    let _guard = TZ_LOCK.lock().unwrap();
    compute_utc_offset(year, month, day, hour, minute)
}

/// Returns the UTC offset in seconds for a given IANA timezone name
/// (e.g. "America/Chicago") at the specified date/time.
/// Temporarily sets the TZ environment variable, computes the offset,
/// then restores the previous TZ value.
pub fn utc_offset_for_tz(
    tz: &str,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
) -> i64 {
    use std::ffi::CString;

    let _guard = TZ_LOCK.lock().unwrap();

    let tz_cstr = CString::new(tz).unwrap();
    let tz_key = c"TZ".as_ptr();

    // Save current TZ
    let old_tz = std::env::var("TZ").ok();

    unsafe {
        libc::setenv(tz_key, tz_cstr.as_ptr(), 1);
        tzset();
    }

    let offset = compute_utc_offset(year, month, day, hour, minute);

    // Restore previous TZ
    unsafe {
        match &old_tz {
            Some(prev) => {
                let prev_cstr = CString::new(prev.as_str()).unwrap();
                libc::setenv(tz_key, prev_cstr.as_ptr(), 1);
            }
            None => {
                libc::unsetenv(tz_key);
            }
        }
        tzset();
    }

    offset
}

/// Formats a UTC offset in seconds as `+HHMM` / `-HHMM`.
fn format_offset(offset_secs: i64) -> String {
    let sign = if offset_secs < 0 { '-' } else { '+' };
    let abs = offset_secs.unsigned_abs();
    let hours = abs / 3600;
    let minutes = (abs % 3600) / 60;
    format!("{sign}{hours:02}{minutes:02}")
}

/// Returns the local IANA timezone name (e.g. "America/Chicago").
/// Reads from TZ env var first, then falls back to /etc/localtime symlink.
pub fn local_iana_timezone() -> Option<String> {
    // Check TZ env var
    if let Ok(tz) = std::env::var("TZ") {
        let tz = tz.strip_prefix(':').unwrap_or(&tz);
        if !tz.is_empty() {
            return Some(tz.to_string());
        }
    }

    // macOS/Linux: /etc/localtime is a symlink into zoneinfo
    if let Ok(target) = std::fs::read_link("/etc/localtime") {
        let target = target.to_string_lossy().to_string();
        if let Some(idx) = target.find("zoneinfo/") {
            return Some(target[idx + 9..].to_string());
        }
    }

    None
}

/// Returns today's local date as (year, month, day).
fn local_today() -> (i32, u32, u32) {
    use std::mem::MaybeUninit;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as libc::time_t;

    let mut tm = unsafe { MaybeUninit::<libc::tm>::zeroed().assume_init() };
    unsafe { libc::localtime_r(&now, &mut tm) };

    (tm.tm_year + 1900, (tm.tm_mon + 1) as u32, tm.tm_mday as u32)
}

// ---------------------------------------------------------------------------
// ParsedDateTime
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub time: Option<(u32, u32)>, // (hour, minute)
}

impl ParsedDateTime {
    pub fn is_all_day(&self) -> bool {
        self.time.is_none()
    }

    /// Format as a TickTick API datetime string with UTC offset.
    ///
    /// If `tz` is Some, computes the offset for that IANA timezone.
    /// If `tz` is None, uses the local system timezone.
    pub fn to_api_string(&self, tz: Option<&str>) -> String {
        let (hour, minute) = self.time.unwrap_or((0, 0));
        let offset = match tz {
            Some(tz_name) => utc_offset_for_tz(tz_name, self.year, self.month, self.day, hour, minute),
            None => local_utc_offset_secs(self.year, self.month, self.day, hour, minute),
        };
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:00.000{}",
            self.year, self.month, self.day, hour, minute, format_offset(offset)
        )
    }

    pub fn add_duration(&self, duration: &Duration) -> Result<ParsedDateTime, String> {
        let (hour, minute) = self
            .time
            .ok_or_else(|| "cannot add duration to an all-day date (no time component)".to_string())?;

        let total_minutes = (hour * 60 + minute) + (duration.hours * 60 + duration.minutes);
        let extra_days = total_minutes / (24 * 60);
        let remaining = total_minutes % (24 * 60);
        let new_hour = remaining / 60;
        let new_minute = remaining % 60;

        let mut y = self.year;
        let mut m = self.month;
        let mut d = self.day;
        for _ in 0..extra_days {
            (y, m, d) = add_days(y, m, d);
        }

        Ok(ParsedDateTime {
            year: y,
            month: m,
            day: d,
            time: Some((new_hour, new_minute)),
        })
    }
}

// ---------------------------------------------------------------------------
// Duration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Duration {
    pub hours: u32,
    pub minutes: u32,
}

/// Parse a duration string such as `"1h"`, `"30m"`, `"1h30m"`, `"2h15m"`.
///
/// Case-insensitive. Errors on zero duration, negative values, or invalid format.
pub fn parse_duration(input: &str) -> Result<Duration, String> {
    let s = input.trim().to_lowercase();
    if s.is_empty() {
        return Err("invalid duration: empty string".to_string());
    }

    // Reject anything that contains a minus sign.
    if s.contains('-') {
        return Err(format!("invalid duration '{input}': negative durations are not allowed"));
    }

    let mut hours: Option<u32> = None;
    let mut minutes: Option<u32> = None;
    let mut num_buf = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num_buf.push(ch);
        } else if ch == 'h' {
            if num_buf.is_empty() {
                return Err(format!("invalid duration '{input}': expected a number before 'h'"));
            }
            if hours.is_some() {
                return Err(format!("invalid duration '{input}': duplicate 'h' component"));
            }
            hours = Some(
                num_buf
                    .parse()
                    .map_err(|_| format!("invalid duration '{input}'"))?,
            );
            num_buf.clear();
        } else if ch == 'm' {
            if num_buf.is_empty() {
                return Err(format!("invalid duration '{input}': expected a number before 'm'"));
            }
            if minutes.is_some() {
                return Err(format!("invalid duration '{input}': duplicate 'm' component"));
            }
            minutes = Some(
                num_buf
                    .parse()
                    .map_err(|_| format!("invalid duration '{input}'"))?,
            );
            num_buf.clear();
        } else {
            return Err(format!("invalid duration '{input}': unexpected character '{ch}'"));
        }
    }

    // If there are leftover digits with no unit suffix, it's invalid.
    if !num_buf.is_empty() {
        return Err(format!("invalid duration '{input}': missing unit (h or m)"));
    }

    if hours.is_none() && minutes.is_none() {
        return Err(format!("invalid duration '{input}'"));
    }

    let h = hours.unwrap_or(0);
    let m = minutes.unwrap_or(0);

    if h == 0 && m == 0 {
        return Err(format!("invalid duration '{input}': duration must be greater than zero"));
    }

    Ok(Duration {
        hours: h,
        minutes: m,
    })
}

// ---------------------------------------------------------------------------
// parse_datetime
// ---------------------------------------------------------------------------

/// Parse a user-provided datetime string into a `ParsedDateTime`.
///
/// Accepts:
///   - `"today"`           -> today's date, no time
///   - `"tomorrow"`        -> tomorrow's date, no time
///   - `"YYYY-MM-DD"`      -> that date, no time
///   - `"YYYY-MM-DDTHH:MM"` -> that date + time
pub fn parse_datetime(input: &str) -> Result<ParsedDateTime, String> {
    let trimmed = input.trim().to_lowercase();

    match trimmed.as_str() {
        "today" => {
            let (year, month, day) = local_today();
            Ok(ParsedDateTime {
                year,
                month,
                day,
                time: None,
            })
        }
        "tomorrow" => {
            let (year, month, day) = local_today();
            let (year, month, day) = add_days(year, month, day);
            Ok(ParsedDateTime {
                year,
                month,
                day,
                time: None,
            })
        }
        _ => {
            // Check for YYYY-MM-DDTHH:MM
            if let Some((date_part, time_part)) = trimmed.split_once('t') {
                let (year, month, day) = parse_date_part(date_part, input)?;
                let (hour, minute) = parse_time_part(time_part, input)?;
                Ok(ParsedDateTime {
                    year,
                    month,
                    day,
                    time: Some((hour, minute)),
                })
            } else {
                let (year, month, day) = parse_date_part(&trimmed, input)?;
                Ok(ParsedDateTime {
                    year,
                    month,
                    day,
                    time: None,
                })
            }
        }
    }
}

fn parse_date_part(s: &str, original: &str) -> Result<(i32, u32, u32), String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(format!(
            "invalid date '{original}': expected YYYY-MM-DD, 'today', or 'tomorrow'"
        ));
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| format!("invalid year in '{original}'"))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| format!("invalid month in '{original}'"))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| format!("invalid day in '{original}'"))?;

    if !(1..=12).contains(&month) {
        return Err(format!("month out of range in '{original}'"));
    }
    if !(1..=31).contains(&day) {
        return Err(format!("day out of range in '{original}'"));
    }

    Ok((year, month, day))
}

fn parse_time_part(s: &str, original: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "invalid time in '{original}': expected HH:MM"
        ));
    }
    let hour: u32 = parts[0]
        .parse()
        .map_err(|_| format!("invalid hour in '{original}'"))?;
    let minute: u32 = parts[1]
        .parse()
        .map_err(|_| format!("invalid minute in '{original}'"))?;

    if hour > 23 {
        return Err(format!("hour out of range in '{original}'"));
    }
    if minute > 59 {
        return Err(format!("minute out of range in '{original}'"));
    }

    Ok((hour, minute))
}

// ---------------------------------------------------------------------------
// DateField + TaskFields
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub enum DateField {
    Set(String), // API-formatted datetime string
    Clear,
}

#[derive(Default, Clone)]
pub struct TaskFields {
    pub title: Option<String>,
    pub project_id: Option<String>,
    pub due_date: Option<DateField>,
    pub start_date: Option<DateField>,
    pub priority: Option<i32>,
    pub is_all_day: Option<bool>,
    pub time_zone: Option<String>,
    pub content: Option<Option<String>>,
    pub desc: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    pub items: Option<Vec<ChecklistItem>>,
    pub reminders: Option<Vec<String>>,
    pub repeat_flag: Option<Option<String>>,
}

impl TaskFields {
    pub fn apply_to(&self, body: &mut serde_json::Value) {
        if let Some(ref t) = self.title {
            body["title"] = serde_json::Value::String(t.clone());
        }
        if let Some(ref pid) = self.project_id {
            body["projectId"] = serde_json::Value::String(pid.clone());
        }
        match &self.due_date {
            Some(DateField::Set(d)) => {
                body["dueDate"] = serde_json::Value::String(d.clone());
            }
            Some(DateField::Clear) => {
                body["dueDate"] = serde_json::Value::Null;
            }
            None => {}
        }
        match &self.start_date {
            Some(DateField::Set(d)) => {
                body["startDate"] = serde_json::Value::String(d.clone());
            }
            Some(DateField::Clear) => {
                body["startDate"] = serde_json::Value::Null;
            }
            None => {}
        }
        if let Some(p) = self.priority {
            body["priority"] = serde_json::Value::Number(p.into());
        }
        if let Some(all_day) = self.is_all_day {
            body["isAllDay"] = serde_json::Value::Bool(all_day);
        }
        if let Some(ref tz) = self.time_zone {
            body["timeZone"] = serde_json::Value::String(tz.clone());
        }
        match &self.content {
            Some(Some(c)) => { body["content"] = serde_json::Value::String(c.clone()); }
            Some(None) => { body["content"] = serde_json::Value::Null; }
            None => {}
        }
        match &self.desc {
            Some(Some(d)) => { body["desc"] = serde_json::Value::String(d.clone()); }
            Some(None) => { body["desc"] = serde_json::Value::Null; }
            None => {}
        }
        if let Some(ref tags) = self.tags {
            body["tags"] = serde_json::Value::Array(
                tags.iter().map(|t| serde_json::Value::String(t.clone())).collect(),
            );
        }
        if let Some(ref items) = self.items {
            body["items"] = serde_json::to_value(items).unwrap();
        }
        if let Some(ref reminders) = self.reminders {
            body["reminders"] = serde_json::Value::Array(
                reminders.iter().map(|r| serde_json::Value::String(r.clone())).collect(),
            );
        }
        match &self.repeat_flag {
            Some(Some(rf)) => { body["repeatFlag"] = serde_json::Value::String(rf.clone()); }
            Some(None) => { body["repeatFlag"] = serde_json::Value::Null; }
            None => {}
        }
    }
}

// ---------------------------------------------------------------------------
// CRUD operations
// ---------------------------------------------------------------------------

pub fn create(token: &str, fields: &TaskFields) -> Result<Task, String> {
    let title = fields
        .title
        .as_ref()
        .ok_or_else(|| "title is required to create a task".to_string())?;

    let mut body = serde_json::json!({
        "title": title
    });

    fields.apply_to(&mut body);

    let resp = super::post("/task", token, &body)?;

    resp.into_json()
        .map_err(|e| format!("failed to parse task: {e}"))
}

pub fn update(
    token: &str,
    project_id: &str,
    task_id: &str,
    fields: &TaskFields,
) -> Result<Task, String> {
    let mut body = serde_json::json!({
        "taskId": task_id,
        "projectId": project_id,
    });

    fields.apply_to(&mut body);

    let resp = super::post(&format!("/task/{task_id}"), token, &body)?;

    resp.into_json()
        .map_err(|e| format!("failed to parse task: {e}"))
}

pub fn complete(token: &str, project_id: &str, task_id: &str) -> Result<(), String> {
    super::post_empty(
        &format!("/project/{project_id}/task/{task_id}/complete"),
        token,
    )?;

    Ok(())
}

pub fn delete(token: &str, project_id: &str, task_id: &str) -> Result<(), String> {
    super::delete(&format!("/project/{project_id}/task/{task_id}"), token)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse_datetime -------------------------------------------------------

    #[test]
    fn parse_datetime_today() {
        let result = parse_datetime("today").unwrap();
        assert!(result.is_all_day());
        assert!(result.time.is_none());
        // We can't assert exact date without freezing time, but we can verify
        // it produces a valid date.
        assert!((1..=12).contains(&result.month));
        assert!((1..=31).contains(&result.day));
    }

    #[test]
    fn parse_datetime_tomorrow() {
        let result = parse_datetime("tomorrow").unwrap();
        assert!(result.is_all_day());
        assert!(result.time.is_none());
        assert!((1..=12).contains(&result.month));
        assert!((1..=31).contains(&result.day));
    }

    #[test]
    fn parse_datetime_date_only() {
        let result = parse_datetime("2026-02-16").unwrap();
        assert_eq!(
            result,
            ParsedDateTime {
                year: 2026,
                month: 2,
                day: 16,
                time: None,
            }
        );
    }

    #[test]
    fn parse_datetime_date_and_time() {
        let result = parse_datetime("2026-02-16T14:30").unwrap();
        assert_eq!(
            result,
            ParsedDateTime {
                year: 2026,
                month: 2,
                day: 16,
                time: Some((14, 30)),
            }
        );
    }

    #[test]
    fn parse_datetime_case_insensitive() {
        let result = parse_datetime("2026-02-16t09:05").unwrap();
        assert_eq!(result.time, Some((9, 5)));
    }

    #[test]
    fn parse_datetime_invalid_format() {
        assert!(parse_datetime("not-a-date").is_err());
        assert!(parse_datetime("2026/02/16").is_err());
        assert!(parse_datetime("").is_err());
    }

    #[test]
    fn parse_datetime_month_out_of_range() {
        assert!(parse_datetime("2026-13-01").is_err());
        assert!(parse_datetime("2026-00-01").is_err());
    }

    #[test]
    fn parse_datetime_day_out_of_range() {
        assert!(parse_datetime("2026-01-00").is_err());
        assert!(parse_datetime("2026-01-32").is_err());
    }

    #[test]
    fn parse_datetime_hour_out_of_range() {
        assert!(parse_datetime("2026-01-15T24:00").is_err());
    }

    #[test]
    fn parse_datetime_minute_out_of_range() {
        assert!(parse_datetime("2026-01-15T12:60").is_err());
    }

    // -- parse_duration -------------------------------------------------------

    #[test]
    fn parse_duration_hours_only() {
        assert_eq!(
            parse_duration("1h").unwrap(),
            Duration {
                hours: 1,
                minutes: 0
            }
        );
    }

    #[test]
    fn parse_duration_minutes_only() {
        assert_eq!(
            parse_duration("30m").unwrap(),
            Duration {
                hours: 0,
                minutes: 30
            }
        );
    }

    #[test]
    fn parse_duration_hours_and_minutes() {
        assert_eq!(
            parse_duration("1h30m").unwrap(),
            Duration {
                hours: 1,
                minutes: 30
            }
        );
        assert_eq!(
            parse_duration("2h15m").unwrap(),
            Duration {
                hours: 2,
                minutes: 15
            }
        );
    }

    #[test]
    fn parse_duration_case_insensitive() {
        assert_eq!(
            parse_duration("1H30M").unwrap(),
            Duration {
                hours: 1,
                minutes: 30
            }
        );
    }

    #[test]
    fn parse_duration_zero_hours() {
        assert!(parse_duration("0h").is_err());
    }

    #[test]
    fn parse_duration_zero_minutes() {
        assert!(parse_duration("0m").is_err());
    }

    #[test]
    fn parse_duration_empty() {
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn parse_duration_no_unit() {
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("123").is_err());
    }

    #[test]
    fn parse_duration_negative() {
        assert!(parse_duration("-1h").is_err());
        assert!(parse_duration("-30m").is_err());
    }

    // -- ParsedDateTime::add_duration -----------------------------------------

    #[test]
    fn add_duration_same_day() {
        let dt = ParsedDateTime {
            year: 2026,
            month: 2,
            day: 16,
            time: Some((10, 0)),
        };
        let dur = Duration {
            hours: 1,
            minutes: 30,
        };
        let result = dt.add_duration(&dur).unwrap();
        assert_eq!(
            result,
            ParsedDateTime {
                year: 2026,
                month: 2,
                day: 16,
                time: Some((11, 30)),
            }
        );
    }

    #[test]
    fn add_duration_day_overflow() {
        // 23:30 + 1h = next day 00:30
        let dt = ParsedDateTime {
            year: 2026,
            month: 2,
            day: 16,
            time: Some((23, 30)),
        };
        let dur = Duration {
            hours: 1,
            minutes: 0,
        };
        let result = dt.add_duration(&dur).unwrap();
        assert_eq!(
            result,
            ParsedDateTime {
                year: 2026,
                month: 2,
                day: 17,
                time: Some((0, 30)),
            }
        );
    }

    #[test]
    fn add_duration_month_overflow() {
        // Jan 31 23:00 + 2h = Feb 1 01:00
        let dt = ParsedDateTime {
            year: 2026,
            month: 1,
            day: 31,
            time: Some((23, 0)),
        };
        let dur = Duration {
            hours: 2,
            minutes: 0,
        };
        let result = dt.add_duration(&dur).unwrap();
        assert_eq!(
            result,
            ParsedDateTime {
                year: 2026,
                month: 2,
                day: 1,
                time: Some((1, 0)),
            }
        );
    }

    #[test]
    fn add_duration_leap_year() {
        // Feb 28 2024 23:30 + 1h = Feb 29 2024 00:30 (2024 is a leap year)
        let dt = ParsedDateTime {
            year: 2024,
            month: 2,
            day: 28,
            time: Some((23, 30)),
        };
        let dur = Duration {
            hours: 1,
            minutes: 0,
        };
        let result = dt.add_duration(&dur).unwrap();
        assert_eq!(
            result,
            ParsedDateTime {
                year: 2024,
                month: 2,
                day: 29,
                time: Some((0, 30)),
            }
        );
    }

    #[test]
    fn add_duration_errors_on_all_day() {
        let dt = ParsedDateTime {
            year: 2026,
            month: 2,
            day: 16,
            time: None,
        };
        let dur = Duration {
            hours: 1,
            minutes: 0,
        };
        assert!(dt.add_duration(&dur).is_err());
    }

    // -- ParsedDateTime::to_api_string ----------------------------------------

    #[test]
    fn to_api_string_date_only_local() {
        let dt = ParsedDateTime {
            year: 2026,
            month: 2,
            day: 16,
            time: None,
        };
        let s = dt.to_api_string(None);
        assert!(s.starts_with("2026-02-16T00:00:00.000"));
        // Offset depends on local timezone; verify format
        let offset = &s["2026-02-16T00:00:00.000".len()..];
        assert!(
            offset.len() == 5 && (offset.starts_with('+') || offset.starts_with('-')),
            "expected +HHMM or -HHMM offset, got: {offset}"
        );
    }

    #[test]
    fn to_api_string_datetime_local() {
        let dt = ParsedDateTime {
            year: 2026,
            month: 2,
            day: 16,
            time: Some((14, 30)),
        };
        let s = dt.to_api_string(None);
        assert!(s.starts_with("2026-02-16T14:30:00.000"));
        let offset = &s["2026-02-16T14:30:00.000".len()..];
        assert!(
            offset.len() == 5 && (offset.starts_with('+') || offset.starts_with('-')),
            "expected +HHMM or -HHMM offset, got: {offset}"
        );
    }

    #[test]
    fn to_api_string_explicit_timezone() {
        let dt = ParsedDateTime {
            year: 2026,
            month: 7,
            day: 15,
            time: Some((14, 0)),
        };
        // UTC should always be +0000
        assert_eq!(
            dt.to_api_string(Some("UTC")),
            "2026-07-15T14:00:00.000+0000"
        );
    }

    #[test]
    fn local_utc_offset_format() {
        // Verify format_offset produces valid +HHMM / -HHMM strings
        assert_eq!(format_offset(0), "+0000");
        assert_eq!(format_offset(-21600), "-0600"); // CST (Chicago winter)
        assert_eq!(format_offset(19800), "+0530"); // IST (India)
        assert_eq!(format_offset(-18000), "-0500"); // EST / CDT
    }

    #[test]
    fn utc_offset_for_known_timezone() {
        // UTC should always be 0
        assert_eq!(utc_offset_for_tz("UTC", 2026, 6, 15, 12, 0), 0);
    }

    // -- ParsedDateTime::is_all_day -------------------------------------------

    #[test]
    fn is_all_day_true() {
        let dt = ParsedDateTime {
            year: 2026,
            month: 1,
            day: 1,
            time: None,
        };
        assert!(dt.is_all_day());
    }

    #[test]
    fn is_all_day_false() {
        let dt = ParsedDateTime {
            year: 2026,
            month: 1,
            day: 1,
            time: Some((0, 0)),
        };
        assert!(!dt.is_all_day());
    }

    // -- is_leap_year ---------------------------------------------------------

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2024));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(2023));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2400));
    }

    // -- days_in_month --------------------------------------------------------

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2026, 1), 31);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }
}
