//! Habit tools (Python `tools/habits.py`): list_habits, add_habit, edit_habit,
//! delete_habits, checkin_habit, habit_log, archive_habits,
//! manage_habit_sections.
//! Stub: fill in the tools and remove them from `PENDING_TOOLS`.

use rmcp::tool_router;

use crate::mcp::TickTickServer;

#[tool_router(router = habits_router, vis = "pub(crate)", allow_empty)]
impl TickTickServer {}
