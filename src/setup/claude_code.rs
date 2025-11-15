//! Claude Code setup module

use super::common::*;
use super::{
    ConnectivityResult, DiagnosisCheck, DiagnosisReport, SetupModule, SetupOptions, SetupResult,
    SetupScope,
};
use crate::error::{IntentError, Result};
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;

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

    /// Setup for user-level installation
    fn setup_user_level(&self, opts: &SetupOptions) -> Result<SetupResult> {
        let mut files_modified = Vec::new();
        let mut backups = Vec::new();

        println!("📦 Setting up user-level Claude Code integration...\n");

        // 1. Setup hooks directory and script
        let claude_dir = Self::get_user_claude_dir()?;
        let hooks_dir = claude_dir.join("hooks");
        let hook_script = hooks_dir.join("session-start.sh");

        if !opts.dry_run {
            fs::create_dir_all(&hooks_dir).map_err(IntentError::IoError)?;
            println!("✓ Created {}", hooks_dir.display());
        } else {
            println!("Would create: {}", hooks_dir.display());
        }

        // Backup existing hook script
        if hook_script.exists() && !opts.force {
            return Err(IntentError::InvalidInput(format!(
                "Hook script already exists: {}. Use --force to overwrite",
                hook_script.display()
            )));
        }

        if hook_script.exists() && !opts.dry_run {
            if let Some(backup) = create_backup(&hook_script)? {
                backups.push((hook_script.clone(), backup.clone()));
                println!("✓ Backed up hook script to {}", backup.display());
            }
        }

        // Install session-start hook script
        let hook_content = include_str!("../../templates/session-start.sh");
        if !opts.dry_run {
            fs::write(&hook_script, hook_content).map_err(IntentError::IoError)?;
            set_executable(&hook_script)?;
            files_modified.push(hook_script.clone());
            println!("✓ Installed {}", hook_script.display());
        } else {
            println!("Would write: {}", hook_script.display());
        }

        // Install format-ie-output hook script
        let format_hook_script = hooks_dir.join("format-ie-output.sh");
        let format_hook_content = include_str!("../../templates/format-ie-output.sh");

        if format_hook_script.exists() && !opts.force {
            return Err(IntentError::InvalidInput(format!(
                "Format hook already exists: {}. Use --force to overwrite",
                format_hook_script.display()
            )));
        }

        if format_hook_script.exists() && !opts.dry_run {
            if let Some(backup) = create_backup(&format_hook_script)? {
                backups.push((format_hook_script.clone(), backup.clone()));
                println!("✓ Backed up format hook to {}", backup.display());
            }
        }

        if !opts.dry_run {
            fs::write(&format_hook_script, format_hook_content).map_err(IntentError::IoError)?;
            set_executable(&format_hook_script)?;
            files_modified.push(format_hook_script.clone());
            println!("✓ Installed {}", format_hook_script.display());
        } else {
            println!("Would write: {}", format_hook_script.display());
        }

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

        if settings_file.exists() && !opts.dry_run {
            if let Some(backup) = create_backup(&settings_file)? {
                backups.push((settings_file.clone(), backup.clone()));
                println!("✓ Backed up settings to {}", backup.display());
            }
        }

        let settings = json!({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": hook_abs_path.to_string_lossy()
                    }]
                }],
                "PostToolUse": [
                    {
                        "matcher": "mcp__intent-engine__task_context",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__task_get",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__current_task_get",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__task_list",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__task_pick_next",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__unified_search",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__event_list",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    }
                ]
            }
        });

        if !opts.dry_run {
            write_json_config(&settings_file, &settings)?;
            files_modified.push(settings_file.clone());
            println!("✓ Created {}", settings_file.display());
        } else {
            println!("Would write: {}", settings_file.display());
        }

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

        // Determine project directory
        let project_dir = if let Some(ref dir) = opts.project_dir {
            dir.clone()
        } else {
            env::current_dir().map_err(IntentError::IoError)?
        };
        let project_dir_abs = resolve_absolute_path(&project_dir)?;

        // Backup existing config
        if config_path.exists() && !opts.dry_run {
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
            "env": {
                "INTENT_ENGINE_PROJECT_DIR": project_dir_abs.to_string_lossy()
            },
            "description": "Strategic intent and task workflow management"
        });

        if !opts.dry_run {
            write_json_config(&config_path, &config)?;
            files_modified.push(config_path.clone());
            println!("✓ Updated {}", config_path.display());
        } else {
            println!("Would write: {}", config_path.display());
        }

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

        if !opts.dry_run {
            fs::create_dir_all(&hooks_dir).map_err(IntentError::IoError)?;
            println!("✓ Created {}", hooks_dir.display());
        } else {
            println!("Would create: {}", hooks_dir.display());
        }

        // Check if hook script already exists
        if hook_script.exists() && !opts.force {
            return Err(IntentError::InvalidInput(format!(
                "Hook script already exists: {}. Use --force to overwrite",
                hook_script.display()
            )));
        }

        // Install session-start hook script
        let hook_content = include_str!("../../templates/session-start.sh");
        if !opts.dry_run {
            fs::write(&hook_script, hook_content).map_err(IntentError::IoError)?;
            set_executable(&hook_script)?;
            files_modified.push(hook_script.clone());
            println!("✓ Installed {}", hook_script.display());
        } else {
            println!("Would write: {}", hook_script.display());
        }

        // Install format-ie-output hook script
        let format_hook_script = hooks_dir.join("format-ie-output.sh");
        let format_hook_content = include_str!("../../templates/format-ie-output.sh");

        if format_hook_script.exists() && !opts.force {
            return Err(IntentError::InvalidInput(format!(
                "Format hook already exists: {}. Use --force to overwrite",
                format_hook_script.display()
            )));
        }

        if !opts.dry_run {
            fs::write(&format_hook_script, format_hook_content).map_err(IntentError::IoError)?;
            set_executable(&format_hook_script)?;
            files_modified.push(format_hook_script.clone());
            println!("✓ Installed {}", format_hook_script.display());
        } else {
            println!("Would write: {}", format_hook_script.display());
        }

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

        let settings = json!({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": hook_abs_path.to_string_lossy()
                    }]
                }],
                "PostToolUse": [
                    {
                        "matcher": "mcp__intent-engine__task_context",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__task_get",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__current_task_get",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__task_list",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__task_pick_next",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__unified_search",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    },
                    {
                        "matcher": "mcp__intent-engine__event_list",
                        "hooks": [{
                            "type": "command",
                            "command": format_hook_abs_path.to_string_lossy()
                        }]
                    }
                ]
            }
        });

        if !opts.dry_run {
            write_json_config(&settings_file, &settings)?;
            files_modified.push(settings_file);
            println!("✓ Created settings.json");
        } else {
            println!("Would write: {}", settings_file.display());
        }

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

    fn diagnose(&self) -> Result<DiagnosisReport> {
        let mut checks = Vec::new();
        let mut suggested_fixes = Vec::new();

        // Check 1: Hook script exists and is executable
        let claude_dir = Self::get_user_claude_dir()?;
        let hook_script = claude_dir.join("hooks").join("session-start.sh");

        let hook_check = if hook_script.exists() {
            if hook_script.metadata().map(|m| m.is_file()).unwrap_or(false) {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = hook_script.metadata().unwrap().permissions();
                    let is_executable = perms.mode() & 0o111 != 0;
                    if is_executable {
                        DiagnosisCheck {
                            name: "Hook script".to_string(),
                            passed: true,
                            details: format!("Found at {}", hook_script.display()),
                        }
                    } else {
                        suggested_fixes.push(format!("chmod +x {}", hook_script.display()));
                        DiagnosisCheck {
                            name: "Hook script".to_string(),
                            passed: false,
                            details: "Script exists but is not executable".to_string(),
                        }
                    }
                }
                #[cfg(not(unix))]
                DiagnosisCheck {
                    name: "Hook script".to_string(),
                    passed: true,
                    details: format!("Found at {}", hook_script.display()),
                }
            } else {
                DiagnosisCheck {
                    name: "Hook script".to_string(),
                    passed: false,
                    details: "Path exists but is not a file".to_string(),
                }
            }
        } else {
            suggested_fixes.push("Run: ie setup --target claude-code".to_string());
            DiagnosisCheck {
                name: "Hook script".to_string(),
                passed: false,
                details: format!("Not found at {}", hook_script.display()),
            }
        };
        checks.push(hook_check);

        // Check 2: Format hook script exists and is executable
        let format_hook_script = claude_dir.join("hooks").join("format-ie-output.sh");
        let format_hook_check = if format_hook_script.exists() {
            if format_hook_script
                .metadata()
                .map(|m| m.is_file())
                .unwrap_or(false)
            {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = format_hook_script.metadata().unwrap().permissions();
                    let is_executable = perms.mode() & 0o111 != 0;
                    if is_executable {
                        DiagnosisCheck {
                            name: "Format hook script".to_string(),
                            passed: true,
                            details: format!("Found at {}", format_hook_script.display()),
                        }
                    } else {
                        suggested_fixes.push(format!("chmod +x {}", format_hook_script.display()));
                        DiagnosisCheck {
                            name: "Format hook script".to_string(),
                            passed: false,
                            details: "Script exists but is not executable".to_string(),
                        }
                    }
                }
                #[cfg(not(unix))]
                DiagnosisCheck {
                    name: "Format hook script".to_string(),
                    passed: true,
                    details: format!("Found at {}", format_hook_script.display()),
                }
            } else {
                DiagnosisCheck {
                    name: "Format hook script".to_string(),
                    passed: false,
                    details: "Path exists but is not a file".to_string(),
                }
            }
        } else {
            suggested_fixes.push("Run: ie setup --target claude-code --force".to_string());
            DiagnosisCheck {
                name: "Format hook script".to_string(),
                passed: false,
                details: format!("Not found at {}", format_hook_script.display()),
            }
        };
        checks.push(format_hook_check);

        // Check 3: Settings file has SessionStart config
        let settings_file = claude_dir.join("settings.json");
        let settings_check = if settings_file.exists() {
            match read_json_config(&settings_file) {
                Ok(config) => {
                    if config
                        .get("hooks")
                        .and_then(|h| h.get("SessionStart"))
                        .is_some()
                    {
                        DiagnosisCheck {
                            name: "Settings file".to_string(),
                            passed: true,
                            details: "SessionStart hook configured".to_string(),
                        }
                    } else {
                        suggested_fixes
                            .push("Run: ie setup --target claude-code --force".to_string());
                        DiagnosisCheck {
                            name: "Settings file".to_string(),
                            passed: false,
                            details: "Missing SessionStart hook configuration".to_string(),
                        }
                    }
                },
                Err(_) => DiagnosisCheck {
                    name: "Settings file".to_string(),
                    passed: false,
                    details: "Failed to parse settings.json".to_string(),
                },
            }
        } else {
            suggested_fixes.push("Run: ie setup --target claude-code".to_string());
            DiagnosisCheck {
                name: "Settings file".to_string(),
                passed: false,
                details: format!("Not found at {}", settings_file.display()),
            }
        };
        checks.push(settings_check);

        // Check 4: Settings file has PostToolUse config
        let posttool_check = if settings_file.exists() {
            match read_json_config(&settings_file) {
                Ok(config) => {
                    if config
                        .get("hooks")
                        .and_then(|h| h.get("PostToolUse"))
                        .is_some()
                    {
                        DiagnosisCheck {
                            name: "PostToolUse hooks".to_string(),
                            passed: true,
                            details: "PostToolUse hook configured".to_string(),
                        }
                    } else {
                        suggested_fixes
                            .push("Run: ie setup --target claude-code --force".to_string());
                        DiagnosisCheck {
                            name: "PostToolUse hooks".to_string(),
                            passed: false,
                            details: "Missing PostToolUse hook configuration".to_string(),
                        }
                    }
                },
                Err(_) => DiagnosisCheck {
                    name: "PostToolUse hooks".to_string(),
                    passed: false,
                    details: "Failed to parse settings.json".to_string(),
                },
            }
        } else {
            DiagnosisCheck {
                name: "PostToolUse hooks".to_string(),
                passed: false,
                details: "Settings file not found".to_string(),
            }
        };
        checks.push(posttool_check);

        // Check 5: MCP config exists and has intent-engine
        let home = get_home_dir()?;
        let mcp_config = home.join(".claude.json");
        let mcp_check = if mcp_config.exists() {
            match read_json_config(&mcp_config) {
                Ok(config) => {
                    if config
                        .get("mcpServers")
                        .and_then(|s| s.get("intent-engine"))
                        .is_some()
                    {
                        DiagnosisCheck {
                            name: "MCP configuration".to_string(),
                            passed: true,
                            details: "intent-engine MCP server configured".to_string(),
                        }
                    } else {
                        suggested_fixes
                            .push("Run: ie setup --target claude-code --force".to_string());
                        DiagnosisCheck {
                            name: "MCP configuration".to_string(),
                            passed: false,
                            details: "Missing intent-engine server entry".to_string(),
                        }
                    }
                },
                Err(_) => DiagnosisCheck {
                    name: "MCP configuration".to_string(),
                    passed: false,
                    details: "Failed to parse .claude.json".to_string(),
                },
            }
        } else {
            suggested_fixes.push("Run: ie setup --target claude-code".to_string());
            DiagnosisCheck {
                name: "MCP configuration".to_string(),
                passed: false,
                details: format!("Not found at {}", mcp_config.display()),
            }
        };
        checks.push(mcp_check);

        // Check 6: Binary in PATH
        let binary_check = match find_ie_binary() {
            Ok(path) => DiagnosisCheck {
                name: "Binary availability".to_string(),
                passed: true,
                details: format!("Found at {}", path.display()),
            },
            Err(_) => {
                suggested_fixes.push("Install: cargo install intent-engine".to_string());
                DiagnosisCheck {
                    name: "Binary availability".to_string(),
                    passed: false,
                    details: "intent-engine not found in PATH".to_string(),
                }
            },
        };
        checks.push(binary_check);

        let overall_status = checks.iter().all(|c| c.passed);

        Ok(DiagnosisReport {
            overall_status,
            checks,
            suggested_fixes,
        })
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
