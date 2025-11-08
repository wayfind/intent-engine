use crate::db::models::{Event, EventsSummary, Task, TaskWithEvents};
use crate::error::{IntentError, Result};
use chrono::Utc;
use sqlx::SqlitePool;

pub struct TaskManager<'a> {
    pool: &'a SqlitePool,
}

impl<'a> TaskManager<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Add a new task
    pub async fn add_task(
        &self,
        name: &str,
        spec: Option<&str>,
        parent_id: Option<i64>,
    ) -> Result<Task> {
        // Check for circular dependency if parent_id is provided
        if let Some(pid) = parent_id {
            self.check_task_exists(pid).await?;
        }

        let now = Utc::now();

        let result = sqlx::query(
            r#"
            INSERT INTO tasks (name, spec, parent_id, status, first_todo_at)
            VALUES (?, ?, ?, 'todo', ?)
            "#,
        )
        .bind(name)
        .bind(spec)
        .bind(parent_id)
        .bind(now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_task(id).await
    }

    /// Get a task by ID
    pub async fn get_task(&self, id: i64) -> Result<Task> {
        let task = sqlx::query_as::<_, Task>(
            r#"
            SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at
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
                return Err(IntentError::CircularDependency);
            }
            self.check_task_exists(pid).await?;
            self.check_circular_dependency(id, pid).await?;
        }

        // Build dynamic update query
        let mut query = String::from("UPDATE tasks SET ");
        let mut updates = Vec::new();

        if let Some(n) = name {
            updates.push(format!("name = '{}'", n.replace('\'', "''")));
        }

        if let Some(s) = spec {
            updates.push(format!("spec = '{}'", s.replace('\'', "''")));
        }

        if let Some(pid) = parent_id {
            match pid {
                Some(p) => updates.push(format!("parent_id = {}", p)),
                None => updates.push("parent_id = NULL".to_string()),
            }
        }

        if let Some(c) = complexity {
            updates.push(format!("complexity = {}", c));
        }

        if let Some(p) = priority {
            updates.push(format!("priority = {}", p));
        }

        if let Some(s) = status {
            updates.push(format!("status = '{}'", s));

            // Update timestamp fields based on status
            let now = Utc::now();
            match s {
                "todo" if task.first_todo_at.is_none() => {
                    updates.push(format!("first_todo_at = '{}'", now.to_rfc3339()));
                }
                "doing" if task.first_doing_at.is_none() => {
                    updates.push(format!("first_doing_at = '{}'", now.to_rfc3339()));
                }
                "done" if task.first_done_at.is_none() => {
                    updates.push(format!("first_done_at = '{}'", now.to_rfc3339()));
                }
                _ => {}
            }
        }

        if updates.is_empty() {
            return Ok(task);
        }

        query.push_str(&updates.join(", "));
        query.push_str(&format!(" WHERE id = {}", id));

        sqlx::query(&query).execute(self.pool).await?;

        self.get_task(id).await
    }

    /// Delete a task
    pub async fn delete_task(&self, id: i64) -> Result<()> {
        self.check_task_exists(id).await?;

        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Find tasks with optional filters
    pub async fn find_tasks(
        &self,
        status: Option<&str>,
        parent_id: Option<Option<i64>>,
    ) -> Result<Vec<Task>> {
        let mut query = String::from(
            "SELECT id, parent_id, name, NULL as spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at FROM tasks WHERE 1=1"
        );
        let mut conditions = Vec::new();

        if let Some(s) = status {
            query.push_str(" AND status = ?");
            conditions.push(s.to_string());
        }

        if let Some(pid) = parent_id {
            if let Some(p) = pid {
                query.push_str(" AND parent_id = ?");
                conditions.push(p.to_string());
            } else {
                query.push_str(" AND parent_id IS NULL");
            }
        }

        query.push_str(" ORDER BY id");

        let mut q = sqlx::query_as::<_, Task>(&query);
        for cond in conditions {
            q = q.bind(cond);
        }

        let tasks = q.fetch_all(self.pool).await?;
        Ok(tasks)
    }

    /// Start a task (atomic: update status + set current)
    pub async fn start_task(&self, id: i64, with_events: bool) -> Result<TaskWithEvents> {
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
            self.get_task_with_events(id).await
        } else {
            let task = self.get_task(id).await?;
            Ok(TaskWithEvents {
                task,
                events_summary: None,
            })
        }
    }

    /// Complete the current focused task (atomic: check children + update status + clear current)
    /// This command only operates on the current_task_id.
    /// Prerequisites: A task must be set as current
    pub async fn done_task(&self) -> Result<Task> {
        let mut tx = self.pool.begin().await?;

        // Get the current task ID
        let current_task_id: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(&mut *tx)
                .await?;

        let id = current_task_id
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or(IntentError::InvalidInput(
                "No current task is set. Use 'current --set <ID>' to set a task first.".to_string(),
            ))?;

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

        tx.commit().await?;

        self.get_task(id).await
    }

    /// Check if a task exists
    async fn check_task_exists(&self, id: i64) -> Result<()> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?)")
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
                return Err(IntentError::CircularDependency);
            }

            let parent: Option<i64> =
                sqlx::query_scalar("SELECT parent_id FROM tasks WHERE id = ?")
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

    /// Switch to a specific task (atomic: update status to doing + set as current)
    /// If the task is not in 'doing' status, it will be transitioned to 'doing'
    pub async fn switch_to_task(&self, id: i64) -> Result<TaskWithEvents> {
        // Verify task exists
        self.check_task_exists(id).await?;

        let mut tx = self.pool.begin().await?;
        let now = Utc::now();

        // Update task to doing status if not already
        sqlx::query(
            r#"
            UPDATE tasks
            SET status = 'doing',
                first_doing_at = COALESCE(first_doing_at, ?)
            WHERE id = ? AND status != 'doing'
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

        // Return task with events
        self.get_task_with_events(id).await
    }

    /// Create a subtask under the current task and switch to it (atomic operation)
    /// Returns error if there is no current task
    pub async fn spawn_subtask(&self, name: &str, spec: Option<&str>) -> Result<Task> {
        // Get current task
        let current_task_id: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(self.pool)
                .await?;

        let parent_id = current_task_id.and_then(|s| s.parse::<i64>().ok()).ok_or(
            IntentError::InvalidInput("No current task to create subtask under".to_string()),
        )?;

        // Create the subtask
        let subtask = self.add_task(name, spec, Some(parent_id)).await?;

        // Switch to the new subtask (returns updated task with status "doing")
        let task_with_events = self.switch_to_task(subtask.id).await?;

        Ok(task_with_events.task)
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
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'doing'")
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
            SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at
            FROM tasks
            WHERE status = 'todo'
            ORDER BY
                COALESCE(priority, 0) DESC,
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
            "SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at
             FROM tasks WHERE id IN ({})
             ORDER BY
                 COALESCE(priority, 0) DESC,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::TestContext;

    #[tokio::test]
    async fn test_add_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager.add_task("Test task", None, None).await.unwrap();

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
            .add_task("Test task", Some(spec), None)
            .await
            .unwrap();

        assert_eq!(task.name, "Test task");
        assert_eq!(task.spec.as_deref(), Some(spec));
    }

    #[tokio::test]
    async fn test_add_task_with_parent() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let parent = manager.add_task("Parent task", None, None).await.unwrap();
        let child = manager
            .add_task("Child task", None, Some(parent.id))
            .await
            .unwrap();

        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[tokio::test]
    async fn test_get_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let created = manager.add_task("Test task", None, None).await.unwrap();
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

        let task = manager.add_task("Original name", None, None).await.unwrap();
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

        let task = manager.add_task("Test task", None, None).await.unwrap();
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

        let task = manager.add_task("Test task", None, None).await.unwrap();
        manager.delete_task(task.id).await.unwrap();

        let result = manager.get_task(task.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_tasks_by_status() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        manager.add_task("Todo task", None, None).await.unwrap();
        let doing_task = manager.add_task("Doing task", None, None).await.unwrap();
        manager
            .update_task(doing_task.id, None, None, None, Some("doing"), None, None)
            .await
            .unwrap();

        let todo_tasks = manager.find_tasks(Some("todo"), None).await.unwrap();
        let doing_tasks = manager.find_tasks(Some("doing"), None).await.unwrap();

        assert_eq!(todo_tasks.len(), 1);
        assert_eq!(doing_tasks.len(), 1);
        assert_eq!(doing_tasks[0].status, "doing");
    }

    #[tokio::test]
    async fn test_find_tasks_by_parent() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let parent = manager.add_task("Parent", None, None).await.unwrap();
        manager
            .add_task("Child 1", None, Some(parent.id))
            .await
            .unwrap();
        manager
            .add_task("Child 2", None, Some(parent.id))
            .await
            .unwrap();

        let children = manager
            .find_tasks(None, Some(Some(parent.id)))
            .await
            .unwrap();

        assert_eq!(children.len(), 2);
    }

    #[tokio::test]
    async fn test_start_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager.add_task("Test task", None, None).await.unwrap();
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

        let task = manager.add_task("Test task", None, None).await.unwrap();

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

        let task = manager.add_task("Test task", None, None).await.unwrap();
        manager.start_task(task.id, false).await.unwrap();
        let done = manager.done_task().await.unwrap();

        assert_eq!(done.status, "done");
        assert!(done.first_done_at.is_some());

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

        let parent = manager.add_task("Parent", None, None).await.unwrap();
        manager
            .add_task("Child", None, Some(parent.id))
            .await
            .unwrap();

        // Set parent as current task
        manager.start_task(parent.id, false).await.unwrap();

        let result = manager.done_task().await;
        assert!(matches!(result, Err(IntentError::UncompletedChildren)));
    }

    #[tokio::test]
    async fn test_done_task_with_completed_children() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let parent = manager.add_task("Parent", None, None).await.unwrap();
        let child = manager
            .add_task("Child", None, Some(parent.id))
            .await
            .unwrap();

        // Complete child first
        manager.start_task(child.id, false).await.unwrap();
        manager.done_task().await.unwrap();

        // Now parent can be completed
        manager.start_task(parent.id, false).await.unwrap();
        let result = manager.done_task().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_circular_dependency() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task1 = manager.add_task("Task 1", None, None).await.unwrap();
        let task2 = manager
            .add_task("Task 2", None, Some(task1.id))
            .await
            .unwrap();

        // Try to make task1 a child of task2 (circular)
        let result = manager
            .update_task(task1.id, None, None, Some(Some(task2.id)), None, None, None)
            .await;

        assert!(matches!(result, Err(IntentError::CircularDependency)));
    }

    #[tokio::test]
    async fn test_invalid_parent_id() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let result = manager.add_task("Test", None, Some(999)).await;
        assert!(matches!(result, Err(IntentError::TaskNotFound(999))));
    }

    #[tokio::test]
    async fn test_update_task_complexity_and_priority() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        let task = manager.add_task("Test task", None, None).await.unwrap();
        let updated = manager
            .update_task(task.id, None, None, None, None, Some(8), Some(10))
            .await
            .unwrap();

        assert_eq!(updated.complexity, Some(8));
        assert_eq!(updated.priority, Some(10));
    }

    #[tokio::test]
    async fn test_switch_to_task() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create a task
        let task = manager.add_task("Test task", None, None).await.unwrap();
        assert_eq!(task.status, "todo");

        // Switch to it
        let switched = manager.switch_to_task(task.id).await.unwrap();
        assert_eq!(switched.task.status, "doing");
        assert!(switched.task.first_doing_at.is_some());

        // Verify it's set as current task
        let current: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(ctx.pool())
                .await
                .unwrap();

        assert_eq!(current, Some(task.id.to_string()));
    }

    #[tokio::test]
    async fn test_switch_to_task_already_doing() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create and start a task
        let task = manager.add_task("Test task", None, None).await.unwrap();
        manager.start_task(task.id, false).await.unwrap();

        // Switch to it again (should be idempotent)
        let switched = manager.switch_to_task(task.id).await.unwrap();
        assert_eq!(switched.task.status, "doing");
    }

    #[tokio::test]
    async fn test_spawn_subtask() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create and start a parent task
        let parent = manager.add_task("Parent task", None, None).await.unwrap();
        manager.start_task(parent.id, false).await.unwrap();

        // Spawn a subtask
        let subtask = manager
            .spawn_subtask("Child task", Some("Details"))
            .await
            .unwrap();

        assert_eq!(subtask.parent_id, Some(parent.id));
        assert_eq!(subtask.name, "Child task");
        assert_eq!(subtask.spec.as_deref(), Some("Details"));

        // Verify subtask is now the current task
        let current: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(ctx.pool())
                .await
                .unwrap();

        assert_eq!(current, Some(subtask.id.to_string()));

        // Verify subtask is in doing status
        let retrieved = manager.get_task(subtask.id).await.unwrap();
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
                .add_task(&format!("Task {}", i), None, None)
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
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'doing'")
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
                .add_task(&format!("Task {}", i), None, None)
                .await
                .unwrap();
        }

        // Start 2 tasks
        let tasks = manager.find_tasks(Some("todo"), None).await.unwrap();
        manager.start_task(tasks[0].id, false).await.unwrap();
        manager.start_task(tasks[1].id, false).await.unwrap();

        // Pick more tasks with capacity limit of 5
        let picked = manager.pick_next_tasks(10, 5).await.unwrap();

        // Should only pick 3 more (5 - 2 = 3)
        assert_eq!(picked.len(), 3);

        // Verify total doing count
        let doing_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'doing'")
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
                .add_task(&format!("Task {}", i), None, None)
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
        let low = manager.add_task("Low priority", None, None).await.unwrap();
        manager
            .update_task(low.id, None, None, None, None, None, Some(1))
            .await
            .unwrap();

        let high = manager.add_task("High priority", None, None).await.unwrap();
        manager
            .update_task(high.id, None, None, None, None, None, Some(10))
            .await
            .unwrap();

        let medium = manager
            .add_task("Medium priority", None, None)
            .await
            .unwrap();
        manager
            .update_task(medium.id, None, None, None, None, None, Some(5))
            .await
            .unwrap();

        // Pick tasks
        let picked = manager.pick_next_tasks(3, 5).await.unwrap();

        // Should be ordered by priority DESC
        assert_eq!(picked.len(), 3);
        assert_eq!(picked[0].priority, Some(10)); // high
        assert_eq!(picked[1].priority, Some(5)); // medium
        assert_eq!(picked[2].priority, Some(1)); // low
    }

    #[tokio::test]
    async fn test_pick_next_tasks_complexity_ordering() {
        let ctx = TestContext::new().await;
        let manager = TaskManager::new(ctx.pool());

        // Create tasks with different complexities (same priority)
        let complex = manager.add_task("Complex", None, None).await.unwrap();
        manager
            .update_task(complex.id, None, None, None, None, Some(9), Some(5))
            .await
            .unwrap();

        let simple = manager.add_task("Simple", None, None).await.unwrap();
        manager
            .update_task(simple.id, None, None, None, None, Some(1), Some(5))
            .await
            .unwrap();

        let medium = manager.add_task("Medium", None, None).await.unwrap();
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
}
