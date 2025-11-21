// Cascade Tests: Testing Doing and Done status cascading behavior
//
// This test suite verifies that:
// 1. When a child task starts (doing), all ancestors automatically become 'doing'
// 2. When all children of a parent complete, the parent automatically becomes 'done'
// 3. Cascading works recursively up the task hierarchy

use intent_engine::db::{create_pool, run_migrations};
use intent_engine::tasks::TaskManager;
use sqlx::SqlitePool;
use tempfile::TempDir;

async fn setup_test_db() -> (TempDir, SqlitePool) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("cascade_test.db");
    let pool = create_pool(&db_path)
        .await
        .expect("Failed to create test database");

    // Run migrations
    run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    (temp_dir, pool)
}

#[tokio::test]
async fn test_doing_cascade_single_level() {
    // Test that starting a child task makes its parent 'doing'
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create parent task (id=1, status=todo)
    mgr.add_task("Parent Task", Some("Parent task spec"), None)
        .await
        .unwrap();

    // Create child task (id=2, status=todo)
    mgr.add_task("Child Task", Some("Child task spec"), Some(1))
        .await
        .unwrap();

    // Verify initial states
    let parent = mgr.get_task(1).await.unwrap();
    assert_eq!(parent.status, "todo", "Parent should start as 'todo'");

    let child = mgr.get_task(2).await.unwrap();
    assert_eq!(child.status, "todo", "Child should start as 'todo'");

    // Start child task - this should cascade parent to 'doing'
    mgr.start_task(2, false).await.unwrap();

    // Verify cascade: both child and parent should now be 'doing'
    let child_after = mgr.get_task(2).await.unwrap();
    assert_eq!(
        child_after.status, "doing",
        "Child should be 'doing' after start"
    );

    let parent_after = mgr.get_task(1).await.unwrap();
    assert_eq!(
        parent_after.status, "doing",
        "Parent should cascade to 'doing' when child starts"
    );
}

#[tokio::test]
async fn test_doing_cascade_multi_level() {
    // Test cascading across 3 levels: grandparent -> parent -> child
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create task hierarchy
    mgr.add_task("Grandparent", None, None).await.unwrap();
    mgr.add_task("Parent", None, Some(1)).await.unwrap();
    mgr.add_task("Child", None, Some(2)).await.unwrap();

    // Start the deepest child (id=3)
    mgr.start_task(3, false).await.unwrap();

    // Verify all ancestors cascaded to 'doing'
    let child = mgr.get_task(3).await.unwrap();
    assert_eq!(child.status, "doing", "Child should be 'doing'");

    let parent = mgr.get_task(2).await.unwrap();
    assert_eq!(parent.status, "doing", "Parent should cascade to 'doing'");

    let grandparent = mgr.get_task(1).await.unwrap();
    assert_eq!(
        grandparent.status, "doing",
        "Grandparent should cascade to 'doing'"
    );
}

#[tokio::test]
async fn test_doing_cascade_skip_already_doing() {
    // Test that cascading skips ancestors already in 'doing' state
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create hierarchy
    mgr.add_task("Parent", None, None).await.unwrap();
    mgr.add_task("Child1", None, Some(1)).await.unwrap();
    mgr.add_task("Child2", None, Some(1)).await.unwrap();

    // Start first child (parent becomes 'doing')
    mgr.start_task(2, false).await.unwrap();

    let parent_first = mgr.get_task(1).await.unwrap();
    assert_eq!(parent_first.status, "doing");

    // Complete first child
    mgr.done_task().await.unwrap();

    // Start second child - parent should already be 'doing', no issue
    mgr.start_task(3, false).await.unwrap();

    let parent_second = mgr.get_task(1).await.unwrap();
    assert_eq!(
        parent_second.status, "doing",
        "Parent should remain 'doing'"
    );
}

#[tokio::test]
async fn test_done_cascade_single_level() {
    // Test that completing all children auto-completes parent
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create parent with one child
    mgr.add_task("Parent", None, None).await.unwrap();
    mgr.add_task("Child", None, Some(1)).await.unwrap();

    // Start and complete child
    mgr.start_task(2, false).await.unwrap();
    mgr.done_task().await.unwrap();

    // Verify cascade: parent should auto-complete
    let child = mgr.get_task(2).await.unwrap();
    assert_eq!(child.status, "done", "Child should be 'done'");

    let parent = mgr.get_task(1).await.unwrap();
    assert_eq!(
        parent.status, "done",
        "Parent should cascade to 'done' when all children complete"
    );
}

#[tokio::test]
async fn test_done_cascade_multi_level() {
    // Test done cascading across 3 levels
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create 3-level hierarchy
    mgr.add_task("Grandparent", None, None).await.unwrap();
    mgr.add_task("Parent", None, Some(1)).await.unwrap();
    mgr.add_task("Child", None, Some(2)).await.unwrap();

    // Complete the child
    mgr.start_task(3, false).await.unwrap();
    mgr.done_task().await.unwrap();

    // All should cascade to 'done'
    let child = mgr.get_task(3).await.unwrap();
    assert_eq!(child.status, "done");

    let parent = mgr.get_task(2).await.unwrap();
    assert_eq!(parent.status, "done", "Parent should cascade to 'done'");

    let grandparent = mgr.get_task(1).await.unwrap();
    assert_eq!(
        grandparent.status, "done",
        "Grandparent should cascade to 'done'"
    );
}

