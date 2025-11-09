use crate::db::{create_pool, run_migrations};
use crate::error::{IntentError, Result};
use sqlx::SqlitePool;
use std::path::PathBuf;

const INTENT_DIR: &str = ".intent-engine";
const DB_FILE: &str = "project.db";

#[derive(Debug)]
pub struct ProjectContext {
    pub root: PathBuf,
    pub db_path: PathBuf,
    pub pool: SqlitePool,
}

impl ProjectContext {
    /// Find the project root by searching upwards for .intent-engine directory
    pub fn find_project_root() -> Option<PathBuf> {
        let mut current = std::env::current_dir().ok()?;

        loop {
            let intent_dir = current.join(INTENT_DIR);
            if intent_dir.exists() && intent_dir.is_dir() {
                return Some(current);
            }

            if !current.pop() {
                break;
            }
        }

        None
    }

    /// Initialize a new Intent-Engine project in the current directory
    pub async fn initialize_project() -> Result<Self> {
        let root = std::env::current_dir()?;
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
}
