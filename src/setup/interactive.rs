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
            SetupTarget::ClaudeCode { .. } => {
                "Install MCP server and session hooks for Claude Code"
            },
            SetupTarget::GeminiCli { .. } => "Install MCP server for Google Gemini CLI",
            SetupTarget::Codex { .. } => "Install MCP server for OpenAI Codex",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_target_display_name() {
        let claude = SetupTarget::ClaudeCode {
            status: TargetStatus::NotConfigured,
        };
        assert_eq!(claude.display_name(), "Claude Code");

        let gemini = SetupTarget::GeminiCli {
            status: TargetStatus::ComingSoon,
        };
        assert_eq!(gemini.display_name(), "Gemini CLI");

        let codex = SetupTarget::Codex {
            status: TargetStatus::ComingSoon,
        };
        assert_eq!(codex.display_name(), "Codex");
    }

    #[test]
    fn test_setup_target_description() {
        let claude = SetupTarget::ClaudeCode {
            status: TargetStatus::NotConfigured,
        };
        assert!(claude.description().contains("MCP server"));
        assert!(claude.description().contains("session hooks"));

        let gemini = SetupTarget::GeminiCli {
            status: TargetStatus::ComingSoon,
        };
        assert!(gemini.description().contains("Gemini"));

        let codex = SetupTarget::Codex {
            status: TargetStatus::ComingSoon,
        };
        assert!(codex.description().contains("Codex"));
    }

    #[test]
    fn test_setup_target_status_icon() {
        let configured = SetupTarget::ClaudeCode {
            status: TargetStatus::Configured,
        };
        assert_eq!(configured.status_icon(), "✓");

        let partial = SetupTarget::ClaudeCode {
            status: TargetStatus::PartiallyConfigured,
        };
        assert_eq!(partial.status_icon(), "⚠");

        let not_configured = SetupTarget::ClaudeCode {
            status: TargetStatus::NotConfigured,
        };
        assert_eq!(not_configured.status_icon(), "○");

        let coming_soon = SetupTarget::GeminiCli {
            status: TargetStatus::ComingSoon,
        };
        assert_eq!(coming_soon.status_icon(), "🔜");
    }

    #[test]
    fn test_setup_target_status_description() {
        let configured = SetupTarget::ClaudeCode {
            status: TargetStatus::Configured,
        };
        assert_eq!(configured.status_description(), "Already configured");

        let partial = SetupTarget::ClaudeCode {
            status: TargetStatus::PartiallyConfigured,
        };
        assert_eq!(partial.status_description(), "Partially configured");

        let coming_soon = SetupTarget::GeminiCli {
            status: TargetStatus::ComingSoon,
        };
        assert_eq!(coming_soon.status_description(), "Not yet supported");

        let not_configured = SetupTarget::ClaudeCode {
            status: TargetStatus::NotConfigured,
        };
        // Should either say "Not configured" or "Detected at ~/.claude/"
        let desc = not_configured.status_description();
        assert!(
            desc.contains("Not configured") || desc.contains("Detected"),
            "Unexpected description: {}",
            desc
        );
    }

    #[test]
    fn test_setup_target_format_for_menu() {
        let target = SetupTarget::ClaudeCode {
            status: TargetStatus::Configured,
        };

        let formatted = target.format_for_menu();

        // Should contain all components
        assert!(formatted.contains("✓")); // status icon
        assert!(formatted.contains("Claude Code")); // display name
        assert!(formatted.contains("MCP server")); // description
        assert!(formatted.contains("Status:")); // status label
        assert!(formatted.contains("Already configured")); // status description
    }

    #[test]
    fn test_setup_target_is_selectable() {
        let claude = SetupTarget::ClaudeCode {
            status: TargetStatus::NotConfigured,
        };
        assert!(claude.is_selectable());

        let gemini = SetupTarget::GeminiCli {
            status: TargetStatus::ComingSoon,
        };
        assert!(!gemini.is_selectable());

        let codex = SetupTarget::Codex {
            status: TargetStatus::ComingSoon,
        };
        assert!(!codex.is_selectable());
    }

    #[test]
    fn test_target_status_equality() {
        assert_eq!(TargetStatus::Configured, TargetStatus::Configured);
        assert_eq!(
            TargetStatus::PartiallyConfigured,
            TargetStatus::PartiallyConfigured
        );
        assert_eq!(TargetStatus::NotConfigured, TargetStatus::NotConfigured);
        assert_eq!(TargetStatus::ComingSoon, TargetStatus::ComingSoon);

        assert_ne!(TargetStatus::Configured, TargetStatus::NotConfigured);
        assert_ne!(TargetStatus::Configured, TargetStatus::ComingSoon);
    }

    #[test]
    fn test_setup_wizard_new() {
        let wizard = SetupWizard::new();

        // Should have 3 targets
        assert_eq!(wizard.targets.len(), 3);

        // First should be ClaudeCode
        assert!(wizard.targets[0].is_selectable());

        // Others should be coming soon
        assert!(!wizard.targets[1].is_selectable());
        assert!(!wizard.targets[2].is_selectable());
    }

    #[test]
    fn test_setup_wizard_default() {
        let wizard1 = SetupWizard::default();
        let wizard2 = SetupWizard::new();

        // Should produce same result
        assert_eq!(wizard1.targets.len(), wizard2.targets.len());
    }

    #[test]
    fn test_detect_claude_code_status() {
        // This will vary based on the test environment
        // Just verify it returns a valid status
        let status = SetupWizard::detect_claude_code_status();

        match status {
            TargetStatus::Configured
            | TargetStatus::PartiallyConfigured
            | TargetStatus::NotConfigured => {
                // All valid
            },
            TargetStatus::ComingSoon => {
                panic!("detect_claude_code_status should not return ComingSoon")
            },
        }
    }
}
