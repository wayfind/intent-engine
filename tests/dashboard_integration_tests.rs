use anyhow::Result;
use intent_engine::db::{create_pool, run_migrations};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Get the path to the ie binary
fn get_ie_binary() -> Result<PathBuf> {
    Ok(std::env::current_exe()?
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("ie"))
}

/// Initialize a project in the given directory
fn init_project(project_path: &Path) -> Result<()> {
    // Create .intent-engine directory
    let intent_dir = project_path.join(".intent-engine");
    std::fs::create_dir_all(&intent_dir)?;

    // Create database
    let db_path = intent_dir.join("project.db");

    // Use tokio runtime to run async database initialization
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let pool = create_pool(&db_path).await?;
        run_migrations(&pool).await?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Copy static files for Dashboard tests
    // Get the project root (where Cargo.toml is)
    let exe_path = std::env::current_exe()?;
    let project_root = exe_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("Failed to find project root"))?;

    let source_static = project_root.join("static");
    if source_static.exists() {
        let dest_static = project_path.join("static");
        copy_dir_recursive(&source_static, &dest_static)?;
    }

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path)?;
        }
    }

    Ok(())
}

/// Helper to manage a dashboard server process for testing
struct DashboardTestServer {
    process: Option<Child>,
    port: u16,
    _project_path: PathBuf,
}

impl DashboardTestServer {
    /// Start a new dashboard server on the given port
    fn start(port: u16, project_path: PathBuf) -> Result<Self> {
        let binary_path = get_ie_binary()?;

        let process = Command::new(&binary_path)
            .args([
                "dashboard",
                "start",
                "--foreground",
                "--port",
                &port.to_string(),
            ])
            .current_dir(&project_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        // Wait for server to start
        thread::sleep(Duration::from_secs(2));

        Ok(Self {
            process: Some(process),
            port,
            _project_path: project_path,
        })
    }

    /// Get the base URL for this server
    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Make a GET request to the server
    fn get(&self, path: &str) -> Result<reqwest::blocking::Response> {
        let url = format!("{}{}", self.base_url(), path);
        Ok(reqwest::blocking::get(&url)?)
    }

    /// Make a POST request to the server
    fn post(&self, path: &str, body: serde_json::Value) -> Result<reqwest::blocking::Response> {
        let url = format!("{}{}", self.base_url(), path);
        let client = reqwest::blocking::Client::new();
        Ok(client.post(&url).json(&body).send()?)
    }

    /// Make a PATCH request to the server
    fn patch(&self, path: &str, body: serde_json::Value) -> Result<reqwest::blocking::Response> {
        let url = format!("{}{}", self.base_url(), path);
        let client = reqwest::blocking::Client::new();
        Ok(client.patch(&url).json(&body).send()?)
    }

    /// Make a DELETE request to the server
    fn delete(&self, path: &str) -> Result<reqwest::blocking::Response> {
        let url = format!("{}{}", self.base_url(), path);
        let client = reqwest::blocking::Client::new();
        Ok(client.delete(&url).send()?)
    }
}

impl Drop for DashboardTestServer {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
}

#[test]
fn test_dashboard_health_check() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Initialize project
    init_project(temp_dir.path())?;

    let server = DashboardTestServer::start(3070, temp_dir.path().to_path_buf())?;

    // Test health endpoint
    let response = server.get("/api/health")?;
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json()?;
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "intent-engine-dashboard");

    Ok(())
}

#[test]
fn test_dashboard_task_crud() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Initialize project
    init_project(temp_dir.path())?;

    let server = DashboardTestServer::start(3071, temp_dir.path().to_path_buf())?;

    // Create a task
    let create_response = server.post(
        "/api/tasks",
        json!({
            "name": "Test Task via Dashboard",
            "spec": "# Test Spec\n\nThis is a test task created via Dashboard API.",
            "priority": 2
        }),
    )?;
    assert_eq!(create_response.status(), 201);

    let created: serde_json::Value = create_response.json()?;
    let task_id = created["data"]["id"].as_i64().unwrap();
    assert_eq!(created["data"]["name"], "Test Task via Dashboard");
    assert_eq!(created["data"]["status"], "todo");

    // Get the task
    let get_response = server.get(&format!("/api/tasks/{}", task_id))?;
    assert_eq!(get_response.status(), 200);

    let fetched: serde_json::Value = get_response.json()?;
    assert_eq!(fetched["data"]["id"], task_id);
    assert_eq!(fetched["data"]["name"], "Test Task via Dashboard");

    // Update the task
    let update_response = server.patch(
        &format!("/api/tasks/{}", task_id),
        json!({
            "name": "Updated Test Task",
            "priority": 1
        }),
    )?;
    assert_eq!(update_response.status(), 200);

    let updated: serde_json::Value = update_response.json()?;
    assert_eq!(updated["data"]["name"], "Updated Test Task");

    // List tasks
    let list_response = server.get("/api/tasks")?;
    assert_eq!(list_response.status(), 200);

    let list: serde_json::Value = list_response.json()?;
    let tasks = list["data"].as_array().unwrap();
    assert!(!tasks.is_empty());

    // Delete the task
    let delete_response = server.delete(&format!("/api/tasks/{}", task_id))?;
    assert_eq!(delete_response.status(), 204);

    // Verify deletion
    let get_after_delete = server.get(&format!("/api/tasks/{}", task_id))?;
    assert_eq!(get_after_delete.status(), 404);

    Ok(())
}

