//! Intent-Engine MCP Server (Rust Implementation)
//!
//! This is a native Rust implementation of the MCP (Model Context Protocol) server
//! that provides a JSON-RPC 2.0 interface for AI assistants to interact with
//! intent-engine's task management capabilities.
//!
//! Unlike the Python wrapper (mcp-server.py), this implementation directly uses
//! the Rust library functions, avoiding subprocess overhead and improving performance.

use crate::error::IntentError;
use crate::events::EventManager;
use crate::project::ProjectContext;
use crate::report::ReportManager;
use crate::tasks::TaskManager;
use crate::workspace::WorkspaceManager;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    arguments: Value,
}

/// MCP Tool Schema
const MCP_TOOLS: &str = include_str!("../../mcp-server.json");

/// Run the MCP server
/// This is the main entry point for MCP server mode
pub async fn run() -> io::Result<()> {
    // Load project context - only load existing projects, don't initialize new ones
    // This prevents blocking when MCP server is started outside an intent-engine project
    let ctx = match ProjectContext::load().await {
        Ok(ctx) => ctx,
        Err(IntentError::NotAProject) => {
            eprintln!("⚠️  Not in an intent-engine project directory.");
            eprintln!("   MCP server requires an intent-engine project to function.");
            eprintln!(
                "   Run 'ie workspace init' to create a project, or cd to an existing project."
            );
            return Err(io::Error::other(
                "MCP server must be run within an intent-engine project directory".to_string(),
            ));
        },
        Err(e) => {
            return Err(io::Error::other(format!(
                "Failed to load project context: {}",
                e
            )));
        },
    };

    // Auto-start Dashboard if not running (fully async, non-blocking)
    if !is_dashboard_running().await {
        eprintln!("🚀 Dashboard not running, starting automatically...");
        // Spawn Dashboard startup in background task - don't block MCP Server initialization
        tokio::spawn(async {
            if let Err(e) = start_dashboard_background().await {
                eprintln!("⚠️  Failed to start Dashboard: {}", e);
                eprintln!("   You can start it manually with: ie dashboard start");
            } else {
                eprintln!("✓ Dashboard started successfully at http://127.0.0.1:11391");
            }
        });
    } else {
        // Dashboard already running, show URL for user convenience
        eprintln!("ℹ️  Dashboard is running at http://127.0.0.1:11391");
    }

    // Show prominent notification about Dashboard GUI
    eprintln!("╭─────────────────────────────────────────────────────────╮");
    eprintln!("│  💡 Intent-Engine Dashboard GUI is available!          │");
    eprintln!("│     Visit: http://127.0.0.1:11391                      │");
    eprintln!("╰─────────────────────────────────────────────────────────╯");

    // Register MCP connection in the global registry (non-blocking)
    let project_root = ctx.root.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = register_mcp_connection(&project_root) {
            eprintln!("⚠ Failed to register MCP connection: {}", e);
        }
    });

    // Start heartbeat task
    let project_path = ctx.root.clone();
    let heartbeat_handle = tokio::spawn(async move {
        heartbeat_task(project_path).await;
    });

    // Run the MCP server
    let result = run_server().await;

    // Clean up: unregister MCP connection
    if let Err(e) = unregister_mcp_connection(&ctx.root) {
        eprintln!("⚠ Failed to unregister MCP connection: {}", e);
    }

    // Cancel heartbeat task
    heartbeat_handle.abort();

    result
}

async fn run_server() -> io::Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => {
                // Handle notifications (no id = no response needed)
                if request.id.is_none() {
                    handle_notification(&request).await;
                    continue; // Skip sending response for notifications
                }
                handle_request(request).await
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("Parse error: {}", e),
                }),
            },
        };

        let response_json = serde_json::to_string(&response)?;
        stdout.write_all(response_json.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_notification(request: &JsonRpcRequest) {
    // Handle MCP notifications (no response required)
    match request.method.as_str() {
        "initialized" => {
            eprintln!("✓ MCP client initialized");
        },
        "notifications/cancelled" => {
            eprintln!("⚠ Request cancelled");
        },
        _ => {
            eprintln!("⚠ Unknown notification: {}", request.method);
        },
    }
}

async fn handle_request(request: JsonRpcRequest) -> JsonRpcResponse {
    // Validate JSON-RPC version
    if request.jsonrpc != "2.0" {
        return JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: format!("Invalid JSON-RPC version: {}", request.jsonrpc),
            }),
        };
    }

    let result = match request.method.as_str() {
        "initialize" => handle_initialize(request.params),
        "ping" => Ok(json!({})), // Ping response for connection keep-alive
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tool_call(request.params).await,
        _ => Err(format!("Method not found: {}", request.method)),
    };

    match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(value),
            error: None,
        },
        Err(message) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message,
            }),
        },
    }
}

