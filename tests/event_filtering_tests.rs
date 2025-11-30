mod common;

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

/// Helper to get the binary path
fn intent_engine_cmd() -> Command {
    common::ie_command()
}

/// Helper to create a test project
fn setup_test_project() -> (tempfile::TempDir, PathBuf) {
    let temp_dir = common::setup_test_env();
    let db_path = temp_dir.path().join(".intent-engine").join("project.db");

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

/// Helper to add an event via CLI
fn add_event(dir: &PathBuf, task_id: i64, log_type: &str, data: &str) {
    intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("add")
        .arg("--task-id")
        .arg(task_id.to_string())
        .arg("--type")
        .arg(log_type)
        .write_stdin(data)
        .arg("--data-stdin")
        .assert()
        .success();
}

#[test]
fn test_event_list_filter_by_type() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create a task
    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    // Add events of different types
    add_event(&dir.to_path_buf(), task_id, "decision", "Decision 1");
    add_event(&dir.to_path_buf(), task_id, "blocker", "Blocker 1");
    add_event(&dir.to_path_buf(), task_id, "milestone", "Milestone 1");
    add_event(&dir.to_path_buf(), task_id, "decision", "Decision 2");
    add_event(&dir.to_path_buf(), task_id, "note", "Note 1");

    // List all events
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let all_events: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(all_events.as_array().unwrap().len(), 5);

    // Filter by type: decision
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .arg("--type")
        .arg("decision")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let decision_events: Value = serde_json::from_str(&stdout).unwrap();
    let events_array = decision_events.as_array().unwrap();
    assert_eq!(events_array.len(), 2);
    assert!(events_array
        .iter()
        .all(|e| e["log_type"].as_str().unwrap() == "decision"));

    // Filter by type: milestone
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .arg("--type")
        .arg("milestone")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let milestone_events: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(milestone_events.as_array().unwrap().len(), 1);
}

#[test]
fn test_event_list_filter_by_since() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create a task
    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    // Add some old events
    add_event(&dir.to_path_buf(), task_id, "note", "Old event 1");
    add_event(&dir.to_path_buf(), task_id, "note", "Old event 2");

    // Wait longer than the filter window to ensure old events are excluded
    sleep(Duration::from_secs(6));

    // Add some recent events
    add_event(&dir.to_path_buf(), task_id, "note", "Recent event 1");
    add_event(&dir.to_path_buf(), task_id, "note", "Recent event 2");

    // List all events
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let all_events: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(all_events.as_array().unwrap().len(), 4);

    // Filter by since: 5s (should get only the recent 2 events)
    // Use 5s to allow for slower CI environments
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .arg("--since")
        .arg("5s")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let recent_events: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(recent_events.as_array().unwrap().len(), 2);
}

#[test]
fn test_event_list_filter_combined() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create a task
    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    // Add events with different types at different times
    add_event(&dir.to_path_buf(), task_id, "decision", "Old decision");
    add_event(&dir.to_path_buf(), task_id, "blocker", "Old blocker");

    // Wait longer than the filter window to ensure old events are excluded
    sleep(Duration::from_secs(6));

    add_event(&dir.to_path_buf(), task_id, "decision", "Recent decision");
    add_event(&dir.to_path_buf(), task_id, "blocker", "Recent blocker");
    add_event(&dir.to_path_buf(), task_id, "milestone", "Recent milestone");

    // Filter by both type=decision AND since=5s
    // Use 5s to allow for slower CI environments
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .arg("--type")
        .arg("decision")
        .arg("--since")
        .arg("5s")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let filtered_events: Value = serde_json::from_str(&stdout).unwrap();
    let events_array = filtered_events.as_array().unwrap();

    // Should only get the recent decision
    assert_eq!(events_array.len(), 1);
    assert_eq!(events_array[0]["log_type"].as_str().unwrap(), "decision");
    assert!(events_array[0]["discussion_data"]
        .as_str()
        .unwrap()
        .contains("Recent"));
}

