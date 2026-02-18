# TickTick CLI

## API Reference

TickTick Open API docs: https://developer.ticktick.com/docs/openapi.md

## Architecture

```
src/
  main.rs        — entry point, SIGPIPE handling, exit codes
  cli.rs         — clap definitions and command dispatch (all commands live here)
  api/
    mod.rs       — HTTP helpers (get, post, post_empty, delete) with auto token refresh
    auth.rs      — OAuth flow
    project.rs   — project CRUD + fuzzy name resolution (case-insensitive, partial match)
    task.rs      — task CRUD, date/time parsing, duration parsing
  config.rs      — XDG config storage, .env loading
  output.rs      — JSON output helpers (success to stdout, errors to stderr)
tests/
  api_tests.rs   — integration tests hitting real API (#[ignore])
  cli_tests.rs   — CLI argument parsing, dry-run, and error case tests
```

## Output Contract

All output is JSON. No exceptions.

- Success data → stdout via `output::success()`
- Errors → stderr via `output::error()` as `{"error": "..."}`
- User prompts and informational messages → stderr via `eprintln!`

## Dependencies

Intentionally minimal. Do not add new dependencies without good reason. Current set:
- `clap` (CLI parsing), `ureq` (HTTP, sync), `serde`/`serde_json` (JSON), `libc` (timezone/signal), `strsim` (fuzzy matching)

## Error Handling

- All fallible functions return `Result<T, String>` — no custom error types
- No panics in library code; `unwrap()` only in tests or where failure is truly impossible

## Build & Check

- `make fix` — auto-format, auto-fix clippy lints, run tests
- `make check` — verify formatting, clippy (zero warnings), and tests. Run before every commit; all three must pass.
- `cargo test -- --ignored --test-threads=1` — run integration tests (requires `.env` with credentials)

## Testing

Always write tests for every change. No exceptions.

- **Unit tests**: Add to the relevant `#[cfg(test)] mod tests` block in the source file
- **Integration tests** (real API): Add to `tests/api_tests.rs` with `#[ignore]` (run with `cargo test -- --ignored --test-threads=1`)
- **CLI tests** (argument parsing, dry-run, error cases): Add to `tests/cli_tests.rs`

If a change can't be tested against the real API without credentials, write a CLI-level test using `--dry-run` or argument validation, or an ignored integration test.

## Conventions

- [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
- [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
- [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
- Update CHANGELOG.md with every user-facing change
