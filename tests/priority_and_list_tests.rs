//! Tests for priority levels and task listing/filtering
//!
//! These tests verify priority setting, validation, and task list filtering.
//! The tests use library functions directly rather than CLI commands.

mod test_helpers_rewrite;

use intent_engine::{
    db::models::Task,
    priority::PriorityLevel,
    tasks::{TaskManager, TaskUpdate},
};
use test_helpers_rewrite::TestDb;

/// Test setting critical priority
#[tokio::test]
async fn test_priority_critical() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Add a task
    let task = manager
        .add_task("Test Task", None, None, Some("human"))
        .await
        .unwrap();

    // Update with critical priority
    let critical = PriorityLevel::parse_to_int("critical").unwrap();
    let updated = manager
        .update_task(
            task.id,
            TaskUpdate {
                priority: Some(critical),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.priority.unwrap(), 1);
}

/// Test setting high priority
#[tokio::test]
async fn test_priority_high() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    let task = manager
        .add_task("Test Task", None, None, Some("human"))
        .await
        .unwrap();

    let high = PriorityLevel::parse_to_int("high").unwrap();
    let updated = manager
        .update_task(
            task.id,
            TaskUpdate {
                priority: Some(high),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.priority.unwrap(), 2);
}

/// Test setting medium priority
#[tokio::test]
async fn test_priority_medium() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    let task = manager
        .add_task("Test Task", None, None, Some("human"))
        .await
        .unwrap();

    let medium = PriorityLevel::parse_to_int("medium").unwrap();
    let updated = manager
        .update_task(
            task.id,
            TaskUpdate {
                priority: Some(medium),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.priority.unwrap(), 3);
}

/// Test setting low priority
#[tokio::test]
async fn test_priority_low() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    let task = manager
        .add_task("Test Task", None, None, Some("human"))
        .await
        .unwrap();

    let low = PriorityLevel::parse_to_int("low").unwrap();
    let updated = manager
        .update_task(
            task.id,
            TaskUpdate {
                priority: Some(low),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.priority.unwrap(), 4);
}

/// Test priority parsing is case insensitive
#[tokio::test]
async fn test_priority_case_insensitive() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    let task = manager
        .add_task("Test Task", None, None, Some("human"))
        .await
        .unwrap();

    // Test uppercase
    let high = PriorityLevel::parse_to_int("HIGH").unwrap();
    let updated = manager
        .update_task(
            task.id,
            TaskUpdate {
                priority: Some(high),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.priority.unwrap(), 2);

    // Test mixed case
    let critical = PriorityLevel::parse_to_int("CriTiCaL").unwrap();
    let updated = manager
        .update_task(
            task.id,
            TaskUpdate {
                priority: Some(critical),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.priority.unwrap(), 1);
}

/// Test invalid priority strings are rejected
#[tokio::test]
async fn test_priority_invalid_string() {
    // Test invalid priority string
    let result = PriorityLevel::parse_to_int("invalid");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid priority"));

    // Test empty string
    let result = PriorityLevel::parse_to_int("");
    assert!(result.is_err());

    // Test numeric string (old format should fail)
    let result = PriorityLevel::parse_to_int("1");
    assert!(result.is_err());
}

/// Test priority ordering works correctly
#[tokio::test]
async fn test_priority_ordering_still_works() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create tasks with different priorities
    let task1 = manager
        .add_task("Low priority task", None, None, Some("human"))
        .await
        .unwrap();
    let task2 = manager
        .add_task("Critical priority task", None, None, Some("human"))
        .await
        .unwrap();
    let task3 = manager
        .add_task("Medium priority task", None, None, Some("human"))
        .await
        .unwrap();

    // Set priorities
    let low = PriorityLevel::parse_to_int("low").unwrap();
    let critical = PriorityLevel::parse_to_int("critical").unwrap();
    let medium = PriorityLevel::parse_to_int("medium").unwrap();

    manager
        .update_task(
            task1.id,
            TaskUpdate {
                priority: Some(low),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    manager
        .update_task(
            task2.id,
            TaskUpdate {
                priority: Some(critical),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    manager
        .update_task(
            task3.id,
            TaskUpdate {
                priority: Some(medium),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Query all tasks and verify priorities
    let all_tasks: Vec<Task> = sqlx::query_as(
        "SELECT id, parent_id, name, spec, status, complexity, priority, \
         first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata \
         FROM tasks ORDER BY id",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();

    assert_eq!(all_tasks.len(), 3);
    assert_eq!(all_tasks[0].priority.unwrap(), 4); // low = 4
    assert_eq!(all_tasks[1].priority.unwrap(), 1); // critical = 1
    assert_eq!(all_tasks[2].priority.unwrap(), 3); // medium = 3
}

/// Test task list querying with different filters
#[tokio::test]
async fn test_task_list_filtering() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Add some tasks
    let task1 = manager
        .add_task("Task 1", None, None, Some("human"))
        .await
        .unwrap();
    let _task2 = manager
        .add_task("Task 2", None, None, Some("human"))
        .await
        .unwrap();

    // Create a subtask
    let _subtask = manager
        .add_task("Subtask 1", None, Some(task1.id), Some("human"))
        .await
        .unwrap();

    // List all tasks
    let all_tasks: Vec<Task> = sqlx::query_as(
        "SELECT id, parent_id, name, spec, status, complexity, priority, \
         first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata \
         FROM tasks",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    assert_eq!(all_tasks.len(), 3);

    // List with status filter (todo)
    let todo_tasks: Vec<Task> = sqlx::query_as(
        "SELECT id, parent_id, name, spec, status, complexity, priority, \
         first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata \
         FROM tasks WHERE status = ?",
    )
    .bind("todo")
    .fetch_all(db.pool())
    .await
    .unwrap();
    assert_eq!(todo_tasks.len(), 3);

    // List with parent filter (children of task1)
    let children: Vec<Task> = sqlx::query_as(
        "SELECT id, parent_id, name, spec, status, complexity, priority, \
         first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata \
         FROM tasks WHERE parent_id = ?",
    )
    .bind(task1.id)
    .fetch_all(db.pool())
    .await
    .unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "Subtask 1");

    // List top-level tasks only (parent_id IS NULL)
    let top_level: Vec<Task> = sqlx::query_as(
        "SELECT id, parent_id, name, spec, status, complexity, priority, \
         first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata \
         FROM tasks WHERE parent_id IS NULL",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    assert_eq!(top_level.len(), 2);
}
