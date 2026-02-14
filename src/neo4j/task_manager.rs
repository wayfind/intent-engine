use crate::db::models::{
    DoneTaskResponse, NextStepSuggestion, PaginatedTasks, PickNextResponse, Task, TaskSortBy,
    TaskWithEvents, WorkspaceStatus,
};
use crate::error::{IntentError, Result};
use crate::tasks::TaskUpdate;
use chrono::{DateTime, Utc};
use neo4rs::{query, Graph};

/// Task management backed by Neo4j.
///
/// Design notes:
/// - `project_id` is cloned into each query param because neo4rs requires owned values.
///   This is a conscious choice: the per-query String clone cost is negligible compared
///   to the network round-trip to Neo4j.
/// - Parent-child hierarchy uses `CHILD_OF` relationships as source of truth for
///   structural queries (filters, traversals). The `parent_id` node property is a
///   denormalized cache set during writes, used only for populating the Task struct.
pub struct Neo4jTaskManager {
    graph: Graph,
    project_id: String,
}

impl Neo4jTaskManager {
    pub fn new(graph: Graph, project_id: String) -> Self {
        Self { graph, project_id }
    }

    // ── Read Operations (Phase 1) ────────────────────────────────

    /// Get all root tasks (no parent) ordered by status priority then id.
    ///
    /// Uses `NOT (t)-[:CHILD_OF]->()` instead of property check to ensure
    /// consistency with the graph relationship model.
    pub async fn get_root_tasks(&self) -> Result<Vec<Task>> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (t:Task {project_id: $pid}) \
                     WHERE NOT (t)-[:CHILD_OF]->() \
                     RETURN t \
                     ORDER BY \
                       CASE t.status \
                         WHEN 'doing' THEN 0 \
                         WHEN 'todo'  THEN 1 \
                         WHEN 'done'  THEN 2 \
                       END, \
                       t.priority ASC, \
                       t.id ASC",
                )
                .param("pid", self.project_id.clone()),
            )
            .await
            .map_err(|e| neo4j_err("get_root_tasks", e))?;

        let mut tasks = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("get_root_tasks iterate", e))?
        {
            tasks.push(row_to_task(&row, "t")?);
        }
        Ok(tasks)
    }

    /// Get a single task by ID.
    pub async fn get_task(&self, task_id: i64) -> Result<Task> {
        let mut result = self
            .graph
            .execute(
                query("MATCH (t:Task {project_id: $pid, id: $id}) RETURN t")
                    .param("pid", self.project_id.clone())
                    .param("id", task_id),
            )
            .await
            .map_err(|e| neo4j_err("get_task query", e))?;

        match result
            .next()
            .await
            .map_err(|e| neo4j_err("get_task fetch", e))?
        {
            Some(row) => {
                let node: neo4rs::Node = row.get("t").map_err(|e| neo4j_err("get_task node", e))?;
                node_to_task(&node)
            },
            None => Err(IntentError::TaskNotFound(task_id)),
        }
    }

    /// Get ancestor chain from a task up to root (immediate parent first).
    ///
    /// Captures the path in MATCH and orders by its length, avoiding
    /// re-evaluation of the variable-length pattern in ORDER BY.
    pub async fn get_task_ancestry(&self, task_id: i64) -> Result<Vec<Task>> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH path = (t:Task {project_id: $pid, id: $id})-[:CHILD_OF*1..]->(ancestor:Task {project_id: $pid}) \
                     RETURN ancestor, length(path) AS depth \
                     ORDER BY depth ASC",
                )
                .param("pid", self.project_id.clone())
                .param("id", task_id),
            )
            .await
            .map_err(|e| neo4j_err("get_task_ancestry", e))?;

        let mut ancestors = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("get_task_ancestry iterate", e))?
        {
            ancestors.push(row_to_task(&row, "ancestor")?);
        }
        Ok(ancestors)
    }

    /// Get sibling tasks (same parent, excluding self).
    ///
    /// Uses `CHILD_OF` relationships for both branches to maintain
    /// consistency with the graph-based hierarchy model.
    pub async fn get_siblings(&self, task_id: i64, parent_id: Option<i64>) -> Result<Vec<Task>> {
        let mut result = match parent_id {
            Some(pid) => {
                self.graph
                    .execute(
                        query(
                            "MATCH (t:Task {project_id: $proj})-[:CHILD_OF]->(parent:Task {project_id: $proj, id: $parent_id}) \
                             WHERE t.id <> $id \
                             RETURN t ORDER BY t.id ASC",
                        )
                        .param("proj", self.project_id.clone())
                        .param("parent_id", pid)
                        .param("id", task_id),
                    )
                    .await
                    .map_err(|e| neo4j_err("get_siblings (with parent)", e))?
            }
            None => {
                // Root-level siblings: other tasks with no CHILD_OF relationship
                self.graph
                    .execute(
                        query(
                            "MATCH (t:Task {project_id: $pid}) \
                             WHERE NOT (t)-[:CHILD_OF]->() AND t.id <> $id \
                             RETURN t ORDER BY t.id ASC",
                        )
                        .param("pid", self.project_id.clone())
                        .param("id", task_id),
                    )
                    .await
                    .map_err(|e| neo4j_err("get_siblings (root)", e))?
            }
        };

        let mut tasks = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("get_siblings iterate", e))?
        {
            tasks.push(row_to_task(&row, "t")?);
        }
        Ok(tasks)
    }

    /// Get direct children of a task.
    pub async fn get_children(&self, task_id: i64) -> Result<Vec<Task>> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (child:Task {project_id: $pid})-[:CHILD_OF]->(t:Task {project_id: $pid, id: $id}) \
                     RETURN child ORDER BY child.id ASC",
                )
                .param("pid", self.project_id.clone())
                .param("id", task_id),
            )
            .await
            .map_err(|e| neo4j_err("get_children", e))?;

        let mut tasks = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("get_children iterate", e))?
        {
            tasks.push(row_to_task(&row, "child")?);
        }
        Ok(tasks)
    }

    /// Get all descendants recursively using variable-length path.
    pub async fn get_descendants(&self, task_id: i64) -> Result<Vec<Task>> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (desc:Task {project_id: $pid})-[:CHILD_OF*1..]->(t:Task {project_id: $pid, id: $id}) \
                     RETURN desc ORDER BY desc.id ASC",
                )
                .param("pid", self.project_id.clone())
                .param("id", task_id),
            )
            .await
            .map_err(|e| neo4j_err("get_descendants", e))?;

        let mut descendants = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("get_descendants iterate", e))?
        {
            descendants.push(row_to_task(&row, "desc")?);
        }
        Ok(descendants)
    }

    /// Build a full StatusResponse for `ie-neo4j status <id>`.
    pub async fn get_status(
        &self,
        task_id: i64,
        with_events: bool,
    ) -> Result<crate::db::models::StatusResponse> {
        use crate::db::models::{StatusResponse, TaskBrief};

        let task = self.get_task(task_id).await?;
        let ancestors = self.get_task_ancestry(task_id).await?;
        let siblings_full = self.get_siblings(task_id, task.parent_id).await?;
        let descendants_full = self.get_descendants(task_id).await?;

        let siblings: Vec<TaskBrief> = siblings_full.iter().map(TaskBrief::from).collect();
        let descendants: Vec<TaskBrief> = descendants_full.iter().map(TaskBrief::from).collect();

        let events = if with_events {
            let event_mgr =
                super::Neo4jEventManager::new(self.graph.clone(), self.project_id.clone());
            let evts = event_mgr
                .list_events(Some(task_id), Some(20), None, None)
                .await?;
            if evts.is_empty() {
                None
            } else {
                Some(evts)
            }
        } else {
            None
        };

        Ok(StatusResponse {
            focused_task: task,
            ancestors,
            siblings,
            descendants,
            events,
        })
    }

    // ── Write Operations (Phase 2) ──────────────────────────────

    /// Add a new task. Returns the created Task.
    ///
    /// Creates a Task node with auto-generated ID, sets status to 'todo',
    /// and creates a CHILD_OF relationship if parent_id is provided.
    /// The parent_id is also stored as a denormalized node property.
    pub async fn add_task(
        &self,
        name: &str,
        spec: Option<&str>,
        parent_id: Option<i64>,
        owner: Option<&str>,
        priority: Option<i32>,
        metadata: Option<&str>,
    ) -> Result<Task> {
        // Validate parent exists if provided
        if let Some(pid) = parent_id {
            self.check_task_exists(pid).await?;
        }

        let id = super::next_id(&self.graph, &self.project_id, "task").await?;
        let now = Utc::now().to_rfc3339();
        let owner = owner.unwrap_or("human");

        // Create node with all properties in one shot.
        // Use COALESCE-style approach: always set core properties,
        // conditionally set optional ones via separate SET clauses below.
        let mut cypher = String::from(
            "CREATE (t:Task {project_id: $pid, id: $id, name: $name, \
             status: 'todo', owner: $owner, first_todo_at: $now})",
        );

        // Build optional SET clauses
        let mut sets = Vec::new();
        if spec.is_some() {
            sets.push("t.spec = $spec");
        }
        if parent_id.is_some() {
            sets.push("t.parent_id = $parent_id");
        }
        if priority.is_some() {
            sets.push("t.priority = $priority");
        }
        if metadata.is_some() {
            sets.push("t.metadata = $metadata");
        }
        if !sets.is_empty() {
            cypher.push_str(" SET ");
            cypher.push_str(&sets.join(", "));
        }

        // Create CHILD_OF relationship if parent exists
        if parent_id.is_some() {
            cypher.push_str(
                " WITH t \
                 MATCH (parent:Task {project_id: $pid, id: $parent_id}) \
                 CREATE (t)-[:CHILD_OF]->(parent)",
            );
        }

        cypher.push_str(" RETURN t");

        let mut q = query(&cypher)
            .param("pid", self.project_id.clone())
            .param("id", id)
            .param("name", name.to_string())
            .param("owner", owner.to_string())
            .param("now", now);

        if let Some(s) = spec {
            q = q.param("spec", s.to_string());
        }
        if let Some(pid) = parent_id {
            q = q.param("parent_id", pid);
        }
        if let Some(p) = priority {
            q = q.param("priority", p as i64);
        }
        if let Some(m) = metadata {
            q = q.param("metadata", m.to_string());
        }

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| neo4j_err("add_task", e))?;

        match result
            .next()
            .await
            .map_err(|e| neo4j_err("add_task fetch", e))?
        {
            Some(row) => row_to_task(&row, "t"),
            None => Err(IntentError::OtherError(anyhow::anyhow!(
                "add_task: CREATE did not return a node"
            ))),
        }
    }

    /// Update a task using the TaskUpdate struct (partial updates).
    ///
    /// Builds dynamic Cypher SET clauses based on which fields are Some.
    /// Handles parent_id changes by managing CHILD_OF relationships.
    pub async fn update_task(&self, id: i64, update: TaskUpdate<'_>) -> Result<Task> {
        let task = self.get_task(id).await?;

        // Validate status
        if let Some(s) = update.status {
            if !["todo", "doing", "done"].contains(&s) {
                return Err(IntentError::InvalidInput(format!("Invalid status: {}", s)));
            }
        }

        // Validate owner early
        if let Some(o) = update.owner {
            if o.is_empty() {
                return Err(IntentError::InvalidInput(
                    "owner cannot be empty".to_string(),
                ));
            }
        }

        // Circular dependency check for parent_id change
        if let Some(Some(pid)) = update.parent_id {
            if pid == id {
                return Err(IntentError::CircularDependency {
                    blocking_task_id: pid,
                    blocked_task_id: id,
                });
            }
            self.check_task_exists(pid).await?;
            self.check_circular_dependency(id, pid).await?;
        }

        // Build dynamic SET clause
        let mut set_parts: Vec<&str> = Vec::new();

        if update.name.is_some() {
            set_parts.push("t.name = $new_name");
        }
        if update.spec.is_some() {
            set_parts.push("t.spec = $new_spec");
        }
        if let Some(pid_opt) = update.parent_id {
            match pid_opt {
                Some(_) => set_parts.push("t.parent_id = $new_parent_id"),
                None => set_parts.push("t.parent_id = null"),
            }
        }
        if update.complexity.is_some() {
            set_parts.push("t.complexity = $new_complexity");
        }
        if update.priority.is_some() {
            set_parts.push("t.priority = $new_priority");
        }
        if update.active_form.is_some() {
            set_parts.push("t.active_form = $new_active_form");
        }
        if update.owner.is_some() {
            set_parts.push("t.owner = $new_owner");
        }
        if update.metadata.is_some() {
            set_parts.push("t.metadata = $new_metadata");
        }
        if let Some(s) = update.status {
            set_parts.push("t.status = $new_status");
            match s {
                "todo" if task.first_todo_at.is_none() => set_parts.push("t.first_todo_at = $ts"),
                "doing" if task.first_doing_at.is_none() => {
                    set_parts.push("t.first_doing_at = $ts")
                },
                "done" if task.first_done_at.is_none() => set_parts.push("t.first_done_at = $ts"),
                _ => {},
            }
        }

        if set_parts.is_empty() {
            return Ok(task);
        }

        // Transaction: atomic property + relationship updates
        let mut txn = self
            .graph
            .start_txn()
            .await
            .map_err(|e| neo4j_err("update_task start txn", e))?;

        let cypher = format!(
            "MATCH (t:Task {{project_id: $pid, id: $id}}) SET {}",
            set_parts.join(", ")
        );

        let mut q = query(&cypher)
            .param("pid", self.project_id.clone())
            .param("id", id);

        // Bind params — re-check Options directly (no boolean flags needed)
        if let Some(name) = update.name {
            q = q.param("new_name", name.to_string());
        }
        if let Some(spec) = update.spec {
            q = q.param("new_spec", spec.to_string());
        }
        if let Some(Some(pid)) = update.parent_id {
            q = q.param("new_parent_id", pid);
        }
        if let Some(c) = update.complexity {
            q = q.param("new_complexity", c as i64);
        }
        if let Some(p) = update.priority {
            q = q.param("new_priority", p as i64);
        }
        if let Some(af) = update.active_form {
            q = q.param("new_active_form", af.to_string());
        }
        if let Some(o) = update.owner {
            q = q.param("new_owner", o.to_string());
        }
        if let Some(m) = update.metadata {
            q = q.param("new_metadata", m.to_string());
        }
        if let Some(s) = update.status {
            q = q.param("new_status", s.to_string());
            let needs_ts = match s {
                "todo" => task.first_todo_at.is_none(),
                "doing" => task.first_doing_at.is_none(),
                "done" => task.first_done_at.is_none(),
                _ => false,
            };
            if needs_ts {
                q = q.param("ts", Utc::now().to_rfc3339());
            }
        }

        txn.run(q)
            .await
            .map_err(|e| neo4j_err("update_task set", e))?;

        // Handle parent_id relationship changes within same transaction
        if let Some(pid_opt) = update.parent_id {
            // Remove existing CHILD_OF relationship
            txn.run(
                query("MATCH (t:Task {project_id: $pid, id: $id})-[r:CHILD_OF]->() DELETE r")
                    .param("pid", self.project_id.clone())
                    .param("id", id),
            )
            .await
            .map_err(|e| neo4j_err("update_task remove CHILD_OF", e))?;

            // Create new CHILD_OF if setting a parent
            if let Some(new_pid) = pid_opt {
                txn.run(
                    query(
                        "MATCH (t:Task {project_id: $pid, id: $id}), \
                         (parent:Task {project_id: $pid, id: $parent_id}) \
                         CREATE (t)-[:CHILD_OF]->(parent)",
                    )
                    .param("pid", self.project_id.clone())
                    .param("id", id)
                    .param("parent_id", new_pid),
                )
                .await
                .map_err(|e| neo4j_err("update_task create CHILD_OF", e))?;
            }
        }

        txn.commit()
            .await
            .map_err(|e| neo4j_err("update_task commit", e))?;

        // Re-read after commit for consistent return value
        self.get_task(id).await
    }

    /// Delete a task by ID. Does NOT cascade to children.
    ///
    /// Refuses to delete tasks that are focused by any session.
    pub async fn delete_task(&self, id: i64) -> Result<()> {
        self.check_task_exists(id).await?;

        // Focus protection
        if let Some((tid, session_id)) = self.find_focused_in_subtree(id).await? {
            return Err(IntentError::ActionNotAllowed(format!(
                "Task #{} is focused by session '{}'. Unfocus it first.",
                tid, session_id
            )));
        }

        self.graph
            .run(
                query("MATCH (t:Task {project_id: $pid, id: $id}) DETACH DELETE t")
                    .param("pid", self.project_id.clone())
                    .param("id", id),
            )
            .await
            .map_err(|e| neo4j_err("delete_task", e))?;

        Ok(())
    }

    /// Delete a task and all its descendants (cascade).
    ///
    /// Refuses if any task in the subtree is focused by any session.
    /// All deletions run in a single transaction.
    pub async fn delete_task_cascade(&self, id: i64) -> Result<usize> {
        self.check_task_exists(id).await?;

        // Focus protection for entire subtree
        if let Some((tid, session_id)) = self.find_focused_in_subtree(id).await? {
            return Err(IntentError::ActionNotAllowed(format!(
                "Cannot cascade delete: task #{} is focused by session '{}'. Unfocus it first.",
                tid, session_id
            )));
        }

        // Count descendants before deletion for reporting
        let descendants = self.get_descendants(id).await?;
        let count = descendants.len();

        // Transaction: delete all descendants then the task itself
        let mut txn = self
            .graph
            .start_txn()
            .await
            .map_err(|e| neo4j_err("delete_task_cascade start txn", e))?;

        txn.run(
            query(
                "MATCH (desc:Task {project_id: $pid})-[:CHILD_OF*1..]->(t:Task {project_id: $pid, id: $id}) \
                 DETACH DELETE desc",
            )
            .param("pid", self.project_id.clone())
            .param("id", id),
        )
        .await
        .map_err(|e| neo4j_err("delete_task_cascade descendants", e))?;

        txn.run(
            query("MATCH (t:Task {project_id: $pid, id: $id}) DETACH DELETE t")
                .param("pid", self.project_id.clone())
                .param("id", id),
        )
        .await
        .map_err(|e| neo4j_err("delete_task_cascade self", e))?;

        txn.commit()
            .await
            .map_err(|e| neo4j_err("delete_task_cascade commit", e))?;

        Ok(count)
    }

    /// Find tasks with optional filters, sorting, and pagination.
    pub async fn find_tasks(
        &self,
        status: Option<&str>,
        parent_id: Option<Option<i64>>,
        sort_by: Option<TaskSortBy>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PaginatedTasks> {
        let sort_by = sort_by.unwrap_or_default();
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        // Build WHERE conditions
        let mut where_parts = vec!["t.project_id = $pid".to_string()];
        let mut has_status_filter = false;
        let mut has_parent_filter = false;

        if status.is_some() {
            where_parts.push("t.status = $filter_status".to_string());
            has_status_filter = true;
        }

        if let Some(pid_opt) = parent_id {
            match pid_opt {
                Some(_) => {
                    // Has a specific parent — use CHILD_OF relationship
                    has_parent_filter = true;
                },
                None => {
                    // Root tasks — no CHILD_OF
                    where_parts.push("NOT (t)-[:CHILD_OF]->()".to_string());
                },
            }
        }

        let where_clause = where_parts.join(" AND ");

        // Order clause
        let order_clause = match sort_by {
            TaskSortBy::Id => "ORDER BY t.id ASC".to_string(),
            TaskSortBy::Priority => {
                "ORDER BY COALESCE(t.priority, 999) ASC, COALESCE(t.complexity, 5) ASC, t.id ASC"
                    .to_string()
            },
            TaskSortBy::Time => "ORDER BY \
                 CASE t.status \
                   WHEN 'doing' THEN t.first_doing_at \
                   WHEN 'todo' THEN t.first_todo_at \
                   WHEN 'done' THEN t.first_done_at \
                 END ASC, t.id ASC"
                .to_string(),
            TaskSortBy::FocusAware => "ORDER BY \
                 CASE t.status \
                   WHEN 'doing' THEN 0 \
                   WHEN 'todo' THEN 1 \
                   WHEN 'done' THEN 2 \
                   ELSE 3 \
                 END ASC, \
                 COALESCE(t.priority, 999) ASC, t.id ASC"
                .to_string(),
        };

        // Build count query
        let count_cypher = if has_parent_filter {
            format!(
                "MATCH (t:Task)-[:CHILD_OF]->(parent:Task {{project_id: $pid, id: $parent_id}}) \
                 WHERE {} RETURN count(t) AS cnt",
                where_clause
            )
        } else {
            format!(
                "MATCH (t:Task) WHERE {} RETURN count(t) AS cnt",
                where_clause
            )
        };

        let mut count_q = query(&count_cypher).param("pid", self.project_id.clone());
        if has_status_filter {
            count_q = count_q.param("filter_status", status.unwrap().to_string());
        }
        if has_parent_filter {
            count_q = count_q.param("parent_id", parent_id.unwrap().unwrap());
        }

        let mut count_result = self
            .graph
            .execute(count_q)
            .await
            .map_err(|e| neo4j_err("find_tasks count", e))?;

        let total_count: i64 = match count_result
            .next()
            .await
            .map_err(|e| neo4j_err("find_tasks count fetch", e))?
        {
            Some(row) => row
                .get("cnt")
                .map_err(|e| neo4j_err("find_tasks count value", e))?,
            None => 0,
        };

        // Build main query
        let main_cypher = if has_parent_filter {
            format!(
                "MATCH (t:Task)-[:CHILD_OF]->(parent:Task {{project_id: $pid, id: $parent_id}}) \
                 WHERE {} {} SKIP $offset LIMIT $limit RETURN t",
                where_clause, order_clause
            )
        } else {
            format!(
                "MATCH (t:Task) WHERE {} {} SKIP $offset LIMIT $limit RETURN t",
                where_clause, order_clause
            )
        };

        let mut main_q = query(&main_cypher)
            .param("pid", self.project_id.clone())
            .param("offset", offset)
            .param("limit", limit);

        if has_status_filter {
            main_q = main_q.param("filter_status", status.unwrap().to_string());
        }
        if has_parent_filter {
            main_q = main_q.param("parent_id", parent_id.unwrap().unwrap());
        }

        let mut result = self
            .graph
            .execute(main_q)
            .await
            .map_err(|e| neo4j_err("find_tasks query", e))?;

        let mut tasks = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("find_tasks iterate", e))?
        {
            tasks.push(row_to_task(&row, "t")?);
        }

        let has_more = offset + (tasks.len() as i64) < total_count;

        Ok(PaginatedTasks {
            tasks,
            total_count,
            has_more,
            limit,
            offset,
        })
    }

    /// Start a task: set status to 'doing' and focus the session on it.
    ///
    /// Checks for blocking dependencies (BLOCKED_BY relationships).
    pub async fn start_task(&self, id: i64, _with_events: bool) -> Result<TaskWithEvents> {
        self.check_task_exists(id).await?;

        // Check blocking dependencies (no-op when BLOCKED_BY relationships don't exist yet)
        let blocking_ids = self.get_blocking_task_ids(id).await?;
        if !blocking_ids.is_empty() {
            return Err(IntentError::TaskBlocked {
                task_id: id,
                blocking_task_ids: blocking_ids,
            });
        }

        let now = Utc::now().to_rfc3339();
        let session_id = crate::workspace::resolve_session_id(None);

        // Transaction: status update + session focus atomically
        let mut txn = self
            .graph
            .start_txn()
            .await
            .map_err(|e| neo4j_err("start_task start txn", e))?;

        // Update task status to doing, set first_doing_at if not already set
        txn.run(
            query(
                "MATCH (t:Task {project_id: $pid, id: $id}) \
                 SET t.status = 'doing', \
                     t.first_doing_at = COALESCE(t.first_doing_at, $now)",
            )
            .param("pid", self.project_id.clone())
            .param("id", id)
            .param("now", now),
        )
        .await
        .map_err(|e| neo4j_err("start_task update", e))?;

        // Set as current task in session
        txn.run(
            query(
                "MERGE (s:Session {project_id: $pid, session_id: $sid}) \
                 ON CREATE SET s.created_at = datetime(), s.last_active_at = datetime() \
                 ON MATCH SET s.last_active_at = datetime() \
                 SET s.current_task_id = $tid",
            )
            .param("pid", self.project_id.clone())
            .param("sid", session_id)
            .param("tid", id),
        )
        .await
        .map_err(|e| neo4j_err("start_task set session", e))?;

        txn.commit()
            .await
            .map_err(|e| neo4j_err("start_task commit", e))?;

        let task = self.get_task(id).await?;
        Ok(TaskWithEvents {
            task,
            events_summary: None,
        })
    }

    /// Complete a task by ID. Validates children are done first.
    ///
    /// State changes (status + focus) run in a single transaction.
    /// Returns a DoneTaskResponse with next-step suggestion.
    pub async fn done_task_by_id(&self, id: i64) -> Result<DoneTaskResponse> {
        let task = self.get_task(id).await?;

        // Check incomplete children
        let incomplete = self.count_incomplete_children(id).await?;
        if incomplete > 0 {
            return Err(IntentError::UncompletedChildren);
        }

        let now = Utc::now().to_rfc3339();
        let session_id = crate::workspace::resolve_session_id(None);

        // Transaction: set done + clear focus atomically
        let mut txn = self
            .graph
            .start_txn()
            .await
            .map_err(|e| neo4j_err("done_task start txn", e))?;

        // Set status to done
        txn.run(
            query(
                "MATCH (t:Task {project_id: $pid, id: $id}) \
                 SET t.status = 'done', \
                     t.first_done_at = COALESCE(t.first_done_at, $now)",
            )
            .param("pid", self.project_id.clone())
            .param("id", id)
            .param("now", now),
        )
        .await
        .map_err(|e| neo4j_err("done_task set done", e))?;

        // Clear session focus if this task is the current focus
        txn.run(
            query(
                "MATCH (s:Session {project_id: $pid, session_id: $sid}) \
                 WHERE s.current_task_id = $tid \
                 SET s.current_task_id = null, s.last_active_at = datetime()",
            )
            .param("pid", self.project_id.clone())
            .param("sid", session_id.clone())
            .param("tid", id),
        )
        .await
        .map_err(|e| neo4j_err("done_task clear focus", e))?;

        txn.commit()
            .await
            .map_err(|e| neo4j_err("done_task commit", e))?;

        // Read focus state after commit
        let mut focus_result = self
            .graph
            .execute(
                query(
                    "OPTIONAL MATCH (s:Session {project_id: $pid, session_id: $sid}) \
                     RETURN s.current_task_id AS current_task_id",
                )
                .param("pid", self.project_id.clone())
                .param("sid", session_id),
            )
            .await
            .map_err(|e| neo4j_err("done_task read focus", e))?;

        let actual_current_task_id: Option<i64> = focus_result
            .next()
            .await
            .map_err(|e| neo4j_err("done_task read focus fetch", e))?
            .and_then(|row| row.get("current_task_id").ok());

        // Build next-step suggestion
        let next_step_suggestion = self
            .build_next_step_suggestion(id, &task.name, task.parent_id)
            .await?;

        let completed_task = self.get_task(id).await?;

        Ok(DoneTaskResponse {
            completed_task,
            workspace_status: WorkspaceStatus {
                current_task_id: actual_current_task_id,
            },
            next_step_suggestion,
        })
    }

    /// Complete the current focused task.
    pub async fn done_task(&self) -> Result<DoneTaskResponse> {
        let session_id = crate::workspace::resolve_session_id(None);

        let mut result = self
            .graph
            .execute(
                query(
                    "OPTIONAL MATCH (s:Session {project_id: $pid, session_id: $sid}) \
                     RETURN s.current_task_id AS current_task_id",
                )
                .param("pid", self.project_id.clone())
                .param("sid", session_id),
            )
            .await
            .map_err(|e| neo4j_err("done_task get focus", e))?;

        let current_task_id: Option<i64> = result
            .next()
            .await
            .map_err(|e| neo4j_err("done_task get focus fetch", e))?
            .and_then(|row| row.get("current_task_id").ok());

        let id = current_task_id.ok_or(IntentError::InvalidInput(
            "No current task is set. Use 'ie-neo4j task start <ID>' to set a task first."
                .to_string(),
        ))?;

        self.done_task_by_id(id).await
    }

    /// Suggest the next task to work on based on context.
    ///
    /// Priority order:
    /// 1. Doing subtasks of current focus
    /// 2. Todo subtasks of current focus
    /// 3. Top-level doing tasks
    /// 4. Top-level todo tasks
    pub async fn pick_next(&self) -> Result<PickNextResponse> {
        let session_id = crate::workspace::resolve_session_id(None);

        // Get current focused task
        let mut focus_result = self
            .graph
            .execute(
                query(
                    "OPTIONAL MATCH (s:Session {project_id: $pid, session_id: $sid}) \
                     RETURN s.current_task_id AS current_task_id",
                )
                .param("pid", self.project_id.clone())
                .param("sid", session_id),
            )
            .await
            .map_err(|e| neo4j_err("pick_next get focus", e))?;

        let current_task_id: Option<i64> = focus_result
            .next()
            .await
            .map_err(|e| neo4j_err("pick_next get focus fetch", e))?
            .and_then(|row| row.get("current_task_id").ok());

        // If we have a focused task, look for subtasks
        if let Some(current_id) = current_task_id {
            // Priority 1: doing subtasks of current focus
            if let Some(task) = self.find_child_by_status(current_id, "doing").await? {
                return Ok(PickNextResponse::focused_subtask(task));
            }

            // Priority 2: todo subtasks of current focus
            if let Some(task) = self.find_child_by_status(current_id, "todo").await? {
                return Ok(PickNextResponse::focused_subtask(task));
            }
        }

        // Priority 3: top-level doing tasks (excluding current)
        if let Some(task) = self
            .find_top_level_by_status("doing", current_task_id)
            .await?
        {
            return Ok(PickNextResponse::top_level_task(task));
        }

        // Priority 4: top-level todo tasks
        if let Some(task) = self.find_top_level_by_status("todo", None).await? {
            return Ok(PickNextResponse::top_level_task(task));
        }

        // Check if there are any tasks at all
        let mut count_result = self
            .graph
            .execute(
                query("MATCH (t:Task {project_id: $pid}) RETURN count(t) AS cnt")
                    .param("pid", self.project_id.clone()),
            )
            .await
            .map_err(|e| neo4j_err("pick_next count", e))?;

        let total: i64 = count_result
            .next()
            .await
            .map_err(|e| neo4j_err("pick_next count fetch", e))?
            .and_then(|row| row.get::<i64>("cnt").ok())
            .unwrap_or(0);

        if total == 0 {
            return Ok(PickNextResponse::no_tasks_in_project());
        }

        // Check if all tasks are done
        let mut incomplete_result = self
            .graph
            .execute(
                query(
                    "MATCH (t:Task {project_id: $pid}) WHERE t.status <> 'done' \
                     RETURN count(t) AS cnt",
                )
                .param("pid", self.project_id.clone()),
            )
            .await
            .map_err(|e| neo4j_err("pick_next incomplete count", e))?;

        let incomplete: i64 = incomplete_result
            .next()
            .await
            .map_err(|e| neo4j_err("pick_next incomplete fetch", e))?
            .and_then(|row| row.get::<i64>("cnt").ok())
            .unwrap_or(0);

        if incomplete == 0 {
            return Ok(PickNextResponse::all_tasks_completed());
        }

        Ok(PickNextResponse::no_available_todos())
    }

    // ── Internal Helpers ────────────────────────────────────────

    /// Check if any task in the subtree (self + descendants) is focused by any session.
    ///
    /// Uses `[:CHILD_OF*0..]` to include the root task itself in the check.
    /// Returns `Some((focused_task_id, session_id))` if found, `None` otherwise.
    async fn find_focused_in_subtree(&self, task_id: i64) -> Result<Option<(i64, String)>> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (desc:Task {project_id: $pid})-[:CHILD_OF*0..]->(root:Task {project_id: $pid, id: $id}) \
                     WITH collect(desc.id) AS subtree_ids \
                     MATCH (s:Session {project_id: $pid}) \
                     WHERE s.current_task_id IN subtree_ids \
                     RETURN s.current_task_id AS tid, s.session_id AS sid \
                     LIMIT 1",
                )
                .param("pid", self.project_id.clone())
                .param("id", task_id),
            )
            .await
            .map_err(|e| neo4j_err("find_focused_in_subtree", e))?;

        match result
            .next()
            .await
            .map_err(|e| neo4j_err("find_focused_in_subtree fetch", e))?
        {
            Some(row) => {
                let tid: i64 = row
                    .get("tid")
                    .map_err(|e| neo4j_err("find_focused_in_subtree tid", e))?;
                let sid: String = row
                    .get("sid")
                    .map_err(|e| neo4j_err("find_focused_in_subtree sid", e))?;
                Ok(Some((tid, sid)))
            },
            None => Ok(None),
        }
    }

    /// Get IDs of incomplete tasks that block the given task via BLOCKED_BY relationships.
    ///
    /// Returns empty vec when no BLOCKED_BY relationships exist (current state).
    /// Will automatically work when dependency support is added to the Neo4j schema.
    async fn get_blocking_task_ids(&self, task_id: i64) -> Result<Vec<i64>> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (t:Task {project_id: $pid, id: $id})-[:BLOCKED_BY]->(blocking:Task {project_id: $pid}) \
                     WHERE blocking.status IN ['todo', 'doing'] \
                     RETURN blocking.id AS blocking_id",
                )
                .param("pid", self.project_id.clone())
                .param("id", task_id),
            )
            .await
            .map_err(|e| neo4j_err("get_blocking_task_ids", e))?;

        let mut ids = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("get_blocking_task_ids iterate", e))?
        {
            if let Ok(bid) = row.get::<i64>("blocking_id") {
                ids.push(bid);
            }
        }
        Ok(ids)
    }

    async fn check_task_exists(&self, id: i64) -> Result<()> {
        let mut result = self
            .graph
            .execute(
                query("MATCH (t:Task {project_id: $pid, id: $id}) RETURN t.id AS id")
                    .param("pid", self.project_id.clone())
                    .param("id", id),
            )
            .await
            .map_err(|e| neo4j_err("check_task_exists", e))?;

        match result
            .next()
            .await
            .map_err(|e| neo4j_err("check_task_exists fetch", e))?
        {
            Some(_) => Ok(()),
            None => Err(IntentError::TaskNotFound(id)),
        }
    }

    /// Walk up the CHILD_OF chain to detect circular dependencies.
    async fn check_circular_dependency(&self, task_id: i64, new_parent_id: i64) -> Result<()> {
        // Use a variable-length path: if there's a path from new_parent up to task_id,
        // then making task_id a child of new_parent would create a cycle.
        let mut result = self
            .graph
            .execute(
                query(
                    "OPTIONAL MATCH path = (p:Task {project_id: $pid, id: $new_parent})-[:CHILD_OF*0..]->(t:Task {project_id: $pid, id: $task_id}) \
                     RETURN path IS NOT NULL AS is_cycle",
                )
                .param("pid", self.project_id.clone())
                .param("new_parent", new_parent_id)
                .param("task_id", task_id),
            )
            .await
            .map_err(|e| neo4j_err("check_circular_dependency", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("check_circular_dependency fetch", e))?
        {
            let is_cycle: bool = row.get("is_cycle").unwrap_or(false);
            if is_cycle {
                return Err(IntentError::CircularDependency {
                    blocking_task_id: new_parent_id,
                    blocked_task_id: task_id,
                });
            }
        }

        Ok(())
    }

    /// Count incomplete (non-done) direct children via CHILD_OF relationships.
    async fn count_incomplete_children(&self, task_id: i64) -> Result<i64> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (child:Task {project_id: $pid})-[:CHILD_OF]->(t:Task {project_id: $pid, id: $id}) \
                     WHERE child.status <> 'done' \
                     RETURN count(child) AS cnt",
                )
                .param("pid", self.project_id.clone())
                .param("id", task_id),
            )
            .await
            .map_err(|e| neo4j_err("count_incomplete_children", e))?;

        Ok(result
            .next()
            .await
            .map_err(|e| neo4j_err("count_incomplete_children fetch", e))?
            .and_then(|row| row.get::<i64>("cnt").ok())
            .unwrap_or(0))
    }

    /// Count total direct children.
    async fn count_children(&self, task_id: i64) -> Result<i64> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (child:Task {project_id: $pid})-[:CHILD_OF]->(t:Task {project_id: $pid, id: $id}) \
                     RETURN count(child) AS cnt",
                )
                .param("pid", self.project_id.clone())
                .param("id", task_id),
            )
            .await
            .map_err(|e| neo4j_err("count_children", e))?;

        Ok(result
            .next()
            .await
            .map_err(|e| neo4j_err("count_children fetch", e))?
            .and_then(|row| row.get::<i64>("cnt").ok())
            .unwrap_or(0))
    }

    /// Find a child task with a given status (for pick_next).
    async fn find_child_by_status(&self, parent_id: i64, status: &str) -> Result<Option<Task>> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (child:Task {project_id: $pid})-[:CHILD_OF]->(parent:Task {project_id: $pid, id: $parent_id}) \
                     WHERE child.status = $status \
                     RETURN child \
                     ORDER BY COALESCE(child.priority, 999) ASC, child.id ASC \
                     LIMIT 1",
                )
                .param("pid", self.project_id.clone())
                .param("parent_id", parent_id)
                .param("status", status.to_string()),
            )
            .await
            .map_err(|e| neo4j_err("find_child_by_status", e))?;

        match result
            .next()
            .await
            .map_err(|e| neo4j_err("find_child_by_status fetch", e))?
        {
            Some(row) => Ok(Some(row_to_task(&row, "child")?)),
            None => Ok(None),
        }
    }

    /// Find a top-level (root) task with a given status, optionally excluding a task.
    async fn find_top_level_by_status(
        &self,
        status: &str,
        exclude_id: Option<i64>,
    ) -> Result<Option<Task>> {
        let cypher = if exclude_id.is_some() {
            "MATCH (t:Task {project_id: $pid}) \
             WHERE NOT (t)-[:CHILD_OF]->() AND t.status = $status AND t.id <> $exclude_id \
             RETURN t \
             ORDER BY COALESCE(t.priority, 999) ASC, t.id ASC \
             LIMIT 1"
        } else {
            "MATCH (t:Task {project_id: $pid}) \
             WHERE NOT (t)-[:CHILD_OF]->() AND t.status = $status \
             RETURN t \
             ORDER BY COALESCE(t.priority, 999) ASC, t.id ASC \
             LIMIT 1"
        };

        let mut q = query(cypher)
            .param("pid", self.project_id.clone())
            .param("status", status.to_string());

        if let Some(eid) = exclude_id {
            q = q.param("exclude_id", eid);
        }

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| neo4j_err("find_top_level_by_status", e))?;

        match result
            .next()
            .await
            .map_err(|e| neo4j_err("find_top_level_by_status fetch", e))?
        {
            Some(row) => Ok(Some(row_to_task(&row, "t")?)),
            None => Ok(None),
        }
    }

    /// Build a next-step suggestion after completing a task.
    async fn build_next_step_suggestion(
        &self,
        id: i64,
        task_name: &str,
        parent_id: Option<i64>,
    ) -> Result<NextStepSuggestion> {
        if let Some(parent_task_id) = parent_id {
            // Count remaining incomplete siblings
            let mut result = self
                .graph
                .execute(
                    query(
                        "MATCH (sibling:Task {project_id: $pid})-[:CHILD_OF]->(parent:Task {project_id: $pid, id: $parent_id}) \
                         WHERE sibling.status <> 'done' AND sibling.id <> $id \
                         RETURN count(sibling) AS cnt",
                    )
                    .param("pid", self.project_id.clone())
                    .param("parent_id", parent_task_id)
                    .param("id", id),
                )
                .await
                .map_err(|e| neo4j_err("next_step remaining siblings", e))?;

            let remaining_siblings: i64 = result
                .next()
                .await
                .map_err(|e| neo4j_err("next_step siblings fetch", e))?
                .and_then(|row| row.get::<i64>("cnt").ok())
                .unwrap_or(0);

            let parent = self.get_task(parent_task_id).await?;
            let parent_name = parent.name;

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
            let child_count = self.count_children(id).await?;

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
                // Count remaining incomplete tasks
                let mut result = self
                    .graph
                    .execute(
                        query(
                            "MATCH (t:Task {project_id: $pid}) \
                             WHERE t.status <> 'done' AND t.id <> $id \
                             RETURN count(t) AS cnt",
                        )
                        .param("pid", self.project_id.clone())
                        .param("id", id),
                    )
                    .await
                    .map_err(|e| neo4j_err("next_step remaining tasks", e))?;

                let remaining_tasks: i64 = result
                    .next()
                    .await
                    .map_err(|e| neo4j_err("next_step remaining fetch", e))?
                    .and_then(|row| row.get::<i64>("cnt").ok())
                    .unwrap_or(0);

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
}

// ── Helpers ─────────────────────────────────────────────────────

/// Extract a Task node from a Row by column name.
fn row_to_task(row: &neo4rs::Row, column: &str) -> Result<Task> {
    let node: neo4rs::Node = row
        .get(column)
        .map_err(|e| neo4j_err(&format!("get column '{column}'"), e))?;
    node_to_task(&node)
}

/// Convert a Neo4j Node to a Task struct.
///
/// Required properties (id, name) produce errors if missing.
/// Nullable properties use explicit null-checking to distinguish
/// "property missing/null" from "property exists with wrong type".
pub(crate) fn node_to_task(node: &neo4rs::Node) -> Result<Task> {
    let id: i64 = node.get("id").map_err(|e| neo4j_err("task.id", e))?;

    let name: String = node.get("name").map_err(|e| neo4j_err("task.name", e))?;

    // For nullable fields: .get() returns Err for both "missing" and "wrong type".
    // Since Neo4j properties are dynamically typed and we control writes,
    // treating all Err as None is acceptable — a wrong-type property would have
    // been caught at write time. If this assumption changes, add explicit type checks.
    let parent_id: Option<i64> = node.get("parent_id").ok();
    let spec: Option<String> = node.get("spec").ok();
    let complexity: Option<i32> = node.get("complexity").ok();
    let priority: Option<i32> = node.get("priority").ok();
    let active_form: Option<String> = node.get("active_form").ok();
    let metadata: Option<String> = node.get("metadata").ok();

    let status: String = node.get("status").unwrap_or_else(|_| "todo".into());
    let owner: String = node.get("owner").unwrap_or_else(|_| "human".into());

    let first_todo_at = parse_datetime_prop(node, "first_todo_at");
    let first_doing_at = parse_datetime_prop(node, "first_doing_at");
    let first_done_at = parse_datetime_prop(node, "first_done_at");

    Ok(Task {
        id,
        parent_id,
        name,
        spec,
        status,
        complexity,
        priority,
        first_todo_at,
        first_doing_at,
        first_done_at,
        active_form,
        owner,
        metadata,
    })
}

/// Try to parse an ISO-8601 datetime string property from a Neo4j node.
fn parse_datetime_prop(node: &neo4rs::Node, key: &str) -> Option<DateTime<Utc>> {
    let s: String = node.get(key).ok()?;
    parse_datetime_str(&s)
}

/// Parse an ISO-8601 / RFC-3339 datetime string into `DateTime<Utc>`.
/// Extracted from `parse_datetime_prop` for testability.
fn parse_datetime_str(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| s.parse::<DateTime<Utc>>().ok())
}