#[test]
#[ignore = "TODO: Fix task_done returning null status"]
fn test_dashboard_task_workflow() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Initialize project
    init_project(temp_dir.path())?;

    let server = DashboardTestServer::start(3072, temp_dir.path().to_path_buf())?;

    // Create a task
    let create_response = server.post(
        "/api/tasks",
        json!({
            "name": "Workflow Test Task",
            "spec": "Test task workflow"
        }),
    )?;
    let created: serde_json::Value = create_response.json()?;
    let task_id = created["data"]["id"].as_i64().unwrap();

    // Start the task
    let start_response = server.post(&format!("/api/tasks/{}/start", task_id), json!({}))?;
    assert_eq!(start_response.status(), 200);

    let started: serde_json::Value = start_response.json()?;
    assert_eq!(started["data"]["status"], "doing");

    // Check current task
    let current_response = server.get("/api/current-task")?;
    assert_eq!(current_response.status(), 200);

    let current: serde_json::Value = current_response.json()?;
    assert_eq!(current["data"]["task"]["id"], task_id);

    // Complete the task
    let done_response = server.post("/api/tasks/done", json!({}))?;
    assert_eq!(done_response.status(), 200);

    let done: serde_json::Value = done_response.json()?;
    assert_eq!(done["data"]["status"], "done");

    // Verify no current task
    let no_current_response = server.get("/api/current-task")?;
    assert_eq!(no_current_response.status(), 200);

    let no_current: serde_json::Value = no_current_response.json()?;
    assert!(no_current["data"].is_null() || no_current["data"]["task"].is_null());

    Ok(())
}

#[test]
fn test_dashboard_events() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Initialize project
    init_project(temp_dir.path())?;

    let server = DashboardTestServer::start(3073, temp_dir.path().to_path_buf())?;

    // Create a task
    let create_response = server.post(
        "/api/tasks",
        json!({
            "name": "Event Test Task"
        }),
    )?;
    let created: serde_json::Value = create_response.json()?;
    let task_id = created["data"]["id"].as_i64().unwrap();

    // Add a decision event
    let add_event_response = server.post(
        &format!("/api/tasks/{}/events", task_id),
        json!({
            "type": "decision",
            "data": "Decided to use approach A because of X, Y, Z"
        }),
    )?;
    assert_eq!(add_event_response.status(), 201);

    let event: serde_json::Value = add_event_response.json()?;
    assert_eq!(event["data"]["log_type"], "decision");

    // List events
    let list_events_response = server.get(&format!("/api/tasks/{}/events", task_id))?;
    assert_eq!(list_events_response.status(), 200);

    let events: serde_json::Value = list_events_response.json()?;
    let event_list = events["data"].as_array().unwrap();
    assert!(!event_list.is_empty());
    assert_eq!(event_list[0]["log_type"], "decision");

    Ok(())
}

#[test]
#[ignore = "TODO: Fix empty search results"]
fn test_dashboard_search() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Initialize project
    init_project(temp_dir.path())?;

    let server = DashboardTestServer::start(3074, temp_dir.path().to_path_buf())?;

    // Create tasks with searchable content
    server.post(
        "/api/tasks",
        json!({
            "name": "Implement Authentication System",
            "spec": "JWT-based authentication with refresh tokens"
        }),
    )?;

    server.post(
        "/api/tasks",
        json!({
            "name": "Setup Database",
            "spec": "PostgreSQL configuration and migrations"
        }),
    )?;

    // Wait for indexing
    thread::sleep(Duration::from_millis(100));

    // Search for "authentication"
    let search_response = server.get("/api/search?query=authentication")?;
    assert_eq!(search_response.status(), 200);

    let results: serde_json::Value = search_response.json()?;
    let result_list = results["data"].as_array().unwrap();
    assert!(!result_list.is_empty());

    // Verify we found the authentication task
    let has_auth_task = result_list.iter().any(|r| {
        r["result_type"] == "task"
            && r["task"]["name"]
                .as_str()
                .unwrap()
                .contains("Authentication")
    });
    assert!(has_auth_task);

    Ok(())
}

