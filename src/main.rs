use clap::Parser;
use intent_engine::cli::{
    Cli, Commands, CurrentAction, DashboardCommands, EventCommands, TaskCommands,
};
use intent_engine::db::models::TaskContext;
use intent_engine::error::{IntentError, Result};
use intent_engine::events::EventManager;
use intent_engine::logging::LoggingConfig;
use intent_engine::plan::{PlanExecutor, PlanRequest};
use intent_engine::project::ProjectContext;
use intent_engine::report::ReportManager;
use intent_engine::sql_constants;
use intent_engine::tasks::TaskManager;
use intent_engine::workspace::WorkspaceManager;
use sqlx::Row;
use std::io::{self, Read};
use std::path::PathBuf;
use std::str::FromStr;

/// Dashboard server default port
const DASHBOARD_PORT: u16 = 11391;

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

    // Check if this is dashboard mode with stdout redirected (daemon mode)
    // In daemon mode, parent spawns child with --foreground but redirects stdout to /dev/null
    // Also support IE_DASHBOARD_LOG_FILE env var for testing
    if matches!(
        cli.command,
        Commands::Dashboard(DashboardCommands::Start { .. })
    ) {
        // Force enable file logging if env var is set (for testing)
        let force_file_log = std::env::var("IE_DASHBOARD_LOG_FILE").is_ok();

        // Check if stdout is not a TTY (redirected to /dev/null in daemon mode)
        if force_file_log || !atty::is(atty::Stream::Stdout) {
            use intent_engine::logging::{log_file_path, ApplicationMode};
            log_config = LoggingConfig::for_mode(ApplicationMode::Dashboard);
            log_config.file_output = Some(log_file_path(ApplicationMode::Dashboard));
        }
    }

    // Enable file logging for MCP Server mode (with graceful fallback)
    if matches!(cli.command, Commands::McpServer) {
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
        } => handle_search_command(&query, tasks, events, limit).await?,
        Commands::Doctor => handle_doctor_command().await?,
        Commands::Init { at, force } => handle_init_command(at, force).await?,
        Commands::Dashboard(dashboard_cmd) => handle_dashboard_command(dashboard_cmd).await?,
        Commands::McpServer => {
            // Run MCP server - this never returns unless there's an error
            // io::Error is automatically converted to IntentError::IoError via #[from]
            intent_engine::mcp::run().await?;
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
            diagnose,
            config_path,
        } => {
            handle_setup(target, &scope, force, diagnose, config_path).await?;
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

        Commands::List { status, parent } => {
            let ctx = ProjectContext::load().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let parent_opt = parent.map(|p| {
                if p == "null" {
                    None
                } else {
                    Some(p.parse::<i64>().unwrap())
                }
            });

            let tasks = task_mgr.find_tasks(status.as_deref(), parent_opt).await?;
            println!("{}", serde_json::to_string_pretty(&tasks)?);
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

async fn handle_task_command(cmd: TaskCommands) -> Result<()> {
    match cmd {
        TaskCommands::Add {
            name,
            parent,
            spec_stdin,
        } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let spec = if spec_stdin {
                Some(read_stdin()?)
            } else {
                None
            };

            let task = task_mgr.add_task(&name, spec.as_deref(), parent).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        },

        TaskCommands::Get { id, with_events } => {
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

        TaskCommands::Update {
            id,
            name,
            parent,
            status,
            complexity,
            priority,
            spec_stdin,
        } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let spec = if spec_stdin {
                Some(read_stdin()?)
            } else {
                None
            };

            // Convert priority string to integer
            let priority_int = match &priority {
                Some(p) => Some(intent_engine::priority::PriorityLevel::parse_to_int(p)?),
                None => None,
            };

            let parent_opt = parent.map(Some);
            let task = task_mgr
                .update_task(
                    id,
                    name.as_deref(),
                    spec.as_deref(),
                    parent_opt,
                    status.as_deref(),
                    complexity,
                    priority_int,
                )
                .await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        },

        TaskCommands::Del { id } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            task_mgr.delete_task(id).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "message": format!("Task {} deleted", id)
                }))?
            );
        },

        TaskCommands::List { status, parent } => {
            let ctx = ProjectContext::load().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let parent_opt = parent.map(|p| {
                if p == "null" {
                    None
                } else {
                    p.parse::<i64>().ok()
                }
            });

            let tasks = task_mgr.find_tasks(status.as_deref(), parent_opt).await?;
            println!("{}", serde_json::to_string_pretty(&tasks)?);
        },

        TaskCommands::Start { id, with_events } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let task = task_mgr.start_task(id, with_events).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        },

        TaskCommands::Done => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let task = task_mgr.done_task().await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        },

        TaskCommands::PickNext { format } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let response = task_mgr.pick_next().await?;

            // Output based on format
            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&response)?);
                },
                _ => {
                    // Default to text format
                    println!("{}", response.format_as_text());
                },
            }
        },

        TaskCommands::SpawnSubtask { name, spec_stdin } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let spec = if spec_stdin {
                Some(read_stdin()?)
            } else {
                None
            };

            let subtask = task_mgr.spawn_subtask(&name, spec.as_deref()).await?;
            println!("{}", serde_json::to_string_pretty(&subtask)?);
        },

        TaskCommands::DependsOn {
            blocked_task_id,
            blocking_task_id,
        } => {
            let ctx = ProjectContext::load().await?;

            let dependency = intent_engine::dependencies::add_dependency(
                &ctx.pool,
                blocking_task_id,
                blocked_task_id,
            )
            .await?;

            let response = serde_json::json!({
                "dependency_id": dependency.id,
                "blocking_task_id": dependency.blocking_task_id,
                "blocked_task_id": dependency.blocked_task_id,
                "created_at": dependency.created_at,
                "message": format!(
                    "Dependency added: Task {} now depends on Task {}",
                    blocked_task_id, blocking_task_id
                )
            });

            println!("{}", serde_json::to_string_pretty(&response)?);
        },

        TaskCommands::Context { task_id } => {
            let ctx = ProjectContext::load().await?;
            let task_mgr = TaskManager::new(&ctx.pool);
            let workspace_mgr = WorkspaceManager::new(&ctx.pool);

            // If no task_id provided, use current task
            let target_id = if let Some(id) = task_id {
                id
            } else {
                let current = workspace_mgr.get_current_task().await?;
                current.current_task_id.ok_or_else(|| {
                    IntentError::InvalidInput(
                        "No task currently focused. Use 'ie task start <ID>' or provide task_id"
                            .to_string(),
                    )
                })?
            };

            let context = task_mgr.get_task_context(target_id).await?;

            // Format and print the context
            print_task_context(&context)?;
        },
    }

    Ok(())
}

