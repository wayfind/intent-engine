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
