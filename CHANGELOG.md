# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project scaffolding with Cargo
- Basic CLI structure with help command
- Config module for .env loading and token storage
- OAuth authentication flow (login command)
- Logout command
- Project listing command (`projects`)
- Get project by ID command (`project <id>`)
- Task listing command (`tasks [project_id]`)
- Create task command (`add <title>`)
- Complete task command (`complete <project_id> <task_id>`)
- Delete task command (`delete <project_id> <task_id>`)
- Integration tests for all API endpoints
- Automatic OAuth token refresh on 401 errors
