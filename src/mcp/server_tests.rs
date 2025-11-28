//! Unit tests for MCP server
//!
//! These tests cover the JSON-RPC protocol handling and MCP tool handlers

use super::*;
use crate::tasks::TaskManager;
use crate::test_utils::test_helpers::TestContext;
use serde_json::json;
use serial_test::serial;
use std::path::PathBuf;

/// Guard that restores the original working directory when dropped
struct WorkingDirGuard {
    original_dir: PathBuf,
}

impl WorkingDirGuard {
    fn new() -> Self {
        Self {
            original_dir: std::env::current_dir().expect("Failed to get current directory"),
        }
    }
}

impl Drop for WorkingDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original_dir);
    }
}

/// Helper function to set up test environment with proper isolation
async fn setup_test_env() -> (TestContext, WorkingDirGuard) {
    let guard = WorkingDirGuard::new();
    let ctx = TestContext::new().await;
    // Set current directory to test project root
    std::env::set_current_dir(ctx.project_root())
        .expect("Failed to change to test project directory");
    (ctx, guard)
}

#[cfg(test)]
mod json_rpc_tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":null}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, Some(json!(1)));
        assert_eq!(req.method, "ping");
    }

    #[test]
    fn test_json_rpc_request_without_params() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.method, "ping");
        assert!(req.params.is_none());
    }

    #[test]
    fn test_json_rpc_request_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"initialized"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.method, "initialized");
        assert!(req.id.is_none());
    }

    #[test]
    fn test_json_rpc_response_serialization() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            result: Some(json!({"status": "ok"})),
            error: None,
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"result\""));
    }

    #[test]
    fn test_json_rpc_error_response_serialization() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Invalid Request".to_string(),
            }),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("-32600"));
        assert!(json.contains("Invalid Request"));
    }

    #[test]
    fn test_tool_call_params_deserialization() {
        let json = r#"{"name":"task_add","arguments":{"name":"Test"}}"#;
        let params: ToolCallParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.name, "task_add");
        assert_eq!(
            params.arguments.get("name").unwrap().as_str().unwrap(),
            "Test"
        );
    }
}

