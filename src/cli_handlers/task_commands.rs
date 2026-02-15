use crate::backend::{TaskBackend, WorkspaceBackend};
use crate::cli::TaskCommands;
use crate::db::models::TaskSortBy;
use crate::error::{IntentError, Result};
use crate::tasks::TaskUpdate;
use serde_json::json;

use super::utils::{merge_metadata, parse_metadata};

/// Handle all `ie task` subcommands
pub async fn handle_task_command(
    task_mgr: &impl TaskBackend,
    ws_mgr: &impl WorkspaceBackend,
    cmd: TaskCommands,
) -> Result<()> {
    match cmd {
        TaskCommands::Create {
            name,
            description,
            parent,
            status,
            priority,
            owner,
            metadata,
            blocked_by,
            blocks,
            format,
        } => {
            handle_create(
                task_mgr,
                ws_mgr,
                name,
                description,
                parent,
                status,
                priority,
                owner,
                metadata,
                blocked_by,
                blocks,
                format,
            )
            .await
        },

        TaskCommands::Get {
            id,
            with_events,
            with_context,
            format,
        } => handle_get(task_mgr, id, with_events, with_context, format).await,

        TaskCommands::Update {
            id,
            name,
            description,
            status,
            priority,
            active_form,
            owner,
            parent,
            metadata,
            add_blocked_by,
            add_blocks,
            rm_blocked_by,
            rm_blocks,
            format,
        } => {
            handle_update(
                task_mgr,
                id,
                name,
                description,
                status,
                priority,
                active_form,
                owner,
                parent,
                metadata,
                add_blocked_by,
                add_blocks,
                rm_blocked_by,
                rm_blocks,
                format,
            )
            .await
        },

        TaskCommands::List {
            status,
            parent,
            sort,
            limit,
            offset,
            tree,
            format,
        } => handle_list(task_mgr, status, parent, sort, limit, offset, tree, format).await,

        TaskCommands::Delete {
            id,
            cascade,
            format,
        } => handle_delete(task_mgr, id, cascade, format).await,

        TaskCommands::Start {
            id,
            description,
            format,
        } => handle_start(task_mgr, id, description, format).await,

        TaskCommands::Done { id, format } => handle_done(task_mgr, id, format).await,

        TaskCommands::Next { format } => handle_next(task_mgr, format).await,
    }
}

// ============================================================================
// Individual command handlers
// ============================================================================

#[allow(clippy::too_many_arguments)]
pub async fn handle_create(
    task_mgr: &impl TaskBackend,
    ws_mgr: &impl WorkspaceBackend,
    name: String,
    description: Option<String>,
    parent: Option<i64>,
    status: String,
    priority: Option<i32>,
    owner: String,
    metadata: Vec<String>,
    blocked_by: Vec<i64>,
    blocks: Vec<i64>,
    format: String,
) -> Result<()> {
    // Determine parent_id:
    // --parent 0 means root task (no parent)
    // --parent N means use task N as parent
    // omitted means auto-parent to current focused task
    let mut focused_task_for_hint: Option<(i64, String, String)> = None;
    let parent_id = match parent {
        Some(0) => {
            // User explicitly requested root task — check if there's a focused task for hint
            let current = ws_mgr.get_current_task(None).await?;
            if let Some(task) = &current.task {
                focused_task_for_hint = Some((task.id, task.name.clone(), task.status.clone()));
            }
            None
        },
        Some(p) => Some(p),
        None => {
            let current = ws_mgr.get_current_task(None).await?;
            current.current_task_id
        },
    };

    // Pre-merge metadata if specified
    let merged_metadata = if !metadata.is_empty() {
        let meta_json = parse_metadata(&metadata)?;
        merge_metadata(None, &meta_json)
    } else {
        None
    };

    // Create the task with priority and metadata
    let mut task = task_mgr
        .add_task(
            &name,
            description.as_deref(),
            parent_id,
            Some(&owner),
            priority,
            merged_metadata.as_deref(),
        )
        .await?;

    // If status is "doing", start the task
    if status == "doing" {
        let result = task_mgr.start_task(task.id, false).await?;
        task = result.task;
    } else if status == "done" {
        // For "done" status, update directly (rare use case)
        task = task_mgr
            .update_task(
                task.id,
                TaskUpdate {
                    status: Some("done"),
                    ..Default::default()
                },
            )
            .await?;
    }

    // Add blocked-by dependencies (task depends on these)
    for blocking_id in &blocked_by {
        task_mgr.add_dependency(*blocking_id, task.id).await?;
    }

    // Add blocks dependencies (these tasks depend on this task)
    for blocked_id in &blocks {
        task_mgr.add_dependency(task.id, *blocked_id).await?;
    }

    // Output
    if format == "json" {
        let mut response = serde_json::to_value(&task)?;
        if let Some((fid, fname, fstatus)) = &focused_task_for_hint {
            response["hint"] = json!(format!(
                "Current focus: #{} {} [{}]. To make this a subtask: ie task update {} --parent {}",
                fid, fname, fstatus, task.id, fid
            ));
        }
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Task created: #{} {}", task.id, task.name);
        println!("  Status: {}", task.status);
        if let Some(pid) = task.parent_id {
            println!("  Parent: #{}", pid);
        }
        if let Some(p) = task.priority {
            println!("  Priority: {}", p);
        }
        if let Some(spec) = &task.spec {
            println!("  Spec: {}", spec);
        }
        println!("  Owner: {}", task.owner);
        if !blocked_by.is_empty() {
            println!("  Blocked by: {:?}", blocked_by);
        }
        if !blocks.is_empty() {
            println!("  Blocks: {:?}", blocks);
        }

        // Hint: suggest making this a subtask of the focused task
        if let Some((fid, fname, fstatus)) = &focused_task_for_hint {
            eprintln!();
            eprintln!("\u{1f4a1} Current focus: #{} {} [{}]", fid, fname, fstatus);
            eprintln!(
                "   To make this a subtask: ie task update {} --parent {}",
                task.id, fid
            );
        }
    }

    Ok(())
}

