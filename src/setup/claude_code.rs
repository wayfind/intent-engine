//! Claude Code setup module

use super::common::*;
use super::{ConnectivityResult, SetupModule, SetupOptions, SetupResult, SetupScope};
use crate::error::{IntentError, Result};
use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ClaudeCodeSetup;

impl ClaudeCodeSetup {
    /// Get the user-level .claude directory
    fn get_user_claude_dir() -> Result<PathBuf> {
        let home = get_home_dir()?;
        Ok(home.join(".claude"))
    }

    /// Get the project-level .claude directory
    fn get_project_claude_dir() -> Result<PathBuf> {
        let current_dir = env::current_dir().map_err(IntentError::IoError)?;
        Ok(current_dir.join(".claude"))
    }

    /// Create Claude Code settings JSON configuration
    ///
    /// Generates the hooks configuration for SessionStart event.
    /// This configuration is shared between user-level and project-level setups.
    ///
    /// # Arguments
    /// * `hook_path` - Absolute path to the SessionStart hook script
    fn create_claude_settings(hook_path: &Path) -> serde_json::Value {
        json!({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": hook_path.to_string_lossy()
                    }]
                }]
            }
        })
    }

    /// Common setup logic for hooks and settings
    ///
    /// Sets up the session-start hook script and settings.json in the given Claude directory.
    /// This function is shared between user-level and project-level setup.
    ///
    /// # Arguments
    /// * `claude_dir` - The .claude directory (user-level or project-level)
    /// * `opts` - Setup options (includes force flag)
    /// * `files_modified` - Mutable vector to track modified files
    fn setup_hooks_and_settings(
        claude_dir: &Path,
        opts: &SetupOptions,
        files_modified: &mut Vec<PathBuf>,
    ) -> Result<()> {
        let hooks_dir = claude_dir.join("hooks");
        let hook_script = hooks_dir.join("session-start.sh");

        // Create hooks directory
        fs::create_dir_all(&hooks_dir).map_err(IntentError::IoError)?;
        println!("✓ Created {}", hooks_dir.display());

        // Check if hook script already exists
        if hook_script.exists() && !opts.force {
            return Err(IntentError::InvalidInput(format!(
                "Hook script already exists: {}. Use --force to overwrite",
                hook_script.display()
            )));
        }

        // Install session-start hook script
        let hook_content = include_str!("../../templates/session-start.sh");
        fs::write(&hook_script, hook_content).map_err(IntentError::IoError)?;
        set_executable(&hook_script)?;
        files_modified.push(hook_script.clone());
        println!("✓ Installed {}", hook_script.display());

        // Setup settings.json with absolute paths
        let settings_file = claude_dir.join("settings.json");
        let hook_abs_path = resolve_absolute_path(&hook_script)?;

        // Check if settings file already exists
        if settings_file.exists() && !opts.force {
            return Err(IntentError::InvalidInput(format!(
                "Settings file already exists: {}. Use --force to overwrite",
                settings_file.display()
            )));
        }

        let settings = Self::create_claude_settings(&hook_abs_path);

        write_json_config(&settings_file, &settings)?;
        files_modified.push(settings_file.clone());
        println!("✓ Created {}", settings_file.display());

        Ok(())
    }

    /// Setup for user-level installation
    fn setup_user_level(&self, opts: &SetupOptions) -> Result<SetupResult> {
        let mut files_modified = Vec::new();

        println!("📦 Setting up user-level Claude Code integration...\n");

        // Setup hooks and settings in user-level .claude directory
        let claude_dir = Self::get_user_claude_dir()?;
        Self::setup_hooks_and_settings(&claude_dir, opts, &mut files_modified)?;

        Ok(SetupResult {
            success: true,
            message: "User-level Claude Code setup complete!".to_string(),
            files_modified,
            connectivity_test: None,
        })
    }

    /// Setup for project-level installation
    fn setup_project_level(&self, opts: &SetupOptions) -> Result<SetupResult> {
        println!("📦 Setting up project-level Claude Code integration...\n");
        println!("⚠️  Note: Project-level setup is for advanced users.\n");

        let mut files_modified = Vec::new();

        // Setup hooks and settings in project-level .claude directory
        let claude_dir = Self::get_project_claude_dir()?;
        Self::setup_hooks_and_settings(&claude_dir, opts, &mut files_modified)?;

        Ok(SetupResult {
            success: true,
            message: "Project-level setup complete!".to_string(),
            files_modified,
            connectivity_test: None,
        })
    }
}

impl SetupModule for ClaudeCodeSetup {
    fn name(&self) -> &str {
        "claude-code"
    }

    fn setup(&self, opts: &SetupOptions) -> Result<SetupResult> {
        match opts.scope {
            SetupScope::User => self.setup_user_level(opts),
            SetupScope::Project => self.setup_project_level(opts),
            SetupScope::Both => {
                // First user-level, then project-level
                let user_result = self.setup_user_level(opts)?;
                let project_result = self.setup_project_level(opts)?;

                // Combine results
                let mut files = user_result.files_modified;
                files.extend(project_result.files_modified);

                Ok(SetupResult {
                    success: true,
                    message: "User and project setup complete!".to_string(),
                    files_modified: files,
                    connectivity_test: user_result.connectivity_test,
                })
            },
        }
    }

