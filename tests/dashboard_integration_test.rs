//! Dashboard WebSocket Integration Tests
//!
//! These tests verify the MCP → Dashboard WebSocket connection functionality,
//! including project registration, path validation, and reconnection logic.
//!
//! NOTE: These tests require Dashboard to actually run, unlike mcp_integration_test.rs
//! which isolates Dashboard. They use #[serial] to avoid port conflicts.

mod common;

// serde traits removed - no longer needed after ProjectRegistry deletion
use serial_test::serial;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

// ProjectRegistry structs removed - no longer needed with WebSocket-based state

// ============================================================================
// Helper Functions
// ============================================================================

/// Initialize a project in the given directory
fn init_project(path: &Path) {
    let output = common::ie_command()
        .current_dir(path)
        .arg("doctor")
        .output()
        .expect("Failed to run doctor command");

    assert!(
        output.status.success(),
        "Failed to initialize project. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Start Dashboard in the background with logging
/// Returns (Child, stdout_path, stderr_path)
fn start_dashboard(project_path: &Path) -> (Child, PathBuf, PathBuf) {
    // Create log files for diagnostics
    let stdout_path = std::env::temp_dir().join(format!("dashboard-{}.stdout", std::process::id()));
    let stderr_path = std::env::temp_dir().join(format!("dashboard-{}.stderr", std::process::id()));

    let stdout_file = File::create(&stdout_path).expect("Failed to create stdout log");
    let stderr_file = File::create(&stderr_path).expect("Failed to create stderr log");

    // Start Dashboard in FOREGROUND mode to avoid setsid issues on macOS
    // In foreground mode:
    // - No child process forking (setsid not used)
    // - Stdout/stderr properly captured by test
    // - Environment variables correctly inherited
    // - Working directory preserved
    let child = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .arg("--foreground") // Run in foreground mode for tests
        .current_dir(project_path)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .expect("Failed to start Dashboard");

    eprintln!("Dashboard started with PID: {:?}", child.id());
    eprintln!("  stdout: {}", stdout_path.display());
    eprintln!("  stderr: {}", stderr_path.display());

    (child, stdout_path, stderr_path)
}

/// Start MCP server
///
/// - `isolated`: if true, sets INTENT_ENGINE_NO_DASHBOARD_AUTOSTART=1
fn start_mcp_server(project_path: &Path, isolated: bool) -> Child {
    let mut cmd = Command::new(common::ie_binary());
    cmd.arg("mcp-server")
        .current_dir(project_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    if isolated {
        cmd.env("INTENT_ENGINE_NO_DASHBOARD_AUTOSTART", "1");
    }

    cmd.spawn().expect("Failed to start MCP server")
}

/// Wait for Dashboard to be ready (polls /api/health)
/// If timeout occurs, prints diagnostic information
fn wait_for_dashboard_ready(stdout_path: &Path, stderr_path: &Path, child: &mut Child) {
    for attempt in 0..30 {
        if check_dashboard_health(11391) {
            // Give it a bit more time to fully initialize
            std::thread::sleep(Duration::from_millis(500));
            return;
        }
        if attempt % 5 == 0 {
            eprintln!("Waiting for Dashboard... (attempt {}/30)", attempt + 1);
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    // Dashboard didn't start - gather diagnostics
    eprintln!("\n========== DASHBOARD STARTUP FAILED ==========");

    // Check if process is still running
    match child.try_wait() {
        Ok(Some(status)) => {
            eprintln!("Dashboard process exited with: {}", status);
        },
        Ok(None) => {
            eprintln!("Dashboard process is still running (PID: {})", child.id());
        },
        Err(e) => {
            eprintln!("Failed to check Dashboard process status: {}", e);
        },
    }

    // Read stdout
    eprintln!("\n--- Dashboard stdout ---");
    if let Ok(mut stdout_content) = std::fs::read_to_string(stdout_path) {
        if stdout_content.is_empty() {
            stdout_content = "(empty)".to_string();
        }
        eprintln!("{}", stdout_content);
    } else {
        eprintln!("(failed to read stdout log)");
    }

    // Read stderr
    eprintln!("\n--- Dashboard stderr ---");
    if let Ok(mut stderr_content) = std::fs::read_to_string(stderr_path) {
        if stderr_content.is_empty() {
            stderr_content = "(empty)".to_string();
        }
        eprintln!("{}", stderr_content);
    } else {
        eprintln!("(failed to read stderr log)");
    }

    // Check port binding
    eprintln!("\n--- Port check ---");
    eprintln!("Attempting to connect to http://127.0.0.1:11391/api/health");
    match reqwest::blocking::get("http://127.0.0.1:11391/api/health") {
        Ok(resp) => eprintln!("Response: {:?}", resp),
        Err(e) => eprintln!("Error: {}", e),
    }

    eprintln!("==============================================\n");

    panic!("Dashboard did not start in time (15 seconds)");
}

/// Check if Dashboard is healthy
fn check_dashboard_health(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{}/api/health", port);
    reqwest::blocking::get(&url)
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Get online projects from Dashboard HTTP API
/// This replaces the old Registry file approach since febf9f9 refactoring
fn get_online_projects(port: u16) -> Vec<serde_json::Value> {
    let url = format!("http://127.0.0.1:{}/api/projects", port);
    match reqwest::blocking::get(&url) {
        Ok(response) => {
            if response.status().is_success() {
                // API returns {"data": [...]} format
                let json: serde_json::Value = response.json().unwrap_or_default();
                json["data"].as_array().cloned().unwrap_or_default()
            } else {
                Vec::new()
            }
        },
        Err(_) => Vec::new(),
    }
}

/// Clean up processes
fn cleanup(mut dashboard: Child, mut mcp: Child) {
    mcp.kill().ok();
    dashboard.kill().ok();

    // Wait for processes to terminate
    let _ = mcp.wait();
    let _ = dashboard.wait();

    std::thread::sleep(Duration::from_millis(500));
}

// ============================================================================
// Test Cases
// ============================================================================

/// Test 1: MCP successfully connects to Dashboard and registers project
///
/// This test uses a real (non-temporary) project directory to verify
/// that normal project registration works correctly.
#[test]
#[serial]
fn test_mcp_connects_to_dashboard_and_registers_project() {
    // Use a non-temporary directory for this test
    // (temporary paths are rejected by our path validation)
    let test_dir = std::env::current_dir().expect("Failed to get current directory");

    // Clean up any existing Dashboard for this project
    let _ = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("stop")
        .current_dir(&test_dir)
        .output();

    std::thread::sleep(Duration::from_secs(1));

    // Initialize project
    init_project(&test_dir);

    // Canonicalize path for comparison (handles symlinks, WSL paths, etc.)
    let canonical_path = test_dir.canonicalize().unwrap_or_else(|_| test_dir.clone());
    let canonical_path_str = canonical_path.to_string_lossy().to_string();
    let port = 11391; // Fixed Dashboard port

    // Start Dashboard
    let (mut dashboard, stdout_path, stderr_path) = start_dashboard(&test_dir);
    wait_for_dashboard_ready(&stdout_path, &stderr_path, &mut dashboard);

    // Get online projects before MCP connection
    let projects_before = get_online_projects(port);
    let was_registered_before = projects_before
        .iter()
        .any(|p| p["path"].as_str() == Some(&canonical_path_str));

    // Start MCP server (NOT isolated - allows WebSocket connection)
    let mcp = start_mcp_server(&test_dir, false);

    // Wait for WebSocket connection to establish
    std::thread::sleep(Duration::from_secs(3));

    // Verify project is registered via Dashboard API
    let projects_after = get_online_projects(port);
    let is_registered_after = projects_after
        .iter()
        .any(|p| p["path"].as_str() == Some(&canonical_path_str));

    assert!(
        is_registered_after,
        "Project should be registered in Dashboard after MCP connection.\n\
         Test dir: {:?}\n\
         Canonical: {:?}\n\
         Before: {}, After: {}\n\
         Online projects: {:?}",
        test_dir,
        canonical_path,
        was_registered_before,
        is_registered_after,
        projects_after
            .iter()
            .map(|p| p["path"].as_str().unwrap_or("unknown"))
            .collect::<Vec<_>>()
    );

    // Clean up
    cleanup(dashboard, mcp);

    eprintln!("✓ Test 1 passed: MCP connects and registers project");
}
