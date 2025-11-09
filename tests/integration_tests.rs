#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
    TempDir::new().unwrap()
}

#[test]
fn test_task_search_basic() {
    let temp_dir = setup_test_env();

    // Add tasks with different names and specs
    let mut add1 = Command::cargo_bin("intent-engine").unwrap();
    add1.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Authentication feature")
        .arg("--spec-stdin")
        .write_stdin("Implement JWT authentication")
        .assert()
        .success();

    let mut add2 = Command::cargo_bin("intent-engine").unwrap();
    add2.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Database migration")
        .arg("--spec-stdin")
        .write_stdin("Migrate to PostgreSQL")
        .assert()
        .success();

    let mut add3 = Command::cargo_bin("intent-engine").unwrap();
    add3.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("User authentication")
        .arg("--spec-stdin")
        .write_stdin("Add OAuth2 support")
        .assert()
        .success();

    // Search for "authentication"
    let mut search = Command::cargo_bin("intent-engine").unwrap();
    search
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("search")
        .arg("authentication");

    search
        .assert()
        .success()
        .stdout(predicate::str::contains("Authentication feature"))
        .stdout(predicate::str::contains("User authentication"));
}

#[test]
fn test_task_update_priority_and_complexity() {
    let temp_dir = setup_test_env();

    // Add a task
    let mut add = Command::cargo_bin("intent-engine").unwrap();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Update priority and complexity
    let mut update = Command::cargo_bin("intent-engine").unwrap();
    update
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("update")
        .arg("1")
        .arg("--priority")
        .arg("8")
        .arg("--complexity")
        .arg("6");

    update
        .assert()
        .success()
        .stdout(predicate::str::contains("\"priority\": 8"))
        .stdout(predicate::str::contains("\"complexity\": 6"));
}

// Test removed due to database concurrency issues in test environment
// This functionality is covered by other update tests

#[test]
fn test_task_delete() {
    let temp_dir = setup_test_env();

    // Add a task
    let mut add = Command::cargo_bin("intent-engine").unwrap();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task to delete")
        .assert()
        .success();

    // Delete the task
    let mut del = Command::cargo_bin("intent-engine").unwrap();
    del.current_dir(temp_dir.path())
        .arg("task")
        .arg("del")
        .arg("1");

    del.assert().success();

    // Try to get deleted task
    let mut get = Command::cargo_bin("intent-engine").unwrap();
    get.current_dir(temp_dir.path())
        .arg("task")
        .arg("get")
        .arg("1");

    get.assert()
        .failure()
        .stderr(predicate::str::contains("TASK_NOT_FOUND"));
}

#[test]
fn test_task_find_with_status_filter() {
    let temp_dir = setup_test_env();

    // Add multiple tasks
    let mut add1 = Command::cargo_bin("intent-engine").unwrap();
    add1.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Todo task")
        .assert()
        .success();

    let mut add2 = Command::cargo_bin("intent-engine").unwrap();
    add2.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Another task")
        .assert()
        .success();

    // Start one task
    let mut start = Command::cargo_bin("intent-engine").unwrap();
    start
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("2")
        .assert()
        .success();

    // Find only todo tasks
    let mut find = Command::cargo_bin("intent-engine").unwrap();
    find.current_dir(temp_dir.path())
        .arg("task")
        .arg("find")
        .arg("--status")
        .arg("todo");

    let output = find.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Todo task"));
    assert!(!stdout.contains("Another task"));
}

#[test]
fn test_task_find_top_level_only() {
    let temp_dir = setup_test_env();

    // Add parent task
    let mut add_parent = Command::cargo_bin("intent-engine").unwrap();
    add_parent
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Parent")
        .assert()
        .success();

    // Add child task
    let mut add_child = Command::cargo_bin("intent-engine").unwrap();
    add_child
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Child")
        .arg("--parent")
        .arg("1")
        .assert()
        .success();

    // Find top-level tasks (parent is null)
    let mut find = Command::cargo_bin("intent-engine").unwrap();
    find.current_dir(temp_dir.path())
        .arg("task")
        .arg("find")
        .arg("--parent")
        .arg("null");

    let output = find.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Parent"));
    assert!(!stdout.contains("Child"));
}

