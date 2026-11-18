use crate::config;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone)]
pub struct ProjectFields {
    pub name: Option<String>,
    pub color: Option<String>,
    pub view_mode: Option<String>,
    pub kind: Option<String>,
    /// Some(Some(id)) = set group, Some(None) = clear group via "NONE", None = skip
    pub group_id: Option<Option<String>>,
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
        match &self.group_id {
            Some(Some(gid)) => {
                body["groupId"] = serde_json::Value::String(gid.clone());
            }
            Some(None) => {
                body["groupId"] = serde_json::Value::String("NONE".to_string());
            }
            None => {}
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

pub fn get_all() -> Result<Vec<Project>, String> {
    let token = config::get_access_token()?;
    let resp = super::get("/project", &token)?;

    resp.into_json()
        .map_err(|e| format!("failed to parse projects: {e}"))
}

/// The cached inbox project ID, else one found by creating a task with no
/// project and deleting it again. Writes, so read-only paths use the cache only.
pub fn get_inbox_id() -> Result<String, String> {
    get_inbox_id_with(super::cli_error)
}

/// [`get_inbox_id`], with the caller's wording of request errors.
pub(crate) fn get_inbox_id_with(
    describe: impl Fn(Box<ureq::Error>) -> String,
) -> Result<String, String> {
    // Concurrent callers in one process share a probe.
    static PROBING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    if let Some(id) = config::get_inbox_project_id() {
        return Ok(id);
    }
    let _probing = config::lock(&PROBING);
    if let Some(id) = config::get_inbox_project_id() {
        return Ok(id);
    }

    let token = config::get_access_token()?;
    let body = serde_json::json!({ "title": "__inbox_probe__" });
    let resp = super::request("POST", "/task", &token, Some(&body)).map_err(&describe)?;
    let task: crate::api::task::Task = resp
        .into_json()
        .map_err(|e| format!("failed to parse probe task: {e}"))?;

    let inbox_id = task
        .project_id
        .ok_or_else(|| "probe task has no projectId".to_string())?;

    // Before the delete, so a retry after a failed delete doesn't probe again.
    let _ = config::save_inbox_project_id(&inbox_id);

    let cleanup = format!("/project/{inbox_id}/task/{}", task.id);
    super::request("DELETE", &cleanup, &token, None).map_err(|e| {
        format!(
            "inbox probe task '{}' could not be deleted from {inbox_id}: {}\n\n  \
             hint: Delete the '__inbox_probe__' task in TickTick",
            task.id,
            describe(e)
        )
    })?;
    Ok(inbox_id)
}

/// Like [`resolve_id`], but "inbox" without a cached ID stays "inbox", so
/// nothing is written.
pub fn resolve_id_without_probe(name_or_id: &str) -> Result<String, String> {
    if name_or_id.eq_ignore_ascii_case("inbox") {
        return Ok(config::get_inbox_project_id().unwrap_or_else(|| "inbox".to_string()));
    }
    resolve_id(name_or_id)
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

pub fn get_by_id(project_id: &str) -> Result<Project, String> {
    // The v1 API has no project record for the inbox (404).
    if project_id.starts_with("inbox") {
        return Ok(Project {
            id: project_id.to_string(),
            name: "Inbox".to_string(),
            color: None,
            sort_order: 0,
            closed: false,
            group_id: None,
            view_mode: Some("list".to_string()),
            kind: Some("TASK".to_string()),
        });
    }
    let token = config::get_access_token()?;
    let resp = super::get(&format!("/project/{project_id}"), &token)?;

    resp.into_json()
        .map_err(|e| format!("failed to parse project: {e}"))
}

/// List a project's members (`GET /project/{id}/members`).
pub fn members(project_id: &str) -> Result<Vec<ProjectMember>, String> {
    let token = config::get_access_token()?;
    let resp = super::get(&format!("/project/{project_id}/members"), &token)?;
    resp.into_json()
        .map_err(|e| format!("failed to parse project members: {e}"))
}

/// A member of a project, as returned by `GET /project/{id}/members`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMember {
    pub username: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default, rename = "self", skip_serializing_if = "Option::is_none")]
    pub is_self: Option<bool>,
}

pub fn create(fields: &ProjectFields) -> Result<Project, String> {
    let name = fields
        .name
        .as_ref()
        .ok_or_else(|| "name is required to create a project".to_string())?;
    let token = config::get_access_token()?;

    let mut body = serde_json::json!({ "name": name });
    fields.apply_to(&mut body);

    let resp = super::post("/project", &token, &body)?;

    resp.into_json()
        .map_err(|e| format!("failed to parse project: {e}"))
}

pub fn update(project_id: &str, fields: &ProjectFields) -> Result<Project, String> {
    let token = config::get_access_token()?;

    let mut body = serde_json::json!({ "id": project_id });
    fields.apply_to(&mut body);

    let resp = super::post(&format!("/project/{project_id}"), &token, &body)?;

    resp.into_json()
        .map_err(|e| format!("failed to parse project: {e}"))
}

/// Returns `false` when the user declines the confirmation prompt.
pub fn delete(project_id: &str, force: bool) -> Result<bool, String> {
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
            return Ok(false);
        }
    }

    super::delete(&format!("/project/{project_id}"), &token)?;
    Ok(true)
}
