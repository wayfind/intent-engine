//! Tests for task start blocking validation with dependencies
//!
//! These tests verify that task_start correctly validates and blocks tasks
//! that have incomplete dependencies. The tests use library functions directly
//! rather than CLI commands.

mod test_helpers_rewrite;

use intent_engine::{dependencies, tasks::TaskManager};
use test_helpers_rewrite::TestDb;

/// Test that starting a task blocked by incomplete dependency fails
#[tokio::test]
async fn test_start_task_blocked_by_incomplete_dependency() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create two tasks
    let task1 = manager
        .add_task("Task A", None, None, Some("human"), None, None)
        .await
        .unwrap();
    let task2 = manager
        .add_task("Task B", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Make Task 2 depend on Task 1
    dependencies::add_dependency(db.pool(), task1.id, task2.id)
        .await
        .unwrap();

    // Try to start Task 2 (should fail because Task 1 is not done)
    let result = manager.start_task(task2.id, false).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("TASK_BLOCKED")
            || error.to_string().contains("blocked")
            || error.to_string().contains("dependencies")
    );
}

/// Test that starting a task is allowed after dependency is completed
#[tokio::test]
async fn test_start_task_allowed_after_dependency_completed() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create two tasks
    let task1 = manager
        .add_task("Task A", None, None, Some("human"), None, None)
        .await
        .unwrap();
    let task2 = manager
        .add_task("Task B", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Make Task 2 depend on Task 1
    dependencies::add_dependency(db.pool(), task1.id, task2.id)
        .await
        .unwrap();

    // Start and complete Task 1
    manager.start_task(task1.id, false).await.unwrap();
    manager.done_task(false).await.unwrap();

    // Now Task 2 should be allowed to start
    let result = manager.start_task(task2.id, false).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.task.status, "doing");
}

/// Test that starting a task blocked by multiple dependencies fails
#[tokio::test]
async fn test_start_task_blocked_by_multiple_dependencies() {
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

    // Task 3 depends on both Task 1 and Task 2
    dependencies::add_dependency(db.pool(), task1.id, task3.id)
        .await
        .unwrap();
    dependencies::add_dependency(db.pool(), task2.id, task3.id)
        .await
        .unwrap();

    // Try to start Task 3 (should fail because both Task 1 and Task 2 are not done)
    let result = manager.start_task(task3.id, false).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("TASK_BLOCKED")
            || error.to_string().contains("blocked")
            || error.to_string().contains("dependencies")
    );
}

/// Test that starting a task with partial dependencies completed still fails
#[tokio::test]
async fn test_start_task_with_partial_dependencies_completed() {
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

    // Task 3 depends on both Task 1 and Task 2
    dependencies::add_dependency(db.pool(), task1.id, task3.id)
        .await
        .unwrap();
    dependencies::add_dependency(db.pool(), task2.id, task3.id)
        .await
        .unwrap();

    // Complete Task 1 only
    manager.start_task(task1.id, false).await.unwrap();
    manager.done_task(false).await.unwrap();

    // Try to start Task 3 (should still fail because Task 2 is not done)
    let result = manager.start_task(task3.id, false).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("TASK_BLOCKED")
            || error.to_string().contains("blocked")
            || error.to_string().contains("dependencies")
    );
}

/// Test that starting a task with no dependencies is allowed (baseline)
#[tokio::test]
async fn test_start_task_no_dependencies_allowed() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create a task with no dependencies
    let task = manager
        .add_task("Independent Task", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Should be able to start immediately
    let result = manager.start_task(task.id, false).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.task.status, "doing");
}

/// Test that starting a task blocked by a dependency in "doing" status fails
#[tokio::test]
async fn test_start_task_blocked_by_doing_dependency() {
    let db = TestDb::new().await;
    let manager = TaskManager::new(db.pool());

    // Create two tasks
    let task1 = manager
        .add_task("Task A", None, None, Some("human"), None, None)
        .await
        .unwrap();
    let task2 = manager
        .add_task("Task B", None, None, Some("human"), None, None)
        .await
        .unwrap();

    // Make Task 2 depend on Task 1
    dependencies::add_dependency(db.pool(), task1.id, task2.id)
        .await
        .unwrap();

    // Start Task 1 (but don't complete it - status is "doing")
    manager.start_task(task1.id, false).await.unwrap();

    // Try to start Task 2 (should fail because Task 1 is doing, not done)
    let result = manager.start_task(task2.id, false).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("TASK_BLOCKED")
            || error.to_string().contains("blocked")
            || error.to_string().contains("dependencies")
    );
}
