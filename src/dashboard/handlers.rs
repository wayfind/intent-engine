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
    let task_mgr = TaskManager::new(&state.db_pool);

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
    let task_mgr = TaskManager::new(&state.db_pool);

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
    let task_mgr = TaskManager::new(&state.db_pool);

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
    let task_mgr = TaskManager::new(&state.db_pool);

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
    let task_mgr = TaskManager::new(&state.db_pool);

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
    let task_mgr = TaskManager::new(&state.db_pool);

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
    let task_mgr = TaskManager::new(&state.db_pool);

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
    let task_mgr = TaskManager::new(&state.db_pool);

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
    let task_mgr = TaskManager::new(&state.db_pool);

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
    let workspace_mgr = WorkspaceManager::new(&state.db_pool);

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
    let task_mgr = TaskManager::new(&state.db_pool);

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
    let event_mgr = EventManager::new(&state.db_pool);

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
    let event_mgr = EventManager::new(&state.db_pool);

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
    let search_mgr = SearchManager::new(&state.db_pool);

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

/// List all registered projects
pub async fn list_projects() -> impl IntoResponse {
    match crate::dashboard::registry::ProjectRegistry::load() {
        Ok(mut registry) => {
            // Clean up stale MCP connections before returning
            registry.cleanup_stale_mcp_connections();
            if let Err(e) = registry.save() {
                eprintln!("⚠ Failed to save registry after cleanup: {}", e);
            }

            let projects: Vec<serde_json::Value> = registry
                .projects
                .iter()
                .map(|p| {
                    json!({
                        "name": p.name,
                        "path": p.path.display().to_string(),
                        "port": p.port,
                        "pid": p.pid,
                        "url": format!("http://127.0.0.1:{}", p.port),
                        "started_at": p.started_at,
                        "mcp_connected": p.mcp_connected,
                        "mcp_agent": p.mcp_agent,
                        "mcp_last_seen": p.mcp_last_seen,
                    })
                })
                .collect();

            (StatusCode::OK, Json(ApiResponse { data: projects })).into_response()
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "REGISTRY_ERROR".to_string(),
                message: format!("Failed to load project registry: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}
