use crate::cli::TaskCommands;
use crate::dependencies::add_dependency;
use crate::error::{IntentError, Result};
use crate::project::ProjectContext;
use crate::tasks::{TaskManager, TaskUpdate};
use crate::workspace::WorkspaceManager;
use serde_json::json;

/// Parse metadata key=value strings into a JSON object.
/// "key=value" sets a key, "key=" deletes a key.
fn parse_metadata(pairs: &[String]) -> Result<serde_json::Value> {
    let mut map = serde_json::Map::new();
    for pair in pairs {
        if let Some(eq_pos) = pair.find('=') {
            let key = pair[..eq_pos].trim().to_string();
            let value = pair[eq_pos + 1..].trim().to_string();
            if key.is_empty() {
                return Err(IntentError::InvalidInput(format!(
                    "Invalid metadata: empty key in '{}'",
                    pair
                )));
            }
            if value.is_empty() {
                // "key=" means delete
                map.insert(key, serde_json::Value::Null);
            } else {
                map.insert(key, serde_json::Value::String(value));
            }
        } else {
            return Err(IntentError::InvalidInput(format!(
                "Invalid metadata format: '{}'. Expected 'key=value'",
                pair
            )));
        }
    }
    Ok(serde_json::Value::Object(map))
}

/// Merge new metadata into existing metadata JSON string.
/// Null values in new_meta mean "delete this key".
fn merge_metadata(existing: Option<&str>, new_meta: &serde_json::Value) -> Option<String> {
    let mut base: serde_json::Map<String, serde_json::Value> = existing
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    if let serde_json::Value::Object(new_map) = new_meta {
        for (key, value) in new_map {
            if value.is_null() {
                base.remove(key);
            } else {
                base.insert(key.clone(), value.clone());
            }
        }
    }

    if base.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&base).unwrap_or_default())
    }
}

/// Handle all `ie task` subcommands
pub async fn handle_task_command(cmd: TaskCommands) -> Result<()> {
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
        } => handle_get(id, with_events, with_context, format).await,

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
        } => handle_list(status, parent, sort, limit, offset, tree, format).await,

        TaskCommands::Delete {
            id,
            cascade,
            format,
        } => handle_delete(id, cascade, format).await,

        TaskCommands::Start {
            id,
            description,
            format,
        } => handle_start(id, description, format).await,

        TaskCommands::Done { id, format } => handle_done(id, format).await,

        TaskCommands::Next { format } => handle_next(format).await,
    }
}

// ============================================================================
// Individual command handlers
// ============================================================================

