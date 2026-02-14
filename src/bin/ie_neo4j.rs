//! `ie-neo4j` — Intent-Engine with Neo4j graph database backend.
//!
//! This binary provides the same CLI interface as `ie`, but stores all data
//! in Neo4j instead of SQLite. It reuses the same types (Task, Event, etc.)
//! and CLI definitions from the main intent-engine crate.
//!
//! Usage:
//!   NEO4J_URI="neo4j+s://..." NEO4J_PASSWORD="..." ie-neo4j status
//!
//! Note on text formatting:
//!   The status/task display code here is intentionally a simplified subset of
//!   `src/main.rs` and `src/cli_handlers/task_commands.rs` formatting.
//!   When the Neo4j backend reaches feature parity, extract shared formatting
//!   functions to avoid duplication.

use clap::Parser;
use intent_engine::cli::{Cli, Commands, TaskCommands};
use intent_engine::db::models::{NoFocusResponse, TaskBrief, TaskSortBy};
use intent_engine::error::{IntentError, Result};
use intent_engine::neo4j::Neo4jContext;
use intent_engine::tasks::TaskUpdate;
use serde_json::json;

#[tokio::main]
async fn main() {
    #[cfg(windows)]
    if let Err(e) = intent_engine::windows_console::setup_windows_console() {
        eprintln!("Warning: Failed to setup Windows console UTF-8: {}", e);
    }

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        let error_response = e.to_error_response();
        eprintln!("{}", serde_json::to_string_pretty(&error_response).unwrap());
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Status {
            task_id,
            with_events,
            format,
        } => handle_status(task_id, with_events, format).await,

        Commands::Task(task_cmd) => handle_task_command(task_cmd).await,

        Commands::Log {
            event_type,
            message,
            task,
            format,
        } => handle_log(event_type, message, task, format).await,

        Commands::Plan { format } => handle_plan(format).await,

        Commands::Search {
            query,
            tasks,
            events,
            limit,
            offset,
            since: _,
            until: _,
            format,
        } => handle_search(query, tasks, events, limit, offset, format).await,

        _ => {
            eprintln!("Command not yet implemented for Neo4j backend.");
            eprintln!("Currently supported: ie-neo4j status, ie-neo4j task *, ie-neo4j log, ie-neo4j plan, ie-neo4j search");
            std::process::exit(1);
        },
    }
}

// ── Status ──────────────────────────────────────────────────────

async fn handle_status(task_id: Option<i64>, with_events: bool, format: String) -> Result<()> {
    let ctx = Neo4jContext::connect().await?;
    let task_mgr = ctx.task_manager();
    let workspace_mgr = ctx.workspace_manager();

    let target_task_id = if let Some(id) = task_id {
        Some(id)
    } else {
        let current = workspace_mgr.get_current_task(None).await?;
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
        },
        None => {
            let root_tasks = task_mgr.get_root_tasks().await?;
            let root_briefs: Vec<TaskBrief> = root_tasks.iter().map(TaskBrief::from).collect();

            let response = NoFocusResponse {
                message: "No focused task. Use 'ie-neo4j task start <id>' to focus a task."
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
                        let icon = status_icon(&task.status);
                        println!("   {} #{}: {} [{}]", icon, task.id, task.name, task.status);
                    }
                }
            }
        },
    }

    Ok(())
}

fn print_status_text(status: &intent_engine::db::models::StatusResponse) {
    let ft = &status.focused_task;
    let icon = status_icon(&ft.status);
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

    if !status.ancestors.is_empty() {
        println!("\n  Ancestors ({}):", status.ancestors.len());
        for ancestor in &status.ancestors {
            println!(
                "   #{}: {} [{}]",
                ancestor.id, ancestor.name, ancestor.status
            );
        }
    }

    if !status.siblings.is_empty() {
        println!("\n  Siblings ({}):", status.siblings.len());
        for sibling in &status.siblings {
            println!("   #{}: {} [{}]", sibling.id, sibling.name, sibling.status);
        }
    }

    if !status.descendants.is_empty() {
        println!("\n  Descendants ({}):", status.descendants.len());
        for desc in &status.descendants {
            println!("   #{}: {} [{}]", desc.id, desc.name, desc.status);
        }
    }
}

// ── Task Commands ───────────────────────────────────────────────

