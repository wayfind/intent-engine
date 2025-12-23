use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::{header, Method, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
};
use rust_embed::RustEmbed;
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use super::websocket;

/// Embedded static assets (HTML, CSS, JS)
#[derive(RustEmbed)]
#[folder = "static/"]
struct StaticAssets;

/// Minimal project info (no connection pool - SQLite is fast enough to open on demand)
#[derive(Clone, Debug)]
pub struct ProjectInfo {
    pub name: String,
    pub path: PathBuf,
    pub db_path: PathBuf,
}

/// Dashboard server state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Known projects (path -> info). No connection pools - SQLite opens fast.
    pub known_projects: Arc<RwLock<HashMap<PathBuf, ProjectInfo>>>,
    /// Currently active project path (for UI display)
    pub active_project_path: Arc<RwLock<PathBuf>>,
    /// The project that started the Dashboard (always considered online)
    pub host_project: super::websocket::ProjectInfo,
    pub port: u16,
    /// WebSocket state for real-time connections
    pub ws_state: super::websocket::WebSocketState,
    /// Shutdown signal sender (for graceful shutdown via HTTP)
    pub shutdown_tx: Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl AppState {
    /// Get database pool for a project (opens on demand - SQLite is fast)
    pub async fn get_db_pool(&self, project_path: &PathBuf) -> Result<SqlitePool, String> {
        let projects = self.known_projects.read().await;
        if let Some(info) = projects.get(project_path) {
            let db_url = format!("sqlite://{}", info.db_path.display());
            SqlitePool::connect(&db_url)
                .await
                .map_err(|e| format!("Failed to connect to database: {}", e))
        } else {
            Err(format!("Project not found: {}", project_path.display()))
        }
    }

    /// Get database pool for the active project
    pub async fn get_active_db_pool(&self) -> Result<SqlitePool, String> {
        let active_path = self.active_project_path.read().await.clone();
        self.get_db_pool(&active_path).await
    }

    /// Add a new project (or update existing)
    pub async fn add_project(&self, path: PathBuf) -> Result<(), String> {
        if !path.exists() {
            return Err(format!("Project path does not exist: {}", path.display()));
        }

        let db_path = path.join(".intent-engine").join("project.db");
        if !db_path.exists() {
            return Err(format!("Database not found: {}", db_path.display()));
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let info = ProjectInfo {
            name,
            path: path.clone(),
            db_path,
        };

        let mut projects = self.known_projects.write().await;
        projects.insert(path, info);
        Ok(())
    }

    /// Get active project info
    pub async fn get_active_project(&self) -> Option<ProjectInfo> {
        let active_path = self.active_project_path.read().await;
        let projects = self.known_projects.read().await;
        projects.get(&*active_path).cloned()
    }

    /// Switch active project
    pub async fn switch_active_project(&self, path: PathBuf) -> Result<(), String> {
        let projects = self.known_projects.read().await;
        if !projects.contains_key(&path) {
            return Err(format!("Project not registered: {}", path.display()));
        }
        drop(projects);

        let mut active = self.active_project_path.write().await;
        *active = path;
        Ok(())
    }

    /// Get active project's db_pool and path (backward compatibility helper)
    /// Returns (db_pool, project_path_string)
    pub async fn get_active_project_context(&self) -> Result<(SqlitePool, String), String> {
        let db_pool = self.get_active_db_pool().await?;
        let project_path = self
            .get_active_project()
            .await
            .map(|p| p.path.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok((db_pool, project_path))
    }
}

/// Dashboard server instance
pub struct DashboardServer {
    port: u16,
    db_path: PathBuf,
    project_name: String,
    project_path: PathBuf,
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
}

/// Project info response (for API)
#[derive(Serialize)]
struct ProjectInfoResponse {
    name: String,
    path: String,
    database: String,
    port: u16,
    is_online: bool,
    mcp_connected: bool,
}

impl DashboardServer {
    /// Create a new Dashboard server instance
    pub async fn new(port: u16, project_path: PathBuf, db_path: PathBuf) -> Result<Self> {
        // Determine project name from path
        let project_name = project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        if !db_path.exists() {
            anyhow::bail!(
                "Database not found at {}. Is this an Intent-Engine project?",
                db_path.display()
            );
        }

        Ok(Self {
            port,
            db_path,
            project_name,
            project_path,
        })
    }

    /// Run the Dashboard server
    pub async fn run(self) -> Result<()> {
        // Initialize known projects with the host project
        let mut known_projects = HashMap::new();
        let host_info = ProjectInfo {
            name: self.project_name.clone(),
            path: self.project_path.clone(),
            db_path: self.db_path.clone(),
        };
        known_projects.insert(self.project_path.clone(), host_info);

        // Create shared state
        let ws_state = websocket::WebSocketState::new();

        let host_project_info = websocket::ProjectInfo {
            name: self.project_name.clone(),
            path: self.project_path.display().to_string(),
            db_path: self.db_path.display().to_string(),
            agent: None,
            mcp_connected: false, // Will be updated dynamically
            is_online: true,      // Host is always online
        };

        // Create shutdown channel for graceful shutdown
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let state = AppState {
            known_projects: Arc::new(RwLock::new(known_projects)),
            active_project_path: Arc::new(RwLock::new(self.project_path.clone())),
            host_project: host_project_info,
            port: self.port,
            ws_state,
            shutdown_tx: Arc::new(tokio::sync::Mutex::new(Some(shutdown_tx))),
        };

        // Build router
        let app = create_router(state);

        // Bind to address
        // Bind to 0.0.0.0 to allow external access (e.g., from Windows host when running in WSL)
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .with_context(|| format!("Failed to bind to {}", addr))?;

        tracing::info!("Dashboard server listening on {}", addr);
        tracing::warn!(
            "⚠️  Dashboard is accessible from external IPs. Access via http://localhost:{} or http://<your-ip>:{}",
            self.port, self.port
        );
        tracing::info!("Project: {}", self.project_name);
        tracing::info!("Database: {}", self.db_path.display());

        // Ignore SIGHUP signal on Unix systems to prevent termination when terminal closes
        #[cfg(unix)]
        {
            unsafe {
                libc::signal(libc::SIGHUP, libc::SIG_IGN);
            }
        }

        // Run server with graceful shutdown
        tracing::info!("Starting server with graceful shutdown support");
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
                tracing::info!("Shutdown signal received, initiating graceful shutdown");
            })
            .await
            .context("Server error")?;

        tracing::info!("Dashboard server shut down successfully");
        Ok(())
    }
}

