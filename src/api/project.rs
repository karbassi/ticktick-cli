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
        let status = if p.closed { "\x1b[90m(closed)\x1b[0m " } else { "" };
        println!("  {} {}{}", "\x1b[36m*\x1b[0m", status, p.name);
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
