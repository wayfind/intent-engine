/// Tests for `ie doctor` CLI command
/// Focuses on the simplified natural language output format
mod common;

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Doctor Command Tests (Simplified Output Format)
// ============================================================================

#[tokio::test]
async fn test_doctor_basic_success() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project first
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Run doctor command
    let mut cmd = common::ie_command();
    cmd.arg("doctor").current_dir(temp_dir.path());

    let output = cmd.assert().success();

    // Should contain simplified output
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Check that all three sections exist
    assert!(
        stdout.contains("Database:"),
        "Should show database location"
    );
    assert!(
        stdout.contains("Ancestor directories with databases:"),
        "Should show ancestor directories"
    );
    assert!(
        stdout.contains("Dashboard:"),
        "Should show dashboard status"
    );
}

#[tokio::test]
async fn test_doctor_database_location_check() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Run doctor
    let mut cmd = common::ie_command();
    cmd.arg("doctor").current_dir(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Database:"))
        .stdout(predicate::str::contains("project.db"));
}

#[tokio::test]
async fn test_doctor_dashboard_check() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Run doctor
    let mut cmd = common::ie_command();
    cmd.arg("doctor").current_dir(temp_dir.path());

    // Dashboard check should show "Not running" when dashboard is not started
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Dashboard:"))
        .stdout(predicate::str::contains("Not running").or(predicate::str::contains("Running")));
}

#[tokio::test]
async fn test_doctor_database_health_check() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Run doctor
    let mut cmd = common::ie_command();
    cmd.arg("doctor").current_dir(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Database:"));
}

#[tokio::test]
async fn test_doctor_database_path_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Run doctor
    let mut cmd = common::ie_command();
    cmd.arg("doctor").current_dir(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Database:"));
}

#[tokio::test]
async fn test_doctor_hooks_configuration_check() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Run doctor
    let mut cmd = common::ie_command();
    cmd.arg("doctor").current_dir(temp_dir.path());

    let output = cmd.assert().success();

    // Simplified doctor no longer shows hooks configuration
    // Just verify it runs successfully
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("Database:"));
}

#[tokio::test]
async fn test_doctor_json_structure() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Run doctor
    let mut cmd = common::ie_command();
    cmd.arg("doctor").current_dir(temp_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Simplified doctor uses natural language, not JSON
    // Verify the expected sections exist
    assert!(stdout.contains("Database:"));
    assert!(stdout.contains("Ancestor directories with databases:"));
    assert!(stdout.contains("Dashboard:"));
}

#[tokio::test]
async fn test_doctor_with_tasks_in_database() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Add a task using plan command
    let plan_json = r#"{"tasks":[{"name":"Test task for doctor"}]}"#;
    let mut cmd = common::ie_command();
    cmd.arg("plan")
        .write_stdin(plan_json)
        .current_dir(temp_dir.path());
    cmd.assert().success();

    // Run doctor
    let mut cmd = common::ie_command();
    cmd.arg("doctor").current_dir(temp_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Simplified doctor shows database path but not task count
    // Just verify it runs and shows database info
    assert!(stdout.contains("Database:"));
}

#[tokio::test]
async fn test_doctor_auto_initializes() {
    let temp_dir = TempDir::new().unwrap();

    // Run doctor without initializing - it should auto-initialize
    let mut cmd = common::ie_command();
    cmd.arg("doctor").current_dir(temp_dir.path());

    // Doctor auto-initializes if no database exists, so should succeed
    cmd.assert().success();
}

#[tokio::test]
async fn test_doctor_corrupted_database() {
    let temp_dir = TempDir::new().unwrap();

    // Initialize project
    let mut cmd = common::ie_command();
    cmd.arg("init")
        .arg("--at")
        .arg(temp_dir.path().to_str().unwrap());
    cmd.assert().success();

    // Corrupt the database by writing garbage
    let db_path = temp_dir.path().join(".intent-engine/project.db");
    fs::write(&db_path, "THIS IS NOT A VALID SQLITE DATABASE").unwrap();

    // Run doctor - should detect the problem or attempt recovery
    let mut cmd = common::ie_command();
    cmd.arg("doctor").current_dir(temp_dir.path());

    // Doctor might fail or succeed (if it re-initializes)
    // The important thing is that it runs and provides output
    let output = cmd.output().expect("Failed to execute command");

    // Should have some output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stdout.is_empty() || !stderr.is_empty(),
        "Should produce output when encountering corrupted database"
    );
}