fn handle_initialize(_params: Option<Value>) -> Result<Value, String> {
    // MCP initialize handshake
    // Return server capabilities and info per MCP specification
    Ok(json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {
                "listChanged": false  // Static tool list, no dynamic changes
            }
        },
        "serverInfo": {
            "name": "intent-engine",
            "version": env!("CARGO_PKG_VERSION")
        }
    }))
}

fn handle_tools_list() -> Result<Value, String> {
    let config: Value = serde_json::from_str(MCP_TOOLS)
        .map_err(|e| format!("Failed to parse MCP tools schema: {}", e))?;

    Ok(json!({
        "tools": config.get("tools").unwrap_or(&json!([]))
    }))
}

async fn handle_tool_call(params: Option<Value>) -> Result<Value, String> {
    let params: ToolCallParams = serde_json::from_value(params.unwrap_or(json!({})))
        .map_err(|e| format!("Invalid tool call parameters: {}", e))?;

    let result = match params.name.as_str() {
        "task_add" => handle_task_add(params.arguments).await,
        "task_add_dependency" => handle_task_add_dependency(params.arguments).await,
        "task_start" => handle_task_start(params.arguments).await,
        "task_pick_next" => handle_task_pick_next(params.arguments).await,
        "task_spawn_subtask" => handle_task_spawn_subtask(params.arguments).await,
        "task_switch" => handle_task_switch(params.arguments).await,
        "task_done" => handle_task_done(params.arguments).await,
        "task_update" => handle_task_update(params.arguments).await,
        "task_list" => handle_task_list(params.arguments).await,
        "task_get" => handle_task_get(params.arguments).await,
        "task_context" => handle_task_context(params.arguments).await,
        "task_delete" => handle_task_delete(params.arguments).await,
        "event_add" => handle_event_add(params.arguments).await,
        "event_list" => handle_event_list(params.arguments).await,
        "search" => handle_unified_search(params.arguments).await,
        "current_task_get" => handle_current_task_get(params.arguments).await,
        "report_generate" => handle_report_generate(params.arguments).await,
        _ => Err(format!("Unknown tool: {}", params.name)),
    }?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&result)
                .unwrap_or_else(|_| "{}".to_string())
        }]
    }))
}

// Tool Handlers

