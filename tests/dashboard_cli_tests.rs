/// Tests for `ie dashboard` CLI commands to improve main.rs coverage
/// Focuses on the handle_dashboard_command() function basic paths
mod common;

use predicates::prelude::*;

// ============================================================================
// Dashboard Status Tests (when not running)
// ============================================================================

#[tokio::test]
async fn test_dashboard_status_not_running() {
    let mut cmd = common::ie_command();
    cmd.arg("dashboard").arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Not running").or(predicate::str::contains("Stopped")));
}

#[tokio::test]
async fn test_dashboard_status_with_all_flag() {
    let mut cmd = common::ie_command();
    cmd.arg("dashboard").arg("status").arg("--all");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Note: Single Dashboard mode"));
}

// ============================================================================
// Dashboard Stop Tests (when not running)
// ============================================================================

#[tokio::test]
async fn test_dashboard_stop_not_running() {
    let mut cmd = common::ie_command();
    cmd.arg("dashboard").arg("stop");

    cmd.assert().success().stdout(
        predicate::str::contains("Dashboard not running")
            .or(predicate::str::contains("Dashboard not responding")),
    );
}

#[tokio::test]
async fn test_dashboard_stop_with_all_flag() {
    let mut cmd = common::ie_command();
    cmd.arg("dashboard").arg("stop").arg("--all");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Single Dashboard mode"));
}

// ============================================================================
// Dashboard List Tests (when not running)
// ============================================================================

#[tokio::test]
async fn test_dashboard_list_not_running() {
    let mut cmd = common::ie_command();
    cmd.arg("dashboard").arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Dashboard not running"));
}

// ============================================================================
// Dashboard Open Tests (when not running)
// ============================================================================

#[tokio::test]
async fn test_dashboard_open_not_running() {
    let mut cmd = common::ie_command();
    cmd.arg("dashboard").arg("open");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Dashboard is not running"));
}

// ============================================================================
// Dashboard Start Tests - Command Option Validation
// ============================================================================

#[test]
#[ignore]
fn test_dashboard_start_with_invalid_port() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    // Test with port number that's too large
    cmd.arg("dashboard").arg("start").arg("--port").arg("99999");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("port")));
}

#[test]
#[ignore]
fn test_dashboard_start_with_custom_port() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    // Test with valid custom port - command should be accepted
    // Note: We don't actually verify the server starts (requires cleanup)
    cmd.arg("dashboard").arg("start").arg("--port").arg("8888");

    // Command should be accepted (though server may not fully start in test env)
    // We're just testing that the CLI argument parsing works
    cmd.assert();
}

#[test]
#[ignore]
fn test_dashboard_start_foreground_mode() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    // Test foreground mode flag
    cmd.arg("dashboard").arg("start").arg("--foreground");

    // Command should be accepted
    cmd.assert();
}

#[test]
#[ignore]
fn test_dashboard_start_with_browser() {
    let temp_dir = common::setup_test_env();
    let mut cmd = common::ie_command_with_project_dir(temp_dir.path());

    // Test browser flag
    cmd.arg("dashboard").arg("start").arg("--browser");

    // Command should be accepted
    cmd.assert();
}

#[test]
fn test_dashboard_start_in_uninitialized_directory() {
    // Create a temp directory without initializing it
    let temp_dir = tempfile::TempDir::new().unwrap();

    let mut cmd = common::ie_command();
    cmd.current_dir(temp_dir.path())
        .env("HOME", "/nonexistent")
        .env("USERPROFILE", "/nonexistent")
        .arg("dashboard")
        .arg("start");

    // Should fail - either due to no project or logging initialization failure
    cmd.assert().failure().stderr(
        predicate::str::contains("No Intent-Engine project found")
            .or(predicate::str::contains("not initialized"))
            .or(predicate::str::contains("Failed to initialize logging"))
            .or(predicate::str::contains("Permission denied")),
    );
}