async fn handle_current_command(set: Option<i64>, command: Option<CurrentAction>) -> Result<()> {
    let ctx = ProjectContext::load().await?;
    let workspace_mgr = WorkspaceManager::new(&ctx.pool);

    // Handle backward compatibility: --set flag takes precedence
    if let Some(task_id) = set {
        eprintln!("⚠️  Warning: 'ie current --set' is a low-level atomic command.");
        eprintln!(
            "   For normal use, prefer 'ie task start {}' which ensures data consistency.",
            task_id
        );
        eprintln!();
        let response = workspace_mgr.set_current_task(task_id).await?;
        println!("✓ Switched to task #{}", task_id);
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }

    // Handle subcommands
    match command {
        Some(CurrentAction::Set { task_id }) => {
            eprintln!("⚠️  Warning: 'ie current set' is a low-level atomic command.");
            eprintln!(
                "   For normal use, prefer 'ie task start {}' which ensures data consistency.",
                task_id
            );
            eprintln!();
            let response = workspace_mgr.set_current_task(task_id).await?;
            println!("✓ Switched to task #{}", task_id);
            println!("{}", serde_json::to_string_pretty(&response)?);
        },
        Some(CurrentAction::Clear) => {
            eprintln!("⚠️  Warning: 'ie current clear' is a low-level atomic command.");
            eprintln!("   For normal use, prefer 'ie task done' or 'ie task switch' which ensures data consistency.");
            eprintln!();
            sqlx::query("DELETE FROM workspace_state WHERE key = 'current_task_id'")
                .execute(&ctx.pool)
                .await?;
            println!("✓ Current task cleared");
        },
        None => {
            // Default: display current task in JSON format
            let response = workspace_mgr.get_current_task().await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        },
    }

    Ok(())
}

async fn handle_report_command(
    since: Option<String>,
    status: Option<String>,
    filter_name: Option<String>,
    filter_spec: Option<String>,
    summary_only: bool,
) -> Result<()> {
    let ctx = ProjectContext::load().await?;
    let report_mgr = ReportManager::new(&ctx.pool);

    let report = report_mgr
        .generate_report(since, status, filter_name, filter_spec, summary_only)
        .await?;
    println!("{}", serde_json::to_string_pretty(&report)?);

    Ok(())
}

async fn handle_event_command(cmd: EventCommands) -> Result<()> {
    match cmd {
        EventCommands::Add {
            task_id,
            log_type,
            data_stdin,
        } => {
            let ctx = ProjectContext::load_or_init().await?;
            let event_mgr = EventManager::new(&ctx.pool);

            let data = if data_stdin {
                read_stdin()?
            } else {
                return Err(IntentError::InvalidInput(
                    "--data-stdin is required".to_string(),
                ));
            };

            // Determine the target task ID
            let target_task_id = if let Some(id) = task_id {
                // Use the provided task_id
                id
            } else {
                // Fall back to current_task_id
                let current_task_id: Option<String> = sqlx::query_scalar(
                    "SELECT value FROM workspace_state WHERE key = 'current_task_id'",
                )
                .fetch_optional(&ctx.pool)
                .await?;

                current_task_id
                    .and_then(|s| s.parse::<i64>().ok())
                    .ok_or_else(|| IntentError::InvalidInput(
                        "No current task is set and --task-id was not provided. Use 'current --set <ID>' to set a task first.".to_string(),
                    ))?
            };

            let event = event_mgr
                .add_event(target_task_id, &log_type, &data)
                .await?;
            println!("{}", serde_json::to_string_pretty(&event)?);
        },

        EventCommands::List {
            task_id,
            limit,
            log_type,
            since,
        } => {
            let ctx = ProjectContext::load().await?;
            let event_mgr = EventManager::new(&ctx.pool);

            let events = event_mgr
                .list_events(task_id, limit, log_type, since)
                .await?;
            println!("{}", serde_json::to_string_pretty(&events)?);
        },
    }

    Ok(())
}

