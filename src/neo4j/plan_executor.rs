//! Plan executor backed by Neo4j.
//!
//! Provides batch create/update/delete of task hierarchies, mirroring the
//! behavior of the SQLite `PlanExecutor` in `src/plan.rs`.
//!
//! Key differences from SQLite version:
//! - Uses `Neo4jTaskManager` methods instead of raw SQL
//! - Individual operations are NOT wrapped in a transaction (no atomicity guarantee)
//! - No dashboard notifications (no SQLite DB available)
//! - Dependencies use BLOCKED_BY relationships (MERGE for idempotency)

use crate::db::models::TaskWithEvents;
use crate::error::{IntentError, Result};
use crate::plan::{
    extract_all_names, find_duplicate_names, flatten_task_tree, ExistingTaskInfo, FlatTask,
    PlanRequest, PlanResult, TaskStatus,
};
use crate::plan_validation;
use crate::tasks::TaskUpdate;
use neo4rs::{query, Graph};
use std::collections::{HashMap, HashSet};

use super::task_manager::neo4j_err;

/// Plan executor for batch task operations on Neo4j.
pub struct Neo4jPlanExecutor {
    graph: Graph,
    project_id: String,
    /// Default parent ID for auto-parenting new root tasks to the focused task.
    default_parent_id: Option<i64>,
}

impl Neo4jPlanExecutor {
    pub fn new(graph: Graph, project_id: String) -> Self {
        Self {
            graph,
            project_id,
            default_parent_id: None,
        }
    }

    /// Set default parent ID for auto-parenting new root tasks.
    pub fn with_default_parent(mut self, parent_id: i64) -> Self {
        self.default_parent_id = Some(parent_id);
        self
    }

