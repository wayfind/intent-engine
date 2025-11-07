use crate::db::models::Task;
use crate::error::{IntentError, Result};
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, Serialize)]
pub struct CurrentTaskResponse {
    pub current_task_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<Task>,
}

pub struct WorkspaceManager<'a> {
    pool: &'a SqlitePool,
}

impl<'a> WorkspaceManager<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Get the current task
    pub async fn get_current_task(&self) -> Result<CurrentTaskResponse> {
        let current_task_id: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(self.pool)
                .await?;

        let current_task_id = current_task_id.and_then(|id| id.parse::<i64>().ok());

        let task = if let Some(id) = current_task_id {
            sqlx::query_as::<_, Task>(
                r#"
                SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at
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
        })
    }

    /// Set the current task
    pub async fn set_current_task(&self, task_id: i64) -> Result<CurrentTaskResponse> {
        // Check if task exists
        let task_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?)")
                .bind(task_id)
                .fetch_one(self.pool)
                .await?;

        if !task_exists {
            return Err(IntentError::TaskNotFound(task_id));
        }

        // Set current task
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO workspace_state (key, value)
            VALUES ('current_task_id', ?)
            "#,
        )
        .bind(task_id.to_string())
        .execute(self.pool)
        .await?;

        self.get_current_task().await
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

        let response = workspace_mgr.get_current_task().await.unwrap();

        assert!(response.current_task_id.is_none());
        assert!(response.task.is_none());
    }

    #[tokio::test]
    async fn test_set_current_task() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task = task_mgr.add_task("Test task", None, None).await.unwrap();

        let response = workspace_mgr.set_current_task(task.id).await.unwrap();

        assert_eq!(response.current_task_id, Some(task.id));
        assert!(response.task.is_some());
        assert_eq!(response.task.unwrap().id, task.id);
    }

    #[tokio::test]
    async fn test_set_current_task_nonexistent() {
        let ctx = TestContext::new().await;
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let result = workspace_mgr.set_current_task(999).await;
        assert!(matches!(result, Err(IntentError::TaskNotFound(999))));
    }

    #[tokio::test]
    async fn test_update_current_task() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task1 = task_mgr.add_task("Task 1", None, None).await.unwrap();
        let task2 = task_mgr.add_task("Task 2", None, None).await.unwrap();

        // Set task1 as current
        workspace_mgr.set_current_task(task1.id).await.unwrap();

        // Update to task2
        let response = workspace_mgr.set_current_task(task2.id).await.unwrap();

        assert_eq!(response.current_task_id, Some(task2.id));
        assert_eq!(response.task.unwrap().id, task2.id);
    }

    #[tokio::test]
    async fn test_get_current_task_after_set() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        let task = task_mgr.add_task("Test task", None, None).await.unwrap();
        workspace_mgr.set_current_task(task.id).await.unwrap();

        let response = workspace_mgr.get_current_task().await.unwrap();

        assert_eq!(response.current_task_id, Some(task.id));
        assert!(response.task.is_some());
    }
}