#[allow(clippy::too_many_arguments)]
async fn handle_create(
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
    let ctx = ProjectContext::load_or_init().await?;
    let project_path = ctx.root.to_string_lossy().to_string();
    let task_mgr = TaskManager::with_project_path(&ctx.pool, project_path);
    let workspace_mgr = WorkspaceManager::new(&ctx.pool);

    // Determine parent_id:
    // --parent 0 means root task (no parent)
    // --parent N means use task N as parent
    // omitted means auto-parent to current focused task
    let mut focused_task_for_hint: Option<(i64, String, String)> = None;
    let parent_id = match parent {
        Some(0) => {
            // User explicitly requested root task — check if there's a focused task for hint
            let current = workspace_mgr.get_current_task(None).await?;
            if let Some(task) = &current.task {
                focused_task_for_hint = Some((task.id, task.name.clone(), task.status.clone()));
            }
            None
        },
        Some(p) => Some(p),
        None => {
            let current = workspace_mgr.get_current_task(None).await?;
            current.current_task_id
        },
    };

    // Create the task
    let mut task = task_mgr
        .add_task(&name, description.as_deref(), parent_id, Some(&owner))
        .await?;

    // Pre-merge metadata if specified
    let merged_metadata = if !metadata.is_empty() {
        let meta_json = parse_metadata(&metadata)?;
        merge_metadata(task.metadata.as_deref(), &meta_json)
    } else {
        None
    };

    // Apply priority and metadata in a single update_task call if either is set
    if priority.is_some() || merged_metadata.is_some() {
        task = task_mgr
            .update_task(
                task.id,
                TaskUpdate {
                    priority,
                    metadata: merged_metadata.as_deref(),
                    ..Default::default()
                },
            )
            .await?;
    }

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
        add_dependency(&ctx.pool, *blocking_id, task.id).await?;
    }

    // Add blocks dependencies (these tasks depend on this task)
    for blocked_id in &blocks {
        add_dependency(&ctx.pool, task.id, *blocked_id).await?;
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

async fn handle_get(id: i64, with_events: bool, with_context: bool, format: String) -> Result<()> {
    let ctx = ProjectContext::load_or_init().await?;
    let task_mgr = TaskManager::new(&ctx.pool);

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
                crate::cli_handlers::print_task_context(&context)?;
                if let Some(summary) = &task_with_events.events_summary {
                    println!("Events ({}):", summary.total_count);
                    for event in summary.recent_events.iter().take(10) {
                        println!(
                            "  #{} [{}] {}: {}",
                            event.id,
                            event.log_type,
                            event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                            event.discussion_data.chars().take(60).collect::<String>()
                        );
                    }
                }
            }
        } else {
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&context)?);
            } else {
                crate::cli_handlers::print_task_context(&context)?;
            }
        }
    } else if with_events {
        let task_with_events = task_mgr.get_task_with_events(id).await?;

        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&task_with_events)?);
        } else {
            let task = &task_with_events.task;
            print_task_summary(task);
            if let Some(summary) = &task_with_events.events_summary {
                println!("Events ({}):", summary.total_count);
                for event in summary.recent_events.iter().take(10) {
                    println!(
                        "  #{} [{}] {}: {}",
                        event.id,
                        event.log_type,
                        event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        event.discussion_data.chars().take(60).collect::<String>()
                    );
                }
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
            print_task_summary(&task);
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
async fn handle_update(
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
    let ctx = ProjectContext::load_or_init().await?;
    let project_path = ctx.root.to_string_lossy().to_string();
    let task_mgr = TaskManager::with_project_path(&ctx.pool, project_path);

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
        add_dependency(&ctx.pool, *blocking_id, id).await?;
    }
    for blocked_id in &add_blocks {
        add_dependency(&ctx.pool, id, *blocked_id).await?;
    }

    // Remove dependencies
    for blocking_id in &rm_blocked_by {
        sqlx::query("DELETE FROM dependencies WHERE blocking_task_id = ? AND blocked_task_id = ?")
            .bind(blocking_id)
            .bind(id)
            .execute(&ctx.pool)
            .await?;
    }
    for blocked_id in &rm_blocks {
        sqlx::query("DELETE FROM dependencies WHERE blocking_task_id = ? AND blocked_task_id = ?")
            .bind(id)
            .bind(blocked_id)
            .execute(&ctx.pool)
            .await?;
    }

    // Output
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&task)?);
    } else {
        println!("Task updated: #{} {}", task.id, task.name);
        print_task_summary(&task);
    }

    Ok(())
}

