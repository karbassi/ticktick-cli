use crate::config;
use serde::{Deserialize, Serialize};

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
}

pub fn list_by_project(project_id: Option<&str>) -> Result<(), String> {
    let token = config::get_access_token()?;

    let tasks = if let Some(pid) = project_id {
        let resp = super::get(&format!("/project/{pid}/data"), &token)?;
        let data: ProjectData = resp
            .into_json()
            .map_err(|e| format!("failed to parse project data: {e}"))?;
        data.tasks
    } else {
        let projects = super::project::get_all()?;
        let mut all_tasks = Vec::new();
        for project in &projects {
            if let Ok(resp) = super::get(&format!("/project/{}/data", project.id), &token)
                && let Ok(data) = resp.into_json::<ProjectData>()
            {
                all_tasks.extend(data.tasks);
            }
        }
        all_tasks
    };

    crate::output::success(&tasks);
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectData {
    #[serde(default)]
    tasks: Vec<Task>,
}

/// Parse a user-provided due date into TickTick's expected RFC 3339 format.
///
/// Accepts:
///   - "today"      → today's date at midnight UTC
///   - "tomorrow"   → tomorrow's date at midnight UTC
///   - "YYYY-MM-DD" → that date at midnight UTC
pub fn parse_due_date(input: &str) -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let epoch_days = || -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            / 86400
    };

    let days = match input.trim().to_lowercase().as_str() {
        "today" => epoch_days(),
        "tomorrow" => epoch_days() + 1,
        _ => {
            // Expect YYYY-MM-DD
            let parts: Vec<&str> = input.split('-').collect();
            if parts.len() != 3 {
                return Err(format!("invalid due date '{input}': expected YYYY-MM-DD, 'today', or 'tomorrow'"));
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

            return Ok(format!("{year:04}-{month:02}-{day:02}T00:00:00.000+0000"));
        }
    };

    // Convert epoch days back to a date for today/tomorrow
    let total_days = days as i64;
    // Inverse of the days-since-epoch calculation
    let y = (10000 * total_days + 14780) / 3652425;
    let doy = total_days - (365 * y + y / 4 - y / 100 + y / 400);
    let (y, doy) = if doy < 0 {
        let y = y - 1;
        (y, total_days - (365 * y + y / 4 - y / 100 + y / 400))
    } else {
        (y, doy)
    };
    let mi = (100 * doy + 52) / 3060;
    let month = if mi < 10 { mi + 3 } else { mi - 9 };
    let year = y + (if month <= 2 { 1 } else { 0 });
    let day = doy - (mi * 306 + 5) / 10 + 1;

    Ok(format!("{year:04}-{month:02}-{day:02}T00:00:00.000+0000"))
}

pub fn create(title: &str, project_id: Option<&str>, due_date: Option<&str>, dry_run: bool) -> Result<(), String> {
    let mut body = serde_json::json!({
        "title": title
    });

    if let Some(pid) = project_id {
        body["projectId"] = serde_json::Value::String(pid.to_string());
    }

    if let Some(due) = due_date {
        body["dueDate"] = serde_json::Value::String(due.to_string());
    }

    if dry_run {
        body["dryRun"] = serde_json::Value::Bool(true);
        crate::output::success(&body);
        return Ok(());
    }

    let token = config::get_access_token()?;

    let resp = super::post("/task", &token, &body)?;

    let task: Task = resp
        .into_json()
        .map_err(|e| format!("failed to parse task: {e}"))?;

    crate::output::success(&task);
    Ok(())
}

pub enum DueDate {
    Set(String),
    Clear,
}

pub fn update(
    project_id: &str,
    task_id: &str,
    title: Option<&str>,
    due_date: Option<DueDate>,
) -> Result<(), String> {
    let token = config::get_access_token()?;

    let mut body = serde_json::json!({
        "taskId": task_id,
        "projectId": project_id,
    });

    if let Some(t) = title {
        body["title"] = serde_json::Value::String(t.to_string());
    }

    match due_date {
        Some(DueDate::Set(d)) => {
            body["dueDate"] = serde_json::Value::String(d);
        }
        Some(DueDate::Clear) => {
            body["dueDate"] = serde_json::Value::Null;
        }
        None => {}
    }

    let resp = super::post(&format!("/task/{task_id}"), &token, &body)?;

    let task: Task = resp
        .into_json()
        .map_err(|e| format!("failed to parse task: {e}"))?;

    crate::output::success(&task);
    Ok(())
}

pub fn complete(project_id: &str, task_id: &str) -> Result<(), String> {
    let token = config::get_access_token()?;

    super::post_empty(
        &format!("/project/{project_id}/task/{task_id}/complete"),
        &token,
    )?;

    crate::output::success(&serde_json::json!({"status": "ok"}));
    Ok(())
}

pub fn delete(project_id: &str, task_id: &str, force: bool) -> Result<(), String> {
    use std::io::IsTerminal;

    let token = config::get_access_token()?;

    if !force {
        let is_ci = std::env::var("CI").ok().as_deref() == Some("true");

        if is_ci || !std::io::stdin().is_terminal() {
            return Err(
                "refusing to delete without confirmation in non-interactive mode\n\n  hint: Use --force to skip confirmation"
                    .to_string(),
            );
        }

        eprint!("Are you sure you want to delete this task? [y/N] ");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("failed to read input: {e}"))?;
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }

    super::delete(&format!("/project/{project_id}/task/{task_id}"), &token)?;

    crate::output::success(&serde_json::json!({"status": "ok"}));
    Ok(())
}
