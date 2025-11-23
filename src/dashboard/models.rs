use serde::{Deserialize, Serialize};
use serde_json::Value;

/// API response wrapper
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

/// API error response
#[derive(Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// Create task request
#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<i64>,
}

/// Update task request
#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Create event request
#[derive(Deserialize)]
pub struct CreateEventRequest {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: String,
}

/// Update event request
#[derive(Deserialize)]
pub struct UpdateEventRequest {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Spawn subtask request
#[derive(Deserialize)]
pub struct SpawnSubtaskRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
}

/// Query parameters for task list
#[derive(Deserialize)]
pub struct TaskListQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// Query parameters for event list
#[derive(Deserialize)]
pub struct EventListQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

/// Switch project request
#[derive(Deserialize)]
pub struct SwitchProjectRequest {
    pub project_path: String,
}

/// Query parameters for search
#[derive(Deserialize)]
pub struct SearchQuery {
    pub query: String,
    #[serde(default = "default_true")]
    pub include_tasks: bool,
    #[serde(default = "default_true")]
    pub include_events: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task_request_deserialization() {
        let json = r#"{"name":"Test Task","spec":"Test spec","priority":1}"#;
        let req: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Test Task");
        assert_eq!(req.spec, Some("Test spec".to_string()));
        assert_eq!(req.priority, Some(1));
    }

    #[test]
    fn test_api_response_serialization() {
        let response = ApiResponse { data: "test" };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"data\""));
    }

    #[test]
    fn test_api_error_serialization() {
        let error = ApiError {
            code: "TEST_ERROR".to_string(),
            message: "Test message".to_string(),
            details: None,
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("TEST_ERROR"));
        assert!(!json.contains("details"));
    }

    #[test]
    fn test_search_query_defaults() {
        let json = r#"{"query":"test"}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.query, "test");
        assert!(query.include_tasks);
        assert!(query.include_events);
    }
}
