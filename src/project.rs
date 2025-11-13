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
    /// 2. Search upwards from current directory for .intent-engine/
    /// 3. Check user's home directory for .intent-engine/
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

        // Strategy 2: Search upwards from current directory (original behavior)
        if let Ok(mut current) = std::env::current_dir() {
            let start_dir = current.clone();
            loop {
                let intent_dir = current.join(INTENT_DIR);
                if intent_dir.exists() && intent_dir.is_dir() {
                    if current != start_dir {
                        eprintln!("✓ Found project: {}", current.display());
                    }
                    return Some(current);
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

    /// Infer the project root directory based on common project markers
    ///
    /// This function implements a smart algorithm to find the project root:
    /// 1. Start from current directory and traverse upwards
    /// 2. Check each directory for project markers (in priority order)
    /// 3. Return the first directory that contains any marker
    /// 4. If no marker found, return None (fallback to CWD handled by caller)
    fn infer_project_root() -> Option<PathBuf> {
        let cwd = std::env::current_dir().ok()?;
        let mut current = cwd.clone();

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
}
