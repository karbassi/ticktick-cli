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

The binary will be at `target/release/ticktick`.

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
ticktick login
```

This will open a URL in your terminal. Copy it to your browser, authorize the app, and the CLI will capture the callback automatically.

## Usage

```
ticktick <COMMAND> [OPTIONS]
```

### Commands

| Command | Description |
|---------|-------------|
| `login` | Authenticate with TickTick |
| `logout` | Remove stored credentials |
| `projects` | List all projects |
| `project <name>` | Get project details by name or ID |
| `tasks` | List all tasks |
| `tasks <project>` | List tasks for a specific project |
| `add <title>` | Create a new task |
| `add <title> -p <project>` | Create a task in a specific project |
| `complete <project> <task_id>` | Mark a task as complete |
| `delete <project> <task_id>` | Delete a task |
| `help` | Show help message |
| `version` | Show version |

**Note:** Project can be specified by name (case-insensitive, partial match supported) or ID.

### Examples

```bash
# List all projects
ticktick projects

# List tasks in a project (by name)
ticktick tasks Work
ticktick tasks 'My Project'
ticktick tasks Personal

# Create a task (goes to inbox)
ticktick add "Buy groceries"

# Create a task in a specific project
ticktick add "Review PR" -p Work

# Complete a task
ticktick complete Work abc123def456

# Delete a task
ticktick delete Personal abc123def456
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
