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
    #[serde(alias = "type", alias = "event_type")]
    pub event_type: String,
    pub data: String,
}

/// Update event request
#[derive(Deserialize)]
pub struct UpdateEventRequest {
    #[serde(
        alias = "type",
        alias = "event_type",
        skip_serializing_if = "Option::is_none"
    )]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
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
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total_count: i64,
    pub has_more: bool,
    pub limit: i64,
    pub offset: i64,
}

use crate::db::models::SearchResult;

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_tasks: i64,
    pub total_events: i64,
    pub has_more: bool,
    pub limit: i64,
    pub offset: i64,
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

    #[test]
    fn test_update_task_request_deserialization() {
        let json = r#"{"name":"Updated","spec":"New spec","status":"done"}"#;
        let req: UpdateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, Some("Updated".to_string()));
        assert_eq!(req.spec, Some("New spec".to_string()));
        assert_eq!(req.status, Some("done".to_string()));
    }

    #[test]
    fn test_create_event_request_deserialization() {
        let json = r#"{"type":"decision","data":"Made a decision"}"#;
        let req: CreateEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.event_type, "decision");
        assert_eq!(req.data, "Made a decision");
    }

    #[test]
    fn test_update_event_request_deserialization() {
        let json = r#"{"type":"milestone","data":"Updated data"}"#;
        let req: UpdateEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.event_type, Some("milestone".to_string()));
        assert_eq!(req.data, Some("Updated data".to_string()));
    }

    #[test]
    fn test_spawn_subtask_request_deserialization() {
        let json = r#"{"name":"Subtask","spec":"Subtask spec"}"#;
        let req: SpawnSubtaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Subtask");
        assert_eq!(req.spec, Some("Subtask spec".to_string()));
    }

    #[test]
    fn test_task_list_query_deserialization() {
        let json = r#"{"status":"doing","parent":"null"}"#;
        let query: TaskListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.status, Some("doing".to_string()));
        assert_eq!(query.parent, Some("null".to_string()));
    }

    #[test]
    fn test_task_list_query_with_pagination() {
        let json = r#"{"status":"doing","sort_by":"priority","limit":50,"offset":10}"#;
        let query: TaskListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.status, Some("doing".to_string()));
        assert_eq!(query.sort_by, Some("priority".to_string()));
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(10));
    }

    #[test]
    fn test_event_list_query_deserialization() {
        let json = r#"{"event_type":"decision","since":"7d","limit":10}"#;
        let query: EventListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.event_type, Some("decision".to_string()));
        assert_eq!(query.since, Some("7d".to_string()));
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn test_switch_project_request_deserialization() {
        let json = r#"{"project_path":"/path/to/project"}"#;
        let req: SwitchProjectRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.project_path, "/path/to/project");
    }

    #[test]
    fn test_api_error_with_details() {
        let details = serde_json::json!({"field": "name", "issue": "too short"});
        let error = ApiError {
            code: "VALIDATION_ERROR".to_string(),
            message: "Validation failed".to_string(),
            details: Some(details),
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("VALIDATION_ERROR"));
        assert!(json.contains("details"));
        assert!(json.contains("field"));
    }

    #[test]
    fn test_search_query_with_overrides() {
        let json = r#"{"query":"test","include_tasks":false,"include_events":true,"limit":20}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.query, "test");
        assert!(!query.include_tasks);
        assert!(query.include_events);
        assert_eq!(query.limit, Some(20));
    }
}
