use crate::db::models::Event;
use crate::error::{IntentError, Result};
use chrono::Utc;
use sqlx::SqlitePool;

pub struct EventManager<'a> {
    pool: &'a SqlitePool,
}

impl<'a> EventManager<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Add a new event
    pub async fn add_event(
        &self,
        task_id: i64,
        log_type: &str,
        discussion_data: &str,
    ) -> Result<Event> {
        // Check if task exists
        let task_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?)")
                .bind(task_id)
                .fetch_one(self.pool)
                .await?;

        if !task_exists {
            return Err(IntentError::TaskNotFound(task_id));
        }

        let now = Utc::now();

        let result = sqlx::query(
            r#"
            INSERT INTO events (task_id, log_type, discussion_data, timestamp)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(task_id)
        .bind(log_type)
        .bind(discussion_data)
        .bind(now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();

        Ok(Event {
            id,
            task_id,
            timestamp: now,
            log_type: log_type.to_string(),
            discussion_data: discussion_data.to_string(),
        })
    }

    /// List events for a task
    pub async fn list_events(&self, task_id: i64, limit: Option<i64>) -> Result<Vec<Event>> {
        // Check if task exists
        let task_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?)")
                .bind(task_id)
                .fetch_one(self.pool)
                .await?;

        if !task_exists {
            return Err(IntentError::TaskNotFound(task_id));
        }

        let limit = limit.unwrap_or(100);

        let events = sqlx::query_as::<_, Event>(
            r#"
            SELECT id, task_id, timestamp, log_type, discussion_data
            FROM events
            WHERE task_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(task_id)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::TaskManager;
    use crate::test_utils::test_helpers::TestContext;

    #[tokio::test]
    async fn test_add_event() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());

        let task = task_mgr.add_task("Test task", None, None).await.unwrap();
        let event = event_mgr
            .add_event(task.id, "decision", "Test decision")
            .await
            .unwrap();

        assert_eq!(event.task_id, task.id);
        assert_eq!(event.log_type, "decision");
        assert_eq!(event.discussion_data, "Test decision");
    }

    #[tokio::test]
    async fn test_add_event_nonexistent_task() {
        let ctx = TestContext::new().await;
        let event_mgr = EventManager::new(ctx.pool());

        let result = event_mgr.add_event(999, "decision", "Test").await;
        assert!(matches!(result, Err(IntentError::TaskNotFound(999))));
    }

    #[tokio::test]
    async fn test_list_events() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());

        let task = task_mgr.add_task("Test task", None, None).await.unwrap();

        // Add multiple events
        event_mgr
            .add_event(task.id, "decision", "Decision 1")
            .await
            .unwrap();
        event_mgr
            .add_event(task.id, "blocker", "Blocker 1")
            .await
            .unwrap();
        event_mgr
            .add_event(task.id, "milestone", "Milestone 1")
            .await
            .unwrap();

        let events = event_mgr.list_events(task.id, None).await.unwrap();
        assert_eq!(events.len(), 3);

        // Events should be in reverse chronological order
        assert_eq!(events[0].log_type, "milestone");
        assert_eq!(events[1].log_type, "blocker");
        assert_eq!(events[2].log_type, "decision");
    }

    #[tokio::test]
    async fn test_list_events_with_limit() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());

        let task = task_mgr.add_task("Test task", None, None).await.unwrap();

        // Add 5 events
        for i in 0..5 {
            event_mgr
                .add_event(task.id, "test", &format!("Event {}", i))
                .await
                .unwrap();
        }

        let events = event_mgr.list_events(task.id, Some(3)).await.unwrap();
        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn test_list_events_nonexistent_task() {
        let ctx = TestContext::new().await;
        let event_mgr = EventManager::new(ctx.pool());

        let result = event_mgr.list_events(999, None).await;
        assert!(matches!(result, Err(IntentError::TaskNotFound(999))));
    }

    #[tokio::test]
    async fn test_list_events_empty() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());

        let task = task_mgr.add_task("Test task", None, None).await.unwrap();

        let events = event_mgr.list_events(task.id, None).await.unwrap();
        assert_eq!(events.len(), 0);
    }
}
