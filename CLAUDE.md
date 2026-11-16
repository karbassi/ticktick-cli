# TickTick CLI

## API Reference

TickTick Open API docs: https://developer.ticktick.com/docs/openapi.md

## Architecture

```
src/
  main.rs        — entry point, SIGPIPE handling, exit codes
  cli.rs         — clap definitions and command dispatch (all commands live here)
  api/
    mod.rs       — HTTP helpers (get, post, post_empty, put, delete, delete_json) with auto token refresh
    auth.rs      — OAuth flow
    project.rs   — project CRUD + fuzzy name resolution (case-insensitive, partial match)
    column.rs    — kanban column list/create/update via v1 API (no delete endpoint), name resolution
    project_group.rs — project folder/group CRUD via v1 API (list, create, delete, rename, resolve)
    task.rs      — task CRUD, batch complete, search/filter, assign, date/time parsing (jiff), duration parsing
    comment.rs   — task comments via v1 API (list, add, delete)
    countdown.rs — countdown list via v1 API (read-only)
    tag.rs       — tag CRUD via v1 API (list, create, delete, rename, edit); merge via v2
    filter.rs    — saved filter CRUD via v2 API (list, create, update, delete, fuzzy resolve)
    calendar.rs  — calendar accounts and events (read-only) via v2 API
    focus.rs     — focus log via v1; timer control and statistics via v2 + ms.ticktick.com APIs
    habit.rs     — habit CRUD, check-in, query, section writes via v2 API; section list via v1
    v2.rs        — v2 API session auth (cookie-based), HTTP helpers (get/post/put), batch check, completed tasks, subtask parent batch, profile, settings, trash
  config.rs      — XDG config storage (locked load-modify-save via `update`), .env loading, v2 session/device management
  output.rs      — JSON output helpers (success to stdout, errors to stderr)
  mcp/           — `ticktick-cli mcp`, a drop-in for the Python `ticktick-mcp` server (see mcp/README.md)
    mod.rs       — TickTickServer, resources, stdio and streamable-HTTP transports
    util.rs      — raw-JSON API calls, name resolution, dates, tool result helpers
    guard.rs     — HTTP Host/Origin checks with FastMCP's rules (ports ignored, fnmatch globs, loopback Origin)
    coerce.rs    — lax argument coercion ("5" → 5, 25.0 → 25, "true" → true) against each tool's schema, like FastMCP
    tools/       — one file per Python tools module, each with its own router
tests/
  api_tests.rs   — integration tests hitting real API (#[ignore])
  cli_tests.rs   — CLI argument parsing, dry-run, and error case tests
  env_override_tests.rs — env token/base-URL overrides against a mock server
  common/mod.rs  — McpBuilder/McpClient: spawn `mcp` against an httpmock server
  mcp_contract.rs — tool names, params and types vs fixtures/mcp-contract.json (Python's tools/list)
  mcp_stdio.rs   — task/project/tag tools end to end over stdio
  mcp_http.rs    — HTTP transport, Host/Origin checks, shutdown, session timeout
  mcp_coercion.rs — lax argument coercion end to end
  mcp_<module>.rs — per-module tool tests (calendar, columns, comments, filters, focus, folders, habits, search)
```

## Output Contract

All output is JSON. No exceptions.

- Success data → stdout via `output::success()`
- Errors → stderr via `output::error()` as `{"error": "..."}`
- User prompts and informational messages → stderr via `eprintln!`

## Dependencies

Intentionally minimal. Do not add new dependencies without good reason. Current set:
- `clap` (CLI parsing), `ureq` (HTTP, sync), `serde`/`serde_json` (JSON), `libc` (signal handling only), `strsim` (fuzzy matching)
- `jiff` (tz database: UTC offsets and DST for dates)
- `rmcp` (MCP server protocol and tool macros), `tokio` (async runtime for the MCP server; API calls stay sync on the blocking pool), `axum` (HTTP transport for `mcp --http`), `schemars` (JSON Schemas for MCP tool parameters)

## Error Handling

- All fallible functions return `Result<T, String>` — no custom error types
- No panics in library code; `unwrap()` only in tests or where failure is truly impossible

## Build & Check

- `make fix` — auto-format, auto-fix clippy lints, run tests
- `make check` — verify formatting, clippy (zero warnings), and tests. Run before every commit; all three must pass.
- `cargo test -- --ignored --test-threads=1` — run integration tests (requires `.env` with credentials)

## Testing

Always write tests for every change. No exceptions. Follow `CODING_STANDARDS.md`: no tautological tests, and comments only say what the code can't.

- **Unit tests**: Add to the relevant `#[cfg(test)] mod tests` block in the source file
- **Integration tests** (real API): Add to `tests/api_tests.rs` with `#[ignore]` (run with `cargo test -- --ignored --test-threads=1`)
- **CLI tests** (argument parsing, dry-run, error cases): Add to `tests/cli_tests.rs`

If a change can't be tested against the real API without credentials, write a CLI-level test using `--dry-run` or argument validation, or an ignored integration test.

## Conventions

- [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
- [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
- [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
- Update CHANGELOG.md with every user-facing change
