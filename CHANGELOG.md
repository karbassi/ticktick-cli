# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0-rc.4]

### Added

- `ticktick-cli mcp` runs an MCP server over stdio (default) or streamable HTTP (`--http`, `--host`, `--port`; serves `/mcp`). It is a drop-in replacement for the Python `ticktick-mcp` server: all 57 tools plus the `ticktick://projects`, `tags`, `settings` and `profile` resources, with tool names, parameter names, required parameters and JSON types checked by `tests/mcp_contract.rs` against the Python server's tool list.
- Like FastMCP's lax validation, tool parameters accept numeric strings, integral floats, `1`/`0`, `"yes"`/`"no"`, `"on"`/`"off"`, `"t"`/`"f"`, `"y"`/`"n"` and booleans where the schema wants a number or boolean. An unknown tool name is an `isError` result ("Unknown tool: 'x'"), not a protocol error.
- The HTTP transport binds loopback only, because it has no auth. It checks `Host` and `Origin` the way FastMCP does: loopback hosts on any port, loopback Origins when the Host is loopback, same-origin requests (honoring `X-Forwarded-Proto` from a loopback proxy), plus `FASTMCP_HTTP_ALLOWED_HOSTS` and `FASTMCP_HTTP_ALLOWED_ORIGINS` (JSON arrays; ports ignored, `*` globs allowed). A disallowed Host gets 421; an empty `Origin` is ignored. Env fallbacks: `TICKTICK_MCP_TRANSPORT`, `TICKTICK_MCP_HOST`, `TICKTICK_MCP_PORT`. `--host`/`--port` with the stdio transport are an error.
- HTTP sessions expire after 30 minutes idle; `TICKTICK_MCP_SESSION_TIMEOUT` sets it in seconds. On SIGTERM or SIGINT the server stops accepting connections, lets in-flight calls finish for up to 5s, then closes open streams and exits; a second signal exits at once. `notifications/cancelled` stops a multi-request tool before its next API request. SIGPIPE is ignored.
- `TICKTICK_TIMEZONE` (IANA name) sets the MCP server's zone for dates given without a `timezone`, including "today"/"tomorrow". Default is the system zone, like the Python server; config.json's `account_timezone` is not used.
- `TICKTICK_API_BASE`, `TICKTICK_V2_BASE`, `TICKTICK_MS_BASE` and `TICKTICK_OAUTH_TOKEN_URL` override the API endpoints, for tests against a mock server. Defaults are unchanged.
- Where the MCP server intentionally differs from the Python server:
  - `list_events` returns a tool error (`isError`) with the real message, such as the missing v2 session token hint. The Python server returns `[]` on any error.
  - `complete_task` with several IDs keeps going past a failed batch request and reports it, e.g. `Completed 70 of 120 tasks; not completed: …; chunk 2 failed: …`. It is a tool error only when no task was completed.
  - `assign_task` adds the "assignee must be a member" hint only for 403 and 500 (what a non-member username returns). The Python server adds it to every HTTP error.
  - `add_task` with `start` and `all_day=true` but no `due` sends `isAllDay` and the all-day reminder. The Python server ignores `all_day` there.
  - Tools that take several habit, filter, folder or habit-section names fetch the list once per call, not once per name. Habit, filter and folder IDs skip the fetch unless an etag is needed (`archive_habits`). `focus_log` fetches its two focus types concurrently.

### Changed

- Where credentials come from now decides all account state. An access token from the environment or `./.env` takes its refresh token (`TICKTICK_REFRESH_TOKEN`) and v2 cookie (`TICKTICK_V2_SESSION_TOKEN`) from the environment or `.env` only, never from config.json; its refreshed token is kept in memory, its inbox ID is cached in config.json under a fingerprint of that token, and config.json's `account_timezone` is neither read nor written. With config.json's login, `TICKTICK_V2_SESSION_TOKEN` still wins over config.json's cookie. `TICKTICK_ACCESS_TOKEN` no longer needs `TICKTICK_CLIENT_ID` and `TICKTICK_CLIENT_SECRET` to be used.
- `.env` and the process environment are merged per key (the environment wins), and `.env` may also hold `TICKTICK_REFRESH_TOKEN` and `TICKTICK_V2_SESSION_TOKEN`.
- `login` clears the cached inbox ID and account timezone.
- config.json writes take an advisory lock on `config.json.lock`, so the CLI and a running `mcp` server don't overwrite each other's changes. A token refreshed from config.json is written back only if config.json still holds the refresh token that was used, so a `login` from another process wins.
- v2 error hints name `TICKTICK_V2_SESSION_TOKEN`, and a v2 403 says the feature requires TickTick Premium. MCP 401 messages match the Python server's and don't mention CLI commands. An expired env access token's hint names the env vars instead of `login`.
- UTC offsets now come from the `jiff` tz database instead of temporarily setting `TZ` (not thread-safe). An unknown `--tz`/`timezone` or an impossible date such as `2026-02-30` is now an error; before, it was silently sent with a UTC or normalized offset. `timeZone` is sent in canonical form (`america/chicago` becomes `America/Chicago`), and file paths and `Etc/Unknown` are rejected.
- `--tz` also decides the date for "today"/"tomorrow", and is no longer cached as the account timezone.
- HTTP requests time out after 10s to connect and 30s per read or write (`TICKTICK_HTTP_TIMEOUT_SECS`), and the token refresh also has a 30s overall deadline. DNS lookup is not bounded.
- Habit date errors are capitalized, and the per-field "invalid year/month/day" messages are merged into one "non-numeric components" message.
- CLI `task assign` adds the "assignee must be a member" hint only for 403 and 500.

