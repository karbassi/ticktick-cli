//! Python `tools/habits.py`.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::habit::HabitFields;
use crate::mcp::TickTickServer;
use crate::mcp::util::{self, ToolResult, nonempty, resolve_name_with_etag, run};

fn default_habit_type() -> String {
    "Boolean".to_string()
}

fn default_today() -> String {
    "today".to_string()
}

fn default_value() -> f64 {
    1.0
}

fn habits() -> Result<Vec<Value>, String> {
    Ok(util::as_list(util::v2_get("/habits")?))
}

fn resolve_habit(name_or_id: &str) -> Result<(String, String), String> {
    resolve_name_with_etag(name_or_id, &habits()?, "habit")
}

fn resolve_habits(names: &[String], with_etag: bool) -> Result<Vec<(String, String)>, String> {
    util::resolve_all(names, "habit", with_etag, habits)
}

fn resolve_habit_ids(names: &[String]) -> Result<Vec<String>, String> {
    Ok(resolve_habits(names, false)?
        .into_iter()
        .map(|(id, _)| id)
        .collect())
}

fn section_list() -> Result<Vec<Value>, String> {
    Ok(util::as_list(util::v1_get("/habit/sections")?))
}

/// Python `_resolve_section`; like Python, always fetches the sections first.
fn resolve_section(name_or_id: &str) -> Result<String, String> {
    resolve_section_in(name_or_id, &section_list()?)
}

fn resolve_section_in(name_or_id: &str, sections: &[Value]) -> Result<String, String> {
    util::resolve_name_with(
        name_or_id,
        sections,
        |query, _| format!("No habit section found matching '{query}'"),
        |query, names| format!("Multiple sections match '{query}': {}", names.join(", ")),
    )
}

