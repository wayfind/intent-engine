// Build script for intent-engine
// This runs before compilation to set up the development environment
//
// Purpose: Automatically install git pre-commit hooks for code formatting
// Timing: Runs during the first `cargo build` after cloning/cleaning
// Safety: Idempotent, skips CI/release builds, uses marker file to avoid repeats

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Tell cargo to rerun if these files change
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=scripts/auto-setup-hooks.sh");
    // Also rerun if the hook itself is deleted
    println!("cargo:rerun-if-changed=.git/hooks/pre-commit");

    // Early returns for situations where we should not install hooks
    if should_skip_hooks_setup() {
        return;
    }

    // Ensure we're in a git repository
    if !is_git_repository() {
        return;
    }

    // Check if hooks are already installed
    if hooks_already_installed() {
        // Hooks exist, create marker file if needed and skip silently
        if let Some(marker) = get_marker_file() {
            if !marker.exists() {
                create_marker_file(&marker);
            }
        }
        return;
    }

    // Hooks don't exist, install them
    if let Some(marker) = get_marker_file() {
        install_git_hooks(&marker);
    }
}

/// Check if we should skip hooks setup (CI, release builds, etc.)
fn should_skip_hooks_setup() -> bool {
    // Skip in CI environments
    let is_ci = env::var("CI").is_ok()
        || env::var("GITHUB_ACTIONS").is_ok()
        || env::var("GITLAB_CI").is_ok()
        || env::var("CIRCLECI").is_ok()
        || env::var("TRAVIS").is_ok();

    if is_ci {
        return true;
    }

    // Skip in release builds
    if let Ok(profile) = env::var("PROFILE") {
        if profile == "release" {
            return true;
        }
    }

    // Skip if explicitly disabled by environment variable
    if env::var("SKIP_GIT_HOOKS_SETUP").is_ok() {
        return true;
    }

    false
}

/// Check if we're in a git repository
fn is_git_repository() -> bool {
    Path::new(".git").exists()
}

/// Get the marker file path, ensuring the target directory exists
fn get_marker_file() -> Option<PathBuf> {
    let target_dir = Path::new("target");

    // Ensure target directory exists
    if !target_dir.exists() {
        if let Err(e) = fs::create_dir_all(target_dir) {
            eprintln!("cargo:warning=Could not create target directory: {}", e);
            return None;
        }
    }

    Some(target_dir.join(".git-hooks-installed"))
}

/// Check if git hooks are already installed
fn hooks_already_installed() -> bool {
    let hook_path = Path::new(".git/hooks/pre-commit");

    if !hook_path.exists() {
        return false;
    }

    // Read and check if it contains our formatting hook
    match fs::read_to_string(hook_path) {
        Ok(content) => content.contains("cargo fmt"),
        Err(_) => false,
    }
}

/// Create the marker file to track installation
fn create_marker_file(marker_path: &Path) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let content = format!(
        "Git hooks installed by build.rs\nTimestamp: {}\n",
        timestamp
    );

    if let Err(e) = fs::write(marker_path, content) {
        // Non-fatal, just log it
        eprintln!("cargo:warning=Could not create marker file: {}", e);
    }
}

/// Install git hooks using the setup script
fn install_git_hooks(marker_path: &Path) {
    let setup_script = Path::new("scripts/auto-setup-hooks.sh");

    if !setup_script.exists() {
        eprintln!(
            "cargo:warning=Setup script not found: {}",
            setup_script.display()
        );
        return;
    }

    println!("cargo:warning=🔧 Setting up git pre-commit hooks for auto-formatting...");

    // Try to find bash
    let bash_cmd = find_bash_command();

    match Command::new(&bash_cmd)
        .arg(setup_script)
        .current_dir(".")
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                create_marker_file(marker_path);
                println!("cargo:warning=✅ Git hooks configured! Commits will be auto-formatted.");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!(
                    "cargo:warning=⚠️  Hook installation failed: {}",
                    stderr.trim()
                );
                eprintln!(
                    "cargo:warning=Run ./scripts/setup-git-hooks.sh manually to install hooks."
                );
            }
        },
        Err(e) => {
            eprintln!("cargo:warning=⚠️  Could not execute setup script: {}", e);
            eprintln!("cargo:warning=Bash may not be available on your system.");
            eprintln!("cargo:warning=Run ./scripts/setup-git-hooks.sh manually to install hooks.");
        },
    }
}

/// Find bash command (handles different platforms)
fn find_bash_command() -> String {
    // Try common bash locations
    let bash_candidates = vec![
        "bash",          // In PATH
        "/bin/bash",     // Standard Unix location
        "/usr/bin/bash", // Alternative Unix location
        "sh",            // Fallback to sh
    ];

    for bash in bash_candidates {
        if Command::new(bash).arg("--version").output().is_ok() {
            return bash.to_string();
        }
    }

    // Default to "bash" and let the system try to find it
    "bash".to_string()
}