    /// Execute a plan request: batch create/update/delete tasks.
    pub async fn execute(&self, request: &PlanRequest) -> Result<PlanResult> {
        let task_mgr = super::Neo4jTaskManager::new(self.graph.clone(), self.project_id.clone());

        // ── 1. Validate: duplicate names ──
        let duplicates = find_duplicate_names(&request.tasks);
        if !duplicates.is_empty() {
            return Ok(PlanResult::error(format!(
                "Duplicate task names in request: {:?}",
                duplicates
            )));
        }

        // ── 2. Extract all names ──
        let all_names = extract_all_names(&request.tasks);

        // ── 3. Flatten task tree ──
        let flat_tasks = flatten_task_tree(&request.tasks);

        // ── 4. Validate dependencies exist in plan ──
        if let Err(e) = plan_validation::validate_dependencies(&flat_tasks) {
            return Ok(PlanResult::error(e.to_string()));
        }

        // ── 5. Detect circular dependencies ──
        if let Err(e) = plan_validation::detect_circular_dependencies(&flat_tasks) {
            return Ok(PlanResult::error(e.to_string()));
        }

        // ── 6. Validate batch-level single doing constraint ──
        if let Err(e) = plan_validation::validate_batch_single_doing(&flat_tasks) {
            return Ok(PlanResult::error(e.to_string()));
        }

        // ── 7. Find existing tasks by name (outside transaction) ──
        let existing = self.find_tasks_by_names(&all_names).await?;

        // ── 8. Separate delete vs normal operations ──
        let (delete_tasks, normal_tasks): (Vec<&FlatTask>, Vec<&FlatTask>) =
            flat_tasks.iter().partition(|t| t.delete);

        // Validate delete operations: each must have an id
        for task in &delete_tasks {
            if task.id.is_none() {
                return Ok(PlanResult::error(
                    "Delete operation requires 'id' field. Use {\"id\": <task_id>, \"delete\": true}",
                ));
            }
        }

        // ── 9. Focus protection check (before any deletions) ──
        for task in &delete_tasks {
            if let Some(id) = task.id {
                if let Some((focused_id, session_id)) = task_mgr.find_focused_in_subtree(id).await?
                {
                    if focused_id == id {
                        return Ok(PlanResult::error(format!(
                            "Task #{} is the current focus of session '{}'. That session must switch focus first.",
                            id, session_id
                        )));
                    } else {
                        return Ok(PlanResult::error(format!(
                            "Task #{} is the current focus of session '{}' and would be deleted by cascade (descendant of #{}). That session must switch focus first.",
                            focused_id, session_id, id
                        )));
                    }
                }
            }
        }

        // ── 10. Execute operations ──
        let mut task_id_map: HashMap<String, i64> = HashMap::new();
        let mut created_count = 0;
        let mut updated_count = 0;
        let mut deleted_count = 0;
        let mut cascade_deleted_count: i64 = 0;
        let mut warnings: Vec<String> = Vec::new();
        let mut newly_created_names: HashSet<String> = HashSet::new();

        // ── 10a. Process deletes first ──
        for task in &delete_tasks {
            if let Some(id) = task.id {
                // Check if task exists
                match task_mgr.get_task(id).await {
                    Ok(_) => {
                        // Count descendants before deletion
                        let descendants = task_mgr.get_descendants(id).await?;
                        let desc_count = descendants.len() as i64;

                        task_mgr.delete_task_cascade(id).await?;
                        deleted_count += 1;

                        if desc_count > 0 {
                            cascade_deleted_count += desc_count;
                            warnings.push(format!(
                                "Task #{} had {} descendant(s) that were also deleted (cascade)",
                                id, desc_count
                            ));
                        }
                    },
                    Err(_) => {
                        warnings.push(format!(
                            "Task #{} not found (may have been already deleted)",
                            id
                        ));
                    },
                }
            }
        }

        // ── 10b. Process create/update operations ──
        for task in &normal_tasks {
            let task_name = match &task.name {
                Some(name) => name,
                None => continue,
            };

            let is_becoming_doing = task.status.as_ref() == Some(&TaskStatus::Doing);
            let has_spec = task
                .spec
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

            if let Some(existing_info) = existing.get(task_name) {
                // ── UPDATE existing task ──

                // Validate spec required for doing transition
                if is_becoming_doing && !has_spec {
                    let existing_is_doing = existing_info.status == "doing";
                    let existing_has_spec = existing_info
                        .spec
                        .as_ref()
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);

                    if !existing_is_doing && !existing_has_spec {
                        return Ok(PlanResult::error(format!(
                            "Task '{}': spec (description) is required when starting a task (status: doing).\n\n\
                            Before starting a task, please describe:\n  \
                            • What is the goal of this task\n  \
                            • How do you plan to approach it",
                            task_name
                        )));
                    }
                }

                let is_becoming_done = task.status.as_ref() == Some(&TaskStatus::Done);

                // Build update
                let status_for_update = if is_becoming_done {
                    None // done handled separately
                } else {
                    task.status.as_ref().map(|s| s.as_db_str())
                };

                // Note: task was found by name match, so name is already correct.
                // No need to fetch task again just to compare names.
                let update = TaskUpdate {
                    spec: task.spec.as_deref(),
                    priority: task.priority.as_ref().map(|p| p.to_int()),
                    status: status_for_update,
                    active_form: task.active_form.as_deref(),
                    ..Default::default()
                };

                task_mgr.update_task(existing_info.id, update).await?;

                // Handle done transition
                if is_becoming_done {
                    if let Err(e) = task_mgr.done_task_by_id(existing_info.id).await {
                        return Ok(PlanResult::error(format!(
                            "Cannot complete task '{}': {}\n\n\
                            Please complete all subtasks before marking the parent as done.",
                            task_name, e
                        )));
                    }
                }

                task_id_map.insert(task_name.clone(), existing_info.id);
                updated_count += 1;
            } else {
                // ── CREATE new task ──

                if is_becoming_doing && !has_spec {
                    return Ok(PlanResult::error(format!(
                        "Task '{}': spec (description) is required when starting a task (status: doing).\n\n\
                        Before starting a task, please describe:\n  \
                        • What is the goal of this task\n  \
                        • How do you plan to approach it",
                        task_name
                    )));
                }

                // Determine initial status (doing is handled via start_task later)
                let initial_status = match &task.status {
                    Some(TaskStatus::Doing) => None, // will use start_task
                    Some(s) => Some(s.as_db_str()),
                    None => None,
                };

                let new_task = task_mgr
                    .add_task(
                        task_name,
                        task.spec.as_deref(),
                        None, // parent set later
                        Some("ai"),
                        task.priority.as_ref().map(|p| p.to_int()),
                        None, // metadata
                    )
                    .await?;

                // Set status and active_form in a single update if needed
                let needs_status = initial_status
                    .as_ref()
                    .map(|s| *s != "todo")
                    .unwrap_or(false);
                if needs_status || task.active_form.is_some() {
                    task_mgr
                        .update_task(
                            new_task.id,
                            TaskUpdate {
                                status: if needs_status { initial_status } else { None },
                                active_form: task.active_form.as_deref(),
                                ..Default::default()
                            },
                        )
                        .await?;
                }

                task_id_map.insert(task_name.clone(), new_task.id);
                newly_created_names.insert(task_name.clone());
                created_count += 1;

                if !has_spec && !is_becoming_doing {
                    warnings.push(format!(
                        "Task '{}' has no description. Consider adding one for better context.",
                        task_name
                    ));
                }
            }
        }

