use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::{Method, StatusCode},
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};

/// Dashboard server state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub project_name: String,
    pub project_path: PathBuf,
    pub port: u16,
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

/// Project info response
#[derive(Serialize)]
struct ProjectInfo {
    name: String,
    path: String,
    database: String,
    port: u16,
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
        // Create database connection pool
        let db_url = format!("sqlite://{}", self.db_path.display());
        let db_pool = SqlitePool::connect(&db_url)
            .await
            .context("Failed to connect to database")?;

        // Create shared state
        let state = AppState {
            db_pool,
            project_name: self.project_name.clone(),
            project_path: self.project_path.clone(),
            port: self.port,
        };

        // Build router
        let app = create_router(state);

        // Bind to address
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .with_context(|| format!("Failed to bind to {}", addr))?;

        tracing::info!("Dashboard server listening on {}", addr);
        tracing::info!("Project: {}", self.project_name);
        tracing::info!("Database: {}", self.db_path.display());

        // Run server
        axum::serve(listener, app).await.context("Server error")?;

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

    // Static file serving
    let static_dir = std::env::current_dir().unwrap().join("static");

    // Main router
    Router::new()
        // Root route - serve index.html
        .route("/", get(serve_index))
        // Static files under /static prefix
        .nest_service("/static", ServeDir::new(static_dir))
        // API routes under /api prefix
        .nest("/api", api_routes)
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

/// Serve the main index.html file
async fn serve_index() -> impl IntoResponse {
    match tokio::fs::read_to_string("static/index.html").await {
        Ok(content) => Html(content).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("<h1>Error: index.html not found</h1>".to_string()),
        )
            .into_response(),
    }
}

/// Legacy root handler - now unused, kept for reference
#[allow(dead_code)]
async fn index_handler(State(state): State<AppState>) -> Html<String> {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Intent-Engine Dashboard - {}</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }}
        .container {{
            background: white;
            border-radius: 16px;
            padding: 48px;
            max-width: 600px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
        }}
        h1 {{
            font-size: 2.5em;
            margin-bottom: 16px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }}
        .subtitle {{
            color: #666;
            font-size: 1.2em;
            margin-bottom: 32px;
        }}
        .info-grid {{
            display: grid;
            gap: 16px;
            margin-bottom: 32px;
        }}
        .info-item {{
            display: flex;
            align-items: center;
            padding: 16px;
            background: #f7f7f7;
            border-radius: 8px;
        }}
        .info-label {{
            font-weight: 600;
            color: #667eea;
            min-width: 100px;
        }}
        .info-value {{
            color: #333;
            word-break: break-all;
        }}
        .status {{
            display: inline-block;
            padding: 8px 16px;
            background: #10b981;
            color: white;
            border-radius: 20px;
            font-weight: 600;
            font-size: 0.9em;
        }}
        .footer {{
            text-align: center;
            color: #999;
            margin-top: 32px;
            font-size: 0.9em;
        }}
        a {{
            color: #667eea;
            text-decoration: none;
        }}
        a:hover {{
            text-decoration: underline;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Intent-Engine Dashboard</h1>
        <div class="subtitle">
            <span class="status">🟢 Running</span>
        </div>

        <div class="info-grid">
            <div class="info-item">
                <span class="info-label">Project:</span>
                <span class="info-value">{}</span>
            </div>
            <div class="info-item">
                <span class="info-label">Path:</span>
                <span class="info-value">{}</span>
            </div>
            <div class="info-item">
                <span class="info-label">Port:</span>
                <span class="info-value">{}</span>
            </div>
        </div>

        <div class="footer">
            <p>API Endpoints: <a href="/api/health">/api/health</a> • <a href="/api/info">/api/info</a></p>
            <p style="margin-top: 8px;">Intent-Engine v{} • <a href="https://github.com/wayfind/intent-engine" target="_blank">GitHub</a></p>
        </div>
    </div>
</body>
</html>
"#,
        state.project_name,
        state.project_name,
        state.project_path.display(),
        state.port,
        env!("CARGO_PKG_VERSION")
    );

    Html(html)
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
async fn info_handler(State(state): State<AppState>) -> Json<ProjectInfo> {
    Json(ProjectInfo {
        name: state.project_name.clone(),
        path: state.project_path.display().to_string(),
        database: state
            .project_path
            .join(".intent-engine")
            .join("intents.db")
            .display()
            .to_string(),
        port: state.port,
    })
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
    fn test_project_info_serialization() {
        let info = ProjectInfo {
            name: "test-project".to_string(),
            path: "/path/to/project".to_string(),
            database: "/path/to/db".to_string(),
            port: 3030,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test-project"));
        assert!(json.contains("3030"));
    }
}
