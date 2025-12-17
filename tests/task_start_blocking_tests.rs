// Tests in this file use CLI commands removed in v0.10.0
// v0.10.0 simplified CLI to just: plan, log, search
// These tests are kept for reference but disabled by default
#![cfg(feature = "test-removed-cli-commands")]
#![allow(deprecated)]

mod common;

use predicates::prelude::*;

#[test]
fn test_start_task_blocked_by_incomplete_dependency() {
    let temp_dir = common::setup_test_env();

    // Create two tasks
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

    // Make Task 2 depend on Task 1
    let mut depends = common::ie_command();
    depends
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("2")
        .arg("1")
        .assert()
        .success();

    // Try to start Task 2 (should fail because Task 1 is not done)
    let mut start = common::ie_command();
    start
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("2");

    start
        .assert()
        .failure()
        .stderr(predicate::str::contains("TASK_BLOCKED"))
        .stderr(predicate::str::contains("Task 2"))
        .stderr(predicate::str::contains("[1]"));
}

#[test]
fn test_start_task_allowed_after_dependency_completed() {
    let temp_dir = common::setup_test_env();

    // Create two tasks
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

    // Make Task 2 depend on Task 1
    let mut depends = common::ie_command();
    depends
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("2")
        .arg("1")
        .assert()
        .success();

    // Start and complete Task 1
    let mut start1 = common::ie_command();
    start1
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("1")
        .assert()
        .success();

    let mut done1 = common::ie_command();
    done1
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("done")
        .assert()
        .success();

    // Now Task 2 should be allowed to start
    let mut start2 = common::ie_command();
    start2
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("2");

    start2
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"doing\""));
}

#[test]
fn test_start_task_blocked_by_multiple_dependencies() {
    let temp_dir = common::setup_test_env();

    // Create three tasks
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

    // Task 3 depends on both Task 1 and Task 2
    let mut dep1 = common::ie_command();
    dep1.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("3")
        .arg("1")
        .assert()
        .success();

    let mut dep2 = common::ie_command();
    dep2.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("3")
        .arg("2")
        .assert()
        .success();

    // Try to start Task 3 (should fail because both Task 1 and Task 2 are not done)
    let mut start = common::ie_command();
    start
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("3");

    start
        .assert()
        .failure()
        .stderr(predicate::str::contains("TASK_BLOCKED"))
        .stderr(predicate::str::contains("Task 3"));
}

#[test]
fn test_start_task_with_partial_dependencies_completed() {
    let temp_dir = common::setup_test_env();

    // Create three tasks
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

    // Task 3 depends on both Task 1 and Task 2
    let mut dep1 = common::ie_command();
    dep1.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("3")
        .arg("1")
        .assert()
        .success();

    let mut dep2 = common::ie_command();
    dep2.current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("3")
        .arg("2")
        .assert()
        .success();

    // Complete Task 1 only
    let mut start1 = common::ie_command();
    start1
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("1")
        .assert()
        .success();

    let mut done1 = common::ie_command();
    done1
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("done")
        .assert()
        .success();

    // Try to start Task 3 (should still fail because Task 2 is not done)
    let mut start3 = common::ie_command();
    start3
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("3");

    start3
        .assert()
        .failure()
        .stderr(predicate::str::contains("TASK_BLOCKED"))
        .stderr(predicate::str::contains("[2]"));
}

#[test]
fn test_start_task_no_dependencies_allowed() {
    let temp_dir = common::setup_test_env();

    // Create a task with no dependencies
    let mut add = common::ie_command();
    add.current_dir(temp_dir.path())
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Independent Task")
        .assert()
        .success();

    // Should be able to start immediately
    let mut start = common::ie_command();
    start
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("1");

    start
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"doing\""));
}

#[test]
fn test_start_task_blocked_by_doing_dependency() {
    let temp_dir = common::setup_test_env();

    // Create two tasks
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

    // Make Task 2 depend on Task 1
    let mut depends = common::ie_command();
    depends
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("depends-on")
        .arg("2")
        .arg("1")
        .assert()
        .success();

    // Start Task 1 (but don't complete it)
    let mut start1 = common::ie_command();
    start1
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("1")
        .assert()
        .success();

    // Try to start Task 2 (should fail because Task 1 is doing, not done)
    let mut start2 = common::ie_command();
    start2
        .current_dir(temp_dir.path())
        .arg("task")
        .arg("start")
        .arg("2");

    start2
        .assert()
        .failure()
        .stderr(predicate::str::contains("TASK_BLOCKED"));
}
