use axum::{
    routing::{get, post},
    Router,
};

use super::handlers;
use super::server::AppState;

/// Create API router with all endpoints
pub fn api_routes() -> Router<AppState> {
    Router::new()
        // Task management routes
        .route("/tasks", get(handlers::list_tasks).post(handlers::create_task))
        .route(
            "/tasks/:id",
            get(handlers::get_task)
                .patch(handlers::update_task)
                .delete(handlers::delete_task),
        )
        .route("/tasks/:id/start", post(handlers::start_task))
        .route("/tasks/:id/switch", post(handlers::switch_task))
        .route("/tasks/:id/spawn-subtask", post(handlers::spawn_subtask))
        // Task done is a global operation
        .route("/tasks/done", post(handlers::done_task))
        // Event routes
        .route(
            "/tasks/:id/events",
            get(handlers::list_events).post(handlers::create_event),
        )
        // Global routes
        .route("/current-task", get(handlers::get_current_task))
        .route("/pick-next", get(handlers::pick_next_task))
        .route("/search", get(handlers::search))
        .route("/switch-project", post(handlers::switch_project))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_routes_creation() {
        // This just verifies the routes can be created without panic
        let _router = api_routes();
    }
}