pub async fn handle_get(
    task_mgr: &impl TaskBackend,
    id: i64,
    with_events: bool,
    with_context: bool,
    format: String,
) -> Result<()> {
    if with_context {
        // Full context includes ancestors, siblings, children, dependencies
        let context = task_mgr.get_task_context(id).await?;

        if with_events {
            // Combine context with events
            let task_with_events = task_mgr.get_task_with_events(id).await?;
            let response = json!({
                "task": context.task,
                "ancestors": context.ancestors,
                "siblings": context.siblings,
                "children": context.children,
                "dependencies": context.dependencies,
                "events_summary": task_with_events.events_summary,
            });

            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                super::utils::print_task_context(&context);
                if let Some(summary) = &task_with_events.events_summary {
                    super::utils::print_events_summary(summary);
                }
            }
        } else if format == "json" {
            println!("{}", serde_json::to_string_pretty(&context)?);
        } else {
            super::utils::print_task_context(&context);
        }
    } else if with_events {
        let task_with_events = task_mgr.get_task_with_events(id).await?;

        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&task_with_events)?);
        } else {
            let task = &task_with_events.task;
            super::utils::print_task_summary(task);
            if let Some(summary) = &task_with_events.events_summary {
                super::utils::print_events_summary(summary);
            }
        }
    } else {
        let task = task_mgr.get_task(id).await?;

        // Also fetch dependencies for display
        let context = task_mgr.get_task_context(id).await?;

        if format == "json" {
            let response = json!({
                "task": task,
                "blocked_by": context.dependencies.blocking_tasks.iter().map(|t| t.id).collect::<Vec<_>>(),
                "blocks": context.dependencies.blocked_by_tasks.iter().map(|t| t.id).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&response)?);
        } else {
            super::utils::print_task_summary(&task);
            if !context.dependencies.blocking_tasks.is_empty() {
                let ids: Vec<String> = context
                    .dependencies
                    .blocking_tasks
                    .iter()
                    .map(|t| format!("#{}", t.id))
                    .collect();
                println!("  Blocked by: {}", ids.join(", "));
            }
            if !context.dependencies.blocked_by_tasks.is_empty() {
                let ids: Vec<String> = context
                    .dependencies
                    .blocked_by_tasks
                    .iter()
                    .map(|t| format!("#{}", t.id))
                    .collect();
                println!("  Blocks: {}", ids.join(", "));
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_update(
    task_mgr: &impl TaskBackend,
    id: i64,
    name: Option<String>,
    description: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
    active_form: Option<String>,
    owner: Option<String>,
    parent: Option<i64>,
    metadata: Vec<String>,
    add_blocked_by: Vec<i64>,
    add_blocks: Vec<i64>,
    rm_blocked_by: Vec<i64>,
    rm_blocks: Vec<i64>,
    format: String,
) -> Result<()> {
    // Convert parent: 0 means set to root (None), N means set parent to N
    let parent_id_opt: Option<Option<i64>> = parent.map(|p| if p == 0 { None } else { Some(p) });

    // Handle status "doing" specially - use start_task for proper workflow
    let effective_status = if status.as_deref() == Some("doing") {
        None // Don't pass to update_task; we'll call start_task after
    } else {
        status.as_deref().map(String::from)
    };

    // Pre-merge metadata if provided (need current task to merge against)
    let merged_metadata = if !metadata.is_empty() {
        let current_task = task_mgr.get_task(id).await?;
        let meta_json = parse_metadata(&metadata)?;
        merge_metadata(current_task.metadata.as_deref(), &meta_json)
    } else {
        None
    };

    // Core update via TaskManager (single call with all fields)
    let mut task = task_mgr
        .update_task(
            id,
            TaskUpdate {
                name: name.as_deref(),
                spec: description.as_deref(),
                parent_id: parent_id_opt,
                status: effective_status.as_deref(),
                priority,
                active_form: active_form.as_deref(),
                owner: owner.as_deref(),
                metadata: merged_metadata.as_deref(),
                ..Default::default()
            },
        )
        .await?;

    // If status was "doing", use start_task for proper workflow
    if status.as_deref() == Some("doing") {
        let result = task_mgr.start_task(id, false).await?;
        task = result.task;
    }

    // Add dependencies
    for blocking_id in &add_blocked_by {
        task_mgr.add_dependency(*blocking_id, id).await?;
    }
    for blocked_id in &add_blocks {
        task_mgr.add_dependency(id, *blocked_id).await?;
    }

    // Remove dependencies
    for blocking_id in &rm_blocked_by {
        task_mgr.remove_dependency(*blocking_id, id).await?;
    }
    for blocked_id in &rm_blocks {
        task_mgr.remove_dependency(id, *blocked_id).await?;
    }

    // Output
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&task)?);
    } else {
        println!("Task updated: #{} {}", task.id, task.name);
        super::utils::print_task_summary(&task);
    }

    Ok(())
}