        // ── 11. Build parent-child relationships ──

        // 11a. Children nesting (parent_name from tree structure)
        for task in &normal_tasks {
            if let Some(parent_name) = &task.parent_name {
                if let Some(task_name) = &task.name {
                    let task_id = task_id_map.get(task_name).ok_or_else(|| {
                        IntentError::InvalidInput(format!("Task not found: {}", task_name))
                    })?;
                    let parent_id = task_id_map.get(parent_name).ok_or_else(|| {
                        IntentError::InvalidInput(format!("Parent task not found: {}", parent_name))
                    })?;
                    task_mgr
                        .update_task(
                            *task_id,
                            TaskUpdate {
                                parent_id: Some(Some(*parent_id)),
                                ..Default::default()
                            },
                        )
                        .await?;
                }
            }
        }

        // 11b. Explicit parent_id (takes precedence over auto-parent)
        for task in &normal_tasks {
            if task.parent_name.is_some() {
                continue; // already handled
            }

            if let Some(explicit_parent) = &task.explicit_parent_id {
                if let Some(task_name) = &task.name {
                    let task_id = task_id_map.get(task_name).ok_or_else(|| {
                        IntentError::InvalidInput(format!("Task not found: {}", task_name))
                    })?;

                    match explicit_parent {
                        None => {
                            // parent_id: null → root task
                            task_mgr
                                .update_task(
                                    *task_id,
                                    TaskUpdate {
                                        parent_id: Some(None),
                                        ..Default::default()
                                    },
                                )
                                .await?;
                        },
                        Some(parent_id) => {
                            task_mgr
                                .update_task(
                                    *task_id,
                                    TaskUpdate {
                                        parent_id: Some(Some(*parent_id)),
                                        ..Default::default()
                                    },
                                )
                                .await?;
                        },
                    }
                }
            }
        }

        // 11c. Auto-parent newly created root tasks to default_parent_id
        if let Some(default_parent) = self.default_parent_id {
            for task in &normal_tasks {
                if let Some(task_name) = &task.name {
                    if newly_created_names.contains(task_name)
                        && task.parent_name.is_none()
                        && task.explicit_parent_id.is_none()
                    {
                        if let Some(&task_id) = task_id_map.get(task_name) {
                            task_mgr
                                .update_task(
                                    task_id,
                                    TaskUpdate {
                                        parent_id: Some(Some(default_parent)),
                                        ..Default::default()
                                    },
                                )
                                .await?;
                        }
                    }
                }
            }
        }

        // ── 12. Build BLOCKED_BY dependency relationships ──
        let dep_count = self.build_dependencies(&flat_tasks, &task_id_map).await?;

        // ── 13. Auto-focus the doing task ──
        let doing_task = normal_tasks
            .iter()
            .find(|task| matches!(task.status, Some(TaskStatus::Doing)));

