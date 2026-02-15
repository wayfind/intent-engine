//! Tests for task edge cases and error handling
//!
//! These tests verify error handling and boundary conditions for task operations.
//! The tests use library functions directly rather than CLI commands.

mod test_helpers_rewrite;

use intent_engine::{
    dependencies,
    priority::PriorityLevel,
    tasks::{TaskManager, TaskUpdate},
    workspace::WorkspaceManager,
};
use test_helpers_rewrite::TestDb;

// ============================================================================
// Invalid Task ID Tests
// ============================================================================

/// Test getting a non-existent task returns error
#[tokio::test]
async fn test_task_get_nonexistent_id() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Try to get task with ID that doesn't exist
    let result = manager.get_task(99999).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Task not found"));
}

/// Test updating a non-existent task returns error
#[tokio::test]
async fn test_task_update_nonexistent_id() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Try to update task with ID that doesn't exist
    let result = manager
        .update_task(
            99999,
            TaskUpdate {
                name: Some("New Name"),
                ..Default::default()
            },
        )
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Task not found"));
}

/// Test deleting a non-existent task returns error
#[tokio::test]
async fn test_task_delete_nonexistent_id() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Try to delete task with ID that doesn't exist
    let result = manager.delete_task(99999).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Task not found"));
}

/// Test starting a non-existent task returns error
#[tokio::test]
async fn test_task_start_nonexistent_id() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Try to start task with ID that doesn't exist
    let result = manager.start_task(99999, false).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Task not found"));
}

// ============================================================================
// Dependency Tests
// ============================================================================

/// Test that creating a self-dependency is rejected
#[tokio::test]
async fn test_task_depends_on_self() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create a task
    let task = manager
        .add_task("Task 1", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Try to create self-dependency
    let result = dependencies::add_dependency(db.pool(), task.id, task.id).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Circular dependency detected"));
}

/// Test that circular dependencies are detected and rejected
#[tokio::test]
async fn test_task_circular_dependency_detection() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create three tasks
    let task1 = manager
        .add_task("Task 1", None, None, Some("human"), None, None)
        .await
        .unwrap();
    let task2 = manager
        .add_task("Task 2", None, None, Some("human"), None, None)
        .await
        .unwrap();
    let task3 = manager
        .add_task("Task 3", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Create dependencies: 1 -> 2 -> 3
    dependencies::add_dependency(db.pool(), task1.id, task2.id)
        .await
        .unwrap();
    dependencies::add_dependency(db.pool(), task2.id, task3.id)
        .await
        .unwrap();

    // Try to create circular dependency: 3 -> 1
    let result = dependencies::add_dependency(db.pool(), task3.id, task1.id).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Circular dependency detected"));
}

/// Test that creating dependency with non-existent task is rejected
#[tokio::test]
async fn test_task_depends_on_nonexistent_task() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create one task
    let task = manager
        .add_task("Task 1", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Try to create dependency with non-existent task
    let result = dependencies::add_dependency(db.pool(), task.id, 99999).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Task not found"));
}

// ============================================================================
// Priority Edge Cases
// ============================================================================

/// Test that invalid priority strings are rejected
#[tokio::test]
async fn test_task_update_invalid_priority() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create a task
    let task = manager
        .add_task("Test Task", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Try to parse invalid priority (this should fail at parse stage)
    let result = PriorityLevel::parse_to_int("invalid");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid priority"));

    // Verify task still exists and wasn't modified
    let retrieved = manager.get_task(task.id).await.unwrap();
    assert_eq!(retrieved.name, "Test Task");
}

// ============================================================================
// Pick Next Edge Cases
// ============================================================================

/// Test pick_next with multiple available tasks
#[tokio::test]
async fn test_pick_next_with_multiple_tasks() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create multiple tasks
    let task1 = manager
        .add_task("Task 1", None, None, Some("human"), None, None)
        .await
        .unwrap();
    manager
        .add_task("Task 2", None, None, Some("human"), None, None)
        .await
        .unwrap();
    manager
        .add_task("Task 3", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Pick next should recommend task1 (first created)
    let result = manager.pick_next().await.unwrap();

    assert!(result.task.is_some());
    assert_eq!(result.task.unwrap().id, task1.id);
}

/// Test pick_next when all tasks are completed
#[tokio::test]
async fn test_pick_next_with_all_tasks_completed() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create a task
    let task = manager
        .add_task("Task 1", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Start and complete the task
    manager.start_task(task.id, false).await.unwrap();
    manager.done_task(false).await.unwrap();

    // Pick next should return None (no tasks available)
    let result = manager.pick_next().await.unwrap();

    assert!(result.task.is_none());
    assert_eq!(result.suggestion_type, "NONE");
}

// ============================================================================
// Task State Transition Tests
// ============================================================================

/// Test that completing parent task with incomplete children fails
#[tokio::test]
async fn test_task_done_with_uncompleted_children() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create parent task
    let parent = manager
        .add_task("Parent Task", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Create child task
    manager
        .add_task(
            "Child Task",
            None,
            Some(parent.id),
            Some("human"),
            None,
            None,
        )
        .await
        .unwrap();

    // Start parent task
    manager.start_task(parent.id, false).await.unwrap();

    // Try to complete parent without completing child
    let result = manager.done_task(false).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("has uncompleted children")
            || error
                .to_string()
                .contains("Cannot complete task with incomplete subtasks")
            || error.to_string().contains("Uncompleted children")
    );
}

/// Test that calling done_task without a focused task fails
#[tokio::test]
async fn test_task_done_without_current_task() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());
    let workspace = WorkspaceManager::new(db.pool());

    // Ensure no task is focused
    let current = workspace.get_current_task(None).await.unwrap();
    assert!(current.current_task_id.is_none());

    // Try to complete without a focused task
    let result = manager.done_task(false).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("No task is currently focused")
            || error.to_string().contains("No current task")
    );
}
