use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to get the binary path
fn intent_engine_cmd() -> Command {
    Command::cargo_bin("intent-engine").unwrap()
}

/// Helper to create a test project
fn setup_test_project() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("intent.db");

    // Initialize project
    intent_engine_cmd()
        .current_dir(temp_dir.path())
        .arg("doctor")
        .assert()
        .success();

    (temp_dir, db_path)
}

/// Helper to add a task via CLI and return its ID
fn add_task(dir: &PathBuf, name: &str) -> i64 {
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg(name)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    json["id"].as_i64().unwrap()
}

#[test]
fn test_priority_critical() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Add a task
    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    // Update with critical priority
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task_id.to_string())
        .arg("--priority")
        .arg("critical")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["priority"].as_i64().unwrap(), 1);
}

#[test]
fn test_priority_high() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task_id.to_string())
        .arg("--priority")
        .arg("high")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["priority"].as_i64().unwrap(), 2);
}

#[test]
fn test_priority_medium() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task_id.to_string())
        .arg("--priority")
        .arg("medium")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["priority"].as_i64().unwrap(), 3);
}

#[test]
fn test_priority_low() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task_id.to_string())
        .arg("--priority")
        .arg("low")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["priority"].as_i64().unwrap(), 4);
}

#[test]
fn test_priority_case_insensitive() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    // Test uppercase
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task_id.to_string())
        .arg("--priority")
        .arg("HIGH")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["priority"].as_i64().unwrap(), 2);

    // Test mixed case
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task_id.to_string())
        .arg("--priority")
        .arg("CriTiCaL")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["priority"].as_i64().unwrap(), 1);
}

#[test]
fn test_priority_invalid_string() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    // Test invalid priority string
    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task_id.to_string())
        .arg("--priority")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid priority"));

    // Test empty string
    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task_id.to_string())
        .arg("--priority")
        .arg("")
        .assert()
        .failure();

    // Test numeric string (old format should fail)
    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task_id.to_string())
        .arg("--priority")
        .arg("1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid priority"));
}

#[test]
fn test_priority_ordering_still_works() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create tasks with different priorities
    let task1_id = add_task(&dir.to_path_buf(), "Low priority task");
    let task2_id = add_task(&dir.to_path_buf(), "Critical priority task");
    let task3_id = add_task(&dir.to_path_buf(), "Medium priority task");

    // Set priorities using new string format
    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task1_id.to_string())
        .arg("--priority")
        .arg("low")
        .assert()
        .success();

    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task2_id.to_string())
        .arg("--priority")
        .arg("critical")
        .assert()
        .success();

    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("update")
        .arg(task3_id.to_string())
        .arg("--priority")
        .arg("medium")
        .assert()
        .success();

    // List all tasks and verify priorities are stored correctly
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("list")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let tasks: Value = serde_json::from_str(&stdout).unwrap();
    let tasks_array = tasks.as_array().unwrap();

    // Find each task and verify priority
    let task1 = tasks_array
        .iter()
        .find(|t| t["id"].as_i64().unwrap() == task1_id)
        .unwrap();
    assert_eq!(task1["priority"].as_i64().unwrap(), 4); // low = 4

    let task2 = tasks_array
        .iter()
        .find(|t| t["id"].as_i64().unwrap() == task2_id)
        .unwrap();
    assert_eq!(task2["priority"].as_i64().unwrap(), 1); // critical = 1

    let task3 = tasks_array
        .iter()
        .find(|t| t["id"].as_i64().unwrap() == task3_id)
        .unwrap();
    assert_eq!(task3["priority"].as_i64().unwrap(), 3); // medium = 3
}

#[test]
fn test_task_list_command() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Add some tasks
    let task1_id = add_task(&dir.to_path_buf(), "Task 1");
    let _task2_id = add_task(&dir.to_path_buf(), "Task 2");

    // Create a subtask
    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Subtask 1")
        .arg("--parent")
        .arg(task1_id.to_string())
        .assert()
        .success();

    // List all tasks
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("list")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let tasks: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 3);

    // List with status filter
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("list")
        .arg("--status")
        .arg("todo")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let tasks: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 3);

    // List with parent filter
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("list")
        .arg("--parent")
        .arg(task1_id.to_string())
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let tasks: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 1);
    assert_eq!(
        tasks.as_array().unwrap()[0]["name"].as_str().unwrap(),
        "Subtask 1"
    );

    // List with parent=null filter (top-level only)
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("list")
        .arg("--parent")
        .arg("null")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let tasks: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 2);
}

#[test]
fn test_task_find_deprecated() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Add a task
    add_task(&dir.to_path_buf(), "Test Task");

    // Use deprecated 'find' command
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("find")
        .arg("--status")
        .arg("todo")
        .assert()
        .success();

    // Check that deprecation warning is shown in stderr
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert!(stderr.contains("Warning"));
    assert!(stderr.contains("deprecated"));
    assert!(stderr.contains("task list"));

    // Verify that it still works (returns data in stdout)
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let tasks: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 1);
}

#[test]
fn test_task_list_vs_find_same_results() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Add some tasks
    add_task(&dir.to_path_buf(), "Task 1");
    add_task(&dir.to_path_buf(), "Task 2");

    // Use 'list' command
    let list_output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("list")
        .arg("--status")
        .arg("todo")
        .assert()
        .success();

    let list_stdout = String::from_utf8_lossy(&list_output.get_output().stdout);
    let list_tasks: Value = serde_json::from_str(&list_stdout).unwrap();

    // Use deprecated 'find' command
    let find_output = intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("find")
        .arg("--status")
        .arg("todo")
        .assert()
        .success();

    let find_stdout = String::from_utf8_lossy(&find_output.get_output().stdout);
    let find_tasks: Value = serde_json::from_str(&find_stdout).unwrap();

    // Both should return the same results
    assert_eq!(list_tasks, find_tasks);
    assert_eq!(list_tasks.as_array().unwrap().len(), 2);
}
