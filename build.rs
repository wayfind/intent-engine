// Build script for intent-engine
// This runs before compilation to set up the development environment

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Tell cargo to rerun this script only if build.rs itself changes
    // This prevents unnecessary reruns on every build
    println!("cargo:rerun-if-changed=build.rs");

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

    // Check if hooks are already installed to avoid repeated messages
    let hook_path = Path::new(".git/hooks/pre-commit");
    if hook_path.exists() {
        // Check if it contains our cargo fmt hook
        if let Ok(content) = std::fs::read_to_string(hook_path) {
            if content.contains("cargo fmt") {
                // Hook already installed, skip
                return;
            }
        }
    }

    // Install git hooks using the existing script
    let setup_script = Path::new("scripts/auto-setup-hooks.sh");
    if setup_script.exists() {
        println!("cargo:warning=🔧 Installing git pre-commit hooks for auto-formatting...");

        let status = Command::new("bash").arg(setup_script).status().ok();

        match status {
            Some(exit_status) if exit_status.success() => {
                println!("cargo:warning=✅ Git hooks installed successfully!");
            },
            Some(_) => {
                println!("cargo:warning=⚠️  Failed to install git hooks. You may need to run ./scripts/setup-git-hooks.sh manually.");
            },
            None => {
                println!("cargo:warning=⚠️  Could not run hooks setup script. Bash may not be available.");
            },
        }
    }
}
