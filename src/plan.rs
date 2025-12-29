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
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TaskTree {
    /// Task name (used as identifier for lookups)
    pub name: String,

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

    /// Optional explicit task ID (for forced updates)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<i64>,

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

impl PlanResult {
    /// Create a successful result with optional focused task
    pub fn success(
        task_id_map: HashMap<String, i64>,
        created_count: usize,
        updated_count: usize,
        dependency_count: usize,
        focused_task: Option<crate::db::models::TaskWithEvents>,
    ) -> Self {
        Self {
            success: true,
            task_id_map,
            created_count,
            updated_count,
            dependency_count,
            focused_task,
            error: None,
            warnings: Vec::new(),
        }
    }

    /// Create a successful result with warnings
    pub fn success_with_warnings(
        task_id_map: HashMap<String, i64>,
        created_count: usize,
        updated_count: usize,
        dependency_count: usize,
        focused_task: Option<crate::db::models::TaskWithEvents>,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            success: true,
            task_id_map,
            created_count,
            updated_count,
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
pub fn extract_all_names(tasks: &[TaskTree]) -> Vec<String> {
    let mut names = Vec::new();

    for task in tasks {
        names.push(task.name.clone());

        if let Some(children) = &task.children {
            names.extend(extract_all_names(children));
        }
    }

    names
}

/// Flatten task tree into a linear list with parent information
#[derive(Debug, Clone, PartialEq)]
pub struct FlatTask {
    pub name: String,
    pub spec: Option<String>,
    pub priority: Option<PriorityValue>,
    /// Parent from children nesting (takes precedence over explicit_parent_id)
    pub parent_name: Option<String>,
    pub depends_on: Vec<String>,
    pub task_id: Option<i64>,
    pub status: Option<TaskStatus>,
    pub active_form: Option<String>,
    /// Explicit parent_id from JSON
    /// - None: use default behavior (auto-parent to focused task for new root tasks)
    /// - Some(None): explicitly create as root task (no parent)
    /// - Some(Some(id)): explicitly set parent to task with given ID
    pub explicit_parent_id: Option<Option<i64>>,
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
            task_id: task.task_id,
            status: task.status.clone(),
            active_form: task.active_form.clone(),
            explicit_parent_id: task.parent_id,
        };

        flat.push(flat_task);

        // Recursively flatten children
        if let Some(children) = &task.children {
            flat.extend(flatten_task_tree_recursive(
                children,
                Some(task.name.clone()),
            ));
        }
    }

    flat
}

/// Operation classification result
#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Create(FlatTask),
    Update { task_id: i64, task: FlatTask },
}

