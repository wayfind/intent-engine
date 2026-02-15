//! Plan Interface - Declarative Task Management
//!
//! Provides a declarative API for creating and updating task structures,
//! inspired by TodoWrite pattern. Simplifies complex operations into
//! single atomic calls.

use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use std::path::PathBuf;

/// Request for creating/updating task structure declaratively
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlanRequest {
    /// Task tree to create or update
    pub tasks: Vec<TaskTree>,
}

/// Hierarchical task definition with nested children
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct TaskTree {
    /// Task name (used as identifier for lookups)
    /// Required for create/update operations, optional for delete (when id is provided)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional task specification/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,

    /// Optional priority level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<PriorityValue>,

    /// Nested child tasks (direct hierarchy expression)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TaskTree>>,

    /// Task dependencies by name (name-based references)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,

    /// Optional explicit task ID (for forced updates or delete)
    /// Aliases: "id" or "task_id"
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "task_id")]
    pub id: Option<i64>,

    /// Optional task status (for TodoWriter compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskStatus>,

    /// Optional active form description (for TodoWriter compatibility)
    /// Used for UI display when task is in_progress
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,

    /// Explicit parent task ID
    /// - None: use default behavior (auto-parent to focused task for new root tasks)
    /// - Some(None): explicitly create as root task (no parent)
    /// - Some(Some(id)): explicitly set parent to task with given ID
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_parent_id"
    )]
    pub parent_id: Option<Option<i64>>,

    /// Delete this task (requires id)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete: Option<bool>,
}

/// Custom deserializer for parent_id field
/// Handles the three-state logic:
/// - Field absent → None (handled by #[serde(default)])
/// - Field is null → Some(None) (explicit root task)
/// - Field is number → Some(Some(id)) (explicit parent)
fn deserialize_parent_id<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Option<i64>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // When this function is called, the field EXISTS in the JSON.
    // (Field-absent case is handled by #[serde(default)] returning None)
    //
    // Now we deserialize the value:
    // - null → inner Option is None → we return Some(None)
    // - number → inner Option is Some(n) → we return Some(Some(n))
    let inner: Option<i64> = Option::deserialize(deserializer)?;
    Ok(Some(inner))
}

/// Task status for workflow management
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    Doing,
    Done,
}

impl TaskStatus {
    /// Convert to database string representation
    pub fn as_db_str(&self) -> &'static str {
        match self {
            TaskStatus::Todo => "todo",
            TaskStatus::Doing => "doing",
            TaskStatus::Done => "done",
        }
    }

    /// Create from database string representation
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "todo" => Some(TaskStatus::Todo),
            "doing" => Some(TaskStatus::Doing),
            "done" => Some(TaskStatus::Done),
            _ => None,
        }
    }

    /// Convert to string representation for JSON API
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Todo => "todo",
            TaskStatus::Doing => "doing",
            TaskStatus::Done => "done",
        }
    }
}

/// Priority value as string enum for JSON API
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PriorityValue {
    Critical,
    High,
    Medium,
    Low,
}

impl PriorityValue {
    /// Convert to integer representation for database storage
    pub fn to_int(&self) -> i32 {
        match self {
            PriorityValue::Critical => 1,
            PriorityValue::High => 2,
            PriorityValue::Medium => 3,
            PriorityValue::Low => 4,
        }
    }

    /// Create from integer representation
    pub fn from_int(value: i32) -> Option<Self> {
        match value {
            1 => Some(PriorityValue::Critical),
            2 => Some(PriorityValue::High),
            3 => Some(PriorityValue::Medium),
            4 => Some(PriorityValue::Low),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PriorityValue::Critical => "critical",
            PriorityValue::High => "high",
            PriorityValue::Medium => "medium",
            PriorityValue::Low => "low",
        }
    }
}

/// Information about an existing task for validation
#[derive(Debug, Clone)]
pub struct ExistingTaskInfo {
    pub id: i64,
    pub status: String,
    pub spec: Option<String>,
}

/// Result of plan execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanResult {
    /// Whether the operation succeeded
    pub success: bool,

    /// Mapping of task names to their IDs (for reference)
    pub task_id_map: HashMap<String, i64>,

    /// Number of tasks created
    pub created_count: usize,

    /// Number of tasks updated
    pub updated_count: usize,

    /// Number of tasks directly deleted
    #[serde(default, skip_serializing_if = "is_zero")]
    pub deleted_count: usize,

    /// Number of tasks cascade-deleted (descendants of deleted tasks)
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub cascade_deleted_count: i64,

    /// Number of dependencies created
    pub dependency_count: usize,

    /// Currently focused task (if a task has status="doing")
    /// Includes full task details and event history
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused_task: Option<crate::db::models::TaskWithEvents>,

    /// Optional error message if success = false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Warning messages (non-fatal hints)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

fn is_zero_i64(n: &i64) -> bool {
    *n == 0
}

impl PlanResult {
    /// Create a successful result with optional focused task
    pub fn success(
        task_id_map: HashMap<String, i64>,
        created_count: usize,
        updated_count: usize,
        deleted_count: usize,
        dependency_count: usize,
        focused_task: Option<crate::db::models::TaskWithEvents>,
    ) -> Self {
        Self {
            success: true,
            task_id_map,
            created_count,
            updated_count,
            deleted_count,
            cascade_deleted_count: 0,
            dependency_count,
            focused_task,
            error: None,
            warnings: Vec::new(),
        }
    }

    /// Create a successful result with warnings and cascade delete count
    #[allow(clippy::too_many_arguments)]
    pub fn success_with_warnings(
        task_id_map: HashMap<String, i64>,
        created_count: usize,
        updated_count: usize,
        deleted_count: usize,
        cascade_deleted_count: i64,
        dependency_count: usize,
        focused_task: Option<crate::db::models::TaskWithEvents>,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            success: true,
            task_id_map,
            created_count,
            updated_count,
            deleted_count,
            cascade_deleted_count,
            dependency_count,
            focused_task,
            error: None,
            warnings,
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            task_id_map: HashMap::new(),
            created_count: 0,
            updated_count: 0,
            deleted_count: 0,
            cascade_deleted_count: 0,
            dependency_count: 0,
            focused_task: None,
            error: Some(message.into()),
            warnings: Vec::new(),
        }
    }
}

// ============================================================================
// Name Extraction and Classification Logic
// ============================================================================

/// Extract all task names from a task tree (recursive)
/// Only includes tasks that have a name (skips delete-only tasks)
pub fn extract_all_names(tasks: &[TaskTree]) -> Vec<String> {
    let mut names = Vec::new();

    for task in tasks {
        if let Some(name) = &task.name {
            names.push(name.clone());
        }

        if let Some(children) = &task.children {
            names.extend(extract_all_names(children));
        }
    }

    names
}

/// Flatten task tree into a linear list with parent information
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlatTask {
    /// Task name (None for delete-only operations)
    pub name: Option<String>,
    pub spec: Option<String>,
    pub priority: Option<PriorityValue>,
    /// Parent from children nesting (takes precedence over explicit_parent_id)
    pub parent_name: Option<String>,
    pub depends_on: Vec<String>,
    /// Task ID for updates or deletes
    pub id: Option<i64>,
    pub status: Option<TaskStatus>,
    pub active_form: Option<String>,
    /// Explicit parent_id from JSON
    /// - None: use default behavior (auto-parent to focused task for new root tasks)
    /// - Some(None): explicitly create as root task (no parent)
    /// - Some(Some(id)): explicitly set parent to task with given ID
    pub explicit_parent_id: Option<Option<i64>>,
    /// Delete this task
    pub delete: bool,
}

pub fn flatten_task_tree(tasks: &[TaskTree]) -> Vec<FlatTask> {
    flatten_task_tree_recursive(tasks, None)
}

fn flatten_task_tree_recursive(tasks: &[TaskTree], parent_name: Option<String>) -> Vec<FlatTask> {
    let mut flat = Vec::new();

    for task in tasks {
        let flat_task = FlatTask {
            name: task.name.clone(),
            spec: task.spec.clone(),
            priority: task.priority.clone(),
            parent_name: parent_name.clone(),
            depends_on: task.depends_on.clone().unwrap_or_default(),
            id: task.id,
            status: task.status.clone(),
            active_form: task.active_form.clone(),
            explicit_parent_id: task.parent_id,
            delete: task.delete.unwrap_or(false),
        };

        flat.push(flat_task);

        // Recursively flatten children (only if task has a name)
        if let Some(children) = &task.children {
            if let Some(name) = &task.name {
                flat.extend(flatten_task_tree_recursive(children, Some(name.clone())));
            }
        }
    }

    flat
}

/// Operation classification result
#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Create(FlatTask),
    Update { id: i64, task: FlatTask },
    Delete { id: i64 },
}

/// Classify tasks into create/update/delete operations based on existing task IDs
///
/// # Arguments
/// * `flat_tasks` - Flattened task list
/// * `existing_names` - Map of existing task names to their IDs
///
/// # Returns
/// Classified operations (create, update, or delete)
pub fn classify_operations(
    flat_tasks: &[FlatTask],
    existing_names: &HashMap<String, i64>,
) -> Vec<Operation> {
    let mut operations = Vec::new();

    for task in flat_tasks {
        // Handle delete operations (requires explicit id)
        if task.delete {
            if let Some(id) = task.id {
                operations.push(Operation::Delete { id });
            }
            // Skip delete without id (validation should catch this earlier)
            continue;
        }

        // Priority: explicit id > name lookup > create
        let operation = if let Some(id) = task.id {
            // Explicit id → forced update
            Operation::Update {
                id,
                task: task.clone(),
            }
        } else if let Some(name) = &task.name {
            // Try name lookup
            if let Some(&id) = existing_names.get(name) {
                // Name found in DB → update
                Operation::Update {
                    id,
                    task: task.clone(),
                }
            } else {
                // Not found → create
                Operation::Create(task.clone())
            }
        } else {
            // No id and no name → skip (invalid)
            continue;
        };

        operations.push(operation);
    }

    operations
}

/// Find duplicate names in a task list
pub fn find_duplicate_names(tasks: &[TaskTree]) -> Vec<String> {
    let mut seen = HashMap::new();
    let mut duplicates = Vec::new();

    for name in extract_all_names(tasks) {
        let count = seen.entry(name.clone()).or_insert(0);
        *count += 1;
        if *count == 2 {
            // Only add once when we first detect the duplicate
            duplicates.push(name);
        }
    }

    duplicates
}

// ============================================================================
// Database Operations (Plan Executor)
// ============================================================================

use crate::error::{IntentError, Result};
use sqlx::SqlitePool;

/// Plan executor for creating/updating task structures
pub struct PlanExecutor<'a> {
    pool: &'a SqlitePool,
    project_path: Option<String>,
    /// Default parent ID for root-level tasks (auto-parenting to focused task)
    default_parent_id: Option<i64>,
}

