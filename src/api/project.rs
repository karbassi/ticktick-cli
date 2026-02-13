use crate::config;
use serde::{Deserialize, Serialize};

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

/// Resolve a project name or ID to an actual project ID.
/// If the input looks like a hex ID (24+ chars), use it directly.
/// Otherwise, search by name (case-insensitive).
pub fn resolve_id(name_or_id: &str) -> Result<String, String> {
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
                .map(|p| (p.name.as_str(), strsim::levenshtein(&search, &p.name.to_lowercase())))
                .min_by_key(|(_, d)| *d);

            if let Some((name, dist)) = closest {
                if dist <= 3 {
                    msg.push_str(&format!("\n\n  Did you mean '{name}'?"));
                }
            }

            msg.push_str("\n\n  hint: Run 'ticktick-cli projects' to see available projects");
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