/// An empty section means none, like Python's falsy check.
fn resolve_optional_section(section: Option<String>) -> Result<Option<String>, String> {
    nonempty(section).map(|s| resolve_section(&s)).transpose()
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddHabitParams {
    /// Habit name (required).
    pub name: String,
    /// "Boolean" for yes/no, "Numeric" for tracked values. Default: "Boolean".
    #[serde(default = "default_habit_type")]
    pub habit_type: String,
    /// Daily goal value (e.g. 8 for "8 glasses of water"). Default: 1 for Boolean.
    #[serde(default)]
    pub goal: Option<f64>,
    /// Unit label for numeric habits (e.g. "glasses", "km", "pages").
    #[serde(default)]
    pub unit: Option<String>,
    /// Habit section name or ID to group this habit under.
    #[serde(default)]
    pub section: Option<String>,
    /// Repeat rule string (e.g. for specific days of the week).
    #[serde(default)]
    pub repeat_rule: Option<String>,
    /// Hex color code (e.g. "#FF0000").
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditHabitParams {
    /// Habit name or ID. Supports fuzzy matching.
    pub habit: String,
    /// New habit name.
    #[serde(default)]
    pub name: Option<String>,
    /// New daily goal value.
    #[serde(default)]
    pub goal: Option<f64>,
    /// New unit label.
    #[serde(default)]
    pub unit: Option<String>,
    /// Move to this section (name or ID).
    #[serde(default)]
    pub section: Option<String>,
    /// New repeat rule string.
    #[serde(default)]
    pub repeat_rule: Option<String>,
    /// New hex color code.
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteHabitsParams {
    /// List of habit names or IDs to delete. Supports fuzzy matching.
    pub habits: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckinHabitParams {
    /// Habit name or ID. Supports fuzzy matching.
    pub habit: String,
    /// Check-in date: "today" (default), "yesterday", or "YYYY-MM-DD".
    #[serde(default = "default_today")]
    pub date: String,
    /// Check-in value. Default: 1.0 (done for Boolean habits).
    #[serde(default = "default_value")]
    pub value: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HabitLogParams {
    /// List of habit names or IDs to query. Supports fuzzy matching.
    pub habits: Vec<String>,
    /// Return check-ins after this date: "today", "yesterday", "YYYY-MM-DD".
    #[serde(default = "default_today")]
    pub after: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveHabitsParams {
    /// List of habit names or IDs to archive. Supports fuzzy matching.
    pub habits: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManageHabitSectionsParams {
    /// Operation to perform: "list", "add", "delete", "rename".
    pub action: String,
    /// Section name (required for "add").
    #[serde(default)]
    pub name: Option<String>,
    /// Section name or ID (required for "rename").
    #[serde(default)]
    pub section: Option<String>,
    /// New name (required for "rename").
    #[serde(default)]
    pub new_name: Option<String>,
    /// List of section names or IDs (required for "delete").
    #[serde(default)]
    pub sections: Option<Vec<String>>,
}

#[tool_router(router = habits_router, vis = "pub(crate)")]
impl TickTickServer {
    #[tool(
        description = "List all habits.\n\nReturns all habits with their IDs, names, types, goals, streaks, and sections. Requires v2 session token.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_habits(&self) -> ToolResult {
        run(|| util::v2_get("/habits")).await
    }

    #[tool(
        description = "Create a new habit.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn add_habit(&self, Parameters(p): Parameters<AddHabitParams>) -> ToolResult {
        run(move || {
            let mut habit = json!({});
            HabitFields {
                name: Some(p.name),
                habit_type: Some(p.habit_type),
                goal: p.goal,
                unit: p.unit,
                section_id: resolve_optional_section(p.section)?,
                repeat_rule: p.repeat_rule,
                color: p.color,
                status: None,
            }
            .apply_to(&mut habit);
            util::v2_post("/habits/batch", &json!({ "add": [habit] }))
        })
        .await
    }

    #[tool(
        description = "Update an existing habit's properties.\n\nOnly provided fields are changed.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn edit_habit(&self, Parameters(p): Parameters<EditHabitParams>) -> ToolResult {
        run(move || {
            let (id, etag) = resolve_habit(&p.habit)?;
            let mut update = json!({ "id": id, "etag": etag });
            HabitFields {
                name: p.name,
                habit_type: None,
                goal: p.goal,
                unit: p.unit,
                section_id: resolve_optional_section(p.section)?,
                repeat_rule: p.repeat_rule,
                color: p.color,
                status: None,
            }
            .apply_to(&mut update);
            util::v2_post("/habits/batch", &json!({ "update": [update] }))
        })
        .await
    }

    #[tool(
        description = "Delete one or more habits.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn delete_habits(&self, Parameters(p): Parameters<DeleteHabitsParams>) -> ToolResult {
        run(move || {
            let ids = resolve_habit_ids(&p.habits)?;
            util::v2_post("/habits/batch", &json!({ "delete": ids }))?;
            Ok(format!("Deleted {} habit(s)", ids.len()))
        })
        .await
    }

    #[tool(
        description = "Record a habit check-in for a given date.\n\nFor Boolean habits, value=1 means done. For Numeric habits, value is the tracked amount (e.g. 3 for \"3 glasses of water\").",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn checkin_habit(&self, Parameters(p): Parameters<CheckinHabitParams>) -> ToolResult {
        run(move || {
            let (id, _) = resolve_habit(&p.habit)?;
            let stamp = crate::api::habit::date_to_stamp_in(&p.date, crate::mcp::util::local_zone)?;
            util::v2_post(
                "/habitCheckins/batch",
                &json!({
                    "add": [{
                        "habitId": id,
                        "checkinStamp": stamp,
                        "value": p.value,
                        "status": 0,
                    }]
                }),
            )
        })
        .await
    }

    #[tool(
        description = "Query check-in history for one or more habits.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn habit_log(&self, Parameters(p): Parameters<HabitLogParams>) -> ToolResult {
        run(move || {
            let ids = resolve_habit_ids(&p.habits)?;
            let stamp =
                crate::api::habit::date_to_stamp_in(&p.after, crate::mcp::util::local_zone)?;
            util::v2_post(
                "/habitCheckins/query",
                &json!({ "habitIds": ids, "afterStamp": stamp }),
            )
        })
        .await
    }

    #[tool(
        description = "Archive one or more habits (set status to inactive).\n\nArchived habits are hidden from the main view but retain their data.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn archive_habits(&self, Parameters(p): Parameters<ArchiveHabitsParams>) -> ToolResult {
        run(move || {
            let updates: Vec<Value> = resolve_habits(&p.habits, true)?
                .into_iter()
                .map(|(id, etag)| json!({ "id": id, "etag": etag, "status": 1 }))
                .collect();
            util::v2_post("/habits/batch", &json!({ "update": updates }))
        })
        .await
    }

    #[tool(
        description = "Manage habit sections (groups for organizing habits).\n\nConsolidates list/add/delete/rename operations into one tool.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn manage_habit_sections(
        &self,
        Parameters(p): Parameters<ManageHabitSectionsParams>,
    ) -> ToolResult {
        run(move || match p.action.as_str() {
            "list" => util::v1_get("/habit/sections").map(Out::Json),
            "add" => {
                let name = nonempty(p.name)
                    .ok_or_else(|| "'name' is required for action='add'".to_string())?;
                util::v2_post(
                    "/habitSections/batch",
                    &json!({ "add": [{ "name": name }] }),
                )
                .map(Out::Json)
            }
            "delete" => {
                let sections = p
                    .sections
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "'sections' is required for action='delete'".to_string())?;
                let all = section_list()?;
                let ids = sections
                    .iter()
                    .map(|s| resolve_section_in(s, &all))
                    .collect::<Result<Vec<String>, String>>()?;
                util::v2_post("/habitSections/batch", &json!({ "delete": ids }))?;
                Ok(Out::Text(format!("Deleted {} section(s)", ids.len())))
            }
            "rename" => {
                let section = nonempty(p.section)
                    .ok_or_else(|| "'section' is required for action='rename'".to_string())?;
                let new_name = nonempty(p.new_name)
                    .ok_or_else(|| "'new_name' is required for action='rename'".to_string())?;
                let id = resolve_section(&section)?;
                util::v2_post(
                    "/habitSections/batch",
                    &json!({ "update": [{ "id": id, "name": new_name }] }),
                )
                .map(Out::Json)
            }
            other => Err(format!(
                "Invalid action '{other}'. Use: list, add, delete, rename"
            )),
        })
        .await
    }
}

enum Out {
    Json(Value),
    Text(String),
}

impl From<Out> for util::Output {
    fn from(o: Out) -> Self {
        match o {
            Out::Json(v) => v.into(),
            Out::Text(s) => s.into(),
        }
    }
}
