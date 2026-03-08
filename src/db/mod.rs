pub mod models;

use crate::error::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;

pub async fn create_pool(db_path: &Path) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_millis(5000))
        // Enforce FK constraints on every connection. Without this SQLite silently
        // ignores all FOREIGN KEY declarations (CASCADE, RESTRICT, etc.).
        .pragma("foreign_keys", "ON");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    Ok(pool)
}

// ---------------------------------------------------------------------------
// Version helpers
// ---------------------------------------------------------------------------

/// Parse a semver string like "0.13.0" into a comparable (major, minor, patch) tuple.
/// String comparison is incorrect for version numbers (e.g. "0.9.0" > "0.14.0" as strings).
fn parse_version(v: &str) -> (u32, u32, u32) {
    let mut parts = v.splitn(3, '.');
    let major = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

/// Check whether a table exists in the database.
async fn table_exists(pool: &SqlitePool, name: &str) -> bool {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name=?")
            .bind(name)
            .fetch_one(pool)
            .await
            .unwrap_or(0);
    count > 0
}

/// Check whether a column exists in a table.
#[cfg(test)]
async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> bool {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pragma_table_info(?) WHERE name=?")
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    count > 0
}

/// Write `schema_version` into `workspace_state`.
async fn set_schema_version(pool: &SqlitePool, version: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO workspace_state (key, value) VALUES ('schema_version', ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(version)
    .execute(pool)
    .await?;
    Ok(())
}

/// Detect the current schema version of the database.
///
/// Detection order (most-specific first):
/// 1. If `workspace_state.schema_version` exists → parse and return it.
/// 2. If the `tasks` table does not exist → fresh database `(0,0,0)`.
/// 3. Otherwise → pre-version database; return `(0,1,0)` so that every
///    incremental upgrade function runs. Each upgrade is idempotent
///    (`IF NOT EXISTS`, ignored `ALTER TABLE` errors, explicit `RESTRICT`
///    check in v0.14.0), so running them on a database that is already at a
///    newer state is safe. Column-based probing that returns an intermediate
///    version is NOT used here: a non-standard schema (e.g. a hand-added
///    `deleted_at`) could cause earlier, essential upgrades to be skipped.
async fn detect_schema_version(pool: &SqlitePool) -> Result<(u32, u32, u32)> {
    // 1. Authoritative version stored in workspace_state
    if table_exists(pool, "workspace_state").await {
        let stored: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'schema_version'")
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

        if let Some(v) = stored {
            return Ok(parse_version(&v));
        }
    }

    // 2. No tasks table → brand-new database
    if !table_exists(pool, "tasks").await {
        return Ok((0, 0, 0));
    }

    // 3. Pre-version database: tasks table exists but no schema_version.
    // Return the lowest non-fresh version so all upgrade functions run.
    Ok((0, 1, 0))
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    // WAL mode — always safe to set, even if already set
    sqlx::query("PRAGMA journal_mode=WAL").execute(pool).await?;

    let version = detect_schema_version(pool).await?;

    if version == (0, 0, 0) {
        // Brand-new database: build the full current schema in one shot
        migrate_fresh(pool).await?;
        return Ok(());
    }

    // Incremental upgrades for existing databases — each gate is independent so
    // that a partially-upgraded database can resume from wherever it stopped.
    if version < (0, 9, 0) {
        upgrade_to_v0_9_0(pool).await?;
    }
    if version < (0, 11, 0) {
        upgrade_to_v0_11_0(pool).await?;
    }
    if version < (0, 12, 0) {
        upgrade_to_v0_12_0(pool).await?;
    }
    if version < (0, 13, 0) {
        upgrade_to_v0_13_0(pool).await?;
    }
    if version < (0, 14, 0) {
        upgrade_to_v0_14_0(pool).await?;
    }
    if version < (0, 15, 0) {
        upgrade_to_v0_15_0(pool).await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Fresh install — full current schema
// ---------------------------------------------------------------------------

async fn migrate_fresh(pool: &SqlitePool) -> Result<()> {
    // ── tasks ──────────────────────────────────────────────────────────────
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
            deleted_at DATETIME,
            FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE,
            CHECK (status IN ('todo', 'doing', 'done')),
            CHECK (owner IS NOT NULL AND owner != '')
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ── tasks_fts (trigram tokenizer for CJK support) ───────────────────
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS tasks_fts USING fts5(
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

    // ── tasks triggers ───────────────────────────────────────────────────
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

    // For active tasks: keep FTS in sync on active→active updates.
    // Both old and new WHEN conditions are required so the trigger fires only
    // when the row remains active. Using only `new.deleted_at IS NULL` would
    // also fire on a restore (deleted→active), where the FTS entry no longer
    // exists — attempting to 'delete' a non-existent FTS5 entry corrupts the
    // index (SQLITE_CORRUPT, code 267).
    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS tasks_au_active
        AFTER UPDATE ON tasks WHEN old.deleted_at IS NULL AND new.deleted_at IS NULL BEGIN
            INSERT INTO tasks_fts(tasks_fts, rowid, name, spec)
                VALUES('delete', old.id, old.name, old.spec);
            INSERT INTO tasks_fts(rowid, name, spec) VALUES (new.id, new.name, new.spec);
        END
        "#,
    )
    .execute(pool)
    .await?;

    // For soft-deleted tasks: remove from FTS on the active→deleted transition.
    // WHEN clause requires old.deleted_at IS NULL so the trigger fires only once
    // (on the transition), not on every subsequent update to an already-deleted
    // task. Re-firing on an already-deleted row would attempt to remove a
    // non-existent FTS entry, which corrupts the FTS5 index.
    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS tasks_au_softdelete
        AFTER UPDATE ON tasks WHEN old.deleted_at IS NULL AND new.deleted_at IS NOT NULL BEGIN
            INSERT INTO tasks_fts(tasks_fts, rowid, name, spec)
                VALUES('delete', old.id, old.name, old.spec);
        END
        "#,
    )
    .execute(pool)
    .await?;

    // ── events ────────────────────────────────────────────────────────────
    // ON DELETE RESTRICT: events are an immutable audit log.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            log_type TEXT NOT NULL,
            discussion_data TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE RESTRICT
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_task_id ON events(task_id)")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_events_task_type_time \
         ON events(task_id, log_type, timestamp)",
    )
    .execute(pool)
    .await?;

    // ── events_fts ────────────────────────────────────────────────────────
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
            INSERT INTO events_fts(events_fts, rowid, discussion_data)
                VALUES('delete', old.id, old.discussion_data);
            INSERT INTO events_fts(rowid, discussion_data)
                VALUES (new.id, new.discussion_data);
        END
        "#,
    )
    .execute(pool)
    .await?;

    // ── workspace_state ───────────────────────────────────────────────────
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

    // ── sessions ──────────────────────────────────────────────────────────
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

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_last_active ON sessions(last_active_at)")
        .execute(pool)
        .await?;

    // ── suggestions ───────────────────────────────────────────────────────
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS suggestions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            dismissed BOOLEAN NOT NULL DEFAULT 0,
            CHECK (type IN ('task_structure', 'event_synthesis', 'error'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_suggestions_active \
         ON suggestions(dismissed, created_at) WHERE dismissed = 0",
    )
    .execute(pool)
    .await?;

    // ── dependencies ──────────────────────────────────────────────────────
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

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_dependencies_blocking ON dependencies(blocking_task_id)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_dependencies_blocked ON dependencies(blocked_task_id)",
    )
    .execute(pool)
    .await?;

    // ── tasks partial indexes (active rows only) ──────────────────────────
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tasks_status_parent_priority \
         ON tasks(status, parent_id, priority, id) WHERE deleted_at IS NULL",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tasks_priority_complexity \
         ON tasks(priority, complexity, id) WHERE deleted_at IS NULL",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tasks_doing_at \
         ON tasks(first_doing_at) WHERE status = 'doing' AND deleted_at IS NULL",
    )
    .execute(pool)
    .await?;

    set_schema_version(pool, "0.15.0").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Incremental upgrade: pre-0.9.0 → 0.9.0
// ---------------------------------------------------------------------------

async fn upgrade_to_v0_9_0(pool: &SqlitePool) -> Result<()> {
    // Ensure tasks_fts uses the trigram tokenizer.
    // Old databases may have been created with the default tokenizer; we must
    // drop and recreate to switch tokenizers.
    let uses_trigram: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_schema \
         WHERE name='tasks_fts' AND type='table' AND sql LIKE '%trigram%'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if uses_trigram == 0 {
        let _ = sqlx::query("DROP TABLE IF EXISTS tasks_fts")
            .execute(pool)
            .await;

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

        // Rebuild FTS index from existing active rows.
        // At this schema stage deleted_at does not exist yet, so all rows are active.
        sqlx::query("INSERT INTO tasks_fts(rowid, name, spec) SELECT id, name, spec FROM tasks")
            .execute(pool)
            .await?;
    }

    // Add owner column (idempotent: ignore error if column already exists)
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN owner TEXT NOT NULL DEFAULT 'human'")
        .execute(pool)
        .await;

    // Ensure workspace_state exists (needed for set_schema_version)
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

    set_schema_version(pool, "0.9.0").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Incremental upgrade: 0.9.x → 0.11.0
// ---------------------------------------------------------------------------

async fn upgrade_to_v0_11_0(pool: &SqlitePool) -> Result<()> {
    // sessions table
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

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_last_active ON sessions(last_active_at)")
        .execute(pool)
        .await?;

    // suggestions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS suggestions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            dismissed BOOLEAN NOT NULL DEFAULT 0,
            CHECK (type IN ('task_structure', 'event_synthesis', 'error'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_suggestions_active \
         ON suggestions(dismissed, created_at) WHERE dismissed = 0",
    )
    .execute(pool)
    .await?;

    // dependencies table
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

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_dependencies_blocking ON dependencies(blocking_task_id)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_dependencies_blocked ON dependencies(blocked_task_id)",
    )
    .execute(pool)
    .await?;

    // Composite event index
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_events_task_type_time \
         ON events(task_id, log_type, timestamp)",
    )
    .execute(pool)
    .await?;

    // Task sorting indexes (non-partial at this version; upgraded later in v0.14.0)
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tasks_status_parent_priority \
         ON tasks(status, parent_id, priority, id)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tasks_priority_complexity \
         ON tasks(priority, complexity, id)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tasks_doing_at \
         ON tasks(first_doing_at) WHERE status = 'doing'",
    )
    .execute(pool)
    .await?;

    // Migrate legacy current_task_id from workspace_state into the default session
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

    set_schema_version(pool, "0.11.0").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Incremental upgrade: 0.11.x → 0.12.0
// ---------------------------------------------------------------------------

async fn upgrade_to_v0_12_0(pool: &SqlitePool) -> Result<()> {
    // Add metadata column (idempotent: ignore error if column already exists)
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN metadata TEXT DEFAULT '{}'")
        .execute(pool)
        .await;

    set_schema_version(pool, "0.12.0").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Incremental upgrade: 0.12.x → 0.13.0
// ---------------------------------------------------------------------------

async fn upgrade_to_v0_13_0(pool: &SqlitePool) -> Result<()> {
    // Add deleted_at column. We capture whether it was actually *newly* added:
    // if the column already existed (idempotent re-run), the FTS index is
    // already correct and a full delete-all + re-insert would be wasteful
    // (and hazardous on large databases if interrupted).
    let column_added = sqlx::query("ALTER TABLE tasks ADD COLUMN deleted_at DATETIME")
        .execute(pool)
        .await
        .is_ok();

    // Replace any existing tasks_au* triggers with the split WHEN-gated versions
    // that correctly handle the soft-delete semantics introduced by deleted_at.
    let _ = sqlx::query("DROP TRIGGER IF EXISTS tasks_au")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TRIGGER IF EXISTS tasks_au_active")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TRIGGER IF EXISTS tasks_au_softdelete")
        .execute(pool)
        .await;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS tasks_au_active
        AFTER UPDATE ON tasks WHEN old.deleted_at IS NULL AND new.deleted_at IS NULL BEGIN
            INSERT INTO tasks_fts(tasks_fts, rowid, name, spec)
                VALUES('delete', old.id, old.name, old.spec);
            INSERT INTO tasks_fts(rowid, name, spec) VALUES (new.id, new.name, new.spec);
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS tasks_au_softdelete
        AFTER UPDATE ON tasks WHEN old.deleted_at IS NULL AND new.deleted_at IS NOT NULL BEGIN
            INSERT INTO tasks_fts(tasks_fts, rowid, name, spec)
                VALUES('delete', old.id, old.name, old.spec);
        END
        "#,
    )
    .execute(pool)
    .await?;

    // Rebuild FTS only when deleted_at was newly added. If the column already
    // existed, the FTS index is consistent and there is nothing to fix.
    if column_added {
        let has_fts_data: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks_fts")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        if has_fts_data > 0 {
            sqlx::query("INSERT INTO tasks_fts(tasks_fts) VALUES('delete-all')")
                .execute(pool)
                .await?;
        }

        sqlx::query(
            "INSERT INTO tasks_fts(rowid, name, spec) \
             SELECT id, name, spec FROM tasks WHERE deleted_at IS NULL",
        )
        .execute(pool)
        .await?;
    }

    set_schema_version(pool, "0.13.0").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Incremental upgrade: 0.13.x → 0.14.0
