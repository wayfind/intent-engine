#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
    TempDir::new().unwrap()
}

#[test]
fn test_cli_task_add() {
    let temp_dir = setup_test_env();
    let mut cmd = Command::cargo_bin("intent-engine").unwrap();

    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"id\":"))
        .stdout(predicate::str::contains("\"name\": \"Test task\""))
        .stdout(predicate::str::contains("\"status\": \"todo\""));
}

#[test]
fn test_cli_task_find() {
    let temp_dir = setup_test_env();

    // Add a task first
    let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
    add_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Find tasks
    let mut find_cmd = Command::cargo_bin("intent-engine").unwrap();
    find_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("find");

    find_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Test task"));
}

#[test]
fn test_cli_task_start() {
    let temp_dir = setup_test_env();

    // Add a task
    let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
    add_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Start the task
    let mut start_cmd = Command::cargo_bin("intent-engine").unwrap();
    start_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("1");

    start_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"doing\""));
}

#[test]
fn test_cli_task_done() {
    let temp_dir = setup_test_env();

    // Add and start a task
    let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
    add_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Complete the task
    let mut done_cmd = Command::cargo_bin("intent-engine").unwrap();
    done_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("done")
        .arg("1");

    done_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"done\""));
}

#[test]
fn test_cli_task_done_with_uncompleted_children() {
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

    // Try to complete parent (should fail)
    let mut done_cmd = Command::cargo_bin("intent-engine").unwrap();
    done_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("done")
        .arg("1");

    done_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("UNCOMPLETED_CHILDREN"));
}

#[test]
fn test_cli_current() {
    let temp_dir = setup_test_env();

    // Add and start a task
    let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
    add_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    let mut start_cmd = Command::cargo_bin("intent-engine").unwrap();
    start_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("1")
        .assert()
        .success();

    // Get current task
    let mut current_cmd = Command::cargo_bin("intent-engine").unwrap();
    current_cmd.current_dir(temp_dir.path()).arg("current");

    current_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("\"current_task_id\": 1"));
}

#[test]
fn test_cli_event_add() {
    let temp_dir = setup_test_env();

    // Add a task
    let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
    add_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Add an event
    let mut event_cmd = Command::cargo_bin("intent-engine").unwrap();
    event_cmd
        .current_dir(temp_dir.path())
        .arg("event")
        .arg("add")
        .arg("--task-id")
        .arg("1")
        .arg("--type")
        .arg("decision")
        .arg("--data-stdin")
        .write_stdin("Test event data");

    event_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("\"log_type\": \"decision\""))
        .stdout(predicate::str::contains("Test event data"));
}

#[test]
fn test_cli_event_list() {
    let temp_dir = setup_test_env();

    // Add a task and event
    let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
    add_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    let mut event_cmd = Command::cargo_bin("intent-engine").unwrap();
    event_cmd
        .current_dir(temp_dir.path())
        .arg("event")
        .arg("add")
        .arg("--task-id")
        .arg("1")
        .arg("--type")
        .arg("decision")
        .arg("--data-stdin")
        .write_stdin("Test event")
        .assert()
        .success();

    // List events
    let mut list_cmd = Command::cargo_bin("intent-engine").unwrap();
    list_cmd
        .current_dir(temp_dir.path())
        .arg("event")
        .arg("list")
        .arg("--task-id")
        .arg("1");

    list_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Test event"));
}

#[test]
fn test_cli_report() {
    let temp_dir = setup_test_env();

    // Add some tasks
    let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
    add_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Generate report
    let mut report_cmd = Command::cargo_bin("intent-engine").unwrap();
    report_cmd
        .current_dir(temp_dir.path())
        .arg("report")
        .arg("--summary-only");

    report_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_tasks\": 1"))
        .stdout(predicate::str::contains("tasks_by_status"));
}

#[test]
fn test_cli_project_not_found() {
    let temp_dir = setup_test_env();

    // Try to get task in non-project directory (read operation)
    let mut cmd = Command::cargo_bin("intent-engine").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("get")
        .arg("1");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("NOT_A_PROJECT"));
}

#[test]
fn test_cli_lazy_init() {
    let temp_dir = setup_test_env();

    // Write operation should auto-initialize
    let mut cmd = Command::cargo_bin("intent-engine").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test");

    cmd.assert().success();

    // Verify .intent-engine directory was created
    assert!(temp_dir.path().join(".intent-engine").exists());
    assert!(temp_dir
        .path()
        .join(".intent-engine")
        .join("project.db")
        .exists());
}

#[test]
fn test_cli_task_with_spec() {
    let temp_dir = setup_test_env();

    let mut cmd = Command::cargo_bin("intent-engine").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .arg("--spec-stdin")
        .write_stdin("This is the task specification");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"Test task\""));
}

