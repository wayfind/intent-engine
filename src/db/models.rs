use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Task {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complexity: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    pub first_todo_at: Option<DateTime<Utc>>,
    pub first_doing_at: Option<DateTime<Utc>>,
    pub first_done_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWithEvents {
    #[serde(flatten)]
    pub task: Task,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events_summary: Option<EventsSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsSummary {
    pub total_count: i64,
    pub recent_events: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: i64,
    pub task_id: i64,
    pub timestamp: DateTime<Utc>,
    pub log_type: String,
    pub discussion_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkspaceState {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub summary: ReportSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<Task>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<Event>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_tasks: i64,
    pub tasks_by_status: StatusBreakdown,
    pub total_events: i64,
    pub date_range: Option<DateRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusBreakdown {
    pub todo: i64,
    pub doing: i64,
    pub done: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoneTaskResponse {
    pub completed_task: Task,
    pub workspace_status: WorkspaceStatus,
    pub next_step_suggestion: NextStepSuggestion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    pub current_task_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NextStepSuggestion {
    #[serde(rename = "PARENT_IS_READY")]
    ParentIsReady {
        message: String,
        parent_task_id: i64,
        parent_task_name: String,
    },
    #[serde(rename = "SIBLING_TASKS_REMAIN")]
    SiblingTasksRemain {
        message: String,
        parent_task_id: i64,
        parent_task_name: String,
        remaining_siblings_count: i64,
    },
    #[serde(rename = "TOP_LEVEL_TASK_COMPLETED")]
    TopLevelTaskCompleted {
        message: String,
        completed_task_id: i64,
        completed_task_name: String,
    },
    #[serde(rename = "NO_PARENT_CONTEXT")]
    NoParentContext {
        message: String,
        completed_task_id: i64,
        completed_task_name: String,
    },
    #[serde(rename = "WORKSPACE_IS_CLEAR")]
    WorkspaceIsClear {
        message: String,
        completed_task_id: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSearchResult {
    #[serde(flatten)]
    pub task: Task,
    pub match_snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickNextResponse {
    pub suggestion_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<Task>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl PickNextResponse {
    /// Create a response for focused subtask suggestion
    pub fn focused_subtask(task: Task) -> Self {
        Self {
            suggestion_type: "FOCUSED_SUB_TASK".to_string(),
            task: Some(task),
            reason_code: None,
            message: None,
        }
    }

    /// Create a response for top-level task suggestion
    pub fn top_level_task(task: Task) -> Self {
        Self {
            suggestion_type: "TOP_LEVEL_TASK".to_string(),
            task: Some(task),
            reason_code: None,
            message: None,
        }
    }

    /// Create a response for no tasks in project
    pub fn no_tasks_in_project() -> Self {
        Self {
            suggestion_type: "NONE".to_string(),
            task: None,
            reason_code: Some("NO_TASKS_IN_PROJECT".to_string()),
            message: Some(
                "No tasks found in this project. Your intent backlog is empty.".to_string(),
            ),
        }
    }

    /// Create a response for all tasks completed
    pub fn all_tasks_completed() -> Self {
        Self {
            suggestion_type: "NONE".to_string(),
            task: None,
            reason_code: Some("ALL_TASKS_COMPLETED".to_string()),
            message: Some("Project Complete! All intents have been realized.".to_string()),
        }
    }

    /// Create a response for no available todos
    pub fn no_available_todos() -> Self {
        Self {
            suggestion_type: "NONE".to_string(),
            task: None,
            reason_code: Some("NO_AVAILABLE_TODOS".to_string()),
            message: Some("No immediate next task found based on the current context.".to_string()),
        }
    }

    /// Format response as human-readable text
    pub fn format_as_text(&self) -> String {
        match self.suggestion_type.as_str() {
            "FOCUSED_SUB_TASK" | "TOP_LEVEL_TASK" => {
                if let Some(task) = &self.task {
                    format!(
                        "Based on your current focus, the recommended next task is:\n\n\
                        [ID: {}] [Priority: {}] [Status: {}]\n\
                        Name: {}\n\n\
                        To start working on it, run:\n  ie task start {}",
                        task.id,
                        task.priority.unwrap_or(0),
                        task.status,
                        task.name,
                        task.id
                    )
                } else {
                    "[ERROR] Invalid response: task is missing".to_string()
                }
            }
            "NONE" => {
                let reason_code = self.reason_code.as_deref().unwrap_or("UNKNOWN");
                let message = self.message.as_deref().unwrap_or("No tasks found");

                match reason_code {
                    "NO_TASKS_IN_PROJECT" => {
                        format!(
                            "[INFO] {}\n\n\
                            To get started, capture your first high-level intent:\n  \
                            ie task add --name \"Setup initial project structure\" --priority 1",
                            message
                        )
                    }
                    "ALL_TASKS_COMPLETED" => {
                        format!(
                            "[SUCCESS] {}\n\n\
                            You can review the accomplishments of the last 30 days with:\n  \
                            ie report --since 30d",
                            message
                        )
                    }
                    "NO_AVAILABLE_TODOS" => {
                        format!(
                            "[INFO] {}\n\n\
                            Possible reasons:\n\
                            - All available 'todo' tasks are part of larger epics that have not been started yet.\n\
                            - You are not currently focused on a task that has 'todo' sub-tasks.\n\n\
                            To see all available top-level tasks you can start, run:\n  \
                            ie task find --parent NULL --status todo",
                            message
                        )
                    }
                    _ => format!("[INFO] {}", message),
                }
            }
            _ => "[ERROR] Unknown suggestion type".to_string(),
        }
    }
}
