use clap::Parser;
use intent_engine::cli::{Cli, Commands, DashboardCommands};
use intent_engine::cli_handlers::{
    handle_current_command, handle_dashboard_command, handle_doctor_command, handle_event_command,
    handle_init_command, handle_logs_command, handle_report_command, handle_search_command,
    handle_session_restore, handle_setup, handle_task_command, read_stdin,
};
use intent_engine::error::{IntentError, Result};
use intent_engine::events::EventManager;
use intent_engine::logging::LoggingConfig;
use intent_engine::plan::{PlanExecutor, PlanRequest};
use intent_engine::project::ProjectContext;
use intent_engine::tasks::TaskManager;
use intent_engine::workspace::WorkspaceManager;

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

    // Check if Dashboard is running with stdout redirected (e.g., started by MCP Server)
    // When MCP Server auto-starts Dashboard, stdout is redirected to /dev/null
    // Also support IE_DASHBOARD_LOG_FILE env var for testing
    if matches!(
        cli.command,
        Commands::Dashboard(DashboardCommands::Start { .. })
    ) {
        // Force enable file logging if env var is set (for testing)
        let force_file_log = std::env::var("IE_DASHBOARD_LOG_FILE").is_ok();

        // Check if stdout is not a TTY (redirected when started by MCP Server)
        if force_file_log || !atty::is(atty::Stream::Stdout) {
            use intent_engine::logging::{log_file_path, ApplicationMode};
            log_config = LoggingConfig::for_mode(ApplicationMode::Dashboard);
            log_config.file_output = Some(log_file_path(ApplicationMode::Dashboard));
        }
    }

    // Enable file logging for MCP Server mode (with graceful fallback)
    if matches!(cli.command, Commands::McpServer { .. }) {
        use intent_engine::logging::{log_file_path, ApplicationMode};
        log_config = LoggingConfig::for_mode(ApplicationMode::McpServer);

        // Try to get log file path and verify directory is writable
        // If it fails (e.g., HOME=/nonexistent in tests), fall back to stdout
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let path = log_file_path(ApplicationMode::McpServer);
            // Verify directory was created successfully
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    Some(path)
                } else {
                    None
                }
            } else {
                None
            }
        })) {
            Ok(Some(path)) => {
                log_config.file_output = Some(path);
            },
            _ => {
                // Directory creation failed, use stdout instead
                log_config.file_output = None;
            },
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

    // Continue with main application logic
    if let Err(e) = run(&cli).await {
        let error_response = e.to_error_response();
        eprintln!("{}", serde_json::to_string_pretty(&error_response).unwrap());
        std::process::exit(1);
    }
}