fn read_stdin() -> Result<String> {
    // On Windows, PowerShell 5.x may send GBK-encoded data through pipes
    // even though we set console encoding. We need to handle this gracefully.

    #[cfg(windows)]
    {
        use encoding_rs::GBK;

        // Try reading as UTF-8 first
        let mut buffer = String::new();
        match io::stdin().read_to_string(&mut buffer) {
            Ok(_) => return Ok(buffer.trim().to_string()),
            Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                // UTF-8 decode failed, try GBK on Windows
                eprintln!("Warning: Input is not valid UTF-8, attempting GBK decoding...");

                // Read as raw bytes
                let mut bytes = Vec::new();
                io::stdin().read_to_end(&mut bytes)?;

                // Try to decode as GBK
                let (decoded, _encoding, had_errors) = GBK.decode(&bytes);

                if had_errors {
                    return Err(IntentError::InvalidInput(format!(
                        "Input contains invalid characters. {}\n\n{}\n{}\n{}",
                        "On Windows PowerShell, pipe encoding may not be UTF-8.",
                        "To fix this, run one of the following before your command:",
                        "  [Console]::InputEncoding = [System.Text.Encoding]::UTF8",
                        "  [Console]::OutputEncoding = [System.Text.Encoding]::UTF8"
                    )));
                }

                return Ok(decoded.trim().to_string());
            },
            Err(e) => return Err(e.into()),
        }
    }

    #[cfg(not(windows))]
    {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer.trim().to_string())
    }
}

async fn handle_search_command(
    query: &str,
    include_tasks: bool,
    include_events: bool,
    limit: Option<i64>,
) -> Result<()> {
    use intent_engine::search::SearchManager;

    let ctx = ProjectContext::load_or_init().await?;
    let search_mgr = SearchManager::new(&ctx.pool);

    let results = search_mgr
        .unified_search(query, include_tasks, include_events, limit)
        .await?;

    println!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}

async fn handle_doctor_command() -> Result<()> {
    use serde_json::json;

    let mut checks = vec![];

    // 1. Database Location
    let db_path_info = ProjectContext::get_database_path_info();
    checks.push(json!({
        "check": "Database Location",
        "status": "✓ INFO",
        "details": db_path_info
    }));

    // 2. Database Health
    match ProjectContext::load_or_init().await {
        Ok(ctx) => {
            match sqlx::query(sql_constants::COUNT_TASKS_TOTAL)
                .fetch_one(&ctx.pool)
                .await
            {
                Ok(row) => {
                    let count: i64 = row.try_get(0).unwrap_or(0);
                    checks.push(json!({
                        "check": "Database Health",
                        "status": "✓ PASS",
                        "details": {
                            "connected": true,
                            "tasks_count": count,
                            "message": format!("Database operational with {} tasks", count)
                        }
                    }));
                },
                Err(e) => {
                    checks.push(json!({
                        "check": "Database Health",
                        "status": "✗ FAIL",
                        "details": {"error": format!("Query failed: {}", e)}
                    }));
                },
            }
        },
        Err(e) => {
            checks.push(json!({
                "check": "Database Health",
                "status": "✗ FAIL",
                "details": {"error": format!("Failed to load database: {}", e)}
            }));
        },
    }

    // 3-5. New checks
    checks.push(check_dashboard_status().await);
    checks.push(check_mcp_connections().await);
    checks.push(check_session_start_hook());

    // Status summary
    let has_failures = checks
        .iter()
        .any(|c| c["status"].as_str().unwrap_or("").contains("✗ FAIL"));
    let has_warnings = checks
        .iter()
        .any(|c| c["status"].as_str().unwrap_or("").contains("⚠ WARNING"));

    let summary = if has_failures {
        "✗ Critical issues detected"
    } else if has_warnings {
        "⚠ Some optional features need attention"
    } else {
        "✓ All systems operational"
    };

    let result = json!({
        "summary": summary,
        "overall_status": if has_failures { "unhealthy" }
                         else if has_warnings { "warnings" }
                         else { "healthy" },
        "checks": checks
    });

    println!("{}", serde_json::to_string_pretty(&result)?);

    if has_failures {
        std::process::exit(1);
    }

    Ok(())
}

