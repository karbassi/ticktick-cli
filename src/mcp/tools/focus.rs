//! Focus/pomodoro tools (Python `tools/focus.py`): focus_status, focus_stats,
//! focus_log, focus_timeline, focus_save. `focus_log` and `focus_save` use the
//! v1 `/focus` endpoints; the rest use v2.
//!
//! The date helpers below are private copies of what Python gets from
//! `datetime` (`date_to_epoch_ms`, `fromisoformat`, `_to_iso`).

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::v2::url_encode;
use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, nonempty, run};

const MS_PER_DAY: i64 = 86_400_000;
/// v1 `/focus` rejects ranges longer than 30 days.
const V1_FOCUS_MAX_RANGE_MS: i64 = 30 * MS_PER_DAY;
/// 0 = pomodoro, 1 = stopwatch. v1 requires one per request.
const FOCUS_TYPES: [u8; 2] = [0, 1];

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FocusLogParams {
    /// Start date: "today", "yesterday", or "YYYY-MM-DD".
    pub from_date: String,
    /// End date: "today", "yesterday", or "YYYY-MM-DD".
    pub to_date: String,
}

fn default_end() -> String {
    "now".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FocusSaveParams {
    /// Duration of the focus session in minutes. Must be >= 1.
    pub duration_minutes: i64,
    /// End time: "now" (default) or ISO datetime string (e.g. "2026-02-22T14:30:00").
    #[serde(default = "default_end")]
    pub end: String,
    /// Optional note to attach to the session.
    #[serde(default)]
    pub note: String,
    /// Optional task ID to associate with the session.
    #[serde(default)]
    pub task_id: Option<String>,
    /// Optional project ID (required if task_id is provided).
    #[serde(default)]
    pub project_id: Option<String>,
}

#[tool_router(router = focus_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "Get the current focus/pomodoro timer status.\n\nReturns the active timer state including elapsed time, task association,\nand whether it's running or paused. Requires v2 session token.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn focus_status(&self) -> ToolResult {
        run(|| util::v2_get("/timer")).await
    }

    #[tool(
        description = "Get focus/pomodoro statistics.\n\nReturns daily and total focus time statistics including session counts\nand total minutes. Requires v2 session token.\n\nNote: \"total\" fields may exclude the current day (TickTick caches these).\nUse focus_log for accurate current-day data.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn focus_stats(&self) -> ToolResult {
        run(|| util::v2_get("/pomodoros/statistics/generalForDesktop")).await
    }

    #[tool(
        description = "Get focus session log for a date range.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn focus_log(&self, Parameters(p): Parameters<FocusLogParams>) -> ToolResult {
        run(move || {
            let start = date_to_epoch_ms(&p.from_date)?;
            // End of day.
            let end = date_to_epoch_ms(&p.to_date)? + MS_PER_DAY - 1;
            v1_focus_records(start, end)
        })
        .await
    }

    #[tool(
        description = "Get the full focus session timeline.\n\nReturns all focus sessions in chronological order.\nRequires v2 session token.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn focus_timeline(&self) -> ToolResult {
        run(|| util::v2_get("/pomodoros/timeline")).await
    }

    #[tool(
        description = "Save a completed focus/pomodoro record.\n\nRecords a focus session that has already happened. This does not start\na live timer \u{2014} it saves a historical record.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn focus_save(&self, Parameters(p): Parameters<FocusSaveParams>) -> ToolResult {
        run(move || {
            if p.duration_minutes < 1 {
                return Err("duration_minutes must be >= 1".to_string());
            }
            let end_ms = if p.end == "now" {
                now_ms()
            } else {
                parse_iso_datetime(&p.end).ok_or_else(|| {
                    format!(
                        "Invalid end time '{}'. Use 'now' or ISO format like '2026-02-22T14:30:00'.",
                        p.end
                    )
                })?
            };
            let start_ms = end_ms - p.duration_minutes * 60_000;
            let start_iso = to_iso(start_ms);
            let end_iso = to_iso(end_ms);

            let mut record = json!({
                "type": 0,
                "startTime": start_iso,
                "endTime": end_iso,
                "status": 1,
                "pauseDuration": 0,
                "note": p.note,
            });
            let task_id = nonempty(p.task_id);
            let project_id = nonempty(p.project_id);
            if task_id.is_some() || project_id.is_some() {
                let mut entry = json!({ "startTime": start_iso, "endTime": end_iso });
                if let Some(task_id) = task_id {
                    entry["taskId"] = json!(task_id);
                    record["taskId"] = json!(task_id);
                }
                if let Some(project_id) = project_id {
                    entry["projectId"] = json!(project_id);
                }
                record["tasks"] = json!([entry]);
            }
            util::v1_post("/focus", &record)
        })
        .await
    }
}