async fn run(cli: &Cli) -> Result<()> {
    match cli.command.clone() {
        Commands::Task(task_cmd) => handle_task_command(task_cmd).await?,
        Commands::Current { set, command } => handle_current_command(set, command).await?,
        Commands::Report {
            since,
            status,
            filter_name,
            filter_spec,
            format: _,
            summary_only,
        } => handle_report_command(since, status, filter_name, filter_spec, summary_only).await?,
        Commands::Event(event_cmd) => handle_event_command(event_cmd).await?,
        Commands::Search {
            query,
            tasks,
            events,
            limit,
            offset,
        } => handle_search_command(&query, tasks, events, limit, offset).await?,
        Commands::Doctor => handle_doctor_command().await?,
        Commands::Init { at, force } => handle_init_command(at, force).await?,
        Commands::Dashboard(dashboard_cmd) => handle_dashboard_command(dashboard_cmd).await?,
        Commands::McpServer { dashboard_port } => {
            // Run MCP server - this never returns unless there's an error
            // io::Error is automatically converted to IntentError::IoError via #[from]
            intent_engine::mcp::run(dashboard_port).await?;
        },
        Commands::SessionRestore {
            include_events,
            workspace,
        } => {
            handle_session_restore(include_events, workspace).await?;
        },
        Commands::Setup {
            target,
            scope,
            force,
            config_path,
        } => {
            handle_setup(target, &scope, force, config_path).await?;
        },

        Commands::Plan { format } => {
            // Read JSON from stdin
            let json_input = read_stdin()?;

            // Parse JSON into PlanRequest
            let request: PlanRequest = serde_json::from_str(&json_input)
                .map_err(|e| IntentError::InvalidInput(format!("Invalid JSON: {}", e)))?;

            // Execute the plan
            let ctx = ProjectContext::load_or_init().await?;
            let executor = PlanExecutor::new(&ctx.pool);
            let result = executor.execute(&request).await?;

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
                    println!("Dependencies: {}", result.dependency_count);
                    println!();
                    println!("Task ID mapping:");
                    for (name, id) in &result.task_id_map {
                        println!("  {} → #{}", name, id);
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

        // ========================================
        // Hybrid Commands - Forward to task/event handlers
        // ========================================
        Commands::Add {
            name,
            spec,
            parent,
            priority,
        } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);
            let mut task = task_mgr.add_task(&name, spec.as_deref(), parent).await?;

            // Update priority if specified
            if let Some(priority_str) = priority {
                use intent_engine::priority::PriorityLevel;
                let priority_value = PriorityLevel::parse_to_int(&priority_str)?;

                task = task_mgr
                    .update_task(task.id, None, None, None, None, None, Some(priority_value))
                    .await?;
            }

            println!("{}", serde_json::to_string_pretty(&task)?);
        },

        Commands::Start { id, with_events } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);
            let task = task_mgr.start_task(id, with_events).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        },

        Commands::Done => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);
            let task = task_mgr.done_task().await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        },

        Commands::Log {
            event_type,
            data,
            task_id,
        } => {
            let ctx = ProjectContext::load_or_init().await?;
            let event_mgr = EventManager::new(&ctx.pool);
            let workspace_mgr = WorkspaceManager::new(&ctx.pool);

            // Determine task_id: use provided, or fall back to current task
            let target_task_id = if let Some(tid) = task_id {
                tid
            } else {
                let current_response = workspace_mgr.get_current_task().await?;
                let current_task = current_response.task.ok_or_else(|| {
                    IntentError::ActionNotAllowed(
                        "No current task set. Use --task-id or start a task first.".to_string(),
                    )
                })?;
                current_task.id
            };

            let event = event_mgr
                .add_event(target_task_id, &event_type, &data)
                .await?;
            println!("{}", serde_json::to_string_pretty(&event)?);
        },

        Commands::Next { format } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);
            let response = task_mgr.pick_next().await?;

            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else if let Some(task) = response.task {
                println!("Recommended next task:");
                println!("  ID: {}", task.id);
                println!("  Name: {}", task.name);
                if let Some(priority) = task.priority {
                    use intent_engine::priority::PriorityLevel;
                    println!("  Priority: {}", PriorityLevel::to_str(priority));
                }
                if let Some(msg) = response.message {
                    println!("  Reason: {}", msg);
                }
            } else {
                println!("No task recommendation available");
                if let Some(msg) = response.message {
                    println!("Reason: {}", msg);
                }
            }
        },

        Commands::List {
            status,
            parent,
            sort_by,
            limit,
            offset,
        } => {
            let ctx = ProjectContext::load().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let parent_opt = parent.map(|p| {
                if p == "null" {
                    None
                } else {
                    Some(p.parse::<i64>().unwrap())
                }
            });

            // Parse sort_by parameter
            use intent_engine::db::models::TaskSortBy;
            let sort_by_parsed = match sort_by.as_deref() {
                Some("id") => Some(TaskSortBy::Id),
                Some("priority") => Some(TaskSortBy::Priority),
                Some("time") => Some(TaskSortBy::Time),
                Some("focus") => Some(TaskSortBy::FocusAware),
                None => None, // Use default from find_tasks
                Some(other) => {
                    return Err(IntentError::InvalidInput(format!(
                        "Invalid sort_by value '{}'. Valid values: id, priority, time, focus",
                        other
                    )));
                },
            };

            let result = task_mgr
                .find_tasks(status.as_deref(), parent_opt, sort_by_parsed, limit, offset)
                .await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        },

        Commands::Context { task_id } => {
            let ctx = ProjectContext::load().await?;
            let task_mgr = TaskManager::new(&ctx.pool);
            let workspace_mgr = WorkspaceManager::new(&ctx.pool);

            // If no task_id provided, use current task
            let target_id = if let Some(id) = task_id {
                id
            } else {
                let current_response = workspace_mgr.get_current_task().await?;
                let current_task = current_response.task.ok_or_else(|| {
                    IntentError::ActionNotAllowed(
                        "No current task set. Provide task_id or start a task first.".to_string(),
                    )
                })?;
                current_task.id
            };

            let context = task_mgr.get_task_context(target_id).await?;
            println!("{}", serde_json::to_string_pretty(&context)?);
        },

        Commands::Get { id, with_events } => {
            let ctx = ProjectContext::load().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            if with_events {
                let task = task_mgr.get_task_with_events(id).await?;
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                let task = task_mgr.get_task(id).await?;
                println!("{}", serde_json::to_string_pretty(&task)?);
            }
        },

        Commands::Logs {
            mode,
            level,
            since,
            until,
            limit,
            follow,
            export,
        } => {
            handle_logs_command(mode, level, since, until, limit, follow, export)?;
        },
    }

    Ok(())
}
