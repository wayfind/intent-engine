//! Tests for dependency blocking logic in pick_next algorithm
//!
//! These tests verify that pick_next correctly handles task dependencies
//! and blocking relationships. The tests use library functions directly
//! rather than CLI commands to test the core business logic.

mod test_helpers_rewrite;

use intent_engine::{
    dependencies,
    priority::PriorityLevel,
    tasks::{TaskManager, TaskUpdate},
    workspace::WorkspaceManager,
};
use test_helpers_rewrite::TestDb;

/// Test that pick_next skips tasks that are blocked by incomplete dependencies
#[tokio::test]
async fn test_pick_next_skips_blocked_task() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create two todo tasks
    let task1 = manager
        .add_task("Task 1 - Blocking", None, None, Some("human"), None, None)
        .await
        .unwrap();
    let task2 = manager
        .add_task("Task 2 - Blocked", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Make task2 depend on task1 (task2 is blocked by task1)
    dependencies::add_dependency(db.pool(), task1.id, task2.id)
        .await
        .unwrap();

    // Pick next should recommend task1 (not blocked)
    let result = manager.pick_next().await.unwrap();

    assert_eq!(result.task.unwrap().id, task1.id);
}

/// Test that pick_next recommends blocked task after blocking task is completed
#[tokio::test]
async fn test_pick_next_recommends_after_blocking_complete() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create two todo tasks
    let task1 = manager
        .add_task("Task 1 - Blocking", None, None, Some("human"), None, None)
        .await
        .unwrap();
    let task2 = manager
        .add_task("Task 2 - Blocked", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Make task2 depend on task1
    dependencies::add_dependency(db.pool(), task1.id, task2.id)
        .await
        .unwrap();

    // Complete task1 (status = "done")
    manager
        .update_task(
            task1.id,
            TaskUpdate {
                status: Some("done"),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Pick next should now recommend task2
    let result = manager.pick_next().await.unwrap();
    assert_eq!(result.task.unwrap().id, task2.id);
}

/// Test pick_next with multiple dependencies
#[tokio::test]
async fn test_pick_next_multiple_dependencies() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create three tasks
    let task1 = manager
        .add_task("Task 1 - Blocker A", None, None, Some("human"), None, None)
        .await
        .unwrap();
    let task2 = manager
        .add_task("Task 2 - Blocker B", None, None, Some("human"), None, None)
        .await
        .unwrap();
    let task3 = manager
        .add_task(
            "Task 3 - Blocked by both",
            None,
            None,
            Some("human"),
            None,
            None,
        )
        .await
        .unwrap();

    // Make task3 depend on both task1 and task2
    dependencies::add_dependency(db.pool(), task1.id, task3.id)
        .await
        .unwrap();
    dependencies::add_dependency(db.pool(), task2.id, task3.id)
        .await
        .unwrap();

    // Complete only task1
    manager
        .update_task(
            task1.id,
            TaskUpdate {
                status: Some("done"),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Pick next should recommend task2, not task3 (still blocked by task2)
    let result = manager.pick_next().await.unwrap();
    assert_eq!(result.task.unwrap().id, task2.id);

    // Now complete task2
    manager
        .update_task(
            task2.id,
            TaskUpdate {
                status: Some("done"),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Pick next should now recommend task3
    let result = manager.pick_next().await.unwrap();
    assert_eq!(result.task.unwrap().id, task3.id);
}

/// Test pick_next with blocked subtask
#[tokio::test]
async fn test_pick_next_blocked_subtask() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());
    let workspace = WorkspaceManager::new(db.pool());

    // Create parent task and two subtasks
    let parent = manager
        .add_task("Parent Task", None, None, Some("human"), None, None)
        .await
        .unwrap();
    let subtask1 = manager
        .add_task(
            "Subtask 1 - Blocker",
            None,
            Some(parent.id),
            Some("human"),
            None,
            None,
        )
        .await
        .unwrap();
    let subtask2 = manager
        .add_task(
            "Subtask 2 - Blocked",
            None,
            Some(parent.id),
            Some("human"),
            None,
            None,
        )
        .await
        .unwrap();

    // Make subtask2 depend on subtask1
    dependencies::add_dependency(db.pool(), subtask1.id, subtask2.id)
        .await
        .unwrap();

    // Set parent as current task
    workspace.set_current_task(parent.id, None).await.unwrap();

    // Pick next should recommend subtask1 (depth-first priority)
    let result = manager.pick_next().await.unwrap();

    assert_eq!(result.task.unwrap().id, subtask1.id);
    assert_eq!(result.suggestion_type, "FOCUSED_SUB_TASK");
}

/// Test pick_next when only doing tasks exist (no todo tasks available)
#[tokio::test]
async fn test_pick_next_no_available_tasks_due_to_blocking() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create two tasks
    let task1 = manager
        .add_task("Task 1 - Blocked", None, None, Some("human"), None, None)
        .await
        .unwrap();
    let task2 = manager
        .add_task("Task 2 - Blocking", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Make task1 depend on task2
    dependencies::add_dependency(db.pool(), task2.id, task1.id)
        .await
        .unwrap();

    // Set task2 to doing (not done, so still blocking)
    manager
        .update_task(
            task2.id,
            TaskUpdate {
                status: Some("doing"),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Pick next should recommend task2 (it's doing and available)
    let result = manager.pick_next().await.unwrap();
    let task = result.task.unwrap();

    assert_eq!(task.id, task2.id);
    assert_eq!(task.status, "doing");
    assert_eq!(result.suggestion_type, "TOP_LEVEL_TASK");
}

/// Test that pick_next respects priority even with blocking relationships
#[tokio::test]
async fn test_pick_next_respects_priority_with_blocking() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create three tasks with different priorities
    let task1 = manager
        .add_task(
            "Task 1 - Low Priority",
            None,
            None,
            Some("human"),
            None,
            None,
        )
        .await
        .unwrap();
    let task2 = manager
        .add_task(
            "Task 2 - High Priority",
            None,
            None,
            Some("human"),
            None,
            None,
        )
        .await
        .unwrap();
    let task3 = manager
        .add_task(
            "Task 3 - Medium Priority",
            None,
            None,
            Some("human"),
            None,
            None,
        )
        .await
        .unwrap();

    // Set priorities using PriorityLevel
    let low_priority = PriorityLevel::parse_to_int("low").unwrap();
    let critical_priority = PriorityLevel::parse_to_int("critical").unwrap();
    let medium_priority = PriorityLevel::parse_to_int("medium").unwrap();

    manager
        .update_task(
            task1.id,
            TaskUpdate {
                priority: Some(low_priority),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    manager
        .update_task(
            task2.id,
            TaskUpdate {
                priority: Some(critical_priority),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    manager
        .update_task(
            task3.id,
            TaskUpdate {
                priority: Some(medium_priority),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Make task2 (high priority) depend on task1
    dependencies::add_dependency(db.pool(), task1.id, task2.id)
        .await
        .unwrap();

    // Pick next should recommend task3 (medium priority, not blocked)
    // because task2 (higher priority) is blocked by task1
    let result = manager.pick_next().await.unwrap();

    assert_eq!(result.task.unwrap().id, task3.id);
}

/// Test normal pick_next behavior without blocking (baseline test)
#[tokio::test]
async fn test_pick_next_unblocked_task_normal_behavior() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create tasks without any dependencies
    let task1 = manager
        .add_task(
            "Task 1 - No Dependencies",
            None,
            None,
            Some("human"),
            None,
            None,
        )
        .await
        .unwrap();
    let task2 = manager
        .add_task(
            "Task 2 - No Dependencies",
            None,
            None,
            Some("human"),
            None,
            None,
        )
        .await
        .unwrap();

    // Pick next should work normally, recommending task1 (first created)
    let result = manager.pick_next().await.unwrap();
    assert_eq!(result.task.unwrap().id, task1.id);

    // Complete task1
    manager
        .update_task(
            task1.id,
            TaskUpdate {
                status: Some("done"),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Pick next should now recommend task2
    let result = manager.pick_next().await.unwrap();
    assert_eq!(result.task.unwrap().id, task2.id);
}
