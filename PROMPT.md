# TickTick CLI - Rust Implementation

## Your Mission
Build a complete TickTick CLI in Rust that implements the ENTIRE API.

## Workflow for EACH API endpoint

1. **Plan first**: Enter plan mode, outline what you'll implement, auto-accept
2. **Implement**: Write the code for ONE endpoint
3. **Test**: Write and run a test that hits the real API (use creds from .env)
4. **Verify**: Ensure the test passes
5. **Follow SDLC**: Read SDLC section
6. **Next**: Move to the next endpoint

## Getting Started

1. Set up the project repo for a rust cli project.
   - Add sensible git ignores
   - Add sensible claude/settings.json accept commands for a rust project
2. Download the OpenAPI spec: `curl -o openapi.yaml https://ticktick.com/openapi.yaml`
3. Parse it to understand all available endpoints
4. Start with OAuth/auth flow, then work through each endpoint systematically

## Technical Requirements
- **Minimal deps**: Prefer native Rust where reasonable. Allowed exceptions:
  - `ureq` for HTTP (no async complexity)
  - `serde` + `serde_json` for JSON
  - `serde_yaml` for parsing the OpenAPI spec
- Config storage: `~/.config/ticktick-cli/`
- Colors: Raw ANSI escape codes
- CLI parsing: Native `std::env::args`

## Auth from .env
The `.env` file contains:
```
TICKTICK_CLIENT_ID=xxx
TICKTICK_CLIENT_SECRET=xxx
```

For testing, you may also have:
```
TICKTICK_ACCESS_TOKEN=xxx
```

## Testing Strategy

For each endpoint, create native rust test:
- Tests should be `#[ignore]` by default (they hit real API)
- Run with: `cargo test -- --ignored --test-threads=1`
- Each test should:
  1. Load .env credentials
  2. Make the actual API call
  3. Assert the response is valid
  4. Print the response for verification

## SDLC
- [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
- [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
- [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Order of Implementation

1. Project scaffolding and basic CLI structure
2. .env loading and config management
3. OAuth flow (login command)
4. Then systematically: download openapi.yaml, parse it, implement each endpoint

## Important

- ONE endpoint at a time
- Test MUST pass before committing
- Commit after EACH working endpoint
- Update CHANGELOG.md after each commit

When the OpenAPI spec is fully implemented (all endpoints working with tests):
Output <promise>COMPLETE</promise>
