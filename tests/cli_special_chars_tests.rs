#![allow(deprecated)]

use assert_cmd::cargo;
/// CLI integration tests for special characters
///
/// Tests that special characters work correctly through the CLI interface
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    // Create a .git marker to prevent falling back to home project
    fs::create_dir(temp_dir.path().join(".git")).unwrap();

    // Initialize the project by adding a dummy task (triggers auto-init)
    // Prevent fallback to home by setting HOME to nonexistent directory
    let mut init_cmd = Command::new(cargo::cargo_bin!("ie"));
    init_cmd
        .current_dir(temp_dir.path())
        .env("HOME", "/nonexistent")  // Prevent fallback to home
        .env("USERPROFILE", "/nonexistent")  // Windows equivalent
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Setup task")
        .assert()
        .success();

    temp_dir
}

#[test]
#[ignore = "Test uses removed CLI commands (v0.10.0 simplified to plan/log/search)"]
fn test_cli_unicode_task_name() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("实现用户认证功能");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("实现用户认证功能"));
}

#[test]
#[ignore = "Test uses removed CLI commands (v0.10.0 simplified to plan/log/search)"]
fn test_cli_emoji_task_name() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("🚀 Deploy to production 🎉");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("🚀 Deploy to production 🎉"));
}

#[test]
#[ignore = "Test uses removed CLI commands (v0.10.0 simplified to plan/log/search)"]
fn test_cli_quotes_in_task_name() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg(r#"Task with "quoted" text"#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#"Task with \"quoted\" text"#));
}

#[test]
#[ignore = "Test uses removed CLI commands (v0.10.0 simplified to plan/log/search)"]
fn test_cli_multiline_spec() {
    let temp_dir = setup_test_env();

    let multiline_spec = "Line 1\nLine 2\nLine 3";

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test")
        .arg("--spec-stdin")
        .write_stdin(multiline_spec);

    cmd.assert().success();
}

#[test]
#[ignore = "Test uses removed CLI commands (v0.10.0 simplified to plan/log/search)"]
fn test_cli_special_chars_in_event() {
    let temp_dir = setup_test_env();

    // Create task first
    let mut add_cmd = Command::new(cargo::cargo_bin!("ie"));
    add_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test")
        .assert()
        .success();

    // Add event with special characters
    let mut event_cmd = Command::new(cargo::cargo_bin!("ie"));
    event_cmd
        .current_dir(temp_dir.path())
        .arg("event")
        .arg("add")
        .arg("--task-id")
        .arg("1")
        .arg("--type")
        .arg("decision")
        .arg("--data-stdin")
        .write_stdin("Decision with <tags> & special chars");

    event_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Decision with"));
}

#[test]
#[ignore = "Test uses removed CLI commands (v0.10.0 simplified to plan/log/search)"]
fn test_cli_very_long_task_name() {
    let temp_dir = setup_test_env();

    let long_name = "A".repeat(1000);

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg(&long_name);

    cmd.assert().success();
}

#[test]
#[ignore = "Test uses removed CLI commands (v0.10.0 simplified to plan/log/search)"]
fn test_cli_url_in_task_name() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Deploy to https://example.com/api?key=value&test=1");

    cmd.assert().success();
}

#[test]
#[ignore = "Test uses removed CLI commands (v0.10.0 simplified to plan/log/search)"]
fn test_cli_shell_metacharacters() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task && echo test | grep bad");

    cmd.assert().success();
}

#[test]
#[ignore = "Test uses removed CLI commands (v0.10.0 simplified to plan/log/search)"]
fn test_cli_markdown_in_name() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("# Header **bold** *italic*");

    cmd.assert().success();
}

#[test]
#[ignore = "Test uses removed CLI commands (v0.10.0 simplified to plan/log/search)"]
fn test_cli_backslash_path() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::new(cargo::cargo_bin!("ie"));
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg(r"C:\Users\test\path");

    cmd.assert().success();
}
