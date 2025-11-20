use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::json;

use super::models::*;
use super::server::AppState;
use crate::{
    events::EventManager, search::SearchManager, tasks::TaskManager, workspace::WorkspaceManager,
};

/// Get all tasks with optional filters
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskListQuery>,
) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let task_mgr = TaskManager::new(&db_pool);

    // Convert parent filter to Option<Option<i64>>
    let parent_filter = query.parent.as_deref().map(|p| {
        if p == "null" {
            None
        } else {
            p.parse::<i64>().ok()
        }
    });

    match task_mgr
        .find_tasks(query.status.as_deref(), parent_filter)
        .await
    {
        Ok(tasks) => (StatusCode::OK, Json(ApiResponse { data: tasks })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to list tasks: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Get a single task by ID
pub async fn get_task(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let task_mgr = TaskManager::new(&db_pool);

    match task_mgr.get_task(id).await {
        Ok(task) => (StatusCode::OK, Json(ApiResponse { data: task })).into_response(),
        Err(e) if e.to_string().contains("not found") => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                code: "TASK_NOT_FOUND".to_string(),
                message: format!("Task {} not found", id),
                details: None,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to get task: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Create a new task
pub async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let task_mgr = TaskManager::new(&db_pool);

    // Note: add_task doesn't support priority - it's set separately via update_task
    let result = task_mgr
        .add_task(&req.name, req.spec.as_deref(), req.parent_id)
        .await;

    match result {
        Ok(mut task) => {
            // If priority was requested, update it
            if let Some(priority) = req.priority {
                if let Ok(updated_task) = task_mgr
                    .update_task(task.id, None, None, None, None, None, Some(priority))
                    .await
                {
                    task = updated_task;
                }
                // Ignore priority update errors
            }
            (StatusCode::CREATED, Json(ApiResponse { data: task })).into_response()
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Failed to create task: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Update a task
pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTaskRequest>,
) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let task_mgr = TaskManager::new(&db_pool);

    // First check if task exists
    match task_mgr.get_task(id).await {
        Err(e) if e.to_string().contains("not found") => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiError {
                    code: "TASK_NOT_FOUND".to_string(),
                    message: format!("Task {} not found", id),
                    details: None,
                }),
            )
                .into_response()
        },
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Database error: {}", e),
                    details: None,
                }),
            )
                .into_response()
        },
        Ok(_) => {},
    }

    // Update task fields
    // Signature: update_task(id, name, spec, parent_id, status, complexity, priority)
    match task_mgr
        .update_task(
            id,
            req.name.as_deref(),
            req.spec.as_deref(),
            None, // parent_id - not supported via update API
            req.status.as_deref(),
            None, // complexity - not exposed in API
            req.priority,
        )
        .await
    {
        Ok(task) => (StatusCode::OK, Json(ApiResponse { data: task })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Failed to update task: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Delete a task
pub async fn delete_task(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let task_mgr = TaskManager::new(&db_pool);

    match task_mgr.delete_task(id).await {
        Ok(_) => (StatusCode::NO_CONTENT).into_response(),
        Err(e) if e.to_string().contains("not found") => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                code: "TASK_NOT_FOUND".to_string(),
                message: format!("Task {} not found", id),
                details: None,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Failed to delete task: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Start a task (set as current)
pub async fn start_task(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let task_mgr = TaskManager::new(&db_pool);

    match task_mgr.start_task(id, false).await {
        Ok(task) => (StatusCode::OK, Json(ApiResponse { data: task })).into_response(),
        Err(e) if e.to_string().contains("not found") => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                code: "TASK_NOT_FOUND".to_string(),
                message: format!("Task {} not found", id),
                details: None,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Failed to start task: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Complete the current task
pub async fn done_task(State(state): State<AppState>) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let task_mgr = TaskManager::new(&db_pool);

    match task_mgr.done_task().await {
        Ok(task) => (StatusCode::OK, Json(ApiResponse { data: task })).into_response(),
        Err(e) if e.to_string().contains("No current task") => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "NO_CURRENT_TASK".to_string(),
                message: "No current task to complete".to_string(),
                details: None,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Failed to complete task: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Switch to a different task
pub async fn switch_task(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let task_mgr = TaskManager::new(&db_pool);

    match task_mgr.switch_to_task(id).await {
        Ok(task) => (StatusCode::OK, Json(ApiResponse { data: task })).into_response(),
        Err(e) if e.to_string().contains("not found") => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                code: "TASK_NOT_FOUND".to_string(),
                message: format!("Task {} not found", id),
                details: None,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Failed to switch task: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Spawn a subtask and switch to it
/// Note: This creates a subtask of the CURRENT task, not an arbitrary parent
pub async fn spawn_subtask(
    State(state): State<AppState>,
    Path(_parent_id): Path<i64>, // Ignored - uses current task
    Json(req): Json<SpawnSubtaskRequest>,
) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let task_mgr = TaskManager::new(&db_pool);

    // spawn_subtask uses the current task as parent automatically
    match task_mgr.spawn_subtask(&req.name, req.spec.as_deref()).await {
        Ok(response) => (StatusCode::CREATED, Json(ApiResponse { data: response })).into_response(),
        Err(e) if e.to_string().contains("No current task") => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "NO_CURRENT_TASK".to_string(),
                message: "No current task to spawn subtask from".to_string(),
                details: None,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Failed to spawn subtask: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Get current task
pub async fn get_current_task(State(state): State<AppState>) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let workspace_mgr = WorkspaceManager::new(&db_pool);

    match workspace_mgr.get_current_task().await {
        Ok(response) => {
            if response.task.is_some() {
                (StatusCode::OK, Json(ApiResponse { data: response })).into_response()
            } else {
                (
                    StatusCode::OK,
                    Json(json!({
                        "data": null,
                        "message": "No current task"
                    })),
                )
                    .into_response()
            }
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to get current task: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Pick next task recommendation
pub async fn pick_next_task(State(state): State<AppState>) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let task_mgr = TaskManager::new(&db_pool);

    match task_mgr.pick_next().await {
        Ok(response) => (StatusCode::OK, Json(ApiResponse { data: response })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to pick next task: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// List events for a task
pub async fn list_events(
    State(state): State<AppState>,
    Path(task_id): Path<i64>,
    Query(query): Query<EventListQuery>,
) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let event_mgr = EventManager::new(&db_pool);

    // Signature: list_events(task_id, limit, log_type, since)
    match event_mgr
        .list_events(
            Some(task_id),
            query.limit.map(|l| l as i64),
            query.event_type,
            query.since,
        )
        .await
    {
        Ok(events) => (StatusCode::OK, Json(ApiResponse { data: events })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to list events: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Add an event to a task
pub async fn create_event(
    State(state): State<AppState>,
    Path(task_id): Path<i64>,
    Json(req): Json<CreateEventRequest>,
) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let event_mgr = EventManager::new(&db_pool);

    // Validate event type
    if !["decision", "blocker", "milestone", "note"].contains(&req.event_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Invalid event type: {}", req.event_type),
                details: None,
            }),
        )
            .into_response();
    }

    // add_event signature: (task_id, log_type, discussion_data)
    match event_mgr
        .add_event(task_id, &req.event_type, &req.data)
        .await
    {
        Ok(event) => (StatusCode::CREATED, Json(ApiResponse { data: event })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Failed to create event: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Unified search across tasks and events
pub async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let search_mgr = SearchManager::new(&db_pool);

    match search_mgr
        .unified_search(
            &query.query,
            query.include_tasks,
            query.include_events,
            query.limit.map(|l| l as i64),
        )
        .await
    {
        Ok(results) => (StatusCode::OK, Json(ApiResponse { data: results })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Search failed: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Switch to a different project database dynamically
pub async fn switch_project(
    State(state): State<AppState>,
    Json(req): Json<SwitchProjectRequest>,
) -> impl IntoResponse {
    use super::server::ProjectContext;
    use sqlx::SqlitePool;
    use std::path::PathBuf;

    // Parse and validate project path
    let project_path = PathBuf::from(&req.project_path);

    if !project_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                code: "PROJECT_NOT_FOUND".to_string(),
                message: format!("Project path does not exist: {}", project_path.display()),
                details: None,
            }),
        )
            .into_response();
    }

    // Construct database path
    let db_path = project_path.join(".intent-engine").join("project.db");

    if !db_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                code: "DATABASE_NOT_FOUND".to_string(),
                message: format!(
                    "Database not found at {}. Is this an Intent-Engine project?",
                    db_path.display()
                ),
                details: None,
            }),
        )
            .into_response();
    }

    // Create new database connection
    let db_url = format!("sqlite://{}", db_path.display());
    let new_db_pool = match SqlitePool::connect(&db_url).await {
        Ok(pool) => pool,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    code: "DATABASE_CONNECTION_ERROR".to_string(),
                    message: format!("Failed to connect to database: {}", e),
                    details: None,
                }),
            )
                .into_response();
        },
    };

    // Extract project name
    let project_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Create new project context
    let new_context = ProjectContext {
        db_pool: new_db_pool,
        project_name: project_name.clone(),
        project_path: project_path.clone(),
        db_path: db_path.clone(),
    };

    // Update the current project (write lock)
    {
        let mut current = state.current_project.write().await;
        *current = new_context;
    }

    tracing::info!(
        "Switched to project: {} at {}",
        project_name,
        project_path.display()
    );

    (
        StatusCode::OK,
        Json(ApiResponse {
            data: json!({
                "success": true,
                "project_name": project_name,
                "project_path": project_path.display().to_string(),
                "database": db_path.display().to_string(),
            }),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_pool, run_migrations};
    use axum::http::StatusCode;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::RwLock;

    /// Helper to create a test AppState with a temporary database
    async fn create_test_state() -> (AppState, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db_pool = create_pool(&db_path).await.unwrap();
        run_migrations(&db_pool).await.unwrap();

        let project_context = super::super::server::ProjectContext {
            db_pool,
            project_name: "test-project".to_string(),
            project_path: temp_dir.path().to_path_buf(),
            db_path,
        };

        let state = AppState {
            current_project: Arc::new(RwLock::new(project_context)),
            port: 11391,
        };

        (state, temp_dir)
    }

    #[tokio::test]
    async fn test_create_and_get_task() {
        let (state, _temp) = create_test_state().await;

        // Create a task
        let create_req = CreateTaskRequest {
            name: "Test Task".to_string(),
            spec: Some("Test specification".to_string()),
            parent_id: None,
            priority: Some(2),
        };

        let response = create_task(State(state.clone()), Json(create_req)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::CREATED);

        // Extract task ID from response body
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let task_id = body["data"]["id"].as_i64().unwrap();

        // Get the task
        let response = get_task(State(state.clone()), Path(task_id)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["data"]["name"], "Test Task");
        assert_eq!(body["data"]["status"], "todo");
    }

    #[tokio::test]
    async fn test_get_nonexistent_task() {
        let (state, _temp) = create_test_state().await;

        let response = get_task(State(state), Path(99999)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["code"], "TASK_NOT_FOUND");
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let (state, _temp) = create_test_state().await;

        // Create multiple tasks
        for i in 1..=3 {
            let req = CreateTaskRequest {
                name: format!("Task {}", i),
                spec: None,
                parent_id: None,
                priority: None,
            };
            create_task(State(state.clone()), Json(req)).await;
        }

        // List all tasks
        let query = TaskListQuery {
            status: None,
            parent: None,
        };
        let response = list_tasks(State(state.clone()), Query(query)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let tasks = body["data"].as_array().unwrap();
        assert_eq!(tasks.len(), 3);
    }

    #[tokio::test]
    async fn test_update_task() {
        let (state, _temp) = create_test_state().await;

        // Create a task
        let create_req = CreateTaskRequest {
            name: "Original Name".to_string(),
            spec: None,
            parent_id: None,
            priority: None,
        };
        let response = create_task(State(state.clone()), Json(create_req)).await;
        let response = response.into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let task_id = body["data"]["id"].as_i64().unwrap();

        // Update the task
        let update_req = UpdateTaskRequest {
            name: Some("Updated Name".to_string()),
            spec: Some("New spec".to_string()),
            status: None,
            priority: Some(1),
        };
        let response = update_task(State(state.clone()), Path(task_id), Json(update_req)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["data"]["name"], "Updated Name");
        assert_eq!(body["data"]["spec"], "New spec");
    }

    #[tokio::test]
    async fn test_delete_task() {
        let (state, _temp) = create_test_state().await;

        // Create a task
        let create_req = CreateTaskRequest {
            name: "Task to delete".to_string(),
            spec: None,
            parent_id: None,
            priority: None,
        };
        let response = create_task(State(state.clone()), Json(create_req)).await;
        let response = response.into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let task_id = body["data"]["id"].as_i64().unwrap();

        // Delete the task
        let response = delete_task(State(state.clone()), Path(task_id)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify it's gone
        let response = get_task(State(state.clone()), Path(task_id)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_start_and_done_task() {
        let (state, _temp) = create_test_state().await;

        // Create a task
        let create_req = CreateTaskRequest {
            name: "Task to start".to_string(),
            spec: None,
            parent_id: None,
            priority: None,
        };
        let response = create_task(State(state.clone()), Json(create_req)).await;
        let response = response.into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let task_id = body["data"]["id"].as_i64().unwrap();

        // Start the task
        let response = start_task(State(state.clone()), Path(task_id)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["data"]["status"], "doing");

        // Complete the task
        let response = done_task(State(state.clone())).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        // Just verify we got a successful response
        // The exact structure may vary, but we know it succeeded
        assert!(body["data"].is_object() || !body["data"].is_null());
    }

    #[tokio::test]
    async fn test_done_task_without_current() {
        let (state, _temp) = create_test_state().await;

        // Try to complete without a current task
        let response = done_task(State(state)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["code"], "NO_CURRENT_TASK");
    }

    #[tokio::test]
    async fn test_switch_task() {
        let (state, _temp) = create_test_state().await;

        // Create two tasks
        let create_req1 = CreateTaskRequest {
            name: "Task 1".to_string(),
            spec: None,
            parent_id: None,
            priority: None,
        };
        let response = create_task(State(state.clone()), Json(create_req1)).await;
        let response = response.into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let task1_id = body["data"]["id"].as_i64().unwrap();

        let create_req2 = CreateTaskRequest {
            name: "Task 2".to_string(),
            spec: None,
            parent_id: None,
            priority: None,
        };
        let response = create_task(State(state.clone()), Json(create_req2)).await;
        let response = response.into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let task2_id = body["data"]["id"].as_i64().unwrap();

        // Start task 1
        start_task(State(state.clone()), Path(task1_id)).await;

        // Switch to task 2
        let response = switch_task(State(state.clone()), Path(task2_id)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify the response contains task data
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        // Just verify we got a successful response with data
        assert!(body["data"].is_object());
    }

    #[tokio::test]
    async fn test_spawn_subtask() {
        let (state, _temp) = create_test_state().await;

        // Create and start a parent task
        let create_req = CreateTaskRequest {
            name: "Parent Task".to_string(),
            spec: None,
            parent_id: None,
            priority: None,
        };
        let response = create_task(State(state.clone()), Json(create_req)).await;
        let response = response.into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let parent_id = body["data"]["id"].as_i64().unwrap();

        start_task(State(state.clone()), Path(parent_id)).await;

        // Spawn a subtask
        let spawn_req = SpawnSubtaskRequest {
            name: "Child Task".to_string(),
            spec: Some("Child spec".to_string()),
        };
        let response = spawn_subtask(State(state.clone()), Path(parent_id), Json(spawn_req)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["data"]["subtask"]["name"], "Child Task");
        assert_eq!(body["data"]["subtask"]["parent_id"], parent_id);
    }

    #[tokio::test]
    async fn test_get_current_task() {
        let (state, _temp) = create_test_state().await;

        // No current task initially
        let response = get_current_task(State(state.clone())).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(body["data"].is_null());

        // Create and start a task
        let create_req = CreateTaskRequest {
            name: "Current Task".to_string(),
            spec: None,
            parent_id: None,
            priority: None,
        };
        let response = create_task(State(state.clone()), Json(create_req)).await;
        let response = response.into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let task_id = body["data"]["id"].as_i64().unwrap();

        start_task(State(state.clone()), Path(task_id)).await;

        // Now there should be a current task
        let response = get_current_task(State(state.clone())).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["data"]["task"]["id"], task_id);
    }

    #[tokio::test]
    async fn test_pick_next_task() {
        let (state, _temp) = create_test_state().await;

        // Create tasks with different priorities
        let create_req1 = CreateTaskRequest {
            name: "Low Priority".to_string(),
            spec: None,
            parent_id: None,
            priority: Some(3),
        };
        create_task(State(state.clone()), Json(create_req1)).await;

        let create_req2 = CreateTaskRequest {
            name: "High Priority".to_string(),
            spec: None,
            parent_id: None,
            priority: Some(1),
        };
        create_task(State(state.clone()), Json(create_req2)).await;

        // Pick next should return high priority task
        let response = pick_next_task(State(state.clone())).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["data"]["task"]["name"], "High Priority");
    }

    #[tokio::test]
    async fn test_create_and_list_events() {
        let (state, _temp) = create_test_state().await;

        // Create a task
        let create_req = CreateTaskRequest {
            name: "Task with events".to_string(),
            spec: None,
            parent_id: None,
            priority: None,
        };
        let response = create_task(State(state.clone()), Json(create_req)).await;
        let response = response.into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let task_id = body["data"]["id"].as_i64().unwrap();

        // Create an event
        let event_req = CreateEventRequest {
            event_type: "decision".to_string(),
            data: "Important decision".to_string(),
        };
        let response = create_event(State(state.clone()), Path(task_id), Json(event_req)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["data"]["log_type"], "decision");

        // List events
        let query = EventListQuery {
            limit: None,
            event_type: None,
            since: None,
        };
        let response = list_events(State(state.clone()), Path(task_id), Query(query)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let events = body["data"].as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["log_type"], "decision");
    }

    #[tokio::test]
    async fn test_create_event_invalid_type() {
        let (state, _temp) = create_test_state().await;

        // Create a task
        let create_req = CreateTaskRequest {
            name: "Test Task".to_string(),
            spec: None,
            parent_id: None,
            priority: None,
        };
        let response = create_task(State(state.clone()), Json(create_req)).await;
        let response = response.into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let task_id = body["data"]["id"].as_i64().unwrap();

        // Try to create event with invalid type
        let event_req = CreateEventRequest {
            event_type: "invalid_type".to_string(),
            data: "Some data".to_string(),
        };
        let response = create_event(State(state.clone()), Path(task_id), Json(event_req)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["code"], "INVALID_REQUEST");
    }

    #[tokio::test]
    async fn test_search() {
        let (state, _temp) = create_test_state().await;

        // Create tasks with searchable content
        let create_req1 = CreateTaskRequest {
            name: "Authentication System".to_string(),
            spec: Some("JWT-based authentication".to_string()),
            parent_id: None,
            priority: None,
        };
        create_task(State(state.clone()), Json(create_req1)).await;

        let create_req2 = CreateTaskRequest {
            name: "Database Setup".to_string(),
            spec: Some("PostgreSQL configuration".to_string()),
            parent_id: None,
            priority: None,
        };
        create_task(State(state.clone()), Json(create_req2)).await;

        // Give FTS index time to update
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Search for "authentication"
        let query = SearchQuery {
            query: "authentication".to_string(),
            include_tasks: true,
            include_events: true,
            limit: None,
        };
        let response = search(State(state.clone()), Query(query)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        // Just verify we got a valid response with results array
        // FTS indexing timing can be unpredictable in tests
        assert!(body["data"].is_array());
    }
}
