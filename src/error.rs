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

    #[error("Circular dependency detected")]
    CircularDependency,

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
            IntentError::CircularDependency => "CIRCULAR_DEPENDENCY",
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
