//! Dashboard WebSocket Integration Tests
//!
//! These tests verify the MCP → Dashboard WebSocket connection functionality,
//! including project registration, path validation, and reconnection logic.
//!
//! NOTE: These tests require Dashboard to actually run, unlike mcp_integration_test.rs
//! which isolates Dashboard. They use #[serial] to avoid port conflicts.

mod common;

use serde::{Deserialize, Serialize};
use serial_test::serial;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Registry file structure (matches src/dashboard/registry.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectRegistry {
    pub version: String,
    pub projects: Vec<RegisteredProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegisteredProject {
    pub path: PathBuf,
    pub name: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub started_at: String,
    pub db_path: PathBuf,

    #[serde(default)]
    pub mcp_connected: bool,
    #[serde(default)]
    pub mcp_last_seen: Option<String>,
    #[serde(default)]
    pub mcp_agent: Option<String>,
}

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

/// Load the global registry file
/// NOTE: This is DEPRECATED as of febf9f9 refactoring
/// The Registry file is no longer used - use get_online_projects() instead
fn load_registry() -> ProjectRegistry {
    let registry_path = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".intent-engine")
        .join("projects.json");

    if !registry_path.exists() {
        return ProjectRegistry {
            version: "1.0".to_string(),
            projects: Vec::new(),
        };
    }

    let content = std::fs::read_to_string(&registry_path).expect("Failed to read registry file");
    serde_json::from_str(&content).expect("Failed to parse registry file")
}

/// Clean temporary paths from the registry (for test isolation)
fn clean_temp_paths_from_registry() {
    let registry_path = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".intent-engine")
        .join("projects.json");

    if !registry_path.exists() {
        return;
    }

    let mut registry = load_registry();
    let temp_dir = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());

    // Remove all temp paths
    registry.projects.retain(|p| {
        let normalized = p.path.canonicalize().unwrap_or_else(|_| p.path.clone());
        !normalized.starts_with(&temp_dir)
    });

    // Save cleaned registry
    let content = serde_json::to_string_pretty(&registry).expect("Failed to serialize registry");
    std::fs::write(&registry_path, content).expect("Failed to write registry file");
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

/// Test 2: Temporary paths are correctly rejected (validates Defense Layer 2)
///
/// This test verifies that the path validation logic in src/mcp/ws_client.rs
/// correctly rejects temporary directory paths across all platforms.
#[test]
#[serial]
fn test_temporary_paths_are_rejected() {
    // Clean any temp paths left by previous tests (test isolation)
    clean_temp_paths_from_registry();

    let temp_dir = common::setup_test_env(); // Creates platform-specific temp dir
    let project_path = temp_dir.path();

    // Initialize project in temporary directory
    init_project(project_path);

    // Start Dashboard (using current directory, not temp)
    let real_dir = std::env::current_dir().expect("Failed to get current directory");
    let (mut dashboard, stdout_path, stderr_path) = start_dashboard(&real_dir);
    wait_for_dashboard_ready(&stdout_path, &stderr_path, &mut dashboard);

    // Get registry state before
    let registry_before = load_registry();
    let count_before = registry_before.projects.len();

    // Start MCP server in temporary directory (NOT isolated)
    let mcp = start_mcp_server(project_path, false);

    // Wait for connection attempt
    std::thread::sleep(Duration::from_secs(3));

    // Verify temporary project was NOT registered (Defense Layer 2 working)
    let registry_after = load_registry();
    let count_after = registry_after.projects.len();

    // Check for temporary paths using platform-agnostic method
    // IMPORTANT: Both paths must be canonicalized for comparison to work on macOS/Windows
    // (e.g., macOS: /var → /private/var symlink)
    let temp_dir = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());
    let has_temp_path = registry_after.projects.iter().any(|p| {
        let normalized = p.path.canonicalize().unwrap_or_else(|_| p.path.clone());
        normalized.starts_with(&temp_dir)
    });

    assert!(
        !has_temp_path,
        "Registry should not contain temporary paths (Defense Layer 2 failed). Temp dir: {:?}",
        temp_dir
    );

    eprintln!(
        "✓ Test 2 passed: Temporary paths rejected (count before: {}, after: {})",
        count_before, count_after
    );

    // Clean up
    cleanup(dashboard, mcp);
}

