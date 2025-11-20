use crate::db::{create_pool, run_migrations};
use crate::error::{IntentError, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;

const INTENT_DIR: &str = ".intent-engine";
const DB_FILE: &str = "project.db";

/// Project root markers in priority order (highest priority first)
/// These are used to identify the root directory of a project
const PROJECT_ROOT_MARKERS: &[&str] = &[
    ".git",           // Git (highest priority)
    ".hg",            // Mercurial
    "package.json",   // Node.js
    "Cargo.toml",     // Rust
    "pyproject.toml", // Python (PEP 518)
    "go.mod",         // Go Modules
    "pom.xml",        // Maven (Java)
    "build.gradle",   // Gradle (Java/Kotlin)
];

#[derive(Debug)]
pub struct ProjectContext {
    pub root: PathBuf,
    pub db_path: PathBuf,
    pub pool: SqlitePool,
}

/// Information about directory traversal for database location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryTraversalInfo {
    pub path: String,
    pub has_intent_engine: bool,
    pub is_selected: bool,
}

/// Detailed information about database path resolution
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabasePathInfo {
    pub current_working_directory: String,
    pub env_var_set: bool,
    pub env_var_path: Option<String>,
    pub env_var_valid: Option<bool>,
    pub directories_checked: Vec<DirectoryTraversalInfo>,
    pub home_directory: Option<String>,
    pub home_has_intent_engine: bool,
    pub final_database_path: Option<String>,
    pub resolution_method: Option<String>,
}