        let focused_task_response: Option<TaskWithEvents> = if let Some(doing_task) = doing_task {
            if let Some(task_name) = &doing_task.name {
                if let Some(&task_id) = task_id_map.get(task_name) {
                    let response = task_mgr.start_task(task_id, true).await?;
                    Some(response)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // ── 14. Return result ──
        Ok(PlanResult::success_with_warnings(
            task_id_map,
            created_count,
            updated_count,
            deleted_count,
            cascade_deleted_count,
            dep_count,
            focused_task_response,
            warnings,
        ))
    }

    /// Build BLOCKED_BY relationships from depends_on declarations.
    ///
    /// For each task with depends_on entries, creates:
    ///   (blocked_task)-[:BLOCKED_BY]->(blocking_task)
    /// where blocking_task is the task that must complete first.
    async fn build_dependencies(
        &self,
        flat_tasks: &[FlatTask],
        task_id_map: &HashMap<String, i64>,
    ) -> Result<usize> {
        let mut count: usize = 0;

        for task in flat_tasks {
            if task.delete || task.depends_on.is_empty() {
                continue;
            }
            let task_name = match &task.name {
                Some(name) => name,
                None => continue,
            };

            let blocked_id = match task_id_map.get(task_name) {
                Some(&id) => id,
                None => continue,
            };

            for dep_name in &task.depends_on {
                let blocking_id = task_id_map.get(dep_name).ok_or_else(|| {
                    IntentError::InvalidInput(format!(
                        "Dependency '{}' not found for task '{}'",
                        dep_name, task_name
                    ))
                })?;

                // Check for circular dependency via Neo4j path query:
                // Would creating blocked->BLOCKED_BY->blocking create a cycle?
                if self.would_create_cycle(blocked_id, *blocking_id).await? {
                    return Err(IntentError::CircularDependency {
                        blocking_task_id: *blocking_id,
                        blocked_task_id: blocked_id,
                    });
                }

                // Create BLOCKED_BY relationship (idempotent via MERGE)
                let mut stream = self
                    .graph
                    .execute(
                        query(
                            "MATCH (blocked:Task {project_id: $pid, id: $blocked_id}) \
                             MATCH (blocking:Task {project_id: $pid, id: $blocking_id}) \
                             MERGE (blocked)-[:BLOCKED_BY]->(blocking)",
                        )
                        .param("pid", self.project_id.clone())
                        .param("blocked_id", blocked_id)
                        .param("blocking_id", *blocking_id),
                    )
                    .await
                    .map_err(|e| neo4j_err("build_dependencies", e))?;
                // Consume stream to ensure query executes
                while stream
                    .next()
                    .await
                    .map_err(|e| neo4j_err("build_dependencies consume", e))?
                    .is_some()
                {}

                count += 1;
            }
        }

        Ok(count)
    }

    /// Check if adding a BLOCKED_BY edge from blocked_id to blocking_id
    /// would create a cycle (i.e., blocking_id already transitively depends on blocked_id).
    async fn would_create_cycle(&self, blocked_id: i64, blocking_id: i64) -> Result<bool> {
        if blocked_id == blocking_id {
            return Ok(true);
        }

        // Check: is there already a path from blocking_id to blocked_id via BLOCKED_BY?
        // If so, adding blocked_id->blocking_id would complete a cycle.
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH path = (start:Task {project_id: $pid, id: $blocking_id})\
                     -[:BLOCKED_BY*1..50]->\
                     (end:Task {project_id: $pid, id: $blocked_id}) \
                     RETURN count(path) > 0 AS has_cycle LIMIT 1",
                )
                .param("pid", self.project_id.clone())
                .param("blocking_id", blocking_id)
                .param("blocked_id", blocked_id),
            )
            .await
            .map_err(|e| neo4j_err("would_create_cycle", e))?;

        match result
            .next()
            .await
            .map_err(|e| neo4j_err("would_create_cycle fetch", e))?
        {
            Some(row) => Ok(row.get::<bool>("has_cycle").unwrap_or(false)),
            None => Ok(false),
        }
    }

    /// Find existing tasks by names using batch Cypher query.
    async fn find_tasks_by_names(
        &self,
        names: &[String],
    ) -> Result<HashMap<String, ExistingTaskInfo>> {
        if names.is_empty() {
            return Ok(HashMap::new());
        }

        let mut result = self
            .graph
            .execute(
                query(
                    "UNWIND $names AS name \
                     MATCH (t:Task {project_id: $pid, name: name}) \
                     RETURN t.name AS name, t.id AS id, t.status AS status, t.spec AS spec",
                )
                .param("pid", self.project_id.clone())
                .param(
                    "names",
                    names.iter().map(|n| n.as_str()).collect::<Vec<&str>>(),
                ),
            )
            .await
            .map_err(|e| neo4j_err("find_tasks_by_names", e))?;

        let mut map = HashMap::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| neo4j_err("find_tasks_by_names iterate", e))?
        {
            let name: String = row
                .get("name")
                .map_err(|e| neo4j_err("find_tasks_by_names name", e))?;
            let id: i64 = row
                .get("id")
                .map_err(|e| neo4j_err("find_tasks_by_names id", e))?;
            let status: String = row
                .get("status")
                .map_err(|e| neo4j_err("find_tasks_by_names status", e))?;
            let spec: Option<String> = row.get("spec").ok();

            map.insert(name, ExistingTaskInfo { id, status, spec });
        }

        Ok(map)
    }
}

// Validation functions (validate_dependencies, validate_batch_single_doing,
// detect_circular_dependencies, tarjan_scc) are in crate::plan_validation.
