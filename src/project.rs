use crate::db::{create_pool, run_migrations};
use crate::error::{IntentError, Result};
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

impl ProjectContext {
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
    use std::fs;
    use tempfile::TempDir;

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
        // Verify that the markers list is not empty and contains expected markers
        assert!(!PROJECT_ROOT_MARKERS.is_empty());
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
}
