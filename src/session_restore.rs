use crate::error::Result;
use crate::events::EventManager;
use crate::tasks::TaskManager;
use crate::workspace::WorkspaceManager;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Session restoration status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Successfully restored focus
    Success,
    /// No active focus (workspace exists but no current task)
    NoFocus,
    /// Error occurred
    Error,
}

/// Error types for session restoration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    WorkspaceNotFound,
    DatabaseCorrupted,
    PermissionDenied,
}

/// Simplified task info for session restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Detailed current task info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentTaskInfo {
    pub id: i64,
    pub name: String,
    pub status: String,
    /// Full spec
    pub spec: Option<String>,
    /// Truncated spec (first 100 chars)
    pub spec_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_doing_at: Option<String>,
}

/// Siblings information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiblingsInfo {
    pub total: usize,
    pub done: usize,
    pub doing: usize,
    pub todo: usize,
    pub done_list: Vec<TaskInfo>,
}

/// Children information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildrenInfo {
    pub total: usize,
    pub todo: usize,
    pub list: Vec<TaskInfo>,
}

/// Event information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventInfo {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: String,
    pub timestamp: String,
}

/// Workspace statistics (for no-focus scenario)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStats {
    pub total_tasks: usize,
    pub todo: usize,
    pub doing: usize,
    pub done: usize,
}

/// Complete session restoration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRestoreResult {
    pub status: SessionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<CurrentTaskInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_task: Option<TaskInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub siblings: Option<SiblingsInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<ChildrenInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_events: Option<Vec<EventInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_commands: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<WorkspaceStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<ErrorType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_suggestion: Option<String>,
}

