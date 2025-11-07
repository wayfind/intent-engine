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
            FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE,
            CHECK (status IN ('todo', 'doing', 'done'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create FTS5 virtual table for tasks
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS tasks_fts USING fts5(
            name,
            spec,
            content=tasks,
            content_rowid=id
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

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS tasks_au AFTER UPDATE ON tasks BEGIN
            UPDATE tasks_fts SET name = new.name, spec = new.spec WHERE rowid = old.id;
        END
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

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS events_au AFTER UPDATE ON events BEGIN
            UPDATE events_fts SET discussion_data = new.discussion_data WHERE rowid = old.id;
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

    Ok(())
}
