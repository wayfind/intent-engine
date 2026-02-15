use clap::Parser;
use intent_engine::cli::{Cli, Commands, DashboardCommands};
use intent_engine::cli_handlers::{
    handle_config_command, handle_dashboard_command, handle_doctor_command, handle_init_command,
    handle_log, handle_search_command, handle_status, handle_task_command, print_plan_result,
    read_stdin,
};
use intent_engine::error::{IntentError, Result};
use intent_engine::events::EventManager;
use intent_engine::logging::LoggingConfig;
use intent_engine::plan::{
    cleanup_included_files, process_file_includes, PlanExecutor, PlanRequest,
};
use intent_engine::project::ProjectContext;
use intent_engine::tasks::TaskManager;
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
                executor = executor.with_default_parent(current_task_id);
            }

            let result = executor.execute(&request).await?;

            // Clean up included files after successful execution
            if result.success && !file_include_result.files_to_delete.is_empty() {
                cleanup_included_files(&file_include_result.files_to_delete);
            }

            print_plan_result(&result, &format)?;
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
            handle_log(
                &event_mgr,
                &workspace_mgr,
                event_type,
                &message,
                task,
                &format,
            )
            .await?;
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

        Commands::Task(task_cmd) => {
            let ctx = ProjectContext::load_or_init().await?;
            let project_path = ctx.root.to_string_lossy().to_string();
            let task_mgr = TaskManager::with_project_path(&ctx.pool, project_path);
            let workspace_mgr = WorkspaceManager::new(&ctx.pool);
            handle_task_command(&task_mgr, &workspace_mgr, task_cmd).await?
        },

        Commands::Suggestions(suggestions_cmd) => {
            use intent_engine::cli::SuggestionsCommands;
            use intent_engine::cli_handlers::suggestions_commands;

            match suggestions_cmd {
                SuggestionsCommands::List { format } => {
                    suggestions_commands::handle_list(&format).await?
                },
                SuggestionsCommands::Dismiss { id, all, format } => {
                    suggestions_commands::handle_dismiss(id, all, &format).await?
                },
                SuggestionsCommands::Clear { format } => {
                    suggestions_commands::handle_clear(&format).await?
                },
            }
        },

        Commands::Config(config_cmd) => handle_config_command(config_cmd).await?,

        Commands::Status {
            task_id,
            with_events,
            format,
        } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);
            let workspace_mgr = WorkspaceManager::new(&ctx.pool);

            // Trigger background task structure analysis (async, non-blocking)
            intent_engine::llm::analyze_task_structure_background(ctx.pool.clone());

            // Use shared status handler
            handle_status(&task_mgr, &workspace_mgr, task_id, with_events, &format).await?;

            // Display LLM suggestions (SQLite-only, text format only)
            if format != "json" {
                intent_engine::llm::display_suggestions(&ctx.pool).await?;
            }
        },
    }

    Ok(())
}
