use assert_cmd::cargo;
use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to get the binary path
fn intent_engine_cmd() -> Command {
    Command::new(cargo::cargo_bin!("intent-engine"))
}

/// Helper to create a test project
fn setup_test_project() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("intent.db");

    // Initialize project by running doctor command
    intent_engine_cmd()
        .current_dir(temp_dir.path())
        .arg("doctor")
        .assert()
        .success();

    (temp_dir, db_path)
}

/// Helper to add a task and return its ID
fn add_task(dir: &PathBuf, name: &str) -> i64 {
    let mut cmd = intent_engine_cmd();
    cmd.current_dir(dir)
        .arg("task")
        .arg("add")
        .arg("--name")
        .arg(name);

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();
    json["id"].as_i64().unwrap()
}

/// Helper to add a dependency
fn add_dependency(dir: &PathBuf, blocked_task_id: i64, blocking_task_id: i64) {
    intent_engine_cmd()
        .current_dir(dir)
        .arg("task")
        .arg("depends-on")
        .arg(blocked_task_id.to_string())
        .arg(blocking_task_id.to_string())
        .assert()
        .success();
}

#[test]
fn test_context_no_dependencies() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create a task with no dependencies
    let task_id = add_task(&dir.to_path_buf(), "Task A");

    // Get task context via MCP
    let context_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_context", "arguments": {{"task_id": {}}}}}}}"#,
        task_id
    );

    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("mcp-server")
        .write_stdin(context_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let dependencies = &json["result"]["content"][0]["text"];
    let context: Value = serde_json::from_str(dependencies.as_str().unwrap()).unwrap();

    assert_eq!(
        context["dependencies"]["blocking_tasks"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        context["dependencies"]["blocked_by_tasks"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn test_context_with_blocking_tasks() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create tasks
    let task_a = add_task(&dir.to_path_buf(), "Task A - Blocker");
    let task_b = add_task(&dir.to_path_buf(), "Task B - Depends on A");

    // Make task_b depend on task_a
    add_dependency(&dir.to_path_buf(), task_b, task_a);

    // Get context for task_b
    let context_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_context", "arguments": {{"task_id": {}}}}}}}"#,
        task_b
    );

    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("mcp-server")
        .write_stdin(context_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let dependencies = &json["result"]["content"][0]["text"];
    let context: Value = serde_json::from_str(dependencies.as_str().unwrap()).unwrap();

    // Task B should have task A as a blocking task
    let blocking_tasks = context["dependencies"]["blocking_tasks"]
        .as_array()
        .unwrap();
    assert_eq!(blocking_tasks.len(), 1);
    assert_eq!(blocking_tasks[0]["id"].as_i64().unwrap(), task_a);
    assert_eq!(
        blocking_tasks[0]["name"].as_str().unwrap(),
        "Task A - Blocker"
    );

    // Task B should have no blocked_by tasks
    assert_eq!(
        context["dependencies"]["blocked_by_tasks"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn test_context_with_blocked_by_tasks() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create tasks
    let task_a = add_task(&dir.to_path_buf(), "Task A - Blocker");
    let task_b = add_task(&dir.to_path_buf(), "Task B - Depends on A");

    // Make task_b depend on task_a
    add_dependency(&dir.to_path_buf(), task_b, task_a);

    // Get context for task_a
    let context_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_context", "arguments": {{"task_id": {}}}}}}}"#,
        task_a
    );

    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("mcp-server")
        .write_stdin(context_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let dependencies = &json["result"]["content"][0]["text"];
    let context: Value = serde_json::from_str(dependencies.as_str().unwrap()).unwrap();

    // Task A should have no blocking tasks
    assert_eq!(
        context["dependencies"]["blocking_tasks"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    // Task A should have task B as a blocked_by task
    let blocked_by_tasks = context["dependencies"]["blocked_by_tasks"]
        .as_array()
        .unwrap();
    assert_eq!(blocked_by_tasks.len(), 1);
    assert_eq!(blocked_by_tasks[0]["id"].as_i64().unwrap(), task_b);
    assert_eq!(
        blocked_by_tasks[0]["name"].as_str().unwrap(),
        "Task B - Depends on A"
    );
}

#[test]
fn test_context_with_multiple_dependencies() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create tasks
    let task_a = add_task(&dir.to_path_buf(), "Task A");
    let task_b = add_task(&dir.to_path_buf(), "Task B");
    let task_c = add_task(&dir.to_path_buf(), "Task C - Depends on A and B");

    // Make task_c depend on both task_a and task_b
    add_dependency(&dir.to_path_buf(), task_c, task_a);
    add_dependency(&dir.to_path_buf(), task_c, task_b);

    // Get context for task_c
    let context_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_context", "arguments": {{"task_id": {}}}}}}}"#,
        task_c
    );

    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("mcp-server")
        .write_stdin(context_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let dependencies = &json["result"]["content"][0]["text"];
    let context: Value = serde_json::from_str(dependencies.as_str().unwrap()).unwrap();

    // Task C should have both A and B as blocking tasks
    let blocking_tasks = context["dependencies"]["blocking_tasks"]
        .as_array()
        .unwrap();
    assert_eq!(blocking_tasks.len(), 2);

    let blocking_ids: Vec<i64> = blocking_tasks
        .iter()
        .map(|t| t["id"].as_i64().unwrap())
        .collect();
    assert!(blocking_ids.contains(&task_a));
    assert!(blocking_ids.contains(&task_b));
}

#[test]
fn test_context_bidirectional_dependencies() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create a chain: A -> B -> C (A blocks B, B blocks C)
    let task_a = add_task(&dir.to_path_buf(), "Task A");
    let task_b = add_task(&dir.to_path_buf(), "Task B");
    let task_c = add_task(&dir.to_path_buf(), "Task C");

    add_dependency(&dir.to_path_buf(), task_b, task_a);
    add_dependency(&dir.to_path_buf(), task_c, task_b);

    // Get context for task_b (middle of chain)
    let context_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_context", "arguments": {{"task_id": {}}}}}}}"#,
        task_b
    );

    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("mcp-server")
        .write_stdin(context_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let dependencies = &json["result"]["content"][0]["text"];
    let context: Value = serde_json::from_str(dependencies.as_str().unwrap()).unwrap();

    // Task B should have A as blocking task
    let blocking_tasks = context["dependencies"]["blocking_tasks"]
        .as_array()
        .unwrap();
    assert_eq!(blocking_tasks.len(), 1);
    assert_eq!(blocking_tasks[0]["id"].as_i64().unwrap(), task_a);

    // Task B should have C as blocked_by task
    let blocked_by_tasks = context["dependencies"]["blocked_by_tasks"]
        .as_array()
        .unwrap();
    assert_eq!(blocked_by_tasks.len(), 1);
    assert_eq!(blocked_by_tasks[0]["id"].as_i64().unwrap(), task_c);
}

#[test]
fn test_context_dependencies_include_task_details() {
    let (temp_dir, _db_path) = setup_test_project();
    let dir = temp_dir.path();

    // Create tasks
    let task_a = add_task(&dir.to_path_buf(), "Task A with specific name");
    let task_b = add_task(&dir.to_path_buf(), "Task B");

    // Make task_b depend on task_a
    add_dependency(&dir.to_path_buf(), task_b, task_a);

    // Get context for task_b
    let context_json = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "task_context", "arguments": {{"task_id": {}}}}}}}"#,
        task_b
    );

    let output = intent_engine_cmd()
        .current_dir(dir)
        .arg("mcp-server")
        .write_stdin(context_json)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap();

    let dependencies = &json["result"]["content"][0]["text"];
    let context: Value = serde_json::from_str(dependencies.as_str().unwrap()).unwrap();

    // Verify that blocking task includes full task details
    let blocking_task = &context["dependencies"]["blocking_tasks"][0];
    assert_eq!(blocking_task["id"].as_i64().unwrap(), task_a);
    assert_eq!(
        blocking_task["name"].as_str().unwrap(),
        "Task A with specific name"
    );
    assert_eq!(blocking_task["status"].as_str().unwrap(), "todo");
    assert!(blocking_task["first_todo_at"].is_string());
}
