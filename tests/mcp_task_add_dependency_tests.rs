mod common;

use serde_json::Value;
use std::path::PathBuf;

/// Helper to create a test project for MCP tests
fn setup_test_project() -> (tempfile::TempDir, PathBuf) {
    // For MCP tests, we need to use the current project directory which is already
    // properly initialized. We still return a temp_dir to keep the interface compatible.
    let temp_dir = tempfile::TempDir::new().unwrap();
    let project_dir = common::current_project_dir();
    let db_path = project_dir.join(".intent-engine").join("project.db");

    (temp_dir, db_path)
}

/// Helper to add a task via MCP and return its ID
fn add_task_mcp(_dir: &PathBuf, name: &str) -> i64 {
    let project_dir = common::current_project_dir();
    let add_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_add", "arguments": {{"name": "{}"}}}}}}"#,
        name
    );

    let output = common::ie_command_with_project_dir(&project_dir)
        .arg("mcp-server")
        .write_stdin(add_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    let task = &json["result"]["content"][0]["text"];
    let task_data: Value = serde_json::from_str(task.as_str().unwrap()).unwrap();
    task_data["id"].as_i64().unwrap()
}

#[test]
#[ignore = "TODO: Fix database isolation - tests conflict with project DB"]
fn test_mcp_add_dependency_success() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create two tasks
    let task_a = add_task_mcp(&dir.to_path_buf(), "Task A");
    let task_b = add_task_mcp(&dir.to_path_buf(), "Task B");

    // Add dependency via MCP
    let dep_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_add_dependency", "arguments": {{"blocked_task_id": {}, "blocking_task_id": {}}}}}}}"#,
        task_b, task_a
    );

    let output = common::ie_command_with_project_dir(&common::current_project_dir())
        .arg("mcp-server")
        .write_stdin(dep_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let dependency = &json["result"]["content"][0]["text"];
    let dep_data: Value = serde_json::from_str(dependency.as_str().unwrap()).unwrap();

    assert_eq!(dep_data["blocking_task_id"].as_i64().unwrap(), task_a);
    assert_eq!(dep_data["blocked_task_id"].as_i64().unwrap(), task_b);
    assert!(dep_data["id"].is_number());
    assert!(dep_data["created_at"].is_string());
}

#[test]
#[ignore = "TODO: Fix database isolation - tests conflict with project DB"]
fn test_mcp_add_dependency_circular_detection() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create two tasks
    let task_a = add_task_mcp(&dir.to_path_buf(), "Task A");
    let task_b = add_task_mcp(&dir.to_path_buf(), "Task B");

    // Add A -> B dependency
    let dep1_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_add_dependency", "arguments": {{"blocked_task_id": {}, "blocking_task_id": {}}}}}}}"#,
        task_b, task_a
    );

    common::ie_command_with_project_dir(&common::current_project_dir())
        .arg("mcp-server")
        .write_stdin(dep1_json)
        .assert()
        .success();

    // Try to add B -> A dependency (would create cycle)
    let dep2_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_add_dependency", "arguments": {{"blocked_task_id": {}, "blocking_task_id": {}}}}}}}"#,
        task_a, task_b
    );

    let output = common::ie_command_with_project_dir(&common::current_project_dir())
        .arg("mcp-server")
        .write_stdin(dep2_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Should contain error
    let error = &json["error"];
    assert!(error.is_object());
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("Circular dependency detected"));
}

#[test]
#[ignore = "TODO: Fix database isolation - tests conflict with project DB"]
fn test_mcp_add_dependency_missing_parameters() {
    let (temp_dir, _db_path) = setup_test_project();
    let _dir = temp_dir.path();

    // Try without blocked_task_id
    let dep_json = r#"{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "task_add_dependency", "arguments": {"blocking_task_id": 1}}}"#;

    let output = common::ie_command_with_project_dir(&common::current_project_dir())
        .arg("mcp-server")
        .write_stdin(dep_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Should contain error about missing parameter
    let error = &json["error"];
    assert!(error.is_object());
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("blocked_task_id"));
}

#[test]
#[ignore = "TODO: Fix database isolation - tests conflict with project DB"]
fn test_mcp_add_dependency_nonexistent_task() {
    let (temp_dir, _db_path) = setup_test_project();
    let _dir = temp_dir.path();

    // Try to add dependency with non-existent task IDs
    let dep_json = r#"{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "task_add_dependency", "arguments": {"blocked_task_id": 999, "blocking_task_id": 998}}}"#;

    let output = common::ie_command_with_project_dir(&common::current_project_dir())
        .arg("mcp-server")
        .write_stdin(dep_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Should contain error about task not found
    let error = &json["error"];
    assert!(error.is_object());
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("Task not found"));
}

#[test]
#[ignore = "TODO: Fix database isolation - tests conflict with project DB"]
fn test_mcp_add_dependency_affects_task_start() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create two tasks
    let task_a = add_task_mcp(&dir.to_path_buf(), "Task A");
    let task_b = add_task_mcp(&dir.to_path_buf(), "Task B");

    // Add B depends on A
    let dep_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_add_dependency", "arguments": {{"blocked_task_id": {}, "blocking_task_id": {}}}}}}}"#,
        task_b, task_a
    );

    common::ie_command_with_project_dir(&common::current_project_dir())
        .arg("mcp-server")
        .write_stdin(dep_json)
        .assert()
        .success();

    // Try to start task B (should fail because A is not done)
    let start_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_start", "arguments": {{"task_id": {}}}}}}}"#,
        task_b
    );

    let output = common::ie_command_with_project_dir(&common::current_project_dir())
        .arg("mcp-server")
        .write_stdin(start_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    // Should contain error about task being blocked
    let error = &json["error"];
    assert!(error.is_object());
    assert!(error["message"].as_str().unwrap().contains("blocked"));
}

#[test]
#[ignore = "TODO: Fix database isolation - tests conflict with project DB"]
fn test_mcp_add_dependency_shows_in_context() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create two tasks
    let task_a = add_task_mcp(&dir.to_path_buf(), "Task A");
    let task_b = add_task_mcp(&dir.to_path_buf(), "Task B");

    // Add B depends on A
    let dep_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_add_dependency", "arguments": {{"blocked_task_id": {}, "blocking_task_id": {}}}}}}}"#,
        task_b, task_a
    );

    common::ie_command_with_project_dir(&common::current_project_dir())
        .arg("mcp-server")
        .write_stdin(dep_json)
        .assert()
        .success();

    // Get context for task B
    let context_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_context", "arguments": {{"task_id": {}}}}}}}"#,
        task_b
    );

    let output = common::ie_command_with_project_dir(&common::current_project_dir())
        .arg("mcp-server")
        .write_stdin(context_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let context = &json["result"]["content"][0]["text"];
    let context_data: Value = serde_json::from_str(context.as_str().unwrap()).unwrap();

    // Verify task B shows task A as a blocking task
    let blocking_tasks = context_data["dependencies"]["blocking_tasks"]
        .as_array()
        .unwrap();
    assert_eq!(blocking_tasks.len(), 1);
    assert_eq!(blocking_tasks[0]["id"].as_i64().unwrap(), task_a);
}
