/// Tests for `ie report` CLI command to improve main.rs coverage
/// Focuses on the handle_report_command() function
mod common;

use serde_json::Value;
use tempfile::TempDir;

// ============================================================================
// Report Command Tests
// ============================================================================

#[tokio::test]
async fn test_report_basic() {
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
        .arg("Test task for report")
        .current_dir(temp_dir.path());
    cmd.assert().success();

    // Generate report
    let mut cmd = common::ie_command();
    cmd.arg("report").current_dir(temp_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Should be valid JSON
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    assert!(json.is_object());
}

#[tokio::test]
async fn test_report_with_since_filter() {
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
        .arg("Test task")
        .current_dir(temp_dir.path());
    cmd.assert().success();

    // Generate report with since filter
    let mut cmd = common::ie_command();
    cmd.arg("report")
        .arg("--since")
        .arg("24h")
        .current_dir(temp_dir.path());

    cmd.assert().success();
}

#[tokio::test]
async fn test_report_with_status_filter() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Add tasks
    let mut cmd = common::ie_command();
    cmd.arg("task")
        .arg("add")
        .arg("--name")
        .arg("Todo task")
        .current_dir(temp_dir.path());
    cmd.assert().success();

    // Generate report with status filter
    let mut cmd = common::ie_command();
    cmd.arg("report")
        .arg("--status")
        .arg("todo")
        .current_dir(temp_dir.path());

    cmd.assert().success();
}

#[tokio::test]
async fn test_report_with_name_filter() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Add task
    let mut cmd = common::ie_command();
    cmd.arg("task")
        .arg("add")
        .arg("--name")
        .arg("Important task")
        .current_dir(temp_dir.path());
    cmd.assert().success();

    // Generate report with name filter
    let mut cmd = common::ie_command();
    cmd.arg("report")
        .arg("--filter-name")
        .arg("Important")
        .current_dir(temp_dir.path());

    cmd.assert().success();
}

#[tokio::test]
async fn test_report_with_spec_filter() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Add task with spec
    let mut cmd = common::ie_command();
    cmd.arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task with spec")
        .current_dir(temp_dir.path());
    cmd.assert().success();

    // Generate report with spec filter
    let mut cmd = common::ie_command();
    cmd.arg("report")
        .arg("--filter-spec")
        .arg("spec")
        .current_dir(temp_dir.path());

    cmd.assert().success();
}

#[tokio::test]
async fn test_report_summary_only() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Add task
    let mut cmd = common::ie_command();
    cmd.arg("task")
        .arg("add")
        .arg("--name")
        .arg("Task for summary")
        .current_dir(temp_dir.path());
    cmd.assert().success();

    // Generate summary-only report
    let mut cmd = common::ie_command();
    cmd.arg("report")
        .arg("--summary-only")
        .current_dir(temp_dir.path());

    cmd.assert().success();
}

#[tokio::test]
async fn test_report_combined_filters() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Add task
    let mut cmd = common::ie_command();
    cmd.arg("task")
        .arg("add")
        .arg("--name")
        .arg("Combined filter task")
        .current_dir(temp_dir.path());
    cmd.assert().success();

    // Generate report with multiple filters
    let mut cmd = common::ie_command();
    cmd.arg("report")
        .arg("--since")
        .arg("7d")
        .arg("--status")
        .arg("todo")
        .arg("--summary-only")
        .current_dir(temp_dir.path());

    cmd.assert().success();
}

#[tokio::test]
async fn test_report_empty_database() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project (no tasks)
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Generate report with no tasks
    let mut cmd = common::ie_command();
    cmd.arg("report").current_dir(temp_dir.path());

    cmd.assert().success();
}

#[tokio::test]
async fn test_report_without_init() {
    let temp_dir = TempDir::new().unwrap();

    // Try to generate report without initializing
    let mut cmd = common::ie_command();
    cmd.arg("report").current_dir(temp_dir.path());

    // Should fail
    cmd.assert().failure();
}
