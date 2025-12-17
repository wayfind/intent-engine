use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::json;

use super::models::*;
use super::server::AppState;
use crate::{
    db::models::TaskSortBy, events::EventManager, search::SearchManager, tasks::TaskManager,
    workspace::WorkspaceManager,
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

    // Parse sort_by parameter
    let sort_by = match query.sort_by.as_deref() {
        Some("id") => Some(TaskSortBy::Id),
        Some("priority") => Some(TaskSortBy::Priority),
        Some("time") => Some(TaskSortBy::Time),
        Some("focus") => Some(TaskSortBy::FocusAware),
        _ => Some(TaskSortBy::FocusAware), // Default to FocusAware
    };

    match task_mgr
        .find_tasks(
            query.status.as_deref(),
            parent_filter,
            sort_by,
            query.limit,
            query.offset,
        )
        .await
    {
        Ok(result) => (StatusCode::OK, Json(ApiResponse { data: result })).into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch tasks: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to list tasks: {}", e),
                    details: None,
                }),
            )
                .into_response()
        },
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
    let project = state.current_project.read().await;
    let db_pool = project.db_pool.clone();
    let project_path = project.project_path.to_string_lossy().to_string();
    drop(project);

    let task_mgr = TaskManager::with_websocket(
        &db_pool,
        std::sync::Arc::new(state.ws_state.clone()),
        project_path,
    );

    // Dashboard creates human-owned tasks (owner=None defaults to 'human')
    // This distinguishes from CLI-created tasks (owner='ai')
    // Note: Priority is set separately via update_task if needed
    let result = task_mgr
        .add_task(&req.name, req.spec.as_deref(), req.parent_id, None)
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
    let project = state.current_project.read().await;
    let db_pool = project.db_pool.clone();
    let project_path = project.project_path.to_string_lossy().to_string();
    drop(project);

    let task_mgr = TaskManager::with_websocket(
        &db_pool,
        std::sync::Arc::new(state.ws_state.clone()),
        project_path,
    );

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
    let project = state.current_project.read().await;
    let db_pool = project.db_pool.clone();
    let project_path = project.project_path.to_string_lossy().to_string();
    drop(project);

    let task_mgr = TaskManager::with_websocket(
        &db_pool,
        std::sync::Arc::new(state.ws_state.clone()),
        project_path,
    );

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
    let project = state.current_project.read().await;
    let db_pool = project.db_pool.clone();
    let project_path = project.project_path.to_string_lossy().to_string();
    drop(project);

    let task_mgr = TaskManager::with_websocket(
        &db_pool,
        std::sync::Arc::new(state.ws_state.clone()),
        project_path,
    );

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
    let project = state.current_project.read().await;
    let db_pool = project.db_pool.clone();
    let project_path = project.project_path.to_string_lossy().to_string();
    drop(project);

    let task_mgr = TaskManager::with_websocket(
        &db_pool,
        std::sync::Arc::new(state.ws_state.clone()),
        project_path,
    );

    // Dashboard = human caller, no passphrase needed
    match task_mgr.done_task(false).await {
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

/// Spawn a subtask and switch to it
/// Note: This creates a subtask of the CURRENT task, not an arbitrary parent
pub async fn spawn_subtask(
    State(state): State<AppState>,
    Path(_parent_id): Path<i64>, // Ignored - uses current task
    Json(req): Json<SpawnSubtaskRequest>,
) -> impl IntoResponse {
    let project = state.current_project.read().await;
    let db_pool = project.db_pool.clone();
    let project_path = project.project_path.to_string_lossy().to_string();
    drop(project);

    let task_mgr = TaskManager::with_websocket(
        &db_pool,
        std::sync::Arc::new(state.ws_state.clone()),
        project_path,
    );

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
    let project = state.current_project.read().await;
    let db_pool = project.db_pool.clone();
    let project_path = project.project_path.to_string_lossy().to_string();
    drop(project);

    let event_mgr = EventManager::with_websocket(
        &db_pool,
        std::sync::Arc::new(state.ws_state.clone()),
        project_path,
    );

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

/// Update an event
pub async fn update_event(
    State(state): State<AppState>,
    Path((task_id, event_id)): Path<(i64, i64)>,
    Json(req): Json<UpdateEventRequest>,
) -> impl IntoResponse {
    let project = state.current_project.read().await;
    let db_pool = project.db_pool.clone();
    let project_path = project.project_path.to_string_lossy().to_string();
    drop(project);

    let event_mgr = EventManager::with_websocket(
        &db_pool,
        std::sync::Arc::new(state.ws_state.clone()),
        project_path,
    );

    // Validate event type if provided
    if let Some(ref event_type) = req.event_type {
        if !["decision", "blocker", "milestone", "note"].contains(&event_type.as_str()) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    code: "INVALID_REQUEST".to_string(),
                    message: format!("Invalid event type: {}", event_type),
                    details: None,
                }),
            )
                .into_response();
        }
    }

    match event_mgr
        .update_event(event_id, req.event_type.as_deref(), req.data.as_deref())
        .await
    {
        Ok(event) => {
            // Verify the event belongs to the specified task
            if event.task_id != task_id {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiError {
                        code: "INVALID_REQUEST".to_string(),
                        message: format!("Event {} does not belong to task {}", event_id, task_id),
                        details: None,
                    }),
                )
                    .into_response();
            }
            (StatusCode::OK, Json(ApiResponse { data: event })).into_response()
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Failed to update event: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Delete an event
pub async fn delete_event(
    State(state): State<AppState>,
    Path((task_id, event_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    let project = state.current_project.read().await;
    let db_pool = project.db_pool.clone();
    let project_path = project.project_path.to_string_lossy().to_string();
    drop(project);

    let event_mgr = EventManager::with_websocket(
        &db_pool,
        std::sync::Arc::new(state.ws_state.clone()),
        project_path,
    );

    // First verify the event exists and belongs to the task
    match sqlx::query_as::<_, crate::db::models::Event>(crate::sql_constants::SELECT_EVENT_BY_ID)
        .bind(event_id)
        .fetch_optional(&db_pool)
        .await
    {
        Ok(Some(event)) => {
            if event.task_id != task_id {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiError {
                        code: "INVALID_REQUEST".to_string(),
                        message: format!("Event {} does not belong to task {}", event_id, task_id),
                        details: None,
                    }),
                )
                    .into_response();
            }
        },
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiError {
                    code: "EVENT_NOT_FOUND".to_string(),
                    message: format!("Event {} not found", event_id),
                    details: None,
                }),
            )
                .into_response();
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
                .into_response();
        },
    }

    match event_mgr.delete_event(event_id).await {
        Ok(_) => (StatusCode::NO_CONTENT).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "INVALID_REQUEST".to_string(),
                message: format!("Failed to delete event: {}", e),
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
        .search(
            &query.query,
            query.include_tasks,
            query.include_events,
            query.limit,
            query.offset,
            false,
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

/// List all registered projects (from in-memory state)
pub async fn list_projects(State(state): State<AppState>) -> impl IntoResponse {
    // Use the same method as WebSocket init for consistency
    let projects_info = {
        let current_project = state.current_project.read().await;
        state
            .ws_state
            .get_online_projects_with_current(
                &current_project.project_name,
                &current_project.project_path,
                &current_project.db_path,
                &state.host_project,
                state.port,
            )
            .await
    };

    // Convert ProjectInfo to API response format with additional metadata
    let port = state.port;
    let pid = std::process::id();

    let projects: Vec<serde_json::Value> = projects_info
        .iter()
        .map(|proj| {
            json!({
                "name": proj.name,
                "path": proj.path,
                "port": port,
                "pid": pid,
                "url": format!("http://127.0.0.1:{}", port),
                "started_at": chrono::Utc::now().to_rfc3339(),
                "mcp_connected": proj.mcp_connected,
                "is_online": proj.is_online,  // Now included!
                "mcp_agent": proj.agent,
                "mcp_last_seen": if proj.mcp_connected {
                    Some(chrono::Utc::now().to_rfc3339())
                } else {
                    None::<String>
                },
            })
        })
        .collect();

    (StatusCode::OK, Json(ApiResponse { data: projects })).into_response()
}

/// Switch to a different project database dynamically
pub async fn switch_project(
    State(state): State<AppState>,
    Json(req): Json<SwitchProjectRequest>,
) -> impl IntoResponse {
    use super::server::ProjectContext;
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

    // Create new database connection using the shared helper
    // This ensures consistent configuration (WAL mode, timeouts) and correct path handling
    let new_db_pool = match crate::db::create_pool(&db_path).await {
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

/// Get task context (ancestors, siblings, children)
pub async fn get_task_context(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let db_pool = state.current_project.read().await.db_pool.clone();
    let task_mgr = TaskManager::new(&db_pool);

    match task_mgr.get_task_context(id).await {
        Ok(context) => (StatusCode::OK, Json(ApiResponse { data: context })).into_response(),
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
                message: format!("Failed to get task context: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Handle CLI notification (internal endpoint for CLI → Dashboard sync)
pub async fn handle_cli_notification(
    State(state): State<AppState>,
    Json(message): Json<crate::dashboard::cli_notifier::NotificationMessage>,
) -> impl IntoResponse {
    tracing::debug!("Received CLI notification: {:?}", message);

    // Broadcast to all WebSocket clients
    let notification_json = serde_json::to_string(&message).unwrap_or_default();
    state.ws_state.broadcast_to_ui(&notification_json).await;

    (StatusCode::OK, Json(json!({"success": true}))).into_response()
}

/// Shutdown the Dashboard server gracefully
/// POST /api/internal/shutdown
pub async fn shutdown_handler(State(state): State<AppState>) -> impl IntoResponse {
    tracing::info!("Shutdown requested via HTTP endpoint");

    // Trigger shutdown signal
    let mut shutdown = state.shutdown_tx.lock().await;
    if let Some(tx) = shutdown.take() {
        if tx.send(()).is_ok() {
            tracing::info!("Shutdown signal sent successfully");
            (
                StatusCode::OK,
                Json(json!({
                    "status": "ok",
                    "message": "Dashboard is shutting down gracefully"
                })),
            )
                .into_response()
        } else {
            tracing::error!("Failed to send shutdown signal");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": "Failed to initiate shutdown"
                })),
            )
                .into_response()
        }
    } else {
        tracing::warn!("Shutdown already initiated");
        (
            StatusCode::CONFLICT,
            Json(json!({
                "status": "error",
                "message": "Shutdown already in progress"
            })),
        )
            .into_response()
    }
}
