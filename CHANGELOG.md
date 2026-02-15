# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