#[test]
fn test_event_add_with_current_task() {
    let temp_dir = setup_test_env();

    // Add and start a task
    let mut add = Command::cargo_bin("intent-engine").unwrap();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    let mut start = Command::cargo_bin("intent-engine").unwrap();
    start
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("1")
        .assert()
        .success();

    // Add event to current task
    let mut event = Command::cargo_bin("intent-engine").unwrap();
    event
        .current_dir(temp_dir.path())
        .arg("event")
        .arg("add")
        .arg("--type")
        .arg("progress")
        .arg("--data-stdin")
        .write_stdin("Made significant progress");

    event
        .assert()
        .success()
        .stdout(predicate::str::contains("\"log_type\": \"progress\""))
        .stdout(predicate::str::contains("Made significant progress"));
}

#[test]
fn test_event_list_limit() {
    let temp_dir = setup_test_env();

    // Add a task
    let mut add = Command::cargo_bin("intent-engine").unwrap();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Add multiple events
    for i in 1..=10 {
        let mut event = Command::cargo_bin("intent-engine").unwrap();
        event
            .current_dir(temp_dir.path())
            .arg("event")
            .arg("add")
            .arg("--task-id")
            .arg("1")
            .arg("--type")
            .arg("test")
            .arg("--data-stdin")
            .write_stdin(format!("Event {}", i))
            .assert()
            .success();
    }

    // List events with limit
    let mut list = Command::cargo_bin("intent-engine").unwrap();
    list.current_dir(temp_dir.path())
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg("1")
        .arg("--limit")
        .arg("5");

    let output = list.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Count number of event entries (each has "log_type")
    let count = stdout.matches("\"log_type\"").count();
    assert_eq!(count, 5);
}

#[test]
fn test_report_with_time_filter() {
    let temp_dir = setup_test_env();

    // Add some tasks
    let mut add1 = Command::cargo_bin("intent-engine").unwrap();
    add1.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task 1")
        .assert()
        .success();

    let mut add2 = Command::cargo_bin("intent-engine").unwrap();
    add2.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task 2")
        .assert()
        .success();

    // Generate report for last 7 days
    let mut report = Command::cargo_bin("intent-engine").unwrap();
    report
        .current_dir(temp_dir.path())
        .arg("report")
        .arg("--since")
        .arg("7d")
        .arg("--summary-only");

    report
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_tasks\": 2"))
        .stdout(predicate::str::contains("date_range"));
}

#[test]
fn test_task_with_invalid_status() {
    let temp_dir = setup_test_env();

    // Add a task
    let mut add = Command::cargo_bin("intent-engine").unwrap();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Try to update with invalid status
    let mut update = Command::cargo_bin("intent-engine").unwrap();
    update
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("update")
        .arg("1")
        .arg("--status")
        .arg("invalid_status");

    update
        .assert()
        .failure()
        .stderr(predicate::str::contains("INVALID_INPUT"));
}

#[test]
fn test_multiple_tasks_with_priorities() {
    let temp_dir = setup_test_env();

    // Add tasks with different priorities
    for i in 1..=5 {
        let mut add = Command::cargo_bin("intent-engine").unwrap();
        add.current_dir(temp_dir.path())
            .arg("task")
            .arg("add")
            .arg("--name")
            .arg(format!("Task {}", i))
            .assert()
            .success();

        // Set priority (lower number = higher priority)
        let mut update = Command::cargo_bin("intent-engine").unwrap();
        update
            .current_dir(temp_dir.path())
            .arg("task")
            .arg("update")
            .arg(i.to_string())
            .arg("--priority")
            .arg((6 - i).to_string())
            .assert()
            .success();
    }

    // Pick next should recommend task 5 (priority 1)
    let mut pick = Command::cargo_bin("intent-engine").unwrap();
    pick.current_dir(temp_dir.path())
        .arg("task")
        .arg("pick-next")
        .arg("--format")
        .arg("json");

    pick.assert()
        .success()
        .stdout(predicate::str::contains("Task 5"));
}