async fn handle_task_command(cmd: TaskCommands) -> Result<()> {
    match cmd {
        TaskCommands::Create {
            name,
            description,
            parent,
            status,
            priority,
            owner,
            metadata,
            format,
            ..
        } => {
            handle_task_create(
                name,
                description,
                parent,
                status,
                priority,
                owner,
                metadata,
                format,
            )
            .await
        },

        TaskCommands::Get {
            id,
            with_events: _,
            with_context: _,
            format,
        } => handle_task_get(id, format).await,

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
            format,
            ..
        } => {
            handle_task_update(
                id,
                name,
                description,
                status,
                priority,
                active_form,
                owner,
                parent,
                metadata,
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
            tree: _,
            format,
        } => handle_task_list(status, parent, sort, limit, offset, format).await,

        TaskCommands::Delete {
            id,
            cascade,
            format,
        } => handle_task_delete(id, cascade, format).await,

        TaskCommands::Start {
            id,
            description,
            format,
        } => handle_task_start(id, description, format).await,

        TaskCommands::Done { id, format } => handle_task_done(id, format).await,

        TaskCommands::Next { format } => handle_task_next(format).await,
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_task_create(
    name: String,
    description: Option<String>,
    parent: Option<i64>,
    status: String,
    priority: Option<i32>,
    owner: String,
    metadata: Vec<String>,
    format: String,
) -> Result<()> {
    let ctx = Neo4jContext::connect().await?;
    let task_mgr = ctx.task_manager();
    let workspace_mgr = ctx.workspace_manager();

    // Determine parent_id: 0 = root, N = specific parent, omit = current focus
    let parent_id = match parent {
        Some(0) => None,
        Some(p) => Some(p),
        None => {
            let current = workspace_mgr.get_current_task(None).await?;
            current.current_task_id
        },
    };

    // Parse metadata upfront so add_task creates everything in one shot
    let merged_metadata = if !metadata.is_empty() {
        let meta_json = parse_metadata(&metadata)?;
        merge_metadata(None, &meta_json)
    } else {
        None
    };

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

    // Handle status transitions
    if status == "doing" {
        let result = task_mgr.start_task(task.id, false).await?;
        task = result.task;
    } else if status == "done" {
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

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&task)?);
    } else {
        println!("Task created: #{} {}", task.id, task.name);
        print_task_summary(&task);
    }

    Ok(())
}

async fn handle_task_get(id: i64, format: String) -> Result<()> {
    let ctx = Neo4jContext::connect().await?;
    let task_mgr = ctx.task_manager();

    let task = task_mgr.get_task(id).await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&task)?);
    } else {
        print_task_summary(&task);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_task_update(
    id: i64,
    name: Option<String>,
    description: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
    active_form: Option<String>,
    owner: Option<String>,
    parent: Option<i64>,
    metadata: Vec<String>,
    format: String,
) -> Result<()> {
    let ctx = Neo4jContext::connect().await?;
    let task_mgr = ctx.task_manager();

    // Convert parent: 0 = root, N = set parent
    let parent_id_opt: Option<Option<i64>> = parent.map(|p| if p == 0 { None } else { Some(p) });

    // Handle "doing" status via start_task
    let effective_status = if status.as_deref() == Some("doing") {
        None
    } else {
        status.as_deref().map(String::from)
    };

    // Merge metadata
    let merged_metadata = if !metadata.is_empty() {
        let current_task = task_mgr.get_task(id).await?;
        let meta_json = parse_metadata(&metadata)?;
        merge_metadata(current_task.metadata.as_deref(), &meta_json)
    } else {
        None
    };

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

    // If status was "doing", use start_task
    if status.as_deref() == Some("doing") {
        let result = task_mgr.start_task(id, false).await?;
        task = result.task;
    }

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&task)?);
    } else {
        println!("Task updated: #{} {}", task.id, task.name);
        print_task_summary(&task);
    }

    Ok(())
}

