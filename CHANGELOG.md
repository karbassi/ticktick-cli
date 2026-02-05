# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