pub async fn handle_list(
    task_mgr: &impl TaskBackend,
    status: Option<String>,
    parent: Option<i64>,
    sort: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    tree: bool,
    format: String,
) -> Result<()> {
    // Parse sort option
    let sort_by = match sort.as_deref() {
        Some("id") => Some(TaskSortBy::Id),
        Some("priority") => Some(TaskSortBy::Priority),
        Some("time") => Some(TaskSortBy::Time),
        Some("focus_aware") | Some("focus") => Some(TaskSortBy::FocusAware),
        Some(other) => {
            return Err(IntentError::InvalidInput(format!(
                "Unknown sort option: '{}'. Valid: id, priority, time, focus_aware",
                other
            )));
        },
        None => None,
    };

    // Convert parent: 0 means root tasks (parent IS NULL)
    let parent_id_opt: Option<Option<i64>> = parent.map(|p| if p == 0 { None } else { Some(p) });

    let result = task_mgr
        .find_tasks(status.as_deref(), parent_id_opt, sort_by, limit, offset)
        .await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if tree {
        // Build tree output
        println!(
            "Tasks: {} total (showing {})",
            result.total_count,
            result.tasks.len()
        );
        println!();
        super::utils::print_task_tree(&result.tasks);
        if result.has_more {
            println!(
                "\n  ... more results available (use --offset {})",
                result.offset + result.limit
            );
        }
    } else {
        println!(
            "Tasks: {} total (showing {})",
            result.total_count,
            result.tasks.len()
        );
        println!();
        for task in &result.tasks {
            let status_icon = super::utils::status_icon(&task.status);
            let parent_info = task
                .parent_id
                .map(|p| format!(" (parent: #{})", p))
                .unwrap_or_default();
            let priority_info = task
                .priority
                .map(|p| format!(" [P{}]", p))
                .unwrap_or_default();
            println!(
                "  {} #{} {}{}{}",
                status_icon, task.id, task.name, parent_info, priority_info
            );
        }
        if result.has_more {
            println!(
                "\n  ... more results available (use --offset {})",
                result.offset + result.limit
            );
        }
    }

    Ok(())
}

pub async fn handle_delete(
    task_mgr: &impl TaskBackend,
    id: i64,
    cascade: bool,
    format: String,
) -> Result<()> {
    // Get task info before deletion
    let task = task_mgr.get_task(id).await?;
    let task_name = task.name.clone();

    if cascade {
        let descendant_count = task_mgr.delete_task_cascade(id).await?;

        if format == "json" {
            let response = json!({
                "deleted": true,
                "task_id": id,
                "task_name": task_name,
                "descendants_deleted": descendant_count,
            });
            println!("{}", serde_json::to_string_pretty(&response)?);
        } else {
            println!("Deleted task #{} '{}'", id, task_name);
            if descendant_count > 0 {
                println!("  Cascade deleted: {} descendant tasks", descendant_count);
            }
        }
    } else {
        // Check if task has children first
        let children = task_mgr.get_children(id).await?;
        if !children.is_empty() {
            return Err(IntentError::ActionNotAllowed(format!(
                "Task #{} has {} child tasks. Use --cascade to delete them too, or delete children first.",
                id,
                children.len()
            )));
        }

        task_mgr.delete_task(id).await?;

        if format == "json" {
            let response = json!({
                "deleted": true,
                "task_id": id,
                "task_name": task_name,
            });
            println!("{}", serde_json::to_string_pretty(&response)?);
        } else {
            println!("Deleted task #{} '{}'", id, task_name);
        }
    }

    Ok(())
}

