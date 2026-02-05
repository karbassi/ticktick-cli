use crate::config;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
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

    if projects.is_empty() {
        println!("\x1b[90mNo projects found\x1b[0m");
        return Ok(());
    }

    println!("\x1b[1mProjects:\x1b[0m\n");
    for p in &projects {
        let status = if p.closed {
            "\x1b[90m(closed)\x1b[0m "
        } else {
            ""
        };
        println!("  \x1b[36m*\x1b[0m {}{}", status, p.name);
        println!("    \x1b[90mid: {}\x1b[0m", p.id);
    }

    println!("\n\x1b[90mTotal: {} projects\x1b[0m", projects.len());
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
        0 => Err(format!("no project found matching '{name_or_id}'")),
        1 => Ok(matches[0].id.clone()),
        _ => {
            let names: Vec<_> = matches.iter().map(|p| p.name.as_str()).collect();
            Err(format!(
                "multiple projects match '{name_or_id}': {}",
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

    println!("\x1b[1mProject:\x1b[0m {}", project.name);
    println!("  \x1b[90mid:\x1b[0m {}", project.id);
    if let Some(color) = &project.color {
        println!("  \x1b[90mcolor:\x1b[0m {}", color);
    }
    if let Some(view_mode) = &project.view_mode {
        println!("  \x1b[90mview:\x1b[0m {}", view_mode);
    }
    if let Some(kind) = &project.kind {
        println!("  \x1b[90mkind:\x1b[0m {}", kind);
    }
    if project.closed {
        println!("  \x1b[90mstatus:\x1b[0m closed");
    }

    Ok(())
}
