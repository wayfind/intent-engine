use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const REGISTRY_FILE: &str = ".intent-engine/projects.json";
const DEFAULT_PORT: u16 = 11391; // Fixed port for Dashboard
const VERSION: &str = "1.0";

/// Global project registry for managing multiple Dashboard instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRegistry {
    pub version: String,
    pub projects: Vec<RegisteredProject>,
}

/// A registered project with Dashboard instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredProject {
    pub path: PathBuf,
    pub name: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub started_at: String,
    pub db_path: PathBuf,

    // MCP connection tracking
    #[serde(default)]
    pub mcp_connected: bool,
    #[serde(default)]
    pub mcp_last_seen: Option<String>,
    #[serde(default)]
    pub mcp_agent: Option<String>,
}

impl ProjectRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            version: VERSION.to_string(),
            projects: Vec::new(),
        }
    }

    /// Get the registry file path
    fn registry_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to get home directory")?;
        Ok(home.join(REGISTRY_FILE))
    }

    /// Load registry from file, or create new if doesn't exist
    pub fn load() -> Result<Self> {
        let path = Self::registry_path()?;

        if !path.exists() {
            // Create parent directory if needed
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).context("Failed to create registry directory")?;
            }
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&path).context("Failed to read registry file")?;

        let registry: Self =
            serde_json::from_str(&content).context("Failed to parse registry JSON")?;

        Ok(registry)
    }

    /// Save registry to file
    pub fn save(&self) -> Result<()> {
        let path = Self::registry_path()?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create registry directory")?;
        }

        let content = serde_json::to_string_pretty(self).context("Failed to serialize registry")?;

        fs::write(&path, content).context("Failed to write registry file")?;

        Ok(())
    }

    /// Allocate port (always uses DEFAULT_PORT)
    pub fn allocate_port(&mut self) -> Result<u16> {
        // Always use the default fixed port
        let port = DEFAULT_PORT;

        // Check if port is available on the system
        if Self::is_port_available(port) {
            Ok(port)
        } else {
            anyhow::bail!(
                "Port {} is already in use. Please stop the existing Dashboard instance first.",
                port
            )
        }
    }

    /// Check if a port is available on the system
    pub fn is_port_available(port: u16) -> bool {
        use std::net::TcpListener;
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    /// Register a new project
    pub fn register(&mut self, project: RegisteredProject) {
        // Remove existing entry for the same path if exists
        self.unregister(&project.path);
        self.projects.push(project);
    }

    /// Unregister a project by path
    pub fn unregister(&mut self, path: &PathBuf) {
        self.projects.retain(|p| p.path != *path);
    }

    /// Find project by path
    pub fn find_by_path(&self, path: &PathBuf) -> Option<&RegisteredProject> {
        self.projects.iter().find(|p| p.path == *path)
    }

    /// Find project by path (mutable)
    pub fn find_by_path_mut(&mut self, path: &PathBuf) -> Option<&mut RegisteredProject> {
        self.projects.iter_mut().find(|p| p.path == *path)
    }

    /// Find project by port
    pub fn find_by_port(&self, port: u16) -> Option<&RegisteredProject> {
        self.projects.iter().find(|p| p.port == port)
    }

    /// Get all registered projects
    pub fn list_all(&self) -> &[RegisteredProject] {
        &self.projects
    }

    /// Register or update MCP connection for a project
    /// This will create a project entry if none exists (for MCP-only projects)
    pub fn register_mcp_connection(
        &mut self,
        path: &PathBuf,
        agent_name: Option<String>,
    ) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Check if project already exists
        if let Some(project) = self.find_by_path_mut(path) {
            // Update existing project's MCP status
            project.mcp_connected = true;
            project.mcp_last_seen = Some(now.clone());
            project.mcp_agent = agent_name;
        } else {
            // Create MCP-only project entry (no Dashboard server, port: 0)
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let db_path = path.join(".intent-engine").join("project.db");

            let project = RegisteredProject {
                path: path.clone(),
                name,
                port: 0, // No Dashboard server
                pid: None,
                started_at: now.clone(),
                db_path,
                mcp_connected: true,
                mcp_last_seen: Some(now),
                mcp_agent: agent_name,
            };

            self.projects.push(project);
        }

        self.save()
    }

    /// Update MCP heartbeat
    pub fn update_mcp_heartbeat(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(project) = self.find_by_path_mut(path) {
            project.mcp_last_seen = Some(chrono::Utc::now().to_rfc3339());
            project.mcp_connected = true;
            self.save()?;
        }
        Ok(())
    }

    /// Unregister MCP connection
    pub fn unregister_mcp_connection(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(project) = self.find_by_path_mut(path) {
            project.mcp_connected = false;
            project.mcp_last_seen = None;
            project.mcp_agent = None;

            // Don't delete the entry - keep it for tracking purposes
            // This allows MCP-only projects to persist in the registry
            self.save()?;
        }
        Ok(())
    }

    /// Clean up projects with dead PIDs
    pub fn cleanup_dead_processes(&mut self) {
        self.projects.retain(|project| {
            if let Some(pid) = project.pid {
                Self::is_process_alive(pid)
            } else {
                true // Keep projects without PID
            }
        });
    }

    /// Clean up stale MCP connections (no heartbeat for 5 minutes)
    pub fn cleanup_stale_mcp_connections(&mut self) {
        use chrono::DateTime;
        let now = chrono::Utc::now();
        const TIMEOUT_MINUTES: i64 = 5;

        for project in &mut self.projects {
            if let Some(last_seen) = &project.mcp_last_seen {
                if let Ok(last_time) = DateTime::parse_from_rfc3339(last_seen) {
                    let duration = now.signed_duration_since(last_time.with_timezone(&chrono::Utc));
                    if duration.num_minutes() > TIMEOUT_MINUTES {
                        project.mcp_connected = false;
                        project.mcp_last_seen = None;
                        project.mcp_agent = None;
                    }
                }
            }
        }

        // Remove MCP-only projects that are disconnected (port = 0 and not connected)
        self.projects.retain(|p| p.port != 0 || p.mcp_connected);
    }

    /// Check if a process is alive
    #[cfg(unix)]
    fn is_process_alive(pid: u32) -> bool {
        use std::process::Command;
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    fn is_process_alive(pid: u32) -> bool {
        use std::process::Command;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}

impl Default for ProjectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_registry() {
        let registry = ProjectRegistry::new();
        assert_eq!(registry.version, VERSION);
        assert_eq!(registry.projects.len(), 0);
    }

    #[test]
    #[serial_test::serial]
    fn test_allocate_port() {
        let mut registry = ProjectRegistry::new();

        // Attempt to allocate port - may fail if Dashboard is running
        match registry.allocate_port() {
            Ok(port) => {
                // Port is available - verify it's the default port
                assert_eq!(port, DEFAULT_PORT);

                // Verify we can register a project with that port
                registry.register(RegisteredProject {
                    path: PathBuf::from("/test/project1"),
                    name: "project1".to_string(),
                    port,
                    pid: None,
                    started_at: "2025-01-01T00:00:00Z".to_string(),
                    db_path: PathBuf::from("/test/project1/.intent-engine/intents.db"),
                    mcp_connected: false,
                    mcp_last_seen: None,
                    mcp_agent: None,
                });
            },
            Err(e) => {
                // Port in use is acceptable - verifies is_port_available() works correctly
                assert!(
                    e.to_string().contains("already in use"),
                    "Expected 'already in use' error, got: {}",
                    e
                );
            },
        }
    }

    #[test]
    fn test_register_and_find() {
        let mut registry = ProjectRegistry::new();

        let project = RegisteredProject {
            path: PathBuf::from("/test/project"),
            name: "test-project".to_string(),
            port: 11391,
            pid: Some(12345),
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        registry.register(project.clone());
        assert_eq!(registry.projects.len(), 1);

        // Find by path
        let found = registry.find_by_path(&PathBuf::from("/test/project"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test-project");

        // Find by port
        let found_by_port = registry.find_by_port(11391);
        assert!(found_by_port.is_some());
        assert_eq!(found_by_port.unwrap().name, "test-project");
    }

    #[test]
    fn test_unregister() {
        let mut registry = ProjectRegistry::new();

        let project = RegisteredProject {
            path: PathBuf::from("/test/project"),
            name: "test-project".to_string(),
            port: 11391,
            pid: None,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        registry.register(project.clone());
        assert_eq!(registry.projects.len(), 1);

        registry.unregister(&PathBuf::from("/test/project"));
        assert_eq!(registry.projects.len(), 0);
    }

    #[test]
    fn test_duplicate_path_replaces() {
        let mut registry = ProjectRegistry::new();

        let project1 = RegisteredProject {
            path: PathBuf::from("/test/project"),
            name: "project-v1".to_string(),
            port: 11391,
            pid: None,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        let project2 = RegisteredProject {
            path: PathBuf::from("/test/project"),
            name: "project-v2".to_string(),
            port: 3031,
            pid: None,
            started_at: "2025-01-01T01:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        registry.register(project1);
        assert_eq!(registry.projects.len(), 1);

        registry.register(project2);
        assert_eq!(registry.projects.len(), 1);

        let found = registry.find_by_path(&PathBuf::from("/test/project"));
        assert_eq!(found.unwrap().name, "project-v2");
    }

    #[test]
    fn test_save_and_load() {
        let _temp_dir = TempDir::new().unwrap();

        // We can't easily override home_dir in tests, so we'll test serialization manually
        let mut registry = ProjectRegistry::new();

        let project = RegisteredProject {
            path: PathBuf::from("/test/project"),
            name: "test-project".to_string(),
            port: 11391,
            pid: Some(12345),
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        registry.register(project);

        // Test serialization
        let json = serde_json::to_string_pretty(&registry).unwrap();
        assert!(json.contains("test-project"));
        assert!(json.contains("11391"));

        // Test deserialization
        let loaded: ProjectRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.projects[0].name, "test-project");
        assert_eq!(loaded.projects[0].port, 11391);
    }

    #[test]
    #[serial_test::serial]
    fn test_fixed_port() {
        let mut registry = ProjectRegistry::new();

        // Attempt to allocate port - may fail if Dashboard is running
        match registry.allocate_port() {
            Ok(port) => {
                // Port is available - verify it's the default port
                assert_eq!(port, DEFAULT_PORT);
            },
            Err(e) => {
                // Port in use is acceptable - verifies is_port_available() works correctly
                assert!(
                    e.to_string().contains("already in use"),
                    "Expected 'already in use' error, got: {}",
                    e
                );
            },
        }
    }

    #[test]
    fn test_list_all() {
        let mut registry = ProjectRegistry::new();

        // Initially empty
        assert_eq!(registry.list_all().len(), 0);

        // Add projects
        let project1 = RegisteredProject {
            path: PathBuf::from("/test/project1"),
            name: "project1".to_string(),
            port: 11391,
            pid: None,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project1/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        let project2 = RegisteredProject {
            path: PathBuf::from("/test/project2"),
            name: "project2".to_string(),
            port: 3031,
            pid: None,
            started_at: "2025-01-01T01:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project2/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        registry.register(project1);
        registry.register(project2);

        assert_eq!(registry.list_all().len(), 2);
    }

    #[test]
    fn test_find_by_path_mut() {
        let mut registry = ProjectRegistry::new();

        let project = RegisteredProject {
            path: PathBuf::from("/test/project"),
            name: "original".to_string(),
            port: 11391,
            pid: None,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        registry.register(project);

        // Mutate via mutable reference
        if let Some(p) = registry.find_by_path_mut(&PathBuf::from("/test/project")) {
            p.name = "modified".to_string();
        }

        // Verify change
        let found = registry.find_by_path(&PathBuf::from("/test/project"));
        assert_eq!(found.unwrap().name, "modified");
    }

    #[test]
    fn test_register_mcp_connection_new_project() {
        let mut registry = ProjectRegistry::new();
        let path = PathBuf::from("/test/mcp-project");

        // Register MCP connection for non-existent project
        registry
            .register_mcp_connection(&path, Some("claude-code".to_string()))
            .ok(); // Ignore save error in test

        // Should create new project with port 0
        let found = registry.find_by_path(&path);
        assert!(found.is_some());
        let project = found.unwrap();
        assert_eq!(project.port, 0); // MCP-only project
        assert!(project.mcp_connected);
        assert_eq!(project.mcp_agent, Some("claude-code".to_string()));
    }

    #[test]
    fn test_register_mcp_connection_existing_project() {
        let mut registry = ProjectRegistry::new();
        let path = PathBuf::from("/test/project");

        // Register regular project first
        let project = RegisteredProject {
            path: path.clone(),
            name: "test-project".to_string(),
            port: 11391,
            pid: None,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        registry.register(project);

        // Now register MCP connection
        registry
            .register_mcp_connection(&path, Some("vscode".to_string()))
            .ok();

        // Should update existing project
        let found = registry.find_by_path(&path);
        assert!(found.is_some());
        let project = found.unwrap();
        assert_eq!(project.port, 11391); // Keep original port
        assert!(project.mcp_connected);
        assert_eq!(project.mcp_agent, Some("vscode".to_string()));
        assert!(project.mcp_last_seen.is_some());
    }

    #[test]
    fn test_update_mcp_heartbeat() {
        let mut registry = ProjectRegistry::new();
        let path = PathBuf::from("/test/project");

        // Register project
        let project = RegisteredProject {
            path: path.clone(),
            name: "test-project".to_string(),
            port: 11391,
            pid: None,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        registry.register(project);

        // Update heartbeat
        registry.update_mcp_heartbeat(&path).ok();

        let found = registry.find_by_path(&path);
        assert!(found.unwrap().mcp_connected);
        assert!(found.unwrap().mcp_last_seen.is_some());
    }

    #[test]
    fn test_unregister_mcp_connection() {
        let mut registry = ProjectRegistry::new();
        let path = PathBuf::from("/test/project");

        // Register project with MCP
        registry
            .register_mcp_connection(&path, Some("claude".to_string()))
            .ok();

        assert!(registry.find_by_path(&path).unwrap().mcp_connected);

        // Unregister MCP
        registry.unregister_mcp_connection(&path).ok();

        let found = registry.find_by_path(&path);
        assert!(found.is_some()); // Project still exists
        assert!(!found.unwrap().mcp_connected);
        assert!(found.unwrap().mcp_last_seen.is_none());
        assert!(found.unwrap().mcp_agent.is_none());
    }

    #[test]
    fn test_cleanup_stale_mcp_connections() {
        use chrono::{Duration, Utc};

        let mut registry = ProjectRegistry::new();

        // Create project with old MCP connection (10 minutes ago)
        let old_time = Utc::now() - Duration::minutes(10);
        let path1 = PathBuf::from("/test/project1");

        let project1 = RegisteredProject {
            path: path1.clone(),
            name: "stale-project".to_string(),
            port: 0, // MCP-only
            pid: None,
            started_at: old_time.to_rfc3339(),
            db_path: PathBuf::from("/test/project1/.intent-engine/intents.db"),
            mcp_connected: true,
            mcp_last_seen: Some(old_time.to_rfc3339()),
            mcp_agent: Some("old-agent".to_string()),
        };

        registry.projects.push(project1);

        // Create project with recent MCP connection (1 minute ago)
        let recent_time = Utc::now() - Duration::minutes(1);
        let path2 = PathBuf::from("/test/project2");

        let project2 = RegisteredProject {
            path: path2.clone(),
            name: "active-project".to_string(),
            port: 11391,
            pid: None,
            started_at: recent_time.to_rfc3339(),
            db_path: PathBuf::from("/test/project2/.intent-engine/intents.db"),
            mcp_connected: true,
            mcp_last_seen: Some(recent_time.to_rfc3339()),
            mcp_agent: Some("active-agent".to_string()),
        };

        registry.projects.push(project2);

        // Run cleanup
        registry.cleanup_stale_mcp_connections();

        // Stale MCP-only project should be removed
        assert!(registry.find_by_path(&path1).is_none());

        // Active project should remain
        let found = registry.find_by_path(&path2);
        assert!(found.is_some());
        assert!(found.unwrap().mcp_connected);
    }

    #[test]
    fn test_cleanup_dead_processes() {
        let mut registry = ProjectRegistry::new();

        // Add project with obviously invalid PID
        let project_dead = RegisteredProject {
            path: PathBuf::from("/test/dead"),
            name: "dead-project".to_string(),
            port: 11391,
            pid: Some(999999), // Very unlikely to exist
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/dead/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        // Add project without PID
        let project_no_pid = RegisteredProject {
            path: PathBuf::from("/test/no-pid"),
            name: "no-pid-project".to_string(),
            port: 3031,
            pid: None,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/no-pid/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        };

        registry.register(project_dead);
        registry.register(project_no_pid);

        assert_eq!(registry.projects.len(), 2);

        // Cleanup
        registry.cleanup_dead_processes();

        // Dead process should be removed, no-PID should remain
        assert_eq!(registry.projects.len(), 1);
        assert_eq!(registry.projects[0].name, "no-pid-project");
    }

    #[test]
    fn test_default() {
        let registry = ProjectRegistry::default();
        assert_eq!(registry.version, VERSION);
        assert_eq!(registry.projects.len(), 0);
    }

    #[test]
    fn test_is_port_available() {
        // Test with a very high port that's likely available
        assert!(ProjectRegistry::is_port_available(65534));

        // We can't reliably test an unavailable port without potentially
        // interfering with other tests or services
    }
}
