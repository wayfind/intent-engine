//! Global Projects Registry
//!
//! Manages a global list of all projects that have used Intent-Engine.
//! This allows the Dashboard to show all known projects even when CLI is not running.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const GLOBAL_DIR: &str = ".intent-engine";
const PROJECTS_FILE: &str = "projects.json";

/// A registered project entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    /// Absolute path to the project root
    pub path: String,
    /// Last time this project was accessed via CLI
    pub last_accessed: DateTime<Utc>,
    /// Optional display name (defaults to directory name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Global projects registry
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectsRegistry {
    pub projects: Vec<ProjectEntry>,
}

impl ProjectsRegistry {
    /// Get the path to the global projects file
    pub fn registry_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(GLOBAL_DIR).join(PROJECTS_FILE))
    }

    /// Load the registry from disk
    pub fn load() -> Self {
        let Some(path) = Self::registry_path() else {
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save the registry to disk
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::registry_path() else {
            return Ok(());
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)
    }

    /// Register or update a project
    pub fn register_project(&mut self, project_path: &Path) {
        let path_str = project_path.to_string_lossy().to_string();
        let now = Utc::now();

        // Check if project already exists
        if let Some(entry) = self.projects.iter_mut().find(|p| p.path == path_str) {
            entry.last_accessed = now;
        } else {
            // Add new project
            let name = project_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());

            self.projects.push(ProjectEntry {
                path: path_str,
                last_accessed: now,
                name,
            });
        }
    }

    /// Remove a project from the registry
    pub fn remove_project(&mut self, project_path: &str) -> bool {
        let initial_len = self.projects.len();
        self.projects.retain(|p| p.path != project_path);
        self.projects.len() < initial_len
    }

    /// Get all registered projects sorted by last_accessed (most recent first)
    pub fn get_projects(&self) -> Vec<&ProjectEntry> {
        let mut projects: Vec<_> = self.projects.iter().collect();
        projects.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        projects
    }

    /// Check if a project exists and has a valid database
    pub fn validate_project(path: &str) -> bool {
        let project_path = PathBuf::from(path);
        let db_path = project_path.join(".intent-engine").join("project.db");
        db_path.exists()
    }
}

/// Register a project in the global registry (convenience function)
pub fn register_project(project_path: &Path) {
    let mut registry = ProjectsRegistry::load();
    registry.register_project(project_path);
    if let Err(e) = registry.save() {
        tracing::warn!("Failed to save global projects registry: {}", e);
    }
}

/// Remove a project from the global registry (convenience function)
pub fn remove_project(project_path: &str) -> bool {
    let mut registry = ProjectsRegistry::load();
    let removed = registry.remove_project(project_path);
    if removed {
        if let Err(e) = registry.save() {
            tracing::warn!("Failed to save global projects registry: {}", e);
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_project_entry_serialization() {
        let entry = ProjectEntry {
            path: "/test/project".to_string(),
            last_accessed: Utc::now(),
            name: Some("project".to_string()),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: ProjectEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, entry.path);
    }

    #[test]
    fn test_registry_register_and_remove() {
        let mut registry = ProjectsRegistry::default();

        // Register a project
        let temp = TempDir::new().unwrap();
        registry.register_project(temp.path());
        assert_eq!(registry.projects.len(), 1);

        // Register same project again (should update, not duplicate)
        registry.register_project(temp.path());
        assert_eq!(registry.projects.len(), 1);

        // Remove project
        let path_str = temp.path().to_string_lossy().to_string();
        assert!(registry.remove_project(&path_str));
        assert_eq!(registry.projects.len(), 0);
    }

    #[test]
    fn test_registry_get_projects_sorted() {
        let mut registry = ProjectsRegistry::default();

        // Add projects with different timestamps
        registry.projects.push(ProjectEntry {
            path: "/old".to_string(),
            last_accessed: Utc::now() - chrono::Duration::hours(2),
            name: None,
        });
        registry.projects.push(ProjectEntry {
            path: "/new".to_string(),
            last_accessed: Utc::now(),
            name: None,
        });

        let projects = registry.get_projects();
        assert_eq!(projects[0].path, "/new");
        assert_eq!(projects[1].path, "/old");
    }
}
