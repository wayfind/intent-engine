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

        // Allocate port (always DEFAULT_PORT)
        let port1 = registry.allocate_port().unwrap();
        assert_eq!(port1, DEFAULT_PORT);

        // Register a project with that port
        registry.register(RegisteredProject {
            path: PathBuf::from("/test/project1"),
            name: "project1".to_string(),
            port: port1,
            pid: None,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project1/.intent-engine/intents.db"),
            mcp_connected: false,
            mcp_last_seen: None,
            mcp_agent: None,
        });

        // Second allocation will succeed if port is not actually in use
        // (Test can't bind to port in unit test environment)
        let port2 = registry.allocate_port().unwrap();
        assert_eq!(port2, DEFAULT_PORT);
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

        // Always allocates DEFAULT_PORT
        let port = registry.allocate_port().unwrap();
        assert_eq!(port, DEFAULT_PORT);
    }
}