#[tokio::test]
async fn test_done_cascade_stops_with_incomplete_siblings() {
    // Test that parent doesn't complete if siblings remain incomplete
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create parent with 2 children
    mgr.add_task("Parent", None, None).await.unwrap();
    mgr.add_task("Child1", None, Some(1)).await.unwrap();
    mgr.add_task("Child2", None, Some(1)).await.unwrap();

    // Complete only first child
    mgr.start_task(2, false).await.unwrap();
    mgr.done_task().await.unwrap();

    // Parent should NOT cascade to 'done' (child2 still todo)
    let child1 = mgr.get_task(2).await.unwrap();
    assert_eq!(child1.status, "done");

    let parent = mgr.get_task(1).await.unwrap();
    assert_eq!(
        parent.status, "doing",
        "Parent should remain 'doing' with incomplete children"
    );

    let child2 = mgr.get_task(3).await.unwrap();
    assert_eq!(child2.status, "todo");

    // Now complete second child
    mgr.start_task(3, false).await.unwrap();
    mgr.done_task().await.unwrap();

    // NOW parent should cascade to 'done'
    let parent_final = mgr.get_task(1).await.unwrap();
    assert_eq!(
        parent_final.status, "done",
        "Parent should cascade to 'done' after all children complete"
    );
}

#[tokio::test]
async fn test_done_cascade_complex_tree() {
    // Test cascading in a complex tree structure
    //
    //     1 (Grandparent)
    //    / \
    //   2   5 (Parents)
    //  / \   \
    // 3   4   6 (Children)
    //
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create the tree
    mgr.add_task("Grandparent", None, None).await.unwrap(); // id=1
    mgr.add_task("Parent1", None, Some(1)).await.unwrap(); // id=2
    mgr.add_task("Child1-1", None, Some(2)).await.unwrap(); // id=3
    mgr.add_task("Child1-2", None, Some(2)).await.unwrap(); // id=4
    mgr.add_task("Parent2", None, Some(1)).await.unwrap(); // id=5
    mgr.add_task("Child2-1", None, Some(5)).await.unwrap(); // id=6

    // Complete Child1-1 (id=3)
    mgr.start_task(3, false).await.unwrap();
    mgr.done_task().await.unwrap();

    // Parent1 should NOT cascade (Child1-2 incomplete)
    let parent1 = mgr.get_task(2).await.unwrap();
    assert_eq!(parent1.status, "doing");

    // Complete Child1-2 (id=4)
    mgr.start_task(4, false).await.unwrap();
    mgr.done_task().await.unwrap();

    // Parent1 should NOW cascade to 'done'
    let parent1_done = mgr.get_task(2).await.unwrap();
    assert_eq!(parent1_done.status, "done");

    // But Grandparent should NOT (Parent2/Child2-1 incomplete)
    let grandparent = mgr.get_task(1).await.unwrap();
    assert_eq!(grandparent.status, "doing");

    // Complete Child2-1 (id=6)
    mgr.start_task(6, false).await.unwrap();
    mgr.done_task().await.unwrap();

    // Parent2 should cascade
    let parent2 = mgr.get_task(5).await.unwrap();
    assert_eq!(parent2.status, "done");

    // And finally Grandparent should cascade
    let grandparent_final = mgr.get_task(1).await.unwrap();
    assert_eq!(
        grandparent_final.status, "done",
        "Grandparent should cascade after all descendants complete"
    );
}

#[tokio::test]
async fn test_combined_doing_and_done_cascade() {
    // Test that doing and done cascades work together correctly
    let (_temp_dir, pool) = setup_test_db().await;
    let mgr = TaskManager::new(&pool);

    // Create 2-level hierarchy
    mgr.add_task("Parent", None, None).await.unwrap();
    mgr.add_task("Child", None, Some(1)).await.unwrap();

    // Initially both 'todo'
    let parent = mgr.get_task(1).await.unwrap();
    let child = mgr.get_task(2).await.unwrap();
    assert_eq!(parent.status, "todo");
    assert_eq!(child.status, "todo");

    // Start child - triggers doing cascade
    mgr.start_task(2, false).await.unwrap();

    let parent_doing = mgr.get_task(1).await.unwrap();
    let child_doing = mgr.get_task(2).await.unwrap();
    assert_eq!(parent_doing.status, "doing");
    assert_eq!(child_doing.status, "doing");

    // Complete child - triggers done cascade
    mgr.done_task().await.unwrap();

    let parent_done = mgr.get_task(1).await.unwrap();
    let child_done = mgr.get_task(2).await.unwrap();
    assert_eq!(parent_done.status, "done");
    assert_eq!(child_done.status, "done");
}
