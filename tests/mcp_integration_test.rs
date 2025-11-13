//! Integration tests for MCP server
//!
//! These tests verify the MCP server handles edge cases and errors robustly.

use assert_cmd::cargo;
use serde_json::{json, Value};
use serial_test::serial;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::tempdir;

/// Get the path to the intent-engine binary built by cargo test
fn get_binary_path() -> PathBuf {
    cargo::cargo_bin!("intent-engine").to_path_buf()
}

/// Helper function to send JSON-RPC request and get response
fn mcp_request(request: &Value) -> Value {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let project_path = temp_dir.path();

    // Initialize project
    std::env::set_current_dir(project_path).expect("Failed to change to project directory");
    let init_output = Command::new(get_binary_path())
        .args(["task", "add", "--name", "test"])
        .output()
        .expect("Failed to execute initialization command");
    assert!(
        init_output.status.success(),
        "Failed to initialize test project. stderr: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Send request to MCP server
    let mut child = Command::new(get_binary_path())
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
#[serial]
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
#[serial]
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
#[serial]
fn test_tools_list_returns_16_tools() {
    // Load expected tools from mcp-server.json (single source of truth)
    let mcp_schema_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("mcp-server.json");
    let mcp_schema_content =
        std::fs::read_to_string(&mcp_schema_path).expect("Failed to read mcp-server.json");
    let mcp_schema: Value =
        serde_json::from_str(&mcp_schema_content).expect("Failed to parse mcp-server.json");

    let expected_tools_from_schema = mcp_schema["tools"]
        .as_array()
        .expect("mcp-server.json should have 'tools' array");

    let expected_tool_names: Vec<String> = expected_tools_from_schema
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    // Make the actual MCP request
    let request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/list"
    });

    let response = mcp_request(&request);

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"]["tools"].is_array());

    let tools = response["result"]["tools"].as_array().unwrap();

    // Verify count matches mcp-server.json (no hard-coded magic number)
    assert_eq!(
        tools.len(),
        expected_tools_from_schema.len(),
        "Tool count mismatch: expected {} tools from mcp-server.json, got {}",
        expected_tools_from_schema.len(),
        tools.len()
    );

    // Verify all expected tools are present
    let actual_tool_names: Vec<String> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    for expected_name in &expected_tool_names {
        assert!(
            actual_tool_names.contains(expected_name),
            "Missing tool: {}",
            expected_name
        );
    }

    // Verify no unexpected tools were returned
    for actual_name in &actual_tool_names {
        assert!(
            expected_tool_names.contains(actual_name),
            "Unexpected tool returned: {}",
            actual_name
        );
    }
}

