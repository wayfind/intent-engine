use clap::Parser;
use intent_engine::cli::{Cli, Commands, DashboardCommands};
use intent_engine::cli_handlers::{
    handle_dashboard_command, handle_doctor_command, handle_init_command, handle_search_command,
    read_stdin,
};
use intent_engine::error::{IntentError, Result};
use intent_engine::events::EventManager;
use intent_engine::logging::LoggingConfig;
use intent_engine::plan::{
    cleanup_included_files, process_file_includes, PlanExecutor, PlanRequest,
};
use intent_engine::project::ProjectContext;
use intent_engine::workspace::WorkspaceManager;
use std::io::IsTerminal;

#[tokio::main]
async fn main() {
    // Setup Windows console for UTF-8 output
    // This ensures Chinese and other non-ASCII characters display correctly
    #[cfg(windows)]
    if let Err(e) = intent_engine::windows_console::setup_windows_console() {
        eprintln!("Warning: Failed to setup Windows console UTF-8: {}", e);
        eprintln!(
            "Chinese characters may not display correctly. Consider running 'chcp 65001' first."
        );
    }

    // Parse CLI arguments first to get logging configuration
    let cli = Cli::parse();

    // Initialize logging system
    let mut log_config = LoggingConfig::from_args(cli.quiet, cli.verbose > 0, cli.json);

    // Check if Dashboard is running with stdout redirected
    // Also support IE_DASHBOARD_LOG_FILE env var for testing
    if matches!(
        cli.command,
        Commands::Dashboard(DashboardCommands::Start { .. })
    ) {
        // Force enable file logging if env var is set (for testing)
        let force_file_log = std::env::var("IE_DASHBOARD_LOG_FILE").is_ok();

        // Check if stdout is not a TTY (redirected)
        if force_file_log || !std::io::stdout().is_terminal() {
            use intent_engine::logging::{log_file_path, ApplicationMode};
            log_config = LoggingConfig::for_mode(ApplicationMode::Dashboard);
            log_config.file_output = Some(log_file_path(ApplicationMode::Dashboard));
        }
    }

    if let Err(e) = intent_engine::logging::init_logging(log_config) {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }

    // Clean up old log files for Dashboard mode (after logging init)
    if matches!(
        cli.command,
        Commands::Dashboard(DashboardCommands::Start { .. })
    ) {
        use intent_engine::logging::cleanup_old_logs;
        let log_dir = dirs::home_dir().map(|h| h.join(".intent-engine").join("logs"));

        if let Some(dir) = log_dir {
            // Default retention: 7 days
            let retention_days = std::env::var("IE_LOG_RETENTION_DAYS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(7);

            cleanup_old_logs(&dir, retention_days).ok();
        }
    }

    // Note: Dashboard auto-start disabled by default (v0.10.1+)
    // Users should manually start Dashboard with: ie dashboard start

    // Continue with main application logic
    if let Err(e) = run(&cli).await {
        let error_response = e.to_error_response();
        eprintln!("{}", serde_json::to_string_pretty(&error_response).unwrap());
        std::process::exit(1);
    }
}

async fn run(cli: &Cli) -> Result<()> {
    match cli.command.clone() {
        Commands::Plan { format } => {
            // Read JSON from stdin
            let json_input = read_stdin()?;

            // Parse JSON into PlanRequest
            let mut request: PlanRequest = serde_json::from_str(&json_input)
                .map_err(|e| IntentError::InvalidInput(format!("Invalid JSON: {}", e)))?;

            // Process @file directives - replace @file(path) with file contents
            let file_include_result =
                process_file_includes(&mut request).map_err(IntentError::InvalidInput)?;

            // Execute the plan
            let ctx = ProjectContext::load_or_init().await?;
            let project_path = ctx.root.to_string_lossy().to_string();

            // Get current focused task for auto-parenting
            let workspace_mgr = WorkspaceManager::new(&ctx.pool);
            let current = workspace_mgr.get_current_task(None).await?;

            // Create executor with project path, then optionally set default parent
            let mut executor = PlanExecutor::with_project_path(&ctx.pool, project_path);
            if let Some(current_task_id) = current.current_task_id {
                // Auto-parent new root tasks to the focused task
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
                // Text format
                if result.success {
                    println!("✓ Plan executed successfully");
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
                        println!("  {} → #{}", name, id);
                    }

                    // Display warnings if any
                    if !result.warnings.is_empty() {
                        println!();
                        println!("⚠ Warnings:");
                        for warning in &result.warnings {
                            println!("  - {}", warning);
                        }
                    }

                    // Display focused task if present
                    if let Some(focused) = &result.focused_task {
                        println!();
                        println!("✓ Current focus:");
                        println!("  ID: {}", focused.task.id);
                        println!("  Name: {}", focused.task.name);
                        println!("  Status: {}", focused.task.status);
                        if let Some(parent_id) = focused.task.parent_id {
                            println!("  Parent: #{}", parent_id);
                        }
                        if let Some(priority) = focused.task.priority {
                            println!("  Priority: {}", priority);
                        }
                        if let Some(spec) = &focused.task.spec {
                            println!("  Spec: {}", spec);
                        }
                        println!("  Owner: {}", focused.task.owner);
                        if let Some(ts) = focused.task.first_todo_at {
                            println!("  First todo: {}", ts.format("%Y-%m-%d %H:%M:%S UTC"));
                        }
                        if let Some(ts) = focused.task.first_doing_at {
                            println!("  First doing: {}", ts.format("%Y-%m-%d %H:%M:%S UTC"));
                        }
                        if let Some(ts) = focused.task.first_done_at {
                            println!("  First done: {}", ts.format("%Y-%m-%d %H:%M:%S UTC"));
                        }

                        // Display event summary if present
                        if let Some(events_summary) = &focused.events_summary {
                            println!();
                            println!("  Event history:");
                            println!("    Total events: {}", events_summary.total_count);
                            if !events_summary.recent_events.is_empty() {
                                println!("    Recent:");
                                for event in events_summary.recent_events.iter().take(3) {
                                    println!(
                                        "      [{}] {}: {}",
                                        event.log_type,
                                        event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                                        event.discussion_data
                                    );
                                }
                            }
                        }
                    }
                } else {
                    eprintln!("✗ Plan execution failed");
                    if let Some(error) = result.error {
                        eprintln!("Error: {}", error);
                    }
                    std::process::exit(1);
                }
            }
        },

        Commands::Log {
            event_type,
            message,
            task,
            format,
        } => {
            let ctx = ProjectContext::load_or_init().await?;
            let project_path = ctx.root.to_string_lossy().to_string();
            let event_mgr = EventManager::with_project_path(&ctx.pool, project_path);
            let workspace_mgr = WorkspaceManager::new(&ctx.pool);

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

            // Convert LogEventType to string using as_str() method
            let event_type_str = event_type.as_str();

            // Add event using existing business logic
            let event = event_mgr
                .add_event(target_task_id, event_type_str, &message)
                .await?;

            // Format output
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&event)?);
            } else {
                println!("✓ Event recorded");
                println!("  ID: {}", event.id);
                println!("  Type: {}", event_type_str);
                println!("  Task: #{}", target_task_id);
                println!(
                    "  Time: {}",
                    event.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
                );
                println!("  Message: {}", message);
            }
        },

        Commands::Search {
            query,
            tasks,
            events,
            limit,
            offset,
            since,
            until,
            format,
        } => {
            handle_search_command(&query, tasks, events, limit, offset, since, until, &format)
                .await?
        },

        Commands::Init { at, force } => handle_init_command(at, force).await?,

        Commands::Dashboard(dashboard_cmd) => handle_dashboard_command(dashboard_cmd).await?,

        Commands::Doctor => handle_doctor_command().await?,

        Commands::Status {
            task_id,
            with_events,
            format,
        } => {
            use intent_engine::db::models::{NoFocusResponse, TaskBrief};
            use intent_engine::tasks::TaskManager;

            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);
            let workspace_mgr = WorkspaceManager::new(&ctx.pool);

            // Determine which task to show status for
            let target_task_id = if let Some(id) = task_id {
                // Explicit task ID provided
                Some(id)
            } else {
                // Use current focused task
                let current = workspace_mgr.get_current_task(None).await?;
                current.current_task_id
            };

            match target_task_id {
                Some(id) => {
                    // Get status for the specified task
                    let status = task_mgr.get_status(id, with_events).await?;

                    if format == "json" {
                        println!("{}", serde_json::to_string_pretty(&status)?);
                    } else {
                        // Text format - focused task
                        let ft = &status.focused_task;
                        println!("🔦 Task #{}: {}", ft.id, ft.name);
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
                            println!("\n📍 Ancestors ({}):", status.ancestors.len());
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
                                    ancestor.id,
                                    ancestor.name,
                                    ancestor.status,
                                    parent_info,
                                    priority_info
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
                            println!("\n👥 Siblings ({}):", status.siblings.len());
                            for sibling in &status.siblings {
                                let parent_info = sibling
                                    .parent_id
                                    .map(|p| format!(" (parent: #{})", p))
                                    .unwrap_or_default();
                                let spec_indicator = if sibling.has_spec { "" } else { " ⚠️" };
                                println!(
                                    "   #{}: {} [{}]{}{}",
                                    sibling.id,
                                    sibling.name,
                                    sibling.status,
                                    parent_info,
                                    spec_indicator
                                );
                            }
                        }

                        if !status.descendants.is_empty() {
                            println!("\n📦 Descendants ({}):", status.descendants.len());
                            for desc in &status.descendants {
                                let parent_info = desc
                                    .parent_id
                                    .map(|p| format!(" (parent: #{})", p))
                                    .unwrap_or_default();
                                let spec_indicator = if desc.has_spec { "" } else { " ⚠️" };
                                println!(
                                    "   #{}: {} [{}]{}{}",
                                    desc.id, desc.name, desc.status, parent_info, spec_indicator
                                );
                            }
                        }

                        if let Some(events) = &status.events {
                            println!("\n📝 Events ({}):", events.len());
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
                },
                None => {
                    // No focused task - show root tasks
                    let root_tasks = task_mgr.get_root_tasks().await?;
                    let root_briefs: Vec<TaskBrief> =
                        root_tasks.iter().map(TaskBrief::from).collect();

                    let response = NoFocusResponse {
                        message:
                            "No focused task. Use 'ie plan' with status:'doing' to start a task."
                                .to_string(),
                        root_tasks: root_briefs,
                    };

                    if format == "json" {
                        println!("{}", serde_json::to_string_pretty(&response)?);
                    } else {
                        println!("⚠️  {}", response.message);
                        if !response.root_tasks.is_empty() {
                            println!("\n📋 Root tasks:");
                            for task in &response.root_tasks {
                                let spec_indicator = if task.has_spec { "" } else { " ⚠️" };
                                println!(
                                    "   #{}: {} [{}]{}",
                                    task.id, task.name, task.status, spec_indicator
                                );
                            }
                        }
                    }
                },
            }
        },
    }

    Ok(())
}
