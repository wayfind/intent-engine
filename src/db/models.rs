use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Dependency {
    pub id: i64,
    pub blocking_task_id: i64,
    pub blocked_task_id: i64,
    pub created_at: DateTime<Utc>,
}

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

/// Unified search result that can represent either a task or event match
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result_type")]
pub enum UnifiedSearchResult {
    #[serde(rename = "task")]
    Task {
        #[serde(flatten)]
        task: Task,
        match_snippet: String,
        match_field: String, // "name" or "spec"
    },
    #[serde(rename = "event")]
    Event {
        event: Event,
        task_chain: Vec<Task>, // Ancestry: [immediate task, parent, grandparent, ...]
        match_snippet: String,
    },
}

/// Response for task switch command - includes previous and current task info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchTaskResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_task: Option<PreviousTaskInfo>,
    pub current_task: CurrentTaskInfo,
}

/// Simplified task info for previous task (only id and status)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousTaskInfo {
    pub id: i64,
    pub status: String,
}

/// Current task info for switch response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentTaskInfo {
    pub id: i64,
    pub name: String,
    pub status: String,
}

/// Response for spawn-subtask command - includes subtask and parent info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSubtaskResponse {
    pub subtask: SubtaskInfo,
    pub parent_task: ParentTaskInfo,
}

/// Subtask info for spawn response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskInfo {
    pub id: i64,
    pub name: String,
    pub parent_id: i64,
    pub status: String,
}

/// Parent task info for spawn response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentTaskInfo {
    pub id: i64,
    pub name: String,
}

/// Dependency information for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependencies {
    /// Tasks that must be completed before this task can start
    pub blocking_tasks: Vec<Task>,
    /// Tasks that are blocked by this task
    pub blocked_by_tasks: Vec<Task>,
}

