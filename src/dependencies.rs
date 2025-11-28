use crate::db::models::Dependency;
use crate::error::{IntentError, Result};
use sqlx::SqlitePool;

/// Check if adding a dependency would create a circular dependency.
///
/// This function implements a depth-first search using SQLite's recursive CTE
/// to detect cycles in the dependency graph.
///
/// # Algorithm
///
/// To check if we can add "blocked_task depends on blocking_task":
/// 1. Start from blocking_task (the new prerequisite)
/// 2. Traverse what blocking_task depends on (its blocking tasks)
/// 3. If we ever reach blocked_task, adding this dependency would create a cycle
///
/// # Example
///
/// Existing: A depends on B (stored as: blocking=B, blocked=A)
/// Trying to add: B depends on A (would be: blocking=A, blocked=B)
///
/// Check: Does A depend on B?
/// - Start from A (new blocking task)
/// - Find what A depends on: B
/// - We reached B (new blocked task) → Cycle detected!
///
/// # Performance
///
/// - Time complexity: O(V + E) where V is tasks and E is dependencies
/// - Expected: <10ms for graphs with 10,000 tasks
/// - Depth limit: 100 levels to prevent infinite loops
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `blocking_task_id` - ID of the task that must be completed first
/// * `blocked_task_id` - ID of the task that depends on the blocking task
///
/// # Returns
///
/// - `Ok(true)` if adding this dependency would create a cycle
/// - `Ok(false)` if the dependency is safe to add
/// - `Err` if database query fails
pub async fn check_circular_dependency(
    pool: &SqlitePool,
    blocking_task_id: i64,
    blocked_task_id: i64,
) -> Result<bool> {
    // Self-dependency is always circular (but should be prevented by DB constraint)
    if blocking_task_id == blocked_task_id {
        return Ok(true);
    }

    // Check if blocking_task already (transitively) depends on blocked_task
    // If yes, adding "blocked depends on blocking" would create a cycle
    let has_cycle: bool = sqlx::query_scalar(
        r#"
        WITH RECURSIVE dep_chain(task_id, depth) AS (
            -- Start from the NEW blocking task
            SELECT ? as task_id, 0 as depth

            UNION ALL

            -- Follow what each task depends on (its blocking tasks)
            SELECT d.blocking_task_id, dc.depth + 1
            FROM dependencies d
            JOIN dep_chain dc ON d.blocked_task_id = dc.task_id
            WHERE dc.depth < 100
        )
        SELECT COUNT(*) > 0
        FROM dep_chain
        WHERE task_id = ?
        "#,
    )
    .bind(blocking_task_id)
    .bind(blocked_task_id)
    .fetch_one(pool)
    .await?;

    Ok(has_cycle)
}

/// Add a dependency between two tasks after checking for circular dependencies.
///
/// This is the safe way to add dependencies. It will:
/// 1. Verify both tasks exist
/// 2. Check for circular dependencies
/// 3. Add the dependency if safe
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `blocking_task_id` - ID of the task that must be completed first
/// * `blocked_task_id` - ID of the task that depends on the blocking task
///
/// # Returns
///
/// - `Ok(Dependency)` if the dependency was added successfully
/// - `Err(IntentError::CircularDependency)` if adding would create a cycle
/// - `Err(IntentError::TaskNotFound)` if either task doesn't exist
pub async fn add_dependency(
    pool: &SqlitePool,
    blocking_task_id: i64,
    blocked_task_id: i64,
) -> Result<Dependency> {
    // Verify both tasks exist
    let blocking_exists: bool = sqlx::query_scalar("SELECT COUNT(*) > 0 FROM tasks WHERE id = ?")
        .bind(blocking_task_id)
        .fetch_one(pool)
        .await?;

    if !blocking_exists {
        return Err(IntentError::TaskNotFound(blocking_task_id));
    }

    let blocked_exists: bool = sqlx::query_scalar("SELECT COUNT(*) > 0 FROM tasks WHERE id = ?")
        .bind(blocked_task_id)
        .fetch_one(pool)
        .await?;

    if !blocked_exists {
        return Err(IntentError::TaskNotFound(blocked_task_id));
    }

    // Check for circular dependency
    if check_circular_dependency(pool, blocking_task_id, blocked_task_id).await? {
        return Err(IntentError::CircularDependency {
            blocking_task_id,
            blocked_task_id,
        });
    }

    // Add the dependency
    let result = sqlx::query(
        r#"
        INSERT INTO dependencies (blocking_task_id, blocked_task_id)
        VALUES (?, ?)
        "#,
    )
    .bind(blocking_task_id)
    .bind(blocked_task_id)
    .execute(pool)
    .await?;

    let dependency_id = result.last_insert_rowid();

    // Fetch the created dependency
    let dependency = sqlx::query_as::<_, Dependency>(
        "SELECT id, blocking_task_id, blocked_task_id, created_at FROM dependencies WHERE id = ?",
    )
    .bind(dependency_id)
    .fetch_one(pool)
    .await?;

    Ok(dependency)
}