impl<'a> PlanExecutor<'a> {
    /// Create a new plan executor
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self {
            pool,
            project_path: None,
            default_parent_id: None,
        }
    }

    /// Create a plan executor with project path for dashboard notifications
    pub fn with_project_path(pool: &'a SqlitePool, project_path: String) -> Self {
        Self {
            pool,
            project_path: Some(project_path),
            default_parent_id: None,
        }
    }

    /// Set default parent ID for root-level tasks (auto-parenting to focused task)
    /// When set, new root-level tasks will automatically become children of this task
    pub fn with_default_parent(mut self, parent_id: i64) -> Self {
        self.default_parent_id = Some(parent_id);
        self
    }

    /// Get TaskManager configured for this executor
    fn get_task_manager(&self) -> crate::tasks::TaskManager<'a> {
        match &self.project_path {
            Some(path) => crate::tasks::TaskManager::with_project_path(self.pool, path.clone()),
            None => crate::tasks::TaskManager::new(self.pool),
        }
    }

    /// Execute a plan request (Phase 2: create + update mode)
    #[tracing::instrument(skip(self, request), fields(task_count = request.tasks.len()))]
    pub async fn execute(&self, request: &PlanRequest) -> Result<PlanResult> {
        // 1. Check for duplicate names in the request
        let duplicates = find_duplicate_names(&request.tasks);
        if !duplicates.is_empty() {
            return Ok(PlanResult::error(format!(
                "Duplicate task names in request: {:?}",
                duplicates
            )));
        }

        // 2. Extract all task names
        let all_names = extract_all_names(&request.tasks);

        // 3. Find existing tasks by name
        let existing = self.find_tasks_by_names(&all_names).await?;

        // 4. Flatten the task tree
        let flat_tasks = flatten_task_tree(&request.tasks);

        // 5. Validate dependencies exist in the plan
        if let Err(e) = self.validate_dependencies(&flat_tasks) {
            return Ok(PlanResult::error(e.to_string()));
        }

        // 6. Detect circular dependencies
        if let Err(e) = self.detect_circular_dependencies(&flat_tasks) {
            return Ok(PlanResult::error(e.to_string()));
        }

        // 7. Validate batch-level single doing constraint
        if let Err(e) = self.validate_batch_single_doing(&flat_tasks) {
            return Ok(PlanResult::error(e.to_string()));
        }

        // 8. Get TaskManager for transaction operations
        let task_mgr = self.get_task_manager();

        // 9. Execute in transaction
        let mut tx = self.pool.begin().await?;

        // 10. Create or update tasks based on existence
        let mut task_id_map = HashMap::new();
        let mut created_count = 0;
        let mut updated_count = 0;
        let mut warnings: Vec<String> = Vec::new();
        let mut newly_created_names: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut deleted_count = 0;

        // ============================================================================
        // Delete Operations (processed first, before create/update)
        // ============================================================================
        // Delete operations are separated from normal operations for several reasons:
        // 1. Deletes should happen first to avoid conflicts with creates/updates
        // 2. Non-existent IDs should generate warnings, not errors (idempotent)
        // 3. Cascade deletes (due to ON DELETE CASCADE on parent_id) are tracked
        // 4. Deleting focused task should warn the user

        // Separate delete operations from normal operations
        let (delete_tasks, normal_tasks): (Vec<_>, Vec<_>) =
            flat_tasks.iter().partition(|t| t.delete);

        // Validate delete operations: each must have an id
        for task in &delete_tasks {
            if task.id.is_none() {
                return Ok(PlanResult::error(
                    "Delete operation requires 'id' field. Use {\"id\": <task_id>, \"delete\": true}",
                ));
            }
        }

        // Process delete operations first
        // ============================================================================
        // Focus Protection Check (BEFORE any deletions)
        // ============================================================================
        // We must check ALL delete targets and their subtrees for focus BEFORE
        // executing any deletes. This prevents:
        // 1. Direct deletion of focused task
        // 2. CASCADE deletion of focused task via parent deletion
        // 3. Batch delete order tricks (deleting parent before checking child)
        //
        // Rationale: Focus represents "commitment to complete". Deleting it
        // without explicitly switching focus is semantically incomplete.
        for task in &delete_tasks {
            if let Some(id) = task.id {
                // Check entire subtree (task + all descendants) for focus in ANY session
                if let Some((focused_id, session_id)) =
                    task_mgr.find_focused_in_subtree_in_tx(&mut tx, id).await?
                {
                    if focused_id == id {
                        // Direct deletion of focused task
                        return Ok(PlanResult::error(format!(
                            "Task #{} is the current focus of session '{}'. That session must switch focus first.",
                            id, session_id
                        )));
                    } else {
                        // Cascade would delete focused task
                        return Ok(PlanResult::error(format!(
                            "Task #{} is the current focus of session '{}' and would be deleted by cascade (descendant of #{}). That session must switch focus first.",
                            focused_id, session_id, id
                        )));
                    }
                }
            }
        }

        // ============================================================================
        // Execute Deletions (focus protection already verified)
        // ============================================================================
        let mut cascade_deleted_count: i64 = 0;
        for task in &delete_tasks {
            if let Some(id) = task.id {
                let delete_result = task_mgr.delete_task_in_tx(&mut tx, id).await?;

                if !delete_result.found {
                    // Task doesn't exist - generate warning but don't fail
                    // This ensures idempotent behavior: deleting already-deleted task is OK
                    warnings.push(format!(
                        "Task #{} not found (may have been already deleted)",
                        id
                    ));
                } else {
                    deleted_count += 1;

                    // Track cascade-deleted descendants (due to ON DELETE CASCADE)
                    if delete_result.descendant_count > 0 {
                        cascade_deleted_count += delete_result.descendant_count;
                        warnings.push(format!(
                            "Task #{} had {} descendant(s) that were also deleted (cascade)",
                            id, delete_result.descendant_count
                        ));
                    }
                }
            }
        }

        // ============================================================================
        // Create/Update Operations
        // ============================================================================

        // Process normal operations (create/update)
        for task in &normal_tasks {
            // Skip tasks without name (shouldn't happen for normal operations)
            let task_name = match &task.name {
                Some(name) => name,
                None => continue, // Skip invalid entries
            };

            // Check if task is transitioning to 'doing' status and validate spec
            let is_becoming_doing = task.status.as_ref() == Some(&TaskStatus::Doing);
            let has_spec = task
                .spec
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

            if let Some(existing_info) = existing.get(task_name) {
                // Task exists -> UPDATE

                // Validation: if transitioning to 'doing' and no spec provided,
                // check if existing spec is also empty
                if is_becoming_doing && !has_spec {
                    let existing_is_doing = existing_info.status == "doing";
                    let existing_has_spec = existing_info
                        .spec
                        .as_ref()
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);

                    // Only error if: transitioning TO doing (wasn't doing before) AND no spec anywhere
                    if !existing_is_doing && !existing_has_spec {
                        return Ok(PlanResult::error(format!(
                            "Task '{}': spec (description) is required when starting a task (status: doing).\n\n\
                            Before starting a task, please describe:\n  \
                            • What is the goal of this task\n  \
                            • How do you plan to approach it\n\n\
                            Tip: Use @file(path) to include content from a file",
                            task_name
                        )));
                    }
                }

                // Check if transitioning to 'done'
                let is_becoming_done = task.status.as_ref() == Some(&TaskStatus::Done);

                // Update non-status fields first
                task_mgr
                    .update_task_in_tx(
                        &mut tx,
                        existing_info.id,
                        task.spec.as_deref(),
                        task.priority.as_ref().map(|p| p.to_int()),
                        // If becoming done, let complete_task_in_tx handle status
                        if is_becoming_done {
                            None
                        } else {
                            task.status.as_ref().map(|s| s.as_db_str())
                        },
                        task.active_form.as_deref(),
                    )
                    .await?;

                // If becoming done, use complete_task_in_tx for business logic
                if is_becoming_done {
                    if let Err(e) = task_mgr
                        .complete_task_in_tx(&mut tx, existing_info.id)
                        .await
                    {
                        // Convert IntentError to user-friendly message
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
                // Task doesn't exist -> CREATE

                // Validation: new task with status=doing must have spec
                if is_becoming_doing && !has_spec {
                    return Ok(PlanResult::error(format!(
                        "Task '{}': spec (description) is required when starting a task (status: doing).\n\n\
                        Before starting a task, please describe:\n  \
                        • What is the goal of this task\n  \
                        • How do you plan to approach it\n\n\
                        Tip: Use @file(path) to include content from a file",
                        task_name
                    )));
                }

                let id = task_mgr
                    .create_task_in_tx(
                        &mut tx,
                        task_name,
                        task.spec.as_deref(),
                        task.priority.as_ref().map(|p| p.to_int()),
                        task.status.as_ref().map(|s| s.as_db_str()),
                        task.active_form.as_deref(),
                        "ai", // Plan-created tasks are AI-owned
                    )
                    .await?;
                task_id_map.insert(task_name.clone(), id);
                newly_created_names.insert(task_name.clone());
                created_count += 1;

                // Warning: new task without spec (non-doing tasks only, doing already validated)
                if !has_spec && !is_becoming_doing {
                    warnings.push(format!(
                        "Task '{}' has no description. Consider adding one for better context.",
                        task_name
                    ));
                }
            }
        }

        // 11. Build parent-child relationships via TaskManager (only for normal tasks)
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
                        .set_parent_in_tx(&mut tx, *task_id, *parent_id)
                        .await?;
                }
            }
        }

        // 11b. Handle explicit parent_id (takes precedence over auto-parenting)
        // Priority: children nesting > explicit parent_id > auto-parent
        for task in &normal_tasks {
            // Skip if parent was set via children nesting
            if task.parent_name.is_some() {
                continue;
            }

            // Handle explicit parent_id
            if let Some(explicit_parent) = &task.explicit_parent_id {
                if let Some(task_name) = &task.name {
                    let task_id = task_id_map.get(task_name).ok_or_else(|| {
                        IntentError::InvalidInput(format!("Task not found: {}", task_name))
                    })?;

                    match explicit_parent {
                        None => {
                            // parent_id: null → explicitly set as root task (clear parent)
                            task_mgr.clear_parent_in_tx(&mut tx, *task_id).await?;
                        },
                        Some(parent_id) => {
                            // parent_id: N → set parent to task N (validate exists)
                            // Note: parent task may be in this batch or already in DB
                            task_mgr
                                .set_parent_in_tx(&mut tx, *task_id, *parent_id)
                                .await?;
                        },
                    }
                }
            }
        }

        // 11c. Auto-parent newly created root tasks to default_parent_id (focused task)
        if let Some(default_parent) = self.default_parent_id {
            for task in &normal_tasks {
                // Only auto-parent if:
                // 1. Task was newly created (not updated)
                // 2. Task has no explicit parent in the plan (children nesting)
                // 3. Task has no explicit parent_id in JSON
                if let Some(task_name) = &task.name {
                    if newly_created_names.contains(task_name)
                        && task.parent_name.is_none()
                        && task.explicit_parent_id.is_none()
                    {
                        if let Some(&task_id) = task_id_map.get(task_name) {
                            task_mgr
                                .set_parent_in_tx(&mut tx, task_id, default_parent)
                                .await?;
                        }
                    }
                }
            }
        }

        // 12. Build dependencies
        let dep_count = self
            .build_dependencies(&mut tx, &flat_tasks, &task_id_map)
            .await?;

        // 13. Commit transaction
        tx.commit().await?;

        // 14. Notify Dashboard about the batch change (via TaskManager)
        task_mgr.notify_batch_changed().await;

        // 15. Auto-focus the doing task if present and return full context
        // Find the doing task in the batch (only from normal tasks, not deletes)
        let doing_task = normal_tasks
            .iter()
            .find(|task| matches!(task.status, Some(TaskStatus::Doing)));

        let focused_task_response = if let Some(doing_task) = doing_task {
            // Get the task ID from the map
            if let Some(task_name) = &doing_task.name {
                if let Some(&task_id) = task_id_map.get(task_name) {
                    // Call task_start with events to get full context
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

        // 16. Return success result with focused task and warnings
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

    /// Find tasks by names (returns full info for validation)
    async fn find_tasks_by_names(
        &self,
        names: &[String],
    ) -> Result<HashMap<String, ExistingTaskInfo>> {
        if names.is_empty() {
            return Ok(HashMap::new());
        }

        let mut map = HashMap::new();

        // Query all names at once using IN clause
        // Build placeholders: ?, ?, ?...
        let placeholders = names.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT id, name, status, spec FROM tasks WHERE name IN ({})",
            placeholders
        );

        let mut query_builder = sqlx::query(&query);
        for name in names {
            query_builder = query_builder.bind(name);
        }

        let rows = query_builder.fetch_all(self.pool).await?;

        for row in rows {
            let id: i64 = row.get("id");
            let name: String = row.get("name");
            let status: String = row.get("status");
            let spec: Option<String> = row.get("spec");
            map.insert(name, ExistingTaskInfo { id, status, spec });
        }

        Ok(map)
    }

    /// Build dependency relationships
    async fn build_dependencies(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        flat_tasks: &[FlatTask],
        task_id_map: &HashMap<String, i64>,
    ) -> Result<usize> {
        let mut count = 0;

        for task in flat_tasks {
            // Skip delete operations and tasks without names
            if task.delete {
                continue;
            }
            let task_name = match &task.name {
                Some(name) => name,
                None => continue,
            };

            if !task.depends_on.is_empty() {
                let blocked_id = task_id_map.get(task_name).ok_or_else(|| {
                    IntentError::InvalidInput(format!("Task not found: {}", task_name))
                })?;

                for dep_name in &task.depends_on {
                    let blocking_id = task_id_map.get(dep_name).ok_or_else(|| {
                        IntentError::InvalidInput(format!(
                            "Dependency '{}' not found for task '{}'",
                            dep_name, task_name
                        ))
                    })?;

                    sqlx::query(
                        "INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)",
                    )
                    .bind(blocking_id)
                    .bind(blocked_id)
                    .execute(&mut **tx)
                    .await?;

                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Validate that all dependencies exist in the plan
    fn validate_dependencies(&self, flat_tasks: &[FlatTask]) -> Result<()> {
        crate::plan_validation::validate_dependencies(flat_tasks)
    }

    fn validate_batch_single_doing(&self, flat_tasks: &[FlatTask]) -> Result<()> {
        crate::plan_validation::validate_batch_single_doing(flat_tasks)
    }

    fn detect_circular_dependencies(&self, flat_tasks: &[FlatTask]) -> Result<()> {
        crate::plan_validation::detect_circular_dependencies(flat_tasks)
    }
}

/// Result of processing @file directives in a PlanRequest
#[derive(Debug, Default)]
pub struct FileIncludeResult {
    /// Files to delete after successful plan execution
    pub files_to_delete: Vec<PathBuf>,
}

/// Parse @file directive from a string value
///
/// Syntax: `@file(path)` or `@file(path, keep)`
///
/// Returns: (file_path, should_delete)
fn parse_file_directive(value: &str) -> Option<(PathBuf, bool)> {
    let trimmed = value.trim();

    // Must start with @file( and end with )
    if !trimmed.starts_with("@file(") || !trimmed.ends_with(')') {
        return None;
    }

    // Extract content between @file( and )
    let inner = &trimmed[6..trimmed.len() - 1];

    // Check for ", keep" suffix
    if let Some(path_str) = inner.strip_suffix(", keep") {
        Some((PathBuf::from(path_str.trim()), false)) // keep = don't delete
    } else if let Some(path_str) = inner.strip_suffix(",keep") {
        Some((PathBuf::from(path_str.trim()), false))
    } else {
        Some((PathBuf::from(inner.trim()), true)) // default = delete
    }
}

/// Process @file directives in a TaskTree recursively
fn process_task_tree_includes(
    task: &mut TaskTree,
    files_to_delete: &mut Vec<PathBuf>,
) -> std::result::Result<(), String> {
    // Process spec field
    if let Some(ref spec_value) = task.spec {
        if let Some((file_path, should_delete)) = parse_file_directive(spec_value) {
            // Read file content
            let content = std::fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read @file({}): {}", file_path.display(), e))?;

            task.spec = Some(content);

            if should_delete {
                files_to_delete.push(file_path);
            }
        }
    }

    // Process children recursively
    if let Some(ref mut children) = task.children {
        for child in children.iter_mut() {
            process_task_tree_includes(child, files_to_delete)?;
        }
    }

    Ok(())
}

/// Process @file directives in a PlanRequest
///
/// This function scans all task specs for @file(path) syntax and replaces
/// them with the file contents. Files are tracked for deletion after
/// successful plan execution.
///
/// # Syntax
///
/// - `@file(/path/to/file.md)` - Include file content, delete after success
/// - `@file(/path/to/file.md, keep)` - Include file content, keep the file
///
/// # Example
///
/// ```json
/// {
///   "tasks": [{
///     "name": "My Task",
///     "spec": "@file(/tmp/task-description.md)"
///   }]
/// }
/// ```
pub fn process_file_includes(
    request: &mut PlanRequest,
) -> std::result::Result<FileIncludeResult, String> {
    let mut result = FileIncludeResult::default();

    for task in request.tasks.iter_mut() {
        process_task_tree_includes(task, &mut result.files_to_delete)?;
    }

    Ok(result)
}

/// Clean up files that were included via @file directive
pub fn cleanup_included_files(files: &[PathBuf]) {
    for file in files {
        if let Err(e) = std::fs::remove_file(file) {
            // Log warning but don't fail - the plan already succeeded
            tracing::warn!("Failed to delete included file {}: {}", file.display(), e);
        }
    }
}

impl crate::backend::PlanBackend for PlanExecutor<'_> {
    fn execute(
        &self,
        request: &PlanRequest,
    ) -> impl std::future::Future<Output = crate::error::Result<PlanResult>> + Send {
        self.execute(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_value_to_int() {
        assert_eq!(PriorityValue::Critical.to_int(), 1);
        assert_eq!(PriorityValue::High.to_int(), 2);
        assert_eq!(PriorityValue::Medium.to_int(), 3);
        assert_eq!(PriorityValue::Low.to_int(), 4);
    }

    #[test]
    fn test_priority_value_from_int() {
        assert_eq!(PriorityValue::from_int(1), Some(PriorityValue::Critical));
        assert_eq!(PriorityValue::from_int(2), Some(PriorityValue::High));
        assert_eq!(PriorityValue::from_int(3), Some(PriorityValue::Medium));
        assert_eq!(PriorityValue::from_int(4), Some(PriorityValue::Low));
        assert_eq!(PriorityValue::from_int(999), None);
    }

    #[test]
    fn test_priority_value_as_str() {
        assert_eq!(PriorityValue::Critical.as_str(), "critical");
        assert_eq!(PriorityValue::High.as_str(), "high");
        assert_eq!(PriorityValue::Medium.as_str(), "medium");
        assert_eq!(PriorityValue::Low.as_str(), "low");
    }

    #[test]
    fn test_plan_request_deserialization_minimal() {
        let json = r#"{"tasks": [{"name": "Test Task"}]}"#;
        let request: PlanRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.tasks.len(), 1);
        assert_eq!(request.tasks[0].name, Some("Test Task".to_string()));
        assert_eq!(request.tasks[0].spec, None);
        assert_eq!(request.tasks[0].priority, None);
        assert_eq!(request.tasks[0].children, None);
        assert_eq!(request.tasks[0].depends_on, None);
        assert_eq!(request.tasks[0].id, None);
    }

    #[test]
    fn test_plan_request_deserialization_full() {
        let json = r#"{
            "tasks": [{
                "name": "Parent Task",
                "spec": "Parent spec",
                "priority": "high",
                "children": [{
                    "name": "Child Task",
                    "spec": "Child spec"
                }],
                "depends_on": ["Other Task"],
                "task_id": 42
            }]
        }"#;

        let request: PlanRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.tasks.len(), 1);
        let parent = &request.tasks[0];
        assert_eq!(parent.name, Some("Parent Task".to_string()));
        assert_eq!(parent.spec, Some("Parent spec".to_string()));
        assert_eq!(parent.priority, Some(PriorityValue::High));
        assert_eq!(parent.id, Some(42));

        let children = parent.children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, Some("Child Task".to_string()));

        let depends = parent.depends_on.as_ref().unwrap();
        assert_eq!(depends.len(), 1);
        assert_eq!(depends[0], "Other Task");
    }

    #[test]
    fn test_plan_request_serialization() {
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Test Task".to_string()),
                spec: Some("Test spec".to_string()),
                priority: Some(PriorityValue::Medium),
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"name\":\"Test Task\""));
        assert!(json.contains("\"spec\":\"Test spec\""));
        assert!(json.contains("\"priority\":\"medium\""));
    }

    #[test]
    fn test_plan_result_success() {
        let mut map = HashMap::new();
        map.insert("Task 1".to_string(), 1);
        map.insert("Task 2".to_string(), 2);

        let result = PlanResult::success(map.clone(), 2, 0, 0, 1, None);

        assert!(result.success);
        assert_eq!(result.task_id_map, map);
        assert_eq!(result.created_count, 2);
        assert_eq!(result.updated_count, 0);
        assert_eq!(result.dependency_count, 1);
        assert_eq!(result.focused_task, None);
        assert_eq!(result.error, None);
    }

    #[test]
    fn test_plan_result_error() {
        let result = PlanResult::error("Test error");

        assert!(!result.success);
        assert_eq!(result.task_id_map.len(), 0);
        assert_eq!(result.created_count, 0);
        assert_eq!(result.updated_count, 0);
        assert_eq!(result.dependency_count, 0);
        assert_eq!(result.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_task_tree_nested() {
        let tree = TaskTree {
            name: Some("Parent".to_string()),
            spec: None,
            priority: None,
            children: Some(vec![
                TaskTree {
                    name: Some("Child 1".to_string()),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Child 2".to_string()),
                    spec: None,
                    priority: Some(PriorityValue::High),
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
            ]),
            depends_on: None,
            id: None,
            status: None,
            active_form: None,
            parent_id: None,
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&tree).unwrap();
        let deserialized: TaskTree = serde_json::from_str(&json).unwrap();

        assert_eq!(tree, deserialized);
        assert_eq!(deserialized.children.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_priority_value_case_insensitive_deserialization() {
        // Test lowercase
        let json = r#"{"name": "Test", "priority": "high"}"#;
        let task: TaskTree = serde_json::from_str(json).unwrap();
        assert_eq!(task.priority, Some(PriorityValue::High));

        // Serde expects exact case match for rename_all = "lowercase"
        // So "High" would fail, which is correct behavior
    }

    #[test]
    fn test_extract_all_names_simple() {
        let tasks = vec![
            TaskTree {
                name: Some("Task 1".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            },
            TaskTree {
                name: Some("Task 2".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            },
        ];

        let names = extract_all_names(&tasks);
        assert_eq!(names, vec!["Task 1", "Task 2"]);
    }

    #[test]
    fn test_extract_all_names_nested() {
        let tasks = vec![TaskTree {
            name: Some("Parent".to_string()),
            spec: None,
            priority: None,
            children: Some(vec![
                TaskTree {
                    name: Some("Child 1".to_string()),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Child 2".to_string()),
                    spec: None,
                    priority: None,
                    children: Some(vec![TaskTree {
                        name: Some("Grandchild".to_string()),
                        spec: None,
                        priority: None,
                        children: None,
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    }]),
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
            ]),
            depends_on: None,
            id: None,
            status: None,
            active_form: None,
            parent_id: None,
            ..Default::default()
        }];

        let names = extract_all_names(&tasks);
        assert_eq!(names, vec!["Parent", "Child 1", "Child 2", "Grandchild"]);
    }

    #[test]
    fn test_flatten_task_tree_simple() {
        let tasks = vec![TaskTree {
            name: Some("Task 1".to_string()),
            spec: Some("Spec 1".to_string()),
            priority: Some(PriorityValue::High),
            children: None,
            depends_on: Some(vec!["Task 0".to_string()]),
            id: None,
            status: None,
            active_form: None,
            parent_id: None,
            ..Default::default()
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].name, Some("Task 1".to_string()));
        assert_eq!(flat[0].spec, Some("Spec 1".to_string()));
        assert_eq!(flat[0].priority, Some(PriorityValue::High));
        assert_eq!(flat[0].parent_name, None);
        assert_eq!(flat[0].depends_on, vec!["Task 0"]);
    }

    #[test]
    fn test_flatten_task_tree_nested() {
        let tasks = vec![TaskTree {
            name: Some("Parent".to_string()),
            spec: None,
            priority: None,
            children: Some(vec![
                TaskTree {
                    name: Some("Child 1".to_string()),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Child 2".to_string()),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
            ]),
            depends_on: None,
            id: None,
            status: None,
            active_form: None,
            parent_id: None,
            ..Default::default()
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 3);

        // Parent should have no parent_name
        assert_eq!(flat[0].name, Some("Parent".to_string()));
        assert_eq!(flat[0].parent_name, None);

        // Children should have Parent as parent_name
        assert_eq!(flat[1].name, Some("Child 1".to_string()));
        assert_eq!(flat[1].parent_name, Some("Parent".to_string()));

        assert_eq!(flat[2].name, Some("Child 2".to_string()));
        assert_eq!(flat[2].parent_name, Some("Parent".to_string()));
    }

    #[test]
    fn test_classify_operations_all_create() {
        let flat_tasks = vec![
            FlatTask {
                name: Some("Task 1".to_string()),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
                ..Default::default()
            },
            FlatTask {
                name: Some("Task 2".to_string()),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
                ..Default::default()
            },
        ];

        let existing = HashMap::new();
        let operations = classify_operations(&flat_tasks, &existing);

        assert_eq!(operations.len(), 2);
        assert!(matches!(operations[0], Operation::Create(_)));
        assert!(matches!(operations[1], Operation::Create(_)));
    }

    #[test]
    fn test_classify_operations_all_update() {
        let flat_tasks = vec![
            FlatTask {
                name: Some("Task 1".to_string()),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
                ..Default::default()
            },
            FlatTask {
                name: Some("Task 2".to_string()),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
                ..Default::default()
            },
        ];

        let mut existing = HashMap::new();
        existing.insert("Task 1".to_string(), 1);
        existing.insert("Task 2".to_string(), 2);

        let operations = classify_operations(&flat_tasks, &existing);

        assert_eq!(operations.len(), 2);
        assert!(matches!(operations[0], Operation::Update { id: 1, .. }));
        assert!(matches!(operations[1], Operation::Update { id: 2, .. }));
    }

    #[test]
    fn test_classify_operations_mixed() {
        let flat_tasks = vec![
            FlatTask {
                name: Some("Existing Task".to_string()),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
                ..Default::default()
            },
            FlatTask {
                name: Some("New Task".to_string()),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
                ..Default::default()
            },
        ];

        let mut existing = HashMap::new();
        existing.insert("Existing Task".to_string(), 42);

        let operations = classify_operations(&flat_tasks, &existing);

        assert_eq!(operations.len(), 2);
        assert!(matches!(operations[0], Operation::Update { id: 42, .. }));
        assert!(matches!(operations[1], Operation::Create(_)));
    }

    #[test]
    fn test_classify_operations_explicit_task_id() {
        let flat_tasks = vec![FlatTask {
            name: Some("Task".to_string()),
            spec: None,
            priority: None,
            parent_name: None,
            depends_on: vec![],
            id: Some(99), // Explicit task_id
            status: None,
            active_form: None,
            explicit_parent_id: None,
            ..Default::default()
        }];

        let existing = HashMap::new(); // Not in existing

        let operations = classify_operations(&flat_tasks, &existing);

        // Should still be update because of explicit task_id
        assert_eq!(operations.len(), 1);
        assert!(matches!(operations[0], Operation::Update { id: 99, .. }));
    }

    #[test]
    fn test_find_duplicate_names_no_duplicates() {
        let tasks = vec![
            TaskTree {
                name: Some("Task 1".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            },
            TaskTree {
                name: Some("Task 2".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            },
        ];

        let duplicates = find_duplicate_names(&tasks);
        assert_eq!(duplicates.len(), 0);
    }

    #[test]
    fn test_find_duplicate_names_with_duplicates() {
        let tasks = vec![
            TaskTree {
                name: Some("Duplicate".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            },
            TaskTree {
                name: Some("Unique".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            },
            TaskTree {
                name: Some("Duplicate".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            },
        ];

        let duplicates = find_duplicate_names(&tasks);
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0], "Duplicate");
    }

    #[test]
    fn test_find_duplicate_names_nested() {
        let tasks = vec![TaskTree {
            name: Some("Parent".to_string()),
            spec: None,
            priority: None,
            children: Some(vec![TaskTree {
                name: Some("Parent".to_string()), // Duplicate name in child
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }]),
            depends_on: None,
            id: None,
            status: None,
            active_form: None,
            parent_id: None,
            ..Default::default()
        }];

        let duplicates = find_duplicate_names(&tasks);
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0], "Parent");
    }

    #[test]
    fn test_flatten_task_tree_empty() {
        let tasks: Vec<TaskTree> = vec![];
        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 0);
    }

    #[test]
    fn test_flatten_task_tree_deep_nesting() {
        // Create 4-level deep nesting: Root -> L1 -> L2 -> L3
        let tasks = vec![TaskTree {
            name: Some("Root".to_string()),
            spec: None,
            priority: None,
            children: Some(vec![TaskTree {
                name: Some("Level1".to_string()),
                spec: None,
                priority: None,
                children: Some(vec![TaskTree {
                    name: Some("Level2".to_string()),
                    spec: None,
                    priority: None,
                    children: Some(vec![TaskTree {
                        name: Some("Level3".to_string()),
                        spec: None,
                        priority: None,
                        children: None,
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    }]),
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                }]),
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }]),
            depends_on: None,
            id: None,
            status: None,
            active_form: None,
            parent_id: None,
            ..Default::default()
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 4);

        // Check parent relationships
        assert_eq!(flat[0].name, Some("Root".to_string()));
        assert_eq!(flat[0].parent_name, None);

        assert_eq!(flat[1].name, Some("Level1".to_string()));
        assert_eq!(flat[1].parent_name, Some("Root".to_string()));

        assert_eq!(flat[2].name, Some("Level2".to_string()));
        assert_eq!(flat[2].parent_name, Some("Level1".to_string()));

        assert_eq!(flat[3].name, Some("Level3".to_string()));
        assert_eq!(flat[3].parent_name, Some("Level2".to_string()));
    }

    #[test]
    fn test_flatten_task_tree_many_siblings() {
        let children: Vec<TaskTree> = (0..10)
            .map(|i| TaskTree {
                name: Some(format!("Child {}", i)),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            })
            .collect();

        let tasks = vec![TaskTree {
            name: Some("Parent".to_string()),
            spec: None,
            priority: None,
            children: Some(children),
            depends_on: None,
            id: None,
            status: None,
            active_form: None,
            parent_id: None,
            ..Default::default()
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 11); // 1 parent + 10 children

        // All children should have same parent
        for child in flat.iter().skip(1).take(10) {
            assert_eq!(child.parent_name, Some("Parent".to_string()));
        }
    }

    #[test]
    fn test_flatten_task_tree_complex_mixed() {
        // Complex structure with multiple levels and siblings
        let tasks = vec![
            TaskTree {
                name: Some("Task 1".to_string()),
                spec: None,
                priority: None,
                children: Some(vec![
                    TaskTree {
                        name: Some("Task 1.1".to_string()),
                        spec: None,
                        priority: None,
                        children: None,
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    },
                    TaskTree {
                        name: Some("Task 1.2".to_string()),
                        spec: None,
                        priority: None,
                        children: Some(vec![TaskTree {
                            name: Some("Task 1.2.1".to_string()),
                            spec: None,
                            priority: None,
                            children: None,
                            depends_on: None,
                            id: None,
                            status: None,
                            active_form: None,
                            parent_id: None,
                            ..Default::default()
                        }]),
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    },
                ]),
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            },
            TaskTree {
                name: Some("Task 2".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: Some(vec!["Task 1".to_string()]),
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            },
        ];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 5);

        // Verify structure
        assert_eq!(flat[0].name, Some("Task 1".to_string()));
        assert_eq!(flat[0].parent_name, None);

        assert_eq!(flat[1].name, Some("Task 1.1".to_string()));
        assert_eq!(flat[1].parent_name, Some("Task 1".to_string()));

        assert_eq!(flat[2].name, Some("Task 1.2".to_string()));
        assert_eq!(flat[2].parent_name, Some("Task 1".to_string()));

        assert_eq!(flat[3].name, Some("Task 1.2.1".to_string()));
        assert_eq!(flat[3].parent_name, Some("Task 1.2".to_string()));

        assert_eq!(flat[4].name, Some("Task 2".to_string()));
        assert_eq!(flat[4].parent_name, None);
        assert_eq!(flat[4].depends_on, vec!["Task 1"]);
    }

    #[tokio::test]
    async fn test_plan_executor_integration() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;

        // Create a plan with hierarchy and dependencies
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Integration Test Plan".to_string()),
                spec: Some("Test plan execution end-to-end".to_string()),
                priority: Some(PriorityValue::High),
                children: Some(vec![
                    TaskTree {
                        name: Some("Subtask A".to_string()),
                        spec: Some("First subtask".to_string()),
                        priority: None,
                        children: None,
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    },
                    TaskTree {
                        name: Some("Subtask B".to_string()),
                        spec: Some("Second subtask depends on A".to_string()),
                        priority: None,
                        children: None,
                        depends_on: Some(vec!["Subtask A".to_string()]),
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    },
                ]),
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        // Execute the plan
        let executor = PlanExecutor::new(&ctx.pool);
        let result = executor.execute(&request).await.unwrap();

        // Verify success
        assert!(result.success, "Plan execution should succeed");
        assert_eq!(result.created_count, 3, "Should create 3 tasks");
        assert_eq!(result.updated_count, 0, "Should not update any tasks");
        assert_eq!(result.dependency_count, 1, "Should create 1 dependency");
        assert!(result.error.is_none(), "Should have no error");

        // Verify task ID map
        assert_eq!(result.task_id_map.len(), 3);
        assert!(result.task_id_map.contains_key("Integration Test Plan"));
        assert!(result.task_id_map.contains_key("Subtask A"));
        assert!(result.task_id_map.contains_key("Subtask B"));

        // Verify tasks were created in database
        let parent_id = *result.task_id_map.get("Integration Test Plan").unwrap();
        let subtask_a_id = *result.task_id_map.get("Subtask A").unwrap();
        let subtask_b_id = *result.task_id_map.get("Subtask B").unwrap();

        // Check parent task
        let parent: (String, String, i64, Option<i64>) =
            sqlx::query_as("SELECT name, spec, priority, parent_id FROM tasks WHERE id = ?")
                .bind(parent_id)
                .fetch_one(&ctx.pool)
                .await
                .unwrap();

        assert_eq!(parent.0, "Integration Test Plan");
        assert_eq!(parent.1, "Test plan execution end-to-end");
        assert_eq!(parent.2, 2); // High priority = 2
        assert_eq!(parent.3, None); // No parent

        // Check subtask A
        let subtask_a: (String, Option<i64>) =
            sqlx::query_as(crate::sql_constants::SELECT_TASK_NAME_PARENT)
                .bind(subtask_a_id)
                .fetch_one(&ctx.pool)
                .await
                .unwrap();

        assert_eq!(subtask_a.0, "Subtask A");
        assert_eq!(subtask_a.1, Some(parent_id)); // Parent should be set

        // Check dependency
        let dep: (i64, i64) = sqlx::query_as(
            "SELECT blocking_task_id, blocked_task_id FROM dependencies WHERE blocked_task_id = ?",
        )
        .bind(subtask_b_id)
        .fetch_one(&ctx.pool)
        .await
        .unwrap();

        assert_eq!(dep.0, subtask_a_id); // Blocking task
        assert_eq!(dep.1, subtask_b_id); // Blocked task
    }

    #[tokio::test]
    async fn test_plan_executor_idempotency() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;

        // Create a plan
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Idempotent Task".to_string()),
                spec: Some("Initial spec".to_string()),
                priority: Some(PriorityValue::High),
                children: Some(vec![
                    TaskTree {
                        name: Some("Child 1".to_string()),
                        spec: Some("Child spec 1".to_string()),
                        priority: None,
                        children: None,
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    },
                    TaskTree {
                        name: Some("Child 2".to_string()),
                        spec: Some("Child spec 2".to_string()),
                        priority: Some(PriorityValue::Low),
                        children: None,
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    },
                ]),
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let executor = PlanExecutor::new(&ctx.pool);

        // First execution - should create all tasks
        let result1 = executor.execute(&request).await.unwrap();
        assert!(result1.success, "First execution should succeed");
        assert_eq!(result1.created_count, 3, "Should create 3 tasks");
        assert_eq!(result1.updated_count, 0, "Should not update any tasks");
        assert_eq!(result1.task_id_map.len(), 3, "Should have 3 task IDs");

        // Get task IDs from first execution
        let parent_id_1 = *result1.task_id_map.get("Idempotent Task").unwrap();
        let child1_id_1 = *result1.task_id_map.get("Child 1").unwrap();
        let child2_id_1 = *result1.task_id_map.get("Child 2").unwrap();

        // Second execution with same plan - should update all tasks (idempotent)
        let result2 = executor.execute(&request).await.unwrap();
        assert!(result2.success, "Second execution should succeed");
        assert_eq!(result2.created_count, 0, "Should not create any new tasks");
        assert_eq!(result2.updated_count, 3, "Should update all 3 tasks");
        assert_eq!(result2.task_id_map.len(), 3, "Should still have 3 task IDs");

        // Task IDs should remain the same (idempotent)
        let parent_id_2 = *result2.task_id_map.get("Idempotent Task").unwrap();
        let child1_id_2 = *result2.task_id_map.get("Child 1").unwrap();
        let child2_id_2 = *result2.task_id_map.get("Child 2").unwrap();

        assert_eq!(parent_id_1, parent_id_2, "Parent ID should not change");
        assert_eq!(child1_id_1, child1_id_2, "Child 1 ID should not change");
        assert_eq!(child2_id_1, child2_id_2, "Child 2 ID should not change");

        // Verify data in database hasn't changed (spec, priority)
        let parent: (String, i64) = sqlx::query_as("SELECT spec, priority FROM tasks WHERE id = ?")
            .bind(parent_id_2)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();

        assert_eq!(parent.0, "Initial spec");
        assert_eq!(parent.1, 2); // High priority = 2

        // Third execution with modified plan - should update with new values
        let modified_request = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Idempotent Task".to_string()),
                spec: Some("Updated spec".to_string()), // Changed
                priority: Some(PriorityValue::Critical), // Changed
                children: Some(vec![
                    TaskTree {
                        name: Some("Child 1".to_string()),
                        spec: Some("Updated child spec 1".to_string()), // Changed
                        priority: None,
                        children: None,
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    },
                    TaskTree {
                        name: Some("Child 2".to_string()),
                        spec: Some("Child spec 2".to_string()), // Unchanged
                        priority: Some(PriorityValue::Low),
                        children: None,
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    },
                ]),
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result3 = executor.execute(&modified_request).await.unwrap();
        assert!(result3.success, "Third execution should succeed");
        assert_eq!(result3.created_count, 0, "Should not create any new tasks");
        assert_eq!(result3.updated_count, 3, "Should update all 3 tasks");

        // Verify updates were applied
        let updated_parent: (String, i64) =
            sqlx::query_as("SELECT spec, priority FROM tasks WHERE id = ?")
                .bind(parent_id_2)
                .fetch_one(&ctx.pool)
                .await
                .unwrap();

        assert_eq!(updated_parent.0, "Updated spec");
        assert_eq!(updated_parent.1, 1); // Critical priority = 1

        let updated_child1: (String,) = sqlx::query_as("SELECT spec FROM tasks WHERE id = ?")
            .bind(child1_id_2)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();

        assert_eq!(updated_child1.0, "Updated child spec 1");
    }

    #[tokio::test]
    async fn test_plan_executor_dependencies() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;

        // Create a plan with multiple dependency relationships
        let request = PlanRequest {
            tasks: vec![
                TaskTree {
                    name: Some("Foundation".to_string()),
                    spec: Some("Base layer".to_string()),
                    priority: Some(PriorityValue::Critical),
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Layer 1".to_string()),
                    spec: Some("Depends on Foundation".to_string()),
                    priority: Some(PriorityValue::High),
                    children: None,
                    depends_on: Some(vec!["Foundation".to_string()]),
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Layer 2".to_string()),
                    spec: Some("Depends on Layer 1".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Layer 1".to_string()]),
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Integration".to_string()),
                    spec: Some("Depends on both Foundation and Layer 2".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Foundation".to_string(), "Layer 2".to_string()]),
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
            ],
        };

        let executor = PlanExecutor::new(&ctx.pool);
        let result = executor.execute(&request).await.unwrap();

        assert!(result.success, "Plan execution should succeed");
        assert_eq!(result.created_count, 4, "Should create 4 tasks");
        assert_eq!(result.dependency_count, 4, "Should create 4 dependencies");

        // Get task IDs
        let foundation_id = *result.task_id_map.get("Foundation").unwrap();
        let layer1_id = *result.task_id_map.get("Layer 1").unwrap();
        let layer2_id = *result.task_id_map.get("Layer 2").unwrap();
        let integration_id = *result.task_id_map.get("Integration").unwrap();

        // Verify dependency: Layer 1 -> Foundation
        let deps1: Vec<(i64,)> =
            sqlx::query_as("SELECT blocking_task_id FROM dependencies WHERE blocked_task_id = ?")
                .bind(layer1_id)
                .fetch_all(&ctx.pool)
                .await
                .unwrap();

        assert_eq!(deps1.len(), 1);
        assert_eq!(deps1[0].0, foundation_id);

        // Verify dependency: Layer 2 -> Layer 1
        let deps2: Vec<(i64,)> =
            sqlx::query_as("SELECT blocking_task_id FROM dependencies WHERE blocked_task_id = ?")
                .bind(layer2_id)
                .fetch_all(&ctx.pool)
                .await
                .unwrap();

        assert_eq!(deps2.len(), 1);
        assert_eq!(deps2[0].0, layer1_id);

        // Verify dependencies: Integration -> Foundation, Layer 2
        let deps3: Vec<(i64,)> =
            sqlx::query_as("SELECT blocking_task_id FROM dependencies WHERE blocked_task_id = ? ORDER BY blocking_task_id")
                .bind(integration_id)
                .fetch_all(&ctx.pool)
                .await
                .unwrap();

        assert_eq!(deps3.len(), 2);
        let mut blocking_ids: Vec<i64> = deps3.iter().map(|d| d.0).collect();
        blocking_ids.sort();

        let mut expected_ids = vec![foundation_id, layer2_id];
        expected_ids.sort();

        assert_eq!(blocking_ids, expected_ids);
    }

    #[tokio::test]
    async fn test_plan_executor_invalid_dependency() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;

        // Create a plan with an invalid dependency
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Task A".to_string()),
                spec: Some("Depends on non-existent task".to_string()),
                priority: None,
                children: None,
                depends_on: Some(vec!["NonExistent".to_string()]),
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let executor = PlanExecutor::new(&ctx.pool);
        let result = executor.execute(&request).await.unwrap();

        assert!(!result.success, "Plan execution should fail");
        assert!(result.error.is_some(), "Should have error message");
        let error = result.error.unwrap();
        assert!(
            error.contains("NonExistent"),
            "Error should mention the missing dependency: {}",
            error
        );
    }

    #[tokio::test]
    async fn test_plan_executor_simple_cycle() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;

        // Create a plan with a simple cycle: A → B → A
        let request = PlanRequest {
            tasks: vec![
                TaskTree {
                    name: Some("Task A".to_string()),
                    spec: Some("Depends on B".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task B".to_string()]),
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Task B".to_string()),
                    spec: Some("Depends on A".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task A".to_string()]),
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
            ],
        };

        let executor = PlanExecutor::new(&ctx.pool);
        let result = executor.execute(&request).await.unwrap();

        assert!(!result.success, "Plan execution should fail");
        assert!(result.error.is_some(), "Should have error message");
        let error = result.error.unwrap();
        assert!(
            error.contains("Circular dependency"),
            "Error should mention circular dependency: {}",
            error
        );
        assert!(
            error.contains("Task A") && error.contains("Task B"),
            "Error should mention both tasks in the cycle: {}",
            error
        );
    }

    #[tokio::test]
    async fn test_plan_executor_complex_cycle() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;

        // Create a plan with a complex cycle: A → B → C → A
        let request = PlanRequest {
            tasks: vec![
                TaskTree {
                    name: Some("Task A".to_string()),
                    spec: Some("Depends on B".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task B".to_string()]),
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Task B".to_string()),
                    spec: Some("Depends on C".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task C".to_string()]),
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Task C".to_string()),
                    spec: Some("Depends on A".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task A".to_string()]),
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
            ],
        };

        let executor = PlanExecutor::new(&ctx.pool);
        let result = executor.execute(&request).await.unwrap();

        assert!(!result.success, "Plan execution should fail");
        assert!(result.error.is_some(), "Should have error message");
        let error = result.error.unwrap();
        assert!(
            error.contains("Circular dependency"),
            "Error should mention circular dependency: {}",
            error
        );
        assert!(
            error.contains("Task A") && error.contains("Task B") && error.contains("Task C"),
            "Error should mention all tasks in the cycle: {}",
            error
        );
    }

    #[tokio::test]
    async fn test_plan_executor_valid_dag() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;

        // Create a valid DAG: no cycles
        //   A
        //  / \
        // B   C
        //  \ /
        //   D
        let request = PlanRequest {
            tasks: vec![
                TaskTree {
                    name: Some("Task A".to_string()),
                    spec: Some("Root task".to_string()),
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Task B".to_string()),
                    spec: Some("Depends on A".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task A".to_string()]),
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Task C".to_string()),
                    spec: Some("Depends on A".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task A".to_string()]),
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Task D".to_string()),
                    spec: Some("Depends on B and C".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task B".to_string(), "Task C".to_string()]),
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
            ],
        };

        let executor = PlanExecutor::new(&ctx.pool);
        let result = executor.execute(&request).await.unwrap();

        assert!(
            result.success,
            "Plan execution should succeed for valid DAG"
        );
        assert_eq!(result.created_count, 4, "Should create 4 tasks");
        assert_eq!(result.dependency_count, 4, "Should create 4 dependencies");
    }

    #[tokio::test]
    async fn test_plan_executor_self_dependency() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;

        // Create a plan with self-dependency: A → A
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Task A".to_string()),
                spec: Some("Depends on itself".to_string()),
                priority: None,
                children: None,
                depends_on: Some(vec!["Task A".to_string()]),
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let executor = PlanExecutor::new(&ctx.pool);
        let result = executor.execute(&request).await.unwrap();

        assert!(
            !result.success,
            "Plan execution should fail for self-dependency"
        );
        assert!(result.error.is_some(), "Should have error message");
        let error = result.error.unwrap();
        assert!(
            error.contains("Circular dependency"),
            "Error should mention circular dependency: {}",
            error
        );
    }

    // Database query tests
    #[tokio::test]
    async fn test_find_tasks_by_names_empty() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        let result = executor.find_tasks_by_names(&[]).await.unwrap();
        assert!(result.is_empty(), "Empty input should return empty map");
    }

    #[tokio::test]
    async fn test_find_tasks_by_names_partial() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Create some tasks first
        let request = PlanRequest {
            tasks: vec![
                TaskTree {
                    name: Some("Task A".to_string()),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Task B".to_string()),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
            ],
        };
        executor.execute(&request).await.unwrap();

        // Query for A, B, and C (C doesn't exist)
        let names = vec![
            "Task A".to_string(),
            "Task B".to_string(),
            "Task C".to_string(),
        ];
        let result = executor.find_tasks_by_names(&names).await.unwrap();

        assert_eq!(result.len(), 2, "Should find 2 out of 3 tasks");
        assert!(result.contains_key("Task A"));
        assert!(result.contains_key("Task B"));
        assert!(!result.contains_key("Task C"));
    }

    // Performance tests
    #[tokio::test]
    async fn test_plan_1000_tasks_performance() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Generate 1000 flat tasks
        let mut tasks = Vec::new();
        for i in 0..1000 {
            tasks.push(TaskTree {
                name: Some(format!("Task {}", i)),
                spec: Some(format!("Spec for task {}", i)),
                priority: Some(PriorityValue::Medium),
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            });
        }

        let request = PlanRequest { tasks };

        let start = std::time::Instant::now();
        let result = executor.execute(&request).await.unwrap();
        let duration = start.elapsed();

        assert!(result.success);
        assert_eq!(result.created_count, 1000);
        assert!(
            duration.as_secs() < 10,
            "Should complete 1000 tasks in under 10 seconds, took {:?}",
            duration
        );

        println!("✅ Created 1000 tasks in {:?}", duration);
    }

    #[tokio::test]
    async fn test_plan_deep_nesting_20_levels() {
        use crate::test_utils::test_helpers::TestContext;

        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Generate deep nesting: 20 levels
        fn build_deep_tree(depth: usize, current: usize) -> TaskTree {
            TaskTree {
                name: Some(format!("Level {}", current)),
                spec: Some(format!("Task at depth {}", current)),
                priority: Some(PriorityValue::Low),
                children: if current < depth {
                    Some(vec![build_deep_tree(depth, current + 1)])
                } else {
                    None
                },
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }
        }

        let request = PlanRequest {
            tasks: vec![build_deep_tree(20, 1)],
        };

        let start = std::time::Instant::now();
        let result = executor.execute(&request).await.unwrap();
        let duration = start.elapsed();

        assert!(result.success);
        assert_eq!(
            result.created_count, 20,
            "Should create 20 tasks (1 per level)"
        );
        assert!(
            duration.as_secs() < 5,
            "Should handle 20-level nesting in under 5 seconds, took {:?}",
            duration
        );

        println!("✅ Created 20-level deep tree in {:?}", duration);
    }

    #[test]
    fn test_flatten_preserves_all_fields() {
        let tasks = vec![TaskTree {
            name: Some("Full Task".to_string()),
            spec: Some("Detailed spec".to_string()),
            priority: Some(PriorityValue::Critical),
            children: None,
            depends_on: Some(vec!["Dep1".to_string(), "Dep2".to_string()]),
            id: Some(42),
            status: None,
            active_form: None,
            parent_id: None,
            ..Default::default()
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 1);

        let task = &flat[0];
        assert_eq!(task.name, Some("Full Task".to_string()));
        assert_eq!(task.spec, Some("Detailed spec".to_string()));
        assert_eq!(task.priority, Some(PriorityValue::Critical));
        assert_eq!(task.depends_on, vec!["Dep1", "Dep2"]);
        assert_eq!(task.id, Some(42));
    }
}

#[cfg(test)]
mod dataflow_tests {
    use super::*;
    use crate::tasks::TaskManager;
    use crate::test_utils::test_helpers::TestContext;

    #[tokio::test]
    async fn test_complete_dataflow_status_and_active_form() {
        // 创建测试环境
        let ctx = TestContext::new().await;

        // 第1步：使用Plan工具创建带status和active_form的任务
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Test Active Form Task".to_string()),
                spec: Some("Testing complete dataflow".to_string()),
                priority: Some(PriorityValue::High),
                children: None,
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Doing),
                active_form: Some("Testing complete dataflow now".to_string()),
                parent_id: None,
                ..Default::default()
            }],
        };

        let executor = PlanExecutor::new(&ctx.pool);
        let result = executor.execute(&request).await.unwrap();

        assert!(result.success);
        assert_eq!(result.created_count, 1);

        // 第2步：使用TaskManager读取任务（模拟MCP task_list工具）
        let task_mgr = TaskManager::new(&ctx.pool);
        let result = task_mgr
            .find_tasks(None, None, None, None, None)
            .await
            .unwrap();

        assert_eq!(result.tasks.len(), 1);
        let task = &result.tasks[0];

        // 第3步：验证所有字段都正确传递
        assert_eq!(task.name, "Test Active Form Task");
        assert_eq!(task.status, "doing"); // InProgress maps to "doing"
        assert_eq!(
            task.active_form,
            Some("Testing complete dataflow now".to_string())
        );

        // 第4步：验证序列化为JSON（模拟MCP输出）
        let json = serde_json::to_value(task).unwrap();
        assert_eq!(json["name"], "Test Active Form Task");
        assert_eq!(json["status"], "doing");
        assert_eq!(json["active_form"], "Testing complete dataflow now");

        println!("✅ 完整数据流验证成功！");
        println!("   Plan工具写入 -> Task读取 -> JSON序列化 -> MCP输出");
        println!("   active_form: {:?}", task.active_form);
    }
}

#[cfg(test)]
mod parent_id_tests {
    use super::*;
    use crate::test_utils::test_helpers::TestContext;

    #[test]
    fn test_parent_id_json_deserialization_absent() {
        // Field absent → None (use default behavior)
        let json = r#"{"name": "Test Task"}"#;
        let task: TaskTree = serde_json::from_str(json).unwrap();
        assert_eq!(task.parent_id, None);
    }

    #[test]
    fn test_parent_id_json_deserialization_null() {
        // Field is null → Some(None) (explicit root task)
        let json = r#"{"name": "Test Task", "parent_id": null}"#;
        let task: TaskTree = serde_json::from_str(json).unwrap();
        assert_eq!(task.parent_id, Some(None));
    }

    #[test]
    fn test_parent_id_json_deserialization_number() {
        // Field is number → Some(Some(id)) (explicit parent)
        let json = r#"{"name": "Test Task", "parent_id": 42}"#;
        let task: TaskTree = serde_json::from_str(json).unwrap();
        assert_eq!(task.parent_id, Some(Some(42)));
    }

    #[test]
    fn test_flatten_propagates_parent_id() {
        let tasks = vec![TaskTree {
            name: Some("Task with explicit parent".to_string()),
            spec: None,
            priority: None,
            children: None,
            depends_on: None,
            id: None,
            status: None,
            active_form: None,
            parent_id: Some(Some(99)),
            ..Default::default()
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].explicit_parent_id, Some(Some(99)));
    }

    #[test]
    fn test_flatten_propagates_null_parent_id() {
        let tasks = vec![TaskTree {
            name: Some("Explicit root task".to_string()),
            spec: None,
            priority: None,
            children: None,
            depends_on: None,
            id: None,
            status: None,
            active_form: None,
            parent_id: Some(None), // Explicit null
            ..Default::default()
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].explicit_parent_id, Some(None));
    }

    #[tokio::test]
    async fn test_explicit_parent_id_sets_parent() {
        let ctx = TestContext::new().await;

        // First create a parent task
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Parent Task".to_string()),
                spec: Some("This is the parent".to_string()),
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let executor = PlanExecutor::new(&ctx.pool);
        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        let parent_id = *result1.task_id_map.get("Parent Task").unwrap();

        // Now create a child task using explicit parent_id
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Child Task".to_string()),
                spec: Some("This uses explicit parent_id".to_string()),
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: Some(Some(parent_id)),
                ..Default::default()
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();
        assert!(result2.success);
        let child_id = *result2.task_id_map.get("Child Task").unwrap();

        // Verify parent-child relationship
        let row: (Option<i64>,) = sqlx::query_as("SELECT parent_id FROM tasks WHERE id = ?")
            .bind(child_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(row.0, Some(parent_id));
    }

    #[tokio::test]
    async fn test_explicit_null_parent_id_creates_root() {
        let ctx = TestContext::new().await;

        // Create a task with explicit null parent_id (should be root)
        // Even when default_parent_id is set
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Explicit Root Task".to_string()),
                spec: Some("Should be root despite default parent".to_string()),
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: Some(None), // Explicit null = root
                ..Default::default()
            }],
        };

        // Create executor with a default parent
        // First create a "default parent" task
        let parent_request = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Default Parent".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };
        let executor = PlanExecutor::new(&ctx.pool);
        let parent_result = executor.execute(&parent_request).await.unwrap();
        let default_parent_id = *parent_result.task_id_map.get("Default Parent").unwrap();

        // Now execute with default parent set, but our task has explicit null parent_id
        let executor_with_default =
            PlanExecutor::new(&ctx.pool).with_default_parent(default_parent_id);
        let result = executor_with_default.execute(&request).await.unwrap();
        assert!(result.success);
        let task_id = *result.task_id_map.get("Explicit Root Task").unwrap();

        // Verify it's a root task (parent_id is NULL)
        let row: (Option<i64>,) = sqlx::query_as("SELECT parent_id FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(
            row.0, None,
            "Task with explicit null parent_id should be root"
        );
    }

    #[tokio::test]
    async fn test_children_nesting_takes_precedence_over_parent_id() {
        let ctx = TestContext::new().await;

        // Create a task hierarchy where children nesting should override parent_id
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Parent via Nesting".to_string()),
                spec: Some("Test parent spec".to_string()),
                priority: None,
                children: Some(vec![TaskTree {
                    name: Some("Child via Nesting".to_string()),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: Some(Some(999)), // This should be ignored!
                    ..Default::default()
                }]),
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let executor = PlanExecutor::new(&ctx.pool);
        let result = executor.execute(&request).await.unwrap();
        assert!(result.success);

        let parent_id = *result.task_id_map.get("Parent via Nesting").unwrap();
        let child_id = *result.task_id_map.get("Child via Nesting").unwrap();

        // Verify child's parent is "Parent via Nesting", not 999
        let row: (Option<i64>,) = sqlx::query_as("SELECT parent_id FROM tasks WHERE id = ?")
            .bind(child_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(
            row.0,
            Some(parent_id),
            "Children nesting should take precedence"
        );
    }

    #[tokio::test]
    async fn test_modify_existing_task_parent() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Create two independent tasks
        let request1 = PlanRequest {
            tasks: vec![
                TaskTree {
                    name: Some("Task A".to_string()),
                    spec: Some("Task A spec".to_string()),
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: Some(TaskStatus::Doing),
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Task B".to_string()),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
            ],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        let task_a_id = *result1.task_id_map.get("Task A").unwrap();
        let task_b_id = *result1.task_id_map.get("Task B").unwrap();

        // Verify both are root tasks initially
        let row: (Option<i64>,) = sqlx::query_as("SELECT parent_id FROM tasks WHERE id = ?")
            .bind(task_b_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(row.0, None, "Task B should initially be root");

        // Now update Task B to be a child of Task A using parent_id
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Task B".to_string()), // Same name = update
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: Some(Some(task_a_id)), // Set parent to Task A
                ..Default::default()
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();
        assert!(result2.success);
        assert_eq!(result2.updated_count, 1, "Should update existing task");

        // Verify Task B is now a child of Task A
        let row: (Option<i64>,) = sqlx::query_as("SELECT parent_id FROM tasks WHERE id = ?")
            .bind(task_b_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(
            row.0,
            Some(task_a_id),
            "Task B should now be child of Task A"
        );
    }

    #[tokio::test]
    async fn test_plan_done_with_incomplete_children_fails() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Create parent with incomplete child
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Parent Task".to_string()),
                spec: Some("Parent spec".to_string()),
                priority: None,
                children: Some(vec![TaskTree {
                    name: Some("Child Task".to_string()),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: Some(TaskStatus::Todo), // Child is not done
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                }]),
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);

        // Try to complete parent while child is incomplete
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Parent Task".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Done), // Try to set done
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();
        assert!(!result2.success, "Should fail when child is incomplete");
        assert!(
            result2
                .error
                .as_ref()
                .unwrap()
                .contains("Uncompleted children"),
            "Error should mention uncompleted children: {:?}",
            result2.error
        );
    }

    #[tokio::test]
    async fn test_plan_done_with_completed_children_succeeds() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Create parent with child
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Parent Task".to_string()),
                spec: Some("Parent spec".to_string()),
                priority: None,
                children: Some(vec![TaskTree {
                    name: Some("Child Task".to_string()),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: Some(TaskStatus::Todo),
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                }]),
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);

        // Complete child first
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Child Task".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Done),
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();
        assert!(result2.success);

        // Now parent can be completed
        let request3 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Parent Task".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Done),
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result3 = executor.execute(&request3).await.unwrap();
        assert!(result3.success, "Should succeed when child is complete");
    }
}

#[cfg(test)]
mod delete_tests {
    use super::*;
    use crate::test_utils::test_helpers::TestContext;
    use serial_test::serial;

    #[tokio::test]
    async fn test_delete_task_by_id_only() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // First create a task
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Task to delete".to_string()),
                spec: Some("This will be deleted".to_string()),
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        let task_id = *result1.task_id_map.get("Task to delete").unwrap();

        // Verify task exists
        let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(exists.0, 1, "Task should exist");

        // Delete by id only (no name)
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: None, // No name needed!
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: Some(task_id),
                status: None,
                active_form: None,
                parent_id: None,
                delete: Some(true),
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();
        assert!(result2.success, "Delete should succeed");
        assert_eq!(result2.deleted_count, 1, "Should delete 1 task");
        assert_eq!(result2.created_count, 0);
        assert_eq!(result2.updated_count, 0);

        // Verify task no longer exists
        let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(exists.0, 0, "Task should be deleted");
    }

    #[tokio::test]
    async fn test_delete_requires_id() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Try to delete without id
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Task name without id".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None, // No id!
                status: None,
                active_form: None,
                parent_id: None,
                delete: Some(true),
            }],
        };

        let result = executor.execute(&request).await.unwrap();
        assert!(!result.success, "Delete without id should fail");
        assert!(
            result.error.as_ref().unwrap().contains("id"),
            "Error should mention 'id' requirement"
        );
    }

    #[tokio::test]
    async fn test_delete_with_json_syntax() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // First create a task
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("JSON delete test".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        let task_id = *result1.task_id_map.get("JSON delete test").unwrap();

        // Test JSON deserialization with just id and delete
        let json = format!(r#"{{"tasks": [{{"id": {}, "delete": true}}]}}"#, task_id);
        let request2: PlanRequest = serde_json::from_str(&json).unwrap();

        let result2 = executor.execute(&request2).await.unwrap();
        assert!(result2.success, "Delete via JSON should succeed");
        assert_eq!(result2.deleted_count, 1);

        // Verify task no longer exists
        let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(exists.0, 0, "Task should be deleted");
    }

    #[tokio::test]
    async fn test_mixed_create_update_delete_in_one_request() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Create initial tasks
        let request1 = PlanRequest {
            tasks: vec![
                TaskTree {
                    name: Some("To Update".to_string()),
                    spec: Some("Original spec".to_string()),
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                TaskTree {
                    name: Some("To Delete".to_string()),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
            ],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        let delete_id = *result1.task_id_map.get("To Delete").unwrap();

        // Mixed request: create + update + delete
        let request2 = PlanRequest {
            tasks: vec![
                // Create new
                TaskTree {
                    name: Some("Newly Created".to_string()),
                    spec: Some("Brand new".to_string()),
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                // Update existing
                TaskTree {
                    name: Some("To Update".to_string()),
                    spec: Some("Updated spec".to_string()),
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                },
                // Delete existing
                TaskTree {
                    name: None,
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: Some(delete_id),
                    status: None,
                    active_form: None,
                    parent_id: None,
                    delete: Some(true),
                },
            ],
        };

        let result2 = executor.execute(&request2).await.unwrap();
        assert!(result2.success);
        assert_eq!(result2.created_count, 1, "Should create 1 task");
        assert_eq!(result2.updated_count, 1, "Should update 1 task");
        assert_eq!(result2.deleted_count, 1, "Should delete 1 task");

        // Verify "To Delete" no longer exists
        let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE id = ?")
            .bind(delete_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(exists.0, 0, "Deleted task should not exist");
    }

    #[tokio::test]
    async fn test_delete_nonexistent_id_returns_warning() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Delete a non-existent ID
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: None,
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: Some(99999), // Non-existent ID
                status: None,
                active_form: None,
                parent_id: None,
                delete: Some(true),
            }],
        };

        let result = executor.execute(&request).await.unwrap();

        // Should succeed but with warning
        assert!(
            result.success,
            "Delete of non-existent ID should still succeed"
        );
        assert_eq!(
            result.deleted_count, 0,
            "Should not count non-existent task as deleted"
        );
        assert!(
            !result.warnings.is_empty(),
            "Should have warning about non-existent task"
        );
        assert!(
            result.warnings[0].contains("not found"),
            "Warning should mention task not found: {:?}",
            result.warnings
        );
    }

    #[tokio::test]
    async fn test_cascade_delete_reports_descendants() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Create a parent with 2 children
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Parent Task".to_string()),
                spec: None,
                priority: None,
                children: Some(vec![
                    TaskTree {
                        name: Some("Child 1".to_string()),
                        spec: None,
                        priority: None,
                        children: None,
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    },
                    TaskTree {
                        name: Some("Child 2".to_string()),
                        spec: None,
                        priority: None,
                        children: None,
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    },
                ]),
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        assert_eq!(
            result1.created_count, 3,
            "Should create parent + 2 children"
        );
        let parent_id = *result1.task_id_map.get("Parent Task").unwrap();

        // Delete parent - should cascade to children
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: None,
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: Some(parent_id),
                status: None,
                active_form: None,
                parent_id: None,
                delete: Some(true),
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();

        assert!(result2.success, "Cascade delete should succeed");
        assert_eq!(
            result2.deleted_count, 1,
            "deleted_count should only count direct deletes"
        );
        assert_eq!(
            result2.cascade_deleted_count, 2,
            "Should report 2 cascade-deleted children"
        );
        assert!(
            result2.warnings.iter().any(|w| w.contains("descendant")),
            "Should have warning about cascade-deleted descendants: {:?}",
            result2.warnings
        );

        // Verify all tasks are gone
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0, "All tasks should be deleted");
    }

    #[tokio::test]
    async fn test_cascade_delete_deep_hierarchy() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Create a deep hierarchy: Root -> L1 -> L2 -> L3
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Root".to_string()),
                spec: None,
                priority: None,
                children: Some(vec![TaskTree {
                    name: Some("Level1".to_string()),
                    spec: None,
                    priority: None,
                    children: Some(vec![TaskTree {
                        name: Some("Level2".to_string()),
                        spec: None,
                        priority: None,
                        children: Some(vec![TaskTree {
                            name: Some("Level3".to_string()),
                            spec: None,
                            priority: None,
                            children: None,
                            depends_on: None,
                            id: None,
                            status: None,
                            active_form: None,
                            parent_id: None,
                            ..Default::default()
                        }]),
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    }]),
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                }]),
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        assert_eq!(result1.created_count, 4);
        let root_id = *result1.task_id_map.get("Root").unwrap();

        // Delete root - should cascade to all descendants
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: None,
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: Some(root_id),
                status: None,
                active_form: None,
                parent_id: None,
                delete: Some(true),
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();

        assert!(result2.success);
        assert_eq!(
            result2.deleted_count, 1,
            "Only root counted as direct delete"
        );
        assert_eq!(
            result2.cascade_deleted_count, 3,
            "Should cascade-delete 3 descendants"
        );
    }

    #[tokio::test]
    async fn test_delete_multiple_ids_with_mixed_results() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Create one task
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Existing Task".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        let existing_id = *result1.task_id_map.get("Existing Task").unwrap();

        // Try to delete: one existing, one non-existent
        let request2 = PlanRequest {
            tasks: vec![
                TaskTree {
                    name: None,
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: Some(existing_id),
                    status: None,
                    active_form: None,
                    parent_id: None,
                    delete: Some(true),
                },
                TaskTree {
                    name: None,
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: Some(88888), // Non-existent
                    status: None,
                    active_form: None,
                    parent_id: None,
                    delete: Some(true),
                },
            ],
        };

        let result2 = executor.execute(&request2).await.unwrap();

        assert!(result2.success, "Mixed delete should still succeed");
        assert_eq!(result2.deleted_count, 1, "Only one task actually deleted");
        assert!(
            result2
                .warnings
                .iter()
                .any(|w| w.contains("88888") && w.contains("not found")),
            "Should warn about non-existent ID 88888: {:?}",
            result2.warnings
        );
    }

    /// P0: Verify that deleting a focused task returns an error (not allowed)
    #[tokio::test]
    #[serial]
    async fn test_delete_focused_task_returns_error() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Set a unique session ID for this test
        let test_session_id = format!("test-delete-focus-{}", std::process::id());
        std::env::set_var("IE_SESSION_ID", &test_session_id);

        // Create a task with status: doing (this auto-focuses the task)
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Focused Task".to_string()),
                spec: Some("## Goal\nTest focus deletion".to_string()),
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success, "Create focused task should succeed");
        let task_id = *result1.task_id_map.get("Focused Task").unwrap();

        // Verify the task is actually the session's focus
        let focus_check: Option<(i64,)> =
            sqlx::query_as("SELECT current_task_id FROM sessions WHERE session_id = ?")
                .bind(&test_session_id)
                .fetch_optional(&ctx.pool)
                .await
                .unwrap();
        assert_eq!(
            focus_check.map(|r| r.0),
            Some(task_id),
            "Task should be the session's current focus"
        );

        // Try to delete the focused task - should FAIL
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: None,
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: Some(task_id),
                status: None,
                active_form: None,
                parent_id: None,
                delete: Some(true),
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();

        // Should fail with error about focus
        assert!(!result2.success, "Delete focused task should fail");
        let error = result2.error.as_ref().unwrap();
        assert!(
            error.contains("focus") && error.contains(&test_session_id),
            "Error should mention focus and session: {}",
            error
        );
        assert_eq!(result2.deleted_count, 0, "Nothing should be deleted");

        // Verify task still exists
        let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(exists.0, 1, "Focused task should NOT be deleted");

        // Clean up env var
        std::env::remove_var("IE_SESSION_ID");
    }

    /// P0: Verify that deleting the same ID twice in a batch behaves correctly
    #[tokio::test]
    async fn test_delete_duplicate_id_in_batch() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Create a task
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Duplicate Delete Target".to_string()),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        let task_id = *result1.task_id_map.get("Duplicate Delete Target").unwrap();

        // Delete the same ID twice in one batch
        let request2 = PlanRequest {
            tasks: vec![
                TaskTree {
                    name: None,
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: Some(task_id),
                    status: None,
                    active_form: None,
                    parent_id: None,
                    delete: Some(true),
                },
                TaskTree {
                    name: None,
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: Some(task_id), // Same ID again
                    status: None,
                    active_form: None,
                    parent_id: None,
                    delete: Some(true),
                },
            ],
        };

        let result2 = executor.execute(&request2).await.unwrap();

        // Should succeed but only count 1 deletion
        assert!(result2.success, "Duplicate delete should still succeed");
        assert_eq!(
            result2.deleted_count, 1,
            "Only the first delete should count"
        );

        // Second delete attempt should generate a "not found" warning
        let not_found_warnings: Vec<_> = result2
            .warnings
            .iter()
            .filter(|w| w.contains("not found"))
            .collect();
        assert_eq!(
            not_found_warnings.len(),
            1,
            "Should have exactly one 'not found' warning for the duplicate: {:?}",
            result2.warnings
        );

        // Verify task is actually deleted
        let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(exists.0, 0, "Task should be deleted");
    }

    /// P0: Verify that deleting a parent task is blocked when a child is focused
    /// This tests CASCADE delete protection
    #[tokio::test]
    #[serial]
    async fn test_delete_parent_blocked_when_child_is_focused() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Set a unique session ID for this test
        let test_session_id = format!("test-cascade-focus-{}", std::process::id());
        std::env::set_var("IE_SESSION_ID", &test_session_id);

        // Create a hierarchy: Parent -> Child (focused)
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Parent Task".to_string()),
                spec: None,
                priority: None,
                children: Some(vec![TaskTree {
                    name: Some("Child Task".to_string()),
                    spec: Some("## Goal\nChild is focused".to_string()),
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: Some(TaskStatus::Doing), // This makes child the focus
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                }]),
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success, "Create hierarchy should succeed");
        let parent_id = *result1.task_id_map.get("Parent Task").unwrap();
        let child_id = *result1.task_id_map.get("Child Task").unwrap();

        // Verify child is the focus
        let focus_check: Option<(i64,)> =
            sqlx::query_as("SELECT current_task_id FROM sessions WHERE session_id = ?")
                .bind(&test_session_id)
                .fetch_optional(&ctx.pool)
                .await
                .unwrap();
        assert_eq!(
            focus_check.map(|r| r.0),
            Some(child_id),
            "Child should be the session's focus"
        );

        // Try to delete parent - should FAIL because child is focused
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: None,
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: Some(parent_id),
                status: None,
                active_form: None,
                parent_id: None,
                delete: Some(true),
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();

        // Should fail with error about cascade
        assert!(
            !result2.success,
            "Delete parent should fail when child is focused"
        );
        let error = result2.error.as_ref().unwrap();
        assert!(
            error.contains("cascade"),
            "Error should mention cascade: {}",
            error
        );
        assert!(
            error.contains(&child_id.to_string()),
            "Error should mention child task ID {}: {}",
            child_id,
            error
        );

        // Verify both tasks still exist
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(count.0, 2, "Both tasks should still exist");

        // Clean up env var
        std::env::remove_var("IE_SESSION_ID");
    }

    /// P0: Verify that batch delete is blocked when ANY subtree contains focus
    /// This prevents order-based bypass tricks
    #[tokio::test]
    #[serial]
    async fn test_batch_delete_blocked_when_subtree_contains_focus() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Set a unique session ID for this test
        let test_session_id = format!("test-batch-focus-{}", std::process::id());
        std::env::set_var("IE_SESSION_ID", &test_session_id);

        // Create: Parent -> Child (focused)
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("BatchParent".to_string()),
                spec: None,
                priority: None,
                children: Some(vec![TaskTree {
                    name: Some("BatchChild".to_string()),
                    spec: Some("## Goal\nFocused child".to_string()),
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: None,
                    status: Some(TaskStatus::Doing),
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                }]),
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        let parent_id = *result1.task_id_map.get("BatchParent").unwrap();
        let child_id = *result1.task_id_map.get("BatchChild").unwrap();

        // Try batch delete: [parent, child] - should fail even though parent is first
        // Because focus check happens BEFORE any deletions
        let request2 = PlanRequest {
            tasks: vec![
                TaskTree {
                    name: None,
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: Some(parent_id),
                    status: None,
                    active_form: None,
                    parent_id: None,
                    delete: Some(true),
                },
                TaskTree {
                    name: None,
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    id: Some(child_id),
                    status: None,
                    active_form: None,
                    parent_id: None,
                    delete: Some(true),
                },
            ],
        };

        let result2 = executor.execute(&request2).await.unwrap();

        // Should fail - focus protection kicks in before any delete
        assert!(!result2.success, "Batch delete should fail");
        assert!(
            result2.error.as_ref().unwrap().contains("focus"),
            "Error should mention focus: {:?}",
            result2.error
        );
        assert_eq!(result2.deleted_count, 0, "Nothing should be deleted");

        // Verify both tasks still exist
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(count.0, 2, "Both tasks should still exist");

        // Clean up env var
        std::env::remove_var("IE_SESSION_ID");
    }

    /// P0: Verify focus protection works for deep hierarchies
    #[tokio::test]
    #[serial]
    async fn test_delete_blocked_when_deep_descendant_is_focused() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        let test_session_id = format!("test-deep-focus-{}", std::process::id());
        std::env::set_var("IE_SESSION_ID", &test_session_id);

        // Create: Root -> L1 -> L2 -> L3 (focused)
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Root".to_string()),
                spec: None,
                priority: None,
                children: Some(vec![TaskTree {
                    name: Some("Level1".to_string()),
                    spec: None,
                    priority: None,
                    children: Some(vec![TaskTree {
                        name: Some("Level2".to_string()),
                        spec: None,
                        priority: None,
                        children: Some(vec![TaskTree {
                            name: Some("Level3".to_string()),
                            spec: Some("## Goal\nDeep focused task".to_string()),
                            priority: None,
                            children: None,
                            depends_on: None,
                            id: None,
                            status: Some(TaskStatus::Doing),
                            active_form: None,
                            parent_id: None,
                            ..Default::default()
                        }]),
                        depends_on: None,
                        id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                        ..Default::default()
                    }]),
                    depends_on: None,
                    id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                    ..Default::default()
                }]),
                depends_on: None,
                id: None,
                status: None,
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        let root_id = *result1.task_id_map.get("Root").unwrap();
        let level3_id = *result1.task_id_map.get("Level3").unwrap();

        // Verify Level3 is focused
        let focus_check: Option<(i64,)> =
            sqlx::query_as("SELECT current_task_id FROM sessions WHERE session_id = ?")
                .bind(&test_session_id)
                .fetch_optional(&ctx.pool)
                .await
                .unwrap();
        assert_eq!(focus_check.map(|r| r.0), Some(level3_id));

        // Try to delete Root - should fail because Level3 (deep descendant) is focused
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: None,
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: Some(root_id),
                status: None,
                active_form: None,
                parent_id: None,
                delete: Some(true),
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();

        assert!(
            !result2.success,
            "Delete root should fail when deep descendant is focused"
        );
        let error = result2.error.as_ref().unwrap();
        assert!(
            error.contains("cascade"),
            "Error should mention cascade: {}",
            error
        );
        assert!(
            error.contains(&level3_id.to_string()),
            "Error should mention Level3 ID {}: {}",
            level3_id,
            error
        );

        // Verify all 4 tasks still exist
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(count.0, 4, "All tasks should still exist");

        std::env::remove_var("IE_SESSION_ID");
    }

    /// Verify that deleting a non-existent task with subtree check works correctly
    /// The subtree check should return None for non-existent tasks (not error)
    #[tokio::test]
    async fn test_delete_nonexistent_task_subtree_check_succeeds() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Try to delete a non-existent task ID
        // The subtree focus check should handle this gracefully
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: None,
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: Some(99999), // Non-existent
                status: None,
                active_form: None,
                parent_id: None,
                delete: Some(true),
            }],
        };

        let result = executor.execute(&request).await.unwrap();

        // Should succeed with warning (not error)
        assert!(result.success, "Delete of non-existent should succeed");
        assert_eq!(result.deleted_count, 0);
        assert!(
            result.warnings.iter().any(|w| w.contains("not found")),
            "Should have 'not found' warning: {:?}",
            result.warnings
        );
    }

    /// Verify that default session (-1) focus is also protected
    /// Even without explicit IE_SESSION_ID, tasks use default session "-1"
    #[tokio::test]
    #[serial]
    async fn test_default_session_focus_also_protected() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Remove IE_SESSION_ID - will use default session "-1"
        std::env::remove_var("IE_SESSION_ID");

        // Create a task with status: doing
        // This uses default session "-1" for focus
        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Default Session Task".to_string()),
                spec: Some("## Goal\nTest default session".to_string()),
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        let task_id = *result1.task_id_map.get("Default Session Task").unwrap();

        // Try to delete - should FAIL because it's focused by default session "-1"
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: None,
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: Some(task_id),
                status: None,
                active_form: None,
                parent_id: None,
                delete: Some(true),
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();

        // Should fail - default session's focus is protected too
        assert!(
            !result2.success,
            "Default session focus should be protected"
        );
        assert_eq!(result2.deleted_count, 0);

        // Error should mention default session "-1"
        let error = result2.error.as_ref().unwrap();
        assert!(
            error.contains("-1"),
            "Error should mention default session '-1': {}",
            error
        );

        // Verify task still exists
        let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(exists.0, 1, "Task should still exist");
    }

    /// Verify cross-session behavior: Session B CANNOT delete Session A's focus
    /// Focus protection is GLOBAL - protects tasks focused by ANY session
    #[tokio::test]
    #[serial]
    async fn test_cross_session_delete_blocked() {
        let ctx = TestContext::new().await;
        let executor = PlanExecutor::new(&ctx.pool);

        // Session A creates a focused task
        let session_a = "session-A-cross-test";
        std::env::set_var("IE_SESSION_ID", session_a);

        let request1 = PlanRequest {
            tasks: vec![TaskTree {
                name: Some("Session A Focus".to_string()),
                spec: Some("## Goal\nSession A's task".to_string()),
                priority: None,
                children: None,
                depends_on: None,
                id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: None,
                ..Default::default()
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        let task_id = *result1.task_id_map.get("Session A Focus").unwrap();

        // Verify Session A has focus
        let focus_a: Option<(i64,)> =
            sqlx::query_as("SELECT current_task_id FROM sessions WHERE session_id = ?")
                .bind(session_a)
                .fetch_optional(&ctx.pool)
                .await
                .unwrap();
        assert_eq!(
            focus_a.map(|r| r.0),
            Some(task_id),
            "Session A should have focus"
        );

        // Session B tries to delete Session A's focus
        let session_b = "session-B-cross-test";
        std::env::set_var("IE_SESSION_ID", session_b);

        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: None,
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                id: Some(task_id),
                status: None,
                active_form: None,
                parent_id: None,
                delete: Some(true),
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();

        // Session B should NOT be able to delete Session A's focus
        assert!(
            !result2.success,
            "Session B should NOT be able to delete Session A's focus"
        );
        assert_eq!(result2.deleted_count, 0);

        // Error should mention Session A
        let error = result2.error.as_ref().unwrap();
        assert!(
            error.contains(session_a),
            "Error should mention session '{}': {}",
            session_a,
            error
        );

        // Verify task still exists
        let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        assert_eq!(exists.0, 1, "Task should still exist");

        // Clean up
        std::env::remove_var("IE_SESSION_ID");
    }
}
