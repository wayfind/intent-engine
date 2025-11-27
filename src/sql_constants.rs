//! SQL query constants and fragments
//!
//! This module centralizes frequently-used SQL query strings and fragments to:
//! 1. Reduce code duplication
//! 2. Ensure consistency across the codebase
//! 3. Make query modifications easier to maintain
//!
//! # Design Philosophy
//!
//! - **Column Lists**: Reusable SELECT column specifications
//! - **Base Queries**: Complete SELECT statements with standard FROM clauses
//! - **Existence Checks**: Common existence validation patterns
//!
//! Note: Dynamic WHERE clauses are still built inline for flexibility.

// ============================================================================
// Task Queries
// ============================================================================

/// Standard column list for task queries (includes spec column)
///
/// Used when fetching complete task data with specification.
/// Columns: id, parent_id, name, spec, status, complexity, priority,
///          first_todo_at, first_doing_at, first_done_at, active_form
pub const TASK_COLUMNS: &str =
    "id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form";

/// Task column list without spec (uses NULL placeholder)
///
/// Used when spec is not needed but schema compatibility is required.
/// Columns: id, parent_id, name, NULL as spec, status, complexity, priority,
///          first_todo_at, first_doing_at, first_done_at, active_form
pub const TASK_COLUMNS_NO_SPEC: &str =
    "id, parent_id, name, NULL as spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form";

/// Base SELECT query for tasks (with spec)
///
/// Returns all task columns. Add WHERE clauses as needed.
pub const SELECT_TASK_FULL: &str = const_format::formatcp!("SELECT {} FROM tasks", TASK_COLUMNS);

/// Base SELECT query for tasks (without spec, using NULL)
///
/// Returns all task columns except spec (NULL as spec). Add WHERE clauses as needed.
pub const SELECT_TASK_NO_SPEC: &str =
    const_format::formatcp!("SELECT {} FROM tasks WHERE 1=1", TASK_COLUMNS_NO_SPEC);

/// Check if a task exists by ID
pub const CHECK_TASK_EXISTS: &str = "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?)";

/// Get task name by ID
pub const SELECT_TASK_NAME: &str = "SELECT name FROM tasks WHERE id = ?";

/// Get task name and parent_id by ID
pub const SELECT_TASK_NAME_PARENT: &str = "SELECT name, parent_id FROM tasks WHERE id = ?";

/// Get parent_id for a task
pub const SELECT_TASK_PARENT_ID: &str = "SELECT parent_id FROM tasks WHERE id = ?";

/// Count total tasks
pub const COUNT_TASKS_TOTAL: &str = "SELECT COUNT(*) FROM tasks";

/// Count incomplete subtasks of a parent
pub const COUNT_INCOMPLETE_CHILDREN: &str =
    "SELECT COUNT(*) FROM tasks WHERE parent_id = ? AND status != 'done'";

/// Count incomplete children excluding specific task
pub const COUNT_INCOMPLETE_CHILDREN_EXCLUDE: &str =
    "SELECT COUNT(*) FROM tasks WHERE parent_id = ? AND status != 'done' AND id != ?";

/// Count total children of a parent
pub const COUNT_CHILDREN_TOTAL: &str = "SELECT COUNT(*) FROM tasks WHERE parent_id = ?";

/// Count tasks with 'doing' status
pub const COUNT_TASKS_DOING: &str = "SELECT COUNT(*) FROM tasks WHERE status = 'doing'";

/// Count incomplete tasks (todo or doing)
pub const COUNT_TASKS_INCOMPLETE: &str =
    "SELECT COUNT(*) FROM tasks WHERE status IN ('todo', 'doing')";

/// Count incomplete tasks excluding specific task
pub const COUNT_INCOMPLETE_TASKS_EXCLUDE: &str =
    "SELECT COUNT(*) FROM tasks WHERE status != 'done' AND id != ?";

// ============================================================================
// Event Queries
// ============================================================================

/// Standard column list for event queries
///
/// Columns: id, task_id, timestamp, log_type, discussion_data
pub const EVENT_COLUMNS: &str = "id, task_id, timestamp, log_type, discussion_data";

/// Base SELECT query for events
///
/// Returns all event columns. Add WHERE clauses as needed.
pub const SELECT_EVENT_FULL: &str = const_format::formatcp!("SELECT {} FROM events", EVENT_COLUMNS);

/// SELECT event with WHERE id = ? condition
pub const SELECT_EVENT_BY_ID: &str =
    const_format::formatcp!("SELECT {} FROM events WHERE id = ?", EVENT_COLUMNS);

/// Base SELECT query for events with dynamic WHERE clause building
pub const SELECT_EVENT_BASE: &str =
    const_format::formatcp!("SELECT {} FROM events WHERE 1=1", EVENT_COLUMNS);

/// Count total events
pub const COUNT_EVENTS_TOTAL: &str = "SELECT COUNT(*) FROM events";

/// Count events for a specific task
pub const COUNT_EVENTS_FOR_TASK: &str = "SELECT COUNT(*) FROM events WHERE task_id = ?";

/// Check if a task exists (also used by event validation)
pub const CHECK_TASK_EXISTS_FOR_EVENT: &str = CHECK_TASK_EXISTS;

// ============================================================================
// Full-Text Search (FTS5) Queries
// ============================================================================

/// Base query for task FTS5 search
pub const SELECT_TASKS_FTS_BASE: &str = "SELECT rowid FROM tasks_fts WHERE ";

/// Count task FTS5 matches
pub const COUNT_TASKS_FTS: &str = "SELECT COUNT(*) FROM tasks_fts WHERE name MATCH ?";

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_columns_format() {
        assert!(TASK_COLUMNS.contains("id"));
        assert!(TASK_COLUMNS.contains("parent_id"));
        assert!(TASK_COLUMNS.contains("spec"));
        assert!(TASK_COLUMNS.contains("active_form"));
    }

    #[test]
    fn test_task_columns_no_spec_format() {
        assert!(TASK_COLUMNS_NO_SPEC.contains("NULL as spec"));
        assert!(TASK_COLUMNS_NO_SPEC.contains("active_form"));
    }

    #[test]
    fn test_event_columns_format() {
        assert!(EVENT_COLUMNS.contains("id"));
        assert!(EVENT_COLUMNS.contains("task_id"));
        assert!(EVENT_COLUMNS.contains("discussion_data"));
    }

    #[test]
    fn test_select_task_full() {
        assert_eq!(
            SELECT_TASK_FULL,
            "SELECT id, parent_id, name, spec, status, complexity, priority, first_todo_at, first_doing_at, first_done_at, active_form FROM tasks"
        );
    }

    #[test]
    fn test_select_event_by_id() {
        assert_eq!(
            SELECT_EVENT_BY_ID,
            "SELECT id, task_id, timestamp, log_type, discussion_data FROM events WHERE id = ?"
        );
    }

    #[test]
    fn test_check_task_exists() {
        assert_eq!(
            CHECK_TASK_EXISTS,
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?)"
        );
    }
}
