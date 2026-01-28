# TickTick CLI

A command-line interface for [TickTick](https://ticktick.com) task management, built in Rust.

## Features

- OAuth authentication flow with automatic token refresh
- List all projects
- Get project details
- List tasks (all or by project)
- Create new tasks
- Complete tasks
- Delete tasks

## Installation

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
3. Set the redirect URI to `http://127.0.0.1:8585/callback`
4. Note your Client ID and Client Secret

### 2. Configure credentials

Create a `.env` file in the project directory:

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

**Note:** Project can be specified by name (case-insensitive, partial match supported) or ID.

Run `ticktick-cli --help` for a list of commands, or `ticktick-cli <command> --help` for detailed usage and examples.

### Examples

```bash
# List all projects
ticktick-cli projects

# List tasks in a project (by name)
ticktick-cli tasks Work
ticktick-cli tasks 'My Project'
ticktick-cli tasks Personal

# Create a task (goes to inbox)
ticktick-cli add "Buy groceries"
ticktick-cli new "Buy groceries"           # alias

# Create a task in a specific project
ticktick-cli add "Review PR" -p Work

# Complete a task
ticktick-cli complete Work abc123def456
ticktick-cli done Work abc123def456    # alias

# Delete a task
ticktick-cli delete Personal abc123def456
ticktick-cli rm Personal abc123def456    # alias

# Get help for a specific command
ticktick-cli add --help
ticktick-cli complete --help
```

## Configuration

Credentials are stored in `~/.config/ticktick-cli/config.json` after authentication.

For development/testing, you can also set `TICKTICK_ACCESS_TOKEN` in your `.env` file to skip the OAuth flow.

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
