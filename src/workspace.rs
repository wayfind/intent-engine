use crate::db::models::Task;
use crate::error::{IntentError, Result};
use serde::Serialize;
use sqlx::SqlitePool;

/// Default session ID for backward compatibility
pub const DEFAULT_SESSION_ID: &str = "-1";

/// Resolve session ID from various sources
/// Priority: explicit param > IE_SESSION_ID env > default "-1"
pub fn resolve_session_id(explicit: Option<&str>) -> String {
    if let Some(s) = explicit {
        if !s.is_empty() {
            return s.to_string();
        }
    }

    if let Ok(s) = std::env::var("IE_SESSION_ID") {
        if !s.is_empty() {
            return s;
        }
    }

    DEFAULT_SESSION_ID.to_string()
}

#[derive(Debug, Serialize)]
pub struct CurrentTaskResponse {
    pub current_task_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<Task>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

pub struct WorkspaceManager<'a> {
    pool: &'a SqlitePool,
}

impl<'a> WorkspaceManager<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Get the current task for a session
    #[tracing::instrument(skip(self))]
    pub async fn get_current_task(&self, session_id: Option<&str>) -> Result<CurrentTaskResponse> {
        let session_id = resolve_session_id(session_id);

        // Try to get from sessions table first
        let current_task_id: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT current_task_id FROM sessions WHERE session_id = ?",
        )
        .bind(&session_id)
        .fetch_optional(self.pool)
        .await?
        .flatten();

        // Update last_active_at if session exists
        if current_task_id.is_some() {
            sqlx::query(
                "UPDATE sessions SET last_active_at = datetime('now') WHERE session_id = ?",
            )
            .bind(&session_id)
            .execute(self.pool)
            .await?;
        }

        let task = if let Some(id) = current_task_id {
            sqlx::query_as::<_, Task>(
                r#"
                SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
                FROM tasks
                WHERE id = ?
                "#,
            )
            .bind(id)
            .fetch_optional(self.pool)
            .await?
        } else {
            None
        };

        Ok(CurrentTaskResponse {
            current_task_id,
            task,
            session_id: Some(session_id),
        })
    }

    /// Set the current task for a session
    #[tracing::instrument(skip(self))]
    pub async fn set_current_task(
        &self,
        task_id: i64,
        session_id: Option<&str>,
    ) -> Result<CurrentTaskResponse> {
        let session_id = resolve_session_id(session_id);

        // Check if task exists
        let task_exists: bool =
            sqlx::query_scalar::<_, bool>(crate::sql_constants::CHECK_TASK_EXISTS)
                .bind(task_id)
                .fetch_one(self.pool)
                .await?;

        if !task_exists {
            return Err(IntentError::TaskNotFound(task_id));
        }

        // Upsert session with current task
        sqlx::query(
            r#"
            INSERT INTO sessions (session_id, current_task_id, created_at, last_active_at)
            VALUES (?, ?, datetime('now'), datetime('now'))
            ON CONFLICT(session_id) DO UPDATE SET
                current_task_id = excluded.current_task_id,
                last_active_at = datetime('now')
            "#,
        )
        .bind(&session_id)
        .bind(task_id)
        .execute(self.pool)
        .await?;

        self.get_current_task(Some(&session_id)).await
    }

    /// Clear the current task for a session
    pub async fn clear_current_task(&self, session_id: Option<&str>) -> Result<()> {
        let session_id = resolve_session_id(session_id);

        sqlx::query(
            "UPDATE sessions SET current_task_id = NULL, last_active_at = datetime('now') WHERE session_id = ?"
        )
        .bind(&session_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Clean up expired sessions (older than given hours)
    pub async fn cleanup_expired_sessions(&self, hours: u32) -> Result<u64> {
        let result = sqlx::query(&format!(
            "DELETE FROM sessions WHERE last_active_at < datetime('now', '-{} hours')",
            hours
        ))
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Enforce session limit (keep most recent N sessions)
    pub async fn enforce_session_limit(&self, max_sessions: u32) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE session_id IN (
                SELECT session_id FROM sessions
                ORDER BY last_active_at DESC
                LIMIT -1 OFFSET ?
            )
            "#,
        )
        .bind(max_sessions)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

impl crate::backend::WorkspaceBackend for WorkspaceManager<'_> {
    fn get_current_task(
        &self,
        session_id: Option<&str>,
    ) -> impl std::future::Future<Output = Result<CurrentTaskResponse>> + Send {
        self.get_current_task(session_id)
    }

    fn set_current_task(
        &self,
        task_id: i64,
        session_id: Option<&str>,
    ) -> impl std::future::Future<Output = Result<CurrentTaskResponse>> + Send {
        self.set_current_task(task_id, session_id)
    }

    fn clear_current_task(
        &self,
        session_id: Option<&str>,
    ) -> impl std::future::Future<Output = Result<()>> + Send {
        self.clear_current_task(session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::TaskManager;
    use crate::test_utils::test_helpers::TestContext;

    #[tokio::test]
    async fn test_get_current_task_none() {
        let ctx = TestContext::new().await;
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let response = workspace_mgr.get_current_task(None).await.unwrap();

        assert!(response.current_task_id.is_none());
        assert!(response.task.is_none());
    }

    #[tokio::test]
    async fn test_set_current_task() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task = task_mgr
            .add_task("Test task", None, None, None, None, None)
            .await
            .unwrap();

        let response = workspace_mgr.set_current_task(task.id, None).await.unwrap();

        assert_eq!(response.current_task_id, Some(task.id));
        assert!(response.task.is_some());
        assert_eq!(response.task.unwrap().id, task.id);
    }

    #[tokio::test]
    async fn test_set_current_task_nonexistent() {
        let ctx = TestContext::new().await;
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let result = workspace_mgr.set_current_task(999, None).await;
        assert!(matches!(result, Err(IntentError::TaskNotFound(999))));
    }

    #[tokio::test]
    async fn test_update_current_task() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task1 = task_mgr
            .add_task("Task 1", None, None, None, None, None)
            .await
            .unwrap();
        let task2 = task_mgr
            .add_task("Task 2", None, None, None, None, None)
            .await
            .unwrap();

        // Set task1 as current
        workspace_mgr
            .set_current_task(task1.id, None)
            .await
            .unwrap();

        // Update to task2
        let response = workspace_mgr
            .set_current_task(task2.id, None)
            .await
            .unwrap();

        assert_eq!(response.current_task_id, Some(task2.id));
        assert_eq!(response.task.unwrap().id, task2.id);
    }

    #[tokio::test]
    async fn test_get_current_task_after_set() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task = task_mgr
            .add_task("Test task", None, None, None, None, None)
            .await
            .unwrap();
        workspace_mgr.set_current_task(task.id, None).await.unwrap();

        let response = workspace_mgr.get_current_task(None).await.unwrap();

        assert_eq!(response.current_task_id, Some(task.id));
        assert!(response.task.is_some());
    }

    #[tokio::test]
    async fn test_current_task_response_serialization() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task = task_mgr
            .add_task("Test task", None, None, None, None, None)
            .await
            .unwrap();
        let response = workspace_mgr.set_current_task(task.id, None).await.unwrap();

        // Should serialize to JSON without errors
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("current_task_id"));
        assert!(json.contains("task"));
    }

    #[tokio::test]
    async fn test_current_task_response_none_serialization() {
        let ctx = TestContext::new().await;
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let response = workspace_mgr.get_current_task(None).await.unwrap();

        // When no task, task field should be omitted (skip_serializing_if)
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("current_task_id"));
        // task field should be omitted when None
        assert!(!json.contains("\"task\""));
    }

    #[tokio::test]
    async fn test_get_current_task_with_deleted_task() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task = task_mgr
            .add_task("Test task", None, None, None, None, None)
            .await
            .unwrap();
        workspace_mgr.set_current_task(task.id, None).await.unwrap();

        // Delete the task - this triggers ON DELETE SET NULL in sessions table
        task_mgr.delete_task(task.id).await.unwrap();

        let response = workspace_mgr.get_current_task(None).await.unwrap();

        // Due to ON DELETE SET NULL, current_task_id should be None
        assert!(response.current_task_id.is_none());
        assert!(response.task.is_none());
    }

    #[tokio::test]
    async fn test_set_current_task_returns_complete_task() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task = task_mgr
            .add_task("Test task", Some("Task spec"), None, None, None, None)
            .await
            .unwrap();

        let response = workspace_mgr.set_current_task(task.id, None).await.unwrap();

        // Verify task object is complete
        let returned_task = response.task.unwrap();
        assert_eq!(returned_task.id, task.id);
        assert_eq!(returned_task.name, "Test task");
        assert_eq!(returned_task.spec, Some("Task spec".to_string()));
        assert_eq!(returned_task.status, "todo");
    }

    #[tokio::test]
    async fn test_set_same_task_multiple_times() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task = task_mgr
            .add_task("Test task", None, None, None, None, None)
            .await
            .unwrap();

        // Set the same task multiple times (idempotent)
        workspace_mgr.set_current_task(task.id, None).await.unwrap();
        workspace_mgr.set_current_task(task.id, None).await.unwrap();
        let response = workspace_mgr.set_current_task(task.id, None).await.unwrap();

        assert_eq!(response.current_task_id, Some(task.id));
    }

    #[tokio::test]
    async fn test_session_isolation() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task1 = task_mgr
            .add_task("Task 1", None, None, None, None, None)
            .await
            .unwrap();
        let task2 = task_mgr
            .add_task("Task 2", None, None, None, None, None)
            .await
            .unwrap();

        // Set different tasks for different sessions
        workspace_mgr
            .set_current_task(task1.id, Some("session-a"))
            .await
            .unwrap();
        workspace_mgr
            .set_current_task(task2.id, Some("session-b"))
            .await
            .unwrap();

        // Each session should see its own task
        let response_a = workspace_mgr
            .get_current_task(Some("session-a"))
            .await
            .unwrap();
        let response_b = workspace_mgr
            .get_current_task(Some("session-b"))
            .await
            .unwrap();

        assert_eq!(response_a.current_task_id, Some(task1.id));
        assert_eq!(response_b.current_task_id, Some(task2.id));
        assert_eq!(response_a.session_id, Some("session-a".to_string()));
        assert_eq!(response_b.session_id, Some("session-b".to_string()));
    }

    #[tokio::test]
    async fn test_session_upsert() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task1 = task_mgr
            .add_task("Task 1", None, None, None, None, None)
            .await
            .unwrap();
        let task2 = task_mgr
            .add_task("Task 2", None, None, None, None, None)
            .await
            .unwrap();

        // Update same session's task
        workspace_mgr
            .set_current_task(task1.id, Some("session-x"))
            .await
            .unwrap();
        workspace_mgr
            .set_current_task(task2.id, Some("session-x"))
            .await
            .unwrap();

        // Should only have one session entry with the latest task
        let response = workspace_mgr
            .get_current_task(Some("session-x"))
            .await
            .unwrap();
        assert_eq!(response.current_task_id, Some(task2.id));

        // Check only one session row exists
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE session_id = 'session-x'")
                .fetch_one(ctx.pool())
                .await
                .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_get_current_task_with_changed_status() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task = task_mgr
            .add_task("Test task", None, None, None, None, None)
            .await
            .unwrap();
        workspace_mgr.set_current_task(task.id, None).await.unwrap();

        // Change task status
        task_mgr.start_task(task.id, false).await.unwrap();

        let response = workspace_mgr.get_current_task(None).await.unwrap();

        // Should reflect updated status
        assert_eq!(response.task.unwrap().status, "doing");
    }

    #[tokio::test]
    async fn test_clear_current_task() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task = task_mgr
            .add_task("Task", None, None, None, None, None)
            .await
            .unwrap();
        workspace_mgr
            .set_current_task(task.id, Some("test-session"))
            .await
            .unwrap();

        // Clear the current task
        workspace_mgr
            .clear_current_task(Some("test-session"))
            .await
            .unwrap();

        let response = workspace_mgr
            .get_current_task(Some("test-session"))
            .await
            .unwrap();
        assert!(response.current_task_id.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_expired_sessions() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task = task_mgr
            .add_task("Task", None, None, None, None, None)
            .await
            .unwrap();

        // Create a session
        workspace_mgr
            .set_current_task(task.id, Some("old-session"))
            .await
            .unwrap();

        // Manually set last_active_at to 25 hours ago
        sqlx::query(
            "UPDATE sessions SET last_active_at = datetime('now', '-25 hours') WHERE session_id = 'old-session'"
        )
        .execute(ctx.pool())
        .await
        .unwrap();

        // Create a recent session
        workspace_mgr
            .set_current_task(task.id, Some("new-session"))
            .await
            .unwrap();

        // Cleanup sessions older than 24 hours
        let deleted = workspace_mgr.cleanup_expired_sessions(24).await.unwrap();
        assert_eq!(deleted, 1);

        // Old session should be gone
        let response = workspace_mgr
            .get_current_task(Some("old-session"))
            .await
            .unwrap();
        assert!(response.current_task_id.is_none());

        // New session should still exist
        let response = workspace_mgr
            .get_current_task(Some("new-session"))
            .await
            .unwrap();
        assert_eq!(response.current_task_id, Some(task.id));
    }

    #[tokio::test]
    async fn test_resolve_session_id_priority() {
        // Test explicit param takes priority
        assert_eq!(resolve_session_id(Some("explicit")), "explicit");

        // Test empty explicit falls through to env var or default
        let empty_result = resolve_session_id(Some(""));
        // When IE_SESSION_ID is set, it uses that; otherwise uses DEFAULT_SESSION_ID
        if let Ok(env_session) = std::env::var("IE_SESSION_ID") {
            if !env_session.is_empty() {
                assert_eq!(empty_result, env_session);
            } else {
                assert_eq!(empty_result, DEFAULT_SESSION_ID);
            }
        } else {
            assert_eq!(empty_result, DEFAULT_SESSION_ID);
        }

        // Test None falls through to default (env var may or may not be set)
        let result = resolve_session_id(None);
        // Either uses env var or default
        assert!(!result.is_empty());
    }
}
