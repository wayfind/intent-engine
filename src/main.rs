use clap::Parser;
use intent_engine::cli::{Cli, Commands, DashboardCommands};
use intent_engine::cli_handlers::{
    handle_dashboard_command, handle_doctor_command, handle_init_command, handle_search_command,
    read_stdin,
};
use intent_engine::error::{IntentError, Result};
use intent_engine::events::EventManager;
use intent_engine::logging::LoggingConfig;
use intent_engine::plan::{PlanExecutor, PlanRequest};
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
            let request: PlanRequest = serde_json::from_str(&json_input)
                .map_err(|e| IntentError::InvalidInput(format!("Invalid JSON: {}", e)))?;

            // Execute the plan
            let ctx = ProjectContext::load_or_init().await?;
            let project_path = ctx.root.to_string_lossy().to_string();
            let executor = PlanExecutor::with_project_path(&ctx.pool, project_path);
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

                    // Display focused task if present
                    if let Some(focused) = &result.focused_task {
                        println!();
                        println!("✓ Current focus:");
                        println!("  ID: {}", focused.task.id);
                        println!("  Name: {}", focused.task.name);
                        println!("  Status: {}", focused.task.status);
                        if let Some(spec) = &focused.task.spec {
                            println!("  Spec: {}", spec);
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
                                        event.timestamp.format("%Y-%m-%d %H:%M"),
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
                let current_response = workspace_mgr.get_current_task().await?;
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
                println!("  Type: {}", event_type_str);
                println!("  Task: #{}", target_task_id);
                println!("  Message: {}", message);
            }
        },

        Commands::Search {
            query,
            tasks,
            events,
            limit,
            offset,
        } => handle_search_command(&query, tasks, events, limit, offset).await?,

        Commands::Init { at, force } => handle_init_command(at, force).await?,

        Commands::Dashboard(dashboard_cmd) => handle_dashboard_command(dashboard_cmd).await?,

        Commands::Doctor => handle_doctor_command().await?,
    }

    Ok(())
}
