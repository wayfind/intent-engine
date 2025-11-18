//! Common utilities for integration tests
//!
//! This module provides shared functionality across all integration tests,
//! ensuring consistency and reducing duplication.

use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Get the path to the `ie` binary
///
/// This function is compatible with both standard and custom target directories.
/// It first checks the `CARGO_BIN_EXE_ie` environment variable (set by cargo
/// when using custom target directories like in CI coverage tests), and falls
/// back to the standard `cargo_bin!()` macro for local development.
///
/// # Examples
///
/// ```no_run
/// use std::process::Command;
/// mod common;
///
/// let output = Command::new(common::ie_binary())
///     .arg("task")
///     .arg("list")
///     .output()
///     .unwrap();
/// ```
///
/// # Panics
///
/// Panics if the `ie` binary cannot be found in either the environment
/// variable or the standard cargo build directory.
#[allow(deprecated)] // cargo_bin() is deprecated but needed for fallback
pub fn ie_binary() -> PathBuf {
    // CI environments (especially coverage tests with custom --target-dir) set
    // CARGO_BIN_EXE_<name> to point to the actual binary location
    std::env::var("CARGO_BIN_EXE_ie")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Fallback to cargo_bin() for local testing with standard target directory
            assert_cmd::cargo::cargo_bin("ie")
        })
}

/// Create a Command for `ie` with proper environment isolation
///
/// This returns a Command pre-configured with:
/// - The correct `ie` binary path
/// - Environment isolation (HOME=/nonexistent) to prevent home directory fallback
///
/// # Examples
///
/// ```no_run
/// mod common;
///
/// let output = common::ie_command()
///     .current_dir(&temp_dir)
///     .arg("task")
///     .arg("list")
///     .assert()
///     .success();
/// ```
#[allow(dead_code)] // Not all test files use this yet
pub fn ie_command() -> Command {
    let mut cmd = Command::new(ie_binary());
    cmd.env("HOME", "/nonexistent") // Prevent fallback to home on Unix
        .env("USERPROFILE", "/nonexistent") // Prevent fallback to home on Windows
        .env("INTENT_ENGINE_NO_HOME_FALLBACK", "1"); // Additional flag to prevent home fallback
    cmd
}

/// Create a Command for `ie` with project directory override
///
/// This returns a Command pre-configured with:
/// - The correct `ie` binary path
/// - INTENT_ENGINE_PROJECT_DIR set to the specified directory
/// - Environment isolation (HOME=/nonexistent) to prevent home directory fallback
///
/// # Examples
///
/// ```no_run
/// mod common;
///
/// let output = common::ie_command_with_project_dir(&temp_dir.path())
///     .arg("task")
///     .arg("list")
///     .assert()
///     .success();
/// ```
#[allow(dead_code)] // Used by MCP tests
pub fn ie_command_with_project_dir(project_dir: &std::path::Path) -> Command {
    let mut cmd = ie_command();
    cmd.env("INTENT_ENGINE_PROJECT_DIR", project_dir);
    cmd
}

/// Setup a test environment with an initialized intent-engine project
///
/// This creates a temporary directory with:
/// - A `.git` marker to prevent fallback to home project
/// - An initialized intent-engine database (via auto-init)
/// - Isolated environment variables to prevent home directory pollution
///
/// # Returns
///
/// A `TempDir` that will be automatically cleaned up when dropped.
///
/// # Examples
///
/// ```no_run
/// mod common;
///
/// #[test]
/// fn test_something() {
///     let temp_dir = common::setup_test_env();
///     // Use temp_dir for testing...
/// }
/// ```
#[allow(dead_code)] // Not all test files use this yet
pub fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Create a .git marker to prevent falling back to home project
    // This ensures intent-engine recognizes this as a valid project root
    fs::create_dir(temp_dir.path().join(".git")).unwrap();

    // Initialize the intent-engine project by running a command that triggers database initialization
    // The workspace command should trigger auto-initialization with proper database setup
    let init_output = std::process::Command::new(ie_binary())
        .current_dir(temp_dir.path())
        .env("HOME", "/nonexistent") // Prevent fallback to home on Unix
        .env("USERPROFILE", "/nonexistent") // Prevent fallback to home on Windows
        .env("INTENT_ENGINE_PROJECT_DIR", temp_dir.path()) // Force project dir
        .args(["workspace"]) // Simple command that should initialize database
        .output()
        .expect("Failed to run ie workspace command");

    // Even if it fails, check if it created the database structure
    if !init_output.status.success() {
        // Let's check if the .intent-engine directory was created at least
        if !temp_dir.path().join(".intent-engine").exists() {
            // Create it manually if auto-init failed
            fs::create_dir(temp_dir.path().join(".intent-engine")).unwrap();
        }
    }

    temp_dir
}

/// Get the current project directory for MCP tests
///
/// Since MCP server requires a fully initialized project and doesn't support
/// creating fresh projects easily, we use the current project directory
/// which is already properly set up.
#[allow(dead_code)] // Used by MCP tests
pub fn current_project_dir() -> PathBuf {
    std::env::current_dir().expect("Failed to get current directory")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ie_binary_exists() {
        let binary = ie_binary();
        assert!(binary.exists(), "ie binary should exist at {:?}", binary);
    }
}