#[test]
fn test_cli_task_hierarchy() {
    let temp_dir = setup_test_env();

    // Add parent
    let mut parent_cmd = Command::cargo_bin("intent-engine").unwrap();
    parent_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Parent")
        .assert()
        .success();

    // Add child
    let mut child_cmd = Command::cargo_bin("intent-engine").unwrap();
    child_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Child")
        .arg("--parent")
        .arg("1");

    child_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("\"parent_id\": 1"));

    // Find children
    let mut find_cmd = Command::cargo_bin("intent-engine").unwrap();
    find_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("find")
        .arg("--parent")
        .arg("1");

    find_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Child"));
}

#[test]
fn test_cli_task_get_with_events() {
    let temp_dir = setup_test_env();

    // Add task and event
    let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
    add_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test")
        .assert()
        .success();

    let mut event_cmd = Command::cargo_bin("intent-engine").unwrap();
    event_cmd
        .current_dir(temp_dir.path())
        .arg("event")
        .arg("add")
        .arg("--task-id")
        .arg("1")
        .arg("--type")
        .arg("test")
        .arg("--data-stdin")
        .write_stdin("Test event")
        .assert()
        .success();

    // Get task with events
    let mut get_cmd = Command::cargo_bin("intent-engine").unwrap();
    get_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("get")
        .arg("1")
        .arg("--with-events");

    get_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("events_summary"));
}

#[test]
fn test_cli_project_awareness_from_subdirectory() {
    let temp_dir = setup_test_env();

    // Initialize in root directory
    let mut init_cmd = Command::cargo_bin("intent-engine").unwrap();
    init_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Root task")
        .assert()
        .success();

    // Create subdirectory
    let subdir = temp_dir.path().join("src").join("components");
    fs::create_dir_all(&subdir).unwrap();

    // Access from subdirectory (should find parent's database)
    let mut find_cmd = Command::cargo_bin("intent-engine").unwrap();
    find_cmd.current_dir(&subdir).arg("task").arg("find");

    find_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Root task"));

    // Verify no new .intent-engine was created in subdirectory
    assert!(!subdir.join(".intent-engine").exists());
    assert!(temp_dir.path().join(".intent-engine").exists());
}

#[test]
fn test_cli_project_awareness_deep_nesting() {
    let temp_dir = setup_test_env();

    // Initialize in root directory
    let mut init_cmd = Command::cargo_bin("intent-engine").unwrap();
    init_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task1")
        .assert()
        .success();

    // Create deeply nested directory
    let deep_dir = temp_dir.path().join("a").join("b").join("c").join("d");
    fs::create_dir_all(&deep_dir).unwrap();

    // Access from deep directory (should traverse up and find root's database)
    let mut find_cmd = Command::cargo_bin("intent-engine").unwrap();
    find_cmd.current_dir(&deep_dir).arg("task").arg("find");

    find_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Task1"));

    // Verify only root has .intent-engine
    assert!(temp_dir.path().join(".intent-engine").exists());
    assert!(!deep_dir.join(".intent-engine").exists());
}

#[test]
fn test_cli_subdirectory_write_uses_parent_db() {
    let temp_dir = setup_test_env();

    // Initialize in root
    let mut init_cmd = Command::cargo_bin("intent-engine").unwrap();
    init_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task1")
        .assert()
        .success();

    // Create subdirectory
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir_all(&subdir).unwrap();

    // Write from subdirectory (should use parent's database, not create new one)
    let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
    add_cmd
        .current_dir(&subdir)
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task2")
        .assert()
        .success();

    // Verify no new .intent-engine in subdirectory
    assert!(!subdir.join(".intent-engine").exists());
    assert!(temp_dir.path().join(".intent-engine").exists());

    // Verify both tasks visible from root
    let mut find_cmd = Command::cargo_bin("intent-engine").unwrap();
    let output = find_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("find")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Task1"));
    assert!(stdout.contains("Task2"));

    // Verify both tasks also visible from subdirectory
    let mut find_cmd2 = Command::cargo_bin("intent-engine").unwrap();
    let output2 = find_cmd2
        .current_dir(&subdir)
        .arg("task")
        .arg("find")
        .output()
        .unwrap();

    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout2.contains("Task1"));
    assert!(stdout2.contains("Task2"));
}

