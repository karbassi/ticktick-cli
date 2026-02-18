# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `task list --completed` — list completed tasks using v2 API
- `--limit` flag on `task list --completed` to cap the number of results (default: 50)
- `task subtask <project> <parent_id> <child_ids...>` — set tasks as subtasks of a parent task via v2 API
- `task unparent <project> <task_ids...>` — remove subtask relationships (make tasks top-level) via v1 API
- `task trash` — list tasks in the trash via v2 API
- `tag list` — list all tags via v2 API
- `tag add <names...>` — create one or more tags via v2 API
- `tag delete <names...> [--force]` — delete tags via v2 API (with confirmation prompt)
- `tag rename <old> <new>` — rename a tag via v2 API
- `tag edit <name>` — update tag properties (`--color`, `--parent`/`--clear-parent`, `--sort-order`, `--sort-type`)
- `tag merge <source> <target>` — merge a tag into another (re-tags all tasks, deletes source)
- `calendar list` — list connected calendar accounts (Google, Outlook, etc.) via v2 API
- `calendar events` — query calendar events by date range (`--from`/`--to`, default ±7 days)
- `profile` — show user profile and account status via v2 API
- `settings` — show user preference settings via v2 API
- `parentId` field on task output (when present)
- `sync` — dump full account state from v2 batch/check endpoint (pipe to `jq` for filtering)
- `folder list` — list all project folders/groups via v2 API
- `folder add <name>` — create a project folder via v2 API
- `folder delete <names...> [--force]` — delete project folders via v2 API
- `folder rename <folder> --name <new>` — rename a project folder via v2 API
- `--folder` flag on `project add` and `project edit` — assign a project to a folder; use `--folder none` to remove
- `filter list` — list all saved filters (smart views) via v2 API
- `filter add <name> --rule <json>` — create a saved filter with optional `--sort-type`
- `filter edit <filter>` — update filter properties (`--name`, `--rule`, `--sort-type`)
- `filter delete <names...> [--force]` — delete saved filters via v2 API
- `habit section list/add/delete/rename` — manage habit sections (groups) via v2 API
- `habit list` — list all habits via v2 API
- `habit add <name>` — create a habit with optional `--type`, `--goal`, `--unit`, `--section`, `--repeat`, `--color`
- `habit delete <names...> [--force]` — delete habits via v2 API
- `habit edit <habit>` — update habit properties (`--name`, `--color`, `--goal`, `--unit`, `--section`, `--repeat`)
- `habit checkin <habit>` — record a habit check-in with optional `--date` and `--value`
- `habit log <habits...>` — query habit check-in history with optional `--after` date filter
- `habit archive <habits...>` — archive habits (set status to 1)
- `focus status` — show current focus/pomodoro timer state via v2 API
- `focus stats` — show focus statistics (today/total) via v2 API
- `focus log [--from --to]` — show focus session history (default: last 30 days)
- `focus timeline` — show full focus session timeline via v2 API
- `focus start [--task ID] [--mode pomo|stopwatch] [--duration MIN]` — start a focus session
- `focus pause` / `focus resume` / `focus stop` — control the current focus session

### Changed
- `task move` now uses TickTick's internal v2 API to move tasks between projects, preserving task ID, history, subtasks, comments, and creation date (previously used delete + recreate which lost this data)
- `task move` now sends a single batch API request for multiple tasks instead of one request per task
- `task move` requires a v2 session token (set `v2_session_token` in config.json)
- v2 API authentication now uses a browser session token (`t` cookie) instead of username/password signon (TickTick added captcha to the signon endpoint)

### Removed
- `--from` and `--to` flags on `task list --completed` (the v2 API returns HTTP 500 when date params are included; use `--limit` to control results)
- Username/password signon for v2 API — replaced with browser session token

### Fixed
- `habit archive` now correctly sets status to 1 (archived) instead of 2

## [0.11.2]

### Fixed
- `task move` now correctly retrieves the moved task using the project data endpoint (the individual task endpoint returns empty for moved tasks)

### Added
- `make fix` target for auto-formatting and auto-fixing clippy lints
- `CLAUDE.md` with project conventions, architecture, and testing rules
- Integration test for task move between projects

## [0.11.1]

### Fixed
- `task move` no longer fails with a JSON parse error when moving tasks between projects (the TickTick API returns an empty response body for moves)

## [0.11.0]

### Added
- `task move` (`mv`) command to move one or more tasks between projects

## [0.10.0]

### Added
- Inbox support: use `inbox` as a project name in all task commands (`task list inbox`, `task get inbox <id>`, `task complete inbox <id>`, `task delete inbox <id>`)
- `task list` (no project filter) now includes inbox tasks when the inbox ID is known
- Automatic inbox ID detection: after the first task create/edit that lands in the inbox, the CLI stores the inbox project ID in config
- Inbox ID discovery: if the inbox ID is not cached, `task list inbox` probes by creating and deleting a temporary task

## [0.9.0]

### Fixed
- Datetime values (`--due`, `--start`) now use the local system timezone offset instead of hardcoding UTC (`+0000`)
- `today` and `tomorrow` now resolve to the local date instead of the UTC date

### Added
- Automatic account timezone detection: after the first task create/edit, the CLI stores the TickTick account timezone in config
- Timezone mismatch prompt: when the local system timezone differs from the stored account timezone, the CLI asks which to use (defaults to local in non-interactive mode)
- `--tz` now also controls the UTC offset used in datetime strings (previously only set the `timeZone` display field)

## [0.8.0]

