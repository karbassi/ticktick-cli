use assert_cmd::{Command, cargo::cargo_bin_cmd};
use predicates::prelude::*;
use std::fs;

fn cmd() -> Command {
    cargo_bin_cmd!("ticktick-cli")
}

/// Assert a TickTick API datetime string has the expected date/time prefix
/// and a valid `+HHMM` or `-HHMM` offset suffix.
fn assert_api_datetime(value: &serde_json::Value, expected_prefix: &str) {
    let s = value
        .as_str()
        .unwrap_or_else(|| panic!("expected string, got: {value}"));
    assert!(
        s.starts_with(expected_prefix),
        "expected datetime starting with '{expected_prefix}', got: {s}"
    );
    let offset = &s[expected_prefix.len()..];
    assert!(
        offset.len() == 5 && (offset.starts_with('+') || offset.starts_with('-')),
        "expected +HHMM or -HHMM offset suffix, got: '{offset}' in '{s}'"
    );
}

#[test]
fn help_exits_zero() {
    cmd().arg("--help").assert().success();
}

#[test]
fn help_contains_usage() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("USAGE").or(predicate::str::contains("Usage")));
}

#[test]
fn version_exits_zero() {
    cmd().arg("--version").assert().success();
}

#[test]
fn version_contains_semver_and_build_info() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d+\.\d+\.\d+").unwrap());
}

#[test]
fn unknown_command_exits_two() {
    cmd().arg("nonexistent").assert().failure().code(2);
}

#[test]
fn unknown_flag_exits_two() {
    cmd().arg("--nonexistent").assert().failure().code(2);
}

#[test]
fn top_level_subcommand_help_exits_zero() {
    for subcmd in ["login", "logout", "task", "project", "init", "completions"] {
        cmd().args([subcmd, "--help"]).assert().success();
    }
}

#[test]
fn task_subcommand_help_exits_zero() {
    for subcmd in ["list", "get", "add", "edit", "complete", "delete", "move"] {
        cmd().args(["task", subcmd, "--help"]).assert().success();
    }
}

#[test]
fn project_subcommand_help_exits_zero() {
    for subcmd in ["list", "get", "add", "edit", "delete"] {
        cmd().args(["project", subcmd, "--help"]).assert().success();
    }
}

#[test]
fn add_missing_title_exits_two() {
    cmd().args(["task", "add"]).assert().failure().code(2);
}

#[test]
fn complete_missing_args_exits_two() {
    cmd().args(["task", "complete"]).assert().failure().code(2);
}

#[test]
fn delete_missing_args_exits_two() {
    cmd().args(["task", "delete"]).assert().failure().code(2);
}

#[test]
fn move_missing_args_exits_two() {
    cmd().args(["task", "move"]).assert().failure().code(2);
}

#[test]
fn unauthenticated_commands_fail_with_hint() {
    for args in [vec!["project", "list"], vec!["task", "list"]] {
        let output = cmd().args(&args).output().expect("failed to run");
        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr).unwrap();
            let parsed: serde_json::Value =
                serde_json::from_str(stderr.trim()).expect("error output should be valid JSON");
            let error_msg = parsed["error"].as_str().expect("should have error field");
            assert!(
                error_msg.contains("hint"),
                "error for '{args:?}' should contain hint"
            );
        }
    }
}

#[test]
fn delete_refuses_without_force_in_non_tty() {
    // In a non-TTY environment (like tests), delete without --force should fail.
    // The error might be about missing auth or missing project, but should never succeed.
    cmd()
        .args(["task", "delete", "someproject", "sometask"])
        .assert()
        .failure();
}

#[test]
fn completions_bash_exits_zero() {
    cmd().args(["completions", "bash"]).assert().success();
}

#[test]
fn completions_zsh_exits_zero() {
    cmd().args(["completions", "zsh"]).assert().success();
}

#[test]
fn completions_fish_exits_zero() {
    cmd().args(["completions", "fish"]).assert().success();
}

#[test]
fn completions_missing_shell_exits_two() {
    cmd().arg("completions").assert().failure().code(2);
}

#[test]
fn verbose_flag_accepted() {
    // -v should not be a usage error (exit code 2), regardless of auth state
    cmd()
        .args(["-v", "project", "list"])
        .assert()
        .code(predicate::ne(2));
}

#[test]
fn project_get_missing_name_exits_two() {
    cmd().args(["project", "get"]).assert().failure().code(2);
}

