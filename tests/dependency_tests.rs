#![allow(deprecated)]

use assert_cmd::cargo;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
    TempDir::new().unwrap()
}

#[test]
fn test_depends_on_success() {
    let temp_dir = setup_test_env();

    // Add two tasks
    let mut add1 = Command::new(cargo::cargo_bin!("intent-engine"));
    add1.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task A")
        .assert()
        .success();

    let mut add2 = Command::new(cargo::cargo_bin!("intent-engine"));
    add2.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task B")
        .assert()
        .success();

    // Add dependency: Task 2 depends on Task 1
    let mut depends = Command::new(cargo::cargo_bin!("intent-engine"));
    depends
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("2") // blocked_task_id
        .arg("1"); // blocking_task_id

    depends
        .assert()
        .success()
        .stdout(predicate::str::contains("dependency_id"))
        .stdout(predicate::str::contains("\"blocking_task_id\": 1"))
        .stdout(predicate::str::contains("\"blocked_task_id\": 2"))
        .stdout(predicate::str::contains("Task 2 now depends on Task 1"));
}

#[test]
fn test_depends_on_direct_cycle() {
    let temp_dir = setup_test_env();

    // Add two tasks
    let mut add1 = Command::new(cargo::cargo_bin!("intent-engine"));
    add1.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task A")
        .assert()
        .success();

    let mut add2 = Command::new(cargo::cargo_bin!("intent-engine"));
    add2.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task B")
        .assert()
        .success();

    // Add dependency: Task 1 depends on Task 2
    let mut depends1 = Command::new(cargo::cargo_bin!("intent-engine"));
    depends1
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("1")
        .arg("2")
        .assert()
        .success();

    // Try to add reverse dependency: Task 2 depends on Task 1 (would create cycle)
    let mut depends2 = Command::new(cargo::cargo_bin!("intent-engine"));
    depends2
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("2")
        .arg("1");

    depends2
        .assert()
        .failure()
        .stderr(predicate::str::contains("CIRCULAR_DEPENDENCY"))
        .stderr(predicate::str::contains("would create a cycle"));
}

#[test]
fn test_depends_on_transitive_cycle() {
    let temp_dir = setup_test_env();

    // Add three tasks
    for i in 1..=3 {
        let mut add = Command::new(cargo::cargo_bin!("intent-engine"));
        add.current_dir(temp_dir.path())
            .arg("task")
            .arg("add")
            .arg("--name")
            .arg(format!("Task {}", i))
            .assert()
            .success();
    }

    // Create chain: Task 1 depends on Task 2
    let mut dep1 = Command::new(cargo::cargo_bin!("intent-engine"));
    dep1.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("1")
        .arg("2")
        .assert()
        .success();

    // Task 2 depends on Task 3
    let mut dep2 = Command::new(cargo::cargo_bin!("intent-engine"));
    dep2.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("2")
        .arg("3")
        .assert()
        .success();

    // Try to create cycle: Task 3 depends on Task 1
    let mut dep3 = Command::new(cargo::cargo_bin!("intent-engine"));
    dep3.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("3")
        .arg("1");

    dep3.assert()
        .failure()
        .stderr(predicate::str::contains("CIRCULAR_DEPENDENCY"));
}

#[test]
fn test_depends_on_self_dependency() {
    let temp_dir = setup_test_env();

    // Add a task
    let mut add = Command::new(cargo::cargo_bin!("intent-engine"));
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task A")
        .assert()
        .success();

    // Try to make task depend on itself
    let mut depends = Command::new(cargo::cargo_bin!("intent-engine"));
    depends
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("1")
        .arg("1");

    depends
        .assert()
        .failure()
        .stderr(predicate::str::contains("CIRCULAR_DEPENDENCY"));
}

#[test]
fn test_depends_on_nonexistent_blocking_task() {
    let temp_dir = setup_test_env();

    // Add one task
    let mut add = Command::new(cargo::cargo_bin!("intent-engine"));
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task A")
        .assert()
        .success();

    // Try to add dependency with nonexistent blocking task
    let mut depends = Command::new(cargo::cargo_bin!("intent-engine"));
    depends
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("1")
        .arg("999");

    depends
        .assert()
        .failure()
        .stderr(predicate::str::contains("TASK_NOT_FOUND"))
        .stderr(predicate::str::contains("999"));
}

#[test]
fn test_depends_on_nonexistent_blocked_task() {
    let temp_dir = setup_test_env();

    // Add one task
    let mut add = Command::new(cargo::cargo_bin!("intent-engine"));
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task A")
        .assert()
        .success();

    // Try to add dependency with nonexistent blocked task
    let mut depends = Command::new(cargo::cargo_bin!("intent-engine"));
    depends
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("999")
        .arg("1");

    depends
        .assert()
        .failure()
        .stderr(predicate::str::contains("TASK_NOT_FOUND"))
        .stderr(predicate::str::contains("999"));
}

#[test]
fn test_depends_on_deep_chain() {
    let temp_dir = setup_test_env();

    // Create a chain of 5 tasks
    for i in 1..=5 {
        let mut add = Command::new(cargo::cargo_bin!("intent-engine"));
        add.current_dir(temp_dir.path())
            .arg("task")
            .arg("add")
            .arg("--name")
            .arg(format!("Task {}", i))
            .assert()
            .success();
    }

    // Build chain: 1->2->3->4->5
    for i in 1..5 {
        let mut dep = Command::new(cargo::cargo_bin!("intent-engine"));
        dep.current_dir(temp_dir.path())
            .arg("task")
            .arg("depends-on")
            .arg(i.to_string())
            .arg((i + 1).to_string())
            .assert()
            .success();
    }

    // Try to close the loop: 5->1
    let mut dep = Command::new(cargo::cargo_bin!("intent-engine"));
    dep.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("5")
        .arg("1");

    dep.assert()
        .failure()
        .stderr(predicate::str::contains("CIRCULAR_DEPENDENCY"));
}

#[test]
fn test_depends_on_multiple_dependencies() {
    let temp_dir = setup_test_env();

    // Add three tasks
    for i in 1..=3 {
        let mut add = Command::new(cargo::cargo_bin!("intent-engine"));
        add.current_dir(temp_dir.path())
            .arg("task")
            .arg("add")
            .arg("--name")
            .arg(format!("Task {}", i))
            .assert()
            .success();
    }

    // Task 1 depends on both Task 2 and Task 3
    let mut dep1 = Command::new(cargo::cargo_bin!("intent-engine"));
    dep1.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("1")
        .arg("2")
        .assert()
        .success();

    let mut dep2 = Command::new(cargo::cargo_bin!("intent-engine"));
    dep2.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("1")
        .arg("3")
        .assert()
        .success();

    // Both dependencies should be added successfully
    // (Testing that we can have multiple blocking tasks)
}
