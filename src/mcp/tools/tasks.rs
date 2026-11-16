//! Python `tools/tasks.py`.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::task::{
    ChecklistItem, DEFAULT_ALL_DAY_REMINDER, DEFAULT_TIMED_REMINDER, DateField, TaskFields,
    TaskMove, assign_body, complete_chunks, unassign_body,
};
use crate::api::v2::TaskParent;
use crate::config;
use crate::mcp::TickTickServer;
use crate::mcp::util::{
    self, ToolResult, api_date, as_list, check_timezone, nonempty, parse_datetime, parse_duration,
    priority_value, resolve_column_id, resolve_project_id, resolve_project_ids, run,
};

fn default_status() -> String {
    "active".to_string()
}

fn default_limit() -> i64 {
    100
}

fn default_priority() -> String {
    "none".to_string()
}

fn timezone(tz: Option<String>) -> Result<Option<String>, String> {
    let tz = nonempty(tz);
    if let Some(ref t) = tz {
        check_timezone(t)?;
    }
    Ok(tz)
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListTasksParams {
    /// Project name or ID to list tasks from. Supports fuzzy matching. Use "inbox" for the default inbox. Omit to list from all projects.
    #[serde(default)]
    pub project: Option<String>,
    /// Filter by task status: "active" (default) or "completed".
    #[serde(default = "default_status")]
    pub status: String,
    /// Maximum number of completed tasks to return (only used with status="completed").
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskRefParams {
    /// The task ID.
    pub task_id: String,
    /// The project name or ID containing the task.
    pub project: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddTaskParams {
    /// Task title (required).
    pub title: String,
    /// Project name or ID. Supports fuzzy matching. Omit for inbox.
    #[serde(default)]
    pub project: Option<String>,
    /// Due date. Accepts "today", "tomorrow", "YYYY-MM-DD", "YYYY-MM-DDTHH:MM".
    #[serde(default)]
    pub due: Option<String>,
    /// Start date. Same format as due. If duration is set, defaults to due.
    #[serde(default)]
    pub start: Option<String>,
    /// Duration like "1h", "30m", "1h30m". Requires a start or due date with a time.
    #[serde(default)]
    pub duration: Option<String>,
    /// Priority level: "none" (default), "low", "medium", "high".
    #[serde(default = "default_priority")]
    pub priority: String,
    /// List of tag names to apply.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Markdown content/notes for the task.
    #[serde(default)]
    pub content: Option<String>,
    /// Plain text description.
    #[serde(default)]
    pub desc: Option<String>,
    /// List of checklist item titles.
    #[serde(default)]
    pub items: Option<Vec<String>>,
    /// Whether this is an all-day task. Auto-detected from date format.
    #[serde(default)]
    pub all_day: Option<bool>,
    /// IANA timezone name (e.g. "America/Chicago"). Defaults to the TickTick account timezone if known, else the system timezone.
    #[serde(default)]
    pub timezone: Option<String>,
    /// Kanban column name or ID to put the task in. Requires project.
    #[serde(default)]
    pub column: Option<String>,
    /// Reminder triggers, e.g. ["TRIGGER:PT0S"] (at due time) or ["TRIGGER:-PT15M"] (15 min before). Omit to get the default when the task has a date: at the due time for timed tasks, 9am for all-day tasks. Pass [] for no reminder.
    #[serde(default)]
    pub reminders: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditTaskParams {
    /// The task ID to edit.
    pub task_id: String,
    /// The project name or ID containing the task.
    pub project: String,
    /// New task title.
    #[serde(default)]
    pub title: Option<String>,
    /// New due date ("today", "tomorrow", "YYYY-MM-DD", "YYYY-MM-DDTHH:MM").
    #[serde(default)]
    pub due: Option<String>,
    /// New start date.
    #[serde(default)]
    pub start: Option<String>,
    /// New priority: "none", "low", "medium", "high".
    #[serde(default)]
    pub priority: Option<String>,
    /// Replace all tags with this list.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// New markdown content.
    #[serde(default)]
    pub content: Option<String>,
    /// New plain text description.
    #[serde(default)]
    pub desc: Option<String>,
    /// Set to true to remove the due date.
    #[serde(default)]
    pub clear_due: bool,
    /// Set to true to remove the start date.
    #[serde(default)]
    pub clear_start: bool,
    /// IANA timezone for date interpretation.
    #[serde(default)]
    pub timezone: Option<String>,
    /// Move the task to this kanban column (name or ID) within its project.
    #[serde(default)]
    pub column: Option<String>,
    /// Replace all reminders with these triggers, e.g. ["TRIGGER:PT0S"]. Pass [] to remove all reminders. Omit to leave reminders unchanged.
    #[serde(default)]
    pub reminders: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompleteTaskParams {
    /// The project name or ID containing the task(s).
    pub project: String,
    /// A single task ID to complete.
    #[serde(default)]
    pub task_id: Option<String>,
    /// Several task IDs to complete (all in `project`).
    #[serde(default)]
    pub task_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveTaskParams {
    /// The task ID to move.
    pub task_id: String,
    /// Source project name or ID.
    pub from_project: String,
    /// Destination project name or ID.
    pub to_project: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AssignTaskParams {
    /// The task ID to assign.
    pub task_id: String,
    /// The project name or ID containing the task.
    pub project: String,
    /// Member username (from list_project_members).
    pub assignee: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetSubtaskParams {
    /// The task ID to make a subtask.
    pub task_id: String,
    /// The parent task ID.
    pub parent_id: String,
    /// The project name or ID containing both tasks.
    pub project: String,
}

fn tasks_of(data: Value) -> Vec<Value> {
    match data {
        Value::Object(mut obj) => as_list(obj.remove("tasks").unwrap_or(Value::Null)),
        _ => Vec::new(),
    }
}

fn list_tasks(p: ListTasksParams) -> Result<Value, String> {
    let project = nonempty(p.project);
    if p.status == "completed" {
        return match project {
            Some(project) => {
                let pid = resolve_project_id(&project)?;
                util::v2_get(&format!("/project/{pid}/completed"))
            }
            None => util::v2_get(&format!("/project/all/completedInAll/?limit={}", p.limit)),
        };
    }
    if let Some(project) = project {
        let pid = resolve_project_id(&project)?;
        return Ok(Value::Array(tasks_of(util::v1_get(&format!(
            "/project/{pid}/data"
        ))?)));
    }
    // /project does not list the inbox.
    let mut ids: Vec<String> = as_list(util::v1_get("/project")?)
        .iter()
        .filter_map(|p| p.get("id").and_then(Value::as_str).map(String::from))
        .collect();
    if let Ok(inbox) = util::inbox_id() {
        ids.push(inbox);
    }
    Ok(Value::Array(project_tasks(&ids)))
}

const MAX_PARALLEL_FETCHES: usize = 8;

/// Tasks of each project, in `ids` order. A project that fails to load is
/// skipped, like Python.
fn project_tasks(ids: &[String]) -> Vec<Value> {
    let mut per_project = vec![Vec::new(); ids.len()];
    for (ids, slots) in ids
        .chunks(MAX_PARALLEL_FETCHES)
        .zip(per_project.chunks_mut(MAX_PARALLEL_FETCHES))
    {
        std::thread::scope(|s| {
            for (id, slot) in ids.iter().zip(slots) {
                s.spawn(move || {
                    if let Ok(data) = util::v1_get(&format!("/project/{id}/data")) {
                        *slot = tasks_of(data);
                    }
                });
            }
        });
    }
    per_project.concat()
}

fn add_task(p: AddTaskParams) -> Result<Value, String> {
    let mut fields = TaskFields {
        title: Some(p.title),
        ..Default::default()
    };
    let project = nonempty(p.project);
    if let Some(ref project) = project {
        fields.project_id = Some(resolve_project_id(project)?);
    }
    if let Some(column) = nonempty(p.column) {
        let Some(ref pid) = fields.project_id else {
            return Err("column requires project".to_string());
        };
        fields.column_id = Some(resolve_column_id(pid, &column)?);
    }

    let priority = priority_value(&p.priority)?;
    if priority != 0 {
        fields.priority = Some(priority);
    }
    if let Some(tags) = p.tags.filter(|t| !t.is_empty()) {
        fields.tags = Some(tags);
    }
    if p.content.is_some() {
        fields.content = Some(p.content);
    }
    if p.desc.is_some() {
        fields.desc = Some(p.desc);
    }
    if let Some(items) = p.items.filter(|i| !i.is_empty()) {
        fields.items = Some(
            items
                .into_iter()
                .map(|title| ChecklistItem {
                    title,
                    status: 0,
                    ..Default::default()
                })
                .collect(),
        );
    }

    let tz = timezone(p.timezone)?;
    let due = nonempty(p.due);
    let start = nonempty(p.start);
    let parsed_due = due.as_deref().map(parse_datetime).transpose()?;
    let parsed_start = start.as_deref().map(parse_datetime).transpose()?;

    if let Some(ref d) = parsed_due {
        fields.due_date = Some(DateField::Set(api_date(d, tz.as_deref())?));
    }
    if let Some(ref s) = parsed_start {
        fields.start_date = Some(DateField::Set(api_date(s, tz.as_deref())?));
    }
    if let Some(date) = parsed_due.as_ref().or(parsed_start.as_ref()) {
        fields.is_all_day = Some(p.all_day.unwrap_or(date.is_all_day()));
    }

    if let Some(duration) = nonempty(p.duration) {
        let dur = parse_duration(&duration)?;
        let Some(base) = parsed_start.as_ref().or(parsed_due.as_ref()) else {
            return Err("Duration requires a start or due date with a time component".to_string());
        };
        if base.is_all_day() {
            return Err(
                "Duration requires a date with a time component (use YYYY-MM-DDTHH:MM)".to_string(),
            );
        }
        let end = base.add_duration(&dur)?;
        match (&parsed_start, &parsed_due) {
            (Some(_), None) => {
                fields.due_date = Some(DateField::Set(api_date(&end, tz.as_deref())?));
            }
            (None, Some(d)) => {
                fields.start_date = Some(DateField::Set(api_date(d, tz.as_deref())?));
                fields.due_date = Some(DateField::Set(api_date(&end, tz.as_deref())?));
            }
            // Both given: Python leaves them as they are.
            _ => {}
        }
    }

    if let Some(ref tz) = tz {
        fields.time_zone = Some(tz.clone());
    }

    fields.reminders = match p.reminders {
        Some(r) => Some(r),
        None if due.is_some() || start.is_some() => Some(vec![
            if fields.is_all_day == Some(true) {
                DEFAULT_ALL_DAY_REMINDER
            } else {
                DEFAULT_TIMED_REMINDER
            }
            .to_string(),
        ]),
        None => None,
    };

    let mut body = json!({});
    fields.apply_to(&mut body);
    util::v1_post("/task", &body)
}

fn edit_task(p: EditTaskParams) -> Result<Value, String> {
    let pid = resolve_project_id(&p.project)?;
    let mut fields = TaskFields::default();
    if let Some(column) = nonempty(p.column) {
        fields.column_id = Some(resolve_column_id(&pid, &column)?);
    }
    fields.title = p.title;
    fields.tags = p.tags;
    if p.content.is_some() {
        fields.content = Some(p.content);
    }
    if p.desc.is_some() {
        fields.desc = Some(p.desc);
    }
    if let Some(priority) = p.priority {
        fields.priority = Some(priority_value(&priority)?);
    }

    let tz = timezone(p.timezone)?;
    if p.clear_due {
        fields.due_date = Some(DateField::Clear);
    } else if let Some(due) = nonempty(p.due) {
        let parsed = parse_datetime(&due)?;
        fields.due_date = Some(DateField::Set(api_date(&parsed, tz.as_deref())?));
        fields.is_all_day = Some(parsed.is_all_day());
    }
    if p.clear_start {
        fields.start_date = Some(DateField::Clear);
    } else if let Some(start) = nonempty(p.start) {
        let parsed = parse_datetime(&start)?;
        fields.start_date = Some(DateField::Set(api_date(&parsed, tz.as_deref())?));
    }
    if let Some(tz) = tz {
        fields.time_zone = Some(tz);
    }
    fields.reminders = p.reminders;

    let mut body = json!({ "taskId": p.task_id, "projectId": pid });
    fields.apply_to(&mut body);
    util::v1_post(&format!("/task/{}", p.task_id), &body)
}

fn complete_task(p: CompleteTaskParams) -> Result<String, String> {
    let mut ids: Vec<String> = Vec::new();
    for id in nonempty(p.task_id)
        .into_iter()
        .chain(p.task_ids.unwrap_or_default())
    {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.is_empty() {
        return Err("Provide task_id or task_ids".to_string());
    }
    let pid = resolve_project_id(&p.project)?;
    if ids.len() == 1 {
        util::v1_post_empty(&format!("/project/{pid}/task/{}/complete", ids[0]))?;
        return Ok(format!("Task {} completed", ids[0]));
    }
    let chunks = complete_chunks(&config::get_access_token()?, &pid, &ids);
    let mut not_completed: Vec<&String> = Vec::new();
    let mut chunk_errors = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        match &chunk.completed {
            Ok(completed) => {
                not_completed.extend(chunk.ids.iter().filter(|id| !completed.contains(*id)));
            }
            Err(e) => {
                not_completed.extend(&chunk.ids);
                chunk_errors.push(format!("chunk {} failed: {e}", i + 1));
            }
        }
    }
    let done = ids.len() - not_completed.len();
    let mut msg = format!("Completed {done} of {} tasks", ids.len());
    if !not_completed.is_empty() {
        let ids: Vec<&str> = not_completed.iter().map(|id| id.as_str()).collect();
        msg.push_str(&format!("; not completed: {}", ids.join(", ")));
    }
    for e in chunk_errors {
        msg.push_str(&format!("; {e}"));
    }
    if done == 0 { Err(msg) } else { Ok(msg) }
}

fn status_code(err: &str) -> Option<u16> {
    let rest = &err[err.find("status code ")? + "status code ".len()..];
    rest.split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

/// A username that isn't a project member gets a bare 500 (seen live by the
/// Python server); 403 is a permission refusal. Other errors pass through.
fn assign_error(e: String) -> String {
    match status_code(&e) {
        Some(code @ (403 | 500)) => format!(
            "Assign failed ({code}). The assignee must be a member of a shared project; check list_project_members."
        ),
        _ => e,
    }
}

#[tool_router(router = tasks_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "List tasks from TickTick.\n\nRetrieves tasks from a specific project or all projects. Use 'status' to filter by active/completed tasks. For completed tasks across all projects, omit the project parameter.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_tasks(&self, Parameters(p): Parameters<ListTasksParams>) -> ToolResult {
        run(move || list_tasks(p)).await
    }

    #[tool(
        description = "Get a single task by its ID.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_task(&self, Parameters(p): Parameters<TaskRefParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            util::v1_get(&format!("/project/{pid}/task/{}", p.task_id))
        })
        .await
    }

    #[tool(
        description = "Create a new task in TickTick.\n\nA dated task gets a default reminder unless `reminders` is given: at the due time for timed tasks, 9am for all-day tasks.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn add_task(&self, Parameters(p): Parameters<AddTaskParams>) -> ToolResult {
        run(move || add_task(p)).await
    }

    #[tool(
        description = "Update an existing task.\n\nOnly provided fields are changed. Use clear_due/clear_start to remove dates.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn edit_task(&self, Parameters(p): Parameters<EditTaskParams>) -> ToolResult {
        run(move || edit_task(p)).await
    }

    #[tool(
        description = "Mark one or more tasks as complete.\n\nPass `task_id` for a single task, or `task_ids` for several. Several tasks are completed with the batch endpoint in chunks of 50 instead of one request each.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn complete_task(&self, Parameters(p): Parameters<CompleteTaskParams>) -> ToolResult {
        run(move || complete_task(p)).await
    }

    #[tool(
        description = "Permanently delete a task.\n\nThis action cannot be undone.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn delete_task(&self, Parameters(p): Parameters<TaskRefParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            util::v1_delete(&format!("/project/{pid}/task/{}", p.task_id))?;
            Ok(format!("Task {} deleted", p.task_id))
        })
        .await
    }

    #[tool(
        description = "Move a task from one project to another.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn move_task(&self, Parameters(p): Parameters<MoveTaskParams>) -> ToolResult {
        run(move || {
            let ids = resolve_project_ids(&[&p.from_project, &p.to_project])?;
            let moves = [TaskMove {
                task_id: p.task_id.clone(),
                from_project_id: ids[0].clone(),
                to_project_id: ids[1].clone(),
            }];
            let body = serde_json::to_value(moves).map_err(|e| e.to_string())?;
            util::v1_post("/task/move", &body)?;
            Ok(format!("Task {} moved to {}", p.task_id, p.to_project))
        })
        .await
    }

    #[tool(
        description = "Assign a task to a member of its (shared) project.\n\nUse list_project_members to find valid usernames. Only works in projects shared with other people.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn assign_task(&self, Parameters(p): Parameters<AssignTaskParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            util::v1_post("/task/assign", &assign_body(&pid, &p.task_id, &p.assignee))
                .map_err(assign_error)
        })
        .await
    }

    #[tool(
        description = "Remove the assignee from a task.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn unassign_task(&self, Parameters(p): Parameters<TaskRefParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            util::v1_post("/task/unassign", &unassign_body(&pid, &p.task_id))
        })
        .await
    }

    #[tool(
        description = "Make a task a subtask of another task.\n\nBoth tasks must be in the same project. Requires the v2 session token.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn set_subtask(&self, Parameters(p): Parameters<SetSubtaskParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            let parents = [TaskParent {
                task_id: p.task_id.clone(),
                parent_id: p.parent_id.clone(),
                project_id: pid,
            }];
            let body = serde_json::to_value(parents).map_err(|e| e.to_string())?;
            util::v2_post("/batch/taskParent", &body)?;
            Ok(format!(
                "Task {} is now a subtask of {}",
                p.task_id, p.parent_id
            ))
        })
        .await
    }

    #[tool(
        description = "Remove a task's parent, making it a top-level task.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn unparent_task(&self, Parameters(p): Parameters<TaskRefParams>) -> ToolResult {
        run(move || {
            let pid = resolve_project_id(&p.project)?;
            let mut body = json!({ "taskId": p.task_id, "projectId": pid });
            TaskFields {
                parent_id: Some(None),
                ..Default::default()
            }
            .apply_to(&mut body);
            util::v1_post(&format!("/task/{}", p.task_id), &body)
        })
        .await
    }

    #[tool(
        description = "List tasks in the trash.\n\nReturns tasks that have been deleted but not yet permanently removed. Requires the v2 session token.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_trash(&self) -> ToolResult {
        run(|| {
            Ok(Value::Array(tasks_of(util::v2_get(
                "/project/all/trash/page",
            )?)))
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_code_from_api_error() {
        assert_eq!(
            status_code("API request failed: http://x/task/assign: status code 500"),
            Some(500)
        );
        assert_eq!(status_code("token expired"), None);
    }

    #[test]
    fn assign_error_hints_only_on_membership_codes() {
        for code in [403, 500] {
            let e = assign_error(format!("API request failed: x: status code {code}"));
            assert!(e.starts_with(&format!("Assign failed ({code})")), "{e}");
        }
        for e in [
            "API request failed: x: status code 404",
            "API request failed: x: status code 429",
            "API request failed: timed out",
        ] {
            assert_eq!(assign_error(e.to_string()), e);
        }
    }
}