/// Check if a task is blocked by any incomplete tasks
///
/// A task is blocked if any of its blocking tasks are not in 'done' status.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `task_id` - ID of the task to check
///
/// # Returns
///
/// - `Ok(Some(Vec<i64>))` with IDs of incomplete blocking tasks if blocked
/// - `Ok(None)` if task is not blocked and can be started
pub async fn get_incomplete_blocking_tasks(
    pool: &SqlitePool,
    task_id: i64,
) -> Result<Option<Vec<i64>>> {
    let incomplete_blocking: Vec<i64> = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT d.blocking_task_id
        FROM dependencies d
        JOIN tasks t ON t.id = d.blocking_task_id
        WHERE d.blocked_task_id = ?
          AND t.status IN ('todo', 'doing')
        "#,
    )
    .bind(task_id)
    .fetch_all(pool)
    .await?;

    if incomplete_blocking.is_empty() {
        Ok(None)
    } else {
        Ok(Some(incomplete_blocking))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_pool, run_migrations};
    use tempfile::TempDir;

    async fn setup_test_db() -> (TempDir, SqlitePool) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = create_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();
        (temp_dir, pool)
    }

    async fn create_test_task(pool: &SqlitePool, name: &str) -> i64 {
        sqlx::query("INSERT INTO tasks (name, status) VALUES (?, 'todo')")
            .bind(name)
            .execute(pool)
            .await
            .unwrap()
            .last_insert_rowid()
    }

    #[tokio::test]
    async fn test_check_circular_dependency_self() {
        let (_temp, pool) = setup_test_db().await;
        let task_a = create_test_task(&pool, "Task A").await;

        // Self-dependency should be detected as circular
        let is_circular = check_circular_dependency(&pool, task_a, task_a)
            .await
            .unwrap();
        assert!(is_circular);
    }

    #[tokio::test]
    async fn test_check_circular_dependency_direct_cycle() {
        let (_temp, pool) = setup_test_db().await;
        let task_a = create_test_task(&pool, "Task A").await;
        let task_b = create_test_task(&pool, "Task B").await;

        // Add dependency: A depends on B (B → A)
        sqlx::query("INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)")
            .bind(task_b)
            .bind(task_a)
            .execute(&pool)
            .await
            .unwrap();

        // Try to add reverse dependency: B depends on A (A → B)
        // This would create a cycle: A → B → A
        let is_circular = check_circular_dependency(&pool, task_a, task_b)
            .await
            .unwrap();
        assert!(is_circular);
    }

    #[tokio::test]
    async fn test_check_circular_dependency_transitive_cycle() {
        let (_temp, pool) = setup_test_db().await;
        let task_a = create_test_task(&pool, "Task A").await;
        let task_b = create_test_task(&pool, "Task B").await;
        let task_c = create_test_task(&pool, "Task C").await;

        // Create chain: A → B → C
        sqlx::query("INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)")
            .bind(task_b)
            .bind(task_a)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)")
            .bind(task_c)
            .bind(task_b)
            .execute(&pool)
            .await
            .unwrap();

        // Try to add C → A (would create cycle: A → B → C → A)
        let is_circular = check_circular_dependency(&pool, task_a, task_c)
            .await
            .unwrap();
        assert!(is_circular);
    }

    #[tokio::test]
    async fn test_check_circular_dependency_no_cycle() {
        let (_temp, pool) = setup_test_db().await;
        let task_a = create_test_task(&pool, "Task A").await;
        let task_b = create_test_task(&pool, "Task B").await;
        let task_c = create_test_task(&pool, "Task C").await;

        // Create chain: A → B
        sqlx::query("INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)")
            .bind(task_b)
            .bind(task_a)
            .execute(&pool)
            .await
            .unwrap();

        // Try to add C → A (no cycle, C is independent)
        let is_circular = check_circular_dependency(&pool, task_a, task_c)
            .await
            .unwrap();
        assert!(!is_circular);
    }

    #[tokio::test]
    async fn test_check_circular_dependency_deep_chain() {
        let (_temp, pool) = setup_test_db().await;
        let task_a = create_test_task(&pool, "Task A").await;
        let task_b = create_test_task(&pool, "Task B").await;
        let task_c = create_test_task(&pool, "Task C").await;
        let task_d = create_test_task(&pool, "Task D").await;
        let task_e = create_test_task(&pool, "Task E").await;

        // Create chain: A → B → C → D → E
        sqlx::query("INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)")
            .bind(task_b)
            .bind(task_a)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)")
            .bind(task_c)
            .bind(task_b)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)")
            .bind(task_d)
            .bind(task_c)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO dependencies (blocking_task_id, blocked_task_id) VALUES (?, ?)")
            .bind(task_e)
            .bind(task_d)
            .execute(&pool)
            .await
            .unwrap();

        // Try to add E → A (would create long cycle)
        let is_circular = check_circular_dependency(&pool, task_a, task_e)
            .await
            .unwrap();
        assert!(is_circular);
    }

    #[tokio::test]
    async fn test_add_dependency_success() {
        let (_temp, pool) = setup_test_db().await;
        let task_a = create_test_task(&pool, "Task A").await;
        let task_b = create_test_task(&pool, "Task B").await;

        let dep = add_dependency(&pool, task_b, task_a).await.unwrap();

        assert_eq!(dep.blocking_task_id, task_b);
        assert_eq!(dep.blocked_task_id, task_a);
    }

    #[tokio::test]
    async fn test_add_dependency_circular_error() {
        let (_temp, pool) = setup_test_db().await;
        let task_a = create_test_task(&pool, "Task A").await;
        let task_b = create_test_task(&pool, "Task B").await;

        // Add A → B
        add_dependency(&pool, task_b, task_a).await.unwrap();

        // Try to add B → A (circular)
        let result = add_dependency(&pool, task_a, task_b).await;
        assert!(matches!(
            result,
            Err(IntentError::CircularDependency { .. })
        ));
    }

    #[tokio::test]
    async fn test_add_dependency_task_not_found() {
        let (_temp, pool) = setup_test_db().await;
        let task_a = create_test_task(&pool, "Task A").await;

        // Try to add dependency with non-existent task
        let result = add_dependency(&pool, 9999, task_a).await;
        assert!(matches!(result, Err(IntentError::TaskNotFound(9999))));
    }

    #[tokio::test]
    async fn test_get_incomplete_blocking_tasks_blocked() {
        let (_temp, pool) = setup_test_db().await;
        let task_a = create_test_task(&pool, "Task A").await;
        let task_b = create_test_task(&pool, "Task B").await;

        // A depends on B (B is todo)
        add_dependency(&pool, task_b, task_a).await.unwrap();

        let incomplete = get_incomplete_blocking_tasks(&pool, task_a).await.unwrap();
        assert!(incomplete.is_some());
        assert_eq!(incomplete.unwrap(), vec![task_b]);
    }

    #[tokio::test]
    async fn test_get_incomplete_blocking_tasks_not_blocked() {
        let (_temp, pool) = setup_test_db().await;
        let task_a = create_test_task(&pool, "Task A").await;
        let task_b = create_test_task(&pool, "Task B").await;

        // A depends on B, but mark B as done
        add_dependency(&pool, task_b, task_a).await.unwrap();
        sqlx::query("UPDATE tasks SET status = 'done' WHERE id = ?")
            .bind(task_b)
            .execute(&pool)
            .await
            .unwrap();

        let incomplete = get_incomplete_blocking_tasks(&pool, task_a).await.unwrap();
        assert!(incomplete.is_none());
    }
}
