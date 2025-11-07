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