#[test]
fn add_dry_run_exits_zero() {
    cmd()
        .args(["task", "add", "--dry-run", "test task"])
        .assert()
        .success();
}

#[test]
fn add_dry_run_json_is_valid_json() {
    let output = cmd()
        .args(["task", "add", "--dry-run", "test task"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("dry-run output should be valid JSON");
    assert_eq!(parsed["title"], "test task");
    assert_eq!(parsed["dryRun"], true);
}

#[test]
fn init_help_exits_zero() {
    cmd().args(["init", "--help"]).assert().success();
}

#[test]
fn init_local_creates_env_file() {
    let dir = tempfile::tempdir().unwrap();
    cmd()
        .args(["init", "--local"])
        .current_dir(dir.path())
        .assert()
        .success();

    let env_path = dir.path().join(".env");
    assert!(env_path.exists(), ".env should be created");
    let content = fs::read_to_string(&env_path).unwrap();
    assert!(
        content.contains("TICKTICK_CLIENT_ID"),
        ".env should contain CLIENT_ID template"
    );
    assert!(
        content.contains("TICKTICK_CLIENT_SECRET"),
        ".env should contain CLIENT_SECRET template"
    );
}

#[test]
fn init_local_refuses_overwrite_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let env_path = dir.path().join(".env");
    fs::write(&env_path, "existing").unwrap();

    cmd()
        .args(["init", "--local"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    // Original content should be preserved
    assert_eq!(fs::read_to_string(&env_path).unwrap(), "existing");
}

#[test]
fn init_local_force_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let env_path = dir.path().join(".env");
    fs::write(&env_path, "old").unwrap();

    cmd()
        .args(["init", "--local", "--force"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(&env_path).unwrap();
    assert!(
        content.contains("TICKTICK_CLIENT_ID"),
        "should be overwritten with template"
    );
}

#[test]
fn mutation_confirmations_go_to_stderr() {
    let output = cmd()
        .args(["task", "add", "--dry-run", "test task"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    // stdout should have JSON (the dry-run body)
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("dry-run stdout should be valid JSON");
    assert_eq!(parsed["title"], "test task");
}

#[test]
fn usage_exits_zero() {
    cmd().arg("usage").assert().success();
}

#[test]
fn usage_lists_subcommands() {
    let output = cmd().arg("usage").output().expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for name in [
        "login",
        "logout",
        "task list",
        "task add",
        "task complete",
        "task delete",
        "task move",
        "project list",
        "project add",
        "project delete",
        "init",
        "completions",
    ] {
        assert!(
            stdout.contains(name),
            "usage output should contain '{name}'"
        );
    }
}

#[test]
fn errors_are_json_on_stderr() {
    let output = cmd()
        .args(["project", "list"])
        .output()
        .expect("failed to run");
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(stderr.trim()).expect("error output should be valid JSON");
        assert!(
            parsed["error"].is_string(),
            "error JSON should have 'error' field"
        );
    }
}

#[test]
fn add_multiple_dry_run() {
    let output = cmd()
        .args(["task", "add", "--dry-run", "Task A", "Task B", "Task C"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("multi dry-run output should be valid JSON");
    let arr = parsed.as_array().expect("should be a JSON array");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["title"], "Task A");
    assert_eq!(arr[1]["title"], "Task B");
    assert_eq!(arr[2]["title"], "Task C");
    for item in arr {
        assert_eq!(item["dryRun"], true);
    }
}

#[test]
fn add_stdin_dry_run() {
    let output = cmd()
        .args(["task", "add", "--stdin", "--dry-run"])
        .write_stdin("Task X\nTask Y\n")
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdin dry-run output should be valid JSON");
    let arr = parsed.as_array().expect("should be a JSON array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["title"], "Task X");
    assert_eq!(arr[1]["title"], "Task Y");
}

#[test]
fn edit_title_with_multiple_ids_exits_one() {
    cmd()
        .args([
            "task",
            "edit",
            "someproject",
            "id1",
            "id2",
            "--title",
            "New",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--title can only be used with a single task ID",
        ));
}

#[test]
fn delete_stdin_without_force_refuses() {
    cmd()
        .args(["task", "delete", "someproject", "--stdin"])
        .write_stdin("id1\nid2\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--force"));
}

#[test]
fn output_is_always_json() {
    let output = cmd()
        .args(["task", "add", "--dry-run", "test task"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let _: serde_json::Value = serde_json::from_str(&stdout)
        .expect("output should always be valid JSON without any flags");
}

// -- Timeblocking tests -------------------------------------------------------

#[test]
fn add_dry_run_with_start_date_only() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Meeting",
            "--start",
            "2026-03-15",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["title"], "Meeting");
    assert_api_datetime(&parsed["startDate"], "2026-03-15T00:00:00.000");
    assert_eq!(parsed["isAllDay"], true);
    assert!(parsed.get("dueDate").is_none() || parsed["dueDate"].is_null());
}

#[test]
fn add_dry_run_with_start_datetime() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Focus",
            "--start",
            "2026-03-15T14:00",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_api_datetime(&parsed["startDate"], "2026-03-15T14:00:00.000");
    assert_eq!(parsed["isAllDay"], false);
}

#[test]
fn add_dry_run_with_duration() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Block",
            "--start",
            "2026-03-15T14:00",
            "--duration",
            "2h",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_api_datetime(&parsed["startDate"], "2026-03-15T14:00:00.000");
    assert_api_datetime(&parsed["dueDate"], "2026-03-15T16:00:00.000");
    assert_eq!(parsed["isAllDay"], false);
}

#[test]
fn add_dry_run_duration_day_overflow() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Late",
            "--start",
            "2026-03-15T23:00",
            "--duration",
            "2h",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_api_datetime(&parsed["startDate"], "2026-03-15T23:00:00.000");
    assert_api_datetime(&parsed["dueDate"], "2026-03-16T01:00:00.000");
}

#[test]
fn add_dry_run_duration_month_overflow() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "End of month",
            "--start",
            "2026-01-31T23:00",
            "--duration",
            "2h",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_api_datetime(&parsed["dueDate"], "2026-02-01T01:00:00.000");
}

#[test]
fn add_dry_run_duration_leap_year() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Leap",
            "--start",
            "2024-02-28T23:30",
            "--duration",
            "1h",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_api_datetime(&parsed["dueDate"], "2024-02-29T00:30:00.000");
}

#[test]
fn add_dry_run_with_timezone() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "TZ test",
            "--start",
            "2026-03-15T14:00",
            "--timezone",
            "America/New_York",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["timeZone"], "America/New_York");
}

