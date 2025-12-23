// Focus Switching Tests: Testing automatic focus switching and paused state
//
// This test suite verifies that:
// 1. task_start automatically switches focus (current_task_id)
// 2. Old focused task becomes "paused" (doing + not focused)
// 3. Paused is a derived state, not stored
// 4. Edge cases are handled correctly

use intent_engine::db::{create_pool, run_migrations};
use intent_engine::tasks::TaskManager;
use sqlx::SqlitePool;
use tempfile::TempDir;

async fn setup_test_db() -> (TempDir, SqlitePool) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("focus_test.db");
    let pool = create_pool(&db_path)
        .await
        .expect("Failed to create test database");

    run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    (temp_dir, pool)
}

async fn get_current_task_id(pool: &SqlitePool) -> Option<i64> {
    sqlx::query_scalar::<_, String>(
        "SELECT value FROM workspace_state WHERE key = 'current_task_id'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|s| s.parse::<i64>().ok())
}

#[tokio::test]
async fn test_focus_switch_from_none() {
    // Test starting a task when no task is currently focused
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create a task
    mgr.add_task("Task A", None, None, None).await.unwrap();

    // Verify no current focus
    assert_eq!(get_current_task_id(&pool).await, None);

    // Start the task
    mgr.start_task(1, false).await.unwrap();

    // Verify focus switched
    assert_eq!(get_current_task_id(&pool).await, Some(1));

    let task = mgr.get_task(1).await.unwrap();
    assert_eq!(task.status, "doing");
}

#[tokio::test]
async fn test_focus_switch_between_tasks() {
    // Test switching focus from one task to another
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create two tasks
    mgr.add_task("Task A", None, None, None).await.unwrap();
    mgr.add_task("Task B", None, None, None).await.unwrap();

    // Start Task A
    mgr.start_task(1, false).await.unwrap();
    assert_eq!(get_current_task_id(&pool).await, Some(1));

    let task_a_doing = mgr.get_task(1).await.unwrap();
    assert_eq!(task_a_doing.status, "doing");

    // Start Task B - this should switch focus
    mgr.start_task(2, false).await.unwrap();
    assert_eq!(
        get_current_task_id(&pool).await,
        Some(2),
        "Focus should switch to Task B"
    );

    // Verify Task A is still 'doing' (paused state is derived)
    let task_a_paused = mgr.get_task(1).await.unwrap();
    assert_eq!(
        task_a_paused.status, "doing",
        "Task A should remain 'doing' (becomes paused implicitly)"
    );

    // Verify Task B is 'doing' and focused
    let task_b_focused = mgr.get_task(2).await.unwrap();
    assert_eq!(task_b_focused.status, "doing");
}

#[tokio::test]
async fn test_paused_state_is_derived() {
    // Test that paused state is derived from (doing + not focused)
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create 3 tasks
    mgr.add_task("Task A", None, None, None).await.unwrap();
    mgr.add_task("Task B", None, None, None).await.unwrap();
    mgr.add_task("Task C", None, None, None).await.unwrap();

    // Start all three in sequence
    mgr.start_task(1, false).await.unwrap();
    mgr.start_task(2, false).await.unwrap();
    mgr.start_task(3, false).await.unwrap();

    // Current focus should be Task C
    assert_eq!(get_current_task_id(&pool).await, Some(3));

    // All three should be 'doing'
    let task_a = mgr.get_task(1).await.unwrap();
    let task_b = mgr.get_task(2).await.unwrap();
    let task_c = mgr.get_task(3).await.unwrap();

    assert_eq!(
        task_a.status, "doing",
        "Task A is paused (doing + unfocused)"
    );
    assert_eq!(
        task_b.status, "doing",
        "Task B is paused (doing + unfocused)"
    );
    assert_eq!(
        task_c.status, "doing",
        "Task C is focused (doing + focused)"
    );

    // Paused state derivation:
    // Task A: doing=true, focused=false → paused
    // Task B: doing=true, focused=false → paused
    // Task C: doing=true, focused=true → focused
}

#[tokio::test]
async fn test_resume_paused_task() {
    // Test resuming a previously paused task
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create two tasks
    mgr.add_task("Task A", None, None, None).await.unwrap();
    mgr.add_task("Task B", None, None, None).await.unwrap();

    // Start A, then B (A becomes paused)
    mgr.start_task(1, false).await.unwrap();
    mgr.start_task(2, false).await.unwrap();

    assert_eq!(get_current_task_id(&pool).await, Some(2));

    // Resume Task A
    mgr.start_task(1, false).await.unwrap();

    // Focus should be back on A
    assert_eq!(get_current_task_id(&pool).await, Some(1));

    // Both should still be 'doing'
    let task_a = mgr.get_task(1).await.unwrap();
    let task_b = mgr.get_task(2).await.unwrap();

    assert_eq!(task_a.status, "doing", "Task A is focused again");
    assert_eq!(task_b.status, "doing", "Task B is now paused");
}

#[tokio::test]
async fn test_start_already_focused_task() {
    // Test calling start on a task that is already focused
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    mgr.add_task("Task A", None, None, None).await.unwrap();

    // Start task
    mgr.start_task(1, false).await.unwrap();
    assert_eq!(get_current_task_id(&pool).await, Some(1));

    // Start again (should be idempotent)
    let result = mgr.start_task(1, false).await;
    assert!(
        result.is_ok(),
        "Starting an already focused task should succeed"
    );

    // Still focused
    assert_eq!(get_current_task_id(&pool).await, Some(1));

    let task = mgr.get_task(1).await.unwrap();
    assert_eq!(task.status, "doing");
}

#[tokio::test]
async fn test_focus_with_parent_child_cascade() {
    // Test that focus switching works correctly with cascading
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create parent-child hierarchy
    mgr.add_task("Parent", None, None, None).await.unwrap();
    mgr.add_task("Child A", None, Some(1), None).await.unwrap();
    mgr.add_task("Child B", None, Some(1), None).await.unwrap();

    // Start Child A (parent won't cascade - feature not implemented)
    mgr.start_task(2, false).await.unwrap();
    assert_eq!(get_current_task_id(&pool).await, Some(2));

    let parent = mgr.get_task(1).await.unwrap();
    let child_a = mgr.get_task(2).await.unwrap();

    assert_eq!(
        parent.status, "todo",
        "Parent remains todo (cascade not implemented)"
    );
    assert_eq!(child_a.status, "doing", "Child A is focused");

    // Switch focus to Child B
    mgr.start_task(3, false).await.unwrap();
    assert_eq!(get_current_task_id(&pool).await, Some(3));

    let parent_after = mgr.get_task(1).await.unwrap();
    let child_a_after = mgr.get_task(2).await.unwrap();
    let child_b_after = mgr.get_task(3).await.unwrap();

    assert_eq!(parent_after.status, "todo", "Parent remains todo");
    assert_eq!(child_a_after.status, "doing", "Child A is now paused");
    assert_eq!(child_b_after.status, "doing", "Child B is now focused");
}

#[tokio::test]
async fn test_multiple_paused_tasks() {
    // Test multiple tasks can be paused simultaneously
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create 5 tasks
    for i in 1..=5 {
        mgr.add_task(&format!("Task {}", i), None, None, None)
            .await
            .unwrap();
    }

    // Start all of them in sequence
    for i in 1..=5 {
        mgr.start_task(i, false).await.unwrap();
    }

    // Only task 5 should be focused
    assert_eq!(get_current_task_id(&pool).await, Some(5));

    // All should be 'doing' (tasks 1-4 are paused)
    for i in 1..=5 {
        let task = mgr.get_task(i).await.unwrap();
        assert_eq!(task.status, "doing");
    }

    // Verify we can query paused tasks:
    // (doing tasks that are not focused)
    let all_doing: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM tasks WHERE status = 'doing' ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();

    assert_eq!(all_doing, vec![1, 2, 3, 4, 5]);

    // Paused tasks = all_doing - current_task_id
    let current = get_current_task_id(&pool).await.unwrap();
    let paused: Vec<i64> = all_doing.into_iter().filter(|&id| id != current).collect();

    assert_eq!(paused, vec![1, 2, 3, 4], "Tasks 1-4 are paused");
}

#[tokio::test]
async fn test_done_removes_from_paused_pool() {
    // Test that completing a task removes it from the paused pool
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create 3 tasks
    mgr.add_task("Task A", None, None, None).await.unwrap();
    mgr.add_task("Task B", None, None, None).await.unwrap();
    mgr.add_task("Task C", None, None, None).await.unwrap();

    // Start all three
    mgr.start_task(1, false).await.unwrap();
    mgr.start_task(2, false).await.unwrap();
    mgr.start_task(3, false).await.unwrap();

    // Tasks 1 and 2 are paused, Task 3 is focused
    assert_eq!(get_current_task_id(&pool).await, Some(3));

    // Complete Task 3
    mgr.done_task(false).await.unwrap();

    let task_c = mgr.get_task(3).await.unwrap();
    assert_eq!(task_c.status, "done");

    // No current focus now
    assert_eq!(get_current_task_id(&pool).await, None);

    // Tasks 1 and 2 are still paused (doing but not focused)
    let task_a = mgr.get_task(1).await.unwrap();
    let task_b = mgr.get_task(2).await.unwrap();

    assert_eq!(task_a.status, "doing");
    assert_eq!(task_b.status, "doing");

    // We can resume either task
    mgr.start_task(1, false).await.unwrap();
    assert_eq!(get_current_task_id(&pool).await, Some(1));
}
