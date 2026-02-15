use crate::backend::{TaskBackend, WorkspaceBackend};
use crate::db::models::{NoFocusResponse, StatusResponse, TaskBrief};
use crate::error::Result;

/// Handle `ie status` command.
///
/// Returns `Ok(true)` if a focused task was displayed, `Ok(false)` if no focus.
/// The caller can add backend-specific logic (e.g. LLM suggestions) after this.
pub async fn handle_status(
    task_mgr: &impl TaskBackend,
    ws_mgr: &impl WorkspaceBackend,
    task_id: Option<i64>,
    with_events: bool,
    format: &str,
) -> Result<bool> {
    // Determine which task to show status for
    let target_task_id = if let Some(id) = task_id {
        Some(id)
    } else {
        let current = ws_mgr.get_current_task(None).await?;
        current.current_task_id
    };

    match target_task_id {
        Some(id) => {
            let status = task_mgr.get_status(id, with_events).await?;

            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                print_status_text(&status);
            }
            Ok(true)
        },
        None => {
            let root_tasks = task_mgr.get_root_tasks().await?;
            let root_briefs: Vec<TaskBrief> = root_tasks.iter().map(TaskBrief::from).collect();

            let response = NoFocusResponse {
                message: "No focused task. Use 'ie plan' with status:'doing' to start a task."
                    .to_string(),
                root_tasks: root_briefs,
            };

            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                println!("  {}", response.message);
                if !response.root_tasks.is_empty() {
                    println!("\n  Root tasks:");
                    for task in &response.root_tasks {
                        let icon = super::utils::status_icon(&task.status);
                        println!("   {} #{}: {} [{}]", icon, task.id, task.name, task.status);
                    }
                }
            }
            Ok(false)
        },
    }
}

/// Print status in text format (shared between SQLite and Neo4j).
pub fn print_status_text(status: &StatusResponse) {
    let ft = &status.focused_task;
    let icon = super::utils::status_icon(&ft.status);
    println!("{} Task #{}: {}", icon, ft.id, ft.name);
    println!("   Status: {}", ft.status);
    if let Some(parent_id) = ft.parent_id {
        println!("   Parent: #{}", parent_id);
    }
    if let Some(priority) = ft.priority {
        println!("   Priority: {}", priority);
    }
    if let Some(spec) = &ft.spec {
        println!("   Spec: {}", spec);
    }
    println!("   Owner: {}", ft.owner);
    if let Some(ts) = ft.first_todo_at {
        println!("   First todo: {}", ts.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    if let Some(ts) = ft.first_doing_at {
        println!("   First doing: {}", ts.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    if let Some(ts) = ft.first_done_at {
        println!("   First done: {}", ts.format("%Y-%m-%d %H:%M:%S UTC"));
    }

    if !status.ancestors.is_empty() {
        println!("\n  Ancestors ({}):", status.ancestors.len());
        for ancestor in &status.ancestors {
            let parent_info = ancestor
                .parent_id
                .map(|p| format!(" (parent: #{})", p))
                .unwrap_or_default();
            let priority_info = ancestor
                .priority
                .map(|p| format!(" [P{}]", p))
                .unwrap_or_default();
            println!(
                "   #{}: {} [{}]{}{}",
                ancestor.id, ancestor.name, ancestor.status, parent_info, priority_info
            );
            if let Some(spec) = &ancestor.spec {
                println!("      Spec: {}", spec);
            }
            println!("      Owner: {}", ancestor.owner);
            if let Some(ts) = ancestor.first_todo_at {
                print!("      todo: {} ", ts.format("%m-%d %H:%M:%S"));
            }
            if let Some(ts) = ancestor.first_doing_at {
                print!("doing: {} ", ts.format("%m-%d %H:%M:%S"));
            }
            if let Some(ts) = ancestor.first_done_at {
                print!("done: {}", ts.format("%m-%d %H:%M:%S"));
            }
            if ancestor.first_todo_at.is_some()
                || ancestor.first_doing_at.is_some()
                || ancestor.first_done_at.is_some()
            {
                println!();
            }
        }
    }

    if !status.siblings.is_empty() {
        println!("\n  Siblings ({}):", status.siblings.len());
        for sibling in &status.siblings {
            let parent_info = sibling
                .parent_id
                .map(|p| format!(" (parent: #{})", p))
                .unwrap_or_default();
            let spec_indicator = if sibling.has_spec { "" } else { " ?" };
            println!(
                "   #{}: {} [{}]{}{}",
                sibling.id, sibling.name, sibling.status, parent_info, spec_indicator
            );
        }
    }

    if !status.descendants.is_empty() {
        println!("\n  Descendants ({}):", status.descendants.len());
        for desc in &status.descendants {
            let parent_info = desc
                .parent_id
                .map(|p| format!(" (parent: #{})", p))
                .unwrap_or_default();
            let spec_indicator = if desc.has_spec { "" } else { " ?" };
            println!(
                "   #{}: {} [{}]{}{}",
                desc.id, desc.name, desc.status, parent_info, spec_indicator
            );
        }
    }

    if let Some(events) = &status.events {
        println!("\n  Events ({}):", events.len());
        for event in events.iter().take(10) {
            println!(
                "   #{} [{}] {}: {}",
                event.id,
                event.log_type,
                event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                event.discussion_data.chars().take(60).collect::<String>()
            );
        }
    }
}
