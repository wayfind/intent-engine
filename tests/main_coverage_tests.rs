// Tests in this file use CLI commands removed in v0.10.0
// v0.10.0 simplified CLI to just: plan, log, search
// These tests are kept for reference but disabled by default
#![cfg(feature = "test-removed-cli-commands")]

//! Comprehensive tests for main.rs to improve code coverage
//! Focuses on error paths and edge cases that are difficult to trigger in normal usage

mod common;

use predicates::prelude::*;
use serde_json::Value;
use std::fs;

// ============================================================================
// Session Restore Tests
// ============================================================================

#[test]
fn test_session_restore_without_workspace() {
    // Don't use setup_test_env() here because it initializes the workspace
    // We want to test the case where there is NO workspace
    let temp_dir = tempfile::TempDir::new().unwrap();
    fs::create_dir(temp_dir.path().join(".git")).unwrap();

    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .arg("session-restore")
        .arg("--include-events")
        .arg("5");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No Intent-Engine workspace found"));
}

#[test]
fn test_session_restore_with_workspace_path() {
    let temp_dir = common::setup_test_env();

    // Initialize workspace
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Try session restore with explicit workspace path
    let mut cmd = common::ie_command();
    cmd.arg("session-restore")
        .arg("--include-events")
        .arg("3")
        .arg("--workspace")
        .arg(temp_dir.path());

    cmd.assert().success();
}

#[test]
fn test_session_restore_with_nonexistent_workspace_path() {
    let temp_dir = common::setup_test_env();
    let nonexistent = temp_dir.path().join("nonexistent");

    let mut cmd = common::ie_command();
    cmd.arg("session-restore")
        .arg("--workspace")
        .arg(&nonexistent);

    // Should fail to change directory
    cmd.assert().failure();
}

// ============================================================================
// Event Command Error Path Tests
// ============================================================================

#[test]
fn test_event_add_without_data_stdin_flag() {
    let temp_dir = common::setup_test_env();

    // Initialize and create a task
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Try to add event without --data-stdin
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .arg("event")
        .arg("add")
        .arg("--log-type")
        .arg("note");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail with InvalidInput error
    assert!(
        stderr.contains("--data-stdin is required") || !output.status.success(),
        "Expected error about missing --data-stdin, got: {}",
        stderr
    );
}

#[test]
fn test_event_add_without_current_task_and_without_task_id() {
    let temp_dir = common::setup_test_env();

    // Initialize workspace but don't set current task
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Try to add event without task_id and without current task
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .arg("event")
        .arg("add")
        .arg("--log-type")
        .arg("note")
        .arg("--data-stdin")
        .write_stdin("test event");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail with error about no current task
    assert!(
        stderr.contains("No current task is set") || !output.status.success(),
        "Expected error about no current task, got: {}",
        stderr
    );
}

// ============================================================================
// Doctor Command Error Paths
// ============================================================================

#[test]
fn test_doctor_in_fresh_environment() {
    let temp_dir = common::setup_test_env();

    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path()).arg("doctor");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Database:"))
        .stdout(predicate::str::contains(
            "Ancestor directories with databases:",
        ))
        .stdout(predicate::str::contains("Dashboard:"));
}

// ============================================================================
// Task Command Edge Cases
// ============================================================================

#[test]
fn test_task_update_with_priority() {
    let temp_dir = common::setup_test_env();

    // Add a task
    let output = common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .output()
        .unwrap();

    assert!(output.status.success());

    // Update with priority
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("update")
        .arg("1")
        .arg("--priority")
        .arg("high");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"priority\": 2"));
}

#[test]
fn test_task_delete() {
    let temp_dir = common::setup_test_env();

    // Add a task
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Delete the task
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("del")
        .arg("1");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("deleted"));
}

#[test]
fn test_task_list_with_parent_filter() {
    let temp_dir = common::setup_test_env();

    // Add parent task
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Parent task")
        .assert()
        .success();

    // Add child task
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Child task")
        .arg("--parent")
        .arg("1")
        .assert()
        .success();

    // List with parent filter
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("list")
        .arg("--parent")
        .arg("1");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Child task"));
}

#[test]
fn test_task_list_with_null_parent() {
    let temp_dir = common::setup_test_env();

    // Add parent task
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Parent task")
        .assert()
        .success();

    // Add child task
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Child task")
        .arg("--parent")
        .arg("1")
        .assert()
        .success();

    // List with null parent filter (only top-level tasks)
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("list")
        .arg("--parent")
        .arg("null");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Parent task"));
    assert!(!stdout.contains("Child task"));
}

#[test]
fn test_task_pick_next_text_format() {
    let temp_dir = common::setup_test_env();

    // Add a task
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Pick next with text format
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("text");

    cmd.assert().success();
}

#[test]
fn test_task_pick_next_json_format() {
    let temp_dir = common::setup_test_env();

    // Add a task
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Pick next with json format
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json");

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The JSON response uses "task" not "recommended_task"
    assert!(stdout.contains("\"task\""));
}

// ============================================================================
// Current Command Tests
// ============================================================================

#[test]
fn test_current_get_when_no_current_task() {
    let temp_dir = common::setup_test_env();

    // Initialize workspace
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Get current task (should be null)
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path()).arg("current");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"current_task_id\": null"));
}

#[test]
fn test_current_set_and_get() {
    let temp_dir = common::setup_test_env();

    // Add a task
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Set current task
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("current")
        .arg("--set")
        .arg("1")
        .assert()
        .success();

    // Get current task
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path()).arg("current");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"current_task_id\": 1"));
}

// ============================================================================
// Report Command Tests
// ============================================================================

#[test]
fn test_report_with_filters() {
    let temp_dir = common::setup_test_env();

    // Add tasks
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Generate report with status filter
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .arg("report")
        .arg("--status")
        .arg("todo");

    cmd.assert().success();
}

#[test]
fn test_report_summary_only() {
    let temp_dir = common::setup_test_env();

    // Add tasks
    common::ie_command()
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Generate summary-only report
    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .arg("report")
        .arg("--summary-only");

    cmd.assert().success();
}
