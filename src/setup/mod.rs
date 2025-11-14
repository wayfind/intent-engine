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
    /// Project directory for INTENT_ENGINE_PROJECT_DIR env var
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
