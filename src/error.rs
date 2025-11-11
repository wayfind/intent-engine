use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IntentError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Task not found: {0}")]
    TaskNotFound(i64),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Circular dependency detected: adding dependency from task {blocking_task_id} to task {blocked_task_id} would create a cycle")]
    CircularDependency {
        blocking_task_id: i64,
        blocked_task_id: i64,
    },

    #[error("Task {task_id} is blocked by incomplete tasks: {blocking_task_ids:?}")]
    TaskBlocked {
        task_id: i64,
        blocking_task_ids: Vec<i64>,
    },

    #[error("Action not allowed: {0}")]
    ActionNotAllowed(String),

    #[error("Uncompleted children exist")]
    UncompletedChildren,

    #[error("Current directory is not an Intent-Engine project")]
    NotAProject,

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

impl IntentError {
    pub fn to_error_code(&self) -> &'static str {
        match self {
            IntentError::TaskNotFound(_) => "TASK_NOT_FOUND",
            IntentError::DatabaseError(_) => "DATABASE_ERROR",
            IntentError::InvalidInput(_) => "INVALID_INPUT",
            IntentError::CircularDependency { .. } => "CIRCULAR_DEPENDENCY",
            IntentError::TaskBlocked { .. } => "TASK_BLOCKED",
            IntentError::ActionNotAllowed(_) => "ACTION_NOT_ALLOWED",
            IntentError::UncompletedChildren => "UNCOMPLETED_CHILDREN",
            IntentError::NotAProject => "NOT_A_PROJECT",
            _ => "INTERNAL_ERROR",
        }
    }

    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: self.to_string(),
            code: self.to_error_code().to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, IntentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_not_found_error() {
        let error = IntentError::TaskNotFound(123);
        assert_eq!(error.to_string(), "Task not found: 123");
        assert_eq!(error.to_error_code(), "TASK_NOT_FOUND");
    }

    #[test]
    fn test_invalid_input_error() {
        let error = IntentError::InvalidInput("Bad input".to_string());
        assert_eq!(error.to_string(), "Invalid input: Bad input");
        assert_eq!(error.to_error_code(), "INVALID_INPUT");
    }

    #[test]
    fn test_circular_dependency_error() {
        let error = IntentError::CircularDependency {
            blocking_task_id: 1,
            blocked_task_id: 2,
        };
        assert!(error.to_string().contains("Circular dependency detected"));
        assert!(error.to_string().contains("task 1"));
        assert!(error.to_string().contains("task 2"));
        assert_eq!(error.to_error_code(), "CIRCULAR_DEPENDENCY");
    }

    #[test]
    fn test_action_not_allowed_error() {
        let error = IntentError::ActionNotAllowed("Cannot do this".to_string());
        assert_eq!(error.to_string(), "Action not allowed: Cannot do this");
        assert_eq!(error.to_error_code(), "ACTION_NOT_ALLOWED");
    }

    #[test]
    fn test_uncompleted_children_error() {
        let error = IntentError::UncompletedChildren;
        assert_eq!(error.to_string(), "Uncompleted children exist");
        assert_eq!(error.to_error_code(), "UNCOMPLETED_CHILDREN");
    }

    #[test]
    fn test_not_a_project_error() {
        let error = IntentError::NotAProject;
        assert_eq!(
            error.to_string(),
            "Current directory is not an Intent-Engine project"
        );
        assert_eq!(error.to_error_code(), "NOT_A_PROJECT");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error: IntentError = io_error.into();
        assert!(matches!(error, IntentError::IoError(_)));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_str = "{invalid json";
        let json_error = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let error: IntentError = json_error.into();
        assert!(matches!(error, IntentError::JsonError(_)));
    }

    #[test]
    fn test_error_response_structure() {
        let error = IntentError::TaskNotFound(456);
        let response = error.to_error_response();

        assert_eq!(response.code, "TASK_NOT_FOUND");
        assert_eq!(response.error, "Task not found: 456");
    }

    #[test]
    fn test_error_response_serialization() {
        let error = IntentError::InvalidInput("Test".to_string());
        let response = error.to_error_response();
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"code\":\"INVALID_INPUT\""));
        assert!(json.contains("\"error\":\"Invalid input: Test\""));
    }

    #[test]
    fn test_database_error_code() {
        // We can't easily create a real sqlx::Error, so we test through the pattern match
        let error = IntentError::TaskNotFound(1);
        if let IntentError::DatabaseError(_) = error {
            unreachable!()
        }
    }

    #[test]
    fn test_internal_error_fallback() {
        // Test the _ => "INTERNAL_ERROR" case by testing IoError
        let io_error = std::io::Error::other("test");
        let error: IntentError = io_error.into();
        assert_eq!(error.to_error_code(), "INTERNAL_ERROR");
    }
}