async fn handle_list(
    status: Option<String>,
    parent: Option<i64>,
    sort: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    tree: bool,
    format: String,
) -> Result<()> {
    use crate::db::models::TaskSortBy;

    let ctx = ProjectContext::load_or_init().await?;
    let task_mgr = TaskManager::new(&ctx.pool);

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
        print_task_tree(&result.tasks);
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
            let status_icon = match task.status.as_str() {
                "todo" => "○",
                "doing" => "●",
                "done" => "✓",
                _ => "?",
            };
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

async fn handle_delete(id: i64, cascade: bool, format: String) -> Result<()> {
    let ctx = ProjectContext::load_or_init().await?;
    let project_path = ctx.root.to_string_lossy().to_string();
    let task_mgr = TaskManager::with_project_path(&ctx.pool, project_path);

    // Get task info before deletion
    let task = task_mgr.get_task(id).await?;
    let task_name = task.name.clone();

    if cascade {
        // Get descendant count for reporting
        let descendants = task_mgr.get_descendants(id).await?;
        let descendant_count = descendants.len();

        // delete_task cascades via ON DELETE CASCADE in SQLite
        task_mgr.delete_task(id).await?;

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
        let child_count: i64 =
            sqlx::query_scalar::<_, i64>(crate::sql_constants::COUNT_CHILDREN_TOTAL)
                .bind(id)
                .fetch_one(&ctx.pool)
                .await?;

        if child_count > 0 {
            return Err(IntentError::ActionNotAllowed(format!(
                "Task #{} has {} child tasks. Use --cascade to delete them too, or delete children first.",
                id,
                child_count
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

async fn handle_start(id: i64, description: Option<String>, format: String) -> Result<()> {
    let ctx = ProjectContext::load_or_init().await?;
    let project_path = ctx.root.to_string_lossy().to_string();
    let task_mgr = TaskManager::with_project_path(&ctx.pool, project_path);

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

async fn handle_done(id: Option<i64>, format: String) -> Result<()> {
    let ctx = ProjectContext::load_or_init().await?;
    let project_path = ctx.root.to_string_lossy().to_string();
    let task_mgr = TaskManager::with_project_path(&ctx.pool, project_path);

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
        match &result.next_step_suggestion {
            crate::db::models::NextStepSuggestion::ParentIsReady {
                message,
                parent_task_id,
                ..
            } => {
                println!("  Next: {} (ie task start {})", message, parent_task_id);
            },
            crate::db::models::NextStepSuggestion::SiblingTasksRemain {
                message,
                remaining_siblings_count,
                ..
            } => {
                println!(
                    "  Next: {} ({} siblings remaining)",
                    message, remaining_siblings_count
                );
            },
            crate::db::models::NextStepSuggestion::TopLevelTaskCompleted { message, .. } => {
                println!("  {}", message);
            },
            crate::db::models::NextStepSuggestion::NoParentContext { message, .. } => {
                println!("  {}", message);
            },
            crate::db::models::NextStepSuggestion::WorkspaceIsClear { message, .. } => {
                println!("  {}", message);
            },
        }
    }

    Ok(())
}

async fn handle_next(format: String) -> Result<()> {
    let ctx = ProjectContext::load_or_init().await?;
    let task_mgr = TaskManager::new(&ctx.pool);

    let result = task_mgr.pick_next().await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.format_as_text());
    }

    Ok(())
}

// ============================================================================
// Helper functions
// ============================================================================

/// Print a concise task summary
fn print_task_summary(task: &crate::db::models::Task) {
    let status_icon = match task.status.as_str() {
        "todo" => "○",
        "doing" => "●",
        "done" => "✓",
        _ => "?",
    };
    println!("  {} #{} {}", status_icon, task.id, task.name);
    println!("  Status: {}", task.status);
    if let Some(pid) = task.parent_id {
        println!("  Parent: #{}", pid);
    }
    if let Some(p) = task.priority {
        println!("  Priority: {}", p);
    }
    if let Some(spec) = &task.spec {
        if !spec.is_empty() {
            println!("  Spec: {}", spec);
        }
    }
    println!("  Owner: {}", task.owner);
    if let Some(af) = &task.active_form {
        println!("  Active form: {}", af);
    }
    if let Some(meta) = &task.metadata {
        println!("  Metadata: {}", meta);
    }
}

/// Print tasks in a hierarchical tree format
fn print_task_tree(tasks: &[crate::db::models::Task]) {
    use std::collections::HashMap;

    // Build parent -> children map
    let mut children_map: HashMap<Option<i64>, Vec<&crate::db::models::Task>> = HashMap::new();
    for task in tasks {
        children_map.entry(task.parent_id).or_default().push(task);
    }

    // Print tree starting from root tasks
    fn print_subtree(
        children_map: &HashMap<Option<i64>, Vec<&crate::db::models::Task>>,
        parent_id: Option<i64>,
        indent: &str,
        _is_last: bool,
    ) {
        if let Some(children) = children_map.get(&parent_id) {
            for (i, task) in children.iter().enumerate() {
                let is_last_child = i == children.len() - 1;
                let connector = if indent.is_empty() {
                    ""
                } else if is_last_child {
                    "└─ "
                } else {
                    "├─ "
                };
                let status_icon = match task.status.as_str() {
                    "todo" => "○",
                    "doing" => "●",
                    "done" => "✓",
                    _ => "?",
                };
                let priority_info = task
                    .priority
                    .map(|p| format!(" [P{}]", p))
                    .unwrap_or_default();

                println!(
                    "  {}{}{} #{} {}{}",
                    indent, connector, status_icon, task.id, task.name, priority_info
                );

                let new_indent = if indent.is_empty() {
                    "".to_string()
                } else if is_last_child {
                    format!("{}   ", indent)
                } else {
                    format!("{}│  ", indent)
                };
                print_subtree(children_map, Some(task.id), &new_indent, is_last_child);
            }
        }
    }

    // Start with root-level tasks (either parent_id is None or parent not in our set)
    let task_ids: std::collections::HashSet<i64> = tasks.iter().map(|t| t.id).collect();
    let roots: Vec<&crate::db::models::Task> = tasks
        .iter()
        .filter(|t| t.parent_id.is_none() || !task_ids.contains(&t.parent_id.unwrap_or(-1)))
        .collect();

    for (i, task) in roots.iter().enumerate() {
        let _is_last = i == roots.len() - 1;
        let status_icon = match task.status.as_str() {
            "todo" => "○",
            "doing" => "●",
            "done" => "✓",
            _ => "?",
        };
        let priority_info = task
            .priority
            .map(|p| format!(" [P{}]", p))
            .unwrap_or_default();
        println!(
            "  {} #{} {}{}",
            status_icon, task.id, task.name, priority_info
        );
        print_subtree(&children_map, Some(task.id), "  ", _is_last);
    }
}

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
