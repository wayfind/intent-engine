//! Interactive setup wizard for configuring intent-engine with AI tools

use crate::error::{IntentError, Result};
use crate::setup::claude_code::ClaudeCodeSetup;
use crate::setup::{SetupModule, SetupOptions, SetupResult};
use dialoguer::{theme::ColorfulTheme, Select};

/// Available setup targets
#[derive(Debug, Clone)]
pub enum SetupTarget {
    /// Claude Code - fully supported
    ClaudeCode { status: TargetStatus },
    /// Gemini CLI - coming soon
    GeminiCli { status: TargetStatus },
    /// Codex - coming soon
    Codex { status: TargetStatus },
}

/// Status of a setup target
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetStatus {
    /// Already configured
    Configured,
    /// Partially configured or needs updates
    PartiallyConfigured,
    /// Not configured yet
    NotConfigured,
    /// Coming soon (not implemented)
    ComingSoon,
}

impl SetupTarget {
    /// Get display name for the target
    pub fn display_name(&self) -> &str {
        match self {
            SetupTarget::ClaudeCode { .. } => "Claude Code",
            SetupTarget::GeminiCli { .. } => "Gemini CLI",
            SetupTarget::Codex { .. } => "Codex",
        }
    }

    /// Get description for the target
    pub fn description(&self) -> &str {
        match self {
            SetupTarget::ClaudeCode { .. } => "Install session hooks for Claude Code integration",
            SetupTarget::GeminiCli { .. } => "Integration for Google Gemini CLI (coming soon)",
            SetupTarget::Codex { .. } => "Integration for OpenAI Codex (coming soon)",
        }
    }

    /// Get status icon
    pub fn status_icon(&self) -> &str {
        let status = match self {
            SetupTarget::ClaudeCode { status } => status,
            SetupTarget::GeminiCli { status } => status,
            SetupTarget::Codex { status } => status,
        };

        match status {
            TargetStatus::Configured => "✓",
            TargetStatus::PartiallyConfigured => "⚠",
            TargetStatus::NotConfigured => "○",
            TargetStatus::ComingSoon => "🔜",
        }
    }

    /// Get status description
    pub fn status_description(&self) -> String {
        let status = match self {
            SetupTarget::ClaudeCode { status } => status,
            SetupTarget::GeminiCli { status } => status,
            SetupTarget::Codex { status } => status,
        };

        match status {
            TargetStatus::Configured => "Already configured".to_string(),
            TargetStatus::PartiallyConfigured => "Partially configured".to_string(),
            TargetStatus::NotConfigured => {
                match self {
                    SetupTarget::ClaudeCode { .. } => {
                        // Check if .claude directory exists
                        if let Ok(home) = crate::setup::common::get_home_dir() {
                            let claude_dir = home.join(".claude");
                            if claude_dir.exists() {
                                "Detected at ~/.claude/".to_string()
                            } else {
                                "Not configured".to_string()
                            }
                        } else {
                            "Not configured".to_string()
                        }
                    },
                    _ => "Not configured".to_string(),
                }
            },
            TargetStatus::ComingSoon => "Not yet supported".to_string(),
        }
    }

    /// Format for display in selection menu
    pub fn format_for_menu(&self) -> String {
        format!(
            "{} {} - {}\n    Status: {}",
            self.status_icon(),
            self.display_name(),
            self.description(),
            self.status_description()
        )
    }

    /// Check if target is selectable (implemented)
    pub fn is_selectable(&self) -> bool {
        matches!(self, SetupTarget::ClaudeCode { .. })
    }
}

/// Interactive setup wizard
pub struct SetupWizard {
    targets: Vec<SetupTarget>,
}

impl SetupWizard {
    /// Create a new setup wizard
    pub fn new() -> Self {
        Self {
            targets: vec![
                SetupTarget::ClaudeCode {
                    status: Self::detect_claude_code_status(),
                },
                SetupTarget::GeminiCli {
                    status: TargetStatus::ComingSoon,
                },
                SetupTarget::Codex {
                    status: TargetStatus::ComingSoon,
                },
            ],
        }
    }

    /// Detect Claude Code configuration status
    fn detect_claude_code_status() -> TargetStatus {
        let home = match crate::setup::common::get_home_dir() {
            Ok(h) => h,
            Err(_) => return TargetStatus::NotConfigured,
        };

        let claude_json = home.join(".claude.json");
        let hooks_dir = home.join(".claude/hooks");
        let settings_json = home.join(".claude/settings.json");

        let has_mcp = claude_json.exists();
        let has_hooks = hooks_dir.exists();
        let has_settings = settings_json.exists();

        if has_mcp && has_hooks && has_settings {
            TargetStatus::Configured
        } else if has_mcp || has_hooks || has_settings {
            TargetStatus::PartiallyConfigured
        } else {
            TargetStatus::NotConfigured
        }
    }

    /// Run the interactive wizard
    pub fn run(&self, opts: &SetupOptions) -> Result<SetupResult> {
        println!("\n🚀 Intent-Engine Setup Wizard\n");
        println!("Please select the tool you want to configure:\n");

        let items: Vec<String> = self.targets.iter().map(|t| t.format_for_menu()).collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .items(&items)
            .default(0)
            .interact()
            .map_err(|e| IntentError::InvalidInput(format!("Selection cancelled: {}", e)))?;

        let selected_target = &self.targets[selection];

        // Check if target is selectable
        if !selected_target.is_selectable() {
            println!("\n⚠️  This target is not yet supported.\n");
            println!(
                "The {} integration is planned for a future release.",
                selected_target.display_name()
            );
            println!("\nCurrently supported:");
            println!("  • Claude Code (fully functional)\n");
            println!(
                "Want to see {} support sooner?",
                selected_target.display_name()
            );
            println!("👉 Vote for it: https://github.com/wayfind/intent-engine/issues\n");

            return Ok(SetupResult {
                success: false,
                message: format!("{} is not yet supported", selected_target.display_name()),
                files_modified: vec![],
                connectivity_test: None,
            });
        }

        // Execute setup for the selected target
        match selected_target {
            SetupTarget::ClaudeCode { .. } => {
                println!("\n📦 Setting up Claude Code integration...\n");
                let module = ClaudeCodeSetup;
                module.setup(opts)
            },
            _ => Err(IntentError::InvalidInput(
                "Target not implemented".to_string(),
            )),
        }
    }
}

impl Default for SetupWizard {
    fn default() -> Self {
        Self::new()
    }
}