async fn handle_init_command(at: Option<String>, force: bool) -> Result<()> {
    use serde_json::json;

    // Determine target directory
    let target_dir = if let Some(path) = &at {
        let p = PathBuf::from(path);
        if !p.exists() {
            return Err(IntentError::InvalidInput(format!(
                "Directory does not exist: {}",
                path
            )));
        }
        if !p.is_dir() {
            return Err(IntentError::InvalidInput(format!(
                "Path is not a directory: {}",
                path
            )));
        }
        p
    } else {
        // Use current working directory
        std::env::current_dir().expect("Failed to get current directory")
    };

    let intent_dir = target_dir.join(".intent-engine");

    // Check if already exists
    if intent_dir.exists() && !force {
        let error_msg = format!(
            ".intent-engine already exists at {}\nUse --force to re-initialize",
            intent_dir.display()
        );
        return Err(IntentError::InvalidInput(error_msg));
    }

    // Perform initialization
    let ctx = ProjectContext::initialize_project_at(target_dir).await?;

    // Success output
    let result = json!({
        "success": true,
        "root": ctx.root.display().to_string(),
        "database_path": ctx.db_path.display().to_string(),
        "message": "Intent-Engine initialized successfully"
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

async fn handle_session_restore(include_events: usize, workspace: Option<String>) -> Result<()> {
    use intent_engine::session_restore::SessionRestoreManager;

    // If workspace path is specified, change to that directory
    if let Some(ws_path) = workspace {
        std::env::set_current_dir(&ws_path)?;
    }

    // Try to load project context
    let ctx = match ProjectContext::load().await {
        Ok(ctx) => ctx,
        Err(_) => {
            // Workspace not found
            let result = intent_engine::session_restore::SessionRestoreResult {
                status: intent_engine::session_restore::SessionStatus::Error,
                workspace_path: std::env::current_dir()
                    .ok()
                    .and_then(|p| p.to_str().map(String::from)),
                current_task: None,
                parent_task: None,
                siblings: None,
                children: None,
                recent_events: None,
                suggested_commands: Some(vec![
                    "ie workspace init".to_string(),
                    "ie help".to_string(),
                ]),
                stats: None,
                error_type: Some(intent_engine::session_restore::ErrorType::WorkspaceNotFound),
                message: Some("No Intent-Engine workspace found in current directory".to_string()),
                recovery_suggestion: Some(
                    "Run 'ie workspace init' to create a new workspace".to_string(),
                ),
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        },
    };

    let restore_mgr = SessionRestoreManager::new(&ctx.pool);
    let result = restore_mgr.restore(include_events).await?;

    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}

async fn handle_setup(
    target: Option<String>,
    scope: &str,
    force: bool,
    diagnose: bool,
    config_path: Option<String>,
) -> Result<()> {
    use intent_engine::setup::claude_code::ClaudeCodeSetup;
    use intent_engine::setup::{SetupModule, SetupOptions, SetupScope};

    println!("Intent-Engine Unified Setup");
    println!("============================\n");

    // Parse scope
    let setup_scope = SetupScope::from_str(scope)?;

    // Build options
    let opts = SetupOptions {
        scope: setup_scope,
        force,
        config_path: config_path.map(PathBuf::from),
    };

    // Determine target (interactive if not specified)
    let target_tool = if let Some(t) = target {
        // Direct mode: target specified via CLI
        t
    } else {
        // Interactive mode: launch wizard
        use intent_engine::setup::interactive::SetupWizard;

        let wizard = SetupWizard::new();
        let result = wizard.run(&opts)?;

        // Print result and exit
        if result.success {
            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("✅ {}", result.message);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

            if !result.files_modified.is_empty() {
                println!("Files modified:");
                for file in &result.files_modified {
                    println!("  - {}", file.display());
                }
                println!();
            }

            if let Some(test) = result.connectivity_test {
                if test.passed {
                    println!("✓ Connectivity test: {}", test.details);
                } else {
                    println!("✗ Connectivity test: {}", test.details);
                }
                println!();
            }

            println!("Next steps:");
            println!("  - Restart Claude Code to load MCP server");
            println!("  - Run 'ie doctor' to verify configuration");
            println!("  - Try 'ie task add --name \"Test task\"'");
            println!();
        } else {
            println!("\n{}", result.message);
        }

        return Ok(());
    };

    // Diagnose mode
    if diagnose {
        return handle_setup_diagnose(&target_tool);
    }

    // Setup mode
    match target_tool.as_str() {
        "claude-code" => {
            let setup = ClaudeCodeSetup;
            let result = setup.setup(&opts)?;

            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("✅ {}", result.message);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

            println!("Files modified:");
            for file in &result.files_modified {
                println!("  - {}", file.display());
            }

            if let Some(conn_test) = result.connectivity_test {
                println!("\nConnectivity test:");
                if conn_test.passed {
                    println!("  ✅ {}", conn_test.details);
                } else {
                    println!("  ⚠️  {}", conn_test.details);
                }
            }

            println!("\nNext steps:");
            println!("  1. Restart Claude Code completely");
            println!("  2. Open a new session in a project directory");
            println!("  3. You should see Intent-Engine context restored");
            println!("\nTo verify setup:");
            println!("  ie setup --target claude-code --diagnose");

            Ok(())
        },
        "gemini-cli" | "codex" => {
            println!("⚠️  Target '{}' is not yet supported.", target_tool);
            println!("Currently supported: claude-code");
            Err(IntentError::InvalidInput(format!(
                "Unsupported target: {}",
                target_tool
            )))
        },
        _ => Err(IntentError::InvalidInput(format!(
            "Unknown target: {}. Available: claude-code, gemini-cli, codex",
            target_tool
        ))),
    }
}

fn handle_setup_diagnose(target: &str) -> Result<()> {
    use intent_engine::setup::claude_code::ClaudeCodeSetup;
    use intent_engine::setup::SetupModule;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔍 Setup Diagnosis for '{}'", target);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    match target {
        "claude-code" => {
            let setup = ClaudeCodeSetup;
            let report = setup.diagnose()?;

            println!("Check results:\n");
            for check in &report.checks {
                let status = if check.passed { "✅" } else { "❌" };
                println!("{} {}", status, check.name);
                println!("   {}", check.details);
            }

            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            if report.overall_status {
                println!("✅ All checks passed!");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

                // Run connectivity test
                println!("Running connectivity test...\n");
                match setup.test_connectivity() {
                    Ok(result) => {
                        if result.passed {
                            println!("✅ {}", result.details);
                        } else {
                            println!("⚠️  {}", result.details);
                        }
                    },
                    Err(e) => {
                        println!("❌ Connectivity test failed: {}", e);
                    },
                }
            } else {
                println!("❌ Some checks failed");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

                if !report.suggested_fixes.is_empty() {
                    println!("Suggested fixes:");
                    for fix in &report.suggested_fixes {
                        println!("  • {}", fix);
                    }
                }
            }

            Ok(())
        },
        _ => Err(IntentError::InvalidInput(format!(
            "Diagnosis not supported for target: {}",
            target
        ))),
    }
}

/// Get status badge icon for task status
fn get_status_badge(status: &str) -> &'static str {
    match status {
        "done" => "✓",
        "doing" => "→",
        "todo" => "○",
        _ => "?",
    }
}

/// Print task context in a human-friendly tree format
fn print_task_context(ctx: &TaskContext) -> Result<()> {
    let task = &ctx.task;

    // Status badge
    let status_badge = get_status_badge(task.status.as_str());

    // Main task info
    println!("Task #{}: {} [{}]", task.id, task.name, status_badge);

    // Timestamps
    if let Some(created) = task.first_todo_at {
        print!("Created: {}", created);
        if let Some(started) = task.first_doing_at {
            print!(" | Started: {}", started);
        }
        if let Some(done) = task.first_done_at {
            print!(" | Done: {}", done);
        }
        println!();
    }

    // Ancestors
    println!("\nAncestors:");
    if ctx.ancestors.is_empty() {
        println!("  (none - top-level task)");
    } else {
        for ancestor in &ctx.ancestors {
            let status = get_status_badge(ancestor.status.as_str());
            println!("  └─ #{}: {} {}", ancestor.id, ancestor.name, status);
        }
    }

    // Children
    let done_count = ctx.children.iter().filter(|c| c.status == "done").count();
    println!(
        "\nChildren ({} subtasks, {} done):",
        ctx.children.len(),
        done_count
    );
    if ctx.children.is_empty() {
        println!("  (none)");
    } else {
        for (i, child) in ctx.children.iter().enumerate() {
            let status = get_status_badge(child.status.as_str());
            let prefix = if i == ctx.children.len() - 1 {
                "└─"
            } else {
                "├─"
            };
            println!("  {} #{} {} {}", prefix, child.id, child.name, status);
        }
    }

    // Siblings
    if !ctx.siblings.is_empty() {
        println!("\nSiblings ({} at same level):", ctx.siblings.len());
        let show_count = 5.min(ctx.siblings.len());
        for (i, sibling) in ctx.siblings.iter().take(show_count).enumerate() {
            let status = get_status_badge(sibling.status.as_str());
            let prefix = if i == show_count - 1 && ctx.siblings.len() <= 5 {
                "└─"
            } else {
                "├─"
            };
            println!("  {} #{} {} {}", prefix, sibling.id, sibling.name, status);
        }
        if ctx.siblings.len() > 5 {
            println!("  ... and {} more", ctx.siblings.len() - 5);
        }
    }

    // Dependencies
    if !ctx.dependencies.blocking_tasks.is_empty() {
        println!("\nBlocking tasks (must complete first):");
        for task in &ctx.dependencies.blocking_tasks {
            let status = get_status_badge(task.status.as_str());
            println!("  • #{} {} {}", task.id, task.name, status);
        }
    }

    if !ctx.dependencies.blocked_by_tasks.is_empty() {
        println!("\nBlocked by this task:");
        for task in &ctx.dependencies.blocked_by_tasks {
            let status = get_status_badge(task.status.as_str());
            println!("  • #{} {} {}", task.id, task.name, status);
        }
    }

    Ok(())
}

/// Check if Dashboard is healthy by querying the health endpoint
/// Returns true if Dashboard is running and responding
async fn check_dashboard_health(port: u16) -> bool {
    let health_url = format!("http://127.0.0.1:{}/api/health", port);

    match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(client) => match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                tracing::debug!("Dashboard health check passed for port {}", port);
                true
            },
            Ok(resp) => {
                tracing::debug!("Dashboard health check failed: status {}", resp.status());
                false
            },
            Err(e) => {
                tracing::debug!("Dashboard health check failed: {}", e);
                false
            },
        },
        Err(e) => {
            tracing::error!("Failed to create HTTP client: {}", e);
            false
        },
    }
}