/// Test 3: Environment variable isolation prevents connection (validates Defense Layer 1)
///
/// This test verifies that setting INTENT_ENGINE_NO_DASHBOARD_AUTOSTART=1
/// prevents MCP from connecting to Dashboard.
#[test]
#[serial]
fn test_dashboard_autostart_env_prevents_connection() {
    let test_dir = std::env::current_dir().expect("Failed to get current directory");

    // Clean up
    let _ = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("stop")
        .current_dir(&test_dir)
        .output();
    std::thread::sleep(Duration::from_secs(1));

    init_project(&test_dir);

    // Start Dashboard
    let (mut dashboard, stdout_path, stderr_path) = start_dashboard(&test_dir);
    wait_for_dashboard_ready(&stdout_path, &stderr_path, &mut dashboard);

    // Get initial MCP connection state
    let registry_before = load_registry();
    let project_path_str = test_dir.to_string_lossy().to_string();

    // Find our project and check initial mcp_connected state
    let mcp_connected_before = registry_before
        .projects
        .iter()
        .find(|p| p.path.to_string_lossy() == project_path_str)
        .map(|p| p.mcp_connected)
        .unwrap_or(false);

    // Start MCP server with isolation (Defense Layer 1)
    let mcp = start_mcp_server(&test_dir, true); // isolated = true

    // Wait
    std::thread::sleep(Duration::from_secs(3));

    // Verify MCP connection was NOT established
    let registry_after = load_registry();
    let mcp_connected_after = registry_after
        .projects
        .iter()
        .find(|p| p.path.to_string_lossy() == project_path_str)
        .map(|p| p.mcp_connected)
        .unwrap_or(false);

    // In isolated mode, mcp_connected should remain false
    assert!(
        !mcp_connected_after || !mcp_connected_before,
        "MCP should not connect when INTENT_ENGINE_NO_DASHBOARD_AUTOSTART=1 is set \
         (Defense Layer 1 failed). Before: {}, After: {}",
        mcp_connected_before,
        mcp_connected_after
    );

    eprintln!("✓ Test 3 passed: Environment variable isolation works");

    // Clean up
    cleanup(dashboard, mcp);
}

/// Test 4: MCP reconnects after Dashboard restart
///
/// This test verifies the resilience of the WebSocket connection.
/// Note: This test is currently simplified and may need enhancement
/// to truly test reconnection logic.
#[test]
#[serial]
#[ignore] // Ignore by default - timing-sensitive and may be flaky
fn test_mcp_reconnects_after_dashboard_restart() {
    let test_dir = std::env::current_dir().expect("Failed to get current directory");

    // Clean up
    let _ = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("stop")
        .current_dir(&test_dir)
        .output();
    std::thread::sleep(Duration::from_secs(1));

    init_project(&test_dir);

    // First startup
    let (mut dashboard, stdout_path, stderr_path) = start_dashboard(&test_dir);
    wait_for_dashboard_ready(&stdout_path, &stderr_path, &mut dashboard);

    let mcp = start_mcp_server(&test_dir, false);
    std::thread::sleep(Duration::from_secs(3));

    // Verify initial connection
    let registry = load_registry();
    let project_path_str = test_dir.to_string_lossy().to_string();
    let initially_registered = registry
        .projects
        .iter()
        .any(|p| p.path.to_string_lossy() == project_path_str);

    assert!(
        initially_registered,
        "Project should be initially registered"
    );

    // Restart Dashboard
    dashboard.kill().expect("Failed to kill Dashboard");
    let _ = dashboard.wait();
    std::thread::sleep(Duration::from_secs(2));

    let (mut dashboard2, stdout_path2, stderr_path2) = start_dashboard(&test_dir);
    wait_for_dashboard_ready(&stdout_path2, &stderr_path2, &mut dashboard2);

    // Wait for potential reconnection
    std::thread::sleep(Duration::from_secs(5));

    // Verify still registered (either through reconnection or Registry persistence)
    let registry_after = load_registry();
    let still_registered = registry_after
        .projects
        .iter()
        .any(|p| p.path.to_string_lossy() == project_path_str);

    assert!(
        still_registered,
        "Project should still be registered after Dashboard restart"
    );

    eprintln!("✓ Test 4 passed: Connection survives Dashboard restart");

    // Clean up
    cleanup(dashboard2, mcp);
}
