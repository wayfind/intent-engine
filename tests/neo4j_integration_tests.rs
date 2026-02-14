//! Integration tests for the Neo4j backend.
//!
//! These tests require a live Neo4j instance. Enable with:
//!   cargo test --features neo4j-tests -- --test-threads=1 neo4j
//!
//! Required environment variables:
//!   NEO4J_URI       — e.g. "neo4j+s://xxx.databases.neo4j.io"
//!   NEO4J_PASSWORD   — database password
//!
//! Each test uses a unique project_id to isolate data, and cleans up after itself.

#![cfg(feature = "neo4j-tests")]

use intent_engine::neo4j::*;
use intent_engine::plan::{PlanRequest, TaskStatus, TaskTree};
use intent_engine::tasks::TaskUpdate;
use neo4rs::Graph;

/// Connect to Neo4j and initialize schema for a unique test project.
async fn setup() -> (Graph, String) {
    let uri = std::env::var("NEO4J_URI").expect("NEO4J_URI must be set for neo4j-tests");
    let password =
        std::env::var("NEO4J_PASSWORD").expect("NEO4J_PASSWORD must be set for neo4j-tests");
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".into());

    let graph = Graph::new(&uri, &user, &password)
        .await
        .expect("Failed to connect to Neo4j");

    // Use timestamp + pid for unique project isolation
    let project_id = format!(
        "test-{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    );

    // Initialize schema (counters, constraints, indexes)
    intent_engine::neo4j::schema::ensure_schema(&graph, &project_id)
        .await
        .expect("Schema init failed");

    (graph, project_id)
}

/// Clean up all nodes for a test project.
async fn teardown(graph: &Graph, project_id: &str) {
    graph
        .run(
            neo4rs::query("MATCH (n {project_id: $pid}) DETACH DELETE n")
                .param("pid", project_id.to_string()),
        )
        .await
        .expect("Teardown failed");
}

// ── Task CRUD ────────────────────────────────────────────────────

#[tokio::test]
async fn neo4j_task_create_get_update() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    // Create
    let task = tm
        .add_task("Test Task", Some("A spec"), None, Some("ai"), None, None)
        .await
        .unwrap();
    assert_eq!(task.name, "Test Task");
    assert_eq!(task.status, "todo");
    assert_eq!(task.spec.as_deref(), Some("A spec"));

    // Get
    let fetched = tm.get_task(task.id).await.unwrap();
    assert_eq!(fetched.id, task.id);
    assert_eq!(fetched.name, "Test Task");

    // Update
    let updated = tm
        .update_task(
            task.id,
            TaskUpdate {
                spec: Some("Updated spec"),
                priority: Some(1),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.spec.as_deref(), Some("Updated spec"));
    assert_eq!(updated.priority, Some(1));

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_task_parent_child_hierarchy() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    let parent = tm
        .add_task("Parent", None, None, None, None, None)
        .await
        .unwrap();
    let child = tm
        .add_task("Child", None, Some(parent.id), None, None, None)
        .await
        .unwrap();
    let grandchild = tm
        .add_task("Grandchild", None, Some(child.id), None, None, None)
        .await
        .unwrap();

    // Children
    let children = tm.get_children(parent.id).await.unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id, child.id);

    // Descendants
    let descendants = tm.get_descendants(parent.id).await.unwrap();
    assert_eq!(descendants.len(), 2);

    // Ancestry from grandchild
    let ancestry = tm.get_task_ancestry(grandchild.id).await.unwrap();
    assert_eq!(ancestry.len(), 2);
    assert_eq!(ancestry[0].id, child.id); // immediate parent first
    assert_eq!(ancestry[1].id, parent.id);

    // Siblings
    let child2 = tm
        .add_task("Child2", None, Some(parent.id), None, None, None)
        .await
        .unwrap();
    let siblings = tm.get_siblings(child.id, Some(parent.id)).await.unwrap();
    assert_eq!(siblings.len(), 1);
    assert_eq!(siblings[0].id, child2.id);

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_task_start_done_lifecycle() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    let task = tm
        .add_task(
            "Lifecycle Task",
            Some("do the thing"),
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(task.status, "todo");

    // Start
    let started = tm.start_task(task.id, false).await.unwrap();
    assert_eq!(started.task.status, "doing");

    // Done
    let done = tm.done_task_by_id(task.id).await.unwrap();
    assert_eq!(done.completed_task.status, "done");

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_task_done_blocked_by_children() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    let parent = tm
        .add_task("Parent", Some("parent spec"), None, None, None, None)
        .await
        .unwrap();
    let _child = tm
        .add_task("Child", None, Some(parent.id), None, None, None)
        .await
        .unwrap();

    // Start parent
    tm.start_task(parent.id, false).await.unwrap();

    // Cannot complete parent while child is not done
    let result = tm.done_task_by_id(parent.id).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("undone") || err.contains("child") || err.contains("subtask"),
        "Expected child-blocking error, got: {}",
        err
    );

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_task_delete_cascade() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    let parent = tm
        .add_task("Parent", None, None, None, None, None)
        .await
        .unwrap();
    let _child = tm
        .add_task("Child1", None, Some(parent.id), None, None, None)
        .await
        .unwrap();
    let _child2 = tm
        .add_task("Child2", None, Some(parent.id), None, None, None)
        .await
        .unwrap();

    let deleted = tm.delete_task_cascade(parent.id).await.unwrap();
    assert_eq!(deleted, 2); // descendants only (not including root)

    // Verify gone
    assert!(tm.get_task(parent.id).await.is_err());

    teardown(&graph, &pid).await;
}