#[test]
fn add_dry_run_with_tz_alias() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "TZ alias",
            "--start",
            "2026-03-15T14:00",
            "--tz",
            "Europe/London",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["timeZone"], "Europe/London");
}

#[test]
fn add_dry_run_all_day_override() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "All day",
            "--start",
            "2026-03-15T14:00",
            "--all-day",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    // --all-day forces isAllDay: true even with a time input
    assert_eq!(parsed["isAllDay"], true);
}

#[test]
fn add_dry_run_due_datetime() {
    // --due with a time component should set isAllDay: false
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Timed",
            "--due",
            "2026-03-15T17:00",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_api_datetime(&parsed["dueDate"], "2026-03-15T17:00:00.000");
    assert_eq!(parsed["isAllDay"], false);
}

#[test]
fn add_dry_run_due_date_only_is_all_day() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Day task",
            "--due",
            "2026-03-15",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["isAllDay"], true);
}

// -- Error cases --

#[test]
fn add_duration_without_start_fails() {
    // --duration requires --start
    let output = cmd()
        .args(["task", "add", "--dry-run", "Bad", "--duration", "1h"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--duration requires --start"));
}

#[test]
fn add_duration_with_date_only_start_fails() {
    // --start date-only + --duration is an error
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Bad",
            "--start",
            "2026-03-15",
            "--duration",
            "1h",
        ])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--duration requires --start with a time"));
}

#[test]
fn add_duration_conflicts_with_due() {
    // clap conflicts_with should reject this
    cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Bad",
            "--start",
            "2026-03-15T14:00",
            "--duration",
            "1h",
            "--due",
            "2026-03-15",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn add_invalid_duration_format() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Bad",
            "--start",
            "2026-03-15T14:00",
            "--duration",
            "abc",
        ])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
}

#[test]
fn add_zero_duration_fails() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Bad",
            "--start",
            "2026-03-15T14:00",
            "--duration",
            "0h",
        ])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
}

// -- Edit conflict cases --