/// Convert a neo4rs error into an IntentError with context.
pub(crate) fn neo4j_err(context: &str, e: impl std::fmt::Display) -> IntentError {
    IntentError::OtherError(anyhow::anyhow!("Neo4j {}: {}", context, e))
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime_str_rfc3339() {
        use chrono::Datelike;
        let dt = parse_datetime_str("2026-02-14T10:30:00Z");
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 2);
        assert_eq!(dt.day(), 14);
    }

    #[test]
    fn test_parse_datetime_str_with_offset() {
        use chrono::Timelike;
        let dt = parse_datetime_str("2026-02-14T10:30:00+08:00");
        assert!(dt.is_some());
        // Should be converted to UTC
        let dt = dt.unwrap();
        assert_eq!(dt.hour(), 2); // 10:30 +08:00 = 02:30 UTC
    }

    #[test]
    fn test_parse_datetime_str_invalid() {
        assert!(parse_datetime_str("not-a-date").is_none());
        assert!(parse_datetime_str("").is_none());
        assert!(parse_datetime_str("2026-13-45").is_none());
    }

    #[test]
    fn test_parse_datetime_str_iso8601_no_offset() {
        let dt = parse_datetime_str("2026-02-14T10:30:00Z");
        assert!(dt.is_some());
    }

    #[test]
    fn test_neo4j_err_includes_context() {
        let err = neo4j_err("get_task query", "connection refused");
        let msg = err.to_string();
        assert!(msg.contains("Neo4j get_task query"));
        assert!(msg.contains("connection refused"));
    }
}
