//! Shared test helpers for library-based integration tests
//!
//! This module provides common utilities for tests that call library functions
//! directly rather than using CLI commands.

use intent_engine::db::{create_pool, run_migrations};
use sqlx::SqlitePool;
use tempfile::TempDir;

/// Test context with database pool and temp directory
pub struct TestDb {
    pub pool: SqlitePool,
    pub _temp_dir: TempDir,
}

impl TestDb {
    /// Create a new test database with migrations run
    pub async fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let intent_dir = temp_dir.path().join(".intent-engine");
        std::fs::create_dir_all(&intent_dir).unwrap();
        let db_path = intent_dir.join("project.db");

        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        Self {
            pool,
            _temp_dir: temp_dir,
        }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
