use crate::db::models::{
    DoneTaskResponse, Event, EventsSummary, NextStepSuggestion, PaginatedTasks, ParentTaskInfo,
    PickNextResponse, SpawnSubtaskResponse, SubtaskInfo, Task, TaskSortBy, TaskWithEvents,
    WorkspaceStats, WorkspaceStatus,
};
use crate::error::{IntentError, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use std::sync::Arc;

pub use crate::db::models::TaskContext;

/// Result of a delete operation within a transaction
#[derive(Debug, Clone)]
pub struct DeleteTaskResult {
    /// Whether the task was found (false if ID didn't exist)
    pub found: bool,
    /// Number of descendant tasks that were cascade-deleted
    pub descendant_count: i64,
}

/// Parameter struct for `TaskManager::update_task`.
/// Only set the fields you want to change; the rest default to `None` (no change).
#[derive(Debug, Default)]
pub struct TaskUpdate<'a> {
    pub name: Option<&'a str>,
    pub spec: Option<&'a str>,
    pub parent_id: Option<Option<i64>>,
    pub status: Option<&'a str>,
    pub complexity: Option<i32>,
    pub priority: Option<i32>,
    pub active_form: Option<&'a str>,
    pub owner: Option<&'a str>,
    pub metadata: Option<&'a str>,
}

pub struct TaskManager<'a> {
    pool: &'a SqlitePool,
    notifier: crate::notifications::NotificationSender,
    cli_notifier: Option<crate::dashboard::cli_notifier::CliNotifier>,
    project_path: Option<String>,
}