/// Check Dashboard status and return formatted JSON result
async fn check_dashboard_status() -> serde_json::Value {
    use serde_json::json;

    let dashboard_url = format!("http://127.0.0.1:{}", DASHBOARD_PORT);

    if check_dashboard_health(DASHBOARD_PORT).await {
        json!({
            "check": "Dashboard",
            "status": "✓ PASS",
            "details": {
                "url": dashboard_url,
                "status": "running",
                "access": format!("Visit {} in your browser", dashboard_url)
            }
        })
    } else {
        json!({
            "check": "Dashboard",
            "status": "⚠ WARNING",
            "details": {
                "status": "not running",
                "message": "Dashboard is not running. Start it with 'ie dashboard start'",
                "command": "ie dashboard start"
            }
        })
    }
}

/// Check MCP connections by querying Dashboard's /api/projects endpoint
async fn check_mcp_connections() -> serde_json::Value {
    use serde_json::json;

    if !check_dashboard_health(DASHBOARD_PORT).await {
        return json!({
            "check": "MCP Connections",
            "status": "⚠ WARNING",
            "details": {
                "count": 0,
                "message": "Dashboard not running - cannot query connections",
                "command": "ie dashboard start"
            }
        });
    }

    // Query /api/projects to get connection count
    let url = format!("http://127.0.0.1:{}/api/projects", DASHBOARD_PORT);
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return json!({
                "check": "MCP Connections",
                "status": "✗ FAIL",
                "details": {
                    "error": format!("Failed to create HTTP client: {}", e)
                }
            });
        },
    };

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                let empty_vec = vec![];
                let projects = data["projects"].as_array().unwrap_or(&empty_vec);
                let mcp_count = projects
                    .iter()
                    .filter(|p| p["mcp_connected"].as_bool().unwrap_or(false))
                    .count();

                json!({
                    "check": "MCP Connections",
                    "status": if mcp_count > 0 { "✓ PASS" } else { "⚠ WARNING" },
                    "details": {
                        "count": mcp_count,
                        "message": if mcp_count > 0 {
                            format!("{} MCP client(s) connected", mcp_count)
                        } else {
                            "No MCP clients connected".to_string()
                        }
                    }
                })
            } else {
                json!({
                    "check": "MCP Connections",
                    "status": "✗ FAIL",
                    "details": {"error": "Failed to parse response"}
                })
            }
        },
        _ => json!({
            "check": "MCP Connections",
            "status": "⚠ WARNING",
            "details": {"count": 0, "message": "Dashboard not responding"}
        }),
    }
}