#[test]
fn test_event_list_since_duration_formats() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create a task
    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    // Add an event
    add_event(&dir.to_path_buf(), task_id, "note", "Test event");

    // Test different duration formats: days
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .arg("--since")
        .arg("7d")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let events: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(events.as_array().unwrap().len(), 1);

    // Test hours
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .arg("--since")
        .arg("24h")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let events: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(events.as_array().unwrap().len(), 1);

    // Test minutes
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .arg("--since")
        .arg("30m")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let events: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(events.as_array().unwrap().len(), 1);
}

#[test]
fn test_event_list_invalid_since_format() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create a task
    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    // Add an event
    add_event(&dir.to_path_buf(), task_id, "note", "Test event");

    // Test invalid format
    intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .arg("--since")
        .arg("invalid")
        .assert()
        .failure();

    // Test invalid unit (use 'y' for years which is not supported)
    intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .arg("--since")
        .arg("7y")
        .assert()
        .failure();
}

#[test]
fn test_event_list_with_limit_and_filter() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create a task
    let task_id = add_task(&dir.to_path_buf(), "Test Task");

    // Add many decision events
    for i in 0..10 {
        add_event(
            &dir.to_path_buf(),
            task_id,
            "decision",
            &format!("Decision {}", i),
        );
    }

    // Filter by type with limit
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg(task_id.to_string())
        .arg("--type")
        .arg("decision")
        .arg("--limit")
        .arg("5")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let events: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(events.as_array().unwrap().len(), 5);
}

// ==================== Global Event Query Tests ====================

#[test]
fn test_event_list_global_no_filter() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create multiple tasks
    let task1 = add_task(&dir.to_path_buf(), "Task 1");
    let task2 = add_task(&dir.to_path_buf(), "Task 2");
    let task3 = add_task(&dir.to_path_buf(), "Task 3");

    // Add events to different tasks
    add_event(&dir.to_path_buf(), task1, "decision", "Task 1 decision");
    add_event(&dir.to_path_buf(), task1, "blocker", "Task 1 blocker");
    add_event(&dir.to_path_buf(), task2, "milestone", "Task 2 milestone");
    add_event(&dir.to_path_buf(), task3, "note", "Task 3 note");
    add_event(&dir.to_path_buf(), task3, "decision", "Task 3 decision");

    // List all events globally (no task-id)
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let all_events: Value = serde_json::from_str(&stdout).unwrap();
    let events_array = all_events.as_array().unwrap();

    // Should return all 5 events across all tasks
    assert_eq!(events_array.len(), 5);

    // Verify events from different tasks are present
    let task_ids: std::collections::HashSet<i64> = events_array
        .iter()
        .map(|e| e["task_id"].as_i64().unwrap())
        .collect();
    assert_eq!(task_ids.len(), 3); // Events from 3 different tasks
}

#[test]
fn test_event_list_global_filter_by_type() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create multiple tasks
    let task1 = add_task(&dir.to_path_buf(), "Task 1");
    let task2 = add_task(&dir.to_path_buf(), "Task 2");

    // Add events of different types across tasks
    add_event(&dir.to_path_buf(), task1, "decision", "Task 1 decision 1");
    add_event(&dir.to_path_buf(), task1, "blocker", "Task 1 blocker");
    add_event(&dir.to_path_buf(), task2, "decision", "Task 2 decision 1");
    add_event(&dir.to_path_buf(), task2, "milestone", "Task 2 milestone");
    add_event(&dir.to_path_buf(), task2, "decision", "Task 2 decision 2");

    // List all decision events globally
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--type")
        .arg("decision")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let decision_events: Value = serde_json::from_str(&stdout).unwrap();
    let events_array = decision_events.as_array().unwrap();

    // Should return 3 decision events (1 from task1, 2 from task2)
    assert_eq!(events_array.len(), 3);
    assert!(events_array
        .iter()
        .all(|e| e["log_type"].as_str().unwrap() == "decision"));

    // Verify events from multiple tasks
    let task_ids: std::collections::HashSet<i64> = events_array
        .iter()
        .map(|e| e["task_id"].as_i64().unwrap())
        .collect();
    assert_eq!(task_ids.len(), 2);
}