impl<'a> TaskManager<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self {
            pool,
            notifier: crate::notifications::NotificationSender::new(None),
            cli_notifier: Some(crate::dashboard::cli_notifier::CliNotifier::new()),
            project_path: None,
        }
    }

    /// Create a TaskManager with project path for CLI notifications
    pub fn with_project_path(pool: &'a SqlitePool, project_path: String) -> Self {
        Self {
            pool,
            notifier: crate::notifications::NotificationSender::new(None),
            cli_notifier: Some(crate::dashboard::cli_notifier::CliNotifier::new()),
            project_path: Some(project_path),
        }
    }

    /// Create a TaskManager with WebSocket notification support
    pub fn with_websocket(
        pool: &'a SqlitePool,
        ws_state: Arc<crate::dashboard::websocket::WebSocketState>,
        project_path: String,
    ) -> Self {
        Self {
            pool,
            notifier: crate::notifications::NotificationSender::new(Some(ws_state)),
            cli_notifier: None, // Dashboard context doesn't need CLI notifier
            project_path: Some(project_path),
        }
    }

    /// Internal helper: Notify UI about task creation
    async fn notify_task_created(&self, task: &Task) {
        use crate::dashboard::websocket::DatabaseOperationPayload;

        // WebSocket notification (Dashboard context)
        if let Some(project_path) = &self.project_path {
            let task_json = match serde_json::to_value(task) {
                Ok(json) => json,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to serialize task for notification");
                    return;
                },
            };

            let payload =
                DatabaseOperationPayload::task_created(task.id, task_json, project_path.clone());
            self.notifier.send(payload).await;
        }

        // CLI → Dashboard HTTP notification (CLI context)
        if let Some(cli_notifier) = &self.cli_notifier {
            cli_notifier
                .notify_task_changed(Some(task.id), "created", self.project_path.clone())
                .await;
        }
    }

    /// Internal helper: Notify UI about task update
    async fn notify_task_updated(&self, task: &Task) {
        use crate::dashboard::websocket::DatabaseOperationPayload;

        // WebSocket notification (Dashboard context)
        if let Some(project_path) = &self.project_path {
            let task_json = match serde_json::to_value(task) {
                Ok(json) => json,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to serialize task for notification");
                    return;
                },
            };

            let payload =
                DatabaseOperationPayload::task_updated(task.id, task_json, project_path.clone());
            self.notifier.send(payload).await;
        }

        // CLI → Dashboard HTTP notification (CLI context)
        if let Some(cli_notifier) = &self.cli_notifier {
            cli_notifier
                .notify_task_changed(Some(task.id), "updated", self.project_path.clone())
                .await;
        }
    }

    /// Internal helper: Notify UI about task deletion
    async fn notify_task_deleted(&self, task_id: i64) {
        use crate::dashboard::websocket::DatabaseOperationPayload;

        // WebSocket notification (Dashboard context)
        if let Some(project_path) = &self.project_path {
            let payload = DatabaseOperationPayload::task_deleted(task_id, project_path.clone());
            self.notifier.send(payload).await;
        }

        // CLI → Dashboard HTTP notification (CLI context)
        if let Some(cli_notifier) = &self.cli_notifier {
            cli_notifier
                .notify_task_changed(Some(task_id), "deleted", self.project_path.clone())
                .await;
        }
    }

    /// Add a new task
    /// owner: identifies who created the task (e.g. 'human', 'ai', or any custom string)
    #[tracing::instrument(skip(self), fields(task_name = %name))]
    pub async fn add_task(
        &self,
        name: &str,
        spec: Option<&str>,
        parent_id: Option<i64>,
        owner: Option<&str>,
    ) -> Result<Task> {
        // Check for circular dependency if parent_id is provided
        if let Some(pid) = parent_id {
            self.check_task_exists(pid).await?;
        }

        let now = Utc::now();
        let owner = owner.unwrap_or("human");

        let result = sqlx::query(
            r#"
            INSERT INTO tasks (name, spec, parent_id, status, first_todo_at, owner)
            VALUES (?, ?, ?, 'todo', ?, ?)
            "#,
        )
        .bind(name)
        .bind(spec)
        .bind(parent_id)
        .bind(now)
        .bind(owner)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        let task = self.get_task(id).await?;

        // Notify WebSocket clients about the new task
        self.notify_task_created(&task).await;

        Ok(task)
    }

    // =========================================================================
    // Transaction-aware methods (for batch operations like PlanExecutor)
    // These methods do NOT notify - caller is responsible for notifications
    // =========================================================================

    /// Create a task within a transaction (no notification)
    ///
    /// This is used by PlanExecutor for batch operations where:
    /// - Multiple tasks need atomic creation
    /// - Notification should happen after all tasks are committed
    ///
    /// # Arguments
    /// * `tx` - The active transaction
    /// * `name` - Task name
    /// * `spec` - Optional task specification
    /// * `priority` - Optional priority (1=critical, 2=high, 3=medium, 4=low)
    /// * `status` - Optional status string ("todo", "doing", "done")
    /// * `active_form` - Optional active form description
    /// * `owner` - Task owner (e.g. "human", "ai", or any custom string)
    ///
    /// # Returns
    /// The ID of the created task
    #[allow(clippy::too_many_arguments)]
    pub async fn create_task_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        name: &str,
        spec: Option<&str>,
        priority: Option<i32>,
        status: Option<&str>,
        active_form: Option<&str>,
        owner: &str,
    ) -> Result<i64> {
        let now = Utc::now();
        let status = status.unwrap_or("todo");
        let priority = priority.unwrap_or(3); // Default: medium

        let result = sqlx::query(
            r#"
            INSERT INTO tasks (name, spec, priority, status, active_form, first_todo_at, owner)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(name)
        .bind(spec)
        .bind(priority)
        .bind(status)
        .bind(active_form)
        .bind(now)
        .bind(owner)
        .execute(&mut **tx)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Update a task within a transaction (no notification)
    ///
    /// Only updates fields that are Some - supports partial updates.
    /// Does NOT update name (used for identity) or timestamps.
    ///
    /// # Arguments
    /// * `tx` - The active transaction
    /// * `task_id` - ID of the task to update
    /// * `spec` - New spec (if Some)
    /// * `priority` - New priority (if Some)
    /// * `status` - New status (if Some)
    /// * `active_form` - New active form (if Some)
    pub async fn update_task_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        task_id: i64,
        spec: Option<&str>,
        priority: Option<i32>,
        status: Option<&str>,
        active_form: Option<&str>,
    ) -> Result<()> {
        // Update spec if provided
        if let Some(spec) = spec {
            sqlx::query("UPDATE tasks SET spec = ? WHERE id = ?")
                .bind(spec)
                .bind(task_id)
                .execute(&mut **tx)
                .await?;
        }

        // Update priority if provided
        if let Some(priority) = priority {
            sqlx::query("UPDATE tasks SET priority = ? WHERE id = ?")
                .bind(priority)
                .bind(task_id)
                .execute(&mut **tx)
                .await?;
        }

        // Update status if provided
        if let Some(status) = status {
            sqlx::query("UPDATE tasks SET status = ? WHERE id = ?")
                .bind(status)
                .bind(task_id)
                .execute(&mut **tx)
                .await?;
        }

        // Update active_form if provided
        if let Some(active_form) = active_form {
            sqlx::query("UPDATE tasks SET active_form = ? WHERE id = ?")
                .bind(active_form)
                .bind(task_id)
                .execute(&mut **tx)
                .await?;
        }

        Ok(())
    }

    /// Set parent_id for a task within a transaction (no notification)
    ///
    /// Used to establish parent-child relationships after tasks are created.
    pub async fn set_parent_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        task_id: i64,
        parent_id: i64,
    ) -> Result<()> {
        sqlx::query("UPDATE tasks SET parent_id = ? WHERE id = ?")
            .bind(parent_id)
            .bind(task_id)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    /// Clear parent_id for a task in a transaction (make it a root task)
    ///
    /// Used when explicitly setting parent_id to null in JSON.
    pub async fn clear_parent_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        task_id: i64,
    ) -> Result<()> {
        sqlx::query("UPDATE tasks SET parent_id = NULL WHERE id = ?")
            .bind(task_id)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    /// Delete a task within a transaction (no notification)
    ///
    /// Used by PlanExecutor for batch delete operations.
    /// WebSocket notification is sent after transaction commit via notify_batch_changed().
    ///
    /// **Warning**: Due to `ON DELETE CASCADE` on `parent_id`, deleting a parent task
    /// will also delete all descendant tasks. The returned `DeleteTaskResult` includes
    /// the count of descendants that will be cascade-deleted.
    ///
    /// Returns `DeleteTaskResult` with:
    /// - `found`: whether the task existed
    /// - `descendant_count`: number of descendants that will be cascade-deleted
    ///
    /// Note: Focus protection is handled by the caller (PlanExecutor) BEFORE
    /// calling this function, using `find_focused_in_subtree_in_tx`.
    pub async fn delete_task_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        task_id: i64,
    ) -> Result<DeleteTaskResult> {
        // Check if task exists and count descendants before deletion
        let task_info: Option<(i64,)> = sqlx::query_as("SELECT id FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_optional(&mut **tx)
            .await?;

        if task_info.is_none() {
            return Ok(DeleteTaskResult {
                found: false,
                descendant_count: 0,
            });
        }

        // Count descendants that will be cascade-deleted
        let descendant_count = self.count_descendants_in_tx(tx, task_id).await?;

        // Perform the delete (CASCADE will handle children)
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(task_id)
            .execute(&mut **tx)
            .await?;

        Ok(DeleteTaskResult {
            found: true,
            descendant_count,
        })
    }

    /// Count all descendants of a task (children, grandchildren, etc.)
    async fn count_descendants_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        task_id: i64,
    ) -> Result<i64> {
        // Use recursive CTE to count all descendants
        let count: (i64,) = sqlx::query_as(
            r#"
            WITH RECURSIVE descendants AS (
                SELECT id FROM tasks WHERE parent_id = ?
                UNION ALL
                SELECT t.id FROM tasks t
                INNER JOIN descendants d ON t.parent_id = d.id
            )
            SELECT COUNT(*) FROM descendants
            "#,
        )
        .bind(task_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(count.0)
    }

    /// Find if a task or any of its descendants is ANY session's focus
    ///
    /// This is critical for delete protection: deleting a parent task cascades
    /// to all descendants, so we must check the entire subtree for focus.
    ///
    /// Focus protection is GLOBAL - a task focused by any session cannot be deleted.
    /// This prevents one session from accidentally breaking another session's work.
    ///
    /// Returns `Some((task_id, session_id))` if any task in the subtree is focused,
    /// `None` if no focus found in the subtree.
    pub async fn find_focused_in_subtree_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        task_id: i64,
    ) -> Result<Option<(i64, String)>> {
        // Use recursive CTE to get all task IDs in the subtree (including the root)
        // Then check if any of them is focused by ANY session
        let row: Option<(i64, String)> = sqlx::query_as(
            r#"
            WITH RECURSIVE subtree AS (
                SELECT id FROM tasks WHERE id = ?
                UNION ALL
                SELECT t.id FROM tasks t
                INNER JOIN subtree s ON t.parent_id = s.id
            )
            SELECT s.current_task_id, s.session_id FROM sessions s
            WHERE s.current_task_id IN (SELECT id FROM subtree)
            LIMIT 1
            "#,
        )
        .bind(task_id)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(row)
    }

    /// Count incomplete children of a task within a transaction
    ///
    /// Returns the number of child tasks that are not in 'done' status.
    /// Used to validate that all children are complete before marking parent as done.
    pub async fn count_incomplete_children_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        task_id: i64,
    ) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(crate::sql_constants::COUNT_INCOMPLETE_CHILDREN)
            .bind(task_id)
            .fetch_one(&mut **tx)
            .await?;

        Ok(count.0)
    }

    /// Complete a task within a transaction (core business logic)
    ///
    /// This is the single source of truth for task completion logic:
    /// - Validates all children are complete
    /// - Updates status to 'done'
    /// - Sets first_done_at timestamp
    ///
    /// Called by both `done_task()` and `PlanExecutor`.
    pub async fn complete_task_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        task_id: i64,
    ) -> Result<()> {
        // Check if all children are done
        let incomplete_count = self.count_incomplete_children_in_tx(tx, task_id).await?;
        if incomplete_count > 0 {
            return Err(IntentError::UncompletedChildren);
        }

        // Update task status to done
        let now = chrono::Utc::now();
        sqlx::query(
            r#"
            UPDATE tasks
            SET status = 'done', first_done_at = COALESCE(first_done_at, ?)
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(task_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Notify Dashboard about a batch operation
    ///
    /// Call this after committing a transaction that created/updated multiple tasks.
    /// Sends a single "batch_update" notification instead of per-task notifications.
    pub async fn notify_batch_changed(&self) {
        if let Some(cli_notifier) = &self.cli_notifier {
            cli_notifier
                .notify_task_changed(None, "batch_update", self.project_path.clone())
                .await;
        }
    }

    // =========================================================================
    // End of transaction-aware methods
    // =========================================================================

    /// Get a task by ID
    #[tracing::instrument(skip(self))]
    pub async fn get_task(&self, id: i64) -> Result<Task> {
        let task = sqlx::query_as::<_, Task>(
            r#"
            SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
            FROM tasks
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?
        .ok_or(IntentError::TaskNotFound(id))?;

        Ok(task)
    }

    /// Get a task with events summary
    pub async fn get_task_with_events(&self, id: i64) -> Result<TaskWithEvents> {
        let task = self.get_task(id).await?;
        let events_summary = self.get_events_summary(id).await?;

        Ok(TaskWithEvents {
            task,
            events_summary: Some(events_summary),
        })
    }

    /// Get full ancestry chain for a task
    ///
    /// Returns a vector of tasks from the given task up to the root:
    /// [task itself, parent, grandparent, ..., root]
    ///
    /// Example:
    /// - Task 42 (parent_id: 55) → [Task 42, Task 55, ...]
    /// - Task 100 (parent_id: null) → [Task 100]
    pub async fn get_task_ancestry(&self, task_id: i64) -> Result<Vec<Task>> {
        let mut chain = Vec::new();
        let mut current_id = Some(task_id);

        while let Some(id) = current_id {
            let task = self.get_task(id).await?;
            current_id = task.parent_id;
            chain.push(task);
        }

        Ok(chain)
    }

    /// Get task context - the complete family tree of a task
    ///
    /// Returns:
    /// - task: The requested task
    /// - ancestors: Parent chain up to root (ordered from immediate parent to root)
    /// - siblings: Other tasks at the same level (same parent_id)
    /// - children: Direct subtasks of this task
    pub async fn get_task_context(&self, id: i64) -> Result<TaskContext> {
        // Get the main task
        let task = self.get_task(id).await?;

        // Get ancestors (walk up parent chain)
        let mut ancestors = Vec::new();
        let mut current_parent_id = task.parent_id;

        while let Some(parent_id) = current_parent_id {
            let parent = self.get_task(parent_id).await?;
            current_parent_id = parent.parent_id;
            ancestors.push(parent);
        }

        // Get siblings (tasks with same parent_id)
        let siblings = if let Some(parent_id) = task.parent_id {
            sqlx::query_as::<_, Task>(
                r#"
                SELECT id, parent_id, name, spec, status, complexity, priority,
                       first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
                FROM tasks
                WHERE parent_id = ? AND id != ?
                ORDER BY priority ASC NULLS LAST, id ASC
                "#,
            )
            .bind(parent_id)
            .bind(id)
            .fetch_all(self.pool)
            .await?
        } else {
            // For root tasks, get other root tasks as siblings
            sqlx::query_as::<_, Task>(
                r#"
                SELECT id, parent_id, name, spec, status, complexity, priority,
                       first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
                FROM tasks
                WHERE parent_id IS NULL AND id != ?
                ORDER BY priority ASC NULLS LAST, id ASC
                "#,
            )
            .bind(id)
            .fetch_all(self.pool)
            .await?
        };

        // Get children (direct subtasks)
        let children = sqlx::query_as::<_, Task>(
            r#"
            SELECT id, parent_id, name, spec, status, complexity, priority,
                   first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
            FROM tasks
            WHERE parent_id = ?
            ORDER BY priority ASC NULLS LAST, id ASC
            "#,
        )
        .bind(id)
        .fetch_all(self.pool)
        .await?;

        // Get blocking tasks (tasks that this task depends on)
        let blocking_tasks = sqlx::query_as::<_, Task>(
            r#"
            SELECT t.id, t.parent_id, t.name, t.spec, t.status, t.complexity, t.priority,
                   t.first_todo_at, t.first_doing_at, t.first_done_at, t.active_form, t.owner, t.metadata
            FROM tasks t
            JOIN dependencies d ON t.id = d.blocking_task_id
            WHERE d.blocked_task_id = ?
            ORDER BY t.priority ASC NULLS LAST, t.id ASC
            "#,
        )
        .bind(id)
        .fetch_all(self.pool)
        .await?;

        // Get blocked_by tasks (tasks that depend on this task)
        let blocked_by_tasks = sqlx::query_as::<_, Task>(
            r#"
            SELECT t.id, t.parent_id, t.name, t.spec, t.status, t.complexity, t.priority,
                   t.first_todo_at, t.first_doing_at, t.first_done_at, t.active_form, t.owner, t.metadata
            FROM tasks t
            JOIN dependencies d ON t.id = d.blocked_task_id
            WHERE d.blocking_task_id = ?
            ORDER BY t.priority ASC NULLS LAST, t.id ASC
            "#,
        )
        .bind(id)
        .fetch_all(self.pool)
        .await?;

        Ok(TaskContext {
            task,
            ancestors,
            siblings,
            children,
            dependencies: crate::db::models::TaskDependencies {
                blocking_tasks,
                blocked_by_tasks,
            },
        })
    }

    /// Get all descendants of a task recursively (children, grandchildren, etc.)
    /// Uses recursive CTE for efficient querying
    pub async fn get_descendants(&self, task_id: i64) -> Result<Vec<Task>> {
        let descendants = sqlx::query_as::<_, Task>(
            r#"
            WITH RECURSIVE descendants AS (
                SELECT id, parent_id, name, spec, status, complexity, priority,
                       first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
                FROM tasks
                WHERE parent_id = ?

                UNION ALL

                SELECT t.id, t.parent_id, t.name, t.spec, t.status, t.complexity, t.priority,
                       t.first_todo_at, t.first_doing_at, t.first_done_at, t.active_form, t.owner, t.metadata
                FROM tasks t
                INNER JOIN descendants d ON t.parent_id = d.id
            )
            SELECT * FROM descendants
            ORDER BY parent_id NULLS FIRST, priority ASC NULLS LAST, id ASC
            "#,
        )
        .bind(task_id)
        .fetch_all(self.pool)
        .await?;

        Ok(descendants)
    }

    /// Get status response for a task (the "spotlight" view)
    /// This is the main method for `ie status` command
    pub async fn get_status(
        &self,
        task_id: i64,
        with_events: bool,
    ) -> Result<crate::db::models::StatusResponse> {
        use crate::db::models::{StatusResponse, TaskBrief};

        // Get task context (reuse existing method)
        let context = self.get_task_context(task_id).await?;

        // Get all descendants recursively
        let descendants_full = self.get_descendants(task_id).await?;

        // Convert siblings and descendants to brief format
        let siblings: Vec<TaskBrief> = context.siblings.iter().map(TaskBrief::from).collect();
        let descendants: Vec<TaskBrief> = descendants_full.iter().map(TaskBrief::from).collect();

        // Get events if requested
        let events = if with_events {
            let event_mgr = crate::events::EventManager::new(self.pool);
            Some(
                event_mgr
                    .list_events(Some(task_id), Some(50), None, None)
                    .await?,
            )
        } else {
            None
        };

        Ok(StatusResponse {
            focused_task: context.task,
            ancestors: context.ancestors,
            siblings,
            descendants,
            events,
        })
    }

    /// Get root tasks (tasks with no parent) for NoFocusResponse
    pub async fn get_root_tasks(&self) -> Result<Vec<Task>> {
        let tasks = sqlx::query_as::<_, Task>(
            r#"
            SELECT id, parent_id, name, spec, status, complexity, priority,
                   first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
            FROM tasks
            WHERE parent_id IS NULL
            ORDER BY
                CASE status
                    WHEN 'doing' THEN 0
                    WHEN 'todo' THEN 1
                    WHEN 'done' THEN 2
                END,
                priority ASC NULLS LAST,
                id ASC
            "#,
        )
        .fetch_all(self.pool)
        .await?;

        Ok(tasks)
    }

    /// Get events summary for a task
    async fn get_events_summary(&self, task_id: i64) -> Result<EventsSummary> {
        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events WHERE task_id = ?")
            .bind(task_id)
            .fetch_one(self.pool)
            .await?;

        let recent_events = sqlx::query_as::<_, Event>(
            r#"
            SELECT id, task_id, timestamp, log_type, discussion_data
            FROM events
            WHERE task_id = ?
            ORDER BY timestamp DESC
            LIMIT 10
            "#,
        )
        .bind(task_id)
        .fetch_all(self.pool)
        .await?;

        Ok(EventsSummary {
            total_count,
            recent_events,
        })
    }

    /// Update a task
    pub async fn update_task(&self, id: i64, update: TaskUpdate<'_>) -> Result<Task> {
        let TaskUpdate {
            name,
            spec,
            parent_id,
            status,
            complexity,
            priority,
            active_form,
            owner,
            metadata,
        } = update;

        // Check task exists
        let task = self.get_task(id).await?;

        // Validate status if provided
        if let Some(s) = status {
            if !["todo", "doing", "done"].contains(&s) {
                return Err(IntentError::InvalidInput(format!("Invalid status: {}", s)));
            }
        }

        // Check for circular dependency if parent_id is being changed
        if let Some(Some(pid)) = parent_id {
            if pid == id {
                return Err(IntentError::CircularDependency {
                    blocking_task_id: pid,
                    blocked_task_id: id,
                });
            }
            self.check_task_exists(pid).await?;
            self.check_circular_dependency(id, pid).await?;
        }

        // Build dynamic update query using QueryBuilder for SQL injection safety
        let mut builder: sqlx::QueryBuilder<sqlx::Sqlite> =
            sqlx::QueryBuilder::new("UPDATE tasks SET ");
        let mut has_updates = false;

        if let Some(n) = name {
            if has_updates {
                builder.push(", ");
            }
            builder.push("name = ").push_bind(n);
            has_updates = true;
        }

        if let Some(s) = spec {
            if has_updates {
                builder.push(", ");
            }
            builder.push("spec = ").push_bind(s);
            has_updates = true;
        }

        if let Some(pid) = parent_id {
            if has_updates {
                builder.push(", ");
            }
            match pid {
                Some(p) => {
                    builder.push("parent_id = ").push_bind(p);
                },
                None => {
                    builder.push("parent_id = NULL");
                },
            }
            has_updates = true;
        }

        if let Some(c) = complexity {
            if has_updates {
                builder.push(", ");
            }
            builder.push("complexity = ").push_bind(c);
            has_updates = true;
        }

        if let Some(p) = priority {
            if has_updates {
                builder.push(", ");
            }
            builder.push("priority = ").push_bind(p);
            has_updates = true;
        }

        if let Some(af) = active_form {
            if has_updates {
                builder.push(", ");
            }
            builder.push("active_form = ").push_bind(af);
            has_updates = true;
        }

        if let Some(o) = owner {
            if o.is_empty() {
                return Err(IntentError::InvalidInput(
                    "owner cannot be empty".to_string(),
                ));
            }
            if has_updates {
                builder.push(", ");
            }
            builder.push("owner = ").push_bind(o);
            has_updates = true;
        }

        if let Some(m) = metadata {
            if has_updates {
                builder.push(", ");
            }
            builder.push("metadata = ").push_bind(m);
            has_updates = true;
        }

        if let Some(s) = status {
            if has_updates {
                builder.push(", ");
            }
            builder.push("status = ").push_bind(s);
            has_updates = true;

            // Update timestamp fields based on status
            let now = Utc::now();
            let timestamp = now.to_rfc3339();
            match s {
                "todo" if task.first_todo_at.is_none() => {
                    builder.push(", first_todo_at = ").push_bind(timestamp);
                },
                "doing" if task.first_doing_at.is_none() => {
                    builder.push(", first_doing_at = ").push_bind(timestamp);
                },
                "done" if task.first_done_at.is_none() => {
                    builder.push(", first_done_at = ").push_bind(timestamp);
                },
                _ => {},
            }
        }

        if !has_updates {
            return Ok(task);
        }

        builder.push(" WHERE id = ").push_bind(id);

        builder.build().execute(self.pool).await?;

        let task = self.get_task(id).await?;

        // Notify WebSocket clients about the task update
        self.notify_task_updated(&task).await;

        Ok(task)
    }

    /// Delete a task
    pub async fn delete_task(&self, id: i64) -> Result<()> {
        self.check_task_exists(id).await?;

        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;

        // Notify WebSocket clients about the task deletion
        self.notify_task_deleted(id).await;

        Ok(())
    }

    /// Find tasks with optional filters, sorting, and pagination
    pub async fn find_tasks(
        &self,
        status: Option<&str>,
        parent_id: Option<Option<i64>>,
        sort_by: Option<TaskSortBy>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PaginatedTasks> {
        // Apply defaults
        let sort_by = sort_by.unwrap_or_default(); // Default: FocusAware
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        // Resolve session_id for FocusAware sorting
        let session_id = crate::workspace::resolve_session_id(None);

        // Build WHERE clause
        let mut where_clause = String::from("WHERE 1=1");
        let mut conditions = Vec::new();

        if let Some(s) = status {
            where_clause.push_str(" AND status = ?");
            conditions.push(s.to_string());
        }

        if let Some(pid) = parent_id {
            if let Some(p) = pid {
                where_clause.push_str(" AND parent_id = ?");
                conditions.push(p.to_string());
            } else {
                where_clause.push_str(" AND parent_id IS NULL");
            }
        }

        // Track if FocusAware mode needs session_id bind
        let uses_session_bind = matches!(sort_by, TaskSortBy::FocusAware);

        // Build ORDER BY clause based on sort mode
        let order_clause = match sort_by {
            TaskSortBy::Id => {
                // Legacy: simple ORDER BY id ASC
                "ORDER BY id ASC".to_string()
            },
            TaskSortBy::Priority => {
                // ORDER BY priority ASC, complexity ASC, id ASC
                "ORDER BY COALESCE(priority, 999) ASC, COALESCE(complexity, 5) ASC, id ASC"
                    .to_string()
            },
            TaskSortBy::Time => {
                // ORDER BY timestamp based on status
                r#"ORDER BY
                    CASE status
                        WHEN 'doing' THEN first_doing_at
                        WHEN 'todo' THEN first_todo_at
                        WHEN 'done' THEN first_done_at
                    END ASC NULLS LAST,
                    id ASC"#
                    .to_string()
            },
            TaskSortBy::FocusAware => {
                // Focus-aware: current focused task → doing tasks → todo tasks
                r#"ORDER BY
                    CASE
                        WHEN t.id = (SELECT current_task_id FROM sessions WHERE session_id = ?) THEN 0
                        WHEN t.status = 'doing' THEN 1
                        WHEN t.status = 'todo' THEN 2
                        ELSE 3
                    END ASC,
                    COALESCE(t.priority, 999) ASC,
                    t.id ASC"#
                    .to_string()
            },
        };

        // Get total count
        let count_query = format!("SELECT COUNT(*) FROM tasks {}", where_clause);
        let mut count_q = sqlx::query_scalar::<_, i64>(&count_query);
        for cond in &conditions {
            count_q = count_q.bind(cond);
        }
        let total_count = count_q.fetch_one(self.pool).await?;

        // Build main query with pagination
        let main_query = format!(
            "SELECT id, parent_id, name, NULL as spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata FROM tasks t {} {} LIMIT ? OFFSET ?",
            where_clause, order_clause
        );

        let mut q = sqlx::query_as::<_, Task>(&main_query);
        for cond in conditions {
            q = q.bind(cond);
        }
        // Bind session_id for FocusAware ORDER BY clause
        if uses_session_bind {
            q = q.bind(&session_id);
        }
        q = q.bind(limit);
        q = q.bind(offset);

        let tasks = q.fetch_all(self.pool).await?;

        // Calculate has_more
        let has_more = offset + (tasks.len() as i64) < total_count;

        Ok(PaginatedTasks {
            tasks,
            total_count,
            has_more,
            limit,
            offset,
        })
    }

    /// Get workspace statistics using SQL aggregation (no data loading)
    ///
    /// This is much more efficient than loading all tasks just to count them.
    /// Used by session restore when there's no focused task.
    pub async fn get_stats(&self) -> Result<WorkspaceStats> {
        let row = sqlx::query_as::<_, (i64, i64, i64, i64)>(
            r#"SELECT
                COUNT(*) as total,
                COALESCE(SUM(CASE WHEN status = 'todo' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'doing' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END), 0)
            FROM tasks"#,
        )
        .fetch_one(self.pool)
        .await?;

        Ok(WorkspaceStats {
            total_tasks: row.0,
            todo: row.1,
            doing: row.2,
            done: row.3,
        })
    }

    /// Start a task (atomic: update status + set current)
    #[tracing::instrument(skip(self))]
    pub async fn start_task(&self, id: i64, with_events: bool) -> Result<TaskWithEvents> {
        // Check if task exists first
        let task_exists: bool =
            sqlx::query_scalar::<_, bool>(crate::sql_constants::CHECK_TASK_EXISTS)
                .bind(id)
                .fetch_one(self.pool)
                .await?;

        if !task_exists {
            return Err(IntentError::TaskNotFound(id));
        }

        // Check if task is blocked by incomplete dependencies
        use crate::dependencies::get_incomplete_blocking_tasks;
        if let Some(blocking_tasks) = get_incomplete_blocking_tasks(self.pool, id).await? {
            return Err(IntentError::TaskBlocked {
                task_id: id,
                blocking_task_ids: blocking_tasks,
            });
        }

        let mut tx = self.pool.begin().await?;

        let now = Utc::now();

        // Update task status to doing
        sqlx::query(
            r#"
            UPDATE tasks
            SET status = 'doing', first_doing_at = COALESCE(first_doing_at, ?)
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Set as current task in sessions table
        // Use session_id from environment if available
        let session_id = crate::workspace::resolve_session_id(None);
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
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        if with_events {
            let result = self.get_task_with_events(id).await?;
            self.notify_task_updated(&result.task).await;
            Ok(result)
        } else {
            let task = self.get_task(id).await?;
            self.notify_task_updated(&task).await;
            Ok(TaskWithEvents {
                task,
                events_summary: None,
            })
        }
    }

    /// Build a next-step suggestion after completing a task.
    ///
    /// Shared by `done_task` and `done_task_by_id` to avoid duplication.
    async fn build_next_step_suggestion(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        id: i64,
        task_name: &str,
        parent_id: Option<i64>,
    ) -> Result<NextStepSuggestion> {
        if let Some(parent_task_id) = parent_id {
            let remaining_siblings: i64 = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tasks WHERE parent_id = ? AND status != 'done' AND id != ?",
            )
            .bind(parent_task_id)
            .bind(id)
            .fetch_one(&mut **tx)
            .await?;

            let parent_name: String =
                sqlx::query_scalar::<_, String>(crate::sql_constants::SELECT_TASK_NAME)
                    .bind(parent_task_id)
                    .fetch_one(&mut **tx)
                    .await?;

            if remaining_siblings == 0 {
                Ok(NextStepSuggestion::ParentIsReady {
                    message: format!(
                        "All sub-tasks of parent #{} '{}' are now complete. The parent task is ready for your attention.",
                        parent_task_id, parent_name
                    ),
                    parent_task_id,
                    parent_task_name: parent_name,
                })
            } else {
                Ok(NextStepSuggestion::SiblingTasksRemain {
                    message: format!(
                        "Task #{} completed. Parent task #{} '{}' has other sub-tasks remaining.",
                        id, parent_task_id, parent_name
                    ),
                    parent_task_id,
                    parent_task_name: parent_name,
                    remaining_siblings_count: remaining_siblings,
                })
            }
        } else {
            let child_count: i64 =
                sqlx::query_scalar::<_, i64>(crate::sql_constants::COUNT_CHILDREN_TOTAL)
                    .bind(id)
                    .fetch_one(&mut **tx)
                    .await?;

            if child_count > 0 {
                Ok(NextStepSuggestion::TopLevelTaskCompleted {
                    message: format!(
                        "Top-level task #{} '{}' has been completed. Well done!",
                        id, task_name
                    ),
                    completed_task_id: id,
                    completed_task_name: task_name.to_string(),
                })
            } else {
                let remaining_tasks: i64 = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM tasks WHERE status != 'done' AND id != ?",
                )
                .bind(id)
                .fetch_one(&mut **tx)
                .await?;

                if remaining_tasks == 0 {
                    Ok(NextStepSuggestion::WorkspaceIsClear {
                        message: format!(
                            "Project complete! Task #{} was the last remaining task. There are no more 'todo' or 'doing' tasks.",
                            id
                        ),
                        completed_task_id: id,
                    })
                } else {
                    Ok(NextStepSuggestion::NoParentContext {
                        message: format!("Task #{} '{}' has been completed.", id, task_name),
                        completed_task_id: id,
                        completed_task_name: task_name.to_string(),
                    })
                }
            }
        }
    }

    /// Complete the current focused task (atomic: check children + update status + clear current)
    /// This command only operates on the current_task_id.
    /// Prerequisites: A task must be set as current
    ///
    /// # Arguments
    /// * `is_ai_caller` - Whether this is called from AI (MCP) or human (CLI/Dashboard).
    ///   When true and task is human-owned, the operation will fail.
    ///   Human tasks can only be completed via CLI or Dashboard.
    #[tracing::instrument(skip(self))]
    pub async fn done_task(&self, is_ai_caller: bool) -> Result<DoneTaskResponse> {
        let session_id = crate::workspace::resolve_session_id(None);
        let mut tx = self.pool.begin().await?;

        // Get the current task ID from sessions table
        let current_task_id: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT current_task_id FROM sessions WHERE session_id = ?",
        )
        .bind(&session_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();

        let id = current_task_id.ok_or(IntentError::InvalidInput(
            "No current task is set. Use 'current --set <ID>' to set a task first.".to_string(),
        ))?;

        // Get the task info before completing it (including owner)
        let task_info: (String, Option<i64>, String) =
            sqlx::query_as("SELECT name, parent_id, owner FROM tasks WHERE id = ?")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        let (task_name, parent_id, owner) = task_info;

        // Human Task Protection: AI cannot complete human-owned tasks
        // Human must complete their own tasks via CLI or Dashboard
        if owner == "human" && is_ai_caller {
            return Err(IntentError::HumanTaskCannotBeCompletedByAI {
                task_id: id,
                task_name: task_name.clone(),
            });
        }

        // Complete the task (validates children + updates status)
        self.complete_task_in_tx(&mut tx, id).await?;

        // Clear the current task in sessions table for this session
        sqlx::query("UPDATE sessions SET current_task_id = NULL, last_active_at = datetime('now') WHERE session_id = ?")
            .bind(&session_id)
            .execute(&mut *tx)
            .await?;

        let next_step_suggestion =
            Self::build_next_step_suggestion(&mut tx, id, &task_name, parent_id).await?;

        tx.commit().await?;

        // Fetch the completed task to notify UI
        let completed_task = self.get_task(id).await?;
        self.notify_task_updated(&completed_task).await;

        Ok(DoneTaskResponse {
            completed_task,
            workspace_status: WorkspaceStatus {
                current_task_id: None,
            },
            next_step_suggestion,
        })
    }

    /// Complete a task by its ID directly (without requiring it to be the current focus).
    ///
    /// Unlike `done_task` which only completes the currently focused task, this method
    /// completes a task by ID. If the task happens to be the current session's focus,
    /// the focus is cleared. Otherwise, the current focus is left unchanged.
    ///
    /// # Arguments
    /// * `id` - The task ID to complete
    /// * `is_ai_caller` - Whether this is called from AI. When true and task is human-owned, fails.
    #[tracing::instrument(skip(self))]
    pub async fn done_task_by_id(&self, id: i64, is_ai_caller: bool) -> Result<DoneTaskResponse> {
        let session_id = crate::workspace::resolve_session_id(None);
        let mut tx = self.pool.begin().await?;

        // Get the task info (name, parent_id, owner) by ID
        let task_info: (String, Option<i64>, String) =
            sqlx::query_as("SELECT name, parent_id, owner FROM tasks WHERE id = ?")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(IntentError::TaskNotFound(id))?;
        let (task_name, parent_id, owner) = task_info;

        // Human Task Protection: AI cannot complete human-owned tasks
        if owner == "human" && is_ai_caller {
            return Err(IntentError::HumanTaskCannotBeCompletedByAI {
                task_id: id,
                task_name: task_name.clone(),
            });
        }

        // Complete the task (validates children + updates status)
        self.complete_task_in_tx(&mut tx, id).await?;

        // If this task is the current session's focus, clear it (otherwise leave focus untouched)
        sqlx::query(
            "UPDATE sessions SET current_task_id = NULL, last_active_at = datetime('now') WHERE session_id = ? AND current_task_id = ?",
        )
        .bind(&session_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Read back the actual current_task_id (may still be set if we completed a non-focused task)
        let actual_current_task_id: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT current_task_id FROM sessions WHERE session_id = ?",
        )
        .bind(&session_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();

        let next_step_suggestion =
            Self::build_next_step_suggestion(&mut tx, id, &task_name, parent_id).await?;

        // LLM Synthesis: Generate updated task description from events (if configured)
        let synthesis_result = self.try_synthesize_task_description(id, &task_name).await;

        tx.commit().await?;

        // Fetch the completed task to notify UI
        let mut completed_task = self.get_task(id).await?;

        // Apply synthesis if available and appropriate (respects owner field)
        if let Ok(Some(new_spec)) = synthesis_result {
            completed_task = self
                .apply_synthesis_if_appropriate(completed_task, &new_spec, &owner)
                .await?;
        }

        self.notify_task_updated(&completed_task).await;

        Ok(DoneTaskResponse {
            completed_task,
            workspace_status: WorkspaceStatus {
                current_task_id: actual_current_task_id,
            },
            next_step_suggestion,
        })
    }

    /// Check if a task exists
    async fn check_task_exists(&self, id: i64) -> Result<()> {
        let exists: bool = sqlx::query_scalar::<_, bool>(crate::sql_constants::CHECK_TASK_EXISTS)
            .bind(id)
            .fetch_one(self.pool)
            .await?;

        if !exists {
            return Err(IntentError::TaskNotFound(id));
        }

        Ok(())
    }

    /// Check for circular dependencies
    async fn check_circular_dependency(&self, task_id: i64, new_parent_id: i64) -> Result<()> {
        let mut current_id = new_parent_id;

        loop {
            if current_id == task_id {
                return Err(IntentError::CircularDependency {
                    blocking_task_id: new_parent_id,
                    blocked_task_id: task_id,
                });
            }

            let parent: Option<i64> =
                sqlx::query_scalar::<_, Option<i64>>(crate::sql_constants::SELECT_TASK_PARENT_ID)
                    .bind(current_id)
                    .fetch_optional(self.pool)
                    .await?
                    .flatten();

            match parent {
                Some(pid) => current_id = pid,
                None => break,
            }
        }

        Ok(())
    }
    /// Create a subtask under the current task and switch to it (atomic operation)
    /// Returns error if there is no current task
    /// Returns response with subtask info and parent task info
    pub async fn spawn_subtask(
        &self,
        name: &str,
        spec: Option<&str>,
    ) -> Result<SpawnSubtaskResponse> {
        // Get current task from sessions table for this session
        let session_id = crate::workspace::resolve_session_id(None);
        let current_task_id: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT current_task_id FROM sessions WHERE session_id = ?",
        )
        .bind(&session_id)
        .fetch_optional(self.pool)
        .await?
        .flatten();

        let parent_id = current_task_id.ok_or(IntentError::InvalidInput(
            "No current task to create subtask under".to_string(),
        ))?;

        // Get parent task info
        let parent_name: String =
            sqlx::query_scalar::<_, String>(crate::sql_constants::SELECT_TASK_NAME)
                .bind(parent_id)
                .fetch_one(self.pool)
                .await?;

        // Create the subtask with AI ownership (CLI operation)
        let subtask = self
            .add_task(name, spec, Some(parent_id), Some("ai"))
            .await?;

        // Start the new subtask (sets status to doing and updates current_task_id)
        // This keeps the parent task in 'doing' status (multi-doing design)
        self.start_task(subtask.id, false).await?;

        Ok(SpawnSubtaskResponse {
            subtask: SubtaskInfo {
                id: subtask.id,
                name: subtask.name,
                parent_id,
                status: "doing".to_string(),
            },
            parent_task: ParentTaskInfo {
                id: parent_id,
                name: parent_name,
            },
        })
    }

    /// Intelligently pick tasks from 'todo' and transition them to 'doing'
    /// Returns tasks that were successfully transitioned
    ///
    /// # Arguments
    /// * `max_count` - Maximum number of tasks to pick
    /// * `capacity_limit` - Maximum total number of tasks allowed in 'doing' status
    ///
    /// # Logic
    /// 1. Check current 'doing' task count
    /// 2. Calculate available capacity
    /// 3. Select tasks from 'todo' (prioritized by: priority DESC, complexity ASC)
    /// 4. Transition selected tasks to 'doing'
    pub async fn pick_next_tasks(
        &self,
        max_count: usize,
        capacity_limit: usize,
    ) -> Result<Vec<Task>> {
        let mut tx = self.pool.begin().await?;

        // Get current doing count
        let doing_count: i64 =
            sqlx::query_scalar::<_, i64>(crate::sql_constants::COUNT_TASKS_DOING)
                .fetch_one(&mut *tx)
                .await?;

        // Calculate available capacity
        let available = capacity_limit.saturating_sub(doing_count as usize);
        if available == 0 {
            return Ok(vec![]);
        }

        let limit = std::cmp::min(max_count, available);

        // Select tasks from todo, prioritizing by priority DESC, complexity ASC
        let todo_tasks = sqlx::query_as::<_, Task>(
            r#"
                        SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
                        FROM tasks
                        WHERE status = 'todo'
                        ORDER BY
                            COALESCE(priority, 0) ASC,
                            COALESCE(complexity, 5) ASC,
                            id ASC
                        LIMIT ?
                        "#,
        )
        .bind(limit as i64)
        .fetch_all(&mut *tx)
        .await?;

        if todo_tasks.is_empty() {
            return Ok(vec![]);
        }

        let now = Utc::now();

        // Transition selected tasks to 'doing'
        for task in &todo_tasks {
            sqlx::query(
                r#"
                UPDATE tasks
                SET status = 'doing',
                    first_doing_at = COALESCE(first_doing_at, ?)
                WHERE id = ?
                "#,
            )
            .bind(now)
            .bind(task.id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // Fetch and return updated tasks in the same order
        let task_ids: Vec<i64> = todo_tasks.iter().map(|t| t.id).collect();
        let placeholders = vec!["?"; task_ids.len()].join(",");
        let query = format!(
            "SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
                         FROM tasks WHERE id IN ({})
                         ORDER BY
                             COALESCE(priority, 0) ASC,
                             COALESCE(complexity, 5) ASC,
                             id ASC",
            placeholders
        );

        let mut q = sqlx::query_as::<_, Task>(&query);
        for id in task_ids {
            q = q.bind(id);
        }

        let updated_tasks = q.fetch_all(self.pool).await?;
        Ok(updated_tasks)
    }

    /// Intelligently recommend the next task to work on based on context-aware priority model.
    ///
    /// Priority logic:
    /// 1. First priority: Subtasks of the current focused task (depth-first)
    /// 2. Second priority: Top-level tasks (breadth-first)
    /// 3. No recommendation: Return appropriate empty state
    ///
    /// This command does NOT modify task status.
    pub async fn pick_next(&self) -> Result<PickNextResponse> {
        // Step 1: Check if there's a current focused task for this session
        let session_id = crate::workspace::resolve_session_id(None);
        let current_task_id: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT current_task_id FROM sessions WHERE session_id = ?",
        )
        .bind(&session_id)
        .fetch_optional(self.pool)
        .await?
        .flatten();

        if let Some(current_id) = current_task_id {
            // Step 1a: First priority - Get **doing** subtasks of current focused task
            // Exclude tasks blocked by incomplete dependencies
            let doing_subtasks = sqlx::query_as::<_, Task>(
                r#"
                        SELECT id, parent_id, name, spec, status, complexity, priority,
                               first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
                        FROM tasks
                        WHERE parent_id = ? AND status = 'doing'
                          AND NOT EXISTS (
                            SELECT 1 FROM dependencies d
                            JOIN tasks bt ON d.blocking_task_id = bt.id
                            WHERE d.blocked_task_id = tasks.id
                              AND bt.status != 'done'
                          )
                        ORDER BY COALESCE(priority, 999999) ASC, id ASC
                        LIMIT 1
                        "#,
            )
            .bind(current_id)
            .fetch_optional(self.pool)
            .await?;

            if let Some(task) = doing_subtasks {
                return Ok(PickNextResponse::focused_subtask(task));
            }

            // Step 1b: Second priority - Get **todo** subtasks if no doing subtasks
            let todo_subtasks = sqlx::query_as::<_, Task>(
                r#"
                            SELECT id, parent_id, name, spec, status, complexity, priority,
                                   first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
                            FROM tasks
                            WHERE parent_id = ? AND status = 'todo'
                              AND NOT EXISTS (
                                SELECT 1 FROM dependencies d
                                JOIN tasks bt ON d.blocking_task_id = bt.id
                                WHERE d.blocked_task_id = tasks.id
                                  AND bt.status != 'done'
                              )
                            ORDER BY COALESCE(priority, 999999) ASC, id ASC
                            LIMIT 1
                            "#,
            )
            .bind(current_id)
            .fetch_optional(self.pool)
            .await?;

            if let Some(task) = todo_subtasks {
                return Ok(PickNextResponse::focused_subtask(task));
            }
        }

        // Step 2a: Third priority - Get top-level **doing** tasks (excluding current task)
        // Exclude tasks blocked by incomplete dependencies
        let doing_top_level = if let Some(current_id) = current_task_id {
            sqlx::query_as::<_, Task>(
                r#"
                SELECT id, parent_id, name, spec, status, complexity, priority,
                       first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
                FROM tasks
                WHERE parent_id IS NULL AND status = 'doing' AND id != ?
                  AND NOT EXISTS (
                    SELECT 1 FROM dependencies d
                    JOIN tasks bt ON d.blocking_task_id = bt.id
                    WHERE d.blocked_task_id = tasks.id
                      AND bt.status != 'done'
                  )
                ORDER BY COALESCE(priority, 999999) ASC, id ASC
                LIMIT 1
                "#,
            )
            .bind(current_id)
            .fetch_optional(self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Task>(
                r#"
                SELECT id, parent_id, name, spec, status, complexity, priority,
                       first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
                FROM tasks
                WHERE parent_id IS NULL AND status = 'doing'
                  AND NOT EXISTS (
                    SELECT 1 FROM dependencies d
                    JOIN tasks bt ON d.blocking_task_id = bt.id
                    WHERE d.blocked_task_id = tasks.id
                      AND bt.status != 'done'
                  )
                ORDER BY COALESCE(priority, 999999) ASC, id ASC
                LIMIT 1
                "#,
            )
            .fetch_optional(self.pool)
            .await?
        };

        if let Some(task) = doing_top_level {
            return Ok(PickNextResponse::top_level_task(task));
        }

        // Step 2b: Fourth priority - Get top-level **todo** tasks
        // Exclude tasks blocked by incomplete dependencies
        let todo_top_level = sqlx::query_as::<_, Task>(
            r#"
            SELECT id, parent_id, name, spec, status, complexity, priority,
                   first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
            FROM tasks
            WHERE parent_id IS NULL AND status = 'todo'
              AND NOT EXISTS (
                SELECT 1 FROM dependencies d
                JOIN tasks bt ON d.blocking_task_id = bt.id
                WHERE d.blocked_task_id = tasks.id
                  AND bt.status != 'done'
              )
            ORDER BY COALESCE(priority, 999999) ASC, id ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(self.pool)
        .await?;

        if let Some(task) = todo_top_level {
            return Ok(PickNextResponse::top_level_task(task));
        }

        // Step 3: No recommendation - determine why
        // Check if there are any tasks at all
        let total_tasks: i64 =
            sqlx::query_scalar::<_, i64>(crate::sql_constants::COUNT_TASKS_TOTAL)
                .fetch_one(self.pool)
                .await?;

        if total_tasks == 0 {
            return Ok(PickNextResponse::no_tasks_in_project());
        }

        // Check if all tasks are completed
        let todo_or_doing_count: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tasks WHERE status IN ('todo', 'doing')",
        )
        .fetch_one(self.pool)
        .await?;

        if todo_or_doing_count == 0 {
            return Ok(PickNextResponse::all_tasks_completed());
        }

        // Otherwise, there are tasks but none available based on current context
        Ok(PickNextResponse::no_available_todos())
    }

    /// Try to synthesize task description using LLM
    ///
    /// Returns Ok(None) if LLM is not configured (graceful degradation)
    /// Returns Ok(Some(synthesis)) if successful
    /// Returns Err only on critical failures
    async fn try_synthesize_task_description(
        &self,
        task_id: i64,
        task_name: &str,
    ) -> Result<Option<String>> {
        // Get task spec and events
        let task = self.get_task(task_id).await?;
        let events = crate::events::EventManager::new(self.pool)
            .list_events(Some(task_id), None, None, None)
            .await?;

        // Call LLM synthesis (returns None if not configured)
        match crate::llm::synthesize_task_description(
            self.pool,
            task_name,
            task.spec.as_deref(),
            &events,
        )
        .await
        {
            Ok(synthesis) => Ok(synthesis),
            Err(e) => {
                // Log error but don't fail the task completion
                tracing::warn!("LLM synthesis failed: {}", e);
                Ok(None)
            },
        }
    }

    /// Apply LLM synthesis to task based on owner field
    ///
    /// - AI-owned tasks: auto-apply
    /// - Human-owned tasks: prompt for approval (currently just logs and skips)
    async fn apply_synthesis_if_appropriate(
        &self,
        task: Task,
        new_spec: &str,
        owner: &str,
    ) -> Result<Task> {
        if owner == "ai" {
            // AI-owned task: auto-apply synthesis
            tracing::info!("Auto-applying LLM synthesis for AI-owned task #{}", task.id);

            let updated = self
                .update_task(
                    task.id,
                    TaskUpdate {
                        spec: Some(new_spec),
                        ..Default::default()
                    },
                )
                .await?;

            Ok(updated)
        } else {
            // Human-owned task: would prompt user, but for CLI we just log
            // TODO: Implement interactive prompt for human tasks
            tracing::info!(
                "LLM synthesis available for human-owned task #{}, but auto-apply disabled. \
                 User would be prompted in interactive mode.",
                task.id
            );
            eprintln!("\n💡 LLM generated a task summary:");
            eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            eprintln!("{}", new_spec);
            eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            eprintln!("(Auto-apply disabled for human-owned tasks)");
            eprintln!(
                "To apply manually: ie task update {} --description \"<new spec>\"",
                task.id
            );

            Ok(task) // Return unchanged
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventManager;
    use crate::test_utils::test_helpers::TestContext;
    use crate::workspace::WorkspaceManager;

    #[tokio::test]
    async fn test_get_stats_empty() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let stats = manager.get_stats().await.unwrap();

        assert_eq!(stats.total_tasks, 0);
        assert_eq!(stats.todo, 0);
        assert_eq!(stats.doing, 0);
        assert_eq!(stats.done, 0);
    }

    #[tokio::test]
    async fn test_get_stats_with_tasks() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create tasks with different statuses
        let task1 = manager.add_task("Task 1", None, None, None).await.unwrap();
        let task2 = manager.add_task("Task 2", None, None, None).await.unwrap();
        let _task3 = manager.add_task("Task 3", None, None, None).await.unwrap();

        // Update statuses
        manager
            .update_task(
                task1.id,
                TaskUpdate {
                    status: Some("doing"),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        manager
            .update_task(
                task2.id,
                TaskUpdate {
                    status: Some("done"),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        // task3 stays as todo

        let stats = manager.get_stats().await.unwrap();

        assert_eq!(stats.total_tasks, 3);
        assert_eq!(stats.todo, 1);
        assert_eq!(stats.doing, 1);
        assert_eq!(stats.done, 1);
    }

    #[tokio::test]
    async fn test_add_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager
            .add_task("Test task", None, None, None)
            .await
            .unwrap();

        assert_eq!(task.name, "Test task");
        assert_eq!(task.status, "todo");
        assert!(task.first_todo_at.is_some());
        assert!(task.first_doing_at.is_none());
        assert!(task.first_done_at.is_none());
    }

    #[tokio::test]
    async fn test_add_task_with_spec() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let spec = "This is a task specification";
        let task = manager
            .add_task("Test task", Some(spec), None, None)
            .await
            .unwrap();

        assert_eq!(task.name, "Test task");
        assert_eq!(task.spec.as_deref(), Some(spec));
    }

    #[tokio::test]
    async fn test_add_task_with_parent() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let parent = manager
            .add_task("Parent task", None, None, None)
            .await
            .unwrap();
        let child = manager
            .add_task("Child task", None, Some(parent.id), None)
            .await
            .unwrap();

        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[tokio::test]
    async fn test_get_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let created = manager
            .add_task("Test task", None, None, None)
            .await
            .unwrap();
        let retrieved = manager.get_task(created.id).await.unwrap();

        assert_eq!(created.id, retrieved.id);
        assert_eq!(created.name, retrieved.name);
    }

    #[tokio::test]
    async fn test_get_task_not_found() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let result = manager.get_task(999).await;
        assert!(matches!(result, Err(IntentError::TaskNotFound(999))));
    }

    #[tokio::test]
    async fn test_update_task_name() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager
            .add_task("Original name", None, None, None)
            .await
            .unwrap();
        let updated = manager
            .update_task(
                task.id,
                TaskUpdate {
                    name: Some("New name"),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.name, "New name");
    }

    #[tokio::test]
    async fn test_update_task_status() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager
            .add_task("Test task", None, None, None)
            .await
            .unwrap();
        let updated = manager
            .update_task(
                task.id,
                TaskUpdate {
                    status: Some("doing"),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.status, "doing");
        assert!(updated.first_doing_at.is_some());
    }

    #[tokio::test]
    async fn test_delete_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager
            .add_task("Test task", None, None, None)
            .await
            .unwrap();
        manager.delete_task(task.id).await.unwrap();

        let result = manager.get_task(task.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_tasks_by_status() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        manager
            .add_task("Todo task", None, None, None)
            .await
            .unwrap();
        let doing_task = manager
            .add_task("Doing task", None, None, None)
            .await
            .unwrap();
        manager
            .update_task(
                doing_task.id,
                TaskUpdate {
                    status: Some("doing"),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let todo_result = manager
            .find_tasks(Some("todo"), None, None, None, None)
            .await
            .unwrap();
        let doing_result = manager
            .find_tasks(Some("doing"), None, None, None, None)
            .await
            .unwrap();

        assert_eq!(todo_result.tasks.len(), 1);
        assert_eq!(doing_result.tasks.len(), 1);
        assert_eq!(doing_result.tasks[0].status, "doing");
    }

    #[tokio::test]
    async fn test_find_tasks_by_parent() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let parent = manager.add_task("Parent", None, None, None).await.unwrap();
        manager
            .add_task("Child 1", None, Some(parent.id), None)
            .await
            .unwrap();
        manager
            .add_task("Child 2", None, Some(parent.id), None)
            .await
            .unwrap();

        let result = manager
            .find_tasks(None, Some(Some(parent.id)), None, None, None)
            .await
            .unwrap();

        assert_eq!(result.tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_start_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager
            .add_task("Test task", None, None, None)
            .await
            .unwrap();
        let started = manager.start_task(task.id, false).await.unwrap();

        assert_eq!(started.task.status, "doing");
        assert!(started.task.first_doing_at.is_some());

        // Verify it's set as current task
        let session_id = crate::workspace::resolve_session_id(None);
        let current: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT current_task_id FROM sessions WHERE session_id = ?",
        )
        .bind(&session_id)
        .fetch_optional(ctx.pool())
        .await
        .unwrap()
        .flatten();

        assert_eq!(current, Some(task.id));
    }

    #[tokio::test]
    async fn test_start_task_with_events() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager
            .add_task("Test task", None, None, None)
            .await
            .unwrap();

        // Add an event
        sqlx::query("INSERT INTO events (task_id, log_type, discussion_data) VALUES (?, ?, ?)")
            .bind(task.id)
            .bind("test")
            .bind("test event")
            .execute(ctx.pool())
            .await
            .unwrap();

        let started = manager.start_task(task.id, true).await.unwrap();

        assert!(started.events_summary.is_some());
        let summary = started.events_summary.unwrap();
        assert_eq!(summary.total_count, 1);
    }

    #[tokio::test]
    async fn test_done_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager
            .add_task("Test task", None, None, None)
            .await
            .unwrap();
        manager.start_task(task.id, false).await.unwrap();
        let response = manager.done_task(false).await.unwrap();

        assert_eq!(response.completed_task.status, "done");
        assert!(response.completed_task.first_done_at.is_some());
        assert_eq!(response.workspace_status.current_task_id, None);

        // Should be WORKSPACE_IS_CLEAR since it's the only task
        match response.next_step_suggestion {
            NextStepSuggestion::WorkspaceIsClear { .. } => {},
            _ => panic!("Expected WorkspaceIsClear suggestion"),
        }

        // Verify current task is cleared
        let session_id = crate::workspace::resolve_session_id(None);
        let current: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT current_task_id FROM sessions WHERE session_id = ?",
        )
        .bind(&session_id)
        .fetch_optional(ctx.pool())
        .await
        .unwrap()
        .flatten();

        assert!(current.is_none());
    }

    #[tokio::test]
    async fn test_done_task_with_uncompleted_children() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let parent = manager.add_task("Parent", None, None, None).await.unwrap();
        manager
            .add_task("Child", None, Some(parent.id), None)
            .await
            .unwrap();

        // Set parent as current task
        manager.start_task(parent.id, false).await.unwrap();

        let result = manager.done_task(false).await;
        assert!(matches!(result, Err(IntentError::UncompletedChildren)));
    }

    #[tokio::test]
    async fn test_done_task_with_completed_children() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let parent = manager.add_task("Parent", None, None, None).await.unwrap();
        let child = manager
            .add_task("Child", None, Some(parent.id), None)
            .await
            .unwrap();

        // Complete child first
        manager.start_task(child.id, false).await.unwrap();
        let child_response = manager.done_task(false).await.unwrap();

        // Child completion should suggest parent is ready
        match child_response.next_step_suggestion {
            NextStepSuggestion::ParentIsReady { parent_task_id, .. } => {
                assert_eq!(parent_task_id, parent.id);
            },
            _ => panic!("Expected ParentIsReady suggestion"),
        }

        // Now parent can be completed
        manager.start_task(parent.id, false).await.unwrap();
        let parent_response = manager.done_task(false).await.unwrap();
        assert_eq!(parent_response.completed_task.status, "done");

        // Parent completion should indicate top-level task completed (since it had children)
        match parent_response.next_step_suggestion {
            NextStepSuggestion::TopLevelTaskCompleted { .. } => {},
            _ => panic!("Expected TopLevelTaskCompleted suggestion"),
        }
    }

    #[tokio::test]
    async fn test_circular_dependency() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task1 = manager.add_task("Task 1", None, None, None).await.unwrap();
        let task2 = manager
            .add_task("Task 2", None, Some(task1.id), None)
            .await
            .unwrap();

        // Try to make task1 a child of task2 (circular)
        let result = manager
            .update_task(
                task1.id,
                TaskUpdate {
                    parent_id: Some(Some(task2.id)),
                    ..Default::default()
                },
            )
            .await;

        assert!(matches!(
            result,
            Err(IntentError::CircularDependency { .. })
        ));
    }

    #[tokio::test]
    async fn test_invalid_parent_id() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let result = manager.add_task("Test", None, Some(999), None).await;
        assert!(matches!(result, Err(IntentError::TaskNotFound(999))));
    }

    #[tokio::test]
    async fn test_update_task_complexity_and_priority() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager
            .add_task("Test task", None, None, None)
            .await
            .unwrap();
        let updated = manager
            .update_task(
                task.id,
                TaskUpdate {
                    complexity: Some(8),
                    priority: Some(10),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.complexity, Some(8));
        assert_eq!(updated.priority, Some(10));
    }

    #[tokio::test]
    async fn test_spawn_subtask() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create and start a parent task
        let parent = manager
            .add_task("Parent task", None, None, None)
            .await
            .unwrap();
        manager.start_task(parent.id, false).await.unwrap();

        // Spawn a subtask
        let response = manager
            .spawn_subtask("Child task", Some("Details"))
            .await
            .unwrap();

        assert_eq!(response.subtask.parent_id, parent.id);
        assert_eq!(response.subtask.name, "Child task");
        assert_eq!(response.subtask.status, "doing");
        assert_eq!(response.parent_task.id, parent.id);
        assert_eq!(response.parent_task.name, "Parent task");

        // Verify subtask is now the current task
        let session_id = crate::workspace::resolve_session_id(None);
        let current: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT current_task_id FROM sessions WHERE session_id = ?",
        )
        .bind(&session_id)
        .fetch_optional(ctx.pool())
        .await
        .unwrap()
        .flatten();

        assert_eq!(current, Some(response.subtask.id));

        // Verify subtask is in doing status
        let retrieved = manager.get_task(response.subtask.id).await.unwrap();
        assert_eq!(retrieved.status, "doing");
    }

    #[tokio::test]
    async fn test_spawn_subtask_no_current_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Try to spawn subtask without a current task
        let result = manager.spawn_subtask("Child", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pick_next_tasks_basic() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create 10 todo tasks
        for i in 1..=10 {
            manager
                .add_task(&format!("Task {}", i), None, None, None)
                .await
                .unwrap();
        }

        // Pick 5 tasks with capacity limit of 5
        let picked = manager.pick_next_tasks(5, 5).await.unwrap();

        assert_eq!(picked.len(), 5);
        for task in &picked {
            assert_eq!(task.status, "doing");
            assert!(task.first_doing_at.is_some());
        }

        // Verify total doing count
        let doing_count: i64 =
            sqlx::query_scalar::<_, i64>(crate::sql_constants::COUNT_TASKS_DOING)
                .fetch_one(ctx.pool())
                .await
                .unwrap();

        assert_eq!(doing_count, 5);
    }

    #[tokio::test]
    async fn test_pick_next_tasks_with_existing_doing() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create 10 todo tasks
        for i in 1..=10 {
            manager
                .add_task(&format!("Task {}", i), None, None, None)
                .await
                .unwrap();
        }

        // Start 2 tasks
        let result = manager
            .find_tasks(Some("todo"), None, None, None, None)
            .await
            .unwrap();
        manager.start_task(result.tasks[0].id, false).await.unwrap();
        manager.start_task(result.tasks[1].id, false).await.unwrap();

        // Pick more tasks with capacity limit of 5
        let picked = manager.pick_next_tasks(10, 5).await.unwrap();

        // Should only pick 3 more (5 - 2 = 3)
        assert_eq!(picked.len(), 3);

        // Verify total doing count
        let doing_count: i64 =
            sqlx::query_scalar::<_, i64>(crate::sql_constants::COUNT_TASKS_DOING)
                .fetch_one(ctx.pool())
                .await
                .unwrap();

        assert_eq!(doing_count, 5);
    }

    #[tokio::test]
    async fn test_pick_next_tasks_at_capacity() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create 10 tasks
        for i in 1..=10 {
            manager
                .add_task(&format!("Task {}", i), None, None, None)
                .await
                .unwrap();
        }

        // Fill capacity
        let first_batch = manager.pick_next_tasks(5, 5).await.unwrap();
        assert_eq!(first_batch.len(), 5);

        // Try to pick more (should return empty)
        let second_batch = manager.pick_next_tasks(5, 5).await.unwrap();
        assert_eq!(second_batch.len(), 0);
    }

    #[tokio::test]
    async fn test_pick_next_tasks_priority_ordering() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create tasks with different priorities
        let low = manager
            .add_task("Low priority", None, None, None)
            .await
            .unwrap();
        manager
            .update_task(
                low.id,
                TaskUpdate {
                    priority: Some(1),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let high = manager
            .add_task("High priority", None, None, None)
            .await
            .unwrap();
        manager
            .update_task(
                high.id,
                TaskUpdate {
                    priority: Some(10),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let medium = manager
            .add_task("Medium priority", None, None, None)
            .await
            .unwrap();
        manager
            .update_task(
                medium.id,
                TaskUpdate {
                    priority: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Pick tasks
        let picked = manager.pick_next_tasks(3, 5).await.unwrap();

        // Should be ordered by priority ASC (lower number = higher priority)
        assert_eq!(picked.len(), 3);
        assert_eq!(picked[0].priority, Some(1)); // lowest number = highest priority
        assert_eq!(picked[1].priority, Some(5)); // medium
        assert_eq!(picked[2].priority, Some(10)); // highest number = lowest priority
    }

    #[tokio::test]
    async fn test_pick_next_tasks_complexity_ordering() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create tasks with different complexities (same priority)
        let complex = manager.add_task("Complex", None, None, None).await.unwrap();
        manager
            .update_task(
                complex.id,
                TaskUpdate {
                    complexity: Some(9),
                    priority: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let simple = manager.add_task("Simple", None, None, None).await.unwrap();
        manager
            .update_task(
                simple.id,
                TaskUpdate {
                    complexity: Some(1),
                    priority: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let medium = manager.add_task("Medium", None, None, None).await.unwrap();
        manager
            .update_task(
                medium.id,
                TaskUpdate {
                    complexity: Some(5),
                    priority: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Pick tasks
        let picked = manager.pick_next_tasks(3, 5).await.unwrap();

        // Should be ordered by complexity ASC (simple first)
        assert_eq!(picked.len(), 3);
        assert_eq!(picked[0].complexity, Some(1)); // simple
        assert_eq!(picked[1].complexity, Some(5)); // medium
        assert_eq!(picked[2].complexity, Some(9)); // complex
    }

    #[tokio::test]
    async fn test_done_task_sibling_tasks_remain() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create parent with multiple children
        let parent = manager
            .add_task("Parent Task", None, None, None)
            .await
            .unwrap();
        let child1 = manager
            .add_task("Child 1", None, Some(parent.id), None)
            .await
            .unwrap();
        let child2 = manager
            .add_task("Child 2", None, Some(parent.id), None)
            .await
            .unwrap();
        let _child3 = manager
            .add_task("Child 3", None, Some(parent.id), None)
            .await
            .unwrap();

        // Complete first child
        manager.start_task(child1.id, false).await.unwrap();
        let response = manager.done_task(false).await.unwrap();

        // Should indicate siblings remain
        match response.next_step_suggestion {
            NextStepSuggestion::SiblingTasksRemain {
                parent_task_id,
                remaining_siblings_count,
                ..
            } => {
                assert_eq!(parent_task_id, parent.id);
                assert_eq!(remaining_siblings_count, 2); // child2 and child3
            },
            _ => panic!("Expected SiblingTasksRemain suggestion"),
        }

        // Complete second child
        manager.start_task(child2.id, false).await.unwrap();
        let response2 = manager.done_task(false).await.unwrap();

        // Should still indicate siblings remain
        match response2.next_step_suggestion {
            NextStepSuggestion::SiblingTasksRemain {
                remaining_siblings_count,
                ..
            } => {
                assert_eq!(remaining_siblings_count, 1); // only child3
            },
            _ => panic!("Expected SiblingTasksRemain suggestion"),
        }
    }

    #[tokio::test]
    async fn test_done_task_top_level_with_children() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create top-level task with children
        let parent = manager
            .add_task("Epic Task", None, None, None)
            .await
            .unwrap();
        let child = manager
            .add_task("Sub Task", None, Some(parent.id), None)
            .await
            .unwrap();

        // Complete child first
        manager.start_task(child.id, false).await.unwrap();
        manager.done_task(false).await.unwrap();

        // Complete parent
        manager.start_task(parent.id, false).await.unwrap();
        let response = manager.done_task(false).await.unwrap();

        // Should be TOP_LEVEL_TASK_COMPLETED
        match response.next_step_suggestion {
            NextStepSuggestion::TopLevelTaskCompleted {
                completed_task_id,
                completed_task_name,
                ..
            } => {
                assert_eq!(completed_task_id, parent.id);
                assert_eq!(completed_task_name, "Epic Task");
            },
            _ => panic!("Expected TopLevelTaskCompleted suggestion"),
        }
    }

    #[tokio::test]
    async fn test_done_task_no_parent_context() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create multiple standalone tasks
        let task1 = manager
            .add_task("Standalone Task 1", None, None, None)
            .await
            .unwrap();
        let _task2 = manager
            .add_task("Standalone Task 2", None, None, None)
            .await
            .unwrap();

        // Complete first task
        manager.start_task(task1.id, false).await.unwrap();
        let response = manager.done_task(false).await.unwrap();

        // Should be NO_PARENT_CONTEXT since task2 is still pending
        match response.next_step_suggestion {
            NextStepSuggestion::NoParentContext {
                completed_task_id,
                completed_task_name,
                ..
            } => {
                assert_eq!(completed_task_id, task1.id);
                assert_eq!(completed_task_name, "Standalone Task 1");
            },
            _ => panic!("Expected NoParentContext suggestion"),
        }
    }

    // =========================================================================
    // done_task_by_id tests
    // =========================================================================

    #[tokio::test]
    async fn test_done_task_by_id_non_focused_task_preserves_focus() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create two tasks, focus on task_a
        let task_a = manager.add_task("Task A", None, None, None).await.unwrap();
        let task_b = manager.add_task("Task B", None, None, None).await.unwrap();
        manager.start_task(task_a.id, false).await.unwrap();

        // Complete task_b by ID (not the focused task)
        let response = manager.done_task_by_id(task_b.id, false).await.unwrap();

        // task_b should be done
        assert_eq!(response.completed_task.status, "done");
        assert_eq!(response.completed_task.id, task_b.id);

        // Focus should still be on task_a
        assert_eq!(response.workspace_status.current_task_id, Some(task_a.id));

        // Verify via direct DB query
        let session_id = crate::workspace::resolve_session_id(None);
        let current: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT current_task_id FROM sessions WHERE session_id = ?",
        )
        .bind(&session_id)
        .fetch_optional(ctx.pool())
        .await
        .unwrap()
        .flatten();
        assert_eq!(current, Some(task_a.id));
    }

    #[tokio::test]
    async fn test_done_task_by_id_focused_task_clears_focus() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager
            .add_task("Focused task", None, None, None)
            .await
            .unwrap();
        manager.start_task(task.id, false).await.unwrap();

        // Complete the focused task by ID
        let response = manager.done_task_by_id(task.id, false).await.unwrap();

        assert_eq!(response.completed_task.status, "done");
        assert_eq!(response.workspace_status.current_task_id, None);

        // Verify via direct DB query
        let session_id = crate::workspace::resolve_session_id(None);
        let current: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT current_task_id FROM sessions WHERE session_id = ?",
        )
        .bind(&session_id)
        .fetch_optional(ctx.pool())
        .await
        .unwrap()
        .flatten();
        assert!(current.is_none());
    }

    #[tokio::test]
    async fn test_done_task_by_id_human_task_rejected_for_ai_caller() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create a human-owned task and set it to doing
        let task = manager
            .add_task("Human task", None, None, Some("human"))
            .await
            .unwrap();
        manager
            .update_task(
                task.id,
                TaskUpdate {
                    status: Some("doing"),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // AI caller should be rejected
        let result = manager.done_task_by_id(task.id, true).await;
        assert!(matches!(
            result,
            Err(IntentError::HumanTaskCannotBeCompletedByAI { .. })
        ));

        // Human caller should succeed
        let response = manager.done_task_by_id(task.id, false).await.unwrap();
        assert_eq!(response.completed_task.status, "done");
    }

    #[tokio::test]
    async fn test_done_task_by_id_with_uncompleted_children() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let parent = manager.add_task("Parent", None, None, None).await.unwrap();
        manager
            .add_task("Incomplete child", None, Some(parent.id), None)
            .await
            .unwrap();

        let result = manager.done_task_by_id(parent.id, false).await;
        assert!(matches!(result, Err(IntentError::UncompletedChildren)));
    }

    #[tokio::test]
    async fn test_done_task_by_id_nonexistent_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let result = manager.done_task_by_id(99999, false).await;
        assert!(matches!(result, Err(IntentError::TaskNotFound(99999))));
    }

    #[tokio::test]
    async fn test_done_task_synthesis_graceful_when_llm_unconfigured() {
        // Verify that task completion works even when LLM is not configured
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());

        // Create and complete a task
        let task = manager
            .add_task("Test Task", Some("Original spec"), None, Some("ai"))
            .await
            .unwrap();

        // Add some events
        event_mgr
            .add_event(task.id, "decision", "Test decision")
            .await
            .unwrap();

        manager.start_task(task.id, false).await.unwrap();

        // Should complete successfully even without LLM
        let result = manager.done_task_by_id(task.id, false).await;
        assert!(result.is_ok(), "Task completion should succeed without LLM");

        // Verify task is actually done
        let completed_task = manager.get_task(task.id).await.unwrap();
        assert_eq!(completed_task.status, "done");

        // Original spec should be unchanged (no synthesis happened)
        assert_eq!(completed_task.spec, Some("Original spec".to_string()));
    }

    #[tokio::test]
    async fn test_done_task_synthesis_respects_owner_field() {
        // This test verifies the owner field logic without actual LLM
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create AI-owned task
        let ai_task = manager
            .add_task("AI Task", Some("AI spec"), None, Some("ai"))
            .await
            .unwrap();
        assert_eq!(ai_task.owner, "ai");

        // Create human-owned task
        let human_task = manager
            .add_task("Human Task", Some("Human spec"), None, Some("human"))
            .await
            .unwrap();
        assert_eq!(human_task.owner, "human");

        // Both should complete successfully
        manager.start_task(ai_task.id, false).await.unwrap();
        let result = manager.done_task_by_id(ai_task.id, false).await;
        assert!(result.is_ok());

        manager.start_task(human_task.id, false).await.unwrap();
        let result = manager.done_task_by_id(human_task.id, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_try_synthesize_task_description_basic() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager
            .add_task("Synthesis Test", Some("Original"), None, None)
            .await
            .unwrap();

        // Should return None when LLM not configured (graceful degradation)
        let result = manager
            .try_synthesize_task_description(task.id, &task.name)
            .await;

        assert!(result.is_ok(), "Should not error when LLM unconfigured");
        assert_eq!(
            result.unwrap(),
            None,
            "Should return None when LLM unconfigured"
        );
    }

    #[tokio::test]
    async fn test_pick_next_focused_subtask() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create parent task and set as current
        let parent = manager
            .add_task("Parent task", None, None, None)
            .await
            .unwrap();
        manager.start_task(parent.id, false).await.unwrap();

        // Create subtasks with different priorities
        let subtask1 = manager
            .add_task("Subtask 1", None, Some(parent.id), None)
            .await
            .unwrap();
        let subtask2 = manager
            .add_task("Subtask 2", None, Some(parent.id), None)
            .await
            .unwrap();

        // Set priorities: subtask1 = 2, subtask2 = 1 (lower number = higher priority)
        manager
            .update_task(
                subtask1.id,
                TaskUpdate {
                    priority: Some(2),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        manager
            .update_task(
                subtask2.id,
                TaskUpdate {
                    priority: Some(1),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Pick next should recommend subtask2 (priority 1)
        let response = manager.pick_next().await.unwrap();

        assert_eq!(response.suggestion_type, "FOCUSED_SUB_TASK");
        assert!(response.task.is_some());
        assert_eq!(response.task.as_ref().unwrap().id, subtask2.id);
        assert_eq!(response.task.as_ref().unwrap().name, "Subtask 2");
    }

    #[tokio::test]
    async fn test_pick_next_top_level_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create top-level tasks with different priorities
        let task1 = manager.add_task("Task 1", None, None, None).await.unwrap();
        let task2 = manager.add_task("Task 2", None, None, None).await.unwrap();

        // Set priorities: task1 = 5, task2 = 3 (lower number = higher priority)
        manager
            .update_task(
                task1.id,
                TaskUpdate {
                    priority: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        manager
            .update_task(
                task2.id,
                TaskUpdate {
                    priority: Some(3),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Pick next should recommend task2 (priority 3)
        let response = manager.pick_next().await.unwrap();

        assert_eq!(response.suggestion_type, "TOP_LEVEL_TASK");
        assert!(response.task.is_some());
        assert_eq!(response.task.as_ref().unwrap().id, task2.id);
        assert_eq!(response.task.as_ref().unwrap().name, "Task 2");
    }

    #[tokio::test]
    async fn test_pick_next_no_tasks() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // No tasks created
        let response = manager.pick_next().await.unwrap();

        assert_eq!(response.suggestion_type, "NONE");
        assert_eq!(response.reason_code.as_deref(), Some("NO_TASKS_IN_PROJECT"));
        assert!(response.message.is_some());
    }

    #[tokio::test]
    async fn test_pick_next_all_completed() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create task and mark as done
        let task = manager.add_task("Task 1", None, None, None).await.unwrap();
        manager.start_task(task.id, false).await.unwrap();
        manager.done_task(false).await.unwrap();

        // Pick next should indicate all tasks completed
        let response = manager.pick_next().await.unwrap();

        assert_eq!(response.suggestion_type, "NONE");
        assert_eq!(response.reason_code.as_deref(), Some("ALL_TASKS_COMPLETED"));
        assert!(response.message.is_some());
    }

    #[tokio::test]
    async fn test_pick_next_no_available_todos() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create a parent task that's in "doing" status
        let parent = manager
            .add_task("Parent task", None, None, None)
            .await
            .unwrap();
        manager.start_task(parent.id, false).await.unwrap();

        // Create a subtask also in "doing" status (no "todo" subtasks)
        let subtask = manager
            .add_task("Subtask", None, Some(parent.id), None)
            .await
            .unwrap();
        // Switch to subtask (this will set parent back to todo, so we need to manually set subtask to doing)
        sqlx::query("UPDATE tasks SET status = 'doing' WHERE id = ?")
            .bind(subtask.id)
            .execute(ctx.pool())
            .await
            .unwrap();

        // Set subtask as current
        let session_id = crate::workspace::resolve_session_id(None);
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
        .bind(subtask.id)
        .execute(ctx.pool())
        .await
        .unwrap();

        // Set parent to doing (not todo)
        sqlx::query("UPDATE tasks SET status = 'doing' WHERE id = ?")
            .bind(parent.id)
            .execute(ctx.pool())
            .await
            .unwrap();

        // With multi-doing semantics, pick next should recommend the doing parent
        // (it's a valid top-level doing task that's not current)
        let response = manager.pick_next().await.unwrap();

        assert_eq!(response.suggestion_type, "TOP_LEVEL_TASK");
        assert_eq!(response.task.as_ref().unwrap().id, parent.id);
        assert_eq!(response.task.as_ref().unwrap().status, "doing");
    }

    #[tokio::test]
    async fn test_pick_next_priority_ordering() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create parent and set as current
        let parent = manager.add_task("Parent", None, None, None).await.unwrap();
        manager.start_task(parent.id, false).await.unwrap();

        // Create multiple subtasks with various priorities
        let sub1 = manager
            .add_task("Priority 10", None, Some(parent.id), None)
            .await
            .unwrap();
        manager
            .update_task(
                sub1.id,
                TaskUpdate {
                    priority: Some(10),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let sub2 = manager
            .add_task("Priority 1", None, Some(parent.id), None)
            .await
            .unwrap();
        manager
            .update_task(
                sub2.id,
                TaskUpdate {
                    priority: Some(1),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let sub3 = manager
            .add_task("Priority 5", None, Some(parent.id), None)
            .await
            .unwrap();
        manager
            .update_task(
                sub3.id,
                TaskUpdate {
                    priority: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Pick next should recommend the task with priority 1 (lowest number)
        let response = manager.pick_next().await.unwrap();

        assert_eq!(response.suggestion_type, "FOCUSED_SUB_TASK");
        assert_eq!(response.task.as_ref().unwrap().id, sub2.id);
        assert_eq!(response.task.as_ref().unwrap().name, "Priority 1");
    }

    #[tokio::test]
    async fn test_pick_next_falls_back_to_top_level_when_no_subtasks() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create parent without subtasks and set as current
        let parent = manager.add_task("Parent", None, None, None).await.unwrap();
        manager.start_task(parent.id, false).await.unwrap();

        // Create another top-level task
        let top_level = manager
            .add_task("Top level task", None, None, None)
            .await
            .unwrap();

        // Pick next should fall back to top-level task since parent has no todo subtasks
        let response = manager.pick_next().await.unwrap();

        assert_eq!(response.suggestion_type, "TOP_LEVEL_TASK");
        assert_eq!(response.task.as_ref().unwrap().id, top_level.id);
    }

    // ===== Missing coverage tests =====

    #[tokio::test]
    async fn test_get_task_with_events() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());

        let task = task_mgr.add_task("Test", None, None, None).await.unwrap();

        // Add some events
        event_mgr
            .add_event(task.id, "progress", "Event 1")
            .await
            .unwrap();
        event_mgr
            .add_event(task.id, "decision", "Event 2")
            .await
            .unwrap();

        let result = task_mgr.get_task_with_events(task.id).await.unwrap();

        assert_eq!(result.task.id, task.id);
        assert!(result.events_summary.is_some());

        let summary = result.events_summary.unwrap();
        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.recent_events.len(), 2);
        assert_eq!(summary.recent_events[0].log_type, "decision"); // Most recent first
        assert_eq!(summary.recent_events[1].log_type, "progress");
    }

    #[tokio::test]
    async fn test_get_task_with_events_nonexistent() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        let result = task_mgr.get_task_with_events(999).await;
        assert!(matches!(result, Err(IntentError::TaskNotFound(999))));
    }

    #[tokio::test]
    async fn test_get_task_with_many_events() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());

        let task = task_mgr.add_task("Test", None, None, None).await.unwrap();

        // Add 20 events
        for i in 0..20 {
            event_mgr
                .add_event(task.id, "test", &format!("Event {}", i))
                .await
                .unwrap();
        }

        let result = task_mgr.get_task_with_events(task.id).await.unwrap();
        let summary = result.events_summary.unwrap();

        assert_eq!(summary.total_count, 20);
        assert_eq!(summary.recent_events.len(), 10); // Limited to 10
    }

    #[tokio::test]
    async fn test_get_task_with_no_events() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        let task = task_mgr.add_task("Test", None, None, None).await.unwrap();

        let result = task_mgr.get_task_with_events(task.id).await.unwrap();
        let summary = result.events_summary.unwrap();

        assert_eq!(summary.total_count, 0);
        assert_eq!(summary.recent_events.len(), 0);
    }

    #[tokio::test]
    async fn test_pick_next_tasks_zero_capacity() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        task_mgr.add_task("Task 1", None, None, None).await.unwrap();

        // capacity_limit = 0 means no capacity available
        let results = task_mgr.pick_next_tasks(10, 0).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_pick_next_tasks_capacity_exceeds_available() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        task_mgr.add_task("Task 1", None, None, None).await.unwrap();
        task_mgr.add_task("Task 2", None, None, None).await.unwrap();

        // Request 10 tasks but only 2 available, capacity = 100
        let results = task_mgr.pick_next_tasks(10, 100).await.unwrap();
        assert_eq!(results.len(), 2); // Only returns available tasks
    }

    // ========== task_context tests ==========

    #[tokio::test]
    async fn test_get_task_context_root_task_no_relations() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        // Create a single root task with no relations
        let task = task_mgr
            .add_task("Root task", None, None, None)
            .await
            .unwrap();

        let context = task_mgr.get_task_context(task.id).await.unwrap();

        // Verify task itself
        assert_eq!(context.task.id, task.id);
        assert_eq!(context.task.name, "Root task");

        // No ancestors (root task)
        assert_eq!(context.ancestors.len(), 0);

        // No siblings
        assert_eq!(context.siblings.len(), 0);

        // No children
        assert_eq!(context.children.len(), 0);
    }

    #[tokio::test]
    async fn test_get_task_context_with_siblings() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        // Create multiple root tasks (siblings)
        let task1 = task_mgr.add_task("Task 1", None, None, None).await.unwrap();
        let task2 = task_mgr.add_task("Task 2", None, None, None).await.unwrap();
        let task3 = task_mgr.add_task("Task 3", None, None, None).await.unwrap();

        let context = task_mgr.get_task_context(task2.id).await.unwrap();

        // Verify task itself
        assert_eq!(context.task.id, task2.id);

        // No ancestors (root task)
        assert_eq!(context.ancestors.len(), 0);

        // Should have 2 siblings
        assert_eq!(context.siblings.len(), 2);
        let sibling_ids: Vec<i64> = context.siblings.iter().map(|t| t.id).collect();
        assert!(sibling_ids.contains(&task1.id));
        assert!(sibling_ids.contains(&task3.id));
        assert!(!sibling_ids.contains(&task2.id)); // Should not include itself

        // No children
        assert_eq!(context.children.len(), 0);
    }

    #[tokio::test]
    async fn test_get_task_context_with_parent() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        // Create parent-child relationship
        let parent = task_mgr
            .add_task("Parent task", None, None, None)
            .await
            .unwrap();
        let child = task_mgr
            .add_task("Child task", None, Some(parent.id), None)
            .await
            .unwrap();

        let context = task_mgr.get_task_context(child.id).await.unwrap();

        // Verify task itself
        assert_eq!(context.task.id, child.id);
        assert_eq!(context.task.parent_id, Some(parent.id));

        // Should have 1 ancestor (the parent)
        assert_eq!(context.ancestors.len(), 1);
        assert_eq!(context.ancestors[0].id, parent.id);
        assert_eq!(context.ancestors[0].name, "Parent task");

        // No siblings
        assert_eq!(context.siblings.len(), 0);

        // No children
        assert_eq!(context.children.len(), 0);
    }

    #[tokio::test]
    async fn test_get_task_context_with_children() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        // Create parent with multiple children
        let parent = task_mgr
            .add_task("Parent task", None, None, None)
            .await
            .unwrap();
        let child1 = task_mgr
            .add_task("Child 1", None, Some(parent.id), None)
            .await
            .unwrap();
        let child2 = task_mgr
            .add_task("Child 2", None, Some(parent.id), None)
            .await
            .unwrap();
        let child3 = task_mgr
            .add_task("Child 3", None, Some(parent.id), None)
            .await
            .unwrap();

        let context = task_mgr.get_task_context(parent.id).await.unwrap();

        // Verify task itself
        assert_eq!(context.task.id, parent.id);

        // No ancestors (root task)
        assert_eq!(context.ancestors.len(), 0);

        // No siblings
        assert_eq!(context.siblings.len(), 0);

        // Should have 3 children
        assert_eq!(context.children.len(), 3);
        let child_ids: Vec<i64> = context.children.iter().map(|t| t.id).collect();
        assert!(child_ids.contains(&child1.id));
        assert!(child_ids.contains(&child2.id));
        assert!(child_ids.contains(&child3.id));
    }

    #[tokio::test]
    async fn test_get_task_context_multi_level_hierarchy() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        // Create 3-level hierarchy: grandparent -> parent -> child
        let grandparent = task_mgr
            .add_task("Grandparent", None, None, None)
            .await
            .unwrap();
        let parent = task_mgr
            .add_task("Parent", None, Some(grandparent.id), None)
            .await
            .unwrap();
        let child = task_mgr
            .add_task("Child", None, Some(parent.id), None)
            .await
            .unwrap();

        let context = task_mgr.get_task_context(child.id).await.unwrap();

        // Verify task itself
        assert_eq!(context.task.id, child.id);

        // Should have 2 ancestors (parent and grandparent, ordered from immediate to root)
        assert_eq!(context.ancestors.len(), 2);
        assert_eq!(context.ancestors[0].id, parent.id);
        assert_eq!(context.ancestors[0].name, "Parent");
        assert_eq!(context.ancestors[1].id, grandparent.id);
        assert_eq!(context.ancestors[1].name, "Grandparent");

        // No siblings
        assert_eq!(context.siblings.len(), 0);

        // No children
        assert_eq!(context.children.len(), 0);
    }

    #[tokio::test]
    async fn test_get_task_context_complex_family_tree() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        // Create complex structure:
        // Root
        //  ├─ Child1
        //  │   ├─ Grandchild1
        //  │   └─ Grandchild2 (target)
        //  └─ Child2

        let root = task_mgr.add_task("Root", None, None, None).await.unwrap();
        let child1 = task_mgr
            .add_task("Child1", None, Some(root.id), None)
            .await
            .unwrap();
        let child2 = task_mgr
            .add_task("Child2", None, Some(root.id), None)
            .await
            .unwrap();
        let grandchild1 = task_mgr
            .add_task("Grandchild1", None, Some(child1.id), None)
            .await
            .unwrap();
        let grandchild2 = task_mgr
            .add_task("Grandchild2", None, Some(child1.id), None)
            .await
            .unwrap();

        // Get context for grandchild2
        let context = task_mgr.get_task_context(grandchild2.id).await.unwrap();

        // Verify task itself
        assert_eq!(context.task.id, grandchild2.id);

        // Should have 2 ancestors: child1 and root
        assert_eq!(context.ancestors.len(), 2);
        assert_eq!(context.ancestors[0].id, child1.id);
        assert_eq!(context.ancestors[1].id, root.id);

        // Should have 1 sibling: grandchild1
        assert_eq!(context.siblings.len(), 1);
        assert_eq!(context.siblings[0].id, grandchild1.id);

        // No children
        assert_eq!(context.children.len(), 0);

        // Now get context for child1 to verify it sees both grandchildren
        let context_child1 = task_mgr.get_task_context(child1.id).await.unwrap();
        assert_eq!(context_child1.ancestors.len(), 1);
        assert_eq!(context_child1.ancestors[0].id, root.id);
        assert_eq!(context_child1.siblings.len(), 1);
        assert_eq!(context_child1.siblings[0].id, child2.id);
        assert_eq!(context_child1.children.len(), 2);
    }

    #[tokio::test]
    async fn test_get_task_context_respects_priority_ordering() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        // Create parent with children having different priorities
        let parent = task_mgr.add_task("Parent", None, None, None).await.unwrap();

        // Add children with priorities (lower number = higher priority)
        let child_low = task_mgr
            .add_task("Low priority", None, Some(parent.id), None)
            .await
            .unwrap();
        let _ = task_mgr
            .update_task(
                child_low.id,
                TaskUpdate {
                    priority: Some(10),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let child_high = task_mgr
            .add_task("High priority", None, Some(parent.id), None)
            .await
            .unwrap();
        let _ = task_mgr
            .update_task(
                child_high.id,
                TaskUpdate {
                    priority: Some(1),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let child_medium = task_mgr
            .add_task("Medium priority", None, Some(parent.id), None)
            .await
            .unwrap();
        let _ = task_mgr
            .update_task(
                child_medium.id,
                TaskUpdate {
                    priority: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let context = task_mgr.get_task_context(parent.id).await.unwrap();

        // Children should be ordered by priority (1, 5, 10)
        assert_eq!(context.children.len(), 3);
        assert_eq!(context.children[0].priority, Some(1));
        assert_eq!(context.children[1].priority, Some(5));
        assert_eq!(context.children[2].priority, Some(10));
    }

    #[tokio::test]
    async fn test_get_task_context_nonexistent_task() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        let result = task_mgr.get_task_context(99999).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(IntentError::TaskNotFound(99999))));
    }

    #[tokio::test]
    async fn test_get_task_context_handles_null_priority() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        // Create siblings with mixed null and set priorities
        let task1 = task_mgr.add_task("Task 1", None, None, None).await.unwrap();
        let _ = task_mgr
            .update_task(
                task1.id,
                TaskUpdate {
                    priority: Some(1),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let task2 = task_mgr.add_task("Task 2", None, None, None).await.unwrap();
        // task2 has NULL priority

        let task3 = task_mgr.add_task("Task 3", None, None, None).await.unwrap();
        let _ = task_mgr
            .update_task(
                task3.id,
                TaskUpdate {
                    priority: Some(5),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let context = task_mgr.get_task_context(task2.id).await.unwrap();

        // Should have 2 siblings, ordered by priority (non-null first, then null)
        assert_eq!(context.siblings.len(), 2);
        // Task with priority 1 should come first
        assert_eq!(context.siblings[0].id, task1.id);
        assert_eq!(context.siblings[0].priority, Some(1));
        // Task with priority 5 should come second
        assert_eq!(context.siblings[1].id, task3.id);
        assert_eq!(context.siblings[1].priority, Some(5));
    }

    #[tokio::test]
    async fn test_pick_next_tasks_priority_order() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        // Create 4 tasks with different priorities
        let critical = task_mgr
            .add_task("Critical Task", None, None, None)
            .await
            .unwrap();
        task_mgr
            .update_task(
                critical.id,
                TaskUpdate {
                    priority: Some(1),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let low = task_mgr
            .add_task("Low Task", None, None, None)
            .await
            .unwrap();
        task_mgr
            .update_task(
                low.id,
                TaskUpdate {
                    priority: Some(4),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let high = task_mgr
            .add_task("High Task", None, None, None)
            .await
            .unwrap();
        task_mgr
            .update_task(
                high.id,
                TaskUpdate {
                    priority: Some(2),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let medium = task_mgr
            .add_task("Medium Task", None, None, None)
            .await
            .unwrap();
        task_mgr
            .update_task(
                medium.id,
                TaskUpdate {
                    priority: Some(3),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Pick next tasks should return them in priority order: critical > high > medium > low
        let tasks = task_mgr.pick_next_tasks(10, 10).await.unwrap();

        assert_eq!(tasks.len(), 4);
        assert_eq!(tasks[0].id, critical.id); // Priority 1
        assert_eq!(tasks[1].id, high.id); // Priority 2
        assert_eq!(tasks[2].id, medium.id); // Priority 3
        assert_eq!(tasks[3].id, low.id); // Priority 4
    }

    #[tokio::test]
    async fn test_pick_next_prefers_doing_over_todo() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        // Create a parent task and set it as current
        let parent = task_mgr.add_task("Parent", None, None, None).await.unwrap();
        let parent_started = task_mgr.start_task(parent.id, false).await.unwrap();
        workspace_mgr
            .set_current_task(parent_started.task.id, None)
            .await
            .unwrap();

        // Create two subtasks with same priority: one doing, one todo
        let doing_subtask = task_mgr
            .add_task("Doing Subtask", None, Some(parent.id), None)
            .await
            .unwrap();
        task_mgr.start_task(doing_subtask.id, false).await.unwrap();
        // Switch back to parent so doing_subtask is "pending" (doing but not current)
        workspace_mgr
            .set_current_task(parent.id, None)
            .await
            .unwrap();

        let _todo_subtask = task_mgr
            .add_task("Todo Subtask", None, Some(parent.id), None)
            .await
            .unwrap();

        // Both have same priority (default), but doing should be picked first
        let result = task_mgr.pick_next().await.unwrap();

        if let Some(task) = result.task {
            assert_eq!(
                task.id, doing_subtask.id,
                "Should recommend doing subtask over todo subtask"
            );
            assert_eq!(task.status, "doing");
        } else {
            panic!("Expected a task recommendation");
        }
    }

    #[tokio::test]
    async fn test_multiple_doing_tasks_allowed() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let workspace_mgr = WorkspaceManager::new(ctx.pool());

        // Create and start task A
        let task_a = task_mgr.add_task("Task A", None, None, None).await.unwrap();
        let task_a_started = task_mgr.start_task(task_a.id, false).await.unwrap();
        assert_eq!(task_a_started.task.status, "doing");

        // Verify task A is current
        let current = workspace_mgr.get_current_task(None).await.unwrap();
        assert_eq!(current.current_task_id, Some(task_a.id));

        // Create and start task B
        let task_b = task_mgr.add_task("Task B", None, None, None).await.unwrap();
        let task_b_started = task_mgr.start_task(task_b.id, false).await.unwrap();
        assert_eq!(task_b_started.task.status, "doing");

        // Verify task B is now current
        let current = workspace_mgr.get_current_task(None).await.unwrap();
        assert_eq!(current.current_task_id, Some(task_b.id));

        // Verify task A is still doing (not reverted to todo)
        let task_a_after = task_mgr.get_task(task_a.id).await.unwrap();
        assert_eq!(
            task_a_after.status, "doing",
            "Task A should remain doing even though it is not current"
        );

        // Verify both tasks are in doing status
        let doing_tasks: Vec<Task> = sqlx::query_as(
            r#"SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata
             FROM tasks WHERE status = 'doing' ORDER BY id"#
        )
        .fetch_all(ctx.pool())
        .await
        .unwrap();

        assert_eq!(doing_tasks.len(), 2, "Should have 2 doing tasks");
        assert_eq!(doing_tasks[0].id, task_a.id);
        assert_eq!(doing_tasks[1].id, task_b.id);
    }
    #[tokio::test]
    async fn test_find_tasks_pagination() {
        let ctx = TestContext::new().await;
        let task_mgr = TaskManager::new(ctx.pool());

        // Create 15 tasks
        for i in 0..15 {
            task_mgr
                .add_task(&format!("Task {}", i), None, None, None)
                .await
                .unwrap();
        }

        // Page 1: Limit 10, Offset 0
        let page1 = task_mgr
            .find_tasks(None, None, None, Some(10), Some(0))
            .await
            .unwrap();
        assert_eq!(page1.tasks.len(), 10);
        assert_eq!(page1.total_count, 15);
        assert!(page1.has_more);
        assert_eq!(page1.offset, 0);

        // Page 2: Limit 10, Offset 10
        let page2 = task_mgr
            .find_tasks(None, None, None, Some(10), Some(10))
            .await
            .unwrap();
        assert_eq!(page2.tasks.len(), 5);
        assert_eq!(page2.total_count, 15);
        assert!(!page2.has_more);
        assert_eq!(page2.offset, 10);
    }
}

// Re-export TaskContext for cli_handlers
