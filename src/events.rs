use crate::db::models::Event;
use crate::error::{IntentError, Result};
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

pub struct EventManager<'a> {
    pool: &'a SqlitePool,
    notifier: crate::notifications::NotificationSender,
    project_path: Option<String>,
}

impl<'a> EventManager<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self {
            pool,
            notifier: crate::notifications::NotificationSender::new(None, None),
            project_path: None,
        }
    }

    /// Create an EventManager with MCP notification support
    pub fn with_mcp_notifier(
        pool: &'a SqlitePool,
        project_path: String,
        mcp_notifier: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Self {
        Self {
            pool,
            notifier: crate::notifications::NotificationSender::new(None, Some(mcp_notifier)),
            project_path: Some(project_path),
        }
    }

    /// Create an EventManager with WebSocket notification support
    pub fn with_websocket(
        pool: &'a SqlitePool,
        ws_state: Arc<crate::dashboard::websocket::WebSocketState>,
        project_path: String,
    ) -> Self {
        Self {
            pool,
            notifier: crate::notifications::NotificationSender::new(Some(ws_state), None),
            project_path: Some(project_path),
        }
    }

    /// Internal helper: Notify UI about event creation
    async fn notify_event_created(&self, event: &Event) {
        use crate::dashboard::websocket::DatabaseOperationPayload;

        let Some(project_path) = &self.project_path else {
            return;
        };

        let event_json = match serde_json::to_value(event) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to serialize event for notification: {}", e);
                return;
            },
        };

        let payload =
            DatabaseOperationPayload::event_created(event.id, event_json, project_path.clone());
        self.notifier.send(payload).await;
    }

    /// Internal helper: Notify UI about event update
    async fn notify_event_updated(&self, event: &Event) {
        use crate::dashboard::websocket::DatabaseOperationPayload;

        let Some(project_path) = &self.project_path else {
            return;
        };

        let event_json = match serde_json::to_value(event) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to serialize event for notification: {}", e);
                return;
            },
        };

        let payload =
            DatabaseOperationPayload::event_updated(event.id, event_json, project_path.clone());
        self.notifier.send(payload).await;
    }

    /// Internal helper: Notify UI about event deletion
    async fn notify_event_deleted(&self, event_id: i64) {
        use crate::dashboard::websocket::DatabaseOperationPayload;

        let Some(project_path) = &self.project_path else {
            return;
        };

        let payload = DatabaseOperationPayload::event_deleted(event_id, project_path.clone());
        self.notifier.send(payload).await;
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

        let event = Event {
            id,
            task_id,
            timestamp: now,
            log_type: log_type.to_string(),
            discussion_data: discussion_data.to_string(),
        };

        // Notify WebSocket clients about the new event
        self.notify_event_created(&event).await;

        Ok(event)
    }

    /// Update an existing event
    pub async fn update_event(
        &self,
        event_id: i64,
        log_type: Option<&str>,
        discussion_data: Option<&str>,
    ) -> Result<Event> {
        // First, get the existing event to check if it exists
        let existing_event: Option<Event> = sqlx::query_as(
            "SELECT id, task_id, timestamp, log_type, discussion_data FROM events WHERE id = ?",
        )
        .bind(event_id)
        .fetch_optional(self.pool)
        .await?;

        let existing_event = existing_event.ok_or(IntentError::InvalidInput(format!(
            "Event {} not found",
            event_id
        )))?;

        // Update only the fields that are provided
        let new_log_type = log_type.unwrap_or(&existing_event.log_type);
        let new_discussion_data = discussion_data.unwrap_or(&existing_event.discussion_data);

        sqlx::query(
            r#"
            UPDATE events
            SET log_type = ?, discussion_data = ?
            WHERE id = ?
            "#,
        )
        .bind(new_log_type)
        .bind(new_discussion_data)
        .bind(event_id)
        .execute(self.pool)
        .await?;

        let updated_event = Event {
            id: existing_event.id,
            task_id: existing_event.task_id,
            timestamp: existing_event.timestamp,
            log_type: new_log_type.to_string(),
            discussion_data: new_discussion_data.to_string(),
        };

        // Notify WebSocket clients about the update
        self.notify_event_updated(&updated_event).await;

        Ok(updated_event)
    }

    /// Delete an event
    pub async fn delete_event(&self, event_id: i64) -> Result<()> {
        // First, get the event to check if it exists and get task_id for notification
        let event: Option<Event> = sqlx::query_as(
            "SELECT id, task_id, timestamp, log_type, discussion_data FROM events WHERE id = ?",
        )
        .bind(event_id)
        .fetch_optional(self.pool)
        .await?;

        let _event = event.ok_or(IntentError::InvalidInput(format!(
            "Event {} not found",
            event_id
        )))?;

        // Delete from FTS index first (if it exists)
        let _ = sqlx::query("DELETE FROM events_fts WHERE rowid = ?")
            .bind(event_id)
            .execute(self.pool)
            .await;

        // Delete the event
        sqlx::query("DELETE FROM events WHERE id = ?")
            .bind(event_id)
            .execute(self.pool)
            .await?;

        // Notify WebSocket clients about the deletion
        self.notify_event_deleted(event_id).await;

        Ok(())
    }

    /// List events for a task (or globally if task_id is None)
    pub async fn list_events(
        &self,
        task_id: Option<i64>,
        limit: Option<i64>,
        log_type: Option<String>,
        since: Option<String>,
    ) -> Result<Vec<Event>> {
        // Check if task exists (only if task_id provided)
        if let Some(tid) = task_id {
            let task_exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?)")
                    .bind(tid)
                    .fetch_one(self.pool)
                    .await?;

            if !task_exists {
                return Err(IntentError::TaskNotFound(tid));
            }
        }

        let limit = limit.unwrap_or(50);

        // Parse since duration if provided
        let since_timestamp = if let Some(duration_str) = since {
            Some(crate::time_utils::parse_duration(&duration_str)?)
        } else {
            None
        };

        // Build dynamic query based on filters
        let mut query = String::from(
            "SELECT id, task_id, timestamp, log_type, discussion_data FROM events WHERE 1=1",
        );
        let mut conditions = Vec::new();

        if task_id.is_some() {
            conditions.push("task_id = ?");
        }

        if log_type.is_some() {
            conditions.push("log_type = ?");
        }

        if since_timestamp.is_some() {
            conditions.push("timestamp >= ?");
        }

        if !conditions.is_empty() {
            query.push_str(" AND ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT ?");

        // Build and execute query
        let mut sql_query = sqlx::query_as::<_, Event>(&query);

        if let Some(tid) = task_id {
            sql_query = sql_query.bind(tid);
        }

        if let Some(ref typ) = log_type {
            sql_query = sql_query.bind(typ);
        }

        if let Some(ts) = since_timestamp {
            sql_query = sql_query.bind(ts);
        }

        sql_query = sql_query.bind(limit);

        let events = sql_query.fetch_all(self.pool).await?;

        Ok(events)
    }

    /// Search events using FTS5
    pub async fn search_events_fts5(
        &self,
        query: &str,
        limit: Option<i64>,
    ) -> Result<Vec<EventSearchResult>> {
        let limit = limit.unwrap_or(20);

        // Use FTS5 to search events and get snippets
        let results = sqlx::query(
            r#"
            SELECT
                e.id,
                e.task_id,
                e.timestamp,
                e.log_type,
                e.discussion_data,
                snippet(events_fts, 0, '**', '**', '...', 15) as match_snippet
            FROM events_fts
            INNER JOIN events e ON events_fts.rowid = e.id
            WHERE events_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            "#,
        )
        .bind(query)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        let mut search_results = Vec::new();
        for row in results {
            let event = Event {
                id: row.get("id"),
                task_id: row.get("task_id"),
                timestamp: row.get("timestamp"),
                log_type: row.get("log_type"),
                discussion_data: row.get("discussion_data"),
            };
            let match_snippet: String = row.get("match_snippet");

            search_results.push(EventSearchResult {
                event,
                match_snippet,
            });
        }

        Ok(search_results)
    }
}

/// Event search result with match snippet
#[derive(Debug)]
pub struct EventSearchResult {
    pub event: Event,
    pub match_snippet: String,
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

        let events = event_mgr
            .list_events(Some(task.id), None, None, None)
            .await
            .unwrap();
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

        let events = event_mgr
            .list_events(Some(task.id), Some(3), None, None)
            .await
            .unwrap();
        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn test_list_events_nonexistent_task() {
        let ctx = TestContext::new().await;
        let event_mgr = EventManager::new(ctx.pool());

        let result = event_mgr.list_events(Some(999), None, None, None).await;
        assert!(matches!(result, Err(IntentError::TaskNotFound(999))));
    }

    #[tokio::test]
    async fn test_list_events_empty() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());

        let task = task_mgr.add_task("Test task", None, None).await.unwrap();

        let events = event_mgr
            .list_events(Some(task.id), None, None, None)
            .await
            .unwrap();
        assert_eq!(events.len(), 0);
    }
}
