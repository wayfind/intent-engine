#![allow(deprecated)]
// Tests in this file use CLI commands removed in v0.10.0
// v0.10.0 simplified CLI to just: plan, log, search
// These tests are kept for reference but disabled by default
#![cfg(feature = "test-removed-cli-commands")]

mod common;

use predicates::prelude::*;

#[test]
fn test_task_search_basic() {
    let temp_dir = common::setup_test_env();

    // Add tasks with different names and specs
    let mut add1 = common::ie_command();
    add1.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Authentication feature")
        .arg("--spec-stdin")
        .write_stdin("Implement JWT authentication")
        .assert()
        .success();

    let mut add2 = common::ie_command();
    add2.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Database migration")
        .arg("--spec-stdin")
        .write_stdin("Migrate to PostgreSQL")
        .assert()
        .success();

    let mut add3 = common::ie_command();
    add3.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("User authentication")
        .arg("--spec-stdin")
        .write_stdin("Add OAuth2 support")
        .assert()
        .success();

    // Search for "authentication" (unified search)
    let mut search = common::ie_command();
    search
        .current_dir(temp_dir.path())
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
    let temp_dir = common::setup_test_env();

    // Add a task
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Update priority and complexity
    let mut update = common::ie_command();
    update
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("update")
        .arg("1")
        .arg("--priority")
        .arg("high")
        .arg("--complexity")
        .arg("6");

    update
        .assert()
        .success()
        .stdout(predicate::str::contains("\"priority\": 2"))
        .stdout(predicate::str::contains("\"complexity\": 6"));
}

// NOTE: Test removed due to database concurrency issues in test environment
//
// ISSUE: SQLite has writer locking limitations - only one writer at a time.
// When running tests in parallel (default with `cargo test`), multiple test
// processes can attempt to write to SQLite databases simultaneously, causing
// lock contention and flaky test failures.
//
// CURRENT MITIGATION: Each test uses its own temporary directory with an
// isolated database, which should prevent most concurrency issues. However,
// certain complex update scenarios may still experience race conditions.
//
// COVERAGE: This specific test functionality is covered by other update tests
// that use simpler operations and avoid the problematic concurrency patterns.
//
// FUTURE IMPROVEMENTS:
// - Consider using `#[serial]` attribute from `serial_test` crate for tests
//   that are known to have concurrency issues
// - Add explicit test synchronization for database operations
// - Use WAL mode for SQLite to improve concurrent access (though this has
//   limitations in test environments with short-lived temp directories)

#[test]
fn test_task_delete() {
    let temp_dir = common::setup_test_env();

    // Add a task
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task to delete")
        .assert()
        .success();

    // Delete the task
    let mut del = common::ie_command();
    del.current_dir(temp_dir.path())
        .arg("task")
        .arg("del")
        .arg("1");

    del.assert().success();

    // Try to get deleted task
    let mut get = common::ie_command();
    get.current_dir(temp_dir.path())
        .arg("task")
        .arg("get")
        .arg("1");

    get.assert()
        .failure()
        .stderr(predicate::str::contains("TASK_NOT_FOUND"));
}

#[test]
fn test_task_list_with_status_filter() {
    let temp_dir = common::setup_test_env();

    // Add multiple tasks
    let mut add1 = common::ie_command();
    add1.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Todo task")
        .assert()
        .success();

    let mut add2 = common::ie_command();
    add2.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Another task")
        .assert()
        .success();

    // Start one task
    let mut start = common::ie_command();
    start
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("2")
        .assert()
        .success();

    // List only todo tasks (using positional argument)
    let mut find = common::ie_command();
    find.current_dir(temp_dir.path())
        .arg("task")
        .arg("list")
        .arg("todo");

    let output = find.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Todo task"));
    assert!(!stdout.contains("Another task"));
}

#[test]
fn test_task_list_top_level_only() {
    let temp_dir = common::setup_test_env();

    // Add parent task
    let mut add_parent = common::ie_command();
    add_parent
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Parent")
        .assert()
        .success();

    // Add child task
    let mut add_child = common::ie_command();
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

    // List top-level tasks (parent is null)
    let mut find = common::ie_command();
    find.current_dir(temp_dir.path())
        .arg("task")
        .arg("list")
        .arg("--parent")
        .arg("null");

    let output = find.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Parent"));
    assert!(!stdout.contains("Child"));
}

#[test]
fn test_event_add_with_current_task() {
    let temp_dir = common::setup_test_env();

    // Add and start a task
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    let mut start = common::ie_command();
    start
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("1")
        .assert()
        .success();

    // Add event to current task
    let mut event = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Add a task
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Add multiple events
    for i in 1..=10 {
        let mut event = common::ie_command();
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
    let mut list = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Add some tasks
    let mut add1 = common::ie_command();
    add1.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task 1")
        .assert()
        .success();

    let mut add2 = common::ie_command();
    add2.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task 2")
        .assert()
        .success();

    // Generate report for last 7 days
    let mut report = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Add a task
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Try to update with invalid status
    let mut update = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Add tasks with different priorities
    let priorities = ["low", "low", "medium", "high", "critical"];
    for i in 1..=5 {
        let mut add = common::ie_command();
        add.current_dir(temp_dir.path())
            .arg("task")
            .arg("add")
            .arg("--name")
            .arg(format!("Task {}", i))
            .assert()
            .success();

        // Set priority (lower number = higher priority)
        let mut update = common::ie_command();
        update
            .current_dir(temp_dir.path())
            .arg("task")
            .arg("update")
            .arg(i.to_string())
            .arg("--priority")
            .arg(priorities[i - 1])
            .assert()
            .success();
    }

    // Pick next should recommend task 5 (priority critical = 1)
    let mut pick = common::ie_command();
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
fn test_spawn_subtask_workflow() {
    let temp_dir = common::setup_test_env();

    // Add and start a parent task
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Parent task")
        .assert()
        .success();

    let mut start = common::ie_command();
    start
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("1")
        .assert()
        .success();

    // Spawn a subtask
    let mut spawn = common::ie_command();
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
    let mut current = common::ie_command();
    current.current_dir(temp_dir.path()).arg("current");

    current
        .assert()
        .success()
        .stdout(predicate::str::contains("\"current_task_id\": 2"));
}

// NOTE: Doctor command test removed from this integration test file
//
// REASON: The doctor command functionality is now tested in a dedicated
// test file: `tests/doctor_command_tests.rs`
//
// RATIONALE: Separating doctor command tests improves test organization
// and allows for more focused testing of diagnostic functionality without
// cluttering the main integration test suite.

#[test]
fn test_task_get_nonexistent() {
    let temp_dir = common::setup_test_env();

    // Initialize project
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test")
        .assert()
        .success();

    // Try to get nonexistent task
    let mut get = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Initialize project
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test")
        .assert()
        .success();

    // Try to add event to nonexistent task
    let mut event = common::ie_command();
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