#[test]
fn test_dashboard_pick_next() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Initialize project
    init_project(temp_dir.path())?;

    let server = DashboardTestServer::start(3075, temp_dir.path().to_path_buf())?;

    // Create a task
    server.post(
        "/api/tasks",
        json!({
            "name": "First Task",
            "priority": 1
        }),
    )?;

    // Get next task recommendation
    let pick_response = server.get("/api/pick-next")?;
    assert_eq!(pick_response.status(), 200);

    let recommendation: serde_json::Value = pick_response.json()?;
    assert!(recommendation["data"]["task"].is_object());
    assert_eq!(recommendation["data"]["task"]["name"], "First Task");

    Ok(())
}

#[test]
#[ignore = "TODO: Fix 500 error on root route"]
fn test_dashboard_static_files() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Initialize project
    init_project(temp_dir.path())?;

    let server = DashboardTestServer::start(3076, temp_dir.path().to_path_buf())?;

    // Test root route (index.html)
    let index_response = server.get("/")?;
    assert_eq!(index_response.status(), 200);

    let html = index_response.text()?;
    assert!(html.contains("Intent-Engine Dashboard"));
    assert!(html.contains("TailwindCSS"));

    // Test static JS file
    let js_response = server.get("/static/js/app.js")?;
    assert_eq!(js_response.status(), 200);

    let js_content = js_response.text()?;
    assert!(js_content.contains("renderMarkdown"));
    assert!(js_content.contains("loadTasks"));

    Ok(())
}

#[test]
fn test_dashboard_spawn_subtask() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Initialize project
    init_project(temp_dir.path())?;

    let server = DashboardTestServer::start(3077, temp_dir.path().to_path_buf())?;

    // Create and start a parent task
    let create_response = server.post(
        "/api/tasks",
        json!({
            "name": "Parent Task"
        }),
    )?;
    let parent: serde_json::Value = create_response.json()?;
    let parent_id = parent["data"]["id"].as_i64().unwrap();

    server.post(&format!("/api/tasks/{}/start", parent_id), json!({}))?;

    // Spawn a subtask
    let spawn_response = server.post(
        &format!("/api/tasks/{}/spawn-subtask", parent_id),
        json!({
            "name": "Child Task",
            "spec": "Subtask specification"
        }),
    )?;
    assert_eq!(spawn_response.status(), 201);

    let spawn_result: serde_json::Value = spawn_response.json()?;
    let subtask_id = spawn_result["data"]["subtask"]["id"].as_i64().unwrap();
    assert_eq!(spawn_result["data"]["subtask"]["parent_id"], parent_id);
    assert_eq!(spawn_result["data"]["subtask"]["name"], "Child Task");

    // Verify subtask is now current
    let current_response = server.get("/api/current-task")?;
    let current: serde_json::Value = current_response.json()?;
    assert_eq!(current["data"]["task"]["id"], subtask_id);

    Ok(())
}

#[test]
fn test_dashboard_markdown_xss_protection() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;

    // Initialize project
    init_project(temp_dir.path())?;

    let server = DashboardTestServer::start(3078, temp_dir.path().to_path_buf())?;

    // Create a task with potentially malicious content
    let create_response = server.post(
        "/api/tasks",
        json!({
            "name": "XSS Test Task",
            "spec": "<script>alert('XSS')</script>\n\n# Safe Markdown\n\nThis should be safe."
        }),
    )?;
    assert_eq!(create_response.status(), 201);

    let created: serde_json::Value = create_response.json()?;

    // The spec should be stored as-is (backend doesn't sanitize)
    // Frontend (DOMPurify) will sanitize during rendering
    assert!(created["data"]["spec"]
        .as_str()
        .unwrap()
        .contains("<script>"));

    // Note: Full XSS protection verification would require browser testing
    // This test just verifies that the data round-trips correctly

    Ok(())
}