pub async fn handle_start(
    task_mgr: &impl TaskBackend,
    id: i64,
    description: Option<String>,
    format: String,
) -> Result<()> {
    // Update description first if provided
    if let Some(desc) = &description {
        task_mgr
            .update_task(
                id,
                TaskUpdate {
                    spec: Some(desc.as_str()),
                    ..Default::default()
                },
            )
            .await?;
    }

    // Start the task (sets status to doing + sets as current focus)
    let result = task_mgr.start_task(id, true).await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let task = &result.task;
        println!("Started task #{} '{}'", task.id, task.name);
        println!("  Status: {}", task.status);
        if let Some(spec) = &task.spec {
            println!("  Spec: {}", spec);
        }
        if let Some(summary) = &result.events_summary {
            if summary.total_count > 0 {
                println!("  Events: {} total", summary.total_count);
            }
        }
    }

    Ok(())
}

pub async fn handle_done(
    task_mgr: &impl TaskBackend,
    id: Option<i64>,
    format: String,
) -> Result<()> {
    // If ID given, complete by ID directly. If not, complete current focus.
    let result = if let Some(task_id) = id {
        task_mgr.done_task_by_id(task_id, false).await?
    } else {
        task_mgr.done_task(false).await?
    };

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let task = &result.completed_task;
        println!("Completed task #{} '{}'", task.id, task.name);

        // Show next step suggestion
        use crate::db::models::NextStepSuggestion;
        match &result.next_step_suggestion {
            NextStepSuggestion::ParentIsReady {
                message,
                parent_task_id,
                ..
            } => {
                println!("  Next: {} (ie task start {})", message, parent_task_id);
            },
            NextStepSuggestion::SiblingTasksRemain {
                message,
                remaining_siblings_count,
                ..
            } => {
                println!(
                    "  Next: {} ({} siblings remaining)",
                    message, remaining_siblings_count
                );
            },
            NextStepSuggestion::TopLevelTaskCompleted { message, .. } => {
                println!("  {}", message);
            },
            NextStepSuggestion::NoParentContext { message, .. } => {
                println!("  {}", message);
            },
            NextStepSuggestion::WorkspaceIsClear { message, .. } => {
                println!("  {}", message);
            },
        }
    }

    Ok(())
}

pub async fn handle_next(task_mgr: &impl TaskBackend, format: String) -> Result<()> {
    let result = task_mgr.pick_next().await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.format_as_text());
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metadata_basic() {
        let pairs = vec!["type=epic".to_string(), "tag=auth".to_string()];
        let result = parse_metadata(&pairs).unwrap();
        assert_eq!(result["type"], "epic");
        assert_eq!(result["tag"], "auth");
    }

    #[test]
    fn test_parse_metadata_delete_key() {
        let pairs = vec!["key=".to_string()];
        let result = parse_metadata(&pairs).unwrap();
        assert!(result["key"].is_null());
    }

    #[test]
    fn test_parse_metadata_invalid_format() {
        let pairs = vec!["no_equals_sign".to_string()];
        let result = parse_metadata(&pairs);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_metadata_empty_key() {
        let pairs = vec!["=value".to_string()];
        let result = parse_metadata(&pairs);
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_metadata_new() {
        let new_meta = serde_json::json!({"type": "epic"});
        let result = merge_metadata(None, &new_meta);
        assert!(result.is_some());
        let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed["type"], "epic");
    }

    #[test]
    fn test_merge_metadata_update_existing() {
        let existing = r#"{"type":"story","tag":"auth"}"#;
        let new_meta = serde_json::json!({"type": "epic"});
        let result = merge_metadata(Some(existing), &new_meta);
        assert!(result.is_some());
        let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed["type"], "epic");
        assert_eq!(parsed["tag"], "auth");
    }

    #[test]
    fn test_merge_metadata_delete_key() {
        let existing = r#"{"type":"story","tag":"auth"}"#;
        let new_meta = serde_json::json!({"tag": null});
        let result = merge_metadata(Some(existing), &new_meta);
        assert!(result.is_some());
        let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed["type"], "story");
        assert!(parsed.get("tag").is_none());
    }

    #[test]
    fn test_merge_metadata_delete_all() {
        let existing = r#"{"tag":"auth"}"#;
        let new_meta = serde_json::json!({"tag": null});
        let result = merge_metadata(Some(existing), &new_meta);
        assert!(result.is_none()); // Empty map returns None
    }
}
