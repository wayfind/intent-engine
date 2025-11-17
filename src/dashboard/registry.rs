use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const REGISTRY_FILE: &str = ".intent-engine/projects.json";
const MIN_PORT: u16 = 3030;
const MAX_PORT: u16 = 3099;
const VERSION: &str = "1.0";

/// Global project registry for managing multiple Dashboard instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRegistry {
    pub version: String,
    pub projects: Vec<RegisteredProject>,
    pub next_port: u16,
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
}

impl ProjectRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            version: VERSION.to_string(),
            projects: Vec::new(),
            next_port: MIN_PORT,
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

    /// Allocate a new port, checking for conflicts
    pub fn allocate_port(&mut self) -> Result<u16> {
        // Try next_port first
        let mut port = self.next_port;
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 70; // Total available ports

        while attempts < MAX_ATTEMPTS {
            if port > MAX_PORT {
                port = MIN_PORT;
            }

            // Check if port is already in use
            if !self.projects.iter().any(|p| p.port == port) {
                // Check if port is actually available on the system
                if Self::is_port_available(port) {
                    self.next_port = if port == MAX_PORT { MIN_PORT } else { port + 1 };
                    return Ok(port);
                }
            }

            port += 1;
            attempts += 1;
        }

        anyhow::bail!("No available ports in range {}-{}", MIN_PORT, MAX_PORT)
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
        assert_eq!(registry.next_port, MIN_PORT);
    }

    #[test]
    fn test_allocate_port() {
        let mut registry = ProjectRegistry::new();

        // Allocate first port
        let port1 = registry.allocate_port().unwrap();
        assert_eq!(port1, MIN_PORT);
        assert_eq!(registry.next_port, MIN_PORT + 1);

        // Register a project with that port
        registry.register(RegisteredProject {
            path: PathBuf::from("/test/project1"),
            name: "project1".to_string(),
            port: port1,
            pid: None,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project1/.intent-engine/intents.db"),
        });

        // Allocate second port
        let port2 = registry.allocate_port().unwrap();
        assert_eq!(port2, MIN_PORT + 1);
    }

    #[test]
    fn test_register_and_find() {
        let mut registry = ProjectRegistry::new();

        let project = RegisteredProject {
            path: PathBuf::from("/test/project"),
            name: "test-project".to_string(),
            port: 3030,
            pid: Some(12345),
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
        };

        registry.register(project.clone());
        assert_eq!(registry.projects.len(), 1);

        // Find by path
        let found = registry.find_by_path(&PathBuf::from("/test/project"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test-project");

        // Find by port
        let found_by_port = registry.find_by_port(3030);
        assert!(found_by_port.is_some());
        assert_eq!(found_by_port.unwrap().name, "test-project");
    }

    #[test]
    fn test_unregister() {
        let mut registry = ProjectRegistry::new();

        let project = RegisteredProject {
            path: PathBuf::from("/test/project"),
            name: "test-project".to_string(),
            port: 3030,
            pid: None,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
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
            port: 3030,
            pid: None,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
        };

        let project2 = RegisteredProject {
            path: PathBuf::from("/test/project"),
            name: "project-v2".to_string(),
            port: 3031,
            pid: None,
            started_at: "2025-01-01T01:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
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
            port: 3030,
            pid: Some(12345),
            started_at: "2025-01-01T00:00:00Z".to_string(),
            db_path: PathBuf::from("/test/project/.intent-engine/intents.db"),
        };

        registry.register(project);

        // Test serialization
        let json = serde_json::to_string_pretty(&registry).unwrap();
        assert!(json.contains("test-project"));
        assert!(json.contains("3030"));

        // Test deserialization
        let loaded: ProjectRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.projects[0].name, "test-project");
        assert_eq!(loaded.projects[0].port, 3030);
    }

    #[test]
    fn test_port_wraparound() {
        let mut registry = ProjectRegistry::new();
        registry.next_port = MAX_PORT;

        // Allocate port at max
        let port = registry.allocate_port().unwrap();
        assert_eq!(port, MAX_PORT);

        // Next allocation should wrap to MIN_PORT
        assert_eq!(registry.next_port, MIN_PORT);
    }
}
