pub mod models;

use crate::error::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;

pub async fn create_pool(db_path: &Path) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_millis(5000));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    // Enable FTS5
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(pool)
        .await?;

    // Create tasks table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            parent_id INTEGER,
            name TEXT NOT NULL,
            spec TEXT,
            status TEXT NOT NULL DEFAULT 'todo',
            complexity INTEGER,
            priority INTEGER DEFAULT 0,
            first_todo_at DATETIME,
            first_doing_at DATETIME,
            first_done_at DATETIME,
            active_form TEXT,
            owner TEXT NOT NULL DEFAULT 'human',
            metadata TEXT DEFAULT '{}',
            FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE,
            CHECK (status IN ('todo', 'doing', 'done')),
            CHECK (owner IS NOT NULL AND owner != '')
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Add active_form column if it doesn't exist (migration for existing databases)
    // This column stores the present progressive form of task description for UI display
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN active_form TEXT")
        .execute(pool)
        .await; // Ignore error if column already exists

    // Create FTS5 virtual table for tasks with trigram tokenizer for better CJK support
    // For existing databases, we need to drop and recreate if tokenizer changed
    let _ = sqlx::query("DROP TABLE IF EXISTS tasks_fts")
        .execute(pool)
        .await; // Ignore error if table doesn't exist

    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE tasks_fts USING fts5(
            name,
            spec,
            content=tasks,
            content_rowid=id,
            tokenize='trigram'
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create triggers to keep FTS in sync
    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS tasks_ai AFTER INSERT ON tasks BEGIN
            INSERT INTO tasks_fts(rowid, name, spec) VALUES (new.id, new.name, new.spec);
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS tasks_ad AFTER DELETE ON tasks BEGIN
            DELETE FROM tasks_fts WHERE rowid = old.id;
        END
        "#,
    )
    .execute(pool)
    .await?;

    // Recreate trigger with correct FTS5 syntax (drop and create for migration from buggy version)
    // Note: We always drop first because SQLite doesn't support CREATE OR REPLACE TRIGGER,
    // and we need to update existing databases that have the buggy trigger.
    let _ = sqlx::query("DROP TRIGGER IF EXISTS tasks_au")
        .execute(pool)
        .await; // Ignore error if trigger doesn't exist

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS tasks_au AFTER UPDATE ON tasks BEGIN
            INSERT INTO tasks_fts(tasks_fts, rowid, name, spec) VALUES('delete', old.id, old.name, old.spec);
            INSERT INTO tasks_fts(rowid, name, spec) VALUES (new.id, new.name, new.spec);
        END
        "#,
    )
    .execute(pool)
    .await?;

    // Rebuild FTS index with existing data from tasks table
    sqlx::query(
        r#"
        INSERT INTO tasks_fts(rowid, name, spec)
        SELECT id, name, spec FROM tasks
        "#,
    )
    .execute(pool)
    .await?;

    // Create events table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            log_type TEXT NOT NULL,
            discussion_data TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create index on task_id for events
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_events_task_id ON events(task_id)
        "#,
    )
    .execute(pool)
    .await?;

    // Create FTS5 virtual table for events
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS events_fts USING fts5(
            discussion_data,
            content=events,
            content_rowid=id
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create triggers for events FTS
    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS events_ai AFTER INSERT ON events BEGIN
            INSERT INTO events_fts(rowid, discussion_data) VALUES (new.id, new.discussion_data);
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS events_ad AFTER DELETE ON events BEGIN
            DELETE FROM events_fts WHERE rowid = old.id;
        END
        "#,
    )
    .execute(pool)
    .await?;

    // Recreate trigger with correct FTS5 syntax (drop and create for migration from buggy version)
    let _ = sqlx::query("DROP TRIGGER IF EXISTS events_au")
        .execute(pool)
        .await; // Ignore error if trigger doesn't exist

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS events_au AFTER UPDATE ON events BEGIN
            INSERT INTO events_fts(events_fts, rowid, discussion_data) VALUES('delete', old.id, old.discussion_data);
            INSERT INTO events_fts(rowid, discussion_data) VALUES (new.id, new.discussion_data);
        END
        "#,
    )
    .execute(pool)
    .await?;

    // Create workspace_state table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS workspace_state (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create sessions table for multi-session focus support (v0.11.0)
    // Each Claude Code session can have its own focused task
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            session_id TEXT PRIMARY KEY,
            current_task_id INTEGER,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_active_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (current_task_id) REFERENCES tasks(id) ON DELETE SET NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for session cleanup queries
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_last_active
        ON sessions(last_active_at)
        "#,
    )
    .execute(pool)
    .await?;

    // Create suggestions table for async LLM hints (v0.13.0)
    // Stores background analysis results to show at next interaction
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS suggestions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            dismissed BOOLEAN NOT NULL DEFAULT 0,
            CHECK (type IN ('task_structure', 'event_synthesis'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Index for finding active suggestions
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_suggestions_active
        ON suggestions(dismissed, created_at)
        WHERE dismissed = 0
        "#,
    )
    .execute(pool)
    .await?;

    // Migrate existing current_task_id from workspace_state to default session (v0.11.0)
    // This ensures backward compatibility - existing focus is preserved in session "-1"
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO sessions (session_id, current_task_id, created_at, last_active_at)
        SELECT '-1', CAST(value AS INTEGER), datetime('now'), datetime('now')
        FROM workspace_state
        WHERE key = 'current_task_id' AND value IS NOT NULL AND value != ''
        "#,
    )
    .execute(pool)
    .await?;

    // Create dependencies table for v0.2.0
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS dependencies (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            blocking_task_id INTEGER NOT NULL,
            blocked_task_id INTEGER NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (blocking_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (blocked_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            UNIQUE(blocking_task_id, blocked_task_id),
            CHECK(blocking_task_id != blocked_task_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes for dependencies table
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_dependencies_blocking
        ON dependencies(blocking_task_id)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_dependencies_blocked
        ON dependencies(blocked_task_id)
        "#,
    )
    .execute(pool)
    .await?;

    // Create composite index for event filtering (v0.2.0)
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_events_task_type_time
        ON events(task_id, log_type, timestamp)
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes for task sorting and filtering (v0.7.2 - Phase 1)
    // Index 1: Support FocusAware and Priority sorting with status/parent filtering
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_tasks_status_parent_priority
        ON tasks(status, parent_id, priority, id)
        "#,
    )
    .execute(pool)
    .await?;

    // Index 2: Support Priority sorting mode (aligned with pick_next)
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_tasks_priority_complexity
        ON tasks(priority, complexity, id)
        "#,
    )
    .execute(pool)
    .await?;

    // Index 3: Support Time sorting and FocusAware secondary sorting (partial index for performance)
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_tasks_doing_at
        ON tasks(first_doing_at)
        WHERE status = 'doing'
        "#,
    )
    .execute(pool)
    .await?;

    // Add owner column to tasks table (v0.9.0 - Human Task Protection)
    // Owner identifies who created the task (e.g. 'human', 'ai', or any custom string)
    // Note: SQLite doesn't support CHECK constraints in ALTER TABLE, so we use a simple default
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN owner TEXT NOT NULL DEFAULT 'human'")
        .execute(pool)
        .await; // Ignore error if column already exists

    // Add metadata column to tasks table (v0.12.0 - Extensible metadata)
    // Free-form JSON string for storing additional task metadata
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN metadata TEXT DEFAULT '{}'")
        .execute(pool)
        .await; // Ignore error if column already exists

    // Update schema version to 0.12.0
    sqlx::query(
        r#"
        INSERT INTO workspace_state (key, value)
        VALUES ('schema_version', '0.12.0')
        ON CONFLICT(key) DO UPDATE SET value = '0.12.0'
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_pool_success() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = create_pool(&db_path).await.unwrap();

        // Verify we can execute a query
        let result: i64 = sqlx::query_scalar("SELECT 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_run_migrations_creates_tables() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();

        run_migrations(&pool).await.unwrap();

        // Verify tables were created
        let tables: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert!(tables.contains(&"tasks".to_string()));
        assert!(tables.contains(&"events".to_string()));
        assert!(tables.contains(&"workspace_state".to_string()));
    }

    #[tokio::test]
    async fn test_run_migrations_creates_fts_tables() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();

        run_migrations(&pool).await.unwrap();

        // Verify FTS tables were created
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%_fts'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert!(tables.contains(&"tasks_fts".to_string()));
        assert!(tables.contains(&"events_fts".to_string()));
    }

    #[tokio::test]
    async fn test_run_migrations_creates_triggers() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();

        run_migrations(&pool).await.unwrap();

        // Verify triggers were created
        let triggers: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='trigger'")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert!(triggers.contains(&"tasks_ai".to_string()));
        assert!(triggers.contains(&"tasks_ad".to_string()));
        assert!(triggers.contains(&"tasks_au".to_string()));
        assert!(triggers.contains(&"events_ai".to_string()));
        assert!(triggers.contains(&"events_ad".to_string()));
        assert!(triggers.contains(&"events_au".to_string()));
    }

    #[tokio::test]
    async fn test_run_migrations_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();

        // Run migrations twice
        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Should not fail - migrations are idempotent
        let tables: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table'")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert!(tables.len() >= 3);
    }

    #[tokio::test]
    async fn test_fts_triggers_work() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Insert a task
        sqlx::query("INSERT INTO tasks (name, spec, status) VALUES (?, ?, ?)")
            .bind("Test task")
            .bind("Test spec")
            .bind("todo")
            .execute(&pool)
            .await
            .unwrap();

        // Verify FTS was updated
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks_fts WHERE name MATCH 'Test'")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_workspace_state_table_structure() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Insert and retrieve workspace state
        sqlx::query("INSERT INTO workspace_state (key, value) VALUES (?, ?)")
            .bind("test_key")
            .bind("test_value")
            .execute(&pool)
            .await
            .unwrap();

        let value: String = sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = ?")
            .bind("test_key")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(value, "test_value");
    }

    #[tokio::test]
    async fn test_task_status_constraint() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Try to insert task with invalid status
        let result = sqlx::query("INSERT INTO tasks (name, status) VALUES (?, ?)")
            .bind("Test")
            .bind("invalid_status")
            .execute(&pool)
            .await;

        // Should fail due to CHECK constraint
        assert!(result.is_err());
    }

    // v0.2.0 Migration Tests

    #[tokio::test]
    async fn test_dependencies_table_created() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Verify dependencies table exists
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='dependencies'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert!(tables.contains(&"dependencies".to_string()));
    }

    #[tokio::test]
    async fn test_dependencies_indexes_created() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Verify indexes exist
        let indexes: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='index' AND name IN ('idx_dependencies_blocking', 'idx_dependencies_blocked', 'idx_events_task_type_time')",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert!(indexes.contains(&"idx_dependencies_blocking".to_string()));
        assert!(indexes.contains(&"idx_dependencies_blocked".to_string()));
        assert!(indexes.contains(&"idx_events_task_type_time".to_string()));
    }

    #[tokio::test]
    async fn test_dependencies_self_dependency_constraint() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Create a task
        sqlx::query("INSERT INTO tasks (name, status) VALUES (?, ?)")
            .bind("Task 1")
            .bind("todo")
            .execute(&pool)
            .await
            .unwrap();

        // Try to create self-dependency (should fail)
        let result = sqlx::query(
            "INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)",
        )
        .bind(1)
        .bind(1)
        .execute(&pool)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dependencies_unique_constraint() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Create tasks
        for i in 1..=2 {
            sqlx::query("INSERT INTO tasks (name, status) VALUES (?, ?)")
                .bind(format!("Task {}", i))
                .bind("todo")
                .execute(&pool)
                .await
                .unwrap();
        }

        // Create dependency
        sqlx::query("INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)")
            .bind(1)
            .bind(2)
            .execute(&pool)
            .await
            .unwrap();

        // Try to create duplicate dependency (should fail)
        let result = sqlx::query(
            "INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)",
        )
        .bind(1)
        .bind(2)
        .execute(&pool)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dependencies_cascade_delete() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Create tasks
        for i in 1..=2 {
            sqlx::query("INSERT INTO tasks (name, status) VALUES (?, ?)")
                .bind(format!("Task {}", i))
                .bind("todo")
                .execute(&pool)
                .await
                .unwrap();
        }

        // Create dependency
        sqlx::query("INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)")
            .bind(1)
            .bind(2)
            .execute(&pool)
            .await
            .unwrap();

        // Verify dependency exists
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dependencies")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Delete blocking task
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(1)
            .execute(&pool)
            .await
            .unwrap();

        // Verify dependency was cascade deleted
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dependencies")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_schema_version_tracking() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Verify schema version is set to 0.2.0
        let version: String =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'schema_version'")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(version, "0.12.0");
    }

    #[tokio::test]
    async fn test_migration_idempotency_v0_11_0() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();

        // Run migrations multiple times
        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Verify dependencies table exists and is functional
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='dependencies'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert!(tables.contains(&"dependencies".to_string()));

        // Verify schema version is still correct
        let version: String =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'schema_version'")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(version, "0.12.0");
    }

    #[tokio::test]
    async fn test_sessions_table_created() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Verify sessions table exists
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='sessions'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert!(tables.contains(&"sessions".to_string()));

        // Verify index exists
        let indices: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_sessions_last_active'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert!(indices.contains(&"idx_sessions_last_active".to_string()));
    }
}