// ── Plan Executor ────────────────────────────────────────────────

#[tokio::test]
async fn neo4j_plan_create_hierarchy() {
    let (graph, pid) = setup().await;
    let pe = Neo4jPlanExecutor::new(graph.clone(), pid.clone());
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    let request = PlanRequest {
        tasks: vec![TaskTree {
            name: Some("Parent Task".to_string()),
            status: Some(TaskStatus::Todo),
            spec: Some("Parent spec".to_string()),
            children: Some(vec![
                TaskTree {
                    name: Some("Child A".to_string()),
                    status: Some(TaskStatus::Todo),
                    ..Default::default()
                },
                TaskTree {
                    name: Some("Child B".to_string()),
                    status: Some(TaskStatus::Todo),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        }],
    };

    let result = pe.execute(&request).await.unwrap();
    assert!(result.success, "Plan failed: {:?}", result.error);
    assert_eq!(result.created_count, 3);

    // Verify hierarchy
    let parent_id = result.task_id_map.get("Parent Task").unwrap();
    let children = tm.get_children(*parent_id).await.unwrap();
    assert_eq!(children.len(), 2);

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_plan_idempotent_update() {
    let (graph, pid) = setup().await;
    let pe = Neo4jPlanExecutor::new(graph.clone(), pid.clone());
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    // Create
    let request = PlanRequest {
        tasks: vec![TaskTree {
            name: Some("Idempotent Task".to_string()),
            status: Some(TaskStatus::Todo),
            spec: Some("Original spec".to_string()),
            ..Default::default()
        }],
    };
    let r1 = pe.execute(&request).await.unwrap();
    assert!(r1.success);
    assert_eq!(r1.created_count, 1);

    // Update same name
    let update_request = PlanRequest {
        tasks: vec![TaskTree {
            name: Some("Idempotent Task".to_string()),
            status: Some(TaskStatus::Todo),
            spec: Some("Updated spec".to_string()),
            ..Default::default()
        }],
    };
    let r2 = pe.execute(&update_request).await.unwrap();
    assert!(r2.success);
    assert_eq!(r2.created_count, 0);
    assert_eq!(r2.updated_count, 1);

    // Verify updated
    let task_id = r2.task_id_map.get("Idempotent Task").unwrap();
    let task = tm.get_task(*task_id).await.unwrap();
    assert_eq!(task.spec.as_deref(), Some("Updated spec"));

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_plan_doing_requires_spec() {
    let (graph, pid) = setup().await;
    let pe = Neo4jPlanExecutor::new(graph.clone(), pid.clone());

    let request = PlanRequest {
        tasks: vec![TaskTree {
            name: Some("No Spec Task".to_string()),
            status: Some(TaskStatus::Doing),
            ..Default::default()
        }],
    };

    let result = pe.execute(&request).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("spec"));

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_plan_duplicate_names_rejected() {
    let (graph, pid) = setup().await;
    let pe = Neo4jPlanExecutor::new(graph.clone(), pid.clone());

    let request = PlanRequest {
        tasks: vec![
            TaskTree {
                name: Some("Dup".to_string()),
                ..Default::default()
            },
            TaskTree {
                name: Some("Dup".to_string()),
                ..Default::default()
            },
        ],
    };

    let result = pe.execute(&request).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Duplicate"));

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_plan_delete_by_id() {
    let (graph, pid) = setup().await;
    let pe = Neo4jPlanExecutor::new(graph.clone(), pid.clone());
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    // Create a task first
    let task = tm
        .add_task("To Delete", None, None, None, None, None)
        .await
        .unwrap();

    // Delete via plan
    let request = PlanRequest {
        tasks: vec![TaskTree {
            id: Some(task.id),
            delete: Some(true),
            ..Default::default()
        }],
    };

    let result = pe.execute(&request).await.unwrap();
    assert!(result.success, "Plan failed: {:?}", result.error);
    assert_eq!(result.deleted_count, 1);

    // Verify gone
    assert!(tm.get_task(task.id).await.is_err());

    teardown(&graph, &pid).await;
}

// ── BLOCKED_BY Dependencies ──────────────────────────────────────

#[tokio::test]
async fn neo4j_plan_dependencies_created() {
    let (graph, pid) = setup().await;
    let pe = Neo4jPlanExecutor::new(graph.clone(), pid.clone());

    let request = PlanRequest {
        tasks: vec![
            TaskTree {
                name: Some("Foundation".to_string()),
                status: Some(TaskStatus::Todo),
                ..Default::default()
            },
            TaskTree {
                name: Some("Building".to_string()),
                status: Some(TaskStatus::Todo),
                depends_on: Some(vec!["Foundation".to_string()]),
                ..Default::default()
            },
        ],
    };

    let result = pe.execute(&request).await.unwrap();
    assert!(result.success, "Plan failed: {:?}", result.error);
    assert_eq!(result.dependency_count, 1);

    // Verify BLOCKED_BY relationship exists in Neo4j
    let building_id = result.task_id_map.get("Building").unwrap();
    let foundation_id = result.task_id_map.get("Foundation").unwrap();

    let mut check = graph
        .execute(
            neo4rs::query(
                "MATCH (blocked:Task {project_id: $pid, id: $bid})\
                 -[:BLOCKED_BY]->\
                 (blocking:Task {project_id: $pid, id: $fid}) \
                 RETURN count(*) AS cnt",
            )
            .param("pid", pid.clone())
            .param("bid", *building_id)
            .param("fid", *foundation_id),
        )
        .await
        .unwrap();

    let row = check.next().await.unwrap().unwrap();
    let cnt: i64 = row.get("cnt").unwrap();
    assert_eq!(cnt, 1, "BLOCKED_BY relationship should exist");

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_plan_circular_dependency_rejected() {
    let (graph, pid) = setup().await;
    let pe = Neo4jPlanExecutor::new(graph.clone(), pid.clone());

    let request = PlanRequest {
        tasks: vec![
            TaskTree {
                name: Some("A".to_string()),
                depends_on: Some(vec!["B".to_string()]),
                ..Default::default()
            },
            TaskTree {
                name: Some("B".to_string()),
                depends_on: Some(vec!["A".to_string()]),
                ..Default::default()
            },
        ],
    };

    let result = pe.execute(&request).await.unwrap();
    assert!(!result.success);
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("circular"),
        "Expected circular dependency error, got: {:?}",
        result.error
    );

    teardown(&graph, &pid).await;
}

// ── Search ───────────────────────────────────────────────────────

/// Retry a search until it returns at least `min_count` results or timeout.
/// Neo4j fulltext indexes are eventually consistent — avoid hardcoded sleeps.
async fn retry_search(
    sm: &Neo4jSearchManager,
    query: &str,
    tasks: bool,
    events: bool,
    min_count: i64,
    max_retries: usize,
) -> intent_engine::db::models::PaginatedSearchResults {
    for i in 0..max_retries {
        let results = sm
            .search(query, tasks, events, Some(20), Some(0))
            .await
            .unwrap();
        let count = if tasks {
            results.total_tasks
        } else {
            results.total_events
        };
        if count >= min_count {
            return results;
        }
        if i < max_retries - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    // Return last attempt for assertion
    sm.search(query, tasks, events, Some(20), Some(0))
        .await
        .unwrap()
}

#[tokio::test]
async fn neo4j_search_tasks_fulltext() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());
    let sm = Neo4jSearchManager::new(graph.clone(), pid.clone());

    tm.add_task(
        "Implement authentication",
        Some("JWT tokens for user login"),
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    tm.add_task(
        "Fix database bug",
        Some("Connection pool timeout"),
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // Retry until fulltext index catches up (eventual consistency)
    let results = retry_search(&sm, "authentication", true, false, 1, 10).await;
    assert!(
        results.total_tasks >= 1,
        "Expected at least 1 task match, got {}",
        results.total_tasks
    );

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_search_tasks_contains_cjk() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());
    let sm = Neo4jSearchManager::new(graph.clone(), pid.clone());

    tm.add_task("实现用户认证", Some("使用JWT令牌"), None, None, None, None)
        .await
        .unwrap();

    // Short CJK query uses CONTAINS fallback (no index delay needed)
    let results = sm
        .search("认证", true, false, Some(10), Some(0))
        .await
        .unwrap();
    assert!(
        results.total_tasks >= 1,
        "CJK CONTAINS search should find task, got {}",
        results.total_tasks
    );

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_search_events() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());
    let em = Neo4jEventManager::new(graph.clone(), pid.clone());
    let sm = Neo4jSearchManager::new(graph.clone(), pid.clone());

    let task = tm
        .add_task("Event Host", None, None, None, None, None)
        .await
        .unwrap();
    em.add_event(
        task.id,
        "decision",
        "Chose PostgreSQL over MySQL for better JSON support",
    )
    .await
    .unwrap();

    // Retry until fulltext index catches up (eventual consistency)
    let results = retry_search(&sm, "PostgreSQL", false, true, 1, 10).await;
    assert!(
        results.total_events >= 1,
        "Expected at least 1 event match, got {}",
        results.total_events
    );

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_search_empty_query() {
    let (graph, pid) = setup().await;
    let sm = Neo4jSearchManager::new(graph.clone(), pid.clone());

    let results = sm.search("", true, true, Some(10), Some(0)).await.unwrap();
    assert_eq!(results.total_tasks, 0);
    assert_eq!(results.total_events, 0);
    assert!(results.results.is_empty());

    teardown(&graph, &pid).await;
}

// ── Events ───────────────────────────────────────────────────────

#[tokio::test]
async fn neo4j_event_crud() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());
    let em = Neo4jEventManager::new(graph.clone(), pid.clone());

    let task = tm
        .add_task("Event Target", None, None, None, None, None)
        .await
        .unwrap();

    // Add events
    let e1 = em
        .add_event(task.id, "decision", "Chose approach A")
        .await
        .unwrap();
    let e2 = em
        .add_event(task.id, "blocker", "Waiting for API access")
        .await
        .unwrap();

    assert_eq!(e1.log_type, "decision");
    assert_eq!(e2.log_type, "blocker");

    // List events
    let events = em
        .list_events(Some(task.id), None, None, None)
        .await
        .unwrap();
    assert_eq!(events.len(), 2);

    // Filter by type
    let decisions = em
        .list_events(Some(task.id), None, Some("decision".to_string()), None)
        .await
        .unwrap();
    assert_eq!(decisions.len(), 1);

    // Event for non-existent task
    let err = em.add_event(99999, "note", "ghost").await;
    assert!(err.is_err());

    teardown(&graph, &pid).await;
}

// ── Workspace (Session/Focus) ────────────────────────────────────

#[tokio::test]
async fn neo4j_workspace_focus() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());
    let wm = Neo4jWorkspaceManager::new(graph.clone(), pid.clone());

    let task = tm
        .add_task("Focus Target", Some("focus spec"), None, None, None, None)
        .await
        .unwrap();

    // Set focus
    let session_id = "test-session-1";
    wm.set_current_task(task.id, Some(session_id))
        .await
        .unwrap();

    // Get focus
    let response = wm.get_current_task(Some(session_id)).await.unwrap();
    assert_eq!(response.current_task_id, Some(task.id));

    // Clear focus
    wm.clear_current_task(Some(session_id)).await.unwrap();
    let cleared = wm.get_current_task(Some(session_id)).await.unwrap();
    assert_eq!(cleared.current_task_id, None);

    teardown(&graph, &pid).await;
}
