# TickTick CLI

A command-line interface for [TickTick](https://ticktick.com) task management, built in Rust. Designed as a JSON-first data API for scripts and LLM agents — all output is structured JSON on stdout, errors are JSON on stderr.

## Features

- **JSON-first** — every command outputs structured JSON, no flags needed
- OAuth authentication flow with automatic token refresh
- Inbox support: use `inbox` as a project name in any task command
- Smart project name resolution (case-insensitive, partial match, "Did you mean?" suggestions)
- Full task management: create, edit, complete, delete, move with bulk operations
- Task details: content, description, tags, checklist items, reminders, recurrence
- Timeblocking: start/due dates, durations, all-day events, local timezone with account mismatch detection
- Project options: color, view mode (list/kanban/timeline), kind (task/note)
- Shell completions (bash, zsh, fish)

## Installation

### Homebrew

```bash
brew install karbassi/tap/ticktick
```

### From source

```bash
git clone https://github.com/karbassi/ticktick-cli.git
cd ticktick-cli
cargo build --release
```

The binary will be at `target/release/ticktick-cli`.

## Setup

### 1. Create a TickTick App

1. Go to [TickTick Developer Center](https://developer.ticktick.com/manage)
2. Create a new app
3. Set the redirect URI to `http://127.0.0.1:8080/callback` (or your preferred port)
4. Note your Client ID and Client Secret

### 2. Configure credentials

Set environment variables directly:

```bash
export TICKTICK_CLIENT_ID=your_client_id
export TICKTICK_CLIENT_SECRET=your_client_secret
export TICKTICK_OAUTH_PORT=8080  # optional, defaults to 8080
```

Or create a `.env` file (environment variables take precedence). The CLI checks these locations in order:

1. `./.env` (current directory)
2. `$XDG_CONFIG_HOME/ticktick-cli/.env` (or `~/.config/ticktick-cli/.env`)

```bash
TICKTICK_CLIENT_ID=your_client_id
TICKTICK_CLIENT_SECRET=your_client_secret
```

### 3. Authenticate

```bash
ticktick-cli login
```

This will open a URL in your terminal. Copy it to your browser, authorize the app, and the CLI will capture the callback automatically.

## Usage

```
ticktick-cli <COMMAND> [OPTIONS]
```

### Commands

| Command | Aliases | Description |
|---------|---------|-------------|
| `login` | | Authenticate with TickTick |
| `logout` | | Remove stored credentials |
| `task list [project]` | | List all tasks, optionally filtered by project |
| `task get <project> <task_id>` | | Get task details |
| `task add <titles...> [-p project]` | `task new` | Create one or more tasks |
| `task edit <project> <task_ids...>` | `task update` | Edit one or more tasks |
| `task complete <project> <task_ids...>` | `task done` | Mark one or more tasks as complete |
| `task delete <project> <task_ids...>` | `task rm` | Delete one or more tasks |
| `task move <project> <task_ids...> --to <dest>` | `task mv` | Move one or more tasks to a different project |
| `project list` | | List all projects |
| `project get <name>` | | Get project details by name or ID |
| `project add <name>` | | Create a new project |
| `project edit <project>` | | Edit a project |
| `project delete <project>` | `project rm` | Delete a project |
| `init [--local]` | | Generate `.env` template |
| `usage` | | Print concise help for all commands |
| `completions <shell>` | | Generate shell completions |

All five task mutation commands (`add`, `edit`, `complete`, `delete`, `move`) accept multiple positional arguments and a `--stdin` flag to read items from a pipe (one per line).

#### Task flags

| Flag | Commands | Description |
|------|----------|-------------|
| `-p`, `--project` | `add` | Target project |
| `-d`, `--due` | `add`, `edit` | Due date (`YYYY-MM-DD`, `YYYY-MM-DDTHH:MM`, `today`, `tomorrow`) |
| `-s`, `--start` | `add`, `edit` | Start date/time |
| `--duration` | `add`, `edit` | Duration (e.g. `1h`, `30m`, `1h30m`). Computes due = start + duration |
| `--all-day` | `add`, `edit` | Force all-day event |
| `--timezone`, `--tz` | `add`, `edit` | IANA timezone override (e.g. `America/New_York`) |
| `-P`, `--priority` | `add`, `edit` | Priority: `none`, `low`, `medium`, `high` |
| `-t`, `--to` | `move` | Destination project |
| `-t`, `--title` | `edit` | New title (single task only) |
| `--content` | `add`, `edit` | Task content/notes |
| `--desc` | `add`, `edit` | Task description |
| `--tag` | `add`, `edit` | Tag (repeatable) |
| `--item` | `add`, `edit` | Checklist item (repeatable) |
| `--reminder` | `add`, `edit` | Reminder trigger (repeatable, e.g. `TRIGGER:PT0S`) |
| `--repeat` | `add`, `edit` | Recurrence rule (e.g. `RRULE:FREQ=DAILY;INTERVAL=1`) |
| `--clear-due` | `edit` | Remove due date |
| `--clear-start` | `edit` | Remove start date |
| `--clear-content` | `edit` | Remove content |
| `--clear-desc` | `edit` | Remove description |
| `--clear-tags` | `edit` | Remove all tags |
| `--clear-reminders` | `edit` | Remove all reminders |
| `--clear-repeat` | `edit` | Remove recurrence |
| `-n`, `--dry-run` | `add` | Preview request body without creating |

#### Project flags

| Flag | Commands | Description |
|------|----------|-------------|
| `-n`, `--name` | `edit` | New project name |
| `--color` | `add`, `edit` | Project color (hex, e.g. `#FF0000`) |
| `--view-mode` | `add`, `edit` | View mode: `list`, `kanban`, `timeline` |
| `--kind` | `add`, `edit` | Project kind: `TASK`, `NOTE` |

**Note:** Project can be specified by name (case-insensitive, partial match supported), ID, or `inbox` for the inbox project.

Run `ticktick-cli --help` for a list of commands, or `ticktick-cli <command> --help` for detailed usage and examples.

### Global flags

| Flag | Description |
|------|-------------|
| `-v`, `--verbose` | Increase verbosity (debug logging to stderr) |

### Examples

```bash
# List all projects (JSON array to stdout)
ticktick-cli project list

# Pipe through jq
ticktick-cli project list | jq '.[].name'

# List tasks in a project (by name)
ticktick-cli task list Work
ticktick-cli task list Personal

# List inbox tasks
ticktick-cli task list inbox

# Create a task (goes to inbox)
ticktick-cli task add "Buy groceries"

# Create a task in a specific project
ticktick-cli task add "Review PR" -p Work

# Create multiple tasks at once
ticktick-cli task add "Task 1" "Task 2" "Task 3" -p Work

# Create a task with due date
ticktick-cli task add "Submit report" --due 2026-03-01
ticktick-cli task add "Call dentist" -d tomorrow

# Timeblocking: start + duration
ticktick-cli task add "Focus block" --start 2026-02-16T14:00 --duration 2h

# Tags, content, and checklist items
ticktick-cli task add "Sprint planning" --tag work --tag urgent \
  --content "Quarterly review" --item "Review backlog" --item "Set priorities"

# Recurring task with reminder
ticktick-cli task add "Daily standup" \
  --repeat "RRULE:FREQ=DAILY;INTERVAL=1" --reminder "TRIGGER:PT0S"

# Edit a task
ticktick-cli task edit Personal abc123 --due tomorrow --priority high
ticktick-cli task edit Personal abc123 --clear-tags

# Preview a task without creating it
ticktick-cli task add --dry-run "Test task"

# Create a project with options
ticktick-cli project add "Sprint Board" --color "#FF0000" --view-mode kanban --kind TASK

# Complete a task
ticktick-cli task complete Work abc123def456
ticktick-cli task done Work abc123def456    # alias

# Complete an inbox task
ticktick-cli task complete inbox abc123def456

# Complete multiple tasks
ticktick-cli task complete Personal id1 id2 id3

# Move a task to another project
ticktick-cli task move inbox abc123 --to Work
ticktick-cli task mv inbox abc123 -t Work     # alias

# Delete a task
ticktick-cli task delete Personal abc123def456
ticktick-cli task rm Personal abc123def456    # alias

# Bulk operations via stdin (pipe-friendly)
ticktick-cli task list Work | jq -r '.[].id' | ticktick-cli task complete Work --stdin
ticktick-cli task list Work | jq -r '.[].id' | ticktick-cli task delete Work --stdin --force
echo -e "Task A\nTask B" | ticktick-cli task add --stdin -p Work

# Concise help for all commands
ticktick-cli usage
```

### Output format

All data commands emit JSON on stdout. Errors emit JSON on stderr:

```bash
# Success: JSON on stdout
ticktick-cli project list
# [{"id": "...", "name": "Personal", ...}, ...]

# Error: JSON on stderr, non-zero exit
ticktick-cli project list
# {"error":"not authenticated\n\n  hint: Run 'ticktick-cli login' to authenticate"}
```

**Bulk operations:** A single item outputs the same JSON as before (backward compatible). Multiple items output a JSON array of result objects:

```json
[
  { "id": "abc", "status": "ok", "data": { ... } },
  { "id": "def", "status": "error", "error": "not found" }
]
```

Exit code is 0 if all operations succeed, 1 if any fail.

Interactive messages (login prompts, delete confirmations) go to stderr as plain text and won't interfere with piped JSON.

### Timezone handling

All datetime values (`--due`, `--start`) use your **local system timezone** by default. When you type `--start 2026-03-15T14:00`, it means 2pm in your local time, not UTC.

- **Local timezone** is detected from `TZ` env var or `/etc/localtime`
- **Account timezone** is learned from the first task create/edit response and stored in config
- **Mismatch prompt**: if your local timezone differs from your TickTick account timezone (e.g. you're traveling), the CLI asks which to use:
  ```
  Local timezone (America/New_York) differs from TickTick account (America/Chicago).
  Use which timezone? [l]ocal / [a]ccount (default: local):
  ```
- **Non-interactive mode** (pipes, CI) defaults to local without prompting
- **`--tz`** overrides both the UTC offset and the TickTick display timezone

## Configuration

Credentials are stored in `$XDG_CONFIG_HOME/ticktick-cli/config.json` (defaults to `~/.config/ticktick-cli/config.json`) after authentication.

You can also set `TICKTICK_ACCESS_TOKEN` as an environment variable or in your `.env` file to skip the OAuth flow.

## Development

### Running tests

Tests hit the real TickTick API and require valid credentials in `.env`:

```bash
# Run all API tests
cargo test -- --ignored --test-threads=1
```

### Building

```bash
cargo build          # Debug build
cargo build --release # Release build
```

## API Coverage

This CLI implements the complete [TickTick OpenAPI specification](https://ticktick.com/openapi.yaml):

| Endpoint | CLI Command |
|----------|-------------|
| `GET /project` | `project list` |
| `GET /project/{id}` | `project get <id>` |
| `POST /project` | `project add <name>` |
| `POST /project/{id}` | `project edit <id>` |
| `DELETE /project/{id}` | `project delete <id>` |
| `GET /project/{id}/data` | `task list <project>` |
| `GET /project/{pid}/task/{tid}` | `task get <project> <tid>` |
| `POST /task` | `task add <title>` |
| `POST /task/{tid}` | `task edit <project> <tid>`, `task move <project> <tid> --to <dest>` |
| `POST /project/{pid}/task/{tid}/complete` | `task complete <project> <tid>` |
| `DELETE /project/{pid}/task/{tid}` | `task delete <project> <tid>` |

## License

MIT
