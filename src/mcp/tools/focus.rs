//! Python `tools/focus.py`.

use std::collections::HashMap;

use jiff::civil::{Date, Time};
use jiff::fmt::temporal::Pieces;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ZonedDateTime};
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
                Timestamp::now().as_millisecond()
            } else {
                parse_iso_datetime(&p.end).ok_or_else(|| {
                    format!(
                        "Invalid end time '{}'. Use 'now' or ISO format like '2026-02-22T14:30:00'.",
                        p.end
                    )
                })?
            };
            let start_ms = end_ms.saturating_sub(p.duration_minutes.saturating_mul(60_000));
            let start_iso = to_iso(start_ms)?;
            let end_iso = to_iso(end_ms)?;

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

/// Records spanning a chunk boundary come back in both chunks, so dedupe by id
/// (last copy wins, first position kept). Records without an id are always kept.
fn v1_focus_records(start: i64, end: i64) -> Result<Value, String> {
    let mut records = Vec::new();
    let mut chunk_start = start;
    while chunk_start < end {
        let chunk_end = (chunk_start + V1_FOCUS_MAX_RANGE_MS).min(end);
        for focus_type in FOCUS_TYPES {
            let path = format!(
                "/focus?from={}&to={}&type={focus_type}",
                url_encode(&to_iso(chunk_start)?),
                url_encode(&to_iso(chunk_end)?),
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

/// Python truthiness, for `if r.get("id")`.
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

fn local_zone() -> TimeZone {
    TimeZone::system()
}

/// Matches Python `_to_iso`.
fn to_iso(ms: i64) -> Result<String, String> {
    let ts = Timestamp::from_millisecond(ms).map_err(|e| e.to_string())?;
    Ok(ts
        .to_zoned_datetime(TimeZone::UTC)
        .strftime("%Y-%m-%dT%H:%M:%S%.3f%z")
        .to_string())
}

/// Python `date_to_epoch_ms`; midnight UTC.
fn date_to_epoch_ms(input: &str) -> Result<i64, String> {
    let s = input.trim().to_lowercase();
    let today = || ZonedDateTime::now().date();
    let date = match s.as_str() {
        "today" => today(),
        "yesterday" => today().yesterday().map_err(|e| e.to_string())?,
        _ => parse_date(&s).ok_or_else(|| format!("Invalid date '{input}'"))??,
    };
    let midnight = TimeZone::UTC
        .to_timestamp(date.to_datetime(Time::midnight()))
        .map_err(|e| e.to_string())?;
    Ok(midnight.as_millisecond())
}

/// `YYYY-MM-DD`: None if malformed, else Python `date(y, m, d)` validation
/// with its error messages.
fn parse_date(s: &str) -> Option<Result<Date, String>> {
    let b = s.as_bytes();
    if b.len() != 10 || !s.is_ascii() || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    fn num<T: std::str::FromStr>(field: &str) -> Option<T> {
        // Unlike `str::parse` alone, rejects signs.
        field.bytes().all(|c| c.is_ascii_digit()).then_some(())?;
        field.parse().ok()
    }
    let (y, m, d): (i16, i8, i8) = (num(&s[..4])?, num(&s[5..7])?, num(&s[8..])?);
    Some(if y == 0 {
        Err(format!("year {y} is out of range"))
    } else if !(1..=12).contains(&m) {
        Err("month must be in 1..12".to_string())
    } else {
        Date::new(y, m, d).map_err(|_| "day is out of range for month".to_string())
    })
}

/// Close to Python 3.11+ `datetime.fromisoformat`. Sub-millisecond digits are
/// truncated.
fn parse_iso_datetime(input: &str) -> Option<i64> {
    let pieces = Pieces::parse(input).ok()?;
    if pieces.time_zone_annotation().is_some() {
        return None;
    }
    let dt = pieces
        .date()
        .to_datetime(pieces.time().unwrap_or(Time::midnight()));
    let ts = match pieces.to_numeric_offset() {
        Some(offset) => offset.to_timestamp(dt).ok()?,
        None => local_zone().to_timestamp(dt).ok()?,
    };
    Some(ts.as_millisecond())
}
