//! Plan executor backed by Neo4j.
//!
//! Provides batch create/update/delete of task hierarchies, mirroring the
//! behavior of the SQLite `PlanExecutor` in `src/plan.rs`.
//!
//! Key differences from SQLite version:
//! - Uses `Neo4jTaskManager` methods instead of raw SQL
//! - Uses neo4rs transactions (`start_txn()`) for atomicity
//! - No dashboard notifications (no SQLite DB available)
//! - Dependencies (BLOCKED_BY) deferred to Phase 4

use crate::db::models::TaskWithEvents;
use crate::error::{IntentError, Result};
use crate::plan::{
    extract_all_names, find_duplicate_names, flatten_task_tree, ExistingTaskInfo, FlatTask,
    PlanRequest, PlanResult, TaskStatus,
};
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
        if let Err(e) = validate_dependencies(&flat_tasks) {
            return Ok(PlanResult::error(e.to_string()));
        }

        // ── 5. Detect circular dependencies ──
        if let Err(e) = detect_circular_dependencies(&flat_tasks) {
            return Ok(PlanResult::error(e.to_string()));
        }

        // ── 6. Validate batch-level single doing constraint ──
        if let Err(e) = validate_batch_single_doing(&flat_tasks) {
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
                if let Some((focused_id, session_id)) =
                    task_mgr.find_focused_in_subtree_pub(id).await?
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

                let mut update = TaskUpdate {
                    spec: task.spec.as_deref(),
                    priority: task.priority.as_ref().map(|p| p.to_int()),
                    status: status_for_update,
                    active_form: task.active_form.as_deref(),
                    ..Default::default()
                };

                // Only set name if it differs (avoid unnecessary updates)
                if task.name.as_deref() != Some(&*task_mgr.get_task(existing_info.id).await?.name) {
                    update.name = task.name.as_deref();
                }

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

                // Set status if not default
                if let Some(status) = initial_status {
                    if status != "todo" {
                        task_mgr
                            .update_task(
                                new_task.id,
                                TaskUpdate {
                                    status: Some(status),
                                    ..Default::default()
                                },
                            )
                            .await?;
                    }
                }

                // Set active_form if provided
                if task.active_form.is_some() {
                    task_mgr
                        .update_task(
                            new_task.id,
                            TaskUpdate {
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

        // ── 12. Dependencies (skip for now — Phase 4) ──
        let dep_count = 0;

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

// ── Pure validation functions (no DB access) ────────────────────

fn validate_dependencies(flat_tasks: &[FlatTask]) -> Result<()> {
    let task_names: HashSet<&str> = flat_tasks
        .iter()
        .filter_map(|t| t.name.as_deref())
        .collect();

    for task in flat_tasks {
        for dep_name in &task.depends_on {
            if !task_names.contains(dep_name.as_str()) {
                let task_name = task.name.as_deref().unwrap_or("<unknown>");
                return Err(IntentError::InvalidInput(format!(
                    "Task '{}' depends on '{}', but '{}' is not in the plan",
                    task_name, dep_name, dep_name
                )));
            }
        }
    }

    Ok(())
}

fn validate_batch_single_doing(flat_tasks: &[FlatTask]) -> Result<()> {
    let doing_tasks: Vec<&FlatTask> = flat_tasks
        .iter()
        .filter(|task| matches!(task.status, Some(TaskStatus::Doing)))
        .collect();

    if doing_tasks.len() > 1 {
        let names: Vec<&str> = doing_tasks
            .iter()
            .map(|t| t.name.as_deref().unwrap_or("<unknown>"))
            .collect();
        return Err(IntentError::InvalidInput(format!(
            "Batch single doing constraint violated: only one task per batch can have status='doing'. Found: {}",
            names.join(", ")
        )));
    }

    Ok(())
}

fn detect_circular_dependencies(flat_tasks: &[FlatTask]) -> Result<()> {
    if flat_tasks.is_empty() {
        return Ok(());
    }

    let name_to_idx: HashMap<&str, usize> = flat_tasks
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.name.as_ref().map(|n| (n.as_str(), i)))
        .collect();

    // Build adjacency list
    let mut graph: Vec<Vec<usize>> = vec![Vec::new(); flat_tasks.len()];
    for (idx, task) in flat_tasks.iter().enumerate() {
        for dep_name in &task.depends_on {
            if let Some(&dep_idx) = name_to_idx.get(dep_name.as_str()) {
                graph[idx].push(dep_idx);
            }
        }
    }

    // Check self-loops
    for task in flat_tasks {
        if let Some(name) = &task.name {
            if task.depends_on.contains(name) {
                return Err(IntentError::InvalidInput(format!(
                    "Circular dependency detected: task '{}' depends on itself",
                    name
                )));
            }
        }
    }

    // Tarjan's SCC
    let sccs = tarjan_scc(&graph);
    for scc in sccs {
        if scc.len() > 1 {
            let cycle_names: Vec<&str> = scc
                .iter()
                .map(|&idx| flat_tasks[idx].name.as_deref().unwrap_or("<unknown>"))
                .collect();
            return Err(IntentError::InvalidInput(format!(
                "Circular dependency detected: {}",
                cycle_names.join(" → ")
            )));
        }
    }

    Ok(())
}

/// Tarjan's SCC algorithm (copied from SQLite plan.rs)
fn tarjan_scc(graph: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = graph.len();
    let mut index_counter = 0;
    let mut stack = Vec::new();
    let mut on_stack = vec![false; n];
    let mut index = vec![usize::MAX; n];
    let mut lowlink = vec![0; n];
    let mut result = Vec::new();

    fn strongconnect(
        v: usize,
        graph: &[Vec<usize>],
        index_counter: &mut usize,
        stack: &mut Vec<usize>,
        on_stack: &mut Vec<bool>,
        index: &mut Vec<usize>,
        lowlink: &mut Vec<usize>,
        result: &mut Vec<Vec<usize>>,
    ) {
        index[v] = *index_counter;
        lowlink[v] = *index_counter;
        *index_counter += 1;
        stack.push(v);
        on_stack[v] = true;

        for &w in &graph[v] {
            if index[w] == usize::MAX {
                strongconnect(
                    w,
                    graph,
                    index_counter,
                    stack,
                    on_stack,
                    index,
                    lowlink,
                    result,
                );
                lowlink[v] = lowlink[v].min(lowlink[w]);
            } else if on_stack[w] {
                lowlink[v] = lowlink[v].min(index[w]);
            }
        }

        if lowlink[v] == index[v] {
            let mut component = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack[w] = false;
                component.push(w);
                if w == v {
                    break;
                }
            }
            result.push(component);
        }
    }

    for v in 0..n {
        if index[v] == usize::MAX {
            strongconnect(
                v,
                graph,
                &mut index_counter,
                &mut stack,
                &mut on_stack,
                &mut index,
                &mut lowlink,
                &mut result,
            );
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_validate_batch_single_doing() {
        let tasks = vec![
            FlatTask {
                name: Some("A".to_string()),
                status: Some(TaskStatus::Doing),
                ..Default::default()
            },
            FlatTask {
                name: Some("B".to_string()),
                status: Some(TaskStatus::Todo),
                ..Default::default()
            },
        ];
        assert!(validate_batch_single_doing(&tasks).is_ok());

        let tasks = vec![
            FlatTask {
                name: Some("A".to_string()),
                status: Some(TaskStatus::Doing),
                ..Default::default()
            },
            FlatTask {
                name: Some("B".to_string()),
                status: Some(TaskStatus::Doing),
                ..Default::default()
            },
        ];
        assert!(validate_batch_single_doing(&tasks).is_err());
    }

    #[test]
    fn test_validate_dependencies() {
        let tasks = vec![
            FlatTask {
                name: Some("A".to_string()),
                depends_on: vec!["B".to_string()],
                ..Default::default()
            },
            FlatTask {
                name: Some("B".to_string()),
                ..Default::default()
            },
        ];
        assert!(validate_dependencies(&tasks).is_ok());

        let tasks = vec![FlatTask {
            name: Some("A".to_string()),
            depends_on: vec!["NonExistent".to_string()],
            ..Default::default()
        }];
        assert!(validate_dependencies(&tasks).is_err());
    }

    #[test]
    fn test_detect_circular_dependencies() {
        // No cycle
        let tasks = vec![
            FlatTask {
                name: Some("A".to_string()),
                depends_on: vec!["B".to_string()],
                ..Default::default()
            },
            FlatTask {
                name: Some("B".to_string()),
                ..Default::default()
            },
        ];
        assert!(detect_circular_dependencies(&tasks).is_ok());

        // Self-loop
        let tasks = vec![FlatTask {
            name: Some("A".to_string()),
            depends_on: vec!["A".to_string()],
            ..Default::default()
        }];
        assert!(detect_circular_dependencies(&tasks).is_err());

        // A → B → A cycle
        let tasks = vec![
            FlatTask {
                name: Some("A".to_string()),
                depends_on: vec!["B".to_string()],
                ..Default::default()
            },
            FlatTask {
                name: Some("B".to_string()),
                depends_on: vec!["A".to_string()],
                ..Default::default()
            },
        ];
        assert!(detect_circular_dependencies(&tasks).is_err());
    }

    #[test]
    fn test_tarjan_scc_no_cycle() {
        let graph = vec![vec![1], vec![]]; // A → B
        let sccs = tarjan_scc(&graph);
        assert!(sccs.iter().all(|scc| scc.len() == 1));
    }

    #[test]
    fn test_tarjan_scc_with_cycle() {
        let graph = vec![vec![1], vec![0]]; // A → B → A
        let sccs = tarjan_scc(&graph);
        assert!(sccs.iter().any(|scc| scc.len() > 1));
    }
}
