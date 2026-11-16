use jiff::civil::DateTime;
use jiff::tz::{AmbiguousOffset, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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

#[derive(Debug, Default, Deserialize, Serialize)]
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
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub column_id: Option<String>,
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
        // Also include inbox tasks if we know the inbox ID
        if let Some(inbox_id) = crate::config::get_inbox_project_id()
            && let Ok(resp) = super::get(&format!("/project/{inbox_id}/data"), token)
            && let Ok(data) = resp.into_json::<ProjectData>()
        {
            all_tasks.extend(data.tasks);
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
// Time zones
// ---------------------------------------------------------------------------

/// The named IANA zone, else [`system_zone`]. jiff matches names
/// case-insensitively and accepts `Etc/Unknown`, which has no IANA name.
pub fn time_zone(name: Option<&str>) -> Result<TimeZone, String> {
    match name {
        Some(name) => TimeZone::get(name)
            .ok()
            .filter(|zone| zone.iana_name().is_some())
            .ok_or_else(|| format!("Unknown timezone '{name}'")),
        None => Ok(system_zone()),
    }
}

/// The IANA spelling of `name`, e.g. "america/chicago" -> "America/Chicago".
pub fn canonical_timezone(name: &str) -> Result<String, String> {
    let zone = time_zone(Some(name))?;
    Ok(zone.iana_name().unwrap_or(name).to_string())
}

/// jiff reads a set-but-empty `TZ` as UTC; libc and Python read it as unset.
pub fn system_zone() -> TimeZone {
    if std::env::var_os("TZ").is_some_and(|tz| tz.is_empty())
        && let Some(zone) = localtime_zone()
    {
        return zone;
    }
    TimeZone::system()
}

fn localtime_zone() -> Option<TimeZone> {
    let target = std::fs::read_link("/etc/localtime").ok()?;
    let target = target.to_string_lossy();
    let name = &target[target.find("zoneinfo/")? + "zoneinfo/".len()..];
    TimeZone::get(name).ok()
}

/// UTC offset in seconds of a wall-clock time in `tz`. Inside a DST gap or
/// fold the pre-transition offset applies, like Python's `fold=0`.
pub fn wall_offset_secs(tz: &TimeZone, dt: DateTime) -> i64 {
    let offset = match tz.to_ambiguous_timestamp(dt).offset() {
        AmbiguousOffset::Unambiguous { offset }
        | AmbiguousOffset::Gap { before: offset, .. }
        | AmbiguousOffset::Fold { before: offset, .. } => offset,
    };
    i64::from(offset.seconds())
}

pub fn civil_datetime(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
) -> Result<DateTime, String> {
    let invalid =
        || format!("invalid date/time {year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}");
    let narrow = |v: i64| i8::try_from(v).map_err(|_| invalid());
    DateTime::new(
        i16::try_from(year).map_err(|_| invalid())?,
        narrow(month)?,
        narrow(day)?,
        narrow(hour)?,
        narrow(minute)?,
        narrow(second)?,
        0,
    )
    .map_err(|_| invalid())
}

pub fn utc_offset_in(
    zone: &TimeZone,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
) -> Result<i64, String> {
    let dt = civil_datetime(
        year.into(),
        month.into(),
        day.into(),
        hour.into(),
        minute.into(),
        0,
    )?;
    Ok(wall_offset_secs(zone, dt))
}

/// Formats a UTC offset in seconds as `+HHMM` / `-HHMM`.
fn format_offset(offset_secs: i64) -> String {
    let sign = if offset_secs < 0 { '-' } else { '+' };
    let abs = offset_secs.unsigned_abs();
    let hours = abs / 3600;
    let minutes = (abs % 3600) / 60;
    format!("{sign}{hours:02}{minutes:02}")
}

pub fn local_iana_timezone() -> Option<String> {
    system_zone().iana_name().map(str::to_string)
}

pub fn local_today() -> (i32, u32, u32) {
    today_in(&system_zone())
}

/// Today's date in `zone` as (year, month, day).
pub fn today_in(zone: &TimeZone) -> (i32, u32, u32) {
    let today = jiff::Timestamp::now()
        .to_zoned_datetime(zone.clone())
        .date();
    (
        today.year().into(),
        today.month().unsigned_abs().into(),
        today.day().unsigned_abs().into(),
    )
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
    /// `tz` is an IANA name; None means the system zone.
    pub fn to_api_string(&self, tz: Option<&str>) -> Result<String, String> {
        self.to_api_string_in(&time_zone(tz)?)
    }

    pub fn to_api_string_in(&self, zone: &TimeZone) -> Result<String, String> {
        let (hour, minute) = self.time.unwrap_or((0, 0));
        let offset = utc_offset_in(zone, self.year, self.month, self.day, hour, minute)?;
        Ok(format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:00.000{}",
            self.year,
            self.month,
            self.day,
            hour,
            minute,
            format_offset(offset)
        ))
    }

    pub fn add_duration(&self, duration: &Duration) -> Result<ParsedDateTime, String> {
        let (hour, minute) = self.time.ok_or_else(|| {
            "cannot add duration to an all-day date (no time component)".to_string()
        })?;

        let total_minutes = i64::from(hour) * 60
            + i64::from(minute)
            + i64::from(duration.hours) * 60
            + i64::from(duration.minutes);
        let remaining = total_minutes % (24 * 60);
        // Python's OverflowError text.
        let out_of_range = |_| "date value out of range".to_string();
        let date = civil_datetime(
            self.year.into(),
            self.month.into(),
            self.day.into(),
            0,
            0,
            0,
        )?
        .date()
        .checked_add(
            jiff::Span::new()
                .try_days(total_minutes / (24 * 60))
                .map_err(out_of_range)?,
        )
        .map_err(out_of_range)?;

        Ok(ParsedDateTime {
            year: date.year().into(),
            month: date.month().unsigned_abs().into(),
            day: date.day().unsigned_abs().into(),
            time: Some(((remaining / 60) as u32, (remaining % 60) as u32)),
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
        return Err(format!(
            "invalid duration '{input}': negative durations are not allowed"
        ));
    }

    let mut hours: Option<u32> = None;
    let mut minutes: Option<u32> = None;
    let mut num_buf = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num_buf.push(ch);
        } else if ch == 'h' {
            if num_buf.is_empty() {
                return Err(format!(
                    "invalid duration '{input}': expected a number before 'h'"
                ));
            }
            if hours.is_some() {
                return Err(format!(
                    "invalid duration '{input}': duplicate 'h' component"
                ));
            }
            hours = Some(
                num_buf
                    .parse()
                    .map_err(|_| format!("invalid duration '{input}'"))?,
            );
            num_buf.clear();
        } else if ch == 'm' {
            if num_buf.is_empty() {
                return Err(format!(
                    "invalid duration '{input}': expected a number before 'm'"
                ));
            }
            if minutes.is_some() {
                return Err(format!(
                    "invalid duration '{input}': duplicate 'm' component"
                ));
            }
            minutes = Some(
                num_buf
                    .parse()
                    .map_err(|_| format!("invalid duration '{input}'"))?,
            );
            num_buf.clear();
        } else {
            return Err(format!(
                "invalid duration '{input}': unexpected character '{ch}'"
            ));
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
        return Err(format!(
            "invalid duration '{input}': duration must be greater than zero"
        ));
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
    parse_datetime_in(input, &system_zone())
}

/// Like [`parse_datetime`], with "today" and "tomorrow" taken in `zone`.
pub fn parse_datetime_in(input: &str, zone: &TimeZone) -> Result<ParsedDateTime, String> {
    let trimmed = input.trim().to_lowercase();

    match trimmed.as_str() {
        "today" => {
            let (year, month, day) = today_in(zone);
            Ok(ParsedDateTime {
                year,
                month,
                day,
                time: None,
            })
        }
        "tomorrow" => {
            let (year, month, day) = today_in(zone);
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
        return Err(format!("invalid time in '{original}': expected HH:MM"));
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

/// Reminder at the due time for timed tasks (TickTick's own default).
pub const DEFAULT_TIMED_REMINDER: &str = "TRIGGER:PT0S";
/// Reminder at 9am for all-day tasks (TickTick's own default).
pub const DEFAULT_ALL_DAY_REMINDER: &str = "TRIGGER:P0DT9H0M0S";

/// Reminders to send when creating a task. Explicit `--reminder` values win;
/// `--no-reminder` sends an empty list; otherwise a dated task gets the default.
/// `None` means send no `reminders` field at all.
pub fn add_reminders(
    explicit: Vec<String>,
    no_reminder: bool,
    is_all_day: Option<bool>,
    has_datetime: bool,
) -> Option<Vec<String>> {
    if !explicit.is_empty() {
        return Some(explicit);
    }
    if no_reminder {
        return Some(Vec::new());
    }
    if !has_datetime {
        return None;
    }
    if is_all_day == Some(true) {
        Some(vec![DEFAULT_ALL_DAY_REMINDER.to_string()])
    } else {
        Some(vec![DEFAULT_TIMED_REMINDER.to_string()])
    }
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
    pub parent_id: Option<Option<String>>,
    pub column_id: Option<String>,
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
            Some(Some(c)) => {
                body["content"] = serde_json::Value::String(c.clone());
            }
            Some(None) => {
                body["content"] = serde_json::Value::Null;
            }
            None => {}
        }
        match &self.desc {
            Some(Some(d)) => {
                body["desc"] = serde_json::Value::String(d.clone());
            }
            Some(None) => {
                body["desc"] = serde_json::Value::Null;
            }
            None => {}
        }
        if let Some(ref tags) = self.tags {
            body["tags"] = serde_json::Value::Array(
                tags.iter()
                    .map(|t| serde_json::Value::String(t.clone()))
                    .collect(),
            );
        }
        if let Some(ref items) = self.items {
            body["items"] = serde_json::to_value(items).unwrap();
        }
        if let Some(ref reminders) = self.reminders {
            body["reminders"] = serde_json::Value::Array(
                reminders
                    .iter()
                    .map(|r| serde_json::Value::String(r.clone()))
                    .collect(),
            );
        }
        match &self.repeat_flag {
            Some(Some(rf)) => {
                body["repeatFlag"] = serde_json::Value::String(rf.clone());
            }
            Some(None) => {
                body["repeatFlag"] = serde_json::Value::Null;
            }
            None => {}
        }
        if let Some(ref cid) = self.column_id {
            body["columnId"] = serde_json::Value::String(cid.clone());
        }
        match &self.parent_id {
            Some(Some(pid)) => {
                body["parentId"] = serde_json::Value::String(pid.clone());
            }
            Some(None) => {
                body["parentId"] = serde_json::Value::String(String::new());
            }
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

/// Most task IDs the batch complete endpoint accepts per request.
pub const COMPLETE_BATCH_SIZE: usize = 50;

/// Request body for `POST /task/completeTasks`.
fn complete_tasks_body(project_id: &str, task_ids: &[String]) -> serde_json::Value {
    serde_json::json!({ "projectId": project_id, "taskIds": task_ids })
}

/// One `completeTasks` request: its IDs, and the IDs the endpoint reported
/// completed or the request's error.
pub struct CompleteChunk {
    pub ids: Vec<String>,
    pub completed: Result<HashSet<String>, String>,
}

/// Complete tasks with the batch endpoint, 50 per request, duplicates removed
/// and order kept. A failed request doesn't stop the later ones.
pub fn complete_chunks(token: &str, project_id: &str, task_ids: &[String]) -> Vec<CompleteChunk> {
    let mut seen = HashSet::new();
    let ids: Vec<String> = task_ids
        .iter()
        .filter(|id| seen.insert(id.as_str()))
        .cloned()
        .collect();
    ids.chunks(COMPLETE_BATCH_SIZE)
        .map(|chunk| {
            let body = complete_tasks_body(project_id, chunk);
            let completed = super::post("/task/completeTasks", token, &body).and_then(|resp| {
                resp.into_json::<Vec<String>>()
                    .map(|done| done.into_iter().collect())
                    .map_err(|e| format!("failed to parse completeTasks response: {e}"))
            });
            CompleteChunk {
                ids: chunk.to_vec(),
                completed,
            }
        })
        .collect()
}

/// [`complete_chunks`] as one result per ID. The endpoint returns only the IDs
/// it completed, so unknown IDs come back as errors.
pub fn complete_many(
    token: &str,
    project_id: &str,
    task_ids: &[String],
) -> Vec<(String, Result<(), String>)> {
    complete_chunks(token, project_id, task_ids)
        .into_iter()
        .flat_map(|CompleteChunk { ids, completed }| {
            ids.into_iter().map(move |id| {
                let r = match &completed {
                    Ok(done) if done.contains(&id) => Ok(()),
                    Ok(_) => Err("not completed (task not found in project?)".to_string()),
                    Err(e) => Err(e.clone()),
                };
                (id, r)
            })
        })
        .collect()
}

pub fn delete(token: &str, project_id: &str, task_id: &str) -> Result<(), String> {
    super::delete(&format!("/project/{project_id}/task/{task_id}"), token)?;

    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMove {
    pub task_id: String,
    pub from_project_id: String,
    pub to_project_id: String,
}

/// Move tasks between projects in one request.
pub fn move_tasks(token: &str, moves: &[TaskMove]) -> Result<(), String> {
    let body = serde_json::to_value(moves)
        .map_err(|e| format!("failed to serialize move request: {e}"))?;
    super::post("/task/move", token, &body)?;
    Ok(())
}

/// Criteria for `POST /task/search`. `keywords` is required by the server
/// (an empty search returns nothing); the rest are optional.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSearch {
    pub keywords: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub project_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub status: Vec<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_to: Option<String>,
}

/// Criteria for `POST /task/filter`. Every field is optional and they combine.
/// Note the singular `tag`, unlike search's `tags`.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFilter {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub project_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub priority: Vec<i32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tag: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub kind: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub status: Vec<i32>,
}

fn post_for_tasks(endpoint: &str, token: &str, body: &impl Serialize) -> Result<Vec<Task>, String> {
    let body = serde_json::to_value(body)
        .map_err(|e| format!("failed to serialize {endpoint} request: {e}"))?;
    let resp = super::post(endpoint, token, &body)?;
    resp.into_json::<Option<Vec<Task>>>()
        .map(Option::unwrap_or_default)
        .map_err(|e| format!("failed to parse {endpoint} response: {e}"))
}

/// Server-side keyword search (whole words, case-insensitive).
pub fn search(token: &str, search: &TaskSearch) -> Result<Vec<Task>, String> {
    post_for_tasks("/task/search", token, search)
}

/// Server-side filter by project, dates, priority, tags, status, and kind.
pub fn filter(token: &str, filter: &TaskFilter) -> Result<Vec<Task>, String> {
    post_for_tasks("/task/filter", token, filter)
}

/// Request body for `POST /task/assign`.
pub fn assign_body(project_id: &str, task_id: &str, assignee: &str) -> serde_json::Value {
    serde_json::json!({
        "projectId": project_id,
        "taskId": task_id,
        "assigneeUsername": assignee,
    })
}

/// Request body for `POST /task/unassign`.
pub fn unassign_body(project_id: &str, task_id: &str) -> serde_json::Value {
    serde_json::json!({ "projectId": project_id, "taskId": task_id })
}

/// Assign a task to a member of its (shared) project. Returns the updated task.
pub fn assign(
    token: &str,
    project_id: &str,
    task_id: &str,
    assignee: &str,
) -> Result<serde_json::Value, String> {
    let resp = super::post(
        "/task/assign",
        token,
        &assign_body(project_id, task_id, assignee),
    )
    .map_err(|e| match assign_membership_status(&e) {
        Some(_) => {
            format!(
                "{e} (the assignee must be a member of a shared project; see 'project members')"
            )
        }
        None => e,
    })?;
    resp.into_json()
        .map_err(|e| format!("failed to parse assign response: {e}"))
}

/// The status of an assign error that means "not a project member": a bare
/// 500 for a non-member username (seen live by the Python server), or a 403.
pub fn assign_membership_status(err: &str) -> Option<u16> {
    let rest = &err[err.find("status code ")? + "status code ".len()..];
    let code = rest
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    matches!(code, 403 | 500).then_some(code)
}

/// Remove the assignee from a task. Returns the updated task.
pub fn unassign(token: &str, project_id: &str, task_id: &str) -> Result<serde_json::Value, String> {
    let resp = super::post("/task/unassign", token, &unassign_body(project_id, task_id))?;
    resp.into_json()
        .map_err(|e| format!("failed to parse unassign response: {e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assign_membership_status_only_for_403_and_500() {
        for code in [403, 500] {
            let e = format!("API request failed: http://x/task/assign: status code {code}");
            assert_eq!(assign_membership_status(&e), Some(code));
        }
        for e in [
            "API request failed: x: status code 404",
            "API request failed: x: status code 5000",
            "API request failed: timed out",
            "token expired",
        ] {
            assert_eq!(assign_membership_status(e), None, "{e}");
        }
    }

    #[test]
    fn task_move_serializes_to_v1_shape() {
        let moves = [TaskMove {
            task_id: "t1".into(),
            from_project_id: "inbox1".into(),
            to_project_id: "p2".into(),
        }];
        assert_eq!(
            serde_json::to_value(moves).unwrap(),
            serde_json::json!([{ "taskId": "t1", "fromProjectId": "inbox1", "toProjectId": "p2" }])
        );
    }

    #[test]
    fn task_search_serializes_only_set_fields() {
        let minimal = TaskSearch {
            keywords: "quokka".into(),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(&minimal).unwrap(),
            serde_json::json!({ "keywords": "quokka" })
        );

        let full = TaskSearch {
            keywords: "plan".into(),
            project_ids: vec!["p1".into()],
            tags: vec!["a".into()],
            status: vec![0, 2],
            due_from: Some("2030-01-01T00:00:00.000+0000".into()),
            due_to: Some("2030-02-01T00:00:00.000+0000".into()),
        };
        assert_eq!(
            serde_json::to_value(&full).unwrap(),
            serde_json::json!({
                "keywords": "plan",
                "projectIds": ["p1"],
                "tags": ["a"],
                "status": [0, 2],
                "dueFrom": "2030-01-01T00:00:00.000+0000",
                "dueTo": "2030-02-01T00:00:00.000+0000"
            })
        );
    }

    #[test]
    fn task_filter_serializes_to_v1_shape() {
        assert_eq!(
            serde_json::to_value(TaskFilter::default()).unwrap(),
            serde_json::json!({})
        );

        let filter = TaskFilter {
            project_ids: vec!["p1".into()],
            start_date: Some("2030-01-01T00:00:00.000+0000".into()),
            end_date: Some("2030-02-01T00:00:00.000+0000".into()),
            priority: vec![5, 3],
            tag: vec!["a".into()],
            kind: vec!["TASK".into()],
            status: vec![-1],
        };
        assert_eq!(
            serde_json::to_value(&filter).unwrap(),
            serde_json::json!({
                "projectIds": ["p1"],
                "startDate": "2030-01-01T00:00:00.000+0000",
                "endDate": "2030-02-01T00:00:00.000+0000",
                "priority": [5, 3],
                "tag": ["a"],
                "kind": ["TASK"],
                "status": [-1]
            })
        );
    }

    // -- add_reminders --------------------------------------------------------

    #[test]
    fn add_reminders_timed_defaults_to_at_due_time() {
        assert_eq!(
            add_reminders(vec![], false, Some(false), true),
            Some(vec!["TRIGGER:PT0S".to_string()])
        );
    }

    #[test]
    fn add_reminders_all_day_defaults_to_9am() {
        assert_eq!(
            add_reminders(vec![], false, Some(true), true),
            Some(vec!["TRIGGER:P0DT9H0M0S".to_string()])
        );
    }

    #[test]
    fn add_reminders_no_date_sends_nothing() {
        assert_eq!(add_reminders(vec![], false, None, false), None);
    }

    #[test]
    fn add_reminders_explicit_wins() {
        let r = vec!["TRIGGER:-PT15M".to_string()];
        assert_eq!(add_reminders(r.clone(), false, Some(false), true), Some(r));
    }

    #[test]
    fn add_reminders_opt_out_sends_empty_list() {
        assert_eq!(
            add_reminders(vec![], true, Some(false), true),
            Some(Vec::new())
        );
        assert_eq!(add_reminders(vec![], true, None, false), Some(Vec::new()));
    }

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
        let s = dt.to_api_string(None).unwrap();
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
        let s = dt.to_api_string(None).unwrap();
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
            dt.to_api_string(Some("UTC")).unwrap(),
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

    // -- Task parentId ---------------------------------------------------------

    #[test]
    fn task_deserialize_with_parent_id() {
        let json = r#"{"id":"t1","title":"child","parentId":"p1"}"#;
        let task: Task = serde_json::from_str(json).unwrap();
        assert_eq!(task.parent_id.as_deref(), Some("p1"));
    }

    #[test]
    fn task_deserialize_without_parent_id() {
        let json = r#"{"id":"t1","title":"standalone"}"#;
        let task: Task = serde_json::from_str(json).unwrap();
        assert!(task.parent_id.is_none());
    }

    #[test]
    fn task_fields_apply_column_id() {
        let fields = TaskFields {
            column_id: Some("col1".to_string()),
            ..Default::default()
        };
        let mut body = serde_json::json!({});
        fields.apply_to(&mut body);
        assert_eq!(body["columnId"], "col1");

        let mut body = serde_json::json!({});
        TaskFields::default().apply_to(&mut body);
        assert!(body.get("columnId").is_none());
    }

    #[test]
    fn task_fields_apply_parent_id_set() {
        let fields = TaskFields {
            parent_id: Some(Some("parent123".to_string())),
            ..Default::default()
        };
        let mut body = serde_json::json!({});
        fields.apply_to(&mut body);
        assert_eq!(body["parentId"], "parent123");
    }

    #[test]
    fn task_fields_apply_parent_id_clear() {
        let fields = TaskFields {
            parent_id: Some(None),
            ..Default::default()
        };
        let mut body = serde_json::json!({});
        fields.apply_to(&mut body);
        assert_eq!(body["parentId"], "");
    }

    #[test]
    fn task_fields_apply_parent_id_skip() {
        let fields = TaskFields {
            parent_id: None,
            ..Default::default()
        };
        let mut body = serde_json::json!({});
        fields.apply_to(&mut body);
        assert!(body.get("parentId").is_none());
    }

    #[test]
    fn utc_offset_across_dst_transitions() {
        let zone = time_zone(Some("America/Chicago")).unwrap();
        let chicago = |m, d, h, mi| utc_offset_in(&zone, 2026, m, d, h, mi);
        // Spring forward 2026-03-08 02:00 CST -> 03:00 CDT.
        assert_eq!(chicago(3, 8, 1, 59), Ok(-6 * 3600));
        assert_eq!(chicago(3, 8, 2, 30), Ok(-6 * 3600));
        assert_eq!(chicago(3, 8, 3, 0), Ok(-5 * 3600));
        // Fall back 2026-11-01 02:00 CDT -> 01:00 CST; 01:30 happens twice.
        assert_eq!(chicago(11, 1, 1, 30), Ok(-5 * 3600));
        assert_eq!(chicago(11, 1, 2, 0), Ok(-6 * 3600));
    }

    #[test]
    fn to_api_string_half_hour_zone() {
        let dt = ParsedDateTime {
            year: 2026,
            month: 7,
            day: 15,
            time: Some((9, 0)),
        };
        assert_eq!(
            dt.to_api_string(Some("Asia/Kolkata")).unwrap(),
            "2026-07-15T09:00:00.000+0530"
        );
    }

    #[test]
    fn unknown_zone_is_an_error() {
        let dt = ParsedDateTime {
            year: 2026,
            month: 6,
            day: 15,
            time: Some((12, 0)),
        };
        for tz in ["Mars/Olympus", "/etc/hosts", "../etc/passwd", ""] {
            assert_eq!(
                dt.to_api_string(Some(tz)),
                Err(format!("Unknown timezone '{tz}'"))
            );
        }
    }

    #[test]
    fn impossible_date_is_an_error() {
        assert!(utc_offset_in(&TimeZone::UTC, 2026, 2, 30, 12, 0).is_err());
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