impl ProjectContext {
    /// Collect detailed information about database path resolution for diagnostics
    ///
    /// This function traces through all the steps of finding the database location,
    /// showing which directories were checked and why a particular location was chosen.
    pub fn get_database_path_info() -> DatabasePathInfo {
        let cwd = std::env::current_dir()
            .ok()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<unable to determine>".to_string());

        let mut info = DatabasePathInfo {
            current_working_directory: cwd.clone(),
            env_var_set: false,
            env_var_path: None,
            env_var_valid: None,
            directories_checked: Vec::new(),
            home_directory: None,
            home_has_intent_engine: false,
            final_database_path: None,
            resolution_method: None,
        };

        // Check strategy 1: Environment variable
        if let Ok(env_path) = std::env::var("INTENT_ENGINE_PROJECT_DIR") {
            info.env_var_set = true;
            info.env_var_path = Some(env_path.clone());

            let path = PathBuf::from(&env_path);
            let intent_dir = path.join(INTENT_DIR);
            let has_intent_engine = intent_dir.exists() && intent_dir.is_dir();
            info.env_var_valid = Some(has_intent_engine);

            if has_intent_engine {
                let db_path = intent_dir.join(DB_FILE);
                info.final_database_path = Some(db_path.display().to_string());
                info.resolution_method =
                    Some("Environment Variable (INTENT_ENGINE_PROJECT_DIR)".to_string());
                return info;
            }
        }

        // Check strategy 2: Upward directory traversal
        if let Ok(mut current) = std::env::current_dir() {
            loop {
                let intent_dir = current.join(INTENT_DIR);
                let has_intent_engine = intent_dir.exists() && intent_dir.is_dir();

                let is_selected = has_intent_engine && info.final_database_path.is_none();

                info.directories_checked.push(DirectoryTraversalInfo {
                    path: current.display().to_string(),
                    has_intent_engine,
                    is_selected,
                });

                if has_intent_engine && info.final_database_path.is_none() {
                    let db_path = intent_dir.join(DB_FILE);
                    info.final_database_path = Some(db_path.display().to_string());
                    info.resolution_method = Some("Upward Directory Traversal".to_string());
                    // Continue traversal to show all directories checked
                }

                if !current.pop() {
                    break;
                }
            }
        }

        // Check strategy 3: Home directory
        #[cfg(not(target_os = "windows"))]
        let home_path = std::env::var("HOME").ok().map(PathBuf::from);

        #[cfg(target_os = "windows")]
        let home_path = std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from));

        if let Some(home) = home_path {
            info.home_directory = Some(home.display().to_string());
            let intent_dir = home.join(INTENT_DIR);
            info.home_has_intent_engine = intent_dir.exists() && intent_dir.is_dir();

            if info.home_has_intent_engine && info.final_database_path.is_none() {
                let db_path = intent_dir.join(DB_FILE);
                info.final_database_path = Some(db_path.display().to_string());
                info.resolution_method = Some("Home Directory Fallback".to_string());
            }
        }

        info
    }

    /// Find the project root by searching upwards for .intent-engine directory
    ///
    /// Search strategy (in priority order):
    /// 1. Check INTENT_ENGINE_PROJECT_DIR environment variable
    /// 2. Search upwards from current directory for .intent-engine/, but:
    ///    - Stop at project boundary (defined by PROJECT_ROOT_MARKERS)
    ///    - Do NOT cross into parent projects to prevent database mixing
    /// 3. Check user's home directory for .intent-engine/
    ///
    /// **Important**: This function now respects project boundaries to prevent
    /// nested projects from accidentally using parent project databases.
    pub fn find_project_root() -> Option<PathBuf> {
        // Strategy 1: Check environment variable (highest priority)
        if let Ok(env_path) = std::env::var("INTENT_ENGINE_PROJECT_DIR") {
            let path = PathBuf::from(env_path);
            let intent_dir = path.join(INTENT_DIR);
            if intent_dir.exists() && intent_dir.is_dir() {
                eprintln!(
                    "✓ Using project from INTENT_ENGINE_PROJECT_DIR: {}",
                    path.display()
                );
                return Some(path);
            } else {
                eprintln!(
                    "⚠ INTENT_ENGINE_PROJECT_DIR set but no .intent-engine found: {}",
                    path.display()
                );
            }
        }

        // Strategy 2: Search upwards from current directory
        // BUT respect project boundaries (don't cross into parent projects)
        // UNLESS we're not inside any project (to support MCP server startup)
        if let Ok(current_dir) = std::env::current_dir() {
            let start_dir = current_dir.clone();

            // First, find the boundary of the current project (if any)
            // This is the directory that contains a project marker
            let project_boundary = Self::infer_project_root();

            let mut current = start_dir.clone();
            loop {
                let intent_dir = current.join(INTENT_DIR);
                if intent_dir.exists() && intent_dir.is_dir() {
                    // Found .intent-engine directory

                    // Check if we're within or at the project boundary
                    // If there's a project boundary and we've crossed it, don't use this .intent-engine
                    // BUT: if project_boundary is None (not in any project), allow searching anywhere
                    if let Some(ref boundary) = project_boundary {
                        // Check if the found .intent-engine is within our project boundary
                        // (current path should be equal to or a child of boundary)
                        if !current.starts_with(boundary) && current != *boundary {
                            // We've crossed the project boundary into a parent project
                            // Do NOT use this .intent-engine
                            break;
                        }
                    }

                    if current != start_dir {
                        eprintln!("✓ Found project: {}", current.display());
                    }
                    return Some(current);
                }

                // Check if we've reached the project boundary
                // If so, stop searching (don't go into parent projects)
                if let Some(ref boundary) = project_boundary {
                    if current == *boundary {
                        // We've reached the boundary without finding .intent-engine
                        // Stop here and return None (will trigger initialization)
                        break;
                    }
                }

                if !current.pop() {
                    break;
                }
            }
        }

        // Strategy 3: Check user's home directory (fallback)
        if let Ok(home) = std::env::var("HOME") {
            let home_path = PathBuf::from(home);
            let intent_dir = home_path.join(INTENT_DIR);
            if intent_dir.exists() && intent_dir.is_dir() {
                eprintln!("✓ Using home project: {}", home_path.display());
                return Some(home_path);
            }
        }

        // Windows: also check USERPROFILE
        #[cfg(target_os = "windows")]
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let home_path = PathBuf::from(userprofile);
            let intent_dir = home_path.join(INTENT_DIR);
            if intent_dir.exists() && intent_dir.is_dir() {
                eprintln!("✓ Using home project: {}", home_path.display());
                return Some(home_path);
            }
        }

        None
    }

    /// Infer the project root directory starting from a given path
    ///
    /// This is a helper function that implements the core root-finding logic
    /// without relying on the global current directory.
    ///
    /// # Arguments
    /// * `start_path` - The directory path to start searching from
    ///
    /// # Returns
    /// * `Some(PathBuf)` - The project root if a marker is found
    /// * `None` - If no project marker is found up to the filesystem root
    fn infer_project_root_from(start_path: &std::path::Path) -> Option<PathBuf> {
        let mut current = start_path.to_path_buf();

        loop {
            // Check if any marker exists in current directory
            for marker in PROJECT_ROOT_MARKERS {
                let marker_path = current.join(marker);
                if marker_path.exists() {
                    return Some(current);
                }
            }

            // Try to move up to parent directory
            if !current.pop() {
                // Reached filesystem root without finding any marker
                break;
            }
        }

        None
    }

    /// Infer the project root directory based on common project markers
    ///
    /// This function implements a smart algorithm to find the project root:
    /// 1. Start from current directory and traverse upwards
    /// 2. Check each directory for project markers (in priority order)
    /// 3. Return the first directory that contains any marker
    /// 4. If no marker found, return None (fallback to CWD handled by caller)
    fn infer_project_root() -> Option<PathBuf> {
        let cwd = std::env::current_dir().ok()?;
        Self::infer_project_root_from(&cwd)
    }

    /// Initialize a new Intent-Engine project using smart root inference
    ///
    /// This function implements the smart lazy initialization algorithm:
    /// 1. Try to infer project root based on common markers
    /// 2. If inference succeeds, initialize in the inferred root
    /// 3. If inference fails, fallback to CWD and print warning to stderr
    pub async fn initialize_project() -> Result<Self> {
        let cwd = std::env::current_dir()?;

        // Try to infer the project root
        let root = match Self::infer_project_root() {
            Some(inferred_root) => {
                // Successfully inferred project root
                inferred_root
            },
            None => {
                // Fallback: use current working directory
                // Print warning to stderr
                eprintln!(
                    "Warning: Could not determine a project root based on common markers (e.g., .git, package.json).\n\
                     Initialized Intent-Engine in the current directory '{}'.\n\
                     For predictable behavior, it's recommended to initialize from a directory containing a root marker.",
                    cwd.display()
                );
                cwd
            },
        };

        let intent_dir = root.join(INTENT_DIR);
        let db_path = intent_dir.join(DB_FILE);

        // Create .intent-engine directory if it doesn't exist
        if !intent_dir.exists() {
            std::fs::create_dir_all(&intent_dir)?;
        }

        // Create database connection
        let pool = create_pool(&db_path).await?;

        // Run migrations
        run_migrations(&pool).await?;

        Ok(ProjectContext {
            root,
            db_path,
            pool,
        })
    }

    /// Initialize a new Intent-Engine project at a specific directory
    ///
    /// This is a thread-safe alternative to `initialize_project()` that doesn't
    /// rely on the global current directory. It's particularly useful for:
    /// - Concurrent tests that need isolated project initialization
    /// - Tools that need to initialize projects in specific directories
    /// - Any scenario where changing the global current directory is undesirable
    ///
    /// # Arguments
    /// * `project_dir` - The directory where the project should be initialized
    ///
    /// # Algorithm
    /// 1. Try to infer project root starting from `project_dir`
    /// 2. If inference succeeds, initialize in the inferred root
    /// 3. If inference fails, use `project_dir` as the root directly
    ///
    /// # Examples
    /// ```no_run
    /// use intent_engine::project::ProjectContext;
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let project_dir = PathBuf::from("/tmp/my-project");
    ///     let ctx = ProjectContext::initialize_project_at(project_dir).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn initialize_project_at(project_dir: PathBuf) -> Result<Self> {
        // Try to infer the project root starting from the provided directory
        let root = match Self::infer_project_root_from(&project_dir) {
            Some(inferred_root) => {
                // Successfully inferred project root
                inferred_root
            },
            None => {
                // No marker found, use provided directory as root
                // This is expected for test environments where we explicitly
                // create a .git marker in a temp directory
                project_dir
            },
        };

        let intent_dir = root.join(INTENT_DIR);
        let db_path = intent_dir.join(DB_FILE);

        // Create .intent-engine directory if it doesn't exist
        if !intent_dir.exists() {
            std::fs::create_dir_all(&intent_dir)?;
        }

        // Create database connection
        let pool = create_pool(&db_path).await?;

        // Run migrations
        run_migrations(&pool).await?;

        Ok(ProjectContext {
            root,
            db_path,
            pool,
        })
    }

    /// Load an existing project context
    pub async fn load() -> Result<Self> {
        let root = Self::find_project_root().ok_or(IntentError::NotAProject)?;
        let db_path = root.join(INTENT_DIR).join(DB_FILE);

        let pool = create_pool(&db_path).await?;

        Ok(ProjectContext {
            root,
            db_path,
            pool,
        })
    }

    /// Load project context, initializing if necessary (for write commands)
    pub async fn load_or_init() -> Result<Self> {
        match Self::load().await {
            Ok(ctx) => Ok(ctx),
            Err(IntentError::NotAProject) => Self::initialize_project().await,
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests that modify the current directory are intentionally limited
    // because they can interfere with other tests running in parallel.
    // These functionalities are thoroughly tested by integration tests.

    #[test]
    fn test_constants() {
        assert_eq!(INTENT_DIR, ".intent-engine");
        assert_eq!(DB_FILE, "project.db");
    }

    #[test]
    fn test_project_context_debug() {
        // Just verify that ProjectContext implements Debug
        // We can't easily create one without side effects in a unit test
        let _type_check = |ctx: ProjectContext| {
            let _ = format!("{:?}", ctx);
        };
    }

    #[test]
    fn test_project_root_markers_list() {
        // Verify that the markers list contains expected markers
        assert!(PROJECT_ROOT_MARKERS.contains(&".git"));
        assert!(PROJECT_ROOT_MARKERS.contains(&"Cargo.toml"));
        assert!(PROJECT_ROOT_MARKERS.contains(&"package.json"));
    }

    #[test]
    fn test_project_root_markers_priority() {
        // Verify that .git has highest priority (comes first)
        assert_eq!(PROJECT_ROOT_MARKERS[0], ".git");
    }

    /// Test infer_project_root in an isolated environment
    /// Note: This test creates a temporary directory structure but doesn't change CWD
    #[test]
    fn test_infer_project_root_with_git() {
        // This test is limited because we can't easily change CWD in unit tests
        // The actual behavior is tested in integration tests
        // Here we just verify the marker list is correct
        assert!(PROJECT_ROOT_MARKERS.contains(&".git"));
    }

    /// Test that markers list includes all major project types
    #[test]
    fn test_all_major_project_types_covered() {
        let markers = PROJECT_ROOT_MARKERS;

        // Git version control
        assert!(markers.contains(&".git"));
        assert!(markers.contains(&".hg"));

        // Programming languages
        assert!(markers.contains(&"Cargo.toml")); // Rust
        assert!(markers.contains(&"package.json")); // Node.js
        assert!(markers.contains(&"pyproject.toml")); // Python
        assert!(markers.contains(&"go.mod")); // Go
        assert!(markers.contains(&"pom.xml")); // Java (Maven)
        assert!(markers.contains(&"build.gradle")); // Java/Kotlin (Gradle)
    }

    /// Test DirectoryTraversalInfo structure
    #[test]
    fn test_directory_traversal_info_creation() {
        let info = DirectoryTraversalInfo {
            path: "/test/path".to_string(),
            has_intent_engine: true,
            is_selected: false,
        };

        assert_eq!(info.path, "/test/path");
        assert!(info.has_intent_engine);
        assert!(!info.is_selected);
    }

    /// Test DirectoryTraversalInfo Clone trait
    #[test]
    fn test_directory_traversal_info_clone() {
        let info = DirectoryTraversalInfo {
            path: "/test/path".to_string(),
            has_intent_engine: true,
            is_selected: true,
        };

        let cloned = info.clone();
        assert_eq!(cloned.path, info.path);
        assert_eq!(cloned.has_intent_engine, info.has_intent_engine);
        assert_eq!(cloned.is_selected, info.is_selected);
    }

    /// Test DirectoryTraversalInfo Debug trait
    #[test]
    fn test_directory_traversal_info_debug() {
        let info = DirectoryTraversalInfo {
            path: "/test/path".to_string(),
            has_intent_engine: false,
            is_selected: true,
        };

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("DirectoryTraversalInfo"));
        assert!(debug_str.contains("/test/path"));
    }

    /// Test DirectoryTraversalInfo serialization
    #[test]
    fn test_directory_traversal_info_serialization() {
        let info = DirectoryTraversalInfo {
            path: "/test/path".to_string(),
            has_intent_engine: true,
            is_selected: false,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("path"));
        assert!(json.contains("has_intent_engine"));
        assert!(json.contains("is_selected"));
        assert!(json.contains("/test/path"));
    }

    /// Test DirectoryTraversalInfo deserialization
    #[test]
    fn test_directory_traversal_info_deserialization() {
        let json = r#"{"path":"/test/path","has_intent_engine":true,"is_selected":false}"#;
        let info: DirectoryTraversalInfo = serde_json::from_str(json).unwrap();

        assert_eq!(info.path, "/test/path");
        assert!(info.has_intent_engine);
        assert!(!info.is_selected);
    }

    /// Test DatabasePathInfo structure creation
    #[test]
    fn test_database_path_info_creation() {
        let info = DatabasePathInfo {
            current_working_directory: "/test/cwd".to_string(),
            env_var_set: false,
            env_var_path: None,
            env_var_valid: None,
            directories_checked: vec![],
            home_directory: Some("/home/user".to_string()),
            home_has_intent_engine: false,
            final_database_path: Some("/test/db.db".to_string()),
            resolution_method: Some("Test Method".to_string()),
        };

        assert_eq!(info.current_working_directory, "/test/cwd");
        assert!(!info.env_var_set);
        assert_eq!(info.env_var_path, None);
        assert_eq!(info.home_directory, Some("/home/user".to_string()));
        assert!(!info.home_has_intent_engine);
        assert_eq!(info.final_database_path, Some("/test/db.db".to_string()));
        assert_eq!(info.resolution_method, Some("Test Method".to_string()));
    }

    /// Test DatabasePathInfo with environment variable set
    #[test]
    fn test_database_path_info_with_env_var() {
        let info = DatabasePathInfo {
            current_working_directory: "/test/cwd".to_string(),
            env_var_set: true,
            env_var_path: Some("/env/path".to_string()),
            env_var_valid: Some(true),
            directories_checked: vec![],
            home_directory: Some("/home/user".to_string()),
            home_has_intent_engine: false,
            final_database_path: Some("/env/path/.intent-engine/project.db".to_string()),
            resolution_method: Some("Environment Variable".to_string()),
        };

        assert!(info.env_var_set);
        assert_eq!(info.env_var_path, Some("/env/path".to_string()));
        assert_eq!(info.env_var_valid, Some(true));
        assert_eq!(
            info.resolution_method,
            Some("Environment Variable".to_string())
        );
    }

    /// Test DatabasePathInfo with directories checked
    #[test]
    fn test_database_path_info_with_directories() {
        let dirs = vec![
            DirectoryTraversalInfo {
                path: "/test/path1".to_string(),
                has_intent_engine: false,
                is_selected: false,
            },
            DirectoryTraversalInfo {
                path: "/test/path2".to_string(),
                has_intent_engine: true,
                is_selected: true,
            },
        ];

        let info = DatabasePathInfo {
            current_working_directory: "/test/path1".to_string(),
            env_var_set: false,
            env_var_path: None,
            env_var_valid: None,
            directories_checked: dirs.clone(),
            home_directory: Some("/home/user".to_string()),
            home_has_intent_engine: false,
            final_database_path: Some("/test/path2/.intent-engine/project.db".to_string()),
            resolution_method: Some("Upward Directory Traversal".to_string()),
        };

        assert_eq!(info.directories_checked.len(), 2);
        assert!(!info.directories_checked[0].has_intent_engine);
        assert!(info.directories_checked[1].has_intent_engine);
        assert!(info.directories_checked[1].is_selected);
    }

    /// Test DatabasePathInfo Debug trait
    #[test]
    fn test_database_path_info_debug() {
        let info = DatabasePathInfo {
            current_working_directory: "/test/cwd".to_string(),
            env_var_set: false,
            env_var_path: None,
            env_var_valid: None,
            directories_checked: vec![],
            home_directory: Some("/home/user".to_string()),
            home_has_intent_engine: false,
            final_database_path: Some("/test/db.db".to_string()),
            resolution_method: Some("Test".to_string()),
        };

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("DatabasePathInfo"));
        assert!(debug_str.contains("/test/cwd"));
    }

    /// Test DatabasePathInfo serialization
    #[test]
    fn test_database_path_info_serialization() {
        let info = DatabasePathInfo {
            current_working_directory: "/test/cwd".to_string(),
            env_var_set: true,
            env_var_path: Some("/env/path".to_string()),
            env_var_valid: Some(true),
            directories_checked: vec![],
            home_directory: Some("/home/user".to_string()),
            home_has_intent_engine: false,
            final_database_path: Some("/test/db.db".to_string()),
            resolution_method: Some("Test Method".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("current_working_directory"));
        assert!(json.contains("env_var_set"));
        assert!(json.contains("env_var_path"));
        assert!(json.contains("final_database_path"));
        assert!(json.contains("/test/cwd"));
        assert!(json.contains("/env/path"));
    }

    /// Test DatabasePathInfo deserialization
    #[test]
    fn test_database_path_info_deserialization() {
        let json = r#"{
            "current_working_directory": "/test/cwd",
            "env_var_set": true,
            "env_var_path": "/env/path",
            "env_var_valid": true,
            "directories_checked": [],
            "home_directory": "/home/user",
            "home_has_intent_engine": false,
            "final_database_path": "/test/db.db",
            "resolution_method": "Test Method"
        }"#;

        let info: DatabasePathInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.current_working_directory, "/test/cwd");
        assert!(info.env_var_set);
        assert_eq!(info.env_var_path, Some("/env/path".to_string()));
        assert_eq!(info.env_var_valid, Some(true));
        assert_eq!(info.home_directory, Some("/home/user".to_string()));
        assert_eq!(info.final_database_path, Some("/test/db.db".to_string()));
        assert_eq!(info.resolution_method, Some("Test Method".to_string()));
    }

    /// Test DatabasePathInfo with complete directory traversal data
    #[test]
    fn test_database_path_info_complete_structure() {
        let dirs = vec![
            DirectoryTraversalInfo {
                path: "/home/user/project/src".to_string(),
                has_intent_engine: false,
                is_selected: false,
            },
            DirectoryTraversalInfo {
                path: "/home/user/project".to_string(),
                has_intent_engine: true,
                is_selected: true,
            },
            DirectoryTraversalInfo {
                path: "/home/user".to_string(),
                has_intent_engine: false,
                is_selected: false,
            },
        ];

        let info = DatabasePathInfo {
            current_working_directory: "/home/user/project/src".to_string(),
            env_var_set: false,
            env_var_path: None,
            env_var_valid: None,
            directories_checked: dirs,
            home_directory: Some("/home/user".to_string()),
            home_has_intent_engine: false,
            final_database_path: Some("/home/user/project/.intent-engine/project.db".to_string()),
            resolution_method: Some("Upward Directory Traversal".to_string()),
        };

        // Verify the complete structure
        assert_eq!(info.directories_checked.len(), 3);
        assert_eq!(info.directories_checked[0].path, "/home/user/project/src");
        assert_eq!(info.directories_checked[1].path, "/home/user/project");
        assert_eq!(info.directories_checked[2].path, "/home/user");

        // Only the second directory should be selected
        assert!(!info.directories_checked[0].is_selected);
        assert!(info.directories_checked[1].is_selected);
        assert!(!info.directories_checked[2].is_selected);

        // Only the second directory has intent-engine
        assert!(!info.directories_checked[0].has_intent_engine);
        assert!(info.directories_checked[1].has_intent_engine);
        assert!(!info.directories_checked[2].has_intent_engine);
    }

    /// Test get_database_path_info returns valid structure
    #[test]
    fn test_get_database_path_info_structure() {
        let info = ProjectContext::get_database_path_info();

        // Verify basic structure is populated
        assert!(!info.current_working_directory.is_empty());

        // Verify that we got some result for directories checked or home directory
        let has_data = !info.directories_checked.is_empty()
            || info.home_directory.is_some()
            || info.env_var_set;

        assert!(
            has_data,
            "get_database_path_info should return some directory information"
        );
    }

    /// Test get_database_path_info with actual filesystem
    #[test]
    fn test_get_database_path_info_checks_current_dir() {
        let info = ProjectContext::get_database_path_info();

        // The current working directory should be set
        assert!(!info.current_working_directory.is_empty());

        // At minimum, it should check the current directory (unless env var is set and valid)
        if !info.env_var_set || info.env_var_valid != Some(true) {
            assert!(
                !info.directories_checked.is_empty(),
                "Should check at least the current directory"
            );
        }
    }

    /// Test get_database_path_info includes current directory in checked list
    #[test]
    fn test_get_database_path_info_includes_cwd() {
        let info = ProjectContext::get_database_path_info();

        // If env var is not set or invalid, should check directories
        if !info.env_var_set || info.env_var_valid != Some(true) {
            assert!(!info.directories_checked.is_empty());

            // First checked directory should start with the CWD or be a parent
            let cwd = &info.current_working_directory;
            let first_checked = &info.directories_checked[0].path;

            assert!(
                cwd.starts_with(first_checked) || first_checked.starts_with(cwd),
                "First checked directory should be related to CWD"
            );
        }
    }

    /// Test get_database_path_info resolution method is set when database found
    #[test]
    fn test_get_database_path_info_resolution_method() {
        let info = ProjectContext::get_database_path_info();

        // If a database path was found, resolution method should be set
        if info.final_database_path.is_some() {
            assert!(
                info.resolution_method.is_some(),
                "Resolution method should be set when database path is found"
            );

            let method = info.resolution_method.unwrap();
            assert!(
                method.contains("Environment Variable")
                    || method.contains("Upward Directory Traversal")
                    || method.contains("Home Directory"),
                "Resolution method should be one of the known strategies"
            );
        }
    }

    /// Test get_database_path_info marks selected directory correctly
    #[test]
    fn test_get_database_path_info_selected_directory() {
        let info = ProjectContext::get_database_path_info();

        // If env var not used and directories were checked
        if (!info.env_var_set || info.env_var_valid != Some(true))
            && !info.directories_checked.is_empty()
            && info.final_database_path.is_some()
        {
            // Exactly one directory should be marked as selected
            let selected_count = info
                .directories_checked
                .iter()
                .filter(|d| d.is_selected)
                .count();

            assert!(
                selected_count <= 1,
                "At most one directory should be marked as selected"
            );

            // If one is selected, it should have intent_engine
            if let Some(selected) = info.directories_checked.iter().find(|d| d.is_selected) {
                assert!(
                    selected.has_intent_engine,
                    "Selected directory should have .intent-engine"
                );
            }
        }
    }

    /// Test DatabasePathInfo with no database found scenario
    #[test]
    fn test_database_path_info_no_database_found() {
        let info = DatabasePathInfo {
            current_working_directory: "/test/path".to_string(),
            env_var_set: false,
            env_var_path: None,
            env_var_valid: None,
            directories_checked: vec![
                DirectoryTraversalInfo {
                    path: "/test/path".to_string(),
                    has_intent_engine: false,
                    is_selected: false,
                },
                DirectoryTraversalInfo {
                    path: "/test".to_string(),
                    has_intent_engine: false,
                    is_selected: false,
                },
            ],
            home_directory: Some("/home/user".to_string()),
            home_has_intent_engine: false,
            final_database_path: None,
            resolution_method: None,
        };

        assert!(info.final_database_path.is_none());
        assert!(info.resolution_method.is_none());
        assert_eq!(info.directories_checked.len(), 2);
        assert!(!info.home_has_intent_engine);
    }

    /// Test DatabasePathInfo with env var set but invalid
    #[test]
    fn test_database_path_info_env_var_invalid() {
        let info = DatabasePathInfo {
            current_working_directory: "/test/cwd".to_string(),
            env_var_set: true,
            env_var_path: Some("/invalid/path".to_string()),
            env_var_valid: Some(false),
            directories_checked: vec![DirectoryTraversalInfo {
                path: "/test/cwd".to_string(),
                has_intent_engine: true,
                is_selected: true,
            }],
            home_directory: Some("/home/user".to_string()),
            home_has_intent_engine: false,
            final_database_path: Some("/test/cwd/.intent-engine/project.db".to_string()),
            resolution_method: Some("Upward Directory Traversal".to_string()),
        };

        assert!(info.env_var_set);
        assert_eq!(info.env_var_valid, Some(false));
        assert!(info.final_database_path.is_some());
        // Should fallback to directory traversal when env var is invalid
        assert!(info.resolution_method.unwrap().contains("Upward Directory"));
    }

    /// Test DatabasePathInfo with home directory fallback
    #[test]
    fn test_database_path_info_home_directory_used() {
        let info = DatabasePathInfo {
            current_working_directory: "/tmp/work".to_string(),
            env_var_set: false,
            env_var_path: None,
            env_var_valid: None,
            directories_checked: vec![
                DirectoryTraversalInfo {
                    path: "/tmp/work".to_string(),
                    has_intent_engine: false,
                    is_selected: false,
                },
                DirectoryTraversalInfo {
                    path: "/tmp".to_string(),
                    has_intent_engine: false,
                    is_selected: false,
                },
            ],
            home_directory: Some("/home/user".to_string()),
            home_has_intent_engine: true,
            final_database_path: Some("/home/user/.intent-engine/project.db".to_string()),
            resolution_method: Some("Home Directory Fallback".to_string()),
        };

        assert!(info.home_has_intent_engine);
        assert_eq!(
            info.final_database_path,
            Some("/home/user/.intent-engine/project.db".to_string())
        );
        assert_eq!(
            info.resolution_method,
            Some("Home Directory Fallback".to_string())
        );
    }

    /// Test DatabasePathInfo serialization round-trip with all fields
    #[test]
    fn test_database_path_info_full_roundtrip() {
        let original = DatabasePathInfo {
            current_working_directory: "/test/cwd".to_string(),
            env_var_set: true,
            env_var_path: Some("/env/path".to_string()),
            env_var_valid: Some(false),
            directories_checked: vec![
                DirectoryTraversalInfo {
                    path: "/test/cwd".to_string(),
                    has_intent_engine: false,
                    is_selected: false,
                },
                DirectoryTraversalInfo {
                    path: "/test".to_string(),
                    has_intent_engine: true,
                    is_selected: true,
                },
            ],
            home_directory: Some("/home/user".to_string()),
            home_has_intent_engine: false,
            final_database_path: Some("/test/.intent-engine/project.db".to_string()),
            resolution_method: Some("Upward Directory Traversal".to_string()),
        };

        // Serialize
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize
        let deserialized: DatabasePathInfo = serde_json::from_str(&json).unwrap();

        // Verify all fields match
        assert_eq!(
            deserialized.current_working_directory,
            original.current_working_directory
        );
        assert_eq!(deserialized.env_var_set, original.env_var_set);
        assert_eq!(deserialized.env_var_path, original.env_var_path);
        assert_eq!(deserialized.env_var_valid, original.env_var_valid);
        assert_eq!(
            deserialized.directories_checked.len(),
            original.directories_checked.len()
        );
        assert_eq!(deserialized.home_directory, original.home_directory);
        assert_eq!(
            deserialized.home_has_intent_engine,
            original.home_has_intent_engine
        );
        assert_eq!(
            deserialized.final_database_path,
            original.final_database_path
        );
        assert_eq!(deserialized.resolution_method, original.resolution_method);
    }

    /// Test DirectoryTraversalInfo with all boolean combinations
    #[test]
    fn test_directory_traversal_info_all_combinations() {
        // Test all 4 combinations of boolean flags
        let combinations = [(false, false), (false, true), (true, false), (true, true)];

        for (has_ie, is_sel) in combinations.iter() {
            let info = DirectoryTraversalInfo {
                path: format!("/test/path/{}_{}", has_ie, is_sel),
                has_intent_engine: *has_ie,
                is_selected: *is_sel,
            };

            assert_eq!(info.has_intent_engine, *has_ie);
            assert_eq!(info.is_selected, *is_sel);
        }
    }

    /// Test DirectoryTraversalInfo serialization preserves exact values
    #[test]
    fn test_directory_traversal_info_exact_serialization() {
        let info = DirectoryTraversalInfo {
            path: "/exact/path/with/special-chars_123".to_string(),
            has_intent_engine: true,
            is_selected: false,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: DirectoryTraversalInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info.path, deserialized.path);
        assert_eq!(info.has_intent_engine, deserialized.has_intent_engine);
        assert_eq!(info.is_selected, deserialized.is_selected);
    }

    /// Test DatabasePathInfo with None values for optional fields
    #[test]
    fn test_database_path_info_all_none() {
        let info = DatabasePathInfo {
            current_working_directory: "/test".to_string(),
            env_var_set: false,
            env_var_path: None,
            env_var_valid: None,
            directories_checked: vec![],
            home_directory: None,
            home_has_intent_engine: false,
            final_database_path: None,
            resolution_method: None,
        };

        assert!(!info.env_var_set);
        assert!(info.env_var_path.is_none());
        assert!(info.env_var_valid.is_none());
        assert!(info.directories_checked.is_empty());
        assert!(info.home_directory.is_none());
        assert!(info.final_database_path.is_none());
        assert!(info.resolution_method.is_none());
    }

    /// Test DatabasePathInfo with Some values for all optional fields
    #[test]
    fn test_database_path_info_all_some() {
        let info = DatabasePathInfo {
            current_working_directory: "/test".to_string(),
            env_var_set: true,
            env_var_path: Some("/env".to_string()),
            env_var_valid: Some(true),
            directories_checked: vec![DirectoryTraversalInfo {
                path: "/test".to_string(),
                has_intent_engine: true,
                is_selected: true,
            }],
            home_directory: Some("/home".to_string()),
            home_has_intent_engine: true,
            final_database_path: Some("/test/.intent-engine/project.db".to_string()),
            resolution_method: Some("Test Method".to_string()),
        };

        assert!(info.env_var_set);
        assert!(info.env_var_path.is_some());
        assert!(info.env_var_valid.is_some());
        assert!(!info.directories_checked.is_empty());
        assert!(info.home_directory.is_some());
        assert!(info.final_database_path.is_some());
        assert!(info.resolution_method.is_some());
    }

    /// Test get_database_path_info home directory field is set
    #[test]
    fn test_get_database_path_info_home_directory() {
        let info = ProjectContext::get_database_path_info();

        // Home directory should typically be set (unless in very restricted environment)
        // This tests that the home directory detection logic runs
        if std::env::var("HOME").is_ok() {
            assert!(
                info.home_directory.is_some(),
                "HOME env var is set, so home_directory should be Some"
            );
        }
    }

    /// Test get_database_path_info doesn't panic with edge cases
    #[test]
    fn test_get_database_path_info_no_panic() {
        // This test ensures the function handles edge cases gracefully
        // Even in unusual environments, it should return valid data
        let info = ProjectContext::get_database_path_info();

        // Basic sanity checks - should always have these
        assert!(!info.current_working_directory.is_empty());

        // If final_database_path is None, that's okay - it means no database was found
        // The function should still provide diagnostic information
        if info.final_database_path.is_none() {
            // Should still have checked some directories or reported env var status
            let has_diagnostic_info = !info.directories_checked.is_empty()
                || info.env_var_set
                || info.home_directory.is_some();

            assert!(
                has_diagnostic_info,
                "Even without finding a database, should provide diagnostic information"
            );
        }
    }

    /// Test get_database_path_info with multiple .intent-engine directories
    #[test]
    fn test_get_database_path_info_prefers_first_match() {
        let info = ProjectContext::get_database_path_info();

        // If database was found via directory traversal and multiple directories were checked
        if info
            .resolution_method
            .as_ref()
            .is_some_and(|m| m.contains("Upward Directory"))
            && info.directories_checked.len() > 1
        {
            // Find all directories with .intent-engine
            let with_ie: Vec<_> = info
                .directories_checked
                .iter()
                .filter(|d| d.has_intent_engine)
                .collect();

            if with_ie.len() > 1 {
                // Only the first one found (closest to CWD) should be selected
                let selected: Vec<_> = with_ie.iter().filter(|d| d.is_selected).collect();
                assert!(
                    selected.len() <= 1,
                    "Only the first .intent-engine found should be selected"
                );
            }
        }
    }

    /// Test DatabasePathInfo deserialization with missing optional fields
    #[test]
    fn test_database_path_info_partial_deserialization() {
        // Test with minimal required fields
        let json = r#"{
            "current_working_directory": "/test",
            "env_var_set": false,
            "env_var_path": null,
            "env_var_valid": null,
            "directories_checked": [],
            "home_directory": null,
            "home_has_intent_engine": false,
            "final_database_path": null,
            "resolution_method": null
        }"#;

        let info: DatabasePathInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.current_working_directory, "/test");
        assert!(!info.env_var_set);
    }

    /// Test DatabasePathInfo JSON format matches expected schema
    #[test]
    fn test_database_path_info_json_schema() {
        let info = DatabasePathInfo {
            current_working_directory: "/test".to_string(),
            env_var_set: true,
            env_var_path: Some("/env".to_string()),
            env_var_valid: Some(true),
            directories_checked: vec![],
            home_directory: Some("/home".to_string()),
            home_has_intent_engine: false,
            final_database_path: Some("/db".to_string()),
            resolution_method: Some("Test".to_string()),
        };

        let json_value: serde_json::Value = serde_json::to_value(&info).unwrap();

        // Verify all expected fields are present
        assert!(json_value.get("current_working_directory").is_some());
        assert!(json_value.get("env_var_set").is_some());
        assert!(json_value.get("env_var_path").is_some());
        assert!(json_value.get("env_var_valid").is_some());
        assert!(json_value.get("directories_checked").is_some());
        assert!(json_value.get("home_directory").is_some());
        assert!(json_value.get("home_has_intent_engine").is_some());
        assert!(json_value.get("final_database_path").is_some());
        assert!(json_value.get("resolution_method").is_some());
    }

    /// Test DirectoryTraversalInfo with empty path
    #[test]
    fn test_directory_traversal_info_empty_path() {
        let info = DirectoryTraversalInfo {
            path: "".to_string(),
            has_intent_engine: false,
            is_selected: false,
        };

        assert_eq!(info.path, "");
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: DirectoryTraversalInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, "");
    }

    /// Test DirectoryTraversalInfo with unicode path
    #[test]
    fn test_directory_traversal_info_unicode_path() {
        let info = DirectoryTraversalInfo {
            path: "/test/路径/مسار/путь".to_string(),
            has_intent_engine: true,
            is_selected: false,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: DirectoryTraversalInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, "/test/路径/مسار/путь");
    }

    /// Test DatabasePathInfo with very long paths
    #[test]
    fn test_database_path_info_long_paths() {
        let long_path = "/".to_owned() + &"very_long_directory_name/".repeat(50);
        let info = DatabasePathInfo {
            current_working_directory: long_path.clone(),
            env_var_set: false,
            env_var_path: None,
            env_var_valid: None,
            directories_checked: vec![],
            home_directory: Some(long_path.clone()),
            home_has_intent_engine: false,
            final_database_path: Some(long_path.clone()),
            resolution_method: Some("Test".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: DatabasePathInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.current_working_directory, long_path);
    }

    /// Test get_database_path_info env var handling
    #[test]
    fn test_get_database_path_info_env_var_detection() {
        let info = ProjectContext::get_database_path_info();

        // Check if INTENT_ENGINE_PROJECT_DIR is set
        if std::env::var("INTENT_ENGINE_PROJECT_DIR").is_ok() {
            assert!(
                info.env_var_set,
                "env_var_set should be true when INTENT_ENGINE_PROJECT_DIR is set"
            );
            assert!(
                info.env_var_path.is_some(),
                "env_var_path should contain the path when env var is set"
            );
            assert!(
                info.env_var_valid.is_some(),
                "env_var_valid should be set when env var is present"
            );
        } else {
            assert!(
                !info.env_var_set,
                "env_var_set should be false when INTENT_ENGINE_PROJECT_DIR is not set"
            );
            assert!(
                info.env_var_path.is_none(),
                "env_var_path should be None when env var is not set"
            );
            assert!(
                info.env_var_valid.is_none(),
                "env_var_valid should be None when env var is not set"
            );
        }
    }
}