#[test]
#[serial]
fn test_invalid_json_returns_parse_error() {
    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path();

    let mut child = Command::new(get_binary_path())
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
#[serial]
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
#[serial]
fn test_task_search_with_fts5_query() {
    use std::thread;
    use std::time::Duration;

    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path();

    // Create .git directory to mark as project root
    std::fs::create_dir(project_path.join(".git")).unwrap();

    // Initialize project by changing to temp dir and running a command
    // This is necessary because initialize_project() doesn't respect INTENT_ENGINE_PROJECT_DIR
    let original_dir = std::env::current_dir().ok();
    std::env::set_current_dir(project_path).unwrap();

    let init_output = Command::new(get_binary_path())
        .args(["task", "add", "--name", "__init_test__"])
        .output()
        .unwrap();

    // Try to restore original directory (may fail if other tests changed it)
    // Note: Failure is acceptable here as we're cleaning up and other tests may have modified CWD
    if let Some(dir) = original_dir {
        let _ = std::env::set_current_dir(&dir); // Intentionally ignoring errors during cleanup
    }

    assert!(
        init_output.status.success(),
        "Failed to initialize project. stderr: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Create test tasks
    let output1 = Command::new(get_binary_path())
        .args(["task", "add", "--name", "Fix authentication bug"])
        .env("INTENT_ENGINE_PROJECT_DIR", project_path)
        .output()
        .unwrap();
    assert!(
        output1.status.success(),
        "Failed to create task 1. stderr: {}",
        String::from_utf8_lossy(&output1.stderr)
    );

    let output2 = Command::new(get_binary_path())
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

    let mut child = Command::new(get_binary_path())
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
#[serial]
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

#[test]
#[serial]
fn test_task_context_returns_family_tree() {
    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path();

    // Create .git directory to mark as project root
    std::fs::create_dir(project_path.join(".git")).unwrap();

    std::env::set_current_dir(project_path).unwrap();

    // Initialize project and create task hierarchy
    let output1 = Command::new(get_binary_path())
        .current_dir(project_path)
        .args(["task", "add", "--name", "Root task"])
        .output()
        .expect("Failed to execute task add for root task");
    assert!(
        output1.status.success(),
        "Failed to add root task. stderr: {}",
        String::from_utf8_lossy(&output1.stderr)
    );

    let output2 = Command::new(get_binary_path())
        .current_dir(project_path)
        .args(["task", "add", "--name", "Child task", "--parent", "1"])
        .output()
        .expect("Failed to execute task add for child task");
    assert!(
        output2.status.success(),
        "Failed to add child task. stderr: {}",
        String::from_utf8_lossy(&output2.stderr)
    );

    let output3 = Command::new(get_binary_path())
        .current_dir(project_path)
        .args(["task", "add", "--name", "Grandchild task", "--parent", "2"])
        .output()
        .expect("Failed to execute task add for grandchild task");
    assert!(
        output3.status.success(),
        "Failed to add grandchild task. stderr: {}",
        String::from_utf8_lossy(&output3.stderr)
    );

    // Request context for the child task (ID: 2)
    let request = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": {
            "name": "task_context",
            "arguments": {
                "task_id": 2
            }
        }
    });

    let mut child = Command::new(get_binary_path())
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
    let response_str = String::from_utf8_lossy(&output.stdout);
    let response: Value =
        serde_json::from_str(response_str.lines().next().unwrap_or("{}")).unwrap();

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 7);

    // Debug output for CI diagnosis
    eprintln!(
        "Response structure: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    assert!(
        response["result"]["content"].is_array(),
        "Expected content to be array. Full response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    let content_text = response["result"]["content"][0]["text"].as_str().unwrap();
    let context: Value = serde_json::from_str(content_text).unwrap();

    // Verify the task itself
    assert_eq!(context["task"]["id"], 2);
    assert_eq!(context["task"]["name"], "Child task");

    // Verify ancestors (should have parent: Root task)
    assert!(context["ancestors"].is_array());
    let ancestors = context["ancestors"].as_array().unwrap();
    assert_eq!(ancestors.len(), 1);
    assert_eq!(ancestors[0]["id"], 1);
    assert_eq!(ancestors[0]["name"], "Root task");

    // Verify children (should have grandchild)
    assert!(context["children"].is_array());
    let children = context["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["id"], 3);
    assert_eq!(children[0]["name"], "Grandchild task");

    // Verify siblings (should have none)
    assert!(context["siblings"].is_array());
    let siblings = context["siblings"].as_array().unwrap();
    assert_eq!(siblings.len(), 0);
}

#[test]
#[serial]
fn test_task_context_uses_current_task_when_no_id_provided() {
    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path();

    // Create .git directory to mark as project root
    std::fs::create_dir(project_path.join(".git")).unwrap();

    std::env::set_current_dir(project_path).unwrap();

    // Initialize project and create a task
    let add_output = Command::new(get_binary_path())
        .current_dir(project_path)
        .args(["task", "add", "--name", "Test task"])
        .output()
        .expect("Failed to execute task add");
    assert!(
        add_output.status.success(),
        "Failed to add task. stderr: {}",
        String::from_utf8_lossy(&add_output.stderr)
    );

    // Start the task (sets it as current)
    let start_output = Command::new(get_binary_path())
        .current_dir(project_path)
        .args(["task", "start", "1"])
        .output()
        .expect("Failed to execute task start");
    assert!(
        start_output.status.success(),
        "Failed to start task. stderr: {}",
        String::from_utf8_lossy(&start_output.stderr)
    );

    // Request context without providing task_id (should use current)
    let request = json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "tools/call",
        "params": {
            "name": "task_context",
            "arguments": {}
        }
    });

    let mut child = Command::new(get_binary_path())
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
    let response_str = String::from_utf8_lossy(&output.stdout);
    let response: Value =
        serde_json::from_str(response_str.lines().next().unwrap_or("{}")).unwrap();

    // Verify response
    assert_eq!(response["jsonrpc"], "2.0");

    // Debug output for CI diagnosis
    eprintln!(
        "Response structure: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    assert!(
        response["result"]["content"].is_array(),
        "Expected content to be array. Full response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    let content_text = response["result"]["content"][0]["text"].as_str().unwrap();
    let context: Value = serde_json::from_str(content_text).unwrap();

    // Should return context for task ID 1
    assert_eq!(context["task"]["id"], 1);
    assert_eq!(context["task"]["name"], "Test task");
}

#[test]
#[serial]
fn test_task_context_error_when_no_current_task_and_no_id() {
    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path();

    // Create .git directory to mark as project root
    std::fs::create_dir(project_path.join(".git")).unwrap();

    std::env::set_current_dir(project_path).unwrap();

    // Initialize project but don't create or start any tasks
    let add_output = Command::new(get_binary_path())
        .current_dir(project_path)
        .args(["task", "add", "--name", "Test task"])
        .output()
        .expect("Failed to execute task add");
    assert!(
        add_output.status.success(),
        "Failed to add task. stderr: {}",
        String::from_utf8_lossy(&add_output.stderr)
    );

    // Request context without task_id and without current task
    let request = json!({
        "jsonrpc": "2.0",
        "id": 9,
        "method": "tools/call",
        "params": {
            "name": "task_context",
            "arguments": {}
        }
    });

    let mut child = Command::new(get_binary_path())
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
    let response_str = String::from_utf8_lossy(&output.stdout);
    let response: Value =
        serde_json::from_str(response_str.lines().next().unwrap_or("{}")).unwrap();

    // Should return error
    assert!(response["error"].is_object());
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("No current task is set"));
}

#[test]
#[serial]
fn test_task_context_nonexistent_task() {
    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path();

    // Create .git directory to mark as project root
    std::fs::create_dir(project_path.join(".git")).unwrap();

    std::env::set_current_dir(project_path).unwrap();

    // Initialize project
    let add_output = Command::new(get_binary_path())
        .current_dir(project_path)
        .args(["task", "add", "--name", "Test task"])
        .output()
        .expect("Failed to execute task add");
    assert!(
        add_output.status.success(),
        "Failed to add task. stderr: {}",
        String::from_utf8_lossy(&add_output.stderr)
    );

    // Request context for nonexistent task
    let request = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "task_context",
            "arguments": {
                "task_id": 99999
            }
        }
    });

    let mut child = Command::new(get_binary_path())
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
    let response_str = String::from_utf8_lossy(&output.stdout);
    let response: Value =
        serde_json::from_str(response_str.lines().next().unwrap_or("{}")).unwrap();

    // Should return error
    assert!(response["error"].is_object());
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Failed to get task context"));
}