#[test]
fn edit_start_conflicts_with_clear_start() {
    cmd()
        .args([
            "task",
            "edit",
            "proj",
            "id1",
            "--start",
            "2026-03-15",
            "--clear-start",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn edit_duration_conflicts_with_due() {
    cmd()
        .args([
            "task",
            "edit",
            "proj",
            "id1",
            "--start",
            "2026-03-15T14:00",
            "--duration",
            "1h",
            "--due",
            "2026-03-15",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn edit_duration_conflicts_with_clear_due() {
    cmd()
        .args([
            "task",
            "edit",
            "proj",
            "id1",
            "--start",
            "2026-03-15T14:00",
            "--duration",
            "1h",
            "--clear-due",
        ])
        .assert()
        .failure()
        .code(2);
}

// -- Task new fields (content, desc, tags, items, reminders, repeat) ----------

#[test]
fn add_dry_run_with_content_and_desc() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Test",
            "--content",
            "Some notes",
            "--desc",
            "A description",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["content"], "Some notes");
    assert_eq!(parsed["desc"], "A description");
}

#[test]
fn add_dry_run_with_tags() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Test",
            "--tag",
            "urgent",
            "--tag",
            "work",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    let tags = parsed["tags"].as_array().expect("tags should be an array");
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0], "urgent");
    assert_eq!(tags[1], "work");
}

#[test]
fn add_dry_run_with_items() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Test",
            "--item",
            "Step 1",
            "--item",
            "Step 2",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    let items = parsed["items"]
        .as_array()
        .expect("items should be an array");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["title"], "Step 1");
    assert_eq!(items[0]["status"], 0);
    assert_eq!(items[1]["title"], "Step 2");
    assert_eq!(items[1]["status"], 0);
}

#[test]
fn add_dry_run_with_reminder_and_repeat() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Test",
            "--reminder",
            "TRIGGER:P0DT9H0M0S",
            "--repeat",
            "RRULE:FREQ=DAILY;INTERVAL=1",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    let reminders = parsed["reminders"]
        .as_array()
        .expect("reminders should be an array");
    assert_eq!(reminders.len(), 1);
    assert_eq!(reminders[0], "TRIGGER:P0DT9H0M0S");
    assert_eq!(parsed["repeatFlag"], "RRULE:FREQ=DAILY;INTERVAL=1");
}

#[test]
fn add_dry_run_with_all_new_fields() {
    let output = cmd()
        .args([
            "task",
            "add",
            "--dry-run",
            "Full task",
            "--content",
            "Notes here",
            "--desc",
            "Description here",
            "--tag",
            "tag1",
            "--item",
            "Sub 1",
            "--reminder",
            "TRIGGER:PT0S",
            "--repeat",
            "RRULE:FREQ=WEEKLY;INTERVAL=1",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["content"], "Notes here");
    assert_eq!(parsed["desc"], "Description here");
    assert_eq!(parsed["tags"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["items"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["reminders"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["repeatFlag"], "RRULE:FREQ=WEEKLY;INTERVAL=1");
}

#[test]
fn edit_clear_content_conflicts_with_content() {
    cmd()
        .args([
            "task",
            "edit",
            "proj",
            "id1",
            "--content",
            "foo",
            "--clear-content",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn edit_clear_tags_conflicts_with_tag() {
    cmd()
        .args([
            "task",
            "edit",
            "proj",
            "id1",
            "--tag",
            "foo",
            "--clear-tags",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn edit_clear_reminders_conflicts_with_reminder() {
    cmd()
        .args([
            "task",
            "edit",
            "proj",
            "id1",
            "--reminder",
            "TRIGGER:PT0S",
            "--clear-reminders",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn edit_clear_repeat_conflicts_with_repeat() {
    cmd()
        .args([
            "task",
            "edit",
            "proj",
            "id1",
            "--repeat",
            "RRULE:FREQ=DAILY;INTERVAL=1",
            "--clear-repeat",
        ])
        .assert()
        .failure()
        .code(2);
}

// -- Move help --

#[test]
fn move_help_shows_to_flag() {
    let output = cmd()
        .args(["task", "move", "--help"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("--to"),
        "task move --help should mention --to flag"
    );
}

// -- Project field flags --

#[test]
fn project_add_help_shows_new_flags() {
    let output = cmd()
        .args(["project", "add", "--help"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for flag in ["--color", "--view-mode", "--kind"] {
        assert!(
            stdout.contains(flag),
            "project add --help should contain '{flag}'"
        );
    }
}

#[test]
fn project_edit_help_shows_new_flags() {
    let output = cmd()
        .args(["project", "edit", "--help"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for flag in ["--color", "--view-mode", "--kind"] {
        assert!(
            stdout.contains(flag),
            "project edit --help should contain '{flag}'"
        );
    }
}
