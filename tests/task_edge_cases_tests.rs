// Tests in this file use CLI commands removed in v0.10.0
// v0.10.0 simplified CLI to just: plan, log, search
// These tests are kept for reference but disabled by default
#![cfg(feature = "test-removed-cli-commands")]

/// Edge case tests for task commands
/// Focuses on error handling and boundary conditions
mod common;

use predicates::prelude::*;

// ============================================================================
// Invalid Task ID Tests
// ============================================================================

#[test]
fn test_task_get_nonexistent_id() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());
    cmd.arg("task").arg("get").arg("99999");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Task not found"));
}

#[test]
fn test_task_update_nonexistent_id() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    cmd.arg("task")
        .arg("update")
        .arg("99999")
        .arg("--name")
        .arg("New Name");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Task not found"));
}

#[test]
fn test_task_delete_nonexistent_id() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());
    cmd.arg("task").arg("del").arg("99999");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Task not found"));
}

#[test]
fn test_task_start_nonexistent_id() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());
    cmd.arg("task").arg("start").arg("99999");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Task not found"));
}

// ============================================================================
// Empty/Invalid Input Tests
// ============================================================================

#[test]
#[ignore]
fn test_task_add_whitespace_only_name() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    cmd.arg("task").arg("add").arg("--name").arg("   ");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Task name cannot be empty"));
}

#[test]
#[ignore]
fn test_task_update_empty_name() {
    let temp_dir = common::setup_test_env();

    // First create a task
    let mut add_cmd = common::ie_command_with_project_dir(temp_dir.path());
    add_cmd
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test Task");
    add_cmd.assert().success();

    // Try to update with empty name
    let mut update_cmd = common::ie_command_with_project_dir(temp_dir.path());
    update_cmd
        .arg("task")
        .arg("update")
        .arg("1")
        .arg("--name")
        .arg("");

    update_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task name cannot be empty"));
}

// ============================================================================
// Task State Transition Tests
// ============================================================================

#[test]
#[ignore]
fn test_task_done_without_current_task() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    cmd.arg("task").arg("done");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No task is currently focused"));
}

#[test]
#[ignore]
fn test_task_done_with_uncompleted_children_via_cli() {
    let temp_dir = common::setup_test_env();

    // Create parent task
    let mut add_parent = common::ie_command_with_project_dir(temp_dir.path());
    add_parent
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Parent Task");
    add_parent.assert().success();

    // Create child task
    let mut add_child = common::ie_command_with_project_dir(temp_dir.path());
    add_child
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Child Task")
        .arg("--parent")
        .arg("1");
    add_child.assert().success();

    // Start parent task
    let mut start_cmd = common::ie_command_with_project_dir(temp_dir.path());
    start_cmd.assert().success();

    // Try to complete parent without completing child
    let mut done_cmd = common::ie_command_with_project_dir(temp_dir.path());
    done_cmd.arg("task").arg("done");

    done_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("has uncompleted children"));
}

// ============================================================================
// Dependency Tests
// ============================================================================

#[test]
fn test_task_depends_on_self() {
    let temp_dir = common::setup_test_env();

    // Create a task
    let mut add_cmd = common::ie_command_with_project_dir(temp_dir.path());
    add_cmd.arg("task").arg("add").arg("--name").arg("Task 1");
    add_cmd.assert().success();

    // Try to create self-dependency
    let mut dep_cmd = common::ie_command_with_project_dir(temp_dir.path());
    dep_cmd.arg("task").arg("depends-on").arg("1").arg("1");

    dep_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("Circular dependency detected"));
}

#[test]
fn test_task_circular_dependency_detection() {
    let temp_dir = common::setup_test_env();

    // Create three tasks
    for i in 1..=3 {
        let mut cmd = common::ie_command_with_project_dir(temp_dir.path());
        cmd.arg("task")
            .arg("add")
            .arg("--name")
            .arg(format!("Task {}", i));
        cmd.assert().success();
    }

    // Create dependencies: 1 -> 2 -> 3
    let mut dep1 = common::ie_command_with_project_dir(temp_dir.path());
    dep1.arg("task").arg("depends-on").arg("1").arg("2");
    dep1.assert().success();

    let mut dep2 = common::ie_command_with_project_dir(temp_dir.path());
    dep2.arg("task").arg("depends-on").arg("2").arg("3");
    dep2.assert().success();

    // Try to create circular dependency: 3 -> 1
    let mut dep3 = common::ie_command_with_project_dir(temp_dir.path());
    dep3.arg("task").arg("depends-on").arg("3").arg("1");

    dep3.assert()
        .failure()
        .stderr(predicate::str::contains("Circular dependency detected"));
}