/// Check SessionStart hook configuration and effectiveness
fn check_session_start_hook() -> serde_json::Value {
    use intent_engine::setup::common::get_home_dir;
    use serde_json::json;

    let home = match get_home_dir() {
        Ok(h) => h,
        Err(_) => {
            return json!({
                "check": "SessionStart Hook",
                "status": "⚠ WARNING",
                "details": {"error": "Unable to determine home directory"}
            })
        },
    };

    let user_hook = home.join(".claude/hooks/session-start.sh");
    let user_settings = home.join(".claude/settings.json");

    let script_exists = user_hook.exists();
    let script_executable = if script_exists {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(&user_hook)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        }
        #[cfg(not(unix))]
        {
            true
        }
    } else {
        false
    };

    let is_configured = if user_settings.exists() {
        std::fs::read_to_string(&user_settings)
            .ok()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
            .map(|settings| {
                settings
                    .get("hooks")
                    .and_then(|h| h.get("SessionStart"))
                    .is_some()
            })
            .unwrap_or(false)
    } else {
        false
    };

    let is_active = script_exists && script_executable && is_configured;

    if is_active {
        json!({
            "check": "SessionStart Hook",
            "status": "✓ PASS",
            "details": {
                "script": user_hook.display().to_string(),
                "configured": true,
                "executable": true,
                "message": "SessionStart hook is active"
            }
        })
    } else if is_configured && !script_exists {
        json!({
            "check": "SessionStart Hook",
            "status": "✗ FAIL",
            "details": {
                "configured": true,
                "exists": false,
                "message": "Hook configured but script file missing"
            }
        })
    } else if script_exists && !script_executable {
        json!({
            "check": "SessionStart Hook",
            "status": "✗ FAIL",
            "details": {
                "executable": false,
                "message": "Script not executable. Run: chmod +x ~/.claude/hooks/session-start.sh"
            }
        })
    } else {
        json!({
            "check": "SessionStart Hook",
            "status": "⚠ WARNING",
            "details": {
                "configured": false,
                "message": "Not configured. Run 'ie setup --target claude-code'",
                "setup_command": "ie setup --target claude-code"
            }
        })
    }
}

