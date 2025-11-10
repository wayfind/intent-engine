//! Integration tests for MCP server
//!
//! These tests verify the MCP server handles edge cases and errors robustly.

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

/// Helper function to send JSON-RPC request and get response
fn mcp_request(request: &Value) -> Value {
    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path();

    // Initialize project
    std::env::set_current_dir(project_path).unwrap();
    let _ = Command::new("intent-engine")
        .args(["task", "add", "--name", "test"])
        .output();

    // Send request to MCP server
    let mut child = Command::new("intent-engine")
        .arg("mcp-server")
        .env("INTENT_ENGINE_PROJECT_DIR", project_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all(request.to_string().as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    let response = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(response.lines().next().unwrap_or("{}")).unwrap()
}

#[test]
fn test_initialize_returns_capabilities() {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });

    let response = mcp_request(&request);

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());
    assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
    assert!(response["result"]["capabilities"]["tools"].is_object());
    assert_eq!(response["result"]["serverInfo"]["name"], "intent-engine");
}

#[test]
fn test_ping_returns_empty_result() {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "ping"
    });

    let response = mcp_request(&request);

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert_eq!(response["result"], json!({}));
}

#[test]
fn test_tools_list_returns_15_tools() {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/list"
    });

    let response = mcp_request(&request);

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"]["tools"].is_array());

    let tools = response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 15, "Expected 15 tools, got {}", tools.len());

    // Verify all expected tools are present
    let tool_names: Vec<String> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    let expected_tools = vec![
        "task_add",
        "task_start",
        "task_pick_next",
        "task_spawn_subtask",
        "task_switch",
        "task_done",
        "task_update",
        "task_find",
        "task_search", // FTS5 精华功能
        "task_get",
        "task_delete",
        "event_add",
        "event_list",
        "current_task_get",
        "report_generate",
    ];

    for expected in &expected_tools {
        assert!(
            tool_names.contains(&expected.to_string()),
            "Missing tool: {}",
            expected
        );
    }
}

#[test]
fn test_invalid_json_returns_parse_error() {
    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path();

    let mut child = Command::new("intent-engine")
        .arg("mcp-server")
        .env("INTENT_ENGINE_PROJECT_DIR", project_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all(b"{invalid json}\n").unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    let response = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(response.lines().next().unwrap_or("{}")).unwrap();

    assert!(json["error"].is_object());
    assert_eq!(json["error"]["code"], -32700); // Parse error code
    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Parse error"));
}

#[test]
fn test_unknown_method_returns_method_not_found() {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "unknown_method_xyz"
    });

    let response = mcp_request(&request);

    assert!(response["error"].is_object());
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Method not found"));
}

#[test]
fn test_task_search_with_fts5_query() {
    // NOTE: This test modifies process-wide current directory and may be flaky when run
    // in parallel with other tests. It passes consistently when run individually or sequentially.
    // This is a known limitation of Rust tests that modify global state.
    use std::thread;
    use std::time::Duration;

    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path();

    // Initialize project by changing to temp dir and running a command
    // This is necessary because initialize_project() doesn't respect INTENT_ENGINE_PROJECT_DIR
    let original_dir = std::env::current_dir().ok();
    std::env::set_current_dir(project_path).unwrap();

    let init_output = Command::new("intent-engine")
        .args(["task", "add", "--name", "__init_test__"])
        .output()
        .unwrap();

    // Try to restore original directory (may fail if other tests changed it)
    if let Some(dir) = original_dir {
        let _ = std::env::set_current_dir(&dir);
    }

    assert!(
        init_output.status.success(),
        "Failed to initialize project. stderr: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Create test tasks
    let output1 = Command::new("intent-engine")
        .args(["task", "add", "--name", "Fix authentication bug"])
        .env("INTENT_ENGINE_PROJECT_DIR", project_path)
        .output()
        .unwrap();
    assert!(
        output1.status.success(),
        "Failed to create task 1. stderr: {}",
        String::from_utf8_lossy(&output1.stderr)
    );

    let output2 = Command::new("intent-engine")
        .args(["task", "add", "--name", "Add payment feature"])
        .env("INTENT_ENGINE_PROJECT_DIR", project_path)
        .output()
        .unwrap();
    assert!(
        output2.status.success(),
        "Failed to create task 2. stderr: {}",
        String::from_utf8_lossy(&output2.stderr)
    );

    // Give DB time to sync
    thread::sleep(Duration::from_millis(100));

    // Search for "authentication"
    let request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "task_search",
            "arguments": {
                "query": "authentication"
            }
        }
    });

    let mut child = Command::new("intent-engine")
        .arg("mcp-server")
        .env("INTENT_ENGINE_PROJECT_DIR", project_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all(request.to_string().as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    let response = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    eprintln!("MCP Response: {}", response);
    eprintln!("MCP Stderr: {}", stderr);

    let json: Value = serde_json::from_str(response.lines().next().unwrap_or("{}")).unwrap();

    assert!(
        json["result"].is_object(),
        "Expected result object. Full response: {:?}",
        json
    );
    assert!(
        json["result"]["content"].is_array(),
        "Expected content array"
    );

    // Verify match snippet contains highlighted term
    let content_text = json["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(content_text).unwrap();
    assert!(
        parsed.is_array() && !parsed.as_array().unwrap().is_empty(),
        "Expected non-empty search results"
    );

    // Check first result has authentication highlighted
    let first_result = &parsed[0];
    let snippet = first_result["match_snippet"].as_str().unwrap();
    assert!(
        snippet.contains("**authentication**") || snippet.contains("**bug**"),
        "Expected highlighted match in snippet, got: {}",
        snippet
    );
}

#[test]
fn test_missing_required_parameter() {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": {
            "name": "task_add",
            "arguments": {
                // Missing required "name" parameter
                "spec": "Some spec"
            }
        }
    });

    let response = mcp_request(&request);

    assert!(response["error"].is_object());
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Missing required parameter"));
}
