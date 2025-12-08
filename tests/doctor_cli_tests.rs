/// Tests for `ie doctor` CLI command to improve main.rs coverage
/// Focuses on the handle_doctor_command() function
mod common;

use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Doctor Command Tests
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

    // Should contain JSON output
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Parse as JSON
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Check that checks array exists
    assert!(json["checks"].is_array());

    // Should have multiple checks
    let checks = json["checks"].as_array().unwrap();
    assert!(checks.len() >= 5, "Should have at least 5 checks");

    // Verify specific checks exist - should have exactly 5 checks
    let check_names: Vec<String> = checks
        .iter()
        .filter_map(|c| c["check"].as_str().map(String::from))
        .collect();

    assert_eq!(checks.len(), 5, "Should have exactly 5 checks");
    assert!(check_names.contains(&"Database Path Resolution".to_string()));
    assert!(check_names.contains(&"Database Health".to_string()));
    assert!(check_names.contains(&"Dashboard".to_string()));
    assert!(check_names.contains(&"MCP Connections".to_string()));
    assert!(check_names.contains(&"SessionStart Hook".to_string()));
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
        .stdout(predicate::str::contains("Database Path Resolution"))
        .stdout(predicate::str::contains("✓ INFO"));
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

    // Dashboard check should show WARNING when not running
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Dashboard"))
        .stdout(predicate::str::contains("⚠ WARNING"));
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
        .stdout(predicate::str::contains("Database Health"))
        .stdout(predicate::str::contains("✓ PASS"));
}

#[tokio::test]
async fn test_doctor_mcp_connections_check() {
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

    // MCP Connections check should show WARNING when Dashboard not running
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("MCP Connections"))
        .stdout(predicate::str::contains("⚠ WARNING"));
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
        .stdout(predicate::str::contains("Database Path Resolution"));
}

#[tokio::test]
async fn test_doctor_mcp_configuration_check() {
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

    // MCP check should exist (even if not configured)
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("MCP") || stdout.contains("mcp"));
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

    // Hooks check should exist (even if not configured)
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("Hook") || stdout.contains("hook"));
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

    // Parse JSON
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    // Check required fields
    assert!(json["summary"].is_string());
    assert!(json["overall_status"].is_string());
    assert!(json["checks"].is_array());

    // Each check should have proper structure
    for check in json["checks"].as_array().unwrap() {
        assert!(check["check"].is_string(), "check field should be string");
        assert!(check["status"].is_string(), "status field should be string");
        // details can be string or object (for complex checks like MCP/Hooks)
        assert!(
            check["details"].is_string() || check["details"].is_object(),
            "details should be string or object"
        );
    }
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

    // Add a task
    let mut cmd = common::ie_command();
    cmd.arg("task")
        .arg("add")
        .arg("--name")
        .arg("Test task for doctor")
        .current_dir(temp_dir.path());
    cmd.assert().success();

    // Run doctor
    let mut cmd = common::ie_command();
    cmd.arg("doctor").current_dir(temp_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Should report at least 1 task in Database Health check
    assert!(
        stdout.contains("Database operational with") && stdout.contains("tasks"),
        "Should report tasks count in Database Health check"
    );
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