#[test]
fn test_event_list_global_filter_by_since() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create multiple tasks
    let task1 = add_task(&dir.to_path_buf(), "Task 1");
    let task2 = add_task(&dir.to_path_buf(), "Task 2");

    // Add old events
    add_event(&dir.to_path_buf(), task1, "note", "Old event task 1");
    add_event(&dir.to_path_buf(), task2, "note", "Old event task 2");

    // Wait
    sleep(Duration::from_secs(6));

    // Add recent events
    add_event(&dir.to_path_buf(), task1, "note", "Recent event task 1");
    add_event(&dir.to_path_buf(), task2, "note", "Recent event task 2");

    // List recent events globally (since 5s)
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--since")
        .arg("5s")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let recent_events: Value = serde_json::from_str(&stdout).unwrap();
    let events_array = recent_events.as_array().unwrap();

    // Should only get recent 2 events
    assert_eq!(events_array.len(), 2);
    assert!(events_array
        .iter()
        .all(|e| e["discussion_data"].as_str().unwrap().contains("Recent")));
}

#[test]
fn test_event_list_global_combined_filters() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create multiple tasks
    let task1 = add_task(&dir.to_path_buf(), "Task 1");
    let task2 = add_task(&dir.to_path_buf(), "Task 2");

    // Add old events
    add_event(&dir.to_path_buf(), task1, "decision", "Old decision task 1");
    add_event(&dir.to_path_buf(), task2, "blocker", "Old blocker task 2");

    // Wait
    sleep(Duration::from_secs(6));

    // Add recent events of different types
    add_event(
        &dir.to_path_buf(),
        task1,
        "decision",
        "Recent decision task 1",
    );
    add_event(
        &dir.to_path_buf(),
        task1,
        "blocker",
        "Recent blocker task 1",
    );
    add_event(
        &dir.to_path_buf(),
        task2,
        "decision",
        "Recent decision task 2",
    );
    add_event(
        &dir.to_path_buf(),
        task2,
        "milestone",
        "Recent milestone task 2",
    );

    // Filter globally by type=decision AND since=5s
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--type")
        .arg("decision")
        .arg("--since")
        .arg("5s")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let filtered_events: Value = serde_json::from_str(&stdout).unwrap();
    let events_array = filtered_events.as_array().unwrap();

    // Should get 2 recent decision events
    assert_eq!(events_array.len(), 2);
    assert!(events_array
        .iter()
        .all(|e| e["log_type"].as_str().unwrap() == "decision"));
    assert!(events_array
        .iter()
        .all(|e| e["discussion_data"].as_str().unwrap().contains("Recent")));
}

#[test]
fn test_event_list_global_respects_limit() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create multiple tasks
    let task1 = add_task(&dir.to_path_buf(), "Task 1");
    let task2 = add_task(&dir.to_path_buf(), "Task 2");
    let task3 = add_task(&dir.to_path_buf(), "Task 3");

    // Add many events across different tasks
    for i in 0..20 {
        add_event(
            &dir.to_path_buf(),
            task1,
            "note",
            &format!("Task 1 event {}", i),
        );
    }
    for i in 0..20 {
        add_event(
            &dir.to_path_buf(),
            task2,
            "note",
            &format!("Task 2 event {}", i),
        );
    }
    for i in 0..20 {
        add_event(
            &dir.to_path_buf(),
            task3,
            "note",
            &format!("Task 3 event {}", i),
        );
    }

    // List globally without limit (should use default 50)
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let events: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(events.as_array().unwrap().len(), 50); // Default limit

    // List globally with custom limit
    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("event")
        .arg("list")
        .arg("--limit")
        .arg("10")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let events: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(events.as_array().unwrap().len(), 10); // Custom limit
}