async fn handle_task_add(args: Value) -> Result<Value, String> {
    // Improved parameter validation with specific error messages
    let name = match args.get("name") {
        None => return Err("Missing required parameter: name".to_string()),
        Some(value) => {
            if value.is_null() {
                return Err("Parameter 'name' cannot be null".to_string());
            }
            match value.as_str() {
                Some(s) if s.trim().is_empty() => {
                    return Err("Parameter 'name' cannot be empty".to_string());
                },
                Some(s) => s,
                None => return Err(format!("Parameter 'name' must be a string, got: {}", value)),
            }
        },
    };

    let spec = args.get("spec").and_then(|v| v.as_str());
    let parent_id = args.get("parent_id").and_then(|v| v.as_i64());

    let ctx = ProjectContext::load_or_init()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let task_mgr = TaskManager::new(&ctx.pool);
    let task = task_mgr
        .add_task(name, spec, parent_id)
        .await
        .map_err(|e| format!("Failed to add task: {}", e))?;

    serde_json::to_value(&task).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_task_add_dependency(args: Value) -> Result<Value, String> {
    let blocked_task_id = args
        .get("blocked_task_id")
        .and_then(|v| v.as_i64())
        .ok_or("Missing required parameter: blocked_task_id")?;

    let blocking_task_id = args
        .get("blocking_task_id")
        .and_then(|v| v.as_i64())
        .ok_or("Missing required parameter: blocking_task_id")?;

    let ctx = ProjectContext::load_or_init()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let dependency =
        crate::dependencies::add_dependency(&ctx.pool, blocking_task_id, blocked_task_id)
            .await
            .map_err(|e| format!("Failed to add dependency: {}", e))?;

    serde_json::to_value(&dependency).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_task_start(args: Value) -> Result<Value, String> {
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_i64())
        .ok_or("Missing required parameter: task_id")?;

    let with_events = args
        .get("with_events")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let ctx = ProjectContext::load_or_init()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let task_mgr = TaskManager::new(&ctx.pool);
    let task = task_mgr
        .start_task(task_id, with_events)
        .await
        .map_err(|e| format!("Failed to start task: {}", e))?;

    serde_json::to_value(&task).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_task_pick_next(args: Value) -> Result<Value, String> {
    let _max_count = args.get("max_count").and_then(|v| v.as_i64());
    let _capacity = args.get("capacity").and_then(|v| v.as_i64());

    let ctx = ProjectContext::load_or_init()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let task_mgr = TaskManager::new(&ctx.pool);
    let response = task_mgr
        .pick_next()
        .await
        .map_err(|e| format!("Failed to pick next task: {}", e))?;

    serde_json::to_value(&response).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_task_spawn_subtask(args: Value) -> Result<Value, String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: name")?;

    let spec = args.get("spec").and_then(|v| v.as_str());

    let ctx = ProjectContext::load_or_init()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let task_mgr = TaskManager::new(&ctx.pool);
    let subtask = task_mgr
        .spawn_subtask(name, spec)
        .await
        .map_err(|e| format!("Failed to spawn subtask: {}", e))?;

    serde_json::to_value(&subtask).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_task_switch(args: Value) -> Result<Value, String> {
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_i64())
        .ok_or("Missing required parameter: task_id")?;

    let ctx = ProjectContext::load_or_init()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let task_mgr = TaskManager::new(&ctx.pool);
    let task = task_mgr
        .switch_to_task(task_id)
        .await
        .map_err(|e| format!("Failed to switch task: {}", e))?;

    serde_json::to_value(&task).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_task_done(args: Value) -> Result<Value, String> {
    let task_id = args.get("task_id").and_then(|v| v.as_i64());

    let ctx = ProjectContext::load_or_init()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let task_mgr = TaskManager::new(&ctx.pool);

    // If task_id is provided, set it as current first
    if let Some(id) = task_id {
        let workspace_mgr = WorkspaceManager::new(&ctx.pool);
        workspace_mgr
            .set_current_task(id)
            .await
            .map_err(|e| format!("Failed to set current task: {}", e))?;
    }

    let task = task_mgr
        .done_task()
        .await
        .map_err(|e| format!("Failed to mark task as done: {}", e))?;

    serde_json::to_value(&task).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_task_update(args: Value) -> Result<Value, String> {
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_i64())
        .ok_or("Missing required parameter: task_id")?;

    let name = args.get("name").and_then(|v| v.as_str());
    let spec = args.get("spec").and_then(|v| v.as_str());
    let status = args.get("status").and_then(|v| v.as_str());
    let complexity = args
        .get("complexity")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);
    let priority = match args.get("priority").and_then(|v| v.as_str()) {
        Some(p) => Some(
            crate::priority::PriorityLevel::parse_to_int(p)
                .map_err(|e| format!("Invalid priority: {}", e))?,
        ),
        None => None,
    };
    let parent_id = args.get("parent_id").and_then(|v| v.as_i64()).map(Some);

    let ctx = ProjectContext::load_or_init()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let task_mgr = TaskManager::new(&ctx.pool);
    let task = task_mgr
        .update_task(task_id, name, spec, parent_id, status, complexity, priority)
        .await
        .map_err(|e| format!("Failed to update task: {}", e))?;

    serde_json::to_value(&task).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_task_list(args: Value) -> Result<Value, String> {
    let status = args.get("status").and_then(|v| v.as_str());
    let parent = args.get("parent").and_then(|v| v.as_str());

    let parent_opt = parent.map(|p| {
        if p == "null" {
            None
        } else {
            p.parse::<i64>().ok()
        }
    });

    let ctx = ProjectContext::load()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let task_mgr = TaskManager::new(&ctx.pool);
    let tasks = task_mgr
        .find_tasks(status, parent_opt)
        .await
        .map_err(|e| format!("Failed to list tasks: {}", e))?;

    serde_json::to_value(&tasks).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_task_get(args: Value) -> Result<Value, String> {
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_i64())
        .ok_or("Missing required parameter: task_id")?;

    let with_events = args
        .get("with_events")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let ctx = ProjectContext::load()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let task_mgr = TaskManager::new(&ctx.pool);

    if with_events {
        let task = task_mgr
            .get_task_with_events(task_id)
            .await
            .map_err(|e| format!("Failed to get task: {}", e))?;
        serde_json::to_value(&task).map_err(|e| format!("Serialization error: {}", e))
    } else {
        let task = task_mgr
            .get_task(task_id)
            .await
            .map_err(|e| format!("Failed to get task: {}", e))?;
        serde_json::to_value(&task).map_err(|e| format!("Serialization error: {}", e))
    }
}

async fn handle_task_context(args: Value) -> Result<Value, String> {
    // Get task_id from args, or fall back to current task
    let task_id = if let Some(id) = args.get("task_id").and_then(|v| v.as_i64()) {
        id
    } else {
        // Fall back to current_task_id if no task_id provided
        let ctx = ProjectContext::load()
            .await
            .map_err(|e| format!("Failed to load project context: {}", e))?;

        let current_task_id: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(&ctx.pool)
                .await
                .map_err(|e| format!("Database error: {}", e))?;

        current_task_id
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| {
                "No current task is set and task_id was not provided. \
                 Use task_start or task_switch to set a task first, or provide task_id parameter."
                    .to_string()
            })?
    };

    let ctx = ProjectContext::load()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let task_mgr = TaskManager::new(&ctx.pool);
    let context = task_mgr
        .get_task_context(task_id)
        .await
        .map_err(|e| format!("Failed to get task context: {}", e))?;

    serde_json::to_value(&context).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_task_delete(args: Value) -> Result<Value, String> {
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_i64())
        .ok_or("Missing required parameter: task_id")?;

    let ctx = ProjectContext::load()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let task_mgr = TaskManager::new(&ctx.pool);
    task_mgr
        .delete_task(task_id)
        .await
        .map_err(|e| format!("Failed to delete task: {}", e))?;

    Ok(json!({"success": true, "deleted_task_id": task_id}))
}

async fn handle_event_add(args: Value) -> Result<Value, String> {
    let task_id = args.get("task_id").and_then(|v| v.as_i64());

    let event_type = args
        .get("event_type")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: event_type")?;

    let data = args
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: data")?;

    let ctx = ProjectContext::load_or_init()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    // Determine the target task ID
    let target_task_id = if let Some(id) = task_id {
        id
    } else {
        // Fall back to current_task_id
        let current_task_id: Option<String> =
            sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = 'current_task_id'")
                .fetch_optional(&ctx.pool)
                .await
                .map_err(|e| format!("Database error: {}", e))?;

        current_task_id
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| {
                "No current task is set and task_id was not provided. \
                 Use task_start or task_switch to set a task first."
                    .to_string()
            })?
    };

    let event_mgr = EventManager::new(&ctx.pool);
    let event = event_mgr
        .add_event(target_task_id, event_type, data)
        .await
        .map_err(|e| format!("Failed to add event: {}", e))?;

    serde_json::to_value(&event).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_event_list(args: Value) -> Result<Value, String> {
    let task_id = args.get("task_id").and_then(|v| v.as_i64());

    let limit = args.get("limit").and_then(|v| v.as_i64());
    let log_type = args
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let since = args
        .get("since")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let ctx = ProjectContext::load()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let event_mgr = EventManager::new(&ctx.pool);
    let events = event_mgr
        .list_events(task_id, limit, log_type, since)
        .await
        .map_err(|e| format!("Failed to list events: {}", e))?;

    serde_json::to_value(&events).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_unified_search(args: Value) -> Result<Value, String> {
    use crate::search::SearchManager;

    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: query")?;

    let include_tasks = args
        .get("include_tasks")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let include_events = args
        .get("include_events")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let limit = args.get("limit").and_then(|v| v.as_i64());

    let ctx = ProjectContext::load()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let search_mgr = SearchManager::new(&ctx.pool);
    let results = search_mgr
        .unified_search(query, include_tasks, include_events, limit)
        .await
        .map_err(|e| format!("Failed to perform unified search: {}", e))?;

    serde_json::to_value(&results).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_current_task_get(_args: Value) -> Result<Value, String> {
    let ctx = ProjectContext::load()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let workspace_mgr = WorkspaceManager::new(&ctx.pool);
    let response = workspace_mgr
        .get_current_task()
        .await
        .map_err(|e| format!("Failed to get current task: {}", e))?;

    serde_json::to_value(&response).map_err(|e| format!("Serialization error: {}", e))
}

async fn handle_report_generate(args: Value) -> Result<Value, String> {
    let since = args.get("since").and_then(|v| v.as_str()).map(String::from);
    let status = args
        .get("status")
        .and_then(|v| v.as_str())
        .map(String::from);
    let filter_name = args
        .get("filter_name")
        .and_then(|v| v.as_str())
        .map(String::from);
    let filter_spec = args
        .get("filter_spec")
        .and_then(|v| v.as_str())
        .map(String::from);
    let summary_only = args
        .get("summary_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let ctx = ProjectContext::load()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let report_mgr = ReportManager::new(&ctx.pool);
    let report = report_mgr
        .generate_report(since, status, filter_name, filter_spec, summary_only)
        .await
        .map_err(|e| format!("Failed to generate report: {}", e))?;

    serde_json::to_value(&report).map_err(|e| format!("Serialization error: {}", e))
}

// ============================================================================
// MCP Connection Registry Integration
// ============================================================================

/// Register this MCP server instance with the global project registry
fn register_mcp_connection(project_path: &std::path::Path) -> anyhow::Result<()> {
    use crate::dashboard::registry::ProjectRegistry;

    let mut registry = ProjectRegistry::load()?;

    // Detect agent type from environment (Claude Code sets specific env vars)
    let agent_name = detect_agent_type();

    // Register MCP connection - this will create a project entry if none exists
    let project = registry.find_by_path(&project_path.to_path_buf());
    let dashboard_info = if let Some(p) = project {
        if p.port > 0 {
            format!("Dashboard: http://127.0.0.1:{}", p.port)
        } else {
            "MCP-only mode (no Dashboard)".to_string()
        }
    } else {
        "MCP-only mode (no Dashboard)".to_string()
    };

    registry.register_mcp_connection(&project_path.to_path_buf(), agent_name)?;

    eprintln!(
        "✓ MCP connection registered for project: {} ({})",
        project_path.display(),
        dashboard_info
    );

    Ok(())
}

/// Unregister this MCP server instance from the global project registry
fn unregister_mcp_connection(project_path: &std::path::Path) -> anyhow::Result<()> {
    use crate::dashboard::registry::ProjectRegistry;

    let mut registry = ProjectRegistry::load()?;
    registry.unregister_mcp_connection(&project_path.to_path_buf())?;

    eprintln!(
        "✓ MCP connection unregistered for project: {}",
        project_path.display()
    );

    Ok(())
}

/// Heartbeat task that keeps the MCP connection alive
async fn heartbeat_task(project_path: std::path::PathBuf) {
    use crate::dashboard::registry::ProjectRegistry;

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        interval.tick().await;

        // Update heartbeat (non-blocking)
        let path = project_path.clone();
        tokio::task::spawn_blocking(move || {
            if let Ok(mut registry) = ProjectRegistry::load() {
                if let Err(e) = registry.update_mcp_heartbeat(&path) {
                    eprintln!("⚠ Failed to update MCP heartbeat: {}", e);
                }
            }
        });
    }
}

/// Detect the agent type from environment variables
fn detect_agent_type() -> Option<String> {
    // Check for Claude Code specific environment variables
    if std::env::var("CLAUDE_CODE_VERSION").is_ok() {
        return Some("claude-code".to_string());
    }

    // Check for Claude Desktop
    if std::env::var("CLAUDE_DESKTOP").is_ok() {
        return Some("claude-desktop".to_string());
    }

    // Generic MCP client
    Some("mcp-client".to_string())
}

/// Check if Dashboard is running by testing the health endpoint
async fn is_dashboard_running() -> bool {
    // Use a timeout to prevent blocking - Dashboard check should be fast
    match tokio::time::timeout(
        std::time::Duration::from_millis(100), // Very short timeout
        tokio::net::TcpStream::connect("127.0.0.1:11391"),
    )
    .await
    {
        Ok(Ok(_)) => true,
        Ok(Err(_)) => false,
        Err(_) => {
            // Timeout occurred - assume dashboard is not running
            false
        },
    }
}

/// Start Dashboard in background using `ie dashboard start` command
async fn start_dashboard_background() -> io::Result<()> {
    use tokio::process::Command;

    // Get the current executable path
    let current_exe = std::env::current_exe()?;

    // Spawn Dashboard process in foreground mode
    Command::new(current_exe)
        .arg("dashboard")
        .arg("start")
        .arg("--foreground")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    // Wait for Dashboard to start (check health endpoint)
    for _ in 0..10 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if is_dashboard_running().await {
            return Ok(());
        }
    }

    Err(io::Error::other(
        "Dashboard failed to start within 5 seconds",
    ))
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