#[test]
fn test_cli_isolated_projects() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();

    // Project 1: Add task
    let mut cmd1 = Command::cargo_bin("intent-engine").unwrap();
    cmd1.current_dir(temp_dir1.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Project1 Task")
        .assert()
        .success();

    // Project 2: Add task
    let mut cmd2 = Command::cargo_bin("intent-engine").unwrap();
    cmd2.current_dir(temp_dir2.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Project2 Task")
        .assert()
        .success();

    // Verify project 1 only sees its own task
    let mut find1 = Command::cargo_bin("intent-engine").unwrap();
    let output1 = find1
        .current_dir(temp_dir1.path())
        .arg("task")
        .arg("find")
        .output()
        .unwrap();

    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    assert!(stdout1.contains("Project1 Task"));
    assert!(!stdout1.contains("Project2 Task"));

    // Verify project 2 only sees its own task
    let mut find2 = Command::cargo_bin("intent-engine").unwrap();
    let output2 = find2
        .current_dir(temp_dir2.path())
        .arg("task")
        .arg("find")
        .output()
        .unwrap();

    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout2.contains("Project2 Task"));
    assert!(!stdout2.contains("Project1 Task"));

    // Verify each project has its own .intent-engine
    assert!(temp_dir1.path().join(".intent-engine").exists());
    assert!(temp_dir2.path().join(".intent-engine").exists());
}

#[test]
fn test_cli_task_update_with_complexity_priority() {
    let temp_dir = setup_test_env();

    // Add a task
    let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
    add_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task")
        .assert()
        .success();

    // Update with complexity and priority
    let mut update_cmd = Command::cargo_bin("intent-engine").unwrap();
    update_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("update")
        .arg("1")
        .arg("--complexity")
        .arg("7")
        .arg("--priority")
        .arg("5");

    update_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("\"complexity\": 7"))
        .stdout(predicate::str::contains("\"priority\": 5"));
}

#[test]
fn test_cli_pick_next_tasks() {
    let temp_dir = setup_test_env();

    // Add multiple tasks with different priorities
    for i in 1..=3 {
        let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
        add_cmd
            .current_dir(temp_dir.path())
            .arg("task")
            .arg("add")
            .arg("--name")
            .arg(format!("Task {}", i))
            .assert()
            .success();

        // Set priority (higher priority for task 2)
        let mut update_cmd = Command::cargo_bin("intent-engine").unwrap();
        update_cmd
            .current_dir(temp_dir.path())
            .arg("task")
            .arg("update")
            .arg(i.to_string())
            .arg("--priority")
            .arg(if i == 2 { "10" } else { "1" })
            .assert()
            .success();
    }

    // Pick next tasks (should prioritize task 2)
    let mut pick_cmd = Command::cargo_bin("intent-engine").unwrap();
    let output = pick_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("pick-next")
        .arg("--max-count")
        .arg("2")
        .arg("--capacity")
        .arg("5")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should pick tasks and transition them to doing
    assert!(stdout.contains("\"status\": \"doing\""));
    // Task 2 should be first (highest priority)
    assert!(stdout.contains("Task 2"));
}

#[test]
fn test_cli_spawn_subtask() {
    let temp_dir = setup_test_env();

    // Add and start a parent task
    let mut add_cmd = Command::cargo_bin("intent-engine").unwrap();
    add_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Parent task")
        .assert()
        .success();

    let mut start_cmd = Command::cargo_bin("intent-engine").unwrap();
    start_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("1")
        .assert()
        .success();

    // Spawn subtask
    let mut spawn_cmd = Command::cargo_bin("intent-engine").unwrap();
    spawn_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("spawn-subtask")
        .arg("--name")
        .arg("Child task");

    spawn_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"Child task\""))
        .stdout(predicate::str::contains("\"parent_id\": 1"))
        .stdout(predicate::str::contains("\"status\": \"doing\""));

    // Verify current task was set to the child
    let mut current_cmd = Command::cargo_bin("intent-engine").unwrap();
    current_cmd
        .current_dir(temp_dir.path())
        .arg("current")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"current_task_id\": 2"));
}

#[test]
fn test_cli_switch_task() {
    let temp_dir = setup_test_env();

    // Add two tasks
    let mut add_cmd1 = Command::cargo_bin("intent-engine").unwrap();
    add_cmd1
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task 1")
        .assert()
        .success();

    let mut add_cmd2 = Command::cargo_bin("intent-engine").unwrap();
    add_cmd2
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task 2")
        .assert()
        .success();

    // Switch to task 2
    let mut switch_cmd = Command::cargo_bin("intent-engine").unwrap();
    switch_cmd
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("switch")
        .arg("2");

    switch_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\": 2"))
        .stdout(predicate::str::contains("\"status\": \"doing\""));

    // Verify current task was set
    let mut current_cmd = Command::cargo_bin("intent-engine").unwrap();
    current_cmd
        .current_dir(temp_dir.path())
        .arg("current")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"current_task_id\": 2"));
}
