# TickTick CLI

A command-line interface for [TickTick](https://ticktick.com) task management, built in Rust. Designed as a JSON-first data API for scripts and LLM agents — all output is structured JSON on stdout, errors are JSON on stderr.

## Features

- **JSON-first** — every command outputs structured JSON, no flags needed
- OAuth authentication flow with automatic token refresh
- Smart project name resolution (case-insensitive, partial match, "Did you mean?" suggestions)
- List all projects and tasks
- Create, complete, and delete tasks
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
| `projects` | | List all projects |
| `project <name>` | | Get project details by name or ID |
| `tasks [project]` | | List all tasks, optionally filtered by project |
| `add <title> [-p project]` | `new` | Create a new task |
| `complete <project> <task_id>` | `done` | Mark a task as complete |
| `delete <project> <task_id>` | `rm` | Delete a task |
| `init [--local]` | | Generate `.env` template |
| `usage` | | Print concise help for all commands |
| `completions <shell>` | | Generate shell completions |

**Note:** Project can be specified by name (case-insensitive, partial match supported) or ID.

Run `ticktick-cli --help` for a list of commands, or `ticktick-cli <command> --help` for detailed usage and examples.

### Global flags

| Flag | Description |
|------|-------------|
| `-v`, `--verbose` | Increase verbosity (debug logging to stderr) |

### Examples

```bash
# List all projects (JSON array to stdout)
ticktick-cli projects

# Pipe through jq
ticktick-cli projects | jq '.[].name'

# List tasks in a project (by name)
ticktick-cli tasks Work
ticktick-cli tasks Personal

# Create a task (goes to inbox)
ticktick-cli add "Buy groceries"

# Create a task in a specific project
ticktick-cli add "Review PR" -p Work

# Preview a task without creating it
ticktick-cli add --dry-run "Test task"

# Complete a task
ticktick-cli complete Work abc123def456
ticktick-cli done Work abc123def456    # alias

# Delete a task
ticktick-cli delete Personal abc123def456
ticktick-cli rm Personal abc123def456    # alias

# Concise help for all commands
ticktick-cli usage
```

### Output format

All data commands emit JSON on stdout. Errors emit JSON on stderr:

```bash
# Success: JSON on stdout
ticktick-cli projects
# [{"id": "...", "name": "Personal", ...}, ...]

# Error: JSON on stderr, non-zero exit
ticktick-cli projects
# {"error":"not authenticated\n\n  hint: Run 'ticktick-cli login' to authenticate"}
```

Interactive messages (login prompts, delete confirmations) go to stderr as plain text and won't interfere with piped JSON.

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
| `GET /project` | `projects` |
| `GET /project/{id}` | `project <id>` |
| `GET /project/{id}/data` | `tasks <id>` |
| `POST /task` | `add <title>` |
| `POST /project/{pid}/task/{tid}/complete` | `complete <pid> <tid>` |
| `DELETE /project/{pid}/task/{tid}` | `delete <pid> <tid>` |

## License

MIT