### Fixed

- Editing or archiving a habit, and editing a filter, no longer wipes the fields you didn't change (a habit's name and color, a filter's name and rule). TickTick's v2 batch update replaces the whole record, so the CLI and the MCP tools now send the full record with the changes applied. The Python server has the same bug.
- `habit list` and `filter list` no longer fail on a record whose name is `null`.
- Answering "n" at a delete confirmation now cancels the delete. Before, task, comment, tag, folder, habit, habit section and filter deletes went ahead anyway.
- A blank `TICKTICK_ACCESS_TOKEN` or empty `.env` value no longer overrides config.json.
- A config.json that fails to parse is no longer overwritten with defaults; saves report the parse error.
- The config temp file is created 0600 with a unique name, and stale temp files are removed.
- One v2 device ID per process when config.json can't be written.
- A huge `--duration`/`duration` is an error instead of a panic or wrapped date.
- `task list` and `--dry-run` no longer create and delete an inbox probe task; `task list` includes the inbox when its ID is cached. A probe task that can't be deleted is reported.
- An empty or invalid account timezone echoed by TickTick is no longer cached.

## [1.0.0-rc.3]

### Added

- `project column list|add|edit` manage kanban columns via `GET`/`POST /open/v1/project/{id}/column` and `POST /open/v1/project/{id}/column/{columnId}`. There is no `column delete`: the official API returns 405 for `DELETE` on a column (verified live). Closes #7.
- `task add --column` and `task edit --column` set a task's `columnId` by column name or ID; `task get` now includes `columnId`.
- `task complete` with several task IDs now uses `POST /open/v1/task/completeTasks` in batches of 50 instead of one request per task. A single ID still uses the per-task endpoint. IDs the API did not complete are reported as errors in the per-task results. Verified live.
- `task search <query>` — server-side keyword search via `POST /open/v1/task/search`. Whole-word, case-insensitive match; narrow with `--project`, `--tag`, `--status`, `--due-from`, `--due-to`.
- `task filter` — server-side filtering via `POST /open/v1/task/filter` by `--project`, `--start-date`, `--end-date`, `--priority`, `--tag`, `--status`, `--kind`. The endpoint calls plain tasks `TASK`, so `--kind text` sends that. Verified live; unrelated to the saved `filter` command.
- `task add` now applies a default reminder when a date is given and no `--reminder` is passed, matching TickTick's own connector: `TRIGGER:PT0S` for timed tasks, `TRIGGER:P0DT9H0M0S` (9am) for all-day tasks. New `--no-reminder` opts out. Before this, dated tasks created here got no reminder, so TickTick never notified. `task edit` is unchanged.
- `task comment list|add|delete` manage task comments via the official v1 API (`GET .../task/{t}/comments`, `POST .../task/{t}/comment`, `DELETE .../task/{t}/comment/{id}`). There is no `edit`: the API has no edit endpoint (`PUT`/`POST`/`PATCH` on `.../comment/{id}` return 405), so delete and re-add instead.
- `project members <project>` (`GET /open/v1/project/{id}/members`), `task assign <project> <task_id> --assignee <username>` (`POST /open/v1/task/assign`) and `task unassign <project> <task_id>` (`POST /open/v1/task/unassign`). Members and unassign verified live; assign is mock-only (request body unit-tested) because it needs a project shared with another person.
- `countdown list` via `GET /open/v1/countdown`. Read-only: `POST`, `PUT` and `DELETE` on `/open/v1/countdown` return 405, so there are no write commands.

## [1.0.0-rc.2]

### Changed — official API migration (#21)

These commands now call TickTick's official Open API (`/open/v1`) instead of the private v2 API, so they no longer need a v2 session cookie. Each one was verified live against v2 before switching.

- `tag list` uses `GET /open/v1/tag`. Same tags, colors, and parents; v1 omits `rawName` and `etag`, which now print as `null`. (#22)
- `tag add` uses `POST /open/v1/tag`, one request per name, and prints the created tags. (#23)
- `tag edit` uses `PUT /open/v1/tag`; only the flags you pass change. (#24)
- `tag rename` uses `PUT /open/v1/tag/rename?scope=0`, renaming the tag on every task, as before. (#25)
- `tag delete` uses `DELETE /open/v1/tag/delete?scope=0`. As before, the tag is also removed from tasks. (#26)
- `folder list` (and folder name resolution for `project add/edit --folder`) uses `GET /open/v1/project/group` instead of a full v2 account sync. (#27)
- `folder add`, `folder rename`, and `folder delete` use `/open/v1/project/group[/{id}]`. Deleting a folder still ungroups its projects instead of deleting them. (#28, #29, #30)
- `habit section list` (and section name resolution) uses `GET /open/v1/habit/sections`. Section add/rename/delete stay on v2: there is no official write endpoint. (#34)
- `focus log` uses `GET /open/v1/focus`. It returns pomodoro and stopwatch records and splits ranges longer than 30 days to fit the v1 limit. (#37)
- `task move` uses `POST /open/v1/task/move` (same body as v2 `/batch/taskProject`, still one request for all IDs). Verified live inbox→project with notes kept. (#42)

### Still on v2

These have no official endpoint, or the official one failed the parity check (details on each issue): `tag merge`, habits (#31–#33, #35, #36), calendar (#39, #40), `task list --completed` (#41), `settings` (#43), filters, trash, subtasks, focus timer/stats/timeline, profile, sync.

## [1.0.0-rc.1]

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
