# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