/// Classify tasks into create/update operations based on existing task IDs
///
/// # Arguments
/// * `flat_tasks` - Flattened task list
/// * `existing_names` - Map of existing task names to their IDs
///
/// # Returns
/// Classified operations (create or update)
pub fn classify_operations(
    flat_tasks: &[FlatTask],
    existing_names: &HashMap<String, i64>,
) -> Vec<Operation> {
    let mut operations = Vec::new();

    for task in flat_tasks {
        // Priority: explicit task_id > name lookup > create
        let operation = if let Some(task_id) = task.task_id {
            // Explicit task_id → forced update
            Operation::Update {
                task_id,
                task: task.clone(),
            }
        } else if let Some(&task_id) = existing_names.get(&task.name) {
            // Name found in DB → update
            Operation::Update {
                task_id,
                task: task.clone(),
            }
        } else {
            // Not found → create
            Operation::Create(task.clone())
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

        for task in &flat_tasks {
            // Check if task is transitioning to 'doing' status and validate spec
            let is_becoming_doing = task.status.as_ref() == Some(&TaskStatus::Doing);
            let has_spec = task
                .spec
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

            if let Some(existing_info) = existing.get(&task.name) {
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
                            task.name
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
                            task.name, e
                        )));
                    }
                }

                task_id_map.insert(task.name.clone(), existing_info.id);
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
                        task.name
                    )));
                }

                let id = task_mgr
                    .create_task_in_tx(
                        &mut tx,
                        &task.name,
                        task.spec.as_deref(),
                        task.priority.as_ref().map(|p| p.to_int()),
                        task.status.as_ref().map(|s| s.as_db_str()),
                        task.active_form.as_deref(),
                        "ai", // Plan-created tasks are AI-owned
                    )
                    .await?;
                task_id_map.insert(task.name.clone(), id);
                newly_created_names.insert(task.name.clone());
                created_count += 1;

                // Warning: new task without spec (non-doing tasks only, doing already validated)
                if !has_spec && !is_becoming_doing {
                    warnings.push(format!(
                        "Task '{}' has no description. Consider adding one for better context.",
                        task.name
                    ));
                }
            }
        }

        // 11. Build parent-child relationships via TaskManager
        for task in &flat_tasks {
            if let Some(parent_name) = &task.parent_name {
                let task_id = task_id_map.get(&task.name).ok_or_else(|| {
                    IntentError::InvalidInput(format!("Task not found: {}", task.name))
                })?;
                let parent_id = task_id_map.get(parent_name).ok_or_else(|| {
                    IntentError::InvalidInput(format!("Parent task not found: {}", parent_name))
                })?;
                task_mgr
                    .set_parent_in_tx(&mut tx, *task_id, *parent_id)
                    .await?;
            }
        }

        // 11b. Handle explicit parent_id (takes precedence over auto-parenting)
        // Priority: children nesting > explicit parent_id > auto-parent
        for task in &flat_tasks {
            // Skip if parent was set via children nesting
            if task.parent_name.is_some() {
                continue;
            }

            // Handle explicit parent_id
            if let Some(explicit_parent) = &task.explicit_parent_id {
                let task_id = task_id_map.get(&task.name).ok_or_else(|| {
                    IntentError::InvalidInput(format!("Task not found: {}", task.name))
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

        // 11c. Auto-parent newly created root tasks to default_parent_id (focused task)
        if let Some(default_parent) = self.default_parent_id {
            for task in &flat_tasks {
                // Only auto-parent if:
                // 1. Task was newly created (not updated)
                // 2. Task has no explicit parent in the plan (children nesting)
                // 3. Task has no explicit parent_id in JSON
                if newly_created_names.contains(&task.name)
                    && task.parent_name.is_none()
                    && task.explicit_parent_id.is_none()
                {
                    if let Some(&task_id) = task_id_map.get(&task.name) {
                        task_mgr
                            .set_parent_in_tx(&mut tx, task_id, default_parent)
                            .await?;
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
        // Find the doing task in the batch
        let doing_task = flat_tasks
            .iter()
            .find(|task| matches!(task.status, Some(TaskStatus::Doing)));

        let focused_task_response = if let Some(doing_task) = doing_task {
            // Get the task ID from the map
            if let Some(&task_id) = task_id_map.get(&doing_task.name) {
                // Call task_start with events to get full context
                let response = task_mgr.start_task(task_id, true).await?;
                Some(response)
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
            if !task.depends_on.is_empty() {
                let blocked_id = task_id_map.get(&task.name).ok_or_else(|| {
                    IntentError::InvalidInput(format!("Task not found: {}", task.name))
                })?;

                for dep_name in &task.depends_on {
                    let blocking_id = task_id_map.get(dep_name).ok_or_else(|| {
                        IntentError::InvalidInput(format!(
                            "Dependency '{}' not found for task '{}'",
                            dep_name, task.name
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
        let task_names: std::collections::HashSet<_> =
            flat_tasks.iter().map(|t| t.name.as_str()).collect();

        for task in flat_tasks {
            for dep_name in &task.depends_on {
                if !task_names.contains(dep_name.as_str()) {
                    return Err(IntentError::InvalidInput(format!(
                        "Task '{}' depends on '{}', but '{}' is not in the plan",
                        task.name, dep_name, dep_name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate batch-level single doing constraint
    /// Ensures only one task in the request batch can have status='doing'
    /// (Database can have multiple 'doing' tasks to support hierarchical workflows)
    fn validate_batch_single_doing(&self, flat_tasks: &[FlatTask]) -> Result<()> {
        // Find all tasks in the request that want to be doing
        let doing_tasks: Vec<&FlatTask> = flat_tasks
            .iter()
            .filter(|task| matches!(task.status, Some(TaskStatus::Doing)))
            .collect();

        // If more than one task in the request wants to be doing, that's an error
        if doing_tasks.len() > 1 {
            let names: Vec<&str> = doing_tasks.iter().map(|t| t.name.as_str()).collect();
            return Err(IntentError::InvalidInput(format!(
                "Batch single doing constraint violated: only one task per batch can have status='doing'. Found: {}",
                names.join(", ")
            )));
        }

        Ok(())
    }

    /// Detect circular dependencies using Tarjan's algorithm for strongly connected components
    fn detect_circular_dependencies(&self, flat_tasks: &[FlatTask]) -> Result<()> {
        if flat_tasks.is_empty() {
            return Ok(());
        }

        // Build name-to-index mapping
        let name_to_idx: HashMap<&str, usize> = flat_tasks
            .iter()
            .enumerate()
            .map(|(i, t)| (t.name.as_str(), i))
            .collect();

        // Build dependency graph (adjacency list)
        let mut graph: Vec<Vec<usize>> = vec![Vec::new(); flat_tasks.len()];
        for (idx, task) in flat_tasks.iter().enumerate() {
            for dep_name in &task.depends_on {
                if let Some(&dep_idx) = name_to_idx.get(dep_name.as_str()) {
                    graph[idx].push(dep_idx);
                }
            }
        }

        // Check for self-loops first
        for task in flat_tasks {
            if task.depends_on.contains(&task.name) {
                return Err(IntentError::InvalidInput(format!(
                    "Circular dependency detected: task '{}' depends on itself",
                    task.name
                )));
            }
        }

        // Run Tarjan's SCC algorithm
        let sccs = self.tarjan_scc(&graph);

        // Check for cycles (any SCC with size > 1)
        for scc in sccs {
            if scc.len() > 1 {
                // Found a cycle - build error message
                let cycle_names: Vec<&str> = scc
                    .iter()
                    .map(|&idx| flat_tasks[idx].name.as_str())
                    .collect();

                return Err(IntentError::InvalidInput(format!(
                    "Circular dependency detected: {}",
                    cycle_names.join(" → ")
                )));
            }
        }

        Ok(())
    }

    /// Tarjan's algorithm for finding strongly connected components
    /// Returns a list of SCCs, where each SCC is a list of node indices
    fn tarjan_scc(&self, graph: &[Vec<usize>]) -> Vec<Vec<usize>> {
        let n = graph.len();
        let mut index = 0;
        let mut stack = Vec::new();
        let mut indices = vec![None; n];
        let mut lowlinks = vec![0; n];
        let mut on_stack = vec![false; n];
        let mut sccs = Vec::new();

        #[allow(clippy::too_many_arguments)]
        fn strongconnect(
            v: usize,
            graph: &[Vec<usize>],
            index: &mut usize,
            stack: &mut Vec<usize>,
            indices: &mut [Option<usize>],
            lowlinks: &mut [usize],
            on_stack: &mut [bool],
            sccs: &mut Vec<Vec<usize>>,
        ) {
            // Set the depth index for v to the smallest unused index
            indices[v] = Some(*index);
            lowlinks[v] = *index;
            *index += 1;
            stack.push(v);
            on_stack[v] = true;

            // Consider successors of v
            for &w in &graph[v] {
                if indices[w].is_none() {
                    // Successor w has not yet been visited; recurse on it
                    strongconnect(w, graph, index, stack, indices, lowlinks, on_stack, sccs);
                    lowlinks[v] = lowlinks[v].min(lowlinks[w]);
                } else if on_stack[w] {
                    // Successor w is in stack and hence in the current SCC
                    lowlinks[v] = lowlinks[v].min(indices[w].unwrap());
                }
            }

            // If v is a root node, pop the stack and generate an SCC
            if lowlinks[v] == indices[v].unwrap() {
                let mut scc = Vec::new();
                loop {
                    let w = stack.pop().unwrap();
                    on_stack[w] = false;
                    scc.push(w);
                    if w == v {
                        break;
                    }
                }
                sccs.push(scc);
            }
        }

        // Find SCCs for all nodes
        for v in 0..n {
            if indices[v].is_none() {
                strongconnect(
                    v,
                    graph,
                    &mut index,
                    &mut stack,
                    &mut indices,
                    &mut lowlinks,
                    &mut on_stack,
                    &mut sccs,
                );
            }
        }

        sccs
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
        assert_eq!(request.tasks[0].name, "Test Task");
        assert_eq!(request.tasks[0].spec, None);
        assert_eq!(request.tasks[0].priority, None);
        assert_eq!(request.tasks[0].children, None);
        assert_eq!(request.tasks[0].depends_on, None);
        assert_eq!(request.tasks[0].task_id, None);
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
        assert_eq!(parent.name, "Parent Task");
        assert_eq!(parent.spec, Some("Parent spec".to_string()));
        assert_eq!(parent.priority, Some(PriorityValue::High));
        assert_eq!(parent.task_id, Some(42));

        let children = parent.children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "Child Task");

        let depends = parent.depends_on.as_ref().unwrap();
        assert_eq!(depends.len(), 1);
        assert_eq!(depends[0], "Other Task");
    }

    #[test]
    fn test_plan_request_serialization() {
        let request = PlanRequest {
            tasks: vec![TaskTree {
                name: "Test Task".to_string(),
                spec: Some("Test spec".to_string()),
                priority: Some(PriorityValue::Medium),
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
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

        let result = PlanResult::success(map.clone(), 2, 0, 1, None);

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
            name: "Parent".to_string(),
            spec: None,
            priority: None,
            children: Some(vec![
                TaskTree {
                    name: "Child 1".to_string(),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Child 2".to_string(),
                    spec: None,
                    priority: Some(PriorityValue::High),
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
            ]),
            depends_on: None,
            task_id: None,
            status: None,
            active_form: None,
            parent_id: None,
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
                name: "Task 1".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            },
            TaskTree {
                name: "Task 2".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            },
        ];

        let names = extract_all_names(&tasks);
        assert_eq!(names, vec!["Task 1", "Task 2"]);
    }

    #[test]
    fn test_extract_all_names_nested() {
        let tasks = vec![TaskTree {
            name: "Parent".to_string(),
            spec: None,
            priority: None,
            children: Some(vec![
                TaskTree {
                    name: "Child 1".to_string(),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Child 2".to_string(),
                    spec: None,
                    priority: None,
                    children: Some(vec![TaskTree {
                        name: "Grandchild".to_string(),
                        spec: None,
                        priority: None,
                        children: None,
                        depends_on: None,
                        task_id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                    }]),
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
            ]),
            depends_on: None,
            task_id: None,
            status: None,
            active_form: None,
            parent_id: None,
        }];

        let names = extract_all_names(&tasks);
        assert_eq!(names, vec!["Parent", "Child 1", "Child 2", "Grandchild"]);
    }

    #[test]
    fn test_flatten_task_tree_simple() {
        let tasks = vec![TaskTree {
            name: "Task 1".to_string(),
            spec: Some("Spec 1".to_string()),
            priority: Some(PriorityValue::High),
            children: None,
            depends_on: Some(vec!["Task 0".to_string()]),
            task_id: None,
            status: None,
            active_form: None,
            parent_id: None,
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].name, "Task 1");
        assert_eq!(flat[0].spec, Some("Spec 1".to_string()));
        assert_eq!(flat[0].priority, Some(PriorityValue::High));
        assert_eq!(flat[0].parent_name, None);
        assert_eq!(flat[0].depends_on, vec!["Task 0"]);
    }

    #[test]
    fn test_flatten_task_tree_nested() {
        let tasks = vec![TaskTree {
            name: "Parent".to_string(),
            spec: None,
            priority: None,
            children: Some(vec![
                TaskTree {
                    name: "Child 1".to_string(),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Child 2".to_string(),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
            ]),
            depends_on: None,
            task_id: None,
            status: None,
            active_form: None,
            parent_id: None,
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 3);

        // Parent should have no parent_name
        assert_eq!(flat[0].name, "Parent");
        assert_eq!(flat[0].parent_name, None);

        // Children should have Parent as parent_name
        assert_eq!(flat[1].name, "Child 1");
        assert_eq!(flat[1].parent_name, Some("Parent".to_string()));

        assert_eq!(flat[2].name, "Child 2");
        assert_eq!(flat[2].parent_name, Some("Parent".to_string()));
    }

    #[test]
    fn test_classify_operations_all_create() {
        let flat_tasks = vec![
            FlatTask {
                name: "Task 1".to_string(),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                task_id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
            },
            FlatTask {
                name: "Task 2".to_string(),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                task_id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
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
                name: "Task 1".to_string(),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                task_id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
            },
            FlatTask {
                name: "Task 2".to_string(),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                task_id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
            },
        ];

        let mut existing = HashMap::new();
        existing.insert("Task 1".to_string(), 1);
        existing.insert("Task 2".to_string(), 2);

        let operations = classify_operations(&flat_tasks, &existing);

        assert_eq!(operations.len(), 2);
        assert!(matches!(
            operations[0],
            Operation::Update { task_id: 1, .. }
        ));
        assert!(matches!(
            operations[1],
            Operation::Update { task_id: 2, .. }
        ));
    }

    #[test]
    fn test_classify_operations_mixed() {
        let flat_tasks = vec![
            FlatTask {
                name: "Existing Task".to_string(),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                task_id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
            },
            FlatTask {
                name: "New Task".to_string(),
                spec: None,
                priority: None,
                parent_name: None,
                depends_on: vec![],
                task_id: None,
                status: None,
                active_form: None,
                explicit_parent_id: None,
            },
        ];

        let mut existing = HashMap::new();
        existing.insert("Existing Task".to_string(), 42);

        let operations = classify_operations(&flat_tasks, &existing);

        assert_eq!(operations.len(), 2);
        assert!(matches!(
            operations[0],
            Operation::Update { task_id: 42, .. }
        ));
        assert!(matches!(operations[1], Operation::Create(_)));
    }

    #[test]
    fn test_classify_operations_explicit_task_id() {
        let flat_tasks = vec![FlatTask {
            name: "Task".to_string(),
            spec: None,
            priority: None,
            parent_name: None,
            depends_on: vec![],
            task_id: Some(99), // Explicit task_id
            status: None,
            active_form: None,
            explicit_parent_id: None,
        }];

        let existing = HashMap::new(); // Not in existing

        let operations = classify_operations(&flat_tasks, &existing);

        // Should still be update because of explicit task_id
        assert_eq!(operations.len(), 1);
        assert!(matches!(
            operations[0],
            Operation::Update { task_id: 99, .. }
        ));
    }

    #[test]
    fn test_find_duplicate_names_no_duplicates() {
        let tasks = vec![
            TaskTree {
                name: "Task 1".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            },
            TaskTree {
                name: "Task 2".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            },
        ];

        let duplicates = find_duplicate_names(&tasks);
        assert_eq!(duplicates.len(), 0);
    }

    #[test]
    fn test_find_duplicate_names_with_duplicates() {
        let tasks = vec![
            TaskTree {
                name: "Duplicate".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            },
            TaskTree {
                name: "Unique".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            },
            TaskTree {
                name: "Duplicate".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            },
        ];

        let duplicates = find_duplicate_names(&tasks);
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0], "Duplicate");
    }

    #[test]
    fn test_find_duplicate_names_nested() {
        let tasks = vec![TaskTree {
            name: "Parent".to_string(),
            spec: None,
            priority: None,
            children: Some(vec![TaskTree {
                name: "Parent".to_string(), // Duplicate name in child
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            }]),
            depends_on: None,
            task_id: None,
            status: None,
            active_form: None,
            parent_id: None,
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
            name: "Root".to_string(),
            spec: None,
            priority: None,
            children: Some(vec![TaskTree {
                name: "Level1".to_string(),
                spec: None,
                priority: None,
                children: Some(vec![TaskTree {
                    name: "Level2".to_string(),
                    spec: None,
                    priority: None,
                    children: Some(vec![TaskTree {
                        name: "Level3".to_string(),
                        spec: None,
                        priority: None,
                        children: None,
                        depends_on: None,
                        task_id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                    }]),
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                }]),
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            }]),
            depends_on: None,
            task_id: None,
            status: None,
            active_form: None,
            parent_id: None,
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 4);

        // Check parent relationships
        assert_eq!(flat[0].name, "Root");
        assert_eq!(flat[0].parent_name, None);

        assert_eq!(flat[1].name, "Level1");
        assert_eq!(flat[1].parent_name, Some("Root".to_string()));

        assert_eq!(flat[2].name, "Level2");
        assert_eq!(flat[2].parent_name, Some("Level1".to_string()));

        assert_eq!(flat[3].name, "Level3");
        assert_eq!(flat[3].parent_name, Some("Level2".to_string()));
    }

    #[test]
    fn test_flatten_task_tree_many_siblings() {
        let children: Vec<TaskTree> = (0..10)
            .map(|i| TaskTree {
                name: format!("Child {}", i),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            })
            .collect();

        let tasks = vec![TaskTree {
            name: "Parent".to_string(),
            spec: None,
            priority: None,
            children: Some(children),
            depends_on: None,
            task_id: None,
            status: None,
            active_form: None,
            parent_id: None,
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
                name: "Task 1".to_string(),
                spec: None,
                priority: None,
                children: Some(vec![
                    TaskTree {
                        name: "Task 1.1".to_string(),
                        spec: None,
                        priority: None,
                        children: None,
                        depends_on: None,
                        task_id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                    },
                    TaskTree {
                        name: "Task 1.2".to_string(),
                        spec: None,
                        priority: None,
                        children: Some(vec![TaskTree {
                            name: "Task 1.2.1".to_string(),
                            spec: None,
                            priority: None,
                            children: None,
                            depends_on: None,
                            task_id: None,
                            status: None,
                            active_form: None,
                            parent_id: None,
                        }]),
                        depends_on: None,
                        task_id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                    },
                ]),
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            },
            TaskTree {
                name: "Task 2".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: Some(vec!["Task 1".to_string()]),
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
            },
        ];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 5);

        // Verify structure
        assert_eq!(flat[0].name, "Task 1");
        assert_eq!(flat[0].parent_name, None);

        assert_eq!(flat[1].name, "Task 1.1");
        assert_eq!(flat[1].parent_name, Some("Task 1".to_string()));

        assert_eq!(flat[2].name, "Task 1.2");
        assert_eq!(flat[2].parent_name, Some("Task 1".to_string()));

        assert_eq!(flat[3].name, "Task 1.2.1");
        assert_eq!(flat[3].parent_name, Some("Task 1.2".to_string()));

        assert_eq!(flat[4].name, "Task 2");
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
                name: "Integration Test Plan".to_string(),
                spec: Some("Test plan execution end-to-end".to_string()),
                priority: Some(PriorityValue::High),
                children: Some(vec![
                    TaskTree {
                        name: "Subtask A".to_string(),
                        spec: Some("First subtask".to_string()),
                        priority: None,
                        children: None,
                        depends_on: None,
                        task_id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                    },
                    TaskTree {
                        name: "Subtask B".to_string(),
                        spec: Some("Second subtask depends on A".to_string()),
                        priority: None,
                        children: None,
                        depends_on: Some(vec!["Subtask A".to_string()]),
                        task_id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                    },
                ]),
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
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
                name: "Idempotent Task".to_string(),
                spec: Some("Initial spec".to_string()),
                priority: Some(PriorityValue::High),
                children: Some(vec![
                    TaskTree {
                        name: "Child 1".to_string(),
                        spec: Some("Child spec 1".to_string()),
                        priority: None,
                        children: None,
                        depends_on: None,
                        task_id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                    },
                    TaskTree {
                        name: "Child 2".to_string(),
                        spec: Some("Child spec 2".to_string()),
                        priority: Some(PriorityValue::Low),
                        children: None,
                        depends_on: None,
                        task_id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                    },
                ]),
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
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
                name: "Idempotent Task".to_string(),
                spec: Some("Updated spec".to_string()), // Changed
                priority: Some(PriorityValue::Critical), // Changed
                children: Some(vec![
                    TaskTree {
                        name: "Child 1".to_string(),
                        spec: Some("Updated child spec 1".to_string()), // Changed
                        priority: None,
                        children: None,
                        depends_on: None,
                        task_id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                    },
                    TaskTree {
                        name: "Child 2".to_string(),
                        spec: Some("Child spec 2".to_string()), // Unchanged
                        priority: Some(PriorityValue::Low),
                        children: None,
                        depends_on: None,
                        task_id: None,
                        status: None,
                        active_form: None,
                        parent_id: None,
                    },
                ]),
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
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
                    name: "Foundation".to_string(),
                    spec: Some("Base layer".to_string()),
                    priority: Some(PriorityValue::Critical),
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Layer 1".to_string(),
                    spec: Some("Depends on Foundation".to_string()),
                    priority: Some(PriorityValue::High),
                    children: None,
                    depends_on: Some(vec!["Foundation".to_string()]),
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Layer 2".to_string(),
                    spec: Some("Depends on Layer 1".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Layer 1".to_string()]),
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Integration".to_string(),
                    spec: Some("Depends on both Foundation and Layer 2".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Foundation".to_string(), "Layer 2".to_string()]),
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
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
                name: "Task A".to_string(),
                spec: Some("Depends on non-existent task".to_string()),
                priority: None,
                children: None,
                depends_on: Some(vec!["NonExistent".to_string()]),
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
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
                    name: "Task A".to_string(),
                    spec: Some("Depends on B".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task B".to_string()]),
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Task B".to_string(),
                    spec: Some("Depends on A".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task A".to_string()]),
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
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
                    name: "Task A".to_string(),
                    spec: Some("Depends on B".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task B".to_string()]),
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Task B".to_string(),
                    spec: Some("Depends on C".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task C".to_string()]),
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Task C".to_string(),
                    spec: Some("Depends on A".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task A".to_string()]),
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
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
                    name: "Task A".to_string(),
                    spec: Some("Root task".to_string()),
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Task B".to_string(),
                    spec: Some("Depends on A".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task A".to_string()]),
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Task C".to_string(),
                    spec: Some("Depends on A".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task A".to_string()]),
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Task D".to_string(),
                    spec: Some("Depends on B and C".to_string()),
                    priority: None,
                    children: None,
                    depends_on: Some(vec!["Task B".to_string(), "Task C".to_string()]),
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
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
                name: "Task A".to_string(),
                spec: Some("Depends on itself".to_string()),
                priority: None,
                children: None,
                depends_on: Some(vec!["Task A".to_string()]),
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
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
                    name: "Task A".to_string(),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Task B".to_string(),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
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
                name: format!("Task {}", i),
                spec: Some(format!("Spec for task {}", i)),
                priority: Some(PriorityValue::Medium),
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
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
                name: format!("Level {}", current),
                spec: Some(format!("Task at depth {}", current)),
                priority: Some(PriorityValue::Low),
                children: if current < depth {
                    Some(vec![build_deep_tree(depth, current + 1)])
                } else {
                    None
                },
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
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
            name: "Full Task".to_string(),
            spec: Some("Detailed spec".to_string()),
            priority: Some(PriorityValue::Critical),
            children: None,
            depends_on: Some(vec!["Dep1".to_string(), "Dep2".to_string()]),
            task_id: Some(42),
            status: None,
            active_form: None,
            parent_id: None,
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 1);

        let task = &flat[0];
        assert_eq!(task.name, "Full Task");
        assert_eq!(task.spec, Some("Detailed spec".to_string()));
        assert_eq!(task.priority, Some(PriorityValue::Critical));
        assert_eq!(task.depends_on, vec!["Dep1", "Dep2"]);
        assert_eq!(task.task_id, Some(42));
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
                name: "Test Active Form Task".to_string(),
                spec: Some("Testing complete dataflow".to_string()),
                priority: Some(PriorityValue::High),
                children: None,
                depends_on: None,
                task_id: None,
                status: Some(TaskStatus::Doing),
                active_form: Some("Testing complete dataflow now".to_string()),
                parent_id: None,
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
            name: "Task with explicit parent".to_string(),
            spec: None,
            priority: None,
            children: None,
            depends_on: None,
            task_id: None,
            status: None,
            active_form: None,
            parent_id: Some(Some(99)),
        }];

        let flat = flatten_task_tree(&tasks);
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].explicit_parent_id, Some(Some(99)));
    }

    #[test]
    fn test_flatten_propagates_null_parent_id() {
        let tasks = vec![TaskTree {
            name: "Explicit root task".to_string(),
            spec: None,
            priority: None,
            children: None,
            depends_on: None,
            task_id: None,
            status: None,
            active_form: None,
            parent_id: Some(None), // Explicit null
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
                name: "Parent Task".to_string(),
                spec: Some("This is the parent".to_string()),
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: None,
            }],
        };

        let executor = PlanExecutor::new(&ctx.pool);
        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);
        let parent_id = *result1.task_id_map.get("Parent Task").unwrap();

        // Now create a child task using explicit parent_id
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: "Child Task".to_string(),
                spec: Some("This uses explicit parent_id".to_string()),
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: Some(Some(parent_id)),
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
                name: "Explicit Root Task".to_string(),
                spec: Some("Should be root despite default parent".to_string()),
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: Some(None), // Explicit null = root
            }],
        };

        // Create executor with a default parent
        // First create a "default parent" task
        let parent_request = PlanRequest {
            tasks: vec![TaskTree {
                name: "Default Parent".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: None,
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
                name: "Parent via Nesting".to_string(),
                spec: Some("Test parent spec".to_string()),
                priority: None,
                children: Some(vec![TaskTree {
                    name: "Child via Nesting".to_string(),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: Some(Some(999)), // This should be ignored!
                }]),
                depends_on: None,
                task_id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: None,
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
                    name: "Task A".to_string(),
                    spec: Some("Task A spec".to_string()),
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: Some(TaskStatus::Doing),
                    active_form: None,
                    parent_id: None,
                },
                TaskTree {
                    name: "Task B".to_string(),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: None,
                    active_form: None,
                    parent_id: None,
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
                name: "Task B".to_string(), // Same name = update
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: None,
                active_form: None,
                parent_id: Some(Some(task_a_id)), // Set parent to Task A
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
                name: "Parent Task".to_string(),
                spec: Some("Parent spec".to_string()),
                priority: None,
                children: Some(vec![TaskTree {
                    name: "Child Task".to_string(),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: Some(TaskStatus::Todo), // Child is not done
                    active_form: None,
                    parent_id: None,
                }]),
                depends_on: None,
                task_id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: None,
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);

        // Try to complete parent while child is incomplete
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: "Parent Task".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: Some(TaskStatus::Done), // Try to set done
                active_form: None,
                parent_id: None,
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
                name: "Parent Task".to_string(),
                spec: Some("Parent spec".to_string()),
                priority: None,
                children: Some(vec![TaskTree {
                    name: "Child Task".to_string(),
                    spec: None,
                    priority: None,
                    children: None,
                    depends_on: None,
                    task_id: None,
                    status: Some(TaskStatus::Todo),
                    active_form: None,
                    parent_id: None,
                }]),
                depends_on: None,
                task_id: None,
                status: Some(TaskStatus::Doing),
                active_form: None,
                parent_id: None,
            }],
        };

        let result1 = executor.execute(&request1).await.unwrap();
        assert!(result1.success);

        // Complete child first
        let request2 = PlanRequest {
            tasks: vec![TaskTree {
                name: "Child Task".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: Some(TaskStatus::Done),
                active_form: None,
                parent_id: None,
            }],
        };

        let result2 = executor.execute(&request2).await.unwrap();
        assert!(result2.success);

        // Now parent can be completed
        let request3 = PlanRequest {
            tasks: vec![TaskTree {
                name: "Parent Task".to_string(),
                spec: None,
                priority: None,
                children: None,
                depends_on: None,
                task_id: None,
                status: Some(TaskStatus::Done),
                active_form: None,
                parent_id: None,
            }],
        };

        let result3 = executor.execute(&request3).await.unwrap();
        assert!(result3.success, "Should succeed when child is complete");
    }
}
