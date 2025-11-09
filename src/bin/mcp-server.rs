#!/usr/bin/env cargo
//! Intent-Engine MCP Server (Rust Implementation)
//!
//! This is a native Rust implementation of the MCP (Model Context Protocol) server
//! that provides a JSON-RPC 2.0 interface for AI assistants to interact with
//! intent-engine's task management capabilities.
//!
//! Unlike the Python wrapper (mcp-server.py), this implementation directly uses
//! the Rust library functions, avoiding subprocess overhead and improving performance.

use intent_engine::events::EventManager;
use intent_engine::project::ProjectContext;
use intent_engine::report::ReportManager;
use intent_engine::tasks::TaskManager;
use intent_engine::workspace::WorkspaceManager;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

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

#[tokio::main]
async fn main() {
    if let Err(e) = run_server().await {
        eprintln!("Fatal error: {}", e);
        std::process::exit(1);
    }
}

async fn run_server() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => handle_request(request).await,
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
        writeln!(stdout, "{}", response_json)?;
        stdout.flush()?;
    }

    Ok(())
}

async fn handle_request(request: JsonRpcRequest) -> JsonRpcResponse {
    let result = match request.method.as_str() {
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tool_call(request.params).await,
        _ => Err(format!("Unknown method: {}", request.method)),
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
        "task_start" => handle_task_start(params.arguments).await,
        "task_pick_next" => handle_task_pick_next(params.arguments).await,
        "task_spawn_subtask" => handle_task_spawn_subtask(params.arguments).await,
        "task_switch" => handle_task_switch(params.arguments).await,
        "task_done" => handle_task_done(params.arguments).await,
        "task_update" => handle_task_update(params.arguments).await,
        "task_find" => handle_task_find(params.arguments).await,
        "task_get" => handle_task_get(params.arguments).await,
        "event_add" => handle_event_add(params.arguments).await,
        "event_list" => handle_event_list(params.arguments).await,
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
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: name")?;

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
    let priority = args
        .get("priority")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);
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

async fn handle_task_find(args: Value) -> Result<Value, String> {
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
        .map_err(|e| format!("Failed to find tasks: {}", e))?;

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
    let task_id = args
        .get("task_id")
        .and_then(|v| v.as_i64())
        .ok_or("Missing required parameter: task_id")?;

    let limit = args.get("limit").and_then(|v| v.as_i64());

    let ctx = ProjectContext::load()
        .await
        .map_err(|e| format!("Failed to load project context: {}", e))?;

    let event_mgr = EventManager::new(&ctx.pool);
    let events = event_mgr
        .list_events(task_id, limit)
        .await
        .map_err(|e| format!("Failed to list events: {}", e))?;

    serde_json::to_value(&events).map_err(|e| format!("Serialization error: {}", e))
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
