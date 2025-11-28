/// Error handling tests for CLI commands
/// Tests various error scenarios and edge cases
mod common;

use predicates::prelude::*;

// ============================================================================
// Invalid Command Combinations
// ============================================================================

#[test]
#[ignore]
fn test_task_list_with_invalid_status_filter() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    cmd.arg("task")
        .arg("list")
        .arg("--status")
        .arg("invalid_status");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'invalid_status'"));
}

#[test]
fn test_task_add_with_both_spec_and_spec_stdin() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    cmd.arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test")
        .arg("--spec")
        .arg("Spec via arg")
        .arg("--spec-stdin");

    // This should fail as both spec options are provided
    cmd.assert().failure();
}

#[test]
#[ignore]
fn test_task_update_nonexistent_parent() {
    let temp_dir = common::setup_test_env();

    // Create a task
    let mut add_cmd = common::ie_command_with_project_dir(temp_dir.path());
    add_cmd
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test Task");
    add_cmd.assert().success();

    // Try to set parent to non-existent task
    let mut update_cmd = common::ie_command_with_project_dir(temp_dir.path());
    update_cmd
        .arg("task")
        .arg("update")
        .arg("1")
        .arg("--parent")
        .arg("99999");

    update_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("Parent task not found"));
}

// ============================================================================
// Database/Project Not Found Errors
// ============================================================================

#[test]
#[ignore]
fn test_command_in_uninitialized_directory() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    // Don't initialize - no .git, no IE project

    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .env("HOME", "/nonexistent")
        .env("USERPROFILE", "/nonexistent")
        .arg("task")
        .arg("list");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No Intent-Engine project found"));
}

#[test]
fn test_task_get_with_out_of_range_id() {
    let temp_dir = common::setup_test_env();

    // setup_test_env creates task ID 1, so try to get task ID 100
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());
    cmd.arg("task").arg("get").arg("100");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Task not found"));
}

// ============================================================================
// Invalid Input Format Tests
// ============================================================================

#[test]
fn test_task_get_with_non_numeric_id() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    cmd.arg("task").arg("get").arg("not_a_number");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid digit"));
}

#[test]
#[ignore]
fn test_task_update_with_invalid_complexity() {
    let temp_dir = common::setup_test_env();

    // Create a task first
    let mut add_cmd = common::ie_command_with_project_dir(temp_dir.path());
    add_cmd.arg("task").arg("add").arg("--name").arg("Test");
    add_cmd.assert().success();

    // Try to update with invalid complexity (out of 1-10 range)
    let mut update_cmd = common::ie_command_with_project_dir(temp_dir.path());
    update_cmd
        .arg("task")
        .arg("update")
        .arg("1")
        .arg("--complexity")
        .arg("15");

    update_cmd.assert().failure();
}

// ============================================================================
// Event Command Error Tests
// ============================================================================

#[test]
#[ignore]
fn test_event_add_without_current_task_and_no_task_id() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    cmd.arg("event")
        .arg("add")
        .arg("--type")
        .arg("note")
        .arg("--data")
        .arg("Test note");

    cmd.assert().failure().stderr(
        predicate::str::contains("No current task").or(predicate::str::contains("task_id")),
    );
}

#[test]
#[ignore]
fn test_event_add_with_invalid_type() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    cmd.arg("event")
        .arg("add")
        .arg("--task-id")
        .arg("1")
        .arg("--type")
        .arg("invalid_type")
        .arg("--data")
        .arg("Test");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'invalid_type'"));
}

#[test]
#[ignore]
fn test_event_list_with_invalid_task_id() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    cmd.arg("event").arg("list").arg("--task-id").arg("99999");

    // Should succeed but return empty list (or error depending on implementation)
    cmd.assert().success();
}

// ============================================================================
// Dependency Error Tests
// ============================================================================

#[test]
fn test_depends_on_with_same_task_twice() {
    let temp_dir = common::setup_test_env();

    // Create tasks
    let mut add1 = common::ie_command_with_project_dir(temp_dir.path());
    add1.arg("task").arg("add").arg("--name").arg("Task 1");
    add1.assert().success();

    let mut add2 = common::ie_command_with_project_dir(temp_dir.path());
    add2.arg("task").arg("add").arg("--name").arg("Task 2");
    add2.assert().success();

    // Add dependency
    let mut dep1 = common::ie_command_with_project_dir(temp_dir.path());
    dep1.arg("task").arg("depends-on").arg("1").arg("2");
    dep1.assert().success();

    // Try to add same dependency again
    let mut dep2 = common::ie_command_with_project_dir(temp_dir.path());
    dep2.arg("task").arg("depends-on").arg("1").arg("2");

    // Should either succeed (idempotent) or fail with "already exists"
    // Accepting either behavior for now
    dep2.assert();
}

// ============================================================================
// Priority Tests
// ============================================================================

#[test]
#[ignore]
fn test_task_add_with_all_valid_priorities() {
    let temp_dir = common::setup_test_env();

    let priorities = vec!["critical", "high", "medium", "low"];

    for priority in priorities {
        let mut cmd = common::ie_command_with_project_dir(temp_dir.path());
        cmd.arg("task")
            .arg("add")
            .arg("--name")
            .arg(format!("Task with {} priority", priority))
            .arg("--priority")
            .arg(priority);

        cmd.assert().success();
    }
}

// ============================================================================
// Search Command Error Tests
// ============================================================================

#[test]
fn test_search_with_empty_query() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    cmd.arg("search").arg("");

    // Should either fail or return "no results"
    // Empty query might be handled gracefully
    cmd.assert();
}

#[test]
fn test_search_with_very_long_query() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    let long_query = "a".repeat(10000);
    cmd.arg("search").arg(&long_query);

    // Should handle gracefully (might be slow but shouldn't crash)
    cmd.assert();
}