// ---------------------------------------------------------------------------

async fn upgrade_to_v0_14_0(pool: &SqlitePool) -> Result<()> {
    // ── Rebuild tasks indexes as partial indexes (WHERE deleted_at IS NULL) ──
    // Idempotent: DROP IF EXISTS + CREATE IF NOT EXISTS makes this safe to re-run.
    // (Pre-version databases are all treated as (0,1,0) so this function may run
    // on databases that already have partial indexes; DROP+CREATE handles that.)
    let _ = sqlx::query("DROP INDEX IF EXISTS idx_tasks_status_parent_priority")
        .execute(pool)
        .await;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tasks_status_parent_priority \
         ON tasks(status, parent_id, priority, id) WHERE deleted_at IS NULL",
    )
    .execute(pool)
    .await?;

    let _ = sqlx::query("DROP INDEX IF EXISTS idx_tasks_priority_complexity")
        .execute(pool)
        .await;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tasks_priority_complexity \
         ON tasks(priority, complexity, id) WHERE deleted_at IS NULL",
    )
    .execute(pool)
    .await?;

    let _ = sqlx::query("DROP INDEX IF EXISTS idx_tasks_doing_at")
        .execute(pool)
        .await;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_tasks_doing_at \
         ON tasks(first_doing_at) WHERE status = 'doing' AND deleted_at IS NULL",
    )
    .execute(pool)
    .await?;

    // ── Recreate events table with ON DELETE RESTRICT ─────────────────────
    // SQLite cannot ALTER a FK constraint in-place; we recreate the table.
    //
    // Gate: skip if events already declares RESTRICT (idempotent re-run).
    let needs_restrict: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_schema \
         WHERE type='table' AND name='events' AND sql NOT LIKE '%ON DELETE RESTRICT%'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if needs_restrict > 0 {
        // All DDL steps MUST run inside a single explicit transaction.
        // Without this, a crash mid-reconstruction leaves the database in an
        // inconsistent state (events_new exists, events gone).
        // pool.begin() pins us to one physical connection for the duration.
        let mut tx = pool.begin().await?;

        // Drop any leftover events_new from a previous interrupted upgrade.
        sqlx::query("DROP TABLE IF EXISTS events_new")
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE events_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                log_type TEXT NOT NULL,
                discussion_data TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE RESTRICT
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT INTO events_new SELECT * FROM events")
            .execute(&mut *tx)
            .await?;

        // DROP old table — also drops events_ai, events_ad, events_au triggers
        sqlx::query("DROP TABLE events").execute(&mut *tx).await?;
        sqlx::query("ALTER TABLE events_new RENAME TO events")
            .execute(&mut *tx)
            .await?;

        // Recreate indexes (dropped with the old table)
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_task_id ON events(task_id)")
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_events_task_type_time \
             ON events(task_id, log_type, timestamp)",
        )
        .execute(&mut *tx)
        .await?;

        // Recreate FTS sync triggers (dropped with the old table)
        sqlx::query(
            r#"
            CREATE TRIGGER IF NOT EXISTS events_ai AFTER INSERT ON events BEGIN
                INSERT INTO events_fts(rowid, discussion_data)
                    VALUES (new.id, new.discussion_data);
            END
            "#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            CREATE TRIGGER IF NOT EXISTS events_ad AFTER DELETE ON events BEGIN
                DELETE FROM events_fts WHERE rowid = old.id;
            END
            "#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            CREATE TRIGGER IF NOT EXISTS events_au AFTER UPDATE ON events BEGIN
                INSERT INTO events_fts(events_fts, rowid, discussion_data)
                    VALUES('delete', old.id, old.discussion_data);
                INSERT INTO events_fts(rowid, discussion_data)
                    VALUES (new.id, new.discussion_data);
            END
            "#,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    set_schema_version(pool, "0.14.0").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Incremental upgrade: 0.14.x → 0.15.0
// ---------------------------------------------------------------------------

async fn upgrade_to_v0_15_0(pool: &SqlitePool) -> Result<()> {
    // Fix tasks_au_active trigger: the original WHEN clause only checked
    // `new.deleted_at IS NULL`, which would fire on restore operations
    // (deleted→active), causing an attempt to remove a non-existent FTS5 entry
    // and corrupting the index.  Narrow to active→active transitions only.
    //
    // tasks_au_softdelete is correct as-is — its WHEN clause
    // (old.deleted_at IS NULL AND new.deleted_at IS NOT NULL) already narrows
    // to active→deleted only and does not need to be touched.
    let _ = sqlx::query("DROP TRIGGER IF EXISTS tasks_au_active")
        .execute(pool)
        .await;

    sqlx::query(
        r#"CREATE TRIGGER IF NOT EXISTS tasks_au_active
AFTER UPDATE ON tasks WHEN old.deleted_at IS NULL AND new.deleted_at IS NULL BEGIN
    INSERT INTO tasks_fts(tasks_fts, rowid, name, spec)
        VALUES('delete', old.id, old.name, old.spec);
    INSERT INTO tasks_fts(rowid, name, spec) VALUES (new.id, new.name, new.spec);
END"#,
    )
    .execute(pool)
    .await
    .map_err(crate::error::IntentError::DatabaseError)?;

    set_schema_version(pool, "0.15.0").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
        assert!(triggers.contains(&"tasks_au_active".to_string()));
        assert!(triggers.contains(&"tasks_au_softdelete".to_string()));
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

    // NOTE: This test uses a physical DELETE (not soft-delete) because it is
    // specifically testing the schema-level ON DELETE CASCADE behaviour of the
    // dependencies FK. Physical task deletion is NOT what the application does
    // (it uses soft-delete via deleted_at). If you need events on these tasks,
    // add them AFTER the physical DELETE assertion, not before — adding events
    // to a task that will be physically deleted would trigger ON DELETE RESTRICT
    // and cause this test to fail (which would be the correct behaviour).
    #[tokio::test]
    async fn test_dependencies_cascade_delete() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Create two tasks (no events: physical delete path requires no events
        // because events FK is ON DELETE RESTRICT)
        for i in 1..=2 {
            sqlx::query("INSERT INTO tasks (name, status) VALUES (?, ?)")
                .bind(format!("Task {}", i))
                .bind("todo")
                .execute(&pool)
                .await
                .unwrap();
        }

        sqlx::query("INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)")
            .bind(1)
            .bind(2)
            .execute(&pool)
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dependencies")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Physical delete (schema-level test only; application uses soft-delete)
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(1)
            .execute(&pool)
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dependencies")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    /// Soft-deleting a task must NOT remove its events: they are an immutable audit log.
    #[tokio::test]
    async fn test_soft_delete_preserves_events() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        sqlx::query("INSERT INTO tasks (name, status) VALUES ('audited task', 'todo')")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO events (task_id, log_type, discussion_data) VALUES (1, 'decision', 'why we chose X')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Soft-delete via deleted_at — the application path
        sqlx::query("UPDATE tasks SET deleted_at = datetime('now') WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        // Task must appear deleted
        let active: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE id = 1 AND deleted_at IS NULL")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(active, 0, "task should be soft-deleted");

        // Event must still exist — audit log is immutable
        let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events WHERE task_id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(events, 1, "event must be retained after soft-delete");
    }

    /// Physical deletion of a task that has events must be rejected by the
    /// ON DELETE RESTRICT FK on the events table (audit log invariant).
    #[tokio::test]
    async fn test_physical_delete_blocked_when_task_has_events() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        sqlx::query("INSERT INTO tasks (name, status) VALUES ('task with history', 'todo')")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO events (task_id, log_type, discussion_data) VALUES (1, 'decision', 'important decision')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Physical DELETE must fail: FK ON DELETE RESTRICT protects the audit log
        let result = sqlx::query("DELETE FROM tasks WHERE id = 1")
            .execute(&pool)
            .await;

        assert!(
            result.is_err(),
            "physical delete of task with events must be rejected by ON DELETE RESTRICT"
        );

        // Event must still be there
        let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events WHERE task_id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(events, 1, "event must not have been deleted");
    }

    #[tokio::test]
    async fn test_schema_version_tracking() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Verify schema version is set to 0.15.0
        let version: String =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'schema_version'")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(version, "0.15.0");
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

        assert_eq!(version, "0.15.0");
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

    // ── New tests for version-aware migration ─────────────────────────────

    #[tokio::test]
    async fn test_detect_schema_version_fresh() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();

        // Brand-new database: no tables at all
        let version = detect_schema_version(&pool).await.unwrap();
        assert_eq!(version, (0, 0, 0), "fresh database must return (0,0,0)");
    }

    #[tokio::test]
    async fn test_detect_schema_version_after_migration() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();

        run_migrations(&pool).await.unwrap();

        let version = detect_schema_version(&pool).await.unwrap();
        assert_eq!(
            version,
            (0, 15, 0),
            "after migration detect must return (0,15,0)"
        );
    }

    #[tokio::test]
    async fn test_migration_from_pre_version_db() {
        // Simulate an old database that has tasks and events but:
        //   - no owner column
        //   - no sessions table
        //   - no metadata column
        //   - no deleted_at column
        //   - no workspace_state (hence no schema_version)
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();

        // Bootstrap a minimal "old" schema manually
        sqlx::query(
            r#"
            CREATE TABLE tasks (
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
                FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE,
                CHECK (status IN ('todo', 'doing', 'done'))
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                log_type TEXT NOT NULL,
                discussion_data TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert some data to verify preservation across upgrade
        sqlx::query("INSERT INTO tasks (name, status) VALUES ('legacy task', 'todo')")
            .execute(&pool)
            .await
            .unwrap();

        // Verify the old schema is detected correctly
        let version_before = detect_schema_version(&pool).await.unwrap();
        assert_eq!(
            version_before,
            (0, 1, 0),
            "old schema without owner should be detected as (0,1,0)"
        );

        // Run migrations — should upgrade from (0,1,0) to (0,15,0)
        run_migrations(&pool).await.unwrap();

        // Verify final version
        let version: String =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'schema_version'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(version, "0.15.0");

        // Verify legacy data is intact
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(task_count, 1, "legacy task must survive the upgrade");

        // Verify new columns were added
        assert!(
            column_exists(&pool, "tasks", "owner").await,
            "owner column must exist after upgrade"
        );
        assert!(
            column_exists(&pool, "tasks", "metadata").await,
            "metadata column must exist after upgrade"
        );
        assert!(
            column_exists(&pool, "tasks", "deleted_at").await,
            "deleted_at column must exist after upgrade"
        );

        // Verify new tables were created
        assert!(
            table_exists(&pool, "sessions").await,
            "sessions table must exist after upgrade"
        );
        assert!(
            table_exists(&pool, "dependencies").await,
            "dependencies table must exist after upgrade"
        );
    }

    // ── FTS + soft delete interaction ─────────────────────────────────────

    /// After soft-deleting a task the FTS index must not return it.
    #[tokio::test]
    async fn test_fts_excludes_soft_deleted_task() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO tasks (name, spec, status) VALUES ('unique_fts_target', 'some spec', 'todo')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Confirm it is searchable before deletion
        let before: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks_fts WHERE name MATCH 'unique_fts_target'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(before, 1, "task must be in FTS before soft-delete");

        // Soft-delete via deleted_at (the tasks_au_softdelete trigger fires)
        sqlx::query(
            "UPDATE tasks SET deleted_at = datetime('now') WHERE name = 'unique_fts_target'",
        )
        .execute(&pool)
        .await
        .unwrap();

        // FTS must no longer return the task
        let after: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks_fts WHERE name MATCH 'unique_fts_target'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(after, 0, "soft-deleted task must be removed from FTS");
    }

    /// Soft-deleting one task must not affect FTS entries for other tasks.
    #[tokio::test]
    async fn test_fts_soft_delete_does_not_affect_siblings() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO tasks (name, spec, status) VALUES ('keep_this_task', 'spec a', 'todo')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tasks (name, spec, status) VALUES ('delete_this_task', 'spec b', 'todo')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Soft-delete only the second task
        sqlx::query(
            "UPDATE tasks SET deleted_at = datetime('now') WHERE name = 'delete_this_task'",
        )
        .execute(&pool)
        .await
        .unwrap();

        // The deleted task must not appear
        let deleted: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks_fts WHERE name MATCH 'delete_this_task'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(deleted, 0, "deleted task must not appear in FTS");

        // The surviving task must still be searchable
        let kept: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks_fts WHERE name MATCH 'keep_this_task'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(kept, 1, "surviving task must still be in FTS");
    }

    /// An update to an already-soft-deleted task (e.g. correcting spec while
    /// archived) must not cause FTS corruption. The softdelete trigger uses
    /// `old.deleted_at IS NULL AND new.deleted_at IS NOT NULL` so it fires only
    /// on the active→deleted transition, not on subsequent updates to a deleted
    /// row. Neither the old nor the new name must appear in FTS after.
    #[tokio::test]
    async fn test_fts_update_of_soft_deleted_task_stays_excluded() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO tasks (name, spec, status) VALUES ('archived_task', 'old spec', 'todo')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Soft-delete
        sqlx::query("UPDATE tasks SET deleted_at = datetime('now') WHERE name = 'archived_task'")
            .execute(&pool)
            .await
            .unwrap();

        // Update the name while keeping deleted_at set — softdelete trigger does NOT
        // fire again (old.deleted_at IS NOT NULL); no FTS operation occurs.
        sqlx::query(
            "UPDATE tasks SET name = 'archived_task_renamed', spec = 'new spec' \
             WHERE name = 'archived_task'",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Neither old nor new name must appear in FTS
        let old_name: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks_fts WHERE name MATCH 'archived_task'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            old_name, 0,
            "old name of soft-deleted task must not be in FTS"
        );

        let new_name: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks_fts WHERE name MATCH 'archived_task_renamed'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            new_name, 0,
            "renamed soft-deleted task must not re-enter FTS"
        );
    }

    /// Restoring a soft-deleted task (clearing deleted_at) must NOT corrupt the FTS index.
    ///
    /// This is the exact scenario fixed in v0.15.0: the old trigger fired on
    /// deleted→active transitions and tried to remove a non-existent FTS entry,
    /// corrupting the index.  The fixed trigger only fires on active→active
    /// (old.deleted_at IS NULL AND new.deleted_at IS NULL), so a restore must be
    /// a no-op for the trigger — the caller is responsible for re-inserting the
    /// FTS row explicitly if needed.
    #[tokio::test]
    async fn test_fts_restore_does_not_corrupt_fts() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO tasks (name, spec, status) VALUES ('restore_me', 'spec text', 'todo')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Soft-delete — removes from FTS via tasks_au_softdelete trigger
        sqlx::query("UPDATE tasks SET deleted_at = datetime('now') WHERE name = 'restore_me'")
            .execute(&pool)
            .await
            .unwrap();

        // Restore — clears deleted_at; tasks_au_active must NOT fire (old.deleted_at IS NOT NULL)
        // This is the regression path: the old trigger would attempt
        // `INSERT INTO tasks_fts(tasks_fts, ...) VALUES('delete', ...)` for a row
        // that is no longer in the FTS index, corrupting it.
        sqlx::query("UPDATE tasks SET deleted_at = NULL WHERE name = 'restore_me'")
            .execute(&pool)
            .await
            .unwrap();

        // The trigger does not re-insert on restore; the row is absent from FTS.
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks_fts WHERE tasks_fts MATCH 'restore_me'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            count, 0,
            "restored task is not in FTS until explicitly re-indexed"
        );

        // The real proof that the FTS index is not corrupted: insert a new task
        // and verify it is searchable.  SQLite FTS5 corruption often surfaces only
        // on the next write or rebuild, not on a read-only COUNT(*).
        sqlx::query(
            "INSERT INTO tasks (name, spec, status) VALUES ('healthy_task', 'healthy spec', 'todo')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let healthy: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks_fts WHERE name MATCH 'healthy_task'")
                .fetch_one(&pool)
                .await
                .expect("FTS must remain functional after restore — index is not corrupted");
        assert_eq!(healthy, 1, "FTS index must still work after restore");
    }
}
