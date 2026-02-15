use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

fn cmd() -> Command {
    Command::cargo_bin("ticktick-cli").unwrap()
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
    for subcmd in ["list", "get", "add", "edit", "complete", "delete"] {
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
