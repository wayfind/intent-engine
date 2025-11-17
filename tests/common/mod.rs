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
pub fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Create a .git marker to prevent falling back to home project
    fs::create_dir(temp_dir.path().join(".git")).unwrap();

    // Initialize the project by adding a dummy task (triggers auto-init)
    // Set HOME to nonexistent directory to prevent fallback to home
    let mut init_cmd = Command::new(ie_binary());
    init_cmd
        .current_dir(temp_dir.path())
        .env("HOME", "/nonexistent") // Prevent fallback to home on Unix
        .env("USERPROFILE", "/nonexistent") // Prevent fallback to home on Windows
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg("Setup task")
        .assert()
        .success();

    temp_dir
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
