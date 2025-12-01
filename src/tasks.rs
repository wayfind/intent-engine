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
pub struct TaskManager<'a> {
    pool: &'a SqlitePool,
    notifier: crate::notifications::NotificationSender,
    project_path: Option<String>,
}

impl<'a> TaskManager<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self {
            pool,
            notifier: crate::notifications::NotificationSender::new(None, None),
            project_path: None,
        }
    }

    /// Create a TaskManager with MCP notification support
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

    /// Create a TaskManager with WebSocket notification support
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

    /// Internal helper: Notify UI about task creation
    async fn notify_task_created(&self, task: &Task) {
        use crate::dashboard::websocket::DatabaseOperationPayload;

        let Some(project_path) = &self.project_path else {
            return;
        };

        let task_json = match serde_json::to_value(task) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to serialize task for notification: {}", e);
                return;
            },
        };

        let payload =
            DatabaseOperationPayload::task_created(task.id, task_json, project_path.clone());
        self.notifier.send(payload).await;
    }

    /// Internal helper: Notify UI about task update
    async fn notify_task_updated(&self, task: &Task) {
        use crate::dashboard::websocket::DatabaseOperationPayload;

        let Some(project_path) = &self.project_path else {
            return;
        };

        let task_json = match serde_json::to_value(task) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to serialize task for notification: {}", e);
                return;
            },
        };

        let payload =
            DatabaseOperationPayload::task_updated(task.id, task_json, project_path.clone());
        self.notifier.send(payload).await;
    }

    /// Internal helper: Notify UI about task deletion
    async fn notify_task_deleted(&self, task_id: i64) {
        use crate::dashboard::websocket::DatabaseOperationPayload;

        let Some(project_path) = &self.project_path else {
            return;
        };

        let payload = DatabaseOperationPayload::task_deleted(task_id, project_path.clone());
        self.notifier.send(payload).await;
    }

    /// Add a new task
    /// owner: 'human' (created via CLI/Dashboard) or 'ai' (created via MCP)
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

    /// Get a task by ID
    pub async fn get_task(&self, id: i64) -> Result<Task> {
        let task = sqlx::query_as::<_, Task>(
            r#"
            SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner
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
                       first_todo_at, first_doing_at, first_done_at, active_form, owner
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
                       first_todo_at, first_doing_at, first_done_at, active_form, owner
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
                   first_todo_at, first_doing_at, first_done_at, active_form, owner
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
                   t.first_todo_at, t.first_doing_at, t.first_done_at, t.active_form, t.owner
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
                   t.first_todo_at, t.first_doing_at, t.first_done_at, t.active_form, t.owner
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
    #[allow(clippy::too_many_arguments)]
    pub async fn update_task(
        &self,
        id: i64,
        name: Option<&str>,
        spec: Option<&str>,
        parent_id: Option<Option<i64>>,
        status: Option<&str>,
        complexity: Option<i32>,
        priority: Option<i32>,
    ) -> Result<Task> {
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
                        WHEN t.id = (SELECT value FROM workspace_state WHERE key = 'current_task_id') THEN 0
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
            "SELECT id, parent_id, name, NULL as spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner FROM tasks t {} {} LIMIT ? OFFSET ?",
            where_clause, order_clause
        );

        let mut q = sqlx::query_as::<_, Task>(&main_query);
        for cond in conditions {
            q = q.bind(cond);
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
    pub async fn start_task(&self, id: i64, with_events: bool) -> Result<TaskWithEvents> {
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

        // Set as current task
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO workspace_state (key, value)
            VALUES ('current_task_id', ?)
            "#,
        )
        .bind(id.to_string())
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

    /// Complete the current focused task (atomic: check children + update status + clear current)
    /// This command only operates on the current_task_id.
    /// Prerequisites: A task must be set as current
    ///
    /// # Arguments
    /// * `is_ai_caller` - Whether this is called from AI (MCP) or human (CLI/Dashboard).
    ///   When true and task is human-owned, the operation will fail.
    ///   Human tasks can only be completed via CLI or Dashboard.
    pub async fn done_task(&self, is_ai_caller: bool) -> Result<DoneTaskResponse> {
        let mut tx = self.pool.begin().await?;

        // Get the current task ID
        let current_task_id: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(&mut *tx)
                .await?;

        let id = current_task_id.and_then(|s| s.parse::<i64>().ok()).ok_or(
            IntentError::InvalidInput(
                "No current task is set. Use 'current --set <ID>' to set a task first.".to_string(),
            ),
        )?;

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

        // Check if all children are done
        let uncompleted_children: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE parent_id = ? AND status != 'done'",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

        if uncompleted_children > 0 {
            return Err(IntentError::UncompletedChildren);
        }

        let now = Utc::now();

        // Update task status to done
        sqlx::query(
            r#"
            UPDATE tasks
            SET status = 'done', first_done_at = COALESCE(first_done_at, ?)
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Clear the current task
        sqlx::query("DELETE FROM workspace_state WHERE key = 'current_task_id'")
            .execute(&mut *tx)
            .await?;

        // Determine next step suggestion based on context
        let next_step_suggestion = if let Some(parent_task_id) = parent_id {
            // Task has a parent - check sibling status
            let remaining_siblings: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM tasks WHERE parent_id = ? AND status != 'done' AND id != ?",
            )
            .bind(parent_task_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;

            if remaining_siblings == 0 {
                // All siblings are done - parent is ready
                let parent_name: String =
                    sqlx::query_scalar(crate::sql_constants::SELECT_TASK_NAME)
                        .bind(parent_task_id)
                        .fetch_one(&mut *tx)
                        .await?;

                NextStepSuggestion::ParentIsReady {
                    message: format!(
                        "All sub-tasks of parent #{} '{}' are now complete. The parent task is ready for your attention.",
                        parent_task_id, parent_name
                    ),
                    parent_task_id,
                    parent_task_name: parent_name,
                }
            } else {
                // Siblings remain
                let parent_name: String =
                    sqlx::query_scalar(crate::sql_constants::SELECT_TASK_NAME)
                        .bind(parent_task_id)
                        .fetch_one(&mut *tx)
                        .await?;

                NextStepSuggestion::SiblingTasksRemain {
                    message: format!(
                        "Task #{} completed. Parent task #{} '{}' has other sub-tasks remaining.",
                        id, parent_task_id, parent_name
                    ),
                    parent_task_id,
                    parent_task_name: parent_name,
                    remaining_siblings_count: remaining_siblings,
                }
            }
        } else {
            // No parent - check if this was a top-level task with children or standalone
            let child_count: i64 = sqlx::query_scalar(crate::sql_constants::COUNT_CHILDREN_TOTAL)
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;

            if child_count > 0 {
                // Top-level task with children completed
                NextStepSuggestion::TopLevelTaskCompleted {
                    message: format!(
                        "Top-level task #{} '{}' has been completed. Well done!",
                        id, task_name
                    ),
                    completed_task_id: id,
                    completed_task_name: task_name.clone(),
                }
            } else {
                // Check if workspace is clear
                let remaining_tasks: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM tasks WHERE status != 'done' AND id != ?",
                )
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;

                if remaining_tasks == 0 {
                    NextStepSuggestion::WorkspaceIsClear {
                        message: format!(
                            "Project complete! Task #{} was the last remaining task. There are no more 'todo' or 'doing' tasks.",
                            id
                        ),
                        completed_task_id: id,
                    }
                } else {
                    NextStepSuggestion::NoParentContext {
                        message: format!("Task #{} '{}' has been completed.", id, task_name),
                        completed_task_id: id,
                        completed_task_name: task_name.clone(),
                    }
                }
            }
        };

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

    /// Check if a task exists
    async fn check_task_exists(&self, id: i64) -> Result<()> {
        let exists: bool = sqlx::query_scalar(crate::sql_constants::CHECK_TASK_EXISTS)
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
                sqlx::query_scalar(crate::sql_constants::SELECT_TASK_PARENT_ID)
                    .bind(current_id)
                    .fetch_optional(self.pool)
                    .await?;

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
        // Get current task
        let current_task_id: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(self.pool)
                .await?;

        let parent_id = current_task_id.and_then(|s| s.parse::<i64>().ok()).ok_or(
            IntentError::InvalidInput("No current task to create subtask under".to_string()),
        )?;

        // Get parent task info
        let parent_name: String = sqlx::query_scalar(crate::sql_constants::SELECT_TASK_NAME)
            .bind(parent_id)
            .fetch_one(self.pool)
            .await?;

        // Create the subtask (inherit owner from parent or use default)
        let subtask = self.add_task(name, spec, Some(parent_id), None).await?;

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
        let doing_count: i64 = sqlx::query_scalar(crate::sql_constants::COUNT_TASKS_DOING)
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
                        SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner
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
            "SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner
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
        // Step 1: Check if there's a current focused task
        let current_task_id: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(self.pool)
                .await?;

        if let Some(current_id_str) = current_task_id.as_ref() {
            if let Ok(current_id) = current_id_str.parse::<i64>() {
                // Step 1a: First priority - Get **doing** subtasks of current focused task
                // Exclude tasks blocked by incomplete dependencies
                let doing_subtasks = sqlx::query_as::<_, Task>(
                    r#"
                            SELECT id, parent_id, name, spec, status, complexity, priority,
                                   first_todo_at, first_doing_at, first_done_at, active_form, owner
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
                                   first_todo_at, first_doing_at, first_done_at, active_form, owner
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
        }

        // Step 2a: Third priority - Get top-level **doing** tasks (excluding current task)
        // Exclude tasks blocked by incomplete dependencies
        let doing_top_level = if let Some(current_id_str) = current_task_id.as_ref() {
            if let Ok(current_id) = current_id_str.parse::<i64>() {
                sqlx::query_as::<_, Task>(
                    r#"
                    SELECT id, parent_id, name, spec, status, complexity, priority,
                           first_todo_at, first_doing_at, first_done_at, active_form, owner
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
                None
            }
        } else {
            sqlx::query_as::<_, Task>(
                r#"
                SELECT id, parent_id, name, spec, status, complexity, priority,
                       first_todo_at, first_doing_at, first_done_at, active_form, owner
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
                   first_todo_at, first_doing_at, first_done_at, active_form, owner
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
        let total_tasks: i64 = sqlx::query_scalar(crate::sql_constants::COUNT_TASKS_TOTAL)
            .fetch_one(self.pool)
            .await?;

        if total_tasks == 0 {
            return Ok(PickNextResponse::no_tasks_in_project());
        }

        // Check if all tasks are completed
        let todo_or_doing_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status IN ('todo', 'doing')")
                .fetch_one(self.pool)
                .await?;

        if todo_or_doing_count == 0 {
            return Ok(PickNextResponse::all_tasks_completed());
        }

        // Otherwise, there are tasks but none available based on current context
        Ok(PickNextResponse::no_available_todos())
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
            .update_task(task1.id, None, None, None, Some("doing"), None, None)
            .await
            .unwrap();
        manager
            .update_task(task2.id, None, None, None, Some("done"), None, None)
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
            .update_task(task.id, Some("New name"), None, None, None, None, None)
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
            .update_task(task.id, None, None, None, Some("doing"), None, None)
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
            .update_task(doing_task.id, None, None, None, Some("doing"), None, None)
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
        let current: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(ctx.pool())
                .await
                .unwrap();

        assert_eq!(current, Some(task.id.to_string()));
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
        let current: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(ctx.pool())
                .await
                .unwrap();

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
            .update_task(task1.id, None, None, Some(Some(task2.id)), None, None, None)
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
            .update_task(task.id, None, None, None, None, Some(8), Some(10))
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
        let current: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(ctx.pool())
                .await
                .unwrap();

        assert_eq!(current, Some(response.subtask.id.to_string()));

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
        let doing_count: i64 = sqlx::query_scalar(crate::sql_constants::COUNT_TASKS_DOING)
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
        let doing_count: i64 = sqlx::query_scalar(crate::sql_constants::COUNT_TASKS_DOING)
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
            .update_task(low.id, None, None, None, None, None, Some(1))
            .await
            .unwrap();

        let high = manager
            .add_task("High priority", None, None, None)
            .await
            .unwrap();
        manager
            .update_task(high.id, None, None, None, None, None, Some(10))
            .await
            .unwrap();

        let medium = manager
            .add_task("Medium priority", None, None, None)
            .await
            .unwrap();
        manager
            .update_task(medium.id, None, None, None, None, None, Some(5))
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
            .update_task(complex.id, None, None, None, None, Some(9), Some(5))
            .await
            .unwrap();

        let simple = manager.add_task("Simple", None, None, None).await.unwrap();
        manager
            .update_task(simple.id, None, None, None, None, Some(1), Some(5))
            .await
            .unwrap();

        let medium = manager.add_task("Medium", None, None, None).await.unwrap();
        manager
            .update_task(medium.id, None, None, None, None, Some(5), Some(5))
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
            .update_task(subtask1.id, None, None, None, None, None, Some(2))
            .await
            .unwrap();
        manager
            .update_task(subtask2.id, None, None, None, None, None, Some(1))
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
            .update_task(task1.id, None, None, None, None, None, Some(5))
            .await
            .unwrap();
        manager
            .update_task(task2.id, None, None, None, None, None, Some(3))
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
        sqlx::query(
            "INSERT OR REPLACE INTO workspace_state (key, value) VALUES ('current_task_id', ?)",
        )
        .bind(subtask.id.to_string())
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
            .update_task(sub1.id, None, None, None, None, None, Some(10))
            .await
            .unwrap();

        let sub2 = manager
            .add_task("Priority 1", None, Some(parent.id), None)
            .await
            .unwrap();
        manager
            .update_task(sub2.id, None, None, None, None, None, Some(1))
            .await
            .unwrap();

        let sub3 = manager
            .add_task("Priority 5", None, Some(parent.id), None)
            .await
            .unwrap();
        manager
            .update_task(sub3.id, None, None, None, None, None, Some(5))
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
            .update_task(child_low.id, None, None, None, None, None, Some(10))
            .await
            .unwrap();

        let child_high = task_mgr
            .add_task("High priority", None, Some(parent.id), None)
            .await
            .unwrap();
        let _ = task_mgr
            .update_task(child_high.id, None, None, None, None, None, Some(1))
            .await
            .unwrap();

        let child_medium = task_mgr
            .add_task("Medium priority", None, Some(parent.id), None)
            .await
            .unwrap();
        let _ = task_mgr
            .update_task(child_medium.id, None, None, None, None, None, Some(5))
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
            .update_task(task1.id, None, None, None, None, None, Some(1))
            .await
            .unwrap();

        let task2 = task_mgr.add_task("Task 2", None, None, None).await.unwrap();
        // task2 has NULL priority

        let task3 = task_mgr.add_task("Task 3", None, None, None).await.unwrap();
        let _ = task_mgr
            .update_task(task3.id, None, None, None, None, None, Some(5))
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
            .update_task(critical.id, None, None, None, None, None, Some(1))
            .await
            .unwrap();

        let low = task_mgr
            .add_task("Low Task", None, None, None)
            .await
            .unwrap();
        task_mgr
            .update_task(low.id, None, None, None, None, None, Some(4))
            .await
            .unwrap();

        let high = task_mgr
            .add_task("High Task", None, None, None)
            .await
            .unwrap();
        task_mgr
            .update_task(high.id, None, None, None, None, None, Some(2))
            .await
            .unwrap();

        let medium = task_mgr
            .add_task("Medium Task", None, None, None)
            .await
            .unwrap();
        task_mgr
            .update_task(medium.id, None, None, None, None, None, Some(3))
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
            .set_current_task(parent_started.task.id)
            .await
            .unwrap();

        // Create two subtasks with same priority: one doing, one todo
        let doing_subtask = task_mgr
            .add_task("Doing Subtask", None, Some(parent.id), None)
            .await
            .unwrap();
        task_mgr.start_task(doing_subtask.id, false).await.unwrap();
        // Switch back to parent so doing_subtask is "pending" (doing but not current)
        workspace_mgr.set_current_task(parent.id).await.unwrap();

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
        let current = workspace_mgr.get_current_task().await.unwrap();
        assert_eq!(current.current_task_id, Some(task_a.id));

        // Create and start task B
        let task_b = task_mgr.add_task("Task B", None, None, None).await.unwrap();
        let task_b_started = task_mgr.start_task(task_b.id, false).await.unwrap();
        assert_eq!(task_b_started.task.status, "doing");

        // Verify task B is now current
        let current = workspace_mgr.get_current_task().await.unwrap();
        assert_eq!(current.current_task_id, Some(task_b.id));

        // Verify task A is still doing (not reverted to todo)
        let task_a_after = task_mgr.get_task(task_a.id).await.unwrap();
        assert_eq!(
            task_a_after.status, "doing",
            "Task A should remain doing even though it is not current"
        );

        // Verify both tasks are in doing status
        let doing_tasks: Vec<Task> = sqlx::query_as(
            r#"SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form, owner
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