async fn handle_dashboard_command(dashboard_cmd: DashboardCommands) -> Result<()> {
    use intent_engine::dashboard::daemon;

    match dashboard_cmd {
        DashboardCommands::Start {
            port,
            foreground,
            browser,
        } => {
            // Load project context to get project path and DB path
            let project_ctx = ProjectContext::load_or_init().await?;
            let project_path = project_ctx.root.clone();
            let db_path = project_ctx.db_path.clone();
            let project_name = project_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Allocate port (always 11391, or custom if specified)
            let allocated_port = port.unwrap_or(11391);

            // Check if already running using PID file + HTTP health check
            if let Ok(Some(existing_pid)) = daemon::read_pid_file(allocated_port) {
                if check_dashboard_health(allocated_port).await {
                    println!("Dashboard already running for this project:");
                    println!("  Port: {}", allocated_port);
                    println!("  PID: {}", existing_pid);
                    println!("  URL: http://127.0.0.1:{}", allocated_port);
                    return Ok(());
                } else {
                    // Dashboard not responding, clean up stale PID file
                    tracing::info!(
                        "Cleaning up stale Dashboard PID file for port {}",
                        allocated_port
                    );
                    daemon::delete_pid_file(allocated_port).ok();
                }
            }

            // Check if port is available
            if std::net::TcpListener::bind(("127.0.0.1", allocated_port)).is_err() {
                return Err(IntentError::InvalidInput(format!(
                    "Port {} is already in use",
                    allocated_port
                )));
            }

            println!("Dashboard starting for project: {}", project_name);
            println!("  Port: {}", allocated_port);
            println!("  URL: http://127.0.0.1:{}", allocated_port);
            println!(
                "  Mode: {}",
                if foreground { "foreground" } else { "daemon" }
            );

            if foreground {
                // Start server in foreground mode
                use intent_engine::dashboard::server::DashboardServer;

                let server =
                    DashboardServer::new(allocated_port, project_path.clone(), db_path.clone())
                        .await?;

                println!(
                    "\n🚀 Dashboard server running at http://127.0.0.1:{}",
                    allocated_port
                );
                println!("   Press Ctrl+C to stop\n");

                // Open browser if explicitly requested
                if browser {
                    let dashboard_url = format!("http://127.0.0.1:{}", allocated_port);
                    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                    println!("🌐 Opening dashboard in browser...");
                    if let Err(e) = open::that(&dashboard_url) {
                        eprintln!("⚠️  Could not open browser automatically: {}", e);
                        eprintln!("   Please manually visit: {}", dashboard_url);
                    }
                    println!();
                }

                // Write PID file
                let current_pid = std::process::id();
                daemon::write_pid_file(allocated_port, current_pid)?;

                // Run server (blocks until terminated)
                let result = server.run().await;

                // Cleanup on exit
                daemon::delete_pid_file(allocated_port).ok();

                result.map_err(IntentError::OtherError)?;
                Ok(())
            } else {
                // Daemon mode: spawn background process
                println!("\n🚀 Dashboard server starting in background...");

                // Spawn new process with same binary but in foreground mode
                let current_exe = std::env::current_exe()?;

                // Properly daemonize using setsid on Unix systems
                #[cfg(unix)]
                let mut cmd = {
                    let mut cmd = std::process::Command::new("setsid");
                    cmd.arg(current_exe)
                        .arg("dashboard")
                        .arg("start")
                        .arg("--foreground")
                        .arg("--port")
                        .arg(allocated_port.to_string());

                    // Pass --browser flag if specified
                    if browser {
                        cmd.arg("--browser");
                    }

                    cmd
                };

                // On Windows, just spawn normally (no setsid available)
                #[cfg(not(unix))]
                let mut cmd = {
                    let mut cmd = std::process::Command::new(current_exe);
                    cmd.arg("dashboard")
                        .arg("start")
                        .arg("--foreground")
                        .arg("--port")
                        .arg(allocated_port.to_string());

                    // Pass --browser flag if specified
                    if browser {
                        cmd.arg("--browser");
                    }

                    cmd
                };

                let child = cmd
                    .current_dir(&project_path)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()?;

                // When using setsid, child.id() returns setsid's PID, not the dashboard's PID
                // We need to find the actual dashboard process
                let _setsid_pid = child.id();

                // Give server a moment to start
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                // Find the actual dashboard PID by searching for the process
                #[cfg(unix)]
                let pid = {
                    use std::process::Command;

                    let output = Command::new("pgrep")
                        .args([
                            "-f",
                            &format!("ie dashboard start --foreground --port {}", allocated_port),
                        ])
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .and_then(|s| s.trim().parse::<u32>().ok());

                    match output {
                        Some(pid) => pid,
                        None => {
                            // Fallback: try to use setsid PID (won't work but better than failing)
                            _setsid_pid
                        },
                    }
                };

                #[cfg(not(unix))]
                let pid = _setsid_pid;

                // Write PID file
                daemon::write_pid_file(allocated_port, pid)?;

                // Wait a moment for server to initialize, then check health
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                if check_dashboard_health(allocated_port).await {
                    let dashboard_url = format!("http://127.0.0.1:{}", allocated_port);
                    println!("✓ Dashboard server started successfully");
                    println!("  PID: {}", pid);
                    println!("  URL: {}", dashboard_url);

                    // Open browser if explicitly requested
                    if browser {
                        println!("\n🌐 Opening dashboard in browser...");
                        if let Err(e) = open::that(&dashboard_url) {
                            eprintln!("⚠️  Could not open browser automatically: {}", e);
                            eprintln!("   Please manually visit: {}", dashboard_url);
                        }
                    }

                    println!("\nUse 'ie dashboard stop' to stop the server");
                } else {
                    // Server failed to start
                    daemon::delete_pid_file(allocated_port).ok();
                    return Err(IntentError::InvalidInput(
                        "Failed to start dashboard server".to_string(),
                    ));
                }

                Ok(())
            }
        },

        DashboardCommands::Stop { all } => {
            // Single Dashboard architecture: all uses fixed port 11391
            let port = 11391;

            if all {
                println!(
                    "⚠️  Note: Single Dashboard mode - stopping Dashboard on port {}",
                    port
                );
            }

            // Check if dashboard is running via PID file + HTTP health check
            match daemon::read_pid_file(port) {
                Ok(Some(pid)) => {
                    // PID file exists - check if dashboard is actually running
                    if check_dashboard_health(port).await {
                        // Dashboard is healthy - stop it
                        daemon::stop_process(pid)?;
                        println!("✓ Stopped dashboard (PID: {})", pid);
                    } else {
                        // Dashboard not responding - clean up stale PID
                        println!(
                            "⚠️  Dashboard not responding (stale PID: {}), cleaning up",
                            pid
                        );
                    }
                    daemon::delete_pid_file(port).ok();
                },
                Ok(None) => {
                    // No PID file - check if something is listening on port anyway
                    if check_dashboard_health(port).await {
                        println!(
                            "⚠️  Dashboard running but no PID file found (port {})",
                            port
                        );
                        println!(
                            "   Try killing the process manually or use: lsof -ti:{} | xargs kill",
                            port
                        );
                        return Err(IntentError::InvalidInput(
                            "Dashboard running without PID file".to_string(),
                        ));
                    } else {
                        println!("Dashboard not running");
                    }
                },
                Err(e) => {
                    tracing::debug!("Error reading PID file: {}", e);
                    println!("Dashboard not running");
                },
            }

            Ok(())
        },

        DashboardCommands::Status { all } => {
            // Single Dashboard architecture: check fixed port 11391
            let port = 11391;

            if all {
                println!(
                    "⚠️  Note: Single Dashboard mode - showing status for port {}",
                    port
                );
            }

            // Check if dashboard is running via PID file + HTTP health check
            match daemon::read_pid_file(port) {
                Ok(Some(pid)) => {
                    // PID file exists - check if dashboard is actually running
                    if check_dashboard_health(port).await {
                        // Dashboard is healthy - get project info via API
                        let url = format!("http://127.0.0.1:{}/api/info", port);
                        match reqwest::get(&url).await {
                            Ok(response) if response.status().is_success() => {
                                #[derive(serde::Deserialize)]
                                struct InfoResponse {
                                    data: serde_json::Value,
                                }
                                if let Ok(info) = response.json::<InfoResponse>().await {
                                    println!("Dashboard status:");
                                    println!("  Status: ✓ Running (PID: {})", pid);
                                    println!("  Port: {}", port);
                                    println!("  URL: http://127.0.0.1:{}", port);
                                    if let Some(project_name) = info.data.get("project_name") {
                                        println!("  Project: {}", project_name);
                                    }
                                    if let Some(project_path) = info.data.get("project_path") {
                                        println!("  Path: {}", project_path);
                                    }
                                } else {
                                    println!("Dashboard status:");
                                    println!("  Status: ✓ Running (PID: {})", pid);
                                    println!("  Port: {}", port);
                                    println!("  URL: http://127.0.0.1:{}", port);
                                }
                            },
                            _ => {
                                println!("Dashboard status:");
                                println!("  Status: ✓ Running (PID: {})", pid);
                                println!("  Port: {}", port);
                                println!("  URL: http://127.0.0.1:{}", port);
                            },
                        }
                    } else {
                        println!("Dashboard status:");
                        println!("  Status: ✗ Stopped (stale PID: {})", pid);
                        println!("  Port: {}", port);
                    }
                },
                Ok(None) => {
                    println!("Dashboard status:");
                    println!("  Status: ✗ Not running");
                    println!("  Port: {}", port);
                },
                Err(e) => {
                    tracing::debug!("Error reading PID file: {}", e);
                    println!("Dashboard status:");
                    println!("  Status: ✗ Not running");
                    println!("  Port: {}", port);
                },
            }

            Ok(())
        },

        DashboardCommands::List => {
            // Single Dashboard architecture: check fixed port 11391
            let port = 11391;

            // Check if dashboard is running
            if !check_dashboard_health(port).await {
                println!("Dashboard not running");
                println!("\nUse 'ie dashboard start' to start the Dashboard");
                return Ok(());
            }

            // Get PID if available
            let pid = daemon::read_pid_file(port).ok().flatten();

            // Get project list via API
            let url = format!("http://127.0.0.1:{}/api/projects", port);
            match reqwest::get(&url).await {
                Ok(response) if response.status().is_success() => {
                    #[derive(serde::Deserialize)]
                    struct ApiResponse {
                        data: Vec<serde_json::Value>,
                    }
                    match response.json::<ApiResponse>().await {
                        Ok(api_response) => {
                            if api_response.data.is_empty() {
                                println!("Dashboard running but no projects registered");
                                if let Some(pid) = pid {
                                    println!("  PID: {}", pid);
                                }
                                println!("  Port: {}", port);
                                println!("  URL: http://127.0.0.1:{}", port);
                                return Ok(());
                            }

                            println!("Dashboard projects:");
                            println!("{:<30} {:<8} {:<15} MCP", "PROJECT", "PORT", "STATUS");
                            println!("{}", "-".repeat(80));

                            for project in api_response.data {
                                let name = project
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                let mcp_connected = project
                                    .get("mcp_connected")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                let mcp_status = if mcp_connected {
                                    "✓ Connected"
                                } else {
                                    "✗ Disconnected"
                                };

                                println!(
                                    "{:<30} {:<8} {:<15} {}",
                                    name, port, "Running", mcp_status
                                );

                                if let Some(path) = project.get("path").and_then(|v| v.as_str()) {
                                    println!("  Path: {}", path);
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to parse projects list: {}", e);
                            println!("Dashboard running on port {}", port);
                            if let Some(pid) = pid {
                                println!("  PID: {}", pid);
                            }
                        },
                    }
                },
                Ok(response) => {
                    eprintln!("Failed to get projects list: HTTP {}", response.status());
                    println!("Dashboard running on port {}", port);
                    if let Some(pid) = pid {
                        println!("  PID: {}", pid);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to connect to Dashboard API: {}", e);
                    println!("Dashboard may not be running properly on port {}", port);
                },
            }

            Ok(())
        },

        DashboardCommands::Open => {
            // Single Dashboard architecture: use fixed port 11391
            let port = 11391;

            // Check if dashboard is running via HTTP health check
            if !check_dashboard_health(port).await {
                eprintln!("Dashboard is not running");
                eprintln!("Start it with: ie dashboard start");
                return Err(IntentError::InvalidInput(
                    "Dashboard not running".to_string(),
                ));
            }

            let url = format!("http://127.0.0.1:{}", port);
            println!("Opening dashboard: {}", url);

            daemon::open_browser(&url)?;

            Ok(())
        },
    }
}

fn handle_logs_command(
    mode: Option<String>,
    level: Option<String>,
    since: Option<String>,
    until: Option<String>,
    limit: Option<usize>,
    follow: bool,
    export: String,
) -> Result<()> {
    use intent_engine::logs::{
        follow_logs, format_entry_json, format_entry_text, parse_duration, query_logs, LogQuery,
    };

    // Build query
    let mut query = LogQuery {
        mode,
        level,
        limit,
        ..Default::default()
    };

    if let Some(since_str) = since {
        query.since = parse_duration(&since_str);
        if query.since.is_none() {
            return Err(IntentError::InvalidInput(format!(
                "Invalid duration format: {}. Use format like '1h', '24h', '7d'",
                since_str
            )));
        }
    }

    if let Some(until_str) = until {
        use chrono::DateTime;
        match DateTime::parse_from_rfc3339(&until_str) {
            Ok(dt) => query.until = Some(dt.with_timezone(&chrono::Utc)),
            Err(e) => {
                return Err(IntentError::InvalidInput(format!(
                    "Invalid timestamp format: {}. Error: {}",
                    until_str, e
                )))
            },
        }
    }

    // Handle follow mode
    if follow {
        return follow_logs(&query).map_err(IntentError::IoError);
    }

    // Query logs
    let entries = query_logs(&query).map_err(IntentError::IoError)?;

    if entries.is_empty() {
        eprintln!("No log entries found matching the criteria");
        return Ok(());
    }

    // Display results
    match export.as_str() {
        "json" => {
            println!("[");
            for (i, entry) in entries.iter().enumerate() {
                print!("  {}", format_entry_json(entry));
                if i < entries.len() - 1 {
                    println!(",");
                } else {
                    println!();
                }
            }
            println!("]");
        },
        _ => {
            for entry in entries {
                println!("{}", format_entry_text(&entry));
            }
        },
    }

    Ok(())
}