/// Session restoration manager
pub struct SessionRestoreManager<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SessionRestoreManager<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Restore session with full context
    pub async fn restore(&self, include_events: usize) -> Result<SessionRestoreResult> {
        let workspace_mgr = WorkspaceManager::new(self.pool);
        let task_mgr = TaskManager::new(self.pool);
        let event_mgr = EventManager::new(self.pool);

        // Get current workspace path (for display purposes)
        let workspace_path = std::env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(String::from));

        // Get current task
        let current_task_id = match workspace_mgr.get_current_task().await {
            Ok(response) => {
                if let Some(id) = response.current_task_id {
                    id
                } else {
                    // No focus - return stats
                    return self.restore_no_focus(workspace_path).await;
                }
            },
            Err(_) => {
                // Error getting current task
                return Ok(Self::error_result(
                    workspace_path,
                    ErrorType::DatabaseCorrupted,
                    "Unable to query workspace state",
                    "Run 'ie workspace init' or check database integrity",
                ));
            },
        };

        // Get current task details
        let task = match task_mgr.get_task(current_task_id).await {
            Ok(t) => t,
            Err(_) => {
                return Ok(Self::error_result(
                    workspace_path,
                    ErrorType::DatabaseCorrupted,
                    "Current task not found in database",
                    "Run 'ie current --set <task_id>' to set a valid task",
                ));
            },
        };

        let spec_preview = task.spec.as_ref().map(|s| Self::truncate_spec(s, 100));

        let current_task = CurrentTaskInfo {
            id: task.id,
            name: task.name.clone(),
            status: task.status.clone(),
            spec: task.spec.clone(),
            spec_preview,
            created_at: task.first_todo_at.map(|dt| dt.to_rfc3339()),
            first_doing_at: task.first_doing_at.map(|dt| dt.to_rfc3339()),
        };

        // Get parent task
        let parent_task = if let Some(parent_id) = task.parent_id {
            task_mgr.get_task(parent_id).await.ok().map(|p| TaskInfo {
                id: p.id,
                name: p.name,
                status: Some(p.status),
            })
        } else {
            None
        };

        // Get siblings info
        let siblings = if let Some(parent_id) = task.parent_id {
            let all_siblings = task_mgr.find_tasks(None, Some(Some(parent_id))).await?;
            Self::build_siblings_info(&all_siblings)
        } else {
            None
        };

        // Get children info
        let children = {
            let all_children = task_mgr
                .find_tasks(None, Some(Some(current_task_id)))
                .await?;
            Self::build_children_info(&all_children)
        };

        // Get recent events
        let events = event_mgr
            .list_events(current_task_id, None, None, None)
            .await?;
        let recent_events: Vec<EventInfo> = events
            .into_iter()
            .take(include_events)
            .map(|e| EventInfo {
                event_type: e.log_type,
                data: e.discussion_data,
                timestamp: e.timestamp.to_rfc3339(),
            })
            .collect();

        // Suggest next commands based on context
        let suggested_commands = Self::suggest_commands(&current_task, children.as_ref());

        Ok(SessionRestoreResult {
            status: SessionStatus::Success,
            workspace_path,
            current_task: Some(current_task),
            parent_task,
            siblings,
            children,
            recent_events: Some(recent_events),
            suggested_commands: Some(suggested_commands),
            stats: None,
            error_type: None,
            message: None,
            recovery_suggestion: None,
        })
    }

    /// Restore when no focus exists
    async fn restore_no_focus(
        &self,
        workspace_path: Option<String>,
    ) -> Result<SessionRestoreResult> {
        let task_mgr = TaskManager::new(self.pool);

        // Get all tasks for stats
        let all_tasks = task_mgr.find_tasks(None, None).await?;

        let stats = WorkspaceStats {
            total_tasks: all_tasks.len(),
            todo: all_tasks.iter().filter(|t| t.status == "todo").count(),
            doing: all_tasks.iter().filter(|t| t.status == "doing").count(),
            done: all_tasks.iter().filter(|t| t.status == "done").count(),
        };

        let suggested_commands = vec![
            "ie pick-next".to_string(),
            "ie task list --status todo".to_string(),
        ];

        Ok(SessionRestoreResult {
            status: SessionStatus::NoFocus,
            workspace_path,
            current_task: None,
            parent_task: None,
            siblings: None,
            children: None,
            recent_events: None,
            suggested_commands: Some(suggested_commands),
            stats: Some(stats),
            error_type: None,
            message: None,
            recovery_suggestion: None,
        })
    }

    /// Build siblings information
    fn build_siblings_info(siblings: &[crate::db::models::Task]) -> Option<SiblingsInfo> {
        if siblings.is_empty() {
            return None;
        }

        let done_list: Vec<TaskInfo> = siblings
            .iter()
            .filter(|s| s.status == "done")
            .map(|s| TaskInfo {
                id: s.id,
                name: s.name.clone(),
                status: Some(s.status.clone()),
            })
            .collect();

        Some(SiblingsInfo {
            total: siblings.len(),
            done: siblings.iter().filter(|s| s.status == "done").count(),
            doing: siblings.iter().filter(|s| s.status == "doing").count(),
            todo: siblings.iter().filter(|s| s.status == "todo").count(),
            done_list,
        })
    }

    /// Build children information
    fn build_children_info(children: &[crate::db::models::Task]) -> Option<ChildrenInfo> {
        if children.is_empty() {
            return None;
        }

        let list: Vec<TaskInfo> = children
            .iter()
            .map(|c| TaskInfo {
                id: c.id,
                name: c.name.clone(),
                status: Some(c.status.clone()),
            })
            .collect();

        Some(ChildrenInfo {
            total: children.len(),
            todo: children.iter().filter(|c| c.status == "todo").count(),
            list,
        })
    }

    /// Suggest next commands based on context
    fn suggest_commands(
        _current_task: &CurrentTaskInfo,
        children: Option<&ChildrenInfo>,
    ) -> Vec<String> {
        let mut commands = vec![];

        // If there are blockers, suggest viewing them
        commands.push("ie event list --type blocker".to_string());

        // Suggest completion or spawning subtasks
        if let Some(children) = children {
            if children.todo > 0 {
                commands.push("ie task done".to_string());
            }
        } else {
            commands.push("ie task done".to_string());
        }

        commands.push("ie task spawn-subtask".to_string());

        commands
    }

    /// Truncate spec to specified length
    fn truncate_spec(spec: &str, max_len: usize) -> String {
        if spec.len() <= max_len {
            spec.to_string()
        } else {
            format!("{}...", &spec[..max_len])
        }
    }

    /// Create error result
    fn error_result(
        workspace_path: Option<String>,
        error_type: ErrorType,
        message: &str,
        recovery: &str,
    ) -> SessionRestoreResult {
        let suggested_commands = match error_type {
            ErrorType::WorkspaceNotFound => {
                vec!["ie workspace init".to_string(), "ie help".to_string()]
            },
            ErrorType::DatabaseCorrupted => {
                vec!["ie workspace init".to_string(), "ie doctor".to_string()]
            },
            ErrorType::PermissionDenied => {
                vec!["chmod 644 .intent-engine/workspace.db".to_string()]
            },
        };

        SessionRestoreResult {
            status: SessionStatus::Error,
            workspace_path,
            current_task: None,
            parent_task: None,
            siblings: None,
            children: None,
            recent_events: None,
            suggested_commands: Some(suggested_commands),
            stats: None,
            error_type: Some(error_type),
            message: Some(message.to_string()),
            recovery_suggestion: Some(recovery.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_spec() {
        let spec = "a".repeat(200);
        let truncated = SessionRestoreManager::truncate_spec(&spec, 100);
        assert_eq!(truncated.len(), 103); // 100 + "..."
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_spec_short() {
        let spec = "Short spec";
        let truncated = SessionRestoreManager::truncate_spec(spec, 100);
        assert_eq!(truncated, spec);
        assert!(!truncated.ends_with("..."));
    }
}
