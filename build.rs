// Build script for intent-engine
// This runs before compilation to set up the development environment
//
// Purpose:
// 1. Automatically install git pre-commit hooks for code formatting
// 2. Automatically build frontend if static/ is missing
//
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
    println!("cargo:rerun-if-changed=front-end/src");
    println!("cargo:rerun-if-changed=front-end/package.json");
    // Also rerun if the hook itself is deleted
    println!("cargo:rerun-if-changed=.git/hooks/pre-commit");

    // Build frontend if needed
    build_frontend_if_needed();

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

/// Build frontend if the static directory is missing or empty
fn build_frontend_if_needed() {
    let static_dir = Path::new("static");
    let index_html = static_dir.join("index.html");
    let front_end_dir = Path::new("front-end");

    // Skip if frontend source doesn't exist
    if !front_end_dir.exists() {
        return;
    }

    // Check if static/index.html exists (indicates frontend is built)
    if index_html.exists() {
        return;
    }

    // Skip in CI - frontend is built separately
    if is_ci_environment() {
        return;
    }

    println!("cargo:warning=📦 Building frontend (static/ not found)...");

    // Check if npm is available
    let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };

    // Run npm ci
    let npm_ci = Command::new(npm_cmd)
        .args(["ci"])
        .current_dir(front_end_dir)
        .output();

    match npm_ci {
        Ok(output) if output.status.success() => {
            // Run npm run build
            let npm_build = Command::new(npm_cmd)
                .args(["run", "build"])
                .current_dir(front_end_dir)
                .output();

            match npm_build {
                Ok(output) if output.status.success() => {
                    println!("cargo:warning=✅ Frontend built successfully!");
                },
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("cargo:warning=⚠️  npm run build failed: {}", stderr.trim());
                    eprintln!("cargo:warning=Run 'cd front-end && npm run build' manually.");
                },
                Err(e) => {
                    eprintln!("cargo:warning=⚠️  Could not run npm build: {}", e);
                },
            }
        },
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("cargo:warning=⚠️  npm ci failed: {}", stderr.trim());
            eprintln!("cargo:warning=Run 'cd front-end && npm ci && npm run build' manually.");
        },
        Err(e) => {
            eprintln!("cargo:warning=⚠️  npm not found: {}", e);
            eprintln!(
                "cargo:warning=Install Node.js and run 'cd front-end && npm ci && npm run build'."
            );
        },
    }
}

/// Check if running in CI environment
fn is_ci_environment() -> bool {
    env::var("CI").is_ok()
        || env::var("GITHUB_ACTIONS").is_ok()
        || env::var("GITLAB_CI").is_ok()
        || env::var("CIRCLECI").is_ok()
        || env::var("TRAVIS").is_ok()
}

/// Check if we should skip hooks setup (CI, release builds, etc.)
fn should_skip_hooks_setup() -> bool {
    // Skip in CI environments
    if is_ci_environment() {
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