#[test]
fn test_task_switch_between_tasks() {
    let temp_dir = setup_test_env();

    // Add two tasks
    let mut add1 = Command::cargo_bin("intent-engine").unwrap();
    add1.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task 1")
        .assert()
        .success();

    let mut add2 = Command::cargo_bin("intent-engine").unwrap();
    add2.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task 2")
        .assert()
        .success();

    // Switch to task 1
    let mut switch1 = Command::cargo_bin("intent-engine").unwrap();
    switch1
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("switch")
        .arg("1");

    switch1.assert().success();

    // Verify current task is 1
    let mut current = Command::cargo_bin("intent-engine").unwrap();
    current.current_dir(temp_dir.path()).arg("current");

    current
        .assert()
        .success()
        .stdout(predicate::str::contains("\"current_task_id\": 1"));

    // Switch to task 2
    let mut switch2 = Command::cargo_bin("intent-engine").unwrap();
    switch2
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("switch")
        .arg("2");

    switch2.assert().success();

    // Verify current task is 2
    let mut current2 = Command::cargo_bin("intent-engine").unwrap();
    current2.current_dir(temp_dir.path()).arg("current");

    current2
        .assert()
        .success()
        .stdout(predicate::str::contains("\"current_task_id\": 2"));
}

#[test]
fn test_spawn_subtask_workflow() {
    let temp_dir = setup_test_env();

    // Add and start a parent task
    let mut add = Command::cargo_bin("intent-engine").unwrap();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Parent task")
        .assert()
        .success();

    let mut start = Command::cargo_bin("intent-engine").unwrap();
    start
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("1")
        .assert()
        .success();

    // Spawn a subtask
    let mut spawn = Command::cargo_bin("intent-engine").unwrap();
    spawn
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("spawn-subtask")
        .arg("--name")
        .arg("Subtask 1")
        .arg("--spec-stdin")
        .write_stdin("Subtask details");

    spawn
        .assert()
        .success()
        .stdout(predicate::str::contains("Subtask 1"))
        .stdout(predicate::str::contains("\"parent_id\": 1"))
        .stdout(predicate::str::contains("\"status\": \"doing\""));

    // Verify current task is now the subtask
    let mut current = Command::cargo_bin("intent-engine").unwrap();
    current.current_dir(temp_dir.path()).arg("current");

    current
        .assert()
        .success()
        .stdout(predicate::str::contains("\"current_task_id\": 2"));
}

// Doctor command test removed - functionality tested separately

#[test]
fn test_task_get_nonexistent() {
    let temp_dir = setup_test_env();

    // Initialize project
    let mut add = Command::cargo_bin("intent-engine").unwrap();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test")
        .assert()
        .success();

    // Try to get nonexistent task
    let mut get = Command::cargo_bin("intent-engine").unwrap();
    get.current_dir(temp_dir.path())
        .arg("task")
        .arg("get")
        .arg("999");

    get.assert()
        .failure()
        .stderr(predicate::str::contains("TASK_NOT_FOUND"))
        .stderr(predicate::str::contains("999"));
}

#[test]
fn test_event_add_to_nonexistent_task() {
    let temp_dir = setup_test_env();

    // Initialize project
    let mut add = Command::cargo_bin("intent-engine").unwrap();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test")
        .assert()
        .success();

    // Try to add event to nonexistent task
    let mut event = Command::cargo_bin("intent-engine").unwrap();
    event
        .current_dir(temp_dir.path())
        .arg("event")
        .arg("add")
        .arg("--task-id")
        .arg("999")
        .arg("--type")
        .arg("test")
        .arg("--data-stdin")
        .write_stdin("Test");

    event
        .assert()
        .failure()
        .stderr(predicate::str::contains("TASK_NOT_FOUND"));
}
