//! The MCP server must be a drop-in replacement for the Python ticktick-mcp
//! server: `tests/fixtures/mcp-contract.json` is the Python server's
//! `tools/list` (with input schemas) and resource list. Every tool registered
//! here must match it by name, parameter names, required set and JSON types.
//!
//! `PENDING_TOOLS` lists fixture tools not implemented yet. A fixture tool
//! must be registered or pending, never both or neither; shrink the list to
//! empty as modules land.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// Fixture tools not yet implemented, grouped by Python module.
const PENDING_TOOLS: &[&str] = &[
    // calendar.py (sync_account is done, in tools/sync.rs)
    "list_calendars",
    "list_events",
    // comments.py
    "list_comments",
    "add_comment",
    "delete_comment",
    // countdowns.py
    "list_countdowns",
    // habits.py
    "list_habits",
    "add_habit",
    "edit_habit",
    "delete_habits",
    "checkin_habit",
    "habit_log",
    "archive_habits",
    "manage_habit_sections",
    // search.py
    "search_tasks",
    "filter_tasks",
];

fn fixture() -> Value {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/mcp-contract.json"
    ))
    .unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn by_name(tools: &[Value]) -> BTreeMap<String, Value> {
    tools
        .iter()
        .map(|t| (t["name"].as_str().unwrap().to_string(), t.clone()))
        .collect()
}

/// The JSON types a property schema allows, from `type` (string or array)
/// or `anyOf`, e.g. `{"string", "null"}`.
fn types(schema: &Value) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    match &schema["type"] {
        Value::String(t) => {
            out.insert(t.clone());
        }
        Value::Array(ts) => out.extend(ts.iter().filter_map(|t| t.as_str()).map(String::from)),
        _ => {}
    }
    if let Some(any) = schema["anyOf"].as_array() {
        for alt in any {
            out.extend(types(alt));
        }
    }
    out
}

/// The item types of an array-typed property (from `items` or `anyOf`).
fn item_types(schema: &Value) -> BTreeSet<String> {
    let mut out = types(&schema["items"]);
    if let Some(any) = schema["anyOf"].as_array() {
        for alt in any {
            out.extend(item_types(alt));
        }
    }
    out
}

fn properties(schema: &Value) -> BTreeMap<String, Value> {
    schema["properties"]
        .as_object()
        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}

fn required(schema: &Value) -> BTreeSet<String> {
    schema["required"]
        .as_array()
        .map(|r| {
            r.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Every difference between a registered tool's schema and the fixture's.
fn schema_diffs(name: &str, ours: &Value, theirs: &Value) -> Vec<String> {
    let mut diffs = Vec::new();
    let (ours, theirs) = (&ours["inputSchema"], &theirs["inputSchema"]);
    let (op, tp) = (properties(ours), properties(theirs));
    let (ok, tk): (BTreeSet<_>, BTreeSet<_>) = (op.keys().collect(), tp.keys().collect());
    if ok != tk {
        diffs.push(format!("{name}: properties {ok:?} != fixture {tk:?}"));
    }
    if required(ours) != required(theirs) {
        diffs.push(format!(
            "{name}: required {:?} != fixture {:?}",
            required(ours),
            required(theirs)
        ));
    }
    for (prop, t_schema) in &tp {
        let Some(o_schema) = op.get(prop) else {
            continue;
        };
        if types(o_schema) != types(t_schema) {
            diffs.push(format!(
                "{name}.{prop}: types {:?} != fixture {:?}",
                types(o_schema),
                types(t_schema)
            ));
        }
        if item_types(o_schema) != item_types(t_schema) {
            diffs.push(format!(
                "{name}.{prop}: item types {:?} != fixture {:?}",
                item_types(o_schema),
                item_types(t_schema)
            ));
        }
    }
    diffs
}

#[test]
fn tools_match_python_contract() {
    let fixture = fixture();
    let expected = by_name(fixture["tools"].as_array().unwrap());

    let server = httpmock::MockServer::start();
    let mut client = common::McpBuilder::new(&server).start();
    let registered = by_name(&client.list_tools());

    let mut problems = Vec::new();
    for (name, tool) in &registered {
        match expected.get(name) {
            None => problems.push(format!("{name}: registered but not in the fixture")),
            Some(theirs) => problems.extend(schema_diffs(name, tool, theirs)),
        }
        if PENDING_TOOLS.contains(&name.as_str()) {
            problems.push(format!(
                "{name}: registered, so remove it from PENDING_TOOLS"
            ));
        }
        if tool["description"].as_str().unwrap_or("").trim().is_empty() {
            problems.push(format!("{name}: missing description"));
        }
    }
    for name in expected.keys() {
        if !registered.contains_key(name) && !PENDING_TOOLS.contains(&name.as_str()) {
            problems.push(format!(
                "{name}: in the fixture but neither registered nor in PENDING_TOOLS"
            ));
        }
    }
    for name in PENDING_TOOLS {
        if !expected.contains_key(*name) {
            problems.push(format!("{name}: in PENDING_TOOLS but not in the fixture"));
        }
    }
    assert!(
        problems.is_empty(),
        "contract mismatches:\n{}",
        problems.join("\n")
    );
}

#[test]
fn resources_match_python_contract() {
    let fixture = fixture();
    let expected: BTreeSet<(String, String)> = fixture["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["uri"].as_str().unwrap().to_string(),
                r["name"].as_str().unwrap().to_string(),
            )
        })
        .collect();

    let server = httpmock::MockServer::start();
    let mut client = common::McpBuilder::new(&server).start();
    let resources = client.list_resources();
    let got: BTreeSet<(String, String)> = resources
        .iter()
        .map(|r| {
            (
                r["uri"].as_str().unwrap().to_string(),
                r["name"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(got, expected);
    assert!(
        resources
            .iter()
            .all(|r| !r["description"].as_str().unwrap_or("").is_empty())
    );
}

#[test]
fn schema_normalizer_handles_both_null_styles() {
    let python = serde_json::json!({"anyOf": [{"type": "string"}, {"type": "null"}]});
    let rust = serde_json::json!({"type": ["string", "null"]});
    assert_eq!(types(&python), types(&rust));
    let python = serde_json::json!({"anyOf": [{"type": "array", "items": {"type": "string"}}, {"type": "null"}]});
    let rust = serde_json::json!({"type": ["array", "null"], "items": {"type": "string"}});
    assert_eq!(item_types(&python), item_types(&rust));
}