/// All focus records (both types) in `[start, end]` (epoch ms), paged to the
/// v1 30-day cap. Records spanning a chunk boundary come back in both chunks,
/// so they are deduplicated by id (last copy wins, first position kept).
/// Records without an id can't be matched, so they are always kept. Sorted by
/// start time.
fn v1_focus_records(start: i64, end: i64) -> Result<Value, String> {
    let mut records = Vec::new();
    let mut chunk_start = start;
    while chunk_start < end {
        let chunk_end = (chunk_start + V1_FOCUS_MAX_RANGE_MS).min(end);
        for focus_type in FOCUS_TYPES {
            let path = format!(
                "/focus?from={}&to={}&type={focus_type}",
                url_encode(&to_iso(chunk_start)),
                url_encode(&to_iso(chunk_end)),
            );
            records.extend(util::as_list(util::v1_get(&path)?));
        }
        chunk_start = chunk_end;
    }

    let mut unique: Vec<Value> = Vec::new();
    let mut position: HashMap<String, usize> = HashMap::new();
    let mut without_id = Vec::new();
    for r in records {
        match r.get("id").filter(|id| is_truthy(id)) {
            Some(id) => {
                let key = id.to_string();
                match position.get(&key) {
                    Some(&i) => unique[i] = r,
                    None => {
                        position.insert(key, unique.len());
                        unique.push(r);
                    }
                }
            }
            None => without_id.push(r),
        }
    }
    unique.extend(without_id);
    unique.sort_by(|a, b| start_time(a).cmp(start_time(b)));
    Ok(Value::Array(unique))
}

/// Python truthiness of a JSON value (for `if r.get("id")`).
fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn start_time(r: &Value) -> &str {
    r.get("startTime").and_then(Value::as_str).unwrap_or("")
}

// ---------------------------------------------------------------------------
// Dates
// ---------------------------------------------------------------------------

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// (year, month, day) for days since 1970-01-01.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        _ => 28,
    }
}

/// Format epoch milliseconds like Python `_to_iso`:
/// `yyyy-MM-ddTHH:mm:ss.SSS+0000` (UTC).
fn to_iso(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let rem = secs.rem_euclid(86_400);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}+0000",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        ms.rem_euclid(1000)
    )
}

/// Today's date in the system's local timezone (Python `date.today()`).
fn local_today() -> (i64, i64, i64) {
    let now = (now_ms() / 1000) as libc::time_t;
    // SAFETY: `tm` is plain data and `localtime_r` fully initializes it.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&now, &mut tm) };
    (
        i64::from(tm.tm_year) + 1900,
        i64::from(tm.tm_mon) + 1,
        i64::from(tm.tm_mday),
    )
}

/// Python `date(y, m, d)` validation, with its error messages.
fn check_date(y: i64, m: i64, d: i64) -> Result<(), String> {
    if !(1..=9999).contains(&y) {
        return Err(format!("year {y} is out of range"));
    }
    if !(1..=12).contains(&m) {
        return Err("month must be in 1..12".to_string());
    }
    if d < 1 || d > days_in_month(y, m) {
        return Err("day is out of range for month".to_string());
    }
    Ok(())
}

/// Digits-only slice to number (rejects signs, spaces and non-ASCII).
fn digits(s: &str) -> Option<i64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Python `date_to_epoch_ms`: "today", "yesterday" or "YYYY-MM-DD" to epoch
/// ms at midnight UTC.
fn date_to_epoch_ms(input: &str) -> Result<i64, String> {
    let s = input.trim().to_lowercase();
    let (y, m, d) = match s.as_str() {
        "today" => local_today(),
        "yesterday" => {
            let (y, m, d) = local_today();
            civil_from_days(days_from_civil(y, m, d) - 1)
        }
        _ => {
            let parsed = (s.len() == 10 && s.is_char_boundary(4) && s.is_char_boundary(7))
                .then(|| (&s[..4], &s[4..5], &s[5..7], &s[7..8], &s[8..]))
                .filter(|(_, d1, _, d2, _)| *d1 == "-" && *d2 == "-")
                .and_then(|(y, _, m, _, d)| Some((digits(y)?, digits(m)?, digits(d)?)));
            let Some((y, m, d)) = parsed else {
                return Err(format!("Invalid date '{input}'"));
            };
            check_date(y, m, d)?;
            (y, m, d)
        }
    };
    Ok(days_from_civil(y, m, d) * MS_PER_DAY)
}

