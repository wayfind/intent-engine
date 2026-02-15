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
use intent_engine::workspace::resolve_session_id;
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
    let done = tm.done_task_by_id(task.id, false).await.unwrap();
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
    let result = tm.done_task_by_id(parent.id, false).await;
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

// ── Phase 2: Coverage for uncovered public methods ──────────────

#[tokio::test]
async fn neo4j_find_tasks_filter_sort_paginate() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    // Create a parent with children in mixed statuses
    let parent = tm
        .add_task("Filter Parent", None, None, None, None, None)
        .await
        .unwrap();
    let t1 = tm
        .add_task("Todo A", None, Some(parent.id), None, Some(2), None)
        .await
        .unwrap();
    let t2 = tm
        .add_task(
            "Doing B",
            Some("spec"),
            Some(parent.id),
            None,
            Some(1),
            None,
        )
        .await
        .unwrap();
    tm.start_task(t2.id, false).await.unwrap();
    let t3 = tm
        .add_task("Todo C", None, Some(parent.id), None, Some(3), None)
        .await
        .unwrap();
    let _root2 = tm
        .add_task("Root Unrelated", None, None, None, None, None)
        .await
        .unwrap();

    // Filter by status
    let doing = tm
        .find_tasks(Some("doing"), None, None, None, None)
        .await
        .unwrap();
    assert!(doing.tasks.iter().all(|t| t.status == "doing"));
    assert!(doing.tasks.iter().any(|t| t.id == t2.id));

    // Filter by parent_id
    let children = tm
        .find_tasks(None, Some(Some(parent.id)), None, None, None)
        .await
        .unwrap();
    assert_eq!(children.total_count, 3);
    assert!(children
        .tasks
        .iter()
        .all(|t| t.parent_id == Some(parent.id)));

    // Pagination: limit=2, offset=0
    let page1 = tm
        .find_tasks(None, Some(Some(parent.id)), None, Some(2), Some(0))
        .await
        .unwrap();
    assert_eq!(page1.tasks.len(), 2);
    assert!(page1.has_more);

    // Pagination: limit=2, offset=2
    let page2 = tm
        .find_tasks(None, Some(Some(parent.id)), None, Some(2), Some(2))
        .await
        .unwrap();
    assert_eq!(page2.tasks.len(), 1);
    assert!(!page2.has_more);

    // Sort by priority
    use intent_engine::db::models::TaskSortBy;
    let sorted = tm
        .find_tasks(
            Some("todo"),
            Some(Some(parent.id)),
            Some(TaskSortBy::Priority),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(sorted.tasks.len(), 2);
    // priority 2 before priority 3
    assert_eq!(sorted.tasks[0].id, t1.id);
    assert_eq!(sorted.tasks[1].id, t3.id);

    // Clean up focus before teardown
    let wm = Neo4jWorkspaceManager::new(graph.clone(), pid.clone());
    let session_id = resolve_session_id(None);
    wm.clear_current_task(Some(&session_id)).await.unwrap();

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_pick_next_priority_order() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    // Empty project → no tasks
    let empty = tm.pick_next().await.unwrap();
    assert_eq!(empty.reason_code.as_deref(), Some("NO_TASKS_IN_PROJECT"));
    assert!(empty.task.is_none());

    // Create a parent with subtasks, start the parent to set focus
    let parent = tm
        .add_task("Parent", Some("spec"), None, None, None, None)
        .await
        .unwrap();
    let child_a = tm
        .add_task("Child A", None, Some(parent.id), None, None, None)
        .await
        .unwrap();
    let child_b = tm
        .add_task("Child B", None, Some(parent.id), None, None, None)
        .await
        .unwrap();
    tm.start_task(parent.id, false).await.unwrap();

    // With focused parent → should suggest a sub-task
    let next = tm.pick_next().await.unwrap();
    assert!(next.task.is_some());
    let suggested = next.task.unwrap();
    assert!(
        suggested.id == child_a.id || suggested.id == child_b.id,
        "Expected a child task, got #{}",
        suggested.id
    );

    // Complete all children → parent can be done → all complete
    tm.update_task(
        child_a.id,
        TaskUpdate {
            status: Some("done"),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tm.update_task(
        child_b.id,
        TaskUpdate {
            status: Some("done"),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tm.done_task_by_id(parent.id, false).await.unwrap();

    let all_done = tm.pick_next().await.unwrap();
    assert_eq!(all_done.reason_code.as_deref(), Some("ALL_TASKS_COMPLETED"));

    // Clean up focus before teardown
    let wm = Neo4jWorkspaceManager::new(graph.clone(), pid.clone());
    let session_id = resolve_session_id(None);
    wm.clear_current_task(Some(&session_id)).await.unwrap();

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_done_task_no_id_uses_focus() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());
    let wm = Neo4jWorkspaceManager::new(graph.clone(), pid.clone());

    // No focus → error
    let err = tm.done_task(false).await;
    assert!(err.is_err());
    let msg = err.unwrap_err().to_string();
    assert!(
        msg.contains("No current task") || msg.contains("no current"),
        "Expected focus error, got: {}",
        msg
    );

    // Create, start (which sets focus), then done via focus
    let task = tm
        .add_task(
            "Focus Done",
            Some("will complete via focus"),
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    tm.start_task(task.id, false).await.unwrap();

    // Verify focus is set
    let session_id = resolve_session_id(None);
    let focus = wm.get_current_task(Some(&session_id)).await.unwrap();
    assert_eq!(focus.current_task_id, Some(task.id));

    // Done with no ID → uses focus
    let result = tm.done_task(false).await.unwrap();
    assert_eq!(result.completed_task.id, task.id);
    assert_eq!(result.completed_task.status, "done");

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_delete_task_no_cascade() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());
    let wm = Neo4jWorkspaceManager::new(graph.clone(), pid.clone());

    // Leaf node delete succeeds
    let leaf = tm
        .add_task("Leaf", None, None, None, None, None)
        .await
        .unwrap();
    tm.delete_task(leaf.id).await.unwrap();
    assert!(tm.get_task(leaf.id).await.is_err());

    // Focused task cannot be deleted
    let focused = tm
        .add_task("Focused", Some("spec"), None, None, None, None)
        .await
        .unwrap();
    tm.start_task(focused.id, false).await.unwrap();

    let err = tm.delete_task(focused.id).await;
    assert!(err.is_err());
    let msg = err.unwrap_err().to_string();
    assert!(
        msg.contains("focused") || msg.contains("Unfocus"),
        "Expected focus-protection error, got: {}",
        msg
    );

    // Clean up: clear focus before teardown
    let session_id = resolve_session_id(None);
    wm.clear_current_task(Some(&session_id)).await.unwrap();

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_get_task_context_full() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    // Build: root -> parent -> [target, sibling]
    //        blocker --BLOCKED_BY--> target
    let root = tm
        .add_task("Root", None, None, None, None, None)
        .await
        .unwrap();
    let parent = tm
        .add_task("Parent", None, Some(root.id), None, None, None)
        .await
        .unwrap();
    let target = tm
        .add_task("Target", None, Some(parent.id), None, None, None)
        .await
        .unwrap();
    let sibling = tm
        .add_task("Sibling", None, Some(parent.id), None, None, None)
        .await
        .unwrap();
    let child = tm
        .add_task("Child of Target", None, Some(target.id), None, None, None)
        .await
        .unwrap();

    // Add dependency: target BLOCKED_BY blocker
    // Both tasks must appear in the plan for dependency validation to pass
    let pe = Neo4jPlanExecutor::new(graph.clone(), pid.clone());
    let dep_req = PlanRequest {
        tasks: vec![
            TaskTree {
                name: Some("Blocker".to_string()),
                ..Default::default()
            },
            TaskTree {
                id: Some(target.id),
                name: Some("Target".to_string()),
                depends_on: Some(vec!["Blocker".to_string()]),
                ..Default::default()
            },
        ],
    };
    let dep_result = pe.execute(&dep_req).await.unwrap();
    assert!(
        dep_result.success,
        "Dependency plan failed: {:?}",
        dep_result.error
    );
    let blocker_id = *dep_result.task_id_map.get("Blocker").unwrap();

    let ctx = tm.get_task_context(target.id).await.unwrap();

    // Ancestors: parent, root (in that order)
    assert_eq!(ctx.ancestors.len(), 2);
    assert_eq!(ctx.ancestors[0].id, parent.id);
    assert_eq!(ctx.ancestors[1].id, root.id);

    // Siblings (excludes target itself)
    assert_eq!(ctx.siblings.len(), 1);
    assert_eq!(ctx.siblings[0].id, sibling.id);

    // Children
    assert_eq!(ctx.children.len(), 1);
    assert_eq!(ctx.children[0].id, child.id);

    // Dependencies: target is blocked by blocker
    assert_eq!(ctx.dependencies.blocking_tasks.len(), 1);
    assert_eq!(ctx.dependencies.blocking_tasks[0].id, blocker_id);

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_get_task_with_events() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());
    let em = Neo4jEventManager::new(graph.clone(), pid.clone());

    let task = tm
        .add_task("Event Rich", None, None, None, None, None)
        .await
        .unwrap();
    em.add_event(task.id, "decision", "Chose Rust")
        .await
        .unwrap();
    em.add_event(task.id, "blocker", "Waiting for review")
        .await
        .unwrap();
    em.add_event(task.id, "note", "Progress update")
        .await
        .unwrap();

    let twe = tm.get_task_with_events(task.id).await.unwrap();
    assert_eq!(twe.task.id, task.id);
    let summary = twe.events_summary.unwrap();
    assert_eq!(summary.total_count, 3);
    // get_events_summary truncates to 10 most recent; 3 < 10 so all returned
    assert_eq!(summary.recent_events.len(), 3);
    let types: Vec<&str> = summary
        .recent_events
        .iter()
        .map(|e| e.log_type.as_str())
        .collect();
    assert!(types.contains(&"decision"));
    assert!(types.contains(&"blocker"));
    assert!(types.contains(&"note"));

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_blocking_and_blocked_by_tasks() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());
    let pe = Neo4jPlanExecutor::new(graph.clone(), pid.clone());

    // A --BLOCKED_BY--> B  (A depends on B; B blocks A)
    let req = PlanRequest {
        tasks: vec![
            TaskTree {
                name: Some("Foundation".to_string()),
                ..Default::default()
            },
            TaskTree {
                name: Some("Building".to_string()),
                depends_on: Some(vec!["Foundation".to_string()]),
                ..Default::default()
            },
        ],
    };
    let result = pe.execute(&req).await.unwrap();
    assert!(result.success);

    let building_id = *result.task_id_map.get("Building").unwrap();
    let foundation_id = *result.task_id_map.get("Foundation").unwrap();

    // Building is blocked by Foundation
    let blocking = tm.get_blocking_tasks(building_id).await.unwrap();
    assert_eq!(blocking.len(), 1);
    assert_eq!(blocking[0].id, foundation_id);

    // Foundation blocks Building (reversed direction)
    let blocked_by = tm.get_blocked_by_tasks(foundation_id).await.unwrap();
    assert_eq!(blocked_by.len(), 1);
    assert_eq!(blocked_by[0].id, building_id);

    // No dependencies → empty
    let none = tm.get_blocking_tasks(foundation_id).await.unwrap();
    assert!(none.is_empty());

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_get_root_tasks_ordering() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    let r1 = tm
        .add_task("Root Todo", None, None, None, None, None)
        .await
        .unwrap();
    let r2 = tm
        .add_task("Root Doing", Some("spec"), None, None, None, None)
        .await
        .unwrap();
    tm.start_task(r2.id, false).await.unwrap();
    let child = tm
        .add_task("Not Root", None, Some(r1.id), None, None, None)
        .await
        .unwrap();

    let roots = tm.get_root_tasks().await.unwrap();

    // Only root tasks (no children)
    assert!(
        roots.iter().all(|t| t.parent_id.is_none()),
        "All returned tasks should be root (no parent_id)"
    );
    assert!(roots.iter().any(|t| t.id == r1.id));
    assert!(roots.iter().any(|t| t.id == r2.id));
    assert!(!roots.iter().any(|t| t.id == child.id));

    // "doing" tasks should come before "todo" tasks
    let doing_pos = roots.iter().position(|t| t.id == r2.id).unwrap();
    let todo_pos = roots.iter().position(|t| t.id == r1.id).unwrap();
    assert!(
        doing_pos < todo_pos,
        "doing root should be ordered before todo root"
    );

    // Clean up focus before teardown
    let wm = Neo4jWorkspaceManager::new(graph.clone(), pid.clone());
    let session_id = resolve_session_id(None);
    wm.clear_current_task(Some(&session_id)).await.unwrap();

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_get_status_full_response() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());
    let em = Neo4jEventManager::new(graph.clone(), pid.clone());

    // Build hierarchy: root -> [target, sibling]; target -> descendant
    let root = tm
        .add_task("Status Root", None, None, None, None, None)
        .await
        .unwrap();
    let target = tm
        .add_task(
            "Status Target",
            Some("spec"),
            Some(root.id),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    let _sibling = tm
        .add_task("Status Sibling", None, Some(root.id), None, None, None)
        .await
        .unwrap();
    let _desc = tm
        .add_task("Descendant", None, Some(target.id), None, None, None)
        .await
        .unwrap();

    // Add an event
    em.add_event(target.id, "note", "Testing status")
        .await
        .unwrap();

    // With events
    let status = tm.get_status(target.id, true).await.unwrap();
    assert_eq!(status.focused_task.id, target.id);
    assert_eq!(status.focused_task.name, "Status Target");

    // Ancestors: exactly root
    assert_eq!(status.ancestors.len(), 1);
    assert_eq!(status.ancestors[0].id, root.id);

    // Siblings: exactly one (Status Sibling)
    assert_eq!(status.siblings.len(), 1);
    assert_eq!(status.siblings[0].id, _sibling.id);
    assert_eq!(status.siblings[0].name, "Status Sibling");
    assert_eq!(status.siblings[0].status, "todo");

    // Descendants: exactly one (Descendant)
    assert_eq!(status.descendants.len(), 1);
    assert_eq!(status.descendants[0].id, _desc.id);
    assert_eq!(status.descendants[0].name, "Descendant");

    // Events: exactly one note
    let events = status
        .events
        .as_ref()
        .expect("events should be Some when with_events=true");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].log_type, "note");
    assert_eq!(events[0].discussion_data, "Testing status");

    // Without events
    let status_no_events = tm.get_status(target.id, false).await.unwrap();
    assert!(
        status_no_events.events.is_none(),
        "events should be None when with_events=false"
    );

    teardown(&graph, &pid).await;
}

#[tokio::test]
async fn neo4j_plan_with_default_parent() {
    let (graph, pid) = setup().await;
    let tm = Neo4jTaskManager::new(graph.clone(), pid.clone());

    // Create parent
    let parent = tm
        .add_task("Default Parent", None, None, None, None, None)
        .await
        .unwrap();

    // Execute plan with default_parent set
    let pe = Neo4jPlanExecutor::new(graph.clone(), pid.clone()).with_default_parent(parent.id);
    let request = PlanRequest {
        tasks: vec![
            TaskTree {
                name: Some("Auto Child A".to_string()),
                ..Default::default()
            },
            TaskTree {
                name: Some("Auto Child B".to_string()),
                ..Default::default()
            },
        ],
    };
    let result = pe.execute(&request).await.unwrap();
    assert!(result.success, "Plan failed: {:?}", result.error);
    assert_eq!(result.created_count, 2);

    // Both tasks should be children of Default Parent
    let children = tm.get_children(parent.id).await.unwrap();
    assert_eq!(children.len(), 2);
    let names: Vec<&str> = children.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"Auto Child A"));
    assert!(names.contains(&"Auto Child B"));

    teardown(&graph, &pid).await;
}
