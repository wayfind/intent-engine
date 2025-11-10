// Build script for intent-engine
// This runs before compilation to set up the development environment

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Only run setup in development builds, not in CI or release builds
    let is_ci = env::var("CI").is_ok()
        || env::var("GITHUB_ACTIONS").is_ok()
        || env::var("GITLAB_CI").is_ok();

    let is_release = env::var("PROFILE").map(|p| p == "release").unwrap_or(false);

    // Skip hooks setup in CI or release builds
    if is_ci || is_release {
        return;
    }

    // Check if we're in a git repository
    if !Path::new(".git").exists() {
        return;
    }

    // Use a marker file to track if hooks have been installed by build.rs
    // This prevents showing the installation message on every build
    let marker_file = Path::new("target/.git-hooks-installed");
    if marker_file.exists() {
        // Hooks already installed by a previous build
        return;
    }

    // Check if hooks are already installed (maybe manually or by SessionStart)
    let hook_path = Path::new(".git/hooks/pre-commit");
    if hook_path.exists() {
        if let Ok(content) = std::fs::read_to_string(hook_path) {
            if content.contains("cargo fmt") {
                // Hook exists, create marker file and skip
                let _ = std::fs::write(marker_file, "");
                return;
            }
        }
    }

    // Install git hooks using the existing script
    let setup_script = Path::new("scripts/auto-setup-hooks.sh");
    if setup_script.exists() {
        println!("cargo:warning=🔧 Setting up git pre-commit hooks for auto-formatting...");

        let status = Command::new("bash").arg(setup_script).status().ok();

        match status {
            Some(exit_status) if exit_status.success() => {
                // Create marker file to avoid repeating this message
                let _ = std::fs::write(marker_file, "");
                println!("cargo:warning=✅ Git hooks configured! Commits will be auto-formatted.");
            },
            Some(_) => {
                println!("cargo:warning=⚠️  Failed to install git hooks. Run ./scripts/setup-git-hooks.sh manually.");
            },
            None => {
                println!("cargo:warning=⚠️  Could not run hooks setup script. Bash may not be available.");
            },
        }
    }
}