/// Response for task_context - provides the complete family tree of a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub task: Task,
    pub ancestors: Vec<Task>,
    pub siblings: Vec<Task>,
    pub children: Vec<Task>,
    pub dependencies: TaskDependencies,
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
            },
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
                    },
                    "ALL_TASKS_COMPLETED" => {
                        format!(
                            "[SUCCESS] {}\n\n\
                            You can review the accomplishments of the last 30 days with:\n  \
                            ie report --since 30d",
                            message
                        )
                    },
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
                    },
                    _ => format!("[INFO] {}", message),
                }
            },
            _ => "[ERROR] Unknown suggestion type".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_task(id: i64, name: &str, priority: Option<i32>) -> Task {
        Task {
            id,
            parent_id: None,
            name: name.to_string(),
            spec: None,
            status: "todo".to_string(),
            complexity: None,
            priority,
            first_todo_at: None,
            first_doing_at: None,
            first_done_at: None,
        }
    }

    #[test]
    fn test_pick_next_response_focused_subtask() {
        let task = create_test_task(1, "Test task", Some(5));
        let response = PickNextResponse::focused_subtask(task.clone());

        assert_eq!(response.suggestion_type, "FOCUSED_SUB_TASK");
        assert!(response.task.is_some());
        assert_eq!(response.task.unwrap().id, 1);
        assert!(response.reason_code.is_none());
        assert!(response.message.is_none());
    }

    #[test]
    fn test_pick_next_response_top_level_task() {
        let task = create_test_task(2, "Top level task", Some(3));
        let response = PickNextResponse::top_level_task(task.clone());

        assert_eq!(response.suggestion_type, "TOP_LEVEL_TASK");
        assert!(response.task.is_some());
        assert_eq!(response.task.unwrap().id, 2);
        assert!(response.reason_code.is_none());
        assert!(response.message.is_none());
    }

    #[test]
    fn test_pick_next_response_no_tasks_in_project() {
        let response = PickNextResponse::no_tasks_in_project();

        assert_eq!(response.suggestion_type, "NONE");
        assert!(response.task.is_none());
        assert_eq!(response.reason_code.as_deref(), Some("NO_TASKS_IN_PROJECT"));
        assert!(response.message.is_some());
        assert!(response.message.unwrap().contains("No tasks found"));
    }

    #[test]
    fn test_pick_next_response_all_tasks_completed() {
        let response = PickNextResponse::all_tasks_completed();

        assert_eq!(response.suggestion_type, "NONE");
        assert!(response.task.is_none());
        assert_eq!(response.reason_code.as_deref(), Some("ALL_TASKS_COMPLETED"));
        assert!(response.message.is_some());
        assert!(response.message.unwrap().contains("Project Complete"));
    }

    #[test]
    fn test_pick_next_response_no_available_todos() {
        let response = PickNextResponse::no_available_todos();

        assert_eq!(response.suggestion_type, "NONE");
        assert!(response.task.is_none());
        assert_eq!(response.reason_code.as_deref(), Some("NO_AVAILABLE_TODOS"));
        assert!(response.message.is_some());
    }

    #[test]
    fn test_format_as_text_focused_subtask() {
        let task = create_test_task(1, "Test task", Some(5));
        let response = PickNextResponse::focused_subtask(task);
        let text = response.format_as_text();

        assert!(text.contains("Based on your current focus"));
        assert!(text.contains("[ID: 1]"));
        assert!(text.contains("[Priority: 5]"));
        assert!(text.contains("Test task"));
        assert!(text.contains("ie task start 1"));
    }

    #[test]
    fn test_format_as_text_top_level_task() {
        let task = create_test_task(2, "Top level task", None);
        let response = PickNextResponse::top_level_task(task);
        let text = response.format_as_text();

        assert!(text.contains("Based on your current focus"));
        assert!(text.contains("[ID: 2]"));
        assert!(text.contains("[Priority: 0]")); // Default priority
        assert!(text.contains("Top level task"));
        assert!(text.contains("ie task start 2"));
    }

    #[test]
    fn test_format_as_text_no_tasks_in_project() {
        let response = PickNextResponse::no_tasks_in_project();
        let text = response.format_as_text();

        assert!(text.contains("[INFO]"));
        assert!(text.contains("No tasks found"));
        assert!(text.contains("ie task add"));
        assert!(text.contains("--priority 1"));
    }

    #[test]
    fn test_format_as_text_all_tasks_completed() {
        let response = PickNextResponse::all_tasks_completed();
        let text = response.format_as_text();

        assert!(text.contains("[SUCCESS]"));
        assert!(text.contains("Project Complete"));
        assert!(text.contains("ie report --since 30d"));
    }

    #[test]
    fn test_format_as_text_no_available_todos() {
        let response = PickNextResponse::no_available_todos();
        let text = response.format_as_text();

        assert!(text.contains("[INFO]"));
        assert!(text.contains("No immediate next task"));
        assert!(text.contains("Possible reasons"));
        assert!(text.contains("ie task find"));
    }

    #[test]
    fn test_error_response_serialization() {
        use crate::error::IntentError;

        let error = IntentError::TaskNotFound(123);
        let response = error.to_error_response();

        assert_eq!(response.code, "TASK_NOT_FOUND");
        assert!(response.error.contains("123"));
    }

    #[test]
    fn test_next_step_suggestion_serialization() {
        let suggestion = NextStepSuggestion::ParentIsReady {
            message: "Test message".to_string(),
            parent_task_id: 1,
            parent_task_name: "Parent".to_string(),
        };

        let json = serde_json::to_string(&suggestion).unwrap();
        assert!(json.contains("\"type\":\"PARENT_IS_READY\""));
        assert!(json.contains("parent_task_id"));
    }

    #[test]
    fn test_task_with_events_serialization() {
        let task = create_test_task(1, "Test", Some(5));
        let task_with_events = TaskWithEvents {
            task,
            events_summary: None,
        };

        let json = serde_json::to_string(&task_with_events).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"name\":\"Test\""));
        // events_summary should be skipped when None
        assert!(!json.contains("events_summary"));
    }

    #[test]
    fn test_report_summary_with_date_range() {
        let from = Utc::now() - chrono::Duration::days(7);
        let to = Utc::now();

        let summary = ReportSummary {
            total_tasks: 10,
            tasks_by_status: StatusBreakdown {
                todo: 5,
                doing: 3,
                done: 2,
            },
            total_events: 20,
            date_range: Some(DateRange { from, to }),
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total_tasks\":10"));
        assert!(json.contains("\"total_events\":20"));
        assert!(json.contains("date_range"));
    }
}
