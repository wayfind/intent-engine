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
    /// Generates the hooks configuration for both SessionStart and PostToolUse events.
    /// This configuration is shared between user-level and project-level setups.
    ///
    /// # Arguments
    /// * `hook_path` - Absolute path to the SessionStart hook script
    /// * `format_hook_path` - Absolute path to the PostToolUse formatting hook script
    fn create_claude_settings(hook_path: &Path, format_hook_path: &Path) -> serde_json::Value {
        const MCP_TOOL_MATCHERS: &[&str] = &[
            "task_context",
            "task_get",
            "current_task_get",
            "task_list",
            "task_pick_next",
            "unified_search",
            "event_list",
        ];

        let post_tool_use_hooks: Vec<serde_json::Value> = MCP_TOOL_MATCHERS
            .iter()
            .map(|matcher| {
                json!({
                    "matcher": format!("mcp__intent-engine__{}", matcher),
                    "hooks": [{
                        "type": "command",
                        "command": format_hook_path.to_string_lossy()
                    }]
                })
            })
            .collect();

        json!({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": hook_path.to_string_lossy()
                    }]
                }],
                "PostToolUse": post_tool_use_hooks
            }
        })
    }

    /// Setup for user-level installation
    fn setup_user_level(&self, opts: &SetupOptions) -> Result<SetupResult> {
        let mut files_modified = Vec::new();
        let mut backups = Vec::new();

        println!("📦 Setting up user-level Claude Code integration...\n");

        // 1. Setup hooks directory and script
        let claude_dir = Self::get_user_claude_dir()?;
        let hooks_dir = claude_dir.join("hooks");
        let hook_script = hooks_dir.join("session-start.sh");

        fs::create_dir_all(&hooks_dir).map_err(IntentError::IoError)?;
        println!("✓ Created {}", hooks_dir.display());

        // Backup existing hook script
        if hook_script.exists() && !opts.force {
            return Err(IntentError::InvalidInput(format!(
                "Hook script already exists: {}. Use --force to overwrite",
                hook_script.display()
            )));
        }

        if hook_script.exists() {
            if let Some(backup) = create_backup(&hook_script)? {
                backups.push((hook_script.clone(), backup.clone()));
                println!("✓ Backed up hook script to {}", backup.display());
            }
        }

        // Install session-start hook script
        let hook_content = include_str!("../../templates/session-start.sh");
        fs::write(&hook_script, hook_content).map_err(IntentError::IoError)?;
        set_executable(&hook_script)?;
        files_modified.push(hook_script.clone());
        println!("✓ Installed {}", hook_script.display());

        // Install format-ie-output hook script
        let format_hook_script = hooks_dir.join("format-ie-output.sh");
        let format_hook_content = include_str!("../../templates/format-ie-output.sh");

        if format_hook_script.exists() && !opts.force {
            return Err(IntentError::InvalidInput(format!(
                "Format hook already exists: {}. Use --force to overwrite",
                format_hook_script.display()
            )));
        }

        if format_hook_script.exists() {
            if let Some(backup) = create_backup(&format_hook_script)? {
                backups.push((format_hook_script.clone(), backup.clone()));
                println!("✓ Backed up format hook to {}", backup.display());
            }
        }

        fs::write(&format_hook_script, format_hook_content).map_err(IntentError::IoError)?;
        set_executable(&format_hook_script)?;
        files_modified.push(format_hook_script.clone());
        println!("✓ Installed {}", format_hook_script.display());

        // 2. Setup settings.json with absolute paths
        let settings_file = claude_dir.join("settings.json");
        let hook_abs_path = resolve_absolute_path(&hook_script)?;
        let format_hook_abs_path = resolve_absolute_path(&format_hook_script)?;

        if settings_file.exists() && !opts.force {
            return Err(IntentError::InvalidInput(format!(
                "Settings file already exists: {}. Use --force to overwrite",
                settings_file.display()
            )));
        }

        if settings_file.exists() {
            if let Some(backup) = create_backup(&settings_file)? {
                backups.push((settings_file.clone(), backup.clone()));
                println!("✓ Backed up settings to {}", backup.display());
            }
        }

        let settings = Self::create_claude_settings(&hook_abs_path, &format_hook_abs_path);

        write_json_config(&settings_file, &settings)?;
        files_modified.push(settings_file.clone());
        println!("✓ Created {}", settings_file.display());

        // 3. Setup MCP configuration
        let mcp_result = self.setup_mcp_config(opts, &mut files_modified, &mut backups)?;

        Ok(SetupResult {
            success: true,
            message: "User-level Claude Code setup complete!".to_string(),
            files_modified,
            connectivity_test: Some(mcp_result),
        })
    }

    /// Setup MCP server configuration
    fn setup_mcp_config(
        &self,
        opts: &SetupOptions,
        files_modified: &mut Vec<PathBuf>,
        backups: &mut Vec<(PathBuf, PathBuf)>,
    ) -> Result<ConnectivityResult> {
        let config_path = if let Some(ref path) = opts.config_path {
            path.clone()
        } else {
            let home = get_home_dir()?;
            home.join(".claude.json")
        };

        // Find binary
        let binary_path = find_ie_binary()?;
        println!("✓ Found binary: {}", binary_path.display());

        // Backup existing config
        if config_path.exists() {
            if let Some(backup) = create_backup(&config_path)? {
                backups.push((config_path.clone(), backup.clone()));
                println!("✓ Backed up MCP config to {}", backup.display());
            }
        }

        // Read or create config
        let mut config = read_json_config(&config_path)?;

        // Check if already configured
        if let Some(mcp_servers) = config.get("mcpServers") {
            if mcp_servers.get("intent-engine").is_some() && !opts.force {
                return Ok(ConnectivityResult {
                    passed: false,
                    details: "intent-engine already configured in MCP config".to_string(),
                });
            }
        }

        // Add intent-engine configuration
        if config.get("mcpServers").is_none() {
            config["mcpServers"] = json!({});
        }

        config["mcpServers"]["intent-engine"] = json!({
            "command": binary_path.to_string_lossy(),
            "args": ["mcp-server"],
            "description": "Strategic intent and task workflow management"
        });

        write_json_config(&config_path, &config)?;
        files_modified.push(config_path.clone());
        println!("✓ Updated {}", config_path.display());

        Ok(ConnectivityResult {
            passed: true,
            details: format!("MCP configured at {}", config_path.display()),
        })
    }

    /// Setup for project-level installation
    fn setup_project_level(&self, opts: &SetupOptions) -> Result<SetupResult> {
        println!("📦 Setting up project-level Claude Code integration...\n");
        println!("⚠️  Note: Project-level setup is for advanced users.");
        println!("    MCP config will still be in ~/.claude.json (user-level)\n");

        let mut files_modified = Vec::new();
        let claude_dir = Self::get_project_claude_dir()?;
        let hooks_dir = claude_dir.join("hooks");
        let hook_script = hooks_dir.join("session-start.sh");

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

        // Install format-ie-output hook script
        let format_hook_script = hooks_dir.join("format-ie-output.sh");
        let format_hook_content = include_str!("../../templates/format-ie-output.sh");

        if format_hook_script.exists() && !opts.force {
            return Err(IntentError::InvalidInput(format!(
                "Format hook already exists: {}. Use --force to overwrite",
                format_hook_script.display()
            )));
        }

        fs::write(&format_hook_script, format_hook_content).map_err(IntentError::IoError)?;
        set_executable(&format_hook_script)?;
        files_modified.push(format_hook_script.clone());
        println!("✓ Installed {}", format_hook_script.display());

        // Create settings.json with absolute paths
        let settings_file = claude_dir.join("settings.json");
        let hook_abs_path = resolve_absolute_path(&hook_script)?;
        let format_hook_abs_path = resolve_absolute_path(&format_hook_script)?;

        // Check if settings file already exists
        if settings_file.exists() && !opts.force {
            return Err(IntentError::InvalidInput(format!(
                "Settings file already exists: {}. Use --force to overwrite",
                settings_file.display()
            )));
        }

        let settings = Self::create_claude_settings(&hook_abs_path, &format_hook_abs_path);

        write_json_config(&settings_file, &settings)?;
        files_modified.push(settings_file);
        println!("✓ Created settings.json");

        // MCP config still goes to user-level
        let mut backups = Vec::new();
        let mcp_result = self.setup_mcp_config(opts, &mut files_modified, &mut backups)?;

        Ok(SetupResult {
            success: true,
            message: "Project-level setup complete!".to_string(),
            files_modified,
            connectivity_test: Some(mcp_result),
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
