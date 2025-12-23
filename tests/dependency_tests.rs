#![allow(deprecated)]
// Tests in this file use CLI commands removed in v0.10.0 (task add-dependency, etc.)
// v0.10.0 simplified CLI to just: plan, log, search
// These tests are kept for reference but disabled by default
#![cfg(feature = "test-removed-cli-commands")]

mod common;

use predicates::prelude::*;

#[test]
fn test_depends_on_success() {
    let temp_dir = common::setup_test_env();

    // Add two tasks
    let mut add1 = common::ie_command();
    add1.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task A")
        .assert()
        .success();

    let mut add2 = common::ie_command();
    add2.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task B")
        .assert()
        .success();

    // Add dependency: Task 2 depends on Task 1
    let mut depends = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Add two tasks
    let mut add1 = common::ie_command();
    add1.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task A")
        .assert()
        .success();

    let mut add2 = common::ie_command();
    add2.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task B")
        .assert()
        .success();

    // Add dependency: Task 1 depends on Task 2
    let mut depends1 = common::ie_command();
    depends1
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("1")
        .arg("2")
        .assert()
        .success();

    // Try to add reverse dependency: Task 2 depends on Task 1 (would create cycle)
    let mut depends2 = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Add three tasks
    for i in 1..=3 {
        let mut add = common::ie_command();
        add.current_dir(temp_dir.path())
            .arg("task")
            .arg("add")
            .arg("--name")
            .arg(format!("Task {}", i))
            .assert()
            .success();
    }

    // Create chain: Task 1 depends on Task 2
    let mut dep1 = common::ie_command();
    dep1.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("1")
        .arg("2")
        .assert()
        .success();

    // Task 2 depends on Task 3
    let mut dep2 = common::ie_command();
    dep2.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("2")
        .arg("3")
        .assert()
        .success();

    // Try to create cycle: Task 3 depends on Task 1
    let mut dep3 = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Add a task
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task A")
        .assert()
        .success();

    // Try to make task depend on itself
    let mut depends = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Add one task
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task A")
        .assert()
        .success();

    // Try to add dependency with nonexistent blocking task
    let mut depends = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Add one task
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task A")
        .assert()
        .success();

    // Try to add dependency with nonexistent blocked task
    let mut depends = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Create a chain of 5 tasks
    for i in 1..=5 {
        let mut add = common::ie_command();
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
        let mut dep = common::ie_command();
        dep.current_dir(temp_dir.path())
            .arg("task")
            .arg("depends-on")
            .arg(i.to_string())
            .arg((i + 1).to_string())
            .assert()
            .success();
    }

    // Try to close the loop: 5->1
    let mut dep = common::ie_command();
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
    let temp_dir = common::setup_test_env();

    // Add three tasks
    for i in 1..=3 {
        let mut add = common::ie_command();
        add.current_dir(temp_dir.path())
            .arg("task")
            .arg("add")
            .arg("--name")
            .arg(format!("Task {}", i))
            .assert()
            .success();
    }

    // Task 1 depends on both Task 2 and Task 3
    let mut dep1 = common::ie_command();
    dep1.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("1")
        .arg("2")
        .assert()
        .success();

    let mut dep2 = common::ie_command();
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