### Added
- `--content` and `--desc` flags on `task add` and `task edit` — set task content/notes and description
- `--clear-content` and `--clear-desc` flags on `task edit` — remove content or description
- `--tag` flag (repeatable) on `task add` and `task edit` — set tags
- `--clear-tags` flag on `task edit` — remove all tags
- `--item` flag (repeatable) on `task add` and `task edit` — add checklist/subtask items
- `--reminder` flag (repeatable) on `task add` and `task edit` — set reminders (e.g. `TRIGGER:PT0S`)
- `--repeat` flag on `task add` and `task edit` — set recurrence rules (e.g. `RRULE:FREQ=DAILY;INTERVAL=1`)
- `--clear-reminders` and `--clear-repeat` flags on `task edit` — remove reminders or recurrence
- `--color` flag on `project add` and `project edit` — set project color (hex string, e.g. `#FF0000`)
- `--view-mode` flag on `project add` and `project edit` — set view mode (`list`, `kanban`, `timeline`)
- `--kind` flag on `project add` and `project edit` — set project kind (`TASK`, `NOTE`)

## [0.7.0]

### Added
- Timeblocking support: `--start`, `--duration`, `--all-day`, `--timezone`/`--tz` flags on `task add` and `task edit`
- `--duration` computes due date from start + duration (e.g. `--start 2026-02-16T14:00 --duration 2h`)
- `--all-day` forces all-day event even with time inputs
- `--timezone` sets the IANA timezone for the task
- `--dry-run` / `-n` flag on `task add` — preview the API request body without creating the task
- Bulk operations for `task add`, `task edit`, `task complete`, and `task delete` — pass multiple positional args to operate on several tasks at once
- `--stdin` flag on all four task mutation commands — read titles or task IDs from stdin (one per line) for pipeline-friendly workflows
- Bulk output format: single item returns the same JSON as before (backward compatible); multiple items return a JSON array of `{id, status, data?, error?}` objects
- Partial failure support: if some operations succeed and others fail, all results are printed and the exit code is non-zero

### Changed
- `task add` positional argument is now `titles` (accepts one or more)
- `task edit`, `task complete`, `task delete` positional argument is now `task_ids` (accepts one or more)
- `task edit --title` is validated at runtime to require exactly one task ID
- `task delete` confirmation prompt now shows the count of tasks being deleted
- `task delete --stdin` requires `--force` (non-interactive context)

## [0.6.0]

### Changed
- **Breaking:** Restructured CLI from flat commands to nested `task`/`project` subcommand groups
  - Task commands: `add`, `edit`, `delete`, `complete`, `tasks`, `task` → `task add`, `task edit`, `task delete`, `task complete`, `task list`, `task get`
  - Project commands: `projects`, `project`, `add-project`, `edit-project`, `delete-project` → `project list`, `project get`, `project add`, `project edit`, `project delete`
  - Aliases preserved on nested commands: `task new`, `task done`, `task rm`, `task update`, `project rm`
- Running `ticktick-cli` with no arguments now prints usage instead of an error

## [0.5.0]

### Added
- `--due` / `-d` flag on `task add` — set due date when creating tasks (YYYY-MM-DD, `today`, or `tomorrow`)
- `task edit` command (aliased as `task update`) — modify existing tasks (due date, title)
- `--clear-due` flag on `task edit` — remove a task's due date

## [0.4.0]

### Added
- `usage` subcommand — prints concise help for all commands
- JSON output on stdout for `init` and `logout` commands

### Changed
- **Breaking:** All command output is now JSON on stdout (no more tab-separated text)
- **Breaking:** Errors are now JSON on stderr (`{"error": "..."}`)
- **Breaking:** Removed `--json`, `--color`, and `--quiet` flags
- Removed `indicatif` dependency (no more spinners)
- Removed pager support (`$PAGER` no longer used)
- Removed ANSI color output from all commands
- Login flow messages remain as plain text on stderr

## [0.3.1]

### Fixed
- Broken pipe error when piping output to commands like `head` or `grep`

## [0.3.0]

### Added
- Pre-commit hook for `cargo fmt`, `cargo clippy`, and `cargo test`
- Configurable OAuth callback port via `TICKTICK_OAUTH_PORT` environment variable (default: 8080)
- Support for reading credentials from environment variables (with precedence over `.env` file)
- XDG Base Directory support for config directory (`$XDG_CONFIG_HOME/ticktick-cli/`)
- `.env` file lookup in XDG config directory as fallback

### Changed
- OAuth redirect URI port changed from 8585 to 8080 (configurable)
- Improved error messages to reference environment variables and XDG config paths

### Fixed
- Homebrew tap dispatch payload now uses correct `ticktick-cli` formula name

## [0.2.0]

### Added
- Comprehensive `--help` with examples for all CLI commands
- Command aliases: `new` (add), `done` (complete), `rm` (delete)
- Homebrew tap installation support
- GitHub Actions release workflow with automatic homebrew-tap update

### Changed
- Refactored CLI to use clap for argument parsing

## [0.1.0]

### Added
- Initial project scaffolding with Cargo
- OAuth authentication flow (login/logout commands)
- Project listing command (`projects`)
- Get project by ID command (`project <id>`)
- Task listing command (`tasks [project_id]`)
- Create task command (`add <title>`)
- Complete task command (`complete <project_id> <task_id>`)
- Delete task command (`delete <project_id> <task_id>`)
- Integration tests for all API endpoints
- Automatic OAuth token refresh on 401 errors
- Project name lookup (use names instead of IDs in all commands)
