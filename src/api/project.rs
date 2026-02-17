use crate::config;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone)]
pub struct ProjectFields {
    pub name: Option<String>,
    pub color: Option<String>,
    pub view_mode: Option<String>,
    pub kind: Option<String>,
}

impl ProjectFields {
    pub fn apply_to(&self, body: &mut serde_json::Value) {
        if let Some(ref n) = self.name {
            body["name"] = serde_json::Value::String(n.clone());
        }
        if let Some(ref c) = self.color {
            body["color"] = serde_json::Value::String(c.clone());
        }
        if let Some(ref vm) = self.view_mode {
            body["viewMode"] = serde_json::Value::String(vm.clone());
        }
        if let Some(ref k) = self.kind {
            body["kind"] = serde_json::Value::String(k.clone());
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub closed: bool,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub view_mode: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
}

pub fn list() -> Result<(), String> {
    let token = config::get_access_token()?;
    let resp = super::get("/project", &token)?;

    let projects: Vec<Project> = resp
        .into_json()
        .map_err(|e| format!("failed to parse projects: {e}"))?;

    crate::output::success(&projects);
    Ok(())
}

pub fn get_all() -> Result<Vec<Project>, String> {
    let token = config::get_access_token()?;
    let resp = super::get("/project", &token)?;

    resp.into_json()
        .map_err(|e| format!("failed to parse projects: {e}"))
}

/// Discover the inbox project ID.
///
/// 1. Returns the cached value from config if available.
/// 2. Otherwise, probes by creating a temporary task (with no projectId),
///    extracts the inbox ID from the response, deletes the task, and caches the ID.
pub fn get_inbox_id() -> Result<String, String> {
    if let Some(id) = config::get_inbox_project_id() {
        return Ok(id);
    }

    // Probe: create a temporary task with no project
    let token = config::get_access_token()?;
    let body = serde_json::json!({ "title": "__inbox_probe__" });
    let resp = super::post("/task", &token, &body)?;
    let task: crate::api::task::Task = resp
        .into_json()
        .map_err(|e| format!("failed to parse probe task: {e}"))?;

    let inbox_id = task
        .project_id
        .ok_or_else(|| "probe task has no projectId".to_string())?;

    // Clean up the probe task
    let _ = super::delete(&format!("/project/{inbox_id}/task/{}", task.id), &token);

    // Cache for future use
    let _ = config::save_inbox_project_id(&inbox_id);

    Ok(inbox_id)
}

/// Resolve a project name or ID to an actual project ID.
/// If the input looks like a hex ID (24+ chars), use it directly.
/// Otherwise, search by name (case-insensitive).
pub fn resolve_id(name_or_id: &str) -> Result<String, String> {
    // "inbox" (case-insensitive) → discover/use inbox ID
    if name_or_id.eq_ignore_ascii_case("inbox") {
        return get_inbox_id();
    }

    // Already an inbox ID (e.g. inbox123456789)
    if name_or_id.starts_with("inbox")
        && name_or_id[5..].chars().all(|c| c.is_ascii_digit())
        && name_or_id.len() > 5
    {
        return Ok(name_or_id.to_string());
    }

    // If it looks like an ID (long hex string), use it directly
    if name_or_id.len() >= 20 && name_or_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(name_or_id.to_string());
    }

    // Otherwise, search by name
    let projects = get_all()?;
    let search = name_or_id.to_lowercase();

    // Try exact match first (case-insensitive)
    if let Some(p) = projects.iter().find(|p| p.name.to_lowercase() == search) {
        return Ok(p.id.clone());
    }

    // Try contains match
    let matches: Vec<_> = projects
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&search))
        .collect();

    match matches.len() {
        0 => {
            let mut msg = format!("no project found matching '{name_or_id}'");

            let closest = projects
                .iter()
                .map(|p| {
                    (
                        p.name.as_str(),
                        strsim::levenshtein(&search, &p.name.to_lowercase()),
                    )
                })
                .min_by_key(|(_, d)| *d);

            if let Some((name, dist)) = closest
                && dist <= 3
            {
                msg.push_str(&format!("\n\n  Did you mean '{name}'?"));
            }

            msg.push_str("\n\n  hint: Run 'ticktick-cli project list' to see available projects");
            Err(msg)
        }
        1 => Ok(matches[0].id.clone()),
        _ => {
            let names: Vec<_> = matches.iter().map(|p| p.name.as_str()).collect();
            Err(format!(
                "multiple projects match '{name_or_id}': {}\n\n  hint: Use a more specific name or the full project ID",
                names.join(", ")
            ))
        }
    }
}

pub fn get_by_id(project_id: &str) -> Result<(), String> {
    let token = config::get_access_token()?;
    let resp = super::get(&format!("/project/{project_id}"), &token)?;

    let project: Project = resp
        .into_json()
        .map_err(|e| format!("failed to parse project: {e}"))?;

    crate::output::success(&project);
    Ok(())
}

pub fn create(fields: &ProjectFields) -> Result<(), String> {
    let name = fields
        .name
        .as_ref()
        .ok_or_else(|| "name is required to create a project".to_string())?;
    let token = config::get_access_token()?;

    let mut body = serde_json::json!({ "name": name });
    fields.apply_to(&mut body);

    let resp = super::post("/project", &token, &body)?;

    let project: Project = resp
        .into_json()
        .map_err(|e| format!("failed to parse project: {e}"))?;

    crate::output::success(&project);
    Ok(())
}

pub fn update(project_id: &str, fields: &ProjectFields) -> Result<(), String> {
    let token = config::get_access_token()?;

    let mut body = serde_json::json!({ "id": project_id });
    fields.apply_to(&mut body);

    let resp = super::post(&format!("/project/{project_id}"), &token, &body)?;

    let project: Project = resp
        .into_json()
        .map_err(|e| format!("failed to parse project: {e}"))?;

    crate::output::success(&project);
    Ok(())
}

pub fn delete(project_id: &str, force: bool) -> Result<(), String> {
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

        eprint!("Are you sure you want to delete this project? [y/N] ");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("failed to read input: {e}"))?;
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }

    super::delete(&format!("/project/{project_id}"), &token)?;

    crate::output::success(&serde_json::json!({"status": "ok"}));
    Ok(())
}
