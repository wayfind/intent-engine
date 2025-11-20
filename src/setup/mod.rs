//! Setup module for configuring intent-engine integration with AI tools
//!
//! This module provides a unified setup command that configures both hooks and MCP servers
//! for various AI assistant tools (Claude Code, Gemini CLI, Codex, etc.)

use crate::error::{IntentError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

pub mod claude_code;
pub mod common;
pub mod interactive;

/// Installation scope for setup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupScope {
    /// User-level installation (recommended, works across all projects)
    User,
    /// Project-level installation (specific to current project)
    Project,
    /// Both user and project level
    Both,
}

impl FromStr for SetupScope {
    type Err = IntentError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "user" => Ok(SetupScope::User),
            "project" => Ok(SetupScope::Project),
            "both" => Ok(SetupScope::Both),
            _ => Err(IntentError::InvalidInput(format!(
                "Invalid scope: {}. Must be 'user', 'project', or 'both'",
                s
            ))),
        }
    }
}

/// Options for setup operations
#[derive(Debug, Clone)]
pub struct SetupOptions {
    /// Installation scope
    pub scope: SetupScope,
    /// Dry run mode (show what would be done)
    pub dry_run: bool,
    /// Force overwrite existing configuration
    pub force: bool,
    /// Custom config file path (optional)
    pub config_path: Option<PathBuf>,
    /// Project directory (auto-detected, not user-specified)
    pub project_dir: Option<PathBuf>,
}

impl Default for SetupOptions {
    fn default() -> Self {
        Self {
            scope: SetupScope::User,
            dry_run: false,
            force: false,
            config_path: None,
            project_dir: None,
        }
    }
}

/// Result of a setup operation
#[derive(Debug, Serialize, Deserialize)]
pub struct SetupResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Files that were created or modified
    pub files_modified: Vec<PathBuf>,
    /// Connectivity test result (if performed)
    pub connectivity_test: Option<ConnectivityResult>,
}

/// Result of a connectivity test
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectivityResult {
    /// Whether the test passed
    pub passed: bool,
    /// Test details
    pub details: String,
}

/// Result of a diagnosis operation
#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosisReport {
    /// Overall status (all checks passed?)
    pub overall_status: bool,
    /// Individual check results
    pub checks: Vec<DiagnosisCheck>,
    /// Suggested fixes for failed checks
    pub suggested_fixes: Vec<String>,
}

/// Individual diagnosis check
#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosisCheck {
    /// Check name
    pub name: String,
    /// Whether the check passed
    pub passed: bool,
    /// Details or error message
    pub details: String,
}

/// Trait for setup modules
pub trait SetupModule {
    /// Module name (e.g., "claude-code")
    fn name(&self) -> &str;

    /// Perform setup
    fn setup(&self, opts: &SetupOptions) -> Result<SetupResult>;

    /// Diagnose existing setup
    fn diagnose(&self) -> Result<DiagnosisReport>;

    /// Test connectivity
    fn test_connectivity(&self) -> Result<ConnectivityResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_scope_from_str_valid() {
        assert_eq!(SetupScope::from_str("user").unwrap(), SetupScope::User);
        assert_eq!(SetupScope::from_str("User").unwrap(), SetupScope::User);
        assert_eq!(SetupScope::from_str("USER").unwrap(), SetupScope::User);

        assert_eq!(
            SetupScope::from_str("project").unwrap(),
            SetupScope::Project
        );
        assert_eq!(
            SetupScope::from_str("Project").unwrap(),
            SetupScope::Project
        );

        assert_eq!(SetupScope::from_str("both").unwrap(), SetupScope::Both);
        assert_eq!(SetupScope::from_str("Both").unwrap(), SetupScope::Both);
        assert_eq!(SetupScope::from_str("BOTH").unwrap(), SetupScope::Both);
    }

    #[test]
    fn test_setup_scope_from_str_invalid() {
        assert!(SetupScope::from_str("invalid").is_err());
        assert!(SetupScope::from_str("").is_err());
        assert!(SetupScope::from_str("global").is_err());

        // Test error message
        match SetupScope::from_str("invalid") {
            Err(IntentError::InvalidInput(msg)) => {
                assert!(msg.contains("Invalid scope"));
                assert!(msg.contains("invalid"));
            },
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_setup_options_default() {
        let opts = SetupOptions::default();
        assert_eq!(opts.scope, SetupScope::User);
        assert!(!opts.dry_run);
        assert!(!opts.force);
        assert!(opts.config_path.is_none());
        assert!(opts.project_dir.is_none());
    }

    #[test]
    fn test_setup_scope_equality() {
        assert_eq!(SetupScope::User, SetupScope::User);
        assert_eq!(SetupScope::Project, SetupScope::Project);
        assert_eq!(SetupScope::Both, SetupScope::Both);

        assert_ne!(SetupScope::User, SetupScope::Project);
        assert_ne!(SetupScope::User, SetupScope::Both);
        assert_ne!(SetupScope::Project, SetupScope::Both);
    }

    #[test]
    fn test_setup_options_custom() {
        let custom_path = PathBuf::from("/custom/config.json");
        let project_dir = PathBuf::from("/my/project");

        let opts = SetupOptions {
            scope: SetupScope::Both,
            dry_run: true,
            force: true,
            config_path: Some(custom_path.clone()),
            project_dir: Some(project_dir.clone()),
        };

        assert_eq!(opts.scope, SetupScope::Both);
        assert!(opts.dry_run);
        assert!(opts.force);
        assert_eq!(opts.config_path, Some(custom_path));
        assert_eq!(opts.project_dir, Some(project_dir));
    }
}