#[cfg(test)]
mod handler_tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_initialize() {
        let result = handle_initialize(None);
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value.get("protocolVersion").is_some());
        assert!(value.get("capabilities").is_some());
        assert!(value.get("serverInfo").is_some());

        let server_info = value.get("serverInfo").unwrap();
        assert_eq!(server_info.get("name").unwrap(), "intent-engine");
    }

    #[tokio::test]
    async fn test_handle_tools_list() {
        let result = handle_tools_list();
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value.get("tools").is_some());

        let tools = value.get("tools").unwrap().as_array().unwrap();
        assert!(!tools.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_add_success() {
        let _ctx = setup_test_env().await;

        let args = json!({
            "name": "Test Task",
            "spec": "Test spec"
        });

        let result = handle_task_add(args).await;
        assert!(result.is_ok());

        let task = result.unwrap();
        assert_eq!(task.get("name").unwrap(), "Test Task");
        assert_eq!(task.get("spec").unwrap(), "Test spec");
    }

    #[tokio::test]
    async fn test_handle_task_add_missing_name() {
        let args = json!({
            "spec": "Test spec"
        });

        let result = handle_task_add(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Missing required parameter: name"));
    }

    #[tokio::test]
    async fn test_handle_task_add_null_name() {
        let args = json!({
            "name": null,
            "spec": "Test spec"
        });

        let result = handle_task_add(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err, "Parameter 'name' cannot be null");
    }

    #[tokio::test]
    async fn test_handle_task_add_empty_name() {
        let args = json!({
            "name": "",
            "spec": "Test spec"
        });

        let result = handle_task_add(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err, "Parameter 'name' cannot be empty");
    }

    #[tokio::test]
    async fn test_handle_task_add_whitespace_only_name() {
        let args = json!({
            "name": "   ",
            "spec": "Test spec"
        });

        let result = handle_task_add(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err, "Parameter 'name' cannot be empty");
    }

    #[tokio::test]
    async fn test_handle_task_add_wrong_type_name() {
        let args = json!({
            "name": 123,
            "spec": "Test spec"
        });

        let result = handle_task_add(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Parameter 'name' must be a string"));
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_add_with_parent() {
        let _ctx = setup_test_env().await;

        // First create parent task
        let parent_args = json!({"name": "Parent Task"});
        let parent_result = handle_task_add(parent_args).await.unwrap();
        let parent_id = parent_result.get("id").unwrap().as_i64().unwrap();

        // Then create child task
        let child_args = json!({
            "name": "Child Task",
            "parent_id": parent_id
        });

        let result = handle_task_add(child_args).await;
        assert!(result.is_ok());

        let child = result.unwrap();
        assert_eq!(child.get("parent_id").unwrap().as_i64().unwrap(), parent_id);
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_start_success() {
        let (ctx, _guard) = setup_test_env().await;

        // Create a task first
        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();

        let args = json!({
            "task_id": task.id,
            "with_events": false
        });

        let result = handle_task_start(args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.get("id").unwrap().as_i64().unwrap(), task.id);
        assert_eq!(response.get("status").unwrap(), "doing");
    }

    #[tokio::test]
    async fn test_handle_task_start_missing_task_id() {
        let args = json!({
            "with_events": false
        });

        let result = handle_task_start(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("task_id"));
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_start_nonexistent_task() {
        let _ctx = setup_test_env().await;

        let args = json!({
            "task_id": 99999,
            "with_events": false
        });

        let result = handle_task_start(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_done_success() {
        let (ctx, _guard) = setup_test_env().await;

        // Create and start a task
        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();
        task_mgr.start_task(task.id, false).await.unwrap();

        let args = json!({});
        let result = handle_task_done(args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(
            response
                .get("completed_task")
                .unwrap()
                .get("status")
                .unwrap(),
            "done"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_done_with_task_id() {
        let (ctx, _guard) = setup_test_env().await;

        // Create and start a task
        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();
        task_mgr.start_task(task.id, false).await.unwrap();

        let args = json!({
            "task_id": task.id
        });

        let result = handle_task_done(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_list_all() {
        let (ctx, _guard) = setup_test_env().await;

        // Create some tasks
        let task_mgr = TaskManager::new(ctx.pool());
        task_mgr.add_task("Task 1", None, None).await.unwrap();
        task_mgr.add_task("Task 2", None, None).await.unwrap();

        let args = json!({});
        let result = handle_task_list(args).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        let tasks = value.get("tasks").unwrap().as_array().unwrap();
        assert!(tasks.len() >= 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_list_by_status() {
        let (ctx, _guard) = setup_test_env().await;

        // Create tasks with different statuses
        let task_mgr = TaskManager::new(ctx.pool());
        task_mgr.add_task("Todo Task", None, None).await.unwrap();
        let doing = task_mgr.add_task("Doing Task", None, None).await.unwrap();
        task_mgr.start_task(doing.id, false).await.unwrap();

        let args = json!({
            "status": "doing"
        });

        let result = handle_task_list(args).await;
        assert!(result.is_ok(), "task_list should succeed");

        let value = result.unwrap();
        let tasks = value.get("tasks").unwrap().as_array().unwrap();
        assert!(!tasks.is_empty());
        assert_eq!(tasks[0].get("status").unwrap(), "doing");
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_get_success() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr
            .add_task("Test Task", Some("Test spec"), None)
            .await
            .unwrap();

        let args = json!({
            "task_id": task.id,
            "with_events": false
        });

        let result = handle_task_get(args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.get("id").unwrap().as_i64().unwrap(), task.id);
        assert_eq!(response.get("name").unwrap(), "Test Task");
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_get_with_events() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();

        let args = json!({
            "task_id": task.id,
            "with_events": true
        });

        let result = handle_task_get(args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        // TaskWithEvents flattens the task fields to top level
        assert!(response.get("id").is_some());
        assert!(response.get("name").is_some());
        assert!(response.get("events_summary").is_some());
    }

    /// Test for task_update with name and spec changes
    ///
    /// FIXED: Previously failed due to SQL injection risk from string concatenation.
    /// Now uses sqlx::QueryBuilder with parameterized queries for safe SQL construction.
    /// See Task #6 and #7 in intent-engine for investigation details.
    #[tokio::test]
    #[serial]
    async fn test_handle_task_update_success() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr
            .add_task("Original Name", None, None)
            .await
            .unwrap();

        let args = json!({
            "task_id": task.id,
            "name": "Updated Name",
            "spec": "Updated spec"
        });

        let result = handle_task_update(args).await;
        assert!(result.is_ok(), "task_update should succeed");

        let updated = result.unwrap();
        assert_eq!(updated.get("name").unwrap(), "Updated Name");
        assert_eq!(updated.get("spec").unwrap(), "Updated spec");
    }

    #[tokio::test]
    async fn test_handle_task_update_missing_task_id() {
        let args = json!({
            "name": "Updated Name"
        });

        let result = handle_task_update(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("task_id"));
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_update_priority() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();

        let args = json!({
            "task_id": task.id,
            "priority": "high"
        });

        let result = handle_task_update(args).await;
        assert!(result.is_ok());

        let updated = result.unwrap();
        assert_eq!(updated.get("priority").unwrap().as_i64().unwrap(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_update_invalid_priority() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();

        let args = json!({
            "task_id": task.id,
            "priority": "invalid"
        });

        let result = handle_task_update(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("priority"));
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_delete_success() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();

        let args = json!({
            "task_id": task.id
        });

        let result = handle_task_delete(args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.get("success").unwrap(), true);
        assert_eq!(
            response.get("deleted_task_id").unwrap().as_i64().unwrap(),
            task.id
        );
    }

    #[tokio::test]
    async fn test_handle_task_delete_missing_task_id() {
        let args = json!({});
        let result = handle_task_delete(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("task_id"));
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_spawn_subtask_success() {
        let (ctx, _guard) = setup_test_env().await;

        // Create and start parent task
        let task_mgr = TaskManager::new(ctx.pool());
        let parent = task_mgr.add_task("Parent Task", None, None).await.unwrap();
        task_mgr.start_task(parent.id, false).await.unwrap();

        let args = json!({
            "name": "Subtask",
            "spec": "Subtask spec"
        });

        let result = handle_task_spawn_subtask(args).await;
        assert!(result.is_ok());

        let subtask_info = result.unwrap();
        let subtask = subtask_info.get("subtask").unwrap();
        assert_eq!(subtask.get("name").unwrap(), "Subtask");
        assert_eq!(
            subtask.get("parent_id").unwrap().as_i64().unwrap(),
            parent.id
        );
    }

    #[tokio::test]
    async fn test_handle_task_spawn_subtask_missing_name() {
        let args = json!({
            "spec": "Test spec"
        });

        let result = handle_task_spawn_subtask(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("name"));
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_pick_next_success() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        task_mgr.add_task("Todo Task", None, None).await.unwrap();

        let args = json!({});
        let result = handle_task_pick_next(args).await;
        assert!(result.is_ok(), "task_pick_next should succeed");

        let response = result.unwrap();
        // PickNextResponse has suggestion_type, task, reason_code, message
        assert!(response.get("suggestion_type").is_some());
        assert!(response.get("task").is_some() || response.get("message").is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_event_add_success() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();
        task_mgr.start_task(task.id, false).await.unwrap();

        let args = json!({
            "event_type": "decision",
            "data": "Test decision"
        });

        let result = handle_event_add(args).await;
        assert!(result.is_ok());

        let event = result.unwrap();
        assert_eq!(event.get("log_type").unwrap(), "decision");
        assert_eq!(event.get("discussion_data").unwrap(), "Test decision");
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_event_add_with_task_id() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();

        let args = json!({
            "task_id": task.id,
            "event_type": "note",
            "data": "Test note"
        });

        let result = handle_event_add(args).await;
        assert!(result.is_ok());

        let event = result.unwrap();
        assert_eq!(event.get("task_id").unwrap().as_i64().unwrap(), task.id);
    }

    #[tokio::test]
    async fn test_handle_event_add_missing_event_type() {
        let args = json!({
            "data": "Test data"
        });

        let result = handle_event_add(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("event_type"));
    }

    #[tokio::test]
    async fn test_handle_event_add_missing_data() {
        let args = json!({
            "event_type": "note"
        });

        let result = handle_event_add(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("data"));
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_event_list_success() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();

        // Add some events
        let event_mgr = crate::events::EventManager::new(ctx.pool());
        event_mgr
            .add_event(task.id, "decision", "Decision 1")
            .await
            .unwrap();
        event_mgr
            .add_event(task.id, "note", "Note 1")
            .await
            .unwrap();

        let args = json!({
            "task_id": task.id
        });

        let result = handle_event_list(args).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        let events = value.as_array().unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_event_list_with_filters() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();

        // Add events
        let event_mgr = crate::events::EventManager::new(ctx.pool());
        event_mgr
            .add_event(task.id, "decision", "Decision 1")
            .await
            .unwrap();
        event_mgr
            .add_event(task.id, "note", "Note 1")
            .await
            .unwrap();

        let args = json!({
            "task_id": task.id,
            "type": "decision",
            "limit": 10
        });

        let result = handle_event_list(args).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        let events = value.as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].get("log_type").unwrap(), "decision");
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_event_list_global() {
        let (ctx, _guard) = setup_test_env().await;
        let task_mgr = TaskManager::new(ctx.pool());
        let event_mgr = EventManager::new(ctx.pool());

        // Create tasks and events
        let task1 = task_mgr.add_task("Task 1", None, None).await.unwrap();
        let task2 = task_mgr.add_task("Task 2", None, None).await.unwrap();
        event_mgr
            .add_event(task1.id, "decision", "Decision 1")
            .await
            .unwrap();
        event_mgr
            .add_event(task2.id, "blocker", "Blocker 1")
            .await
            .unwrap();

        // Query global events (no task_id)
        let args = json!({});
        let result = handle_event_list(args).await;
        assert!(result.is_ok());

        let events = result.unwrap();
        let events_array = events.as_array().unwrap();
        assert!(events_array.len() >= 2); // Should contain events from both tasks
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_current_task_get_no_current() {
        let _ctx = setup_test_env().await;

        let args = json!({});
        let result = handle_current_task_get(args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.get("current_task_id").is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_current_task_get_with_current() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();
        task_mgr.start_task(task.id, false).await.unwrap();

        let args = json!({});
        let result = handle_current_task_get(args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(
            response.get("current_task_id").unwrap().as_i64().unwrap(),
            task.id
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_report_generate_success() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        task_mgr.add_task("Task 1", None, None).await.unwrap();
        task_mgr.add_task("Task 2", None, None).await.unwrap();

        let args = json!({
            "summary_only": true
        });

        let result = handle_report_generate(args).await;
        assert!(result.is_ok());

        let report = result.unwrap();
        assert!(report.get("summary").is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_report_generate_with_filters() {
        let _ctx = setup_test_env().await;

        let args = json!({
            "status": "todo",
            "since": "7d",
            "summary_only": false
        });

        let result = handle_report_generate(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_context_success() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();

        let args = json!({
            "task_id": task.id
        });

        let result = handle_task_context(args).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert!(context.get("task").is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_context_uses_current_task() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();
        task_mgr.start_task(task.id, false).await.unwrap();

        // No task_id in args, should use current task
        let args = json!({});
        let result = handle_task_context(args).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(
            context
                .get("task")
                .unwrap()
                .get("id")
                .unwrap()
                .as_i64()
                .unwrap(),
            task.id
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_add_dependency_success() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task1 = task_mgr.add_task("Task 1", None, None).await.unwrap();
        let task2 = task_mgr.add_task("Task 2", None, None).await.unwrap();

        let args = json!({
            "blocking_task_id": task1.id,
            "blocked_task_id": task2.id
        });

        let result = handle_task_add_dependency(args).await;
        assert!(result.is_ok());

        let dependency = result.unwrap();
        assert_eq!(
            dependency
                .get("blocking_task_id")
                .unwrap()
                .as_i64()
                .unwrap(),
            task1.id
        );
        assert_eq!(
            dependency.get("blocked_task_id").unwrap().as_i64().unwrap(),
            task2.id
        );
    }

    #[tokio::test]
    async fn test_handle_task_add_dependency_missing_params() {
        let args = json!({
            "blocking_task_id": 1
        });

        let result = handle_task_add_dependency(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("blocked_task_id"));
    }
}

#[cfg(test)]
mod request_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_request_initialize() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: None,
        };

        let response = handle_request(request).await;
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_handle_request_ping() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "ping".to_string(),
            params: None,
        };

        let response = handle_request(request).await;
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_handle_request_tools_list() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = handle_request(request).await;
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_handle_request_invalid_jsonrpc_version() {
        let request = JsonRpcRequest {
            jsonrpc: "1.0".to_string(),
            id: Some(json!(1)),
            method: "ping".to_string(),
            params: None,
        };

        let response = handle_request(request).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32600);
    }

    #[tokio::test]
    async fn test_handle_request_method_not_found() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "unknown_method".to_string(),
            params: None,
        };

        let response = handle_request(request).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32000);
        assert!(error.message.contains("Method not found"));
    }

    #[tokio::test]
    async fn test_handle_notification_initialized() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "initialized".to_string(),
            params: None,
        };

        // Should not panic
        handle_notification(&request).await;
    }

    #[tokio::test]
    async fn test_handle_notification_cancelled() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "notifications/cancelled".to_string(),
            params: None,
        };

        // Should not panic
        handle_notification(&request).await;
    }

    #[tokio::test]
    async fn test_handle_notification_unknown() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "unknown_notification".to_string(),
            params: None,
        };

        // Should not panic
        handle_notification(&request).await;
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_tool_call_unknown_tool() {
        let params = json!({
            "name": "unknown_tool",
            "arguments": {}
        });

        let result = handle_tool_call(Some(params)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_handle_tool_call_invalid_params() {
        let result = handle_tool_call(Some(json!("invalid"))).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_tool_call_empty_params() {
        let result = handle_tool_call(None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_list_parent_null() {
        let (ctx, _guard) = setup_test_env().await;

        // Create top-level and child tasks
        let task_mgr = TaskManager::new(ctx.pool());
        task_mgr.add_task("Top Level", None, None).await.unwrap();
        let parent = task_mgr.add_task("Parent", None, None).await.unwrap();
        task_mgr
            .add_task("Child", None, Some(parent.id))
            .await
            .unwrap();

        // Query for top-level tasks only
        let args = json!({
            "parent": "null"
        });

        let result = handle_task_list(args).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        let tasks = value.get("tasks").unwrap().as_array().unwrap();
        // Should only return top-level tasks
        for task in tasks {
            assert!(task.get("parent_id").is_none() || task.get("parent_id").unwrap().is_null());
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_task_start_default_with_events() {
        let (ctx, _guard) = setup_test_env().await;

        let task_mgr = TaskManager::new(ctx.pool());
        let task = task_mgr.add_task("Test Task", None, None).await.unwrap();

        // with_events should default to true
        let args = json!({
            "task_id": task.id
        });

        let result = handle_task_start(args).await;
        assert!(result.is_ok());
    }
}
