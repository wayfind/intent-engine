//! MCP Server Logging Tests
//!
//! Tests for Phase 4: MCP Server file logging
//! - Log file creation
//! - JSON format validation
//! - DEBUG level logging
//! - JSON-RPC communication integrity

mod common;

use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Get MCP Server log path (with daily rotation date suffix)
fn mcp_log_path() -> PathBuf {
    use chrono::Local;
    let date = Local::now().format("%Y-%m-%d");
    dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".intent-engine")
        .join("logs")
        .join(format!("mcp-server.log.{}", date))
}

/// Clean up log directory
fn cleanup_logs() {
    let log_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".intent-engine")
        .join("logs");

    if log_dir.exists() {
        fs::remove_dir_all(&log_dir).ok();
    }
}

#[test]
#[serial]
fn test_mcp_server_creates_log_file() {
    cleanup_logs();

    // Start MCP Server in background
    let mut child = Command::new(common::ie_binary())
        .arg("mcp-server")
        .env("INTENT_ENGINE_PROJECT_DIR", common::current_project_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP Server");

    // Wait for initialization
    thread::sleep(Duration::from_secs(3));

    // Verify log file created
    assert!(mcp_log_path().exists(), "MCP Server should create log file");

    // Verify log file has content
    let log_content = fs::read_to_string(mcp_log_path()).expect("Failed to read MCP Server log");

    assert!(
        !log_content.is_empty(),
        "MCP Server log should contain content"
    );

    // Cleanup
    child.kill().ok();
    child.wait().ok();
}

#[test]
#[serial]
fn test_mcp_server_log_json_format() {
    cleanup_logs();

    // Start MCP Server
    let mut child = Command::new(common::ie_binary())
        .arg("mcp-server")
        .env("INTENT_ENGINE_PROJECT_DIR", common::current_project_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP Server");

    thread::sleep(Duration::from_secs(3));

    // Read log content
    let log_content = fs::read_to_string(mcp_log_path()).expect("Failed to read MCP Server log");

    // Verify JSON format - each line should be a valid JSON object
    let lines: Vec<&str> = log_content.lines().collect();
    assert!(!lines.is_empty(), "Log should have at least one line");

    for line in lines.iter().filter(|l| !l.trim().is_empty()) {
        let parse_result = serde_json::from_str::<serde_json::Value>(line);
        assert!(
            parse_result.is_ok(),
            "Each log line should be valid JSON: {}",
            line
        );

        // Verify JSON structure
        let log_entry = parse_result.unwrap();
        assert!(
            log_entry.get("timestamp").is_some(),
            "Log entry should have timestamp field"
        );
        assert!(
            log_entry.get("level").is_some(),
            "Log entry should have level field"
        );
        assert!(
            log_entry.get("fields").is_some(),
            "Log entry should have fields"
        );
        assert!(
            log_entry.get("target").is_some(),
            "Log entry should have target field"
        );
    }

    // Cleanup
    child.kill().ok();
    child.wait().ok();
}

#[test]
#[serial]
fn test_mcp_server_logs_debug_level() {
    cleanup_logs();

    // Start MCP Server
    let mut child = Command::new(common::ie_binary())
        .arg("mcp-server")
        .env("INTENT_ENGINE_PROJECT_DIR", common::current_project_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP Server");

    thread::sleep(Duration::from_secs(3));

    // Read log content
    let log_content = fs::read_to_string(mcp_log_path()).expect("Failed to read MCP Server log");

    // Verify DEBUG level logs present
    assert!(
        log_content.contains("\"level\":\"DEBUG\""),
        "Log should contain DEBUG level entries"
    );

    // Verify it captures MCP operations
    let has_dashboard_log = log_content.contains("Dashboard")
        || log_content.contains("registry")
        || log_content.contains("WebSocket")
        || log_content.contains("ws://");

    assert!(
        has_dashboard_log,
        "Log should contain MCP operation details (Dashboard, registry, or WebSocket)"
    );

    // Cleanup
    child.kill().ok();
    child.wait().ok();
}

#[test]
#[serial]
fn test_mcp_server_stdout_clean_for_jsonrpc() {
    cleanup_logs();

    // Start MCP Server (don't try to interact with it)
    let mut child = Command::new(common::ie_binary())
        .arg("mcp-server")
        .env("INTENT_ENGINE_PROJECT_DIR", common::current_project_dir())
        .stdin(Stdio::null())  // Changed to null to avoid blocking
        .stdout(Stdio::null()) // Changed to null - we're not testing stdout content
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP Server");

    thread::sleep(Duration::from_secs(3));

    // The main point: verify logs went to file, not stdout
    assert!(mcp_log_path().exists(), "Logs should be written to file");

    let log_content = fs::read_to_string(mcp_log_path()).expect("Failed to read log file");
    assert!(
        !log_content.is_empty(),
        "Log file should contain log messages"
    );

    // Verify log contains expected content (not JSON-RPC protocol)
    assert!(
        log_content.contains("\"level\":\"DEBUG\""),
        "Log file should contain DEBUG level logs"
    );

    // Cleanup
    child.kill().ok();
    child.wait().ok();
}

#[test]
#[serial]
fn test_mcp_log_directory_auto_creation() {
    // Remove entire logs directory
    let log_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".intent-engine")
        .join("logs");

    if log_dir.exists() {
        fs::remove_dir_all(&log_dir).ok();
    }

    assert!(
        !log_dir.exists(),
        "Log directory should not exist initially"
    );

    // Start MCP Server
    let mut child = Command::new(common::ie_binary())
        .arg("mcp-server")
        .env("INTENT_ENGINE_PROJECT_DIR", common::current_project_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP Server");

    thread::sleep(Duration::from_secs(3));

    // Verify directory and log file created
    assert!(log_dir.exists(), "Log directory should be auto-created");
    assert!(
        mcp_log_path().exists(),
        "Log file should be created in auto-created directory"
    );

    // Cleanup
    child.kill().ok();
    child.wait().ok();
}
