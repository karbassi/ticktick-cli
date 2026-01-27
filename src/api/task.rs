use crate::config;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
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

pub fn list(args: &[String]) -> Result<(), String> {
    let project_id = args.first();

    let token = config::get_access_token()?;

    if let Some(pid) = project_id {
        let resp = super::get(&format!("/project/{pid}/data"), &token)?;
        let data: ProjectData = resp
            .into_json()
            .map_err(|e| format!("failed to parse project data: {e}"))?;

        print_tasks(&data.tasks);
    } else {
        // List all tasks from all projects
        let projects = super::project::get_all()?;
        let mut all_tasks = Vec::new();

        for project in &projects {
            if let Ok(resp) = super::get(&format!("/project/{}/data", project.id), &token) {
                if let Ok(data) = resp.into_json::<ProjectData>() {
                    all_tasks.extend(data.tasks);
                }
            }
        }

        print_tasks(&all_tasks);
    }

    Ok(())
}

fn print_tasks(tasks: &[Task]) {
    if tasks.is_empty() {
        println!("\x1b[90mNo tasks found\x1b[0m");
        return;
    }

    println!("\x1b[1mTasks:\x1b[0m\n");
    for t in tasks {
        let priority_color = match t.priority {
            5 => "\x1b[31m", // high - red
            3 => "\x1b[33m", // medium - yellow
            1 => "\x1b[34m", // low - blue
            _ => "\x1b[90m", // none - gray
        };

        let checkbox = if t.status == 2 {
            "\x1b[32m[x]\x1b[0m"
        } else {
            "[ ]"
        };

        println!("  {} {}{}\x1b[0m", checkbox, priority_color, t.title);

        if let Some(due) = &t.due_date {
            println!("    \x1b[90mdue: {}\x1b[0m", due);
        }
    }

    println!("\n\x1b[90mTotal: {} tasks\x1b[0m", tasks.len());
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectData {
    #[serde(default)]
    tasks: Vec<Task>,
}
