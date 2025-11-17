mod common;

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to get the binary path
fn intent_engine_cmd() -> Command {
    common::ie_command()
}

/// Helper to create a test project
fn setup_test_project() -> (TempDir, PathBuf) {
    let temp_dir = common::setup_test_env();
    let db_path = temp_dir.path().join(".intent-engine").join("project.db");
    (temp_dir, db_path)
}

/// Helper to add a task and return its ID
fn add_task(dir: &PathBuf, name: &str, parent: Option<i64>) -> i64 {
    let mut cmd = intent_engine_cmd();
    cmd.current_dir(dir)
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg(name);

    if let Some(parent_id) = parent {
        cmd.arg("--parent").arg(parent_id.to_string());
    }

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    json["id"].as_i64().unwrap()
}

/// Helper to add a dependency
fn add_dependency(dir: &PathBuf, blocked_task_id: i64, blocking_task_id: i64) {
    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("depends-on")
        .arg(blocked_task_id.to_string())
        .arg(blocking_task_id.to_string())
        .assert()
        .success();
}

/// Helper to update task status
fn update_task_status(dir: &PathBuf, task_id: i64, status: &str) {
    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task_id.to_string())
        .arg("--status")
        .arg(status)
        .assert()
        .success();
}

/// Helper to set current task
fn set_current_task(dir: &PathBuf, task_id: i64) {
    intent_engine_cmd()
        .current_dir(dir)
        .arg("current")
        .arg("--set")
        .arg(task_id.to_string())
        .assert()
        .success();
}

#[test]
fn test_pick_next_skips_blocked_task() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create two todo tasks
    let task1 = add_task(&dir.to_path_buf(), "Task 1 - Blocking", None);
    let task2 = add_task(&dir.to_path_buf(), "Task 2 - Blocked", None);

    // Make task2 depend on task1
    add_dependency(&dir.to_path_buf(), task2, task1);

    // Pick next should recommend task1 (not blocked)
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["task"]["id"].as_i64().unwrap(), task1);
    assert_eq!(json["task"]["name"].as_str().unwrap(), "Task 1 - Blocking");
}

#[test]
fn test_pick_next_recommends_after_blocking_complete() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create two todo tasks
    let task1 = add_task(&dir.to_path_buf(), "Task 1 - Blocking", None);
    let task2 = add_task(&dir.to_path_buf(), "Task 2 - Blocked", None);

    // Make task2 depend on task1
    add_dependency(&dir.to_path_buf(), task2, task1);

    // Complete task1
    update_task_status(&dir.to_path_buf(), task1, "done");

    // Pick next should now recommend task2
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["task"]["id"].as_i64().unwrap(), task2);
    assert_eq!(json["task"]["name"].as_str().unwrap(), "Task 2 - Blocked");
}

#[test]
fn test_pick_next_multiple_dependencies() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create three tasks
    let task1 = add_task(&dir.to_path_buf(), "Task 1 - Blocker A", None);
    let task2 = add_task(&dir.to_path_buf(), "Task 2 - Blocker B", None);
    let task3 = add_task(&dir.to_path_buf(), "Task 3 - Blocked by both", None);

    // Make task3 depend on both task1 and task2
    add_dependency(&dir.to_path_buf(), task3, task1);
    add_dependency(&dir.to_path_buf(), task3, task2);

    // Complete only task1
    update_task_status(&dir.to_path_buf(), task1, "done");

    // Pick next should recommend task2, not task3 (still blocked by task2)
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["task"]["id"].as_i64().unwrap(), task2);

    // Now complete task2
    update_task_status(&dir.to_path_buf(), task2, "done");

    // Pick next should now recommend task3
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["task"]["id"].as_i64().unwrap(), task3);
}

#[test]
fn test_pick_next_blocked_subtask() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create parent task and two subtasks
    let parent = add_task(&dir.to_path_buf(), "Parent Task", None);
    let subtask1 = add_task(&dir.to_path_buf(), "Subtask 1 - Blocker", Some(parent));
    let subtask2 = add_task(&dir.to_path_buf(), "Subtask 2 - Blocked", Some(parent));

    // Make subtask2 depend on subtask1
    add_dependency(&dir.to_path_buf(), subtask2, subtask1);

    // Set parent as current task
    set_current_task(&dir.to_path_buf(), parent);

    // Pick next should recommend subtask1 (depth-first priority)
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["task"]["id"].as_i64().unwrap(), subtask1);
    assert_eq!(
        json["suggestion_type"].as_str().unwrap(),
        "FOCUSED_SUB_TASK"
    );
}

#[test]
fn test_pick_next_no_available_tasks_due_to_blocking() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create two tasks with circular-like blocking (but not actual circular)
    let task1 = add_task(&dir.to_path_buf(), "Task 1 - Blocked", None);
    let task2 = add_task(&dir.to_path_buf(), "Task 2 - Blocking", None);

    // Make task1 depend on task2
    add_dependency(&dir.to_path_buf(), task1, task2);

    // Set task2 to doing (not done, so still blocking)
    update_task_status(&dir.to_path_buf(), task2, "doing");

    // Pick next should recommend task2 (it's doing but available)
    // Actually, pick-next only looks for 'todo' tasks, so this should show no available
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Should indicate no available tasks (task1 is blocked, task2 is doing)
    assert_eq!(json["reason_code"].as_str().unwrap(), "NO_AVAILABLE_TODOS");
}

#[test]
fn test_pick_next_respects_priority_with_blocking() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create three tasks with different priorities
    let task1 = add_task(&dir.to_path_buf(), "Task 1 - Low Priority", None);
    let task2 = add_task(&dir.to_path_buf(), "Task 2 - High Priority", None);
    let task3 = add_task(&dir.to_path_buf(), "Task 3 - Medium Priority", None);

    // Set priorities (lower number = higher priority)
    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task1.to_string())
        .arg("--priority")
        .arg("low")
        .assert()
        .success();

    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task2.to_string())
        .arg("--priority")
        .arg("critical")
        .assert()
        .success();

    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task3.to_string())
        .arg("--priority")
        .arg("medium")
        .assert()
        .success();

    // Make task2 (high priority) depend on task1
    add_dependency(&dir.to_path_buf(), task2, task1);

    // Pick next should recommend task3 (medium priority, not blocked)
    // because task2 (higher priority) is blocked
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["task"]["id"].as_i64().unwrap(), task3);
}

#[test]
fn test_pick_next_unblocked_task_normal_behavior() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create tasks without any dependencies
    let task1 = add_task(&dir.to_path_buf(), "Task 1 - No Dependencies", None);
    let task2 = add_task(&dir.to_path_buf(), "Task 2 - No Dependencies", None);

    // Pick next should work normally, recommending task1 (first created)
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["task"]["id"].as_i64().unwrap(), task1);

    // Complete task1
    update_task_status(&dir.to_path_buf(), task1, "done");

    // Pick next should now recommend task2
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["task"]["id"].as_i64().unwrap(), task2);
}