/// A subset of Python's `datetime.fromisoformat` (3.11+) converted to epoch
/// ms: `YYYY-MM-DD`, optionally followed by a one-character separator, a time
/// (`HH`, `HH:MM`, `HH:MM:SS`, optional fraction, or the compact `HHMM[SS]`)
/// and a `Z` / `+HH[:MM[:SS]]` / `+HHMM` offset. Without an offset the value
/// is local time, like Python's `astimezone`. Sub-millisecond digits are
/// truncated.
fn parse_iso_datetime(input: &str) -> Option<i64> {
    let s = input;
    if !s.is_ascii() || s.len() < 10 {
        return None;
    }
    let b = s.as_bytes();
    if b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, mo, d) = (digits(&s[..4])?, digits(&s[5..7])?, digits(&s[8..10])?);
    check_date(y, mo, d).ok()?;

    let (mut h, mut mi, mut sec, mut frac_ms) = (0, 0, 0, 0);
    let mut offset: Option<i64> = None;
    let rest = &s[10..];
    if !rest.is_empty() {
        // One separator character, then the time.
        let time_and_tz = rest.get(1..).filter(|t| !t.is_empty())?;
        let tz_at = time_and_tz.find(['Z', 'z', '+', '-']);
        let (time, tz) = match tz_at {
            Some(i) => time_and_tz.split_at(i),
            None => (time_and_tz, ""),
        };

        let (hms, frac) = match time.find(['.', ',']) {
            Some(i) => (&time[..i], Some(&time[i + 1..])),
            None => (time, None),
        };
        let fields: Vec<&str> = if hms.contains(':') {
            hms.split(':').collect()
        } else if matches!(hms.len(), 2 | 4 | 6) {
            (0..hms.len()).step_by(2).map(|i| &hms[i..i + 2]).collect()
        } else {
            return None;
        };
        if fields.is_empty() || fields.len() > 3 || fields.iter().any(|f| f.len() != 2) {
            return None;
        }
        h = digits(fields[0])?;
        mi = fields.get(1).map_or(Some(0), |f| digits(f))?;
        sec = fields.get(2).map_or(Some(0), |f| digits(f))?;
        if let Some(frac) = frac {
            // A fraction needs a seconds field.
            if fields.len() < 3 || frac.is_empty() || digits(frac).is_none() {
                return None;
            }
            let padded = format!("{frac:0<3}");
            frac_ms = digits(&padded[..3])?;
        }
        if h > 23 || mi > 59 || sec > 59 {
            return None;
        }

        if !tz.is_empty() {
            offset = Some(parse_offset_secs(tz)?);
        }
    }

    let local_secs = days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + sec;
    let epoch_secs = match offset {
        Some(off) => local_secs - off,
        None => local_to_epoch_secs(y, mo, d, h, mi, sec),
    };
    Some(epoch_secs * 1000 + frac_ms)
}

/// `Z`, `+HH`, `+HH:MM`, `+HH:MM:SS` or `+HHMM[SS]` to seconds east of UTC.
fn parse_offset_secs(tz: &str) -> Option<i64> {
    if tz.eq_ignore_ascii_case("z") {
        return Some(0);
    }
    let sign = match tz.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let body = &tz[1..];
    let fields: Vec<&str> = if body.contains(':') {
        body.split(':').collect()
    } else if matches!(body.len(), 2 | 4 | 6) {
        (0..body.len())
            .step_by(2)
            .map(|i| &body[i..i + 2])
            .collect()
    } else {
        return None;
    };
    if fields.len() > 3 || fields.iter().any(|f| f.len() != 2) {
        return None;
    }
    let h = digits(fields[0])?;
    let m = fields.get(1).map_or(Some(0), |f| digits(f))?;
    let s = fields.get(2).map_or(Some(0), |f| digits(f))?;
    if h > 23 || m > 59 || s > 59 {
        return None;
    }
    Some(sign * (h * 3600 + m * 60 + s))
}

/// Epoch seconds for a wall-clock time in the system's local timezone.
fn local_to_epoch_secs(y: i64, mo: i64, d: i64, h: i64, mi: i64, sec: i64) -> i64 {
    // SAFETY: `tm` is plain data; `mktime` normalizes it in place.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    tm.tm_year = (y - 1900) as i32;
    tm.tm_mon = (mo - 1) as i32;
    tm.tm_mday = d as i32;
    tm.tm_hour = h as i32;
    tm.tm_min = mi as i32;
    tm.tm_sec = sec as i32;
    tm.tm_isdst = -1; // let mktime work out DST
    unsafe { libc::mktime(&mut tm) as i64 }
}