    fn test_connectivity(&self) -> Result<ConnectivityResult> {
        // Test 1: Can we execute session-restore?
        println!("Testing session-restore command...");
        let output = std::process::Command::new("ie")
            .args(["session-restore", "--workspace", "."])
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    Ok(ConnectivityResult {
                        passed: true,
                        details: "session-restore command executed successfully".to_string(),
                    })
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    Ok(ConnectivityResult {
                        passed: false,
                        details: format!("session-restore failed: {}", stderr),
                    })
                }
            },
            Err(e) => Ok(ConnectivityResult {
                passed: false,
                details: format!("Failed to execute session-restore: {}", e),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // ========== create_claude_settings tests ==========

    #[test]
    fn test_create_claude_settings_structure() {
        let hook_path = PathBuf::from("/tmp/session-start.sh");

        let settings = ClaudeCodeSetup::create_claude_settings(&hook_path);

        // Verify hooks key exists
        assert!(settings.get("hooks").is_some());

        // Verify SessionStart hook
        let hooks = &settings["hooks"];
        assert!(hooks.get("SessionStart").is_some());
        let session_start = &hooks["SessionStart"];
        assert!(session_start.is_array());
        assert_eq!(session_start.as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_create_claude_settings_session_start_hook() {
        let hook_path = PathBuf::from("/home/user/.claude/hooks/session-start.sh");

        let settings = ClaudeCodeSetup::create_claude_settings(&hook_path);

        let session_start = &settings["hooks"]["SessionStart"][0];
        assert!(session_start.get("hooks").is_some());

        let hooks_array = session_start["hooks"].as_array().unwrap();
        assert_eq!(hooks_array.len(), 1);

        let hook = &hooks_array[0];
        assert_eq!(hook["type"], "command");
        assert_eq!(hook["command"], "/home/user/.claude/hooks/session-start.sh");
    }

    // ========== Directory path tests ==========

    #[test]
    fn test_get_user_claude_dir() {
        // This test depends on HOME environment variable
        let result = ClaudeCodeSetup::get_user_claude_dir();
        assert!(result.is_ok());

        let dir = result.unwrap();
        assert!(dir.ends_with(".claude"));
    }

    #[test]
    fn test_get_project_claude_dir() {
        let result = ClaudeCodeSetup::get_project_claude_dir();
        assert!(result.is_ok());

        let dir = result.unwrap();
        assert!(dir.ends_with(".claude"));
    }

    #[test]
    fn test_claude_code_setup_name() {
        let setup = ClaudeCodeSetup;
        assert_eq!(setup.name(), "claude-code");
    }

    // ========== JSON structure validation tests ==========

    #[test]
    fn test_create_claude_settings_paths_preserved() {
        // Test with special characters in path
        let hook_path = PathBuf::from("/home/user name/with spaces/.claude/hooks/session-start.sh");

        let settings = ClaudeCodeSetup::create_claude_settings(&hook_path);

        // Paths should be preserved as strings
        let session_start_cmd = settings["hooks"]["SessionStart"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert!(session_start_cmd.contains("with spaces"));
    }

    // ========== File system tests with tempdir ==========

    #[test]
    fn test_setup_hooks_and_settings_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join(".claude");

        let opts = SetupOptions {
            force: false,
            scope: SetupScope::User,
            config_path: None,
        };
        let mut files_modified = Vec::new();

        let result =
            ClaudeCodeSetup::setup_hooks_and_settings(&claude_dir, &opts, &mut files_modified);

        assert!(result.is_ok());

        // Verify directories created
        assert!(claude_dir.join("hooks").exists());

        // Verify hook script created and executable
        let hook_script = claude_dir.join("hooks/session-start.sh");
        assert!(hook_script.exists());

        // Verify settings.json created
        let settings_file = claude_dir.join("settings.json");
        assert!(settings_file.exists());

        // Verify files tracked
        assert_eq!(files_modified.len(), 2);
    }

    #[test]
    fn test_setup_hooks_and_settings_force_overwrites() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join(".claude");

        // First setup without force
        let opts = SetupOptions {
            force: false,
            scope: SetupScope::User,
            config_path: None,
        };
        let mut files_modified = Vec::new();
        ClaudeCodeSetup::setup_hooks_and_settings(&claude_dir, &opts, &mut files_modified).unwrap();

        // Second setup without force should fail
        let result =
            ClaudeCodeSetup::setup_hooks_and_settings(&claude_dir, &opts, &mut files_modified);
        assert!(result.is_err());

        // Third setup with force should succeed
        let opts_force = SetupOptions {
            force: true,
            scope: SetupScope::User,
            config_path: None,
        };
        let mut files_modified2 = Vec::new();
        let result = ClaudeCodeSetup::setup_hooks_and_settings(
            &claude_dir,
            &opts_force,
            &mut files_modified2,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_setup_hooks_and_settings_hook_content() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join(".claude");

        let opts = SetupOptions {
            force: false,
            scope: SetupScope::User,
            config_path: None,
        };
        let mut files_modified = Vec::new();

        ClaudeCodeSetup::setup_hooks_and_settings(&claude_dir, &opts, &mut files_modified).unwrap();

        // Read and verify hook script content
        let hook_script = claude_dir.join("hooks/session-start.sh");
        let content = std::fs::read_to_string(&hook_script).unwrap();

        // Should contain shebang and ie command
        assert!(content.contains("#!/"));
        assert!(content.contains("ie ") || content.contains("session-restore"));
    }

    #[test]
    fn test_setup_hooks_and_settings_json_valid() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join(".claude");

        let opts = SetupOptions {
            force: false,
            scope: SetupScope::User,
            config_path: None,
        };
        let mut files_modified = Vec::new();

        ClaudeCodeSetup::setup_hooks_and_settings(&claude_dir, &opts, &mut files_modified).unwrap();

        // Read and parse settings.json
        let settings_file = claude_dir.join("settings.json");
        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify structure
        assert!(settings.get("hooks").is_some());
        assert!(settings["hooks"].get("SessionStart").is_some());
    }
}
