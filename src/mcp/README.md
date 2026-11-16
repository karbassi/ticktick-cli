# MCP server (`ticktick-cli mcp`)

A drop-in replacement for the Python `ticktick-mcp` server (FastMCP). Callers use tools as `mcp__ticktick__<name>` with the Python parameter names, so names, parameter names, required sets and JSON types must match `tests/fixtures/mcp-contract.json` exactly. Descriptions may be reworded.

## Layout

- `mod.rs`: `TickTickServer`, the `ServerHandler` (tools via `#[tool_handler]`, plus the four resources), and the stdio and streamable-HTTP transports.

- `util.rs`: shared helpers. Use these instead of re-deriving:

  - `run(move || -> Result<impl Into<Output>, String>)` runs a blocking body on the blocking pool. `Ok(Value)` becomes compact JSON text, `Ok(String)` becomes plain text, and `Err(msg)` or a panic becomes an `isError: true` result. Every tool returns `ToolResult`.

  - Raw JSON API: `v1_get/v1_post/v1_post_empty/v1_put/v1_delete/v1_delete_json`, `v2_get/v2_post/v2_put`, `as_list`. They return the API's JSON untouched, like Python. Don't call the typed `api::*` functions that return structs (they drop fields) or that print (`api::project::{list,get_by_id,members,create,update,delete}` write to stdout, which corrupts the stdio transport).

  - Resolution like Python `resolve.py`: `resolve_project_id` (handles "inbox"), `resolve_column_id`, `resolve_folder_id` (returns etag), and the generic `resolve_name` / `resolve_name_with_etag` over `[Value]` with `id`/`name`/`etag` keys.

  - Dates like `dates.py`: `parse_datetime`, `parse_duration`, `api_date(&parsed, tz)`, `check_timezone`. Also `priority_value` and `nonempty`.

- `tools/<module>.rs`: one file per Python `tools/<module>.py`, each with its own router. `tools/mod.rs` adds the routers together.

## Adding a module

Every Python module already has a stub, `src/mcp/tools/<module>.rs` (calendar, columns, comments, countdowns, filters, focus, folders, habits, search), declared and wired in `tools/mod.rs`. Filling one in touches only that file, the module's group in `PENDING_TOOLS`, and a new test file, so parallel work doesn't conflict. `sync_account` (from Python `calendar.py`) is already in `sync.rs`. Don't re-add it: a duplicate route would silently replace it.

1. Fill in the stub. Drop `allow_empty` from its `#[tool_router(...)]`, then copy the shape of `tags.rs`:

   ```rust
   #[derive(Debug, Deserialize, JsonSchema)]
   #[serde(deny_unknown_fields)]
   pub struct FooParams {
       /// Doc comment = parameter description.
       pub project: String,              // required
       #[serde(default)]
       pub name: Option<String>,         // optional, nullable (Python `str | None = None`)
       #[serde(default)]
       pub force: bool,                  // optional non-null (Python `bool = False`)
       #[serde(default = "default_limit")]
       pub limit: i64,                   // optional with a default (Python `int = 100`)
   }

   #[tool_router(router = <module>_router, vis = "pub(crate)")]
   impl TickTickServer {
       #[tool(description = "...", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
       async fn foo(&self, Parameters(p): Parameters<FooParams>) -> ToolResult {
           run(move || { let pid = resolve_project_id(&p.project)?; util::v1_get(&format!("/project/{pid}/foo")) }).await
       }
   }
   ```

   Every non-`Option` field with a Python default needs `#[serde(default...)]`, or it becomes required. Use `i64` for `int`, `f64` for `float`, `Vec<String>` for `list[str]`. Copy the annotations from the Python decorator. A tool with no parameters takes only `&self`.

2. For a brand-new module with no stub, add `mod <module>;` to `tools/mod.rs` and `+ TickTickServer::<module>_router()` to `router()`.

3. Remove the module's tools from `PENDING_TOOLS` in `tests/mcp_contract.rs`. Run `cargo test --test mcp_contract`; it fails on any name, parameter, required or type mismatch.

4. Add end-to-end tests in a new `tests/mcp_<module>.rs` with `mod common;`. `common::McpBuilder::new(&server).start()` spawns the server against an httpmock `MockServer` (v1 at `/open/v1`, v2 at `/api/v2`, ms at `/ms`, `TZ=America/Chicago`, isolated config). `call_tool(name, args)` returns `ToolCall { is_error, text }`. Assert exact request bodies with `json_body` and `mock.assert()`. Port cases from the Python suite (`ticktick-mcp/tests/test_tools/test_<module>.py`).

5. Match Python messages and output exactly: confirmation strings, error texts, and which fields are sent only when passed. `make check` must pass.