#[test]
fn test_task_depends_on_nonexistent_task() {
    let temp_dir = common::setup_test_env();

    // Create one task
    let mut add_cmd = common::ie_command_with_project_dir(temp_dir.path());
    add_cmd.arg("task").arg("add").arg("--name").arg("Task 1");
    add_cmd.assert().success();

    // Try to create dependency with nonexistent task
    let mut dep_cmd = common::ie_command_with_project_dir(temp_dir.path());
    dep_cmd.arg("task").arg("depends-on").arg("1").arg("99999");

    dep_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task not found"));
}

// ============================================================================
// Spawn Subtask Edge Cases
// ============================================================================

#[test]
#[ignore]
fn test_spawn_subtask_without_current_task() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    cmd.arg("task")
        .arg("spawn-subtask")
        .arg("--name")
        .arg("Subtask");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No task is currently focused"));
}

#[test]
#[ignore]
fn test_spawn_subtask_with_empty_name() {
    let temp_dir = common::setup_test_env();

    // Create and start a task
    let mut add_cmd = common::ie_command_with_project_dir(temp_dir.path());
    add_cmd.arg("task").arg("add").arg("--name").arg("Parent");
    add_cmd.assert().success();

    let mut start_cmd = common::ie_command_with_project_dir(temp_dir.path());
    start_cmd.assert().success();

    // Try to spawn subtask with empty name
    let mut spawn_cmd = common::ie_command_with_project_dir(temp_dir.path());
    spawn_cmd
        .arg("task")
        .arg("spawn-subtask")
        .arg("--name")
        .arg("");

    spawn_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task name cannot be empty"));
}

// ============================================================================
// Priority and Complexity Edge Cases
// ============================================================================

#[test]
fn test_task_update_invalid_priority() {
    let temp_dir = common::setup_test_env();

    // Create a task
    let mut add_cmd = common::ie_command_with_project_dir(temp_dir.path());
    add_cmd
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test Task");
    add_cmd.assert().success();

    // Try to update with invalid priority
    let mut update_cmd = common::ie_command_with_project_dir(temp_dir.path());
    update_cmd
        .arg("task")
        .arg("update")
        .arg("1")
        .arg("--priority")
        .arg("invalid");

    update_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid priority"));
}

// ============================================================================
// Pick Next Edge Cases
// ============================================================================

#[test]
fn test_pick_next_with_multiple_tasks() {
    let temp_dir = common::setup_test_env();

    // Create some tasks
    for i in 1..=3 {
        let mut cmd = common::ie_command_with_project_dir(temp_dir.path());
        cmd.arg("task")
            .arg("add")
            .arg("--name")
            .arg(format!("Task {}", i));
        cmd.assert().success();
    }

    // Pick next should recommend one task
    let mut pick_cmd = common::ie_command_with_project_dir(temp_dir.path());
    pick_cmd.arg("task").arg("pick-next");

    pick_cmd.assert().success();
}

#[test]
#[ignore]
fn test_pick_next_with_all_tasks_completed() {
    let temp_dir = common::setup_test_env();

    // Create, start, and complete a task
    let mut add_cmd = common::ie_command_with_project_dir(temp_dir.path());
    add_cmd.arg("task").arg("add").arg("--name").arg("Task 1");
    add_cmd.assert().success();

    let mut start_cmd = common::ie_command_with_project_dir(temp_dir.path());
    start_cmd.arg("task").arg("start").arg("1");
    start_cmd.assert().success();

    let mut done_cmd = common::ie_command_with_project_dir(temp_dir.path());
    done_cmd.arg("task").arg("done");
    done_cmd.assert().success();

    // Try to pick next
    let mut pick_cmd = common::ie_command_with_project_dir(temp_dir.path());
    pick_cmd.arg("task").arg("pick-next");

    pick_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("all tasks completed"));
}
