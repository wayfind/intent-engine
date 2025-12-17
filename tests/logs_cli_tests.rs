// Tests in this file use CLI commands removed in v0.10.0
// v0.10.0 simplified CLI to just: plan, log, search
// These tests are kept for reference but disabled by default
#![cfg(feature = "test-removed-cli-commands")]

//! Tests for `ie logs` CLI command to improve main.rs coverage
//! Focuses on the handle_logs_command() function and its various branches

mod common;

use predicates::prelude::*;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

// Helper to get log directory
fn log_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".intent-engine")
        .join("logs")
}

// Helper to create test log entries
fn create_test_log_file(mode: &str) -> PathBuf {
    let log_dir = log_dir();
    fs::create_dir_all(&log_dir).ok();

    let log_file = log_dir.join(format!("{}.log", mode));
    let mut file = File::create(&log_file).unwrap();

    // Write some test log entries
    writeln!(
        file,
        "2025-11-22T10:00:00.000000000+00:00  INFO test::module: First message"
    )
    .unwrap();
    writeln!(
        file,
        "2025-11-22T10:01:00.000000000+00:00  ERROR test::error: Error occurred"
    )
    .unwrap();
    writeln!(
        file,
        "2025-11-22T10:02:00.000000000+00:00  WARN test::warn: Warning message"
    )
    .unwrap();
    writeln!(
        file,
        "2025-11-22T10:03:00.000000000+00:00  DEBUG test::debug: Debug info"
    )
    .unwrap();
    writeln!(
        file,
        "2025-11-22T10:04:00.000000000+00:00  INFO test::module: Last message"
    )
    .unwrap();

    log_file
}

// ============================================================================
// Basic Logs Command Tests
// ============================================================================

#[test]
fn test_logs_command_basic() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs");

    cmd.assert().success();
}

#[test]
fn test_logs_command_with_mode_filter() {
    create_test_log_file("dashboard");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--mode").arg("dashboard");

    cmd.assert().success();
}

#[test]
fn test_logs_command_with_level_filter() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--level").arg("ERROR");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ERROR").or(predicate::str::is_empty()))
        .stderr(predicate::str::is_empty().or(predicate::str::contains("No log entries")));
}

#[test]
fn test_logs_command_with_limit() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--limit").arg("2");

    cmd.assert().success();
}

// ============================================================================
// Time-based Filtering Tests
// ============================================================================

#[test]
fn test_logs_command_with_since_duration() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--since").arg("24h");

    cmd.assert().success();
}

#[test]
fn test_logs_command_with_since_duration_days() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--since").arg("7d");

    cmd.assert().success();
}

#[test]
fn test_logs_command_with_since_duration_minutes() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--since").arg("30m");

    cmd.assert().success();
}

#[test]
fn test_logs_command_with_invalid_since_format() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--since").arg("invalid");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid duration format"));
}

#[test]
fn test_logs_command_with_until_timestamp() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs")
        .arg("--until")
        .arg("2025-11-22T12:00:00+00:00");

    cmd.assert().success();
}

#[test]
fn test_logs_command_with_invalid_until_timestamp() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--until").arg("not-a-timestamp");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid timestamp format"));
}

#[test]
fn test_logs_command_with_both_since_and_until() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs")
        .arg("--since")
        .arg("48h")
        .arg("--until")
        .arg("2025-11-22T12:00:00+00:00");

    cmd.assert().success();
}

// ============================================================================
// Export Format Tests
// ============================================================================

#[test]
fn test_logs_command_json_export() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--export").arg("json");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check for JSON array format in stdout, or "No log entries" in stderr
    assert!(
        (stdout.contains("[") && stdout.contains("]")) || stderr.contains("No log entries"),
        "Expected JSON array format or no entries message. stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_logs_command_text_export_default() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs");

    // Default export should be text format (not JSON array)
    cmd.assert().success();
}

// ============================================================================
// Edge Cases and Error Paths
// ============================================================================

#[test]
fn test_logs_command_no_matching_entries() {
    // Create log file but query for non-existent mode
    create_test_log_file("dashboard");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--mode").arg("nonexistent-mode");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("No log entries found").or(predicate::str::is_empty()));
}

#[test]
fn test_logs_command_with_multiple_filters() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs")
        .arg("--mode")
        .arg("cli")
        .arg("--level")
        .arg("INFO")
        .arg("--limit")
        .arg("10")
        .arg("--since")
        .arg("24h");

    cmd.assert().success();
}

#[test]
fn test_logs_command_mode_mcp_server() {
    create_test_log_file("mcp-server");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--mode").arg("mcp-server");

    cmd.assert().success();
}

#[test]
fn test_logs_command_level_warn() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--level").arg("WARN");

    cmd.assert().success();
}

#[test]
fn test_logs_command_level_debug() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--level").arg("DEBUG");

    cmd.assert().success();
}

// Note: --follow mode is intentionally not tested here as it would hang
// It requires special handling (background process, timeout, etc.)
// This is documented as a known limitation for the test suite

// ============================================================================
// Additional Error Path Tests
// ============================================================================

#[test]
fn test_logs_command_invalid_mode() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    // Invalid mode should still succeed but just return no results
    cmd.arg("logs").arg("--mode").arg("nonexistent-mode");

    cmd.assert().success();
}

#[test]
fn test_logs_command_invalid_level() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    // Invalid level should still succeed but return no results
    cmd.arg("logs").arg("--level").arg("INVALID_LEVEL");

    cmd.assert().success();
}

// ============================================================================
// JSON Export Format Validation
// ============================================================================

#[test]
fn test_logs_json_export_with_filters() {
    create_test_log_file("cli");

    let mut cmd = common::ie_command();
    cmd.arg("logs")
        .arg("--level")
        .arg("INFO")
        .arg("--export")
        .arg("json");

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Either we have JSON array in stdout, or "No log entries" in stderr
    assert!(
        (stdout.contains("[") && stdout.contains("]")) || stderr.contains("No log entries"),
        "Expected JSON array format or no entries message"
    );
}

#[test]
fn test_logs_command_limit_zero() {
    create_test_log_file("cli");

    // Test with limit=0 (should return no results or handle gracefully)
    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--limit").arg("0");

    cmd.assert().success();
}

#[test]
fn test_logs_command_very_large_limit() {
    create_test_log_file("cli");

    // Test with very large limit
    let mut cmd = common::ie_command();
    cmd.arg("logs").arg("--limit").arg("9999");

    cmd.assert().success();
}
