use crate::backend::{EventBackend, WorkspaceBackend};
use crate::cli::LogEventType;
use crate::error::{IntentError, Result};

/// Handle `ie log` command.
pub async fn handle_log(
    event_mgr: &impl EventBackend,
    ws_mgr: &impl WorkspaceBackend,
    event_type: LogEventType,
    message: &str,
    task: Option<i64>,
    format: &str,
) -> Result<()> {
    // Determine task_id: use --task flag, or fall back to current focused task
    let target_task_id = if let Some(tid) = task {
        tid
    } else {
        let current_response = ws_mgr.get_current_task(None).await?;
        let current_task = current_response.task.ok_or_else(|| {
            IntentError::ActionNotAllowed(
                "No current task set. Use --task <ID> or start a task first.".to_string(),
            )
        })?;
        current_task.id
    };

    let event_type_str = event_type.as_str();

    let event = event_mgr
        .add_event(target_task_id, event_type_str, message)
        .await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&event)?);
    } else {
        println!("  Event recorded");
        println!("  ID: {}", event.id);
        println!("  Type: {}", event_type_str);
        println!("  Task: #{}", target_task_id);
        println!(
            "  Time: {}",
            event.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!("  Message: {}", message);
    }

    Ok(())
}