/// Create the Axum router with all routes and middleware
fn create_router(state: AppState) -> Router {
    use super::routes;

    // Combine basic API routes with full API routes
    let api_routes = Router::new()
        .route("/health", get(health_handler))
        .route("/info", get(info_handler))
        .merge(routes::api_routes());

    // Main router - all routes share the same AppState
    Router::new()
        // Root route - serve index.html
        .route("/", get(serve_index))
        // Static files under /static prefix (embedded)
        .route("/static/*path", get(serve_static))
        // Vite assets under /assets prefix
        .route("/assets/*path", get(serve_assets))
        // API routes under /api prefix
        .nest("/api", api_routes)
        // WebSocket routes (now use full AppState)
        .route("/ws/mcp", get(websocket::handle_mcp_websocket))
        .route("/ws/ui", get(websocket::handle_ui_websocket))
        // Fallback to 404
        .fallback(not_found_handler)
        // Add state
        .with_state(state)
        // Add middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
}

/// Serve the main index.html file from embedded assets
async fn serve_index() -> impl IntoResponse {
    match StaticAssets::get("index.html") {
        Some(content) => {
            let body = content.data.to_vec();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
                .header(header::PRAGMA, "no-cache")
                .header(header::EXPIRES, "0")
                .body(body.into())
                .unwrap()
        },
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("<h1>Error: index.html not found</h1>".to_string()),
        )
            .into_response(),
    }
}

/// Serve static files from embedded assets
async fn serve_static(Path(path): Path<String>) -> impl IntoResponse {
    // Remove leading slash if present
    let path = path.trim_start_matches('/');

    match StaticAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let body = content.data.to_vec();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
                .header(header::PRAGMA, "no-cache")
                .header(header::EXPIRES, "0")
                .body(body.into())
                .unwrap()
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "File not found",
                "code": "NOT_FOUND",
                "path": path
            })),
        )
            .into_response(),
    }
}

/// Serve assets from embedded assets (for Vite)
async fn serve_assets(Path(path): Path<String>) -> impl IntoResponse {
    // Remove leading slash if present
    let path = path.trim_start_matches('/');
    // Prepend "assets/" if not present (though the route is /assets/*path, so path usually won't have it unless we strip it in route)
    // Actually, the route is /assets/*path. If we request /assets/index.css, path is index.css.
    // We need to look up "assets/index.css" in StaticAssets.
    let full_path = format!("assets/{}", path);

    match StaticAssets::get(&full_path) {
        Some(content) => {
            let mime = mime_guess::from_path(&full_path).first_or_octet_stream();
            let body = content.data.to_vec();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .header(header::PRAGMA, "no-cache")
                .header(header::EXPIRES, "0")
                .body(body.into())
                .unwrap()
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Asset not found",
                "code": "NOT_FOUND",
                "path": full_path
            })),
        )
            .into_response(),
    }
}

/// Health check handler
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "intent-engine-dashboard".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Project info handler
/// Returns current Dashboard project info from the single source of truth (WebSocketState)
async fn info_handler(State(state): State<AppState>) -> Json<ProjectInfoResponse> {
    let active_project = state.get_active_project().await;

    match active_project {
        Some(project) => {
            // Get project info from WebSocketState (single source of truth)
            let projects = state
                .ws_state
                .get_online_projects_with_current(
                    &project.name,
                    &project.path,
                    &project.db_path,
                    &state.host_project,
                    state.port,
                )
                .await;

            // Return the first project (which is always the current Dashboard project)
            let current_project = projects.first().expect("Current project must exist");

            Json(ProjectInfoResponse {
                name: current_project.name.clone(),
                path: current_project.path.clone(),
                database: current_project.db_path.clone(),
                port: state.port,
                is_online: current_project.is_online,
                mcp_connected: current_project.mcp_connected,
            })
        },
        None => Json(ProjectInfoResponse {
            name: "unknown".to_string(),
            path: "".to_string(),
            database: "".to_string(),
            port: state.port,
            is_online: false,
            mcp_connected: false,
        }),
    }
}

/// 404 Not Found handler
async fn not_found_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": "Not found",
            "code": "NOT_FOUND"
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            service: "test".to_string(),
            version: "1.0.0".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_project_info_response_serialization() {
        let info = ProjectInfoResponse {
            name: "test-project".to_string(),
            path: "/path/to/project".to_string(),
            database: "/path/to/db".to_string(),
            port: 11391,
            is_online: true,
            mcp_connected: false,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test-project"));
        assert!(json.contains("11391"));
    }
}
