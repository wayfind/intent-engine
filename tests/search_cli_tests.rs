/// Tests for `ie search` CLI command to improve main.rs coverage
/// Focuses on the handle_search_command() function
mod common;

use predicates::prelude::*;
use tempfile::TempDir;

// ============================================================================
// Search Command Tests
// ============================================================================

#[tokio::test]
async fn test_search_basic() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Add a task
    let mut cmd = common::ie_command();
    cmd.arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task for search")
        .current_dir(temp_dir.path());
    cmd.assert().success();

    // Search for the task
    let mut cmd = common::ie_command();
    cmd.arg("search").arg("search").current_dir(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test task for search"));
}

#[tokio::test]
async fn test_search_with_limit() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Search with limit
    let mut cmd = common::ie_command();
    cmd.arg("search")
        .arg("test")
        .arg("--limit")
        .arg("5")
        .current_dir(temp_dir.path());

    cmd.assert().success();
}

#[tokio::test]
async fn test_search_tasks_only() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Search tasks only
    let mut cmd = common::ie_command();
    cmd.arg("search")
        .arg("test")
        .arg("--tasks")
        .current_dir(temp_dir.path());

    cmd.assert().success();
}

#[tokio::test]
async fn test_search_events_only() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Search events only
    let mut cmd = common::ie_command();
    cmd.arg("search")
        .arg("test")
        .arg("--events")
        .current_dir(temp_dir.path());

    cmd.assert().success();
}

#[tokio::test]
async fn test_search_both_tasks_and_events() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Search both (default)
    let mut cmd = common::ie_command();
    cmd.arg("search")
        .arg("test")
        .arg("--tasks")
        .arg("--events")
        .current_dir(temp_dir.path());

    cmd.assert().success();
}

#[tokio::test]
async fn test_search_no_results() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Search for something that doesn't exist
    let mut cmd = common::ie_command();
    cmd.arg("search")
        .arg("nonexistent_unique_string_xyz")
        .current_dir(temp_dir.path());

    cmd.assert().success();
}