async fn handle_task_list(
    status: Option<String>,
    parent: Option<i64>,
    sort: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    format: String,
) -> Result<()> {
    let ctx = Neo4jContext::connect().await?;
    let task_mgr = ctx.task_manager();

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

    let parent_id_opt: Option<Option<i64>> = parent.map(|p| if p == 0 { None } else { Some(p) });

    let result = task_mgr
        .find_tasks(status.as_deref(), parent_id_opt, sort_by, limit, offset)
        .await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "Tasks: {} total (showing {})",
            result.total_count,
            result.tasks.len()
        );
        println!();
        for task in &result.tasks {
            let icon = status_icon(&task.status);
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
                icon, task.id, task.name, parent_info, priority_info
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

async fn handle_task_delete(id: i64, cascade: bool, format: String) -> Result<()> {
    let ctx = Neo4jContext::connect().await?;
    let task_mgr = ctx.task_manager();

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
        // Check children before non-cascade delete
        let children = task_mgr.get_children(id).await?;
        if !children.is_empty() {
            return Err(IntentError::ActionNotAllowed(format!(
                "Task #{} has {} child tasks. Use --cascade to delete them too, or delete children first.",
                id, children.len()
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

async fn handle_task_start(id: i64, description: Option<String>, format: String) -> Result<()> {
    let ctx = Neo4jContext::connect().await?;
    let task_mgr = ctx.task_manager();

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
    }

    Ok(())
}

async fn handle_task_done(id: Option<i64>, format: String) -> Result<()> {
    let ctx = Neo4jContext::connect().await?;
    let task_mgr = ctx.task_manager();

    let result = if let Some(task_id) = id {
        task_mgr.done_task_by_id(task_id).await?
    } else {
        task_mgr.done_task().await?
    };

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let task = &result.completed_task;
        println!("Completed task #{} '{}'", task.id, task.name);

        use intent_engine::db::models::NextStepSuggestion;
        match &result.next_step_suggestion {
            NextStepSuggestion::ParentIsReady {
                message,
                parent_task_id,
                ..
            } => {
                println!(
                    "  Next: {} (ie-neo4j task start {})",
                    message, parent_task_id
                );
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

async fn handle_task_next(format: String) -> Result<()> {
    let ctx = Neo4jContext::connect().await?;
    let task_mgr = ctx.task_manager();

    let result = task_mgr.pick_next().await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.format_as_text());
    }

    Ok(())
}

// ── Log Command ─────────────────────────────────────────────────

async fn handle_log(
    event_type: intent_engine::cli::LogEventType,
    message: String,
    task: Option<i64>,
    format: String,
) -> Result<()> {
    let ctx = Neo4jContext::connect().await?;
    let event_mgr = ctx.event_manager();
    let workspace_mgr = ctx.workspace_manager();

    // Determine task_id: use --task flag, or fall back to current focused task
    let target_task_id = if let Some(tid) = task {
        tid
    } else {
        let current_response = workspace_mgr.get_current_task(None).await?;
        let current_task = current_response.task.ok_or_else(|| {
            IntentError::ActionNotAllowed(
                "No current task set. Use --task <ID> or start a task first.".to_string(),
            )
        })?;
        current_task.id
    };

    let event_type_str = event_type.as_str();

    let event = event_mgr
        .add_event(target_task_id, event_type_str, &message)
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

// ── Plan Command ────────────────────────────────────────────────

async fn handle_plan(format: String) -> Result<()> {
    use intent_engine::cli_handlers::read_stdin;
    use intent_engine::plan::{cleanup_included_files, process_file_includes, PlanRequest};

    // Read JSON from stdin
    let json_input = read_stdin()?;

    // Parse JSON into PlanRequest
    let mut request: PlanRequest = serde_json::from_str(&json_input)
        .map_err(|e| IntentError::InvalidInput(format!("Invalid JSON: {}", e)))?;

    // Process @file directives
    let file_include_result =
        process_file_includes(&mut request).map_err(IntentError::InvalidInput)?;

    let ctx = Neo4jContext::connect().await?;
    let workspace_mgr = ctx.workspace_manager();

    // Get current focused task for auto-parenting
    let current = workspace_mgr.get_current_task(None).await?;
    let mut executor = ctx.plan_executor();
    if let Some(current_task_id) = current.current_task_id {
        executor = executor.with_default_parent(current_task_id);
    }

    let result = executor.execute(&request).await?;

    // Clean up included files after successful execution
    if result.success && !file_include_result.files_to_delete.is_empty() {
        cleanup_included_files(&file_include_result.files_to_delete);
    }

    // Format output
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        if result.success {
            println!("Plan executed successfully");
            println!();
            println!("Created: {} tasks", result.created_count);
            println!("Updated: {} tasks", result.updated_count);
            if result.deleted_count > 0 {
                println!("Deleted: {} tasks", result.deleted_count);
            }
            if result.cascade_deleted_count > 0 {
                println!("Cascade deleted: {} tasks", result.cascade_deleted_count);
            }
            println!("Dependencies: {}", result.dependency_count);
            println!();
            println!("Task ID mapping:");
            for (name, id) in &result.task_id_map {
                println!("  {} -> #{}", name, id);
            }

            if !result.warnings.is_empty() {
                println!();
                println!("Warnings:");
                for warning in &result.warnings {
                    println!("  - {}", warning);
                }
            }

            if let Some(focused) = &result.focused_task {
                println!();
                println!("Current focus:");
                println!("  ID: {}", focused.task.id);
                println!("  Name: {}", focused.task.name);
                println!("  Status: {}", focused.task.status);
                if let Some(spec) = &focused.task.spec {
                    println!("  Spec: {}", spec);
                }
            }
        } else if let Some(error) = &result.error {
            eprintln!("Plan failed: {}", error);
        }
    }

    Ok(())
}

// ── Search Command ──────────────────────────────────────────────

async fn handle_search(
    query: String,
    include_tasks: bool,
    include_events: bool,
    limit: Option<i64>,
    offset: Option<i64>,
    format: String,
) -> Result<()> {
    let ctx = Neo4jContext::connect().await?;

    // Mode 1: #ID lookup
    if let Some(task_id) = parse_task_id_query(&query) {
        let task_mgr = ctx.task_manager();
        match task_mgr.get_task(task_id).await {
            Ok(task) => {
                if format == "json" {
                    println!("{}", serde_json::to_string_pretty(&task)?);
                } else {
                    println!("Task #{}", task.id);
                    print_task_summary(&task);
                }
                return Ok(());
            },
            Err(_) => {
                // Fall through to fulltext search
            },
        }
    }

    // Mode 2: Status keywords (todo/doing/done)
    if let Some(statuses) = parse_status_keywords(&query) {
        let task_mgr = ctx.task_manager();
        let mut all_tasks = Vec::new();
        for status in &statuses {
            let result = task_mgr
                .find_tasks(Some(status), None, None, limit, offset)
                .await?;
            all_tasks.extend(result.tasks);
        }

        // Sort by priority then id
        all_tasks.sort_by(|a, b| {
            let pri_a = a.priority.unwrap_or(999);
            let pri_b = b.priority.unwrap_or(999);
            pri_a.cmp(&pri_b).then_with(|| a.id.cmp(&b.id))
        });

        let max = limit.unwrap_or(100) as usize;
        if all_tasks.len() > max {
            all_tasks.truncate(max);
        }

        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&all_tasks)?);
        } else {
            let status_str = statuses.join(", ");
            println!(
                "Tasks with status [{}]: {} found",
                status_str,
                all_tasks.len()
            );
            println!();
            for task in &all_tasks {
                let icon = status_icon(&task.status);
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
                    icon, task.id, task.name, parent_info, priority_info
                );
            }
        }
        return Ok(());
    }

    // Mode 3: Fulltext search
    let search_mgr = ctx.search_manager();
    let results = search_mgr
        .search(&query, include_tasks, include_events, limit, offset)
        .await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        use intent_engine::db::models::SearchResult;

        println!(
            "Search: \"{}\" -> {} tasks, {} events (limit: {}, offset: {})",
            query, results.total_tasks, results.total_events, results.limit, results.offset
        );
        println!();

        for result in &results.results {
            match result {
                SearchResult::Task {
                    task,
                    match_snippet,
                    match_field,
                } => {
                    let icon = status_icon(&task.status);
                    println!("  {} #{} {} [{}]", icon, task.id, task.name, task.status);
                    println!("    Match ({}): {}", match_field, match_snippet);
                },
                SearchResult::Event {
                    event,
                    task_chain,
                    match_snippet,
                } => {
                    let chain_str: String = task_chain
                        .iter()
                        .map(|t| format!("#{} {}", t.id, t.name))
                        .collect::<Vec<_>>()
                        .join(" > ");
                    println!(
                        "  [Event] #{} ({}) on task #{}",
                        event.id, event.log_type, event.task_id
                    );
                    if !chain_str.is_empty() {
                        println!("    Chain: {}", chain_str);
                    }
                    println!("    Match: {}", match_snippet);
                },
            }
        }

        if results.has_more {
            println!(
                "\n  ... more results available (use --offset {})",
                results.offset + results.limit
            );
        }
    }

    Ok(())
}

/// Parse a #ID query (e.g., "#123")
fn parse_task_id_query(query: &str) -> Option<i64> {
    let query = query.trim();
    if !query.starts_with('#') || query.len() < 2 {
        return None;
    }
    query[1..].parse::<i64>().ok()
}

/// Check if query is a status keyword combination (todo, doing, done)
fn parse_status_keywords(query: &str) -> Option<Vec<String>> {
    let query_lower = query.to_lowercase();
    let words: Vec<&str> = query_lower.split_whitespace().collect();

    if words.is_empty() {
        return None;
    }

    let valid_statuses = ["todo", "doing", "done"];
    let mut statuses: Vec<String> = Vec::new();

    for word in words {
        if valid_statuses.contains(&word) {
            if !statuses.iter().any(|s| s == word) {
                statuses.push(word.to_string());
            }
        } else {
            return None;
        }
    }

    Some(statuses)
}

// ── Helpers ─────────────────────────────────────────────────────

fn status_icon(status: &str) -> &'static str {
    match status {
        "todo" => "○",
        "doing" => "●",
        "done" => "✓",
        _ => "?",
    }
}

fn print_task_summary(task: &intent_engine::db::models::Task) {
    let icon = status_icon(&task.status);
    println!("  {} #{} {}", icon, task.id, task.name);
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

/// Parse metadata key=value strings into a JSON object.
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
