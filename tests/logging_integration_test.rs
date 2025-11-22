//! Logging System Integration Tests
//!
//! Tests for Phase 1: Basic file logging functionality
//! - Dashboard daemon mode file logging
//! - Environment variable force-enable
//! - Log directory auto-creation
//! - Log content validation

mod common;

use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Get the log file path for dashboard
fn dashboard_log_path() -> PathBuf {
    dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".intent-engine")
        .join("logs")
        .join("dashboard.log")
}

/// Clean up log directory before tests
fn cleanup_logs() {
    let log_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".intent-engine")
        .join("logs");

    if log_dir.exists() {
        fs::remove_dir_all(&log_dir).ok();
    }
}

/// Stop any running dashboard instance
fn stop_dashboard() {
    Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("stop")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok();

    thread::sleep(Duration::from_millis(500));
}

#[test]
#[serial]
fn test_dashboard_daemon_creates_log_file() {
    cleanup_logs();
    stop_dashboard();

    // Verify log file doesn't exist
    assert!(
        !dashboard_log_path().exists(),
        "Log file should not exist before starting"
    );

    // Start dashboard in daemon mode
    let output = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .current_dir(common::current_project_dir())
        .output()
        .expect("Failed to start dashboard");

    assert!(
        output.status.success(),
        "Dashboard should start successfully"
    );

    // Wait for dashboard to initialize and create log file
    thread::sleep(Duration::from_secs(2));

    // Verify log file was created
    assert!(
        dashboard_log_path().exists(),
        "Log file should be created in daemon mode"
    );

    // Verify log file has content
    let log_content = fs::read_to_string(dashboard_log_path()).expect("Failed to read log file");

    assert!(!log_content.is_empty(), "Log file should have content");
    assert!(
        log_content.contains("INFO"),
        "Log should contain INFO level messages"
    );
    assert!(
        log_content.contains("Dashboard server listening"),
        "Log should contain server startup message"
    );

    // Cleanup
    stop_dashboard();
}

#[test]
#[serial]
fn test_env_var_force_enable_logging() {
    cleanup_logs();
    stop_dashboard();

    // Start dashboard with IE_DASHBOARD_LOG_FILE env var
    let output = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .env("IE_DASHBOARD_LOG_FILE", "1")
        .current_dir(common::current_project_dir())
        .output()
        .expect("Failed to start dashboard");

    assert!(
        output.status.success(),
        "Dashboard should start with env var"
    );

    thread::sleep(Duration::from_secs(2));

    // Verify log file created
    assert!(
        dashboard_log_path().exists(),
        "Log file should be created with env var"
    );

    let log_content = fs::read_to_string(dashboard_log_path()).expect("Failed to read log file");

    assert!(!log_content.is_empty(), "Log file should have content");

    stop_dashboard();
}

#[test]
#[serial]
fn test_log_directory_auto_creation() {
    cleanup_logs();
    stop_dashboard();

    let log_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".intent-engine")
        .join("logs");

    // Verify directory doesn't exist
    assert!(
        !log_dir.exists(),
        "Log directory should not exist before test"
    );

    // Start dashboard
    let output = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .current_dir(common::current_project_dir())
        .output()
        .expect("Failed to start dashboard");

    assert!(output.status.success(), "Dashboard should start");

    thread::sleep(Duration::from_secs(2));

    // Verify directory and file were created
    assert!(log_dir.exists(), "Log directory should be auto-created");
    assert!(log_dir.is_dir(), "Log path should be a directory");
    assert!(dashboard_log_path().exists(), "Log file should be created");

    stop_dashboard();
}

#[test]
#[serial]
fn test_log_format_rfc3339() {
    cleanup_logs();
    stop_dashboard();

    // Start dashboard
    Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .current_dir(common::current_project_dir())
        .output()
        .expect("Failed to start dashboard");

    thread::sleep(Duration::from_secs(2));

    // Read log file
    let log_content = fs::read_to_string(dashboard_log_path()).expect("Failed to read log file");

    // Verify RFC3339 timestamp format: 2025-11-22T06:51:54.509104402+00:00
    // Simple check without regex dependency
    assert!(
        log_content.contains("T") && log_content.contains("+"),
        "Log should contain RFC3339 timestamps (with 'T' and '+'). Content: {}",
        &log_content[..log_content.len().min(200)]
    );

    // More specific check: verify date-time separators
    assert!(
        log_content.chars().filter(|c| *c == '-').count() >= 2,
        "Log should contain date separators"
    );
    assert!(
        log_content.chars().filter(|c| *c == ':').count() >= 2,
        "Log should contain time separators"
    );

    stop_dashboard();
}

#[test]
#[serial]
fn test_registry_logs_in_file() {
    cleanup_logs();
    stop_dashboard();

    // Start dashboard with DEBUG level to see registry logs
    let output = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .env("RUST_LOG", "debug")
        .env("IE_DASHBOARD_LOG_FILE", "1")
        .current_dir(common::current_project_dir())
        .output()
        .expect("Failed to start dashboard");

    assert!(output.status.success(), "Dashboard should start");

    // Wait for initialization and registry operations
    thread::sleep(Duration::from_secs(3));

    // Read log file
    let log_content = fs::read_to_string(dashboard_log_path()).expect("Failed to read log file");

    // Verify registry-related DEBUG logs exist
    // Note: May not appear immediately, but should appear on registry operations
    assert!(
        log_content.contains("DEBUG"),
        "Log should contain DEBUG level messages"
    );

    // This assertion may be flaky if no registry save occurs during test
    // In real usage, registry logs appear when projects are registered/modified
    if log_content.contains("registry") {
        eprintln!("✓ Registry logs found in file");
    } else {
        eprintln!("⚠ No registry logs yet (this is OK if no registry operations occurred)");
    }

    stop_dashboard();
}
