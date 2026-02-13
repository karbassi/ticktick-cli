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

pub fn create(title: &str, project_id: Option<&str>, dry_run: bool) -> Result<(), String> {
    let mut body = serde_json::json!({
        "title": title
    });

    if let Some(pid) = project_id {
        body["projectId"] = serde_json::Value::String(pid.to_string());
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
