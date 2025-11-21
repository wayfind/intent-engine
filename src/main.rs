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
use intent_engine::tasks::TaskManager;
use intent_engine::workspace::WorkspaceManager;
use sqlx::Row;
use std::io::{self, Read};
use std::path::PathBuf;
use std::str::FromStr;

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
    let log_config = LoggingConfig::from_args(cli.quiet, cli.verbose > 0, cli.json);
    if let Err(e) = intent_engine::logging::init_logging(log_config) {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
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
            dry_run,
            force,
            diagnose,
            config_path,
            project_dir,
        } => {
            handle_setup(
                target,
                &scope,
                dry_run,
                force,
                diagnose,
                config_path,
                project_dir,
            )
            .await?;
        },

        Commands::Plan { dry_run, format } => {
            // Read JSON from stdin
            let json_input = read_stdin()?;

            // Parse JSON into PlanRequest
            let request: PlanRequest = serde_json::from_str(&json_input)
                .map_err(|e| IntentError::InvalidInput(format!("Invalid JSON: {}", e)))?;

            if dry_run {
                // Dry-run mode: just validate and show what would be created
                println!("DRY RUN - no changes will be made");
                println!();
                println!("Would create/update:");
                for task in &request.tasks {
                    print_task_tree(task, 0);
                }
            } else {
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

        Commands::Switch { id, with_events } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            if with_events {
                // Switch and then get task with events
                task_mgr.switch_to_task(id).await?;
                let task = task_mgr.get_task_with_events(id).await?;
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                let task = task_mgr.switch_to_task(id).await?;
                println!("{}", serde_json::to_string_pretty(&task)?);
            }
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

        TaskCommands::Switch { id } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let task = task_mgr.switch_to_task(id).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
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

/// Check MCP server configuration in ~/.claude.json
fn check_mcp_configuration() -> serde_json::Value {
    use intent_engine::setup::common::get_home_dir;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    let home = match get_home_dir() {
        Ok(h) => h,
        Err(_) => {
            return json!({
                "check": "MCP Configuration",
                "status": "⚠ WARNING",
                "passed": false,
                "details": {
                    "error": "Unable to determine home directory",
                    "config_file": "~/.claude.json",
                    "config_exists": false
                }
            });
        },
    };

    let config_path = home.join(".claude.json");

    if !config_path.exists() {
        return json!({
            "check": "MCP Configuration",
            "status": "⚠ WARNING",
            "passed": false,
            "details": {
                "config_file": config_path.display().to_string(),
                "config_exists": false,
                "mcp_configured": false,
                "message": "MCP not configured. Run 'ie setup --target claude-code' to configure",
                "setup_command": "ie setup --target claude-code"
            }
        });
    }

    // Read and parse config
    let config_content = match fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => {
            return json!({
                "check": "MCP Configuration",
                "status": "✗ FAIL",
                "passed": false,
                "details": {
                    "config_file": config_path.display().to_string(),
                    "config_exists": true,
                    "error": format!("Failed to read config: {}", e)
                }
            });
        },
    };

    let config: serde_json::Value = match serde_json::from_str(&config_content) {
        Ok(c) => c,
        Err(e) => {
            return json!({
                "check": "MCP Configuration",
                "status": "✗ FAIL",
                "passed": false,
                "details": {
                    "config_file": config_path.display().to_string(),
                    "config_exists": true,
                    "error": format!("Invalid JSON: {}", e)
                }
            });
        },
    };

    // Check if intent-engine is configured
    let mcp_servers = config.get("mcpServers");
    let ie_config = mcp_servers.and_then(|s| s.get("intent-engine"));

    if ie_config.is_none() {
        return json!({
            "check": "MCP Configuration",
            "status": "⚠ WARNING",
            "passed": false,
            "details": {
                "config_file": config_path.display().to_string(),
                "config_exists": true,
                "mcp_configured": false,
                "message": "intent-engine not configured in MCP servers",
                "setup_command": "ie setup --target claude-code"
            }
        });
    }

    let ie_config = ie_config.unwrap();
    let binary_path = ie_config
        .get("command")
        .and_then(|c| c.as_str())
        .unwrap_or("");
    let binary_path_buf = PathBuf::from(binary_path);
    let binary_exists = binary_path_buf.exists();
    let binary_executable = if binary_exists {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::metadata(&binary_path_buf)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        }
        #[cfg(not(unix))]
        {
            true // On Windows, assume executable if exists
        }
    } else {
        false
    };

    let env_config = ie_config.get("env");
    let project_dir = env_config
        .and_then(|e| e.get("INTENT_ENGINE_PROJECT_DIR"))
        .and_then(|p| p.as_str())
        .unwrap_or("");

    let status = if binary_exists && binary_executable {
        "✓ PASS"
    } else if binary_exists {
        "⚠ WARNING"
    } else {
        "✗ FAIL"
    };

    let passed = binary_exists && binary_executable;

    json!({
        "check": "MCP Configuration",
        "status": status,
        "passed": passed,
        "details": {
            "config_file": config_path.display().to_string(),
            "config_exists": true,
            "mcp_configured": true,
            "binary_path": binary_path,
            "binary_exists": binary_exists,
            "binary_executable": binary_executable,
            "project_dir": project_dir,
            "message": if passed {
                "MCP server configured correctly"
            } else if !binary_exists {
                "Binary not found at configured path"
            } else {
                "Binary not executable"
            }
        }
    })
}

/// Check hooks configuration in ~/.claude/ or ./.claude/
fn check_hooks_configuration() -> serde_json::Value {
    use intent_engine::setup::common::get_home_dir;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    let home = match get_home_dir() {
        Ok(h) => h,
        Err(_) => {
            return json!({
                "check": "Hooks Configuration",
                "status": "⚠ WARNING",
                "passed": false,
                "details": {
                    "error": "Unable to determine home directory"
                }
            });
        },
    };

    // Check both user-level and project-level
    let user_hook = home.join(".claude/hooks/session-start.sh");
    let user_settings = home.join(".claude/settings.json");
    let project_hook = PathBuf::from(".claude/hooks/session-start.sh");
    let project_settings = PathBuf::from(".claude/settings.json");

    let mut details = json!({
        "user_level": {
            "hook_script": user_hook.display().to_string(),
            "script_exists": user_hook.exists(),
            "script_executable": false,
            "settings_file": user_settings.display().to_string(),
            "settings_exists": user_settings.exists(),
            "settings_configured": false
        },
        "project_level": {
            "hook_script": project_hook.display().to_string(),
            "script_exists": project_hook.exists(),
            "script_executable": false,
            "settings_file": project_settings.display().to_string(),
            "settings_exists": project_settings.exists(),
            "settings_configured": false
        }
    });

    let mut any_configured = false;

    // Check user-level
    if user_hook.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&user_hook) {
                details["user_level"]["script_executable"] =
                    json!(metadata.permissions().mode() & 0o111 != 0);
            }
        }
        #[cfg(not(unix))]
        {
            details["user_level"]["script_executable"] = json!(true);
        }

        if user_settings.exists() {
            if let Ok(content) = fs::read_to_string(&user_settings) {
                if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                    let has_session_start = settings
                        .get("hooks")
                        .and_then(|h| h.get("SessionStart"))
                        .is_some();
                    details["user_level"]["settings_configured"] = json!(has_session_start);
                    if has_session_start {
                        any_configured = true;
                    }
                }
            }
        }
    }

    // Check project-level
    if project_hook.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&project_hook) {
                details["project_level"]["script_executable"] =
                    json!(metadata.permissions().mode() & 0o111 != 0);
            }
        }
        #[cfg(not(unix))]
        {
            details["project_level"]["script_executable"] = json!(true);
        }

        if project_settings.exists() {
            if let Ok(content) = fs::read_to_string(&project_settings) {
                if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                    let has_session_start = settings
                        .get("hooks")
                        .and_then(|h| h.get("SessionStart"))
                        .is_some();
                    details["project_level"]["settings_configured"] = json!(has_session_start);
                    if has_session_start {
                        any_configured = true;
                    }
                }
            }
        }
    }

    let status = if any_configured {
        "✓ PASS"
    } else {
        "⚠ WARNING"
    };

    details["message"] = if any_configured {
        json!("Hooks configured correctly")
    } else {
        json!("Hooks not configured. Run 'ie setup --target claude-code' to configure")
    };

    details["setup_command"] = json!("ie setup --target claude-code");

    json!({
        "check": "Hooks Configuration",
        "status": status,
        "passed": any_configured,
        "details": details
    })
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
    let mut all_passed = true;

    // Check OS and architecture
    checks.push(json!({
        "check": "System Information",
        "status": "✓ PASS",
        "details": format!("OS: {}, Arch: {}", std::env::consts::OS, std::env::consts::ARCH)
    }));

    // Check SQLite version
    match sqlx::query("SELECT sqlite_version()")
        .fetch_optional(&sqlx::SqlitePool::connect(":memory:").await?)
        .await
    {
        Ok(Some(row)) => {
            let version: String = row.try_get(0).unwrap_or_else(|_| "unknown".to_string());
            checks.push(json!({
                "check": "SQLite",
                "status": "✓ PASS",
                "details": format!("SQLite version: {}", version)
            }));
        },
        Ok(None) | Err(_) => {
            all_passed = false;
            checks.push(json!({
                "check": "SQLite",
                "status": "✗ FAIL",
                "details": "Unable to query SQLite version"
            }));
        },
    }

    // Check database initialization
    match ProjectContext::load_or_init().await {
        Ok(ctx) => {
            // Test a simple query
            match sqlx::query("SELECT COUNT(*) FROM tasks")
                .fetch_one(&ctx.pool)
                .await
            {
                Ok(row) => {
                    let count: i64 = row.try_get(0).unwrap_or(0);
                    checks.push(json!({
                        "check": "Database Connection",
                        "status": "✓ PASS",
                        "details": format!("Connected to database, {} tasks found", count)
                    }));
                },
                Err(e) => {
                    all_passed = false;
                    checks.push(json!({
                        "check": "Database Connection",
                        "status": "✗ FAIL",
                        "details": format!("Database query failed: {}", e)
                    }));
                },
            }
        },
        Err(e) => {
            all_passed = false;
            checks.push(json!({
                "check": "Database Initialization",
                "status": "✗ FAIL",
                "details": format!("Failed to initialize database: {}", e)
            }));
        },
    }

    // Check intent-engine version
    checks.push(json!({
        "check": "Intent Engine Version",
        "status": "✓ PASS",
        "details": format!("v{}", env!("CARGO_PKG_VERSION"))
    }));

    // Database path resolution diagnostics
    let db_path_info = ProjectContext::get_database_path_info();
    checks.push(json!({
        "check": "Database Path Resolution",
        "status": "✓ INFO",
        "details": db_path_info
    }));

    // Check MCP configuration
    let mcp_check = check_mcp_configuration();
    // MCP is optional, so don't fail overall status
    // if !mcp_check["passed"].as_bool().unwrap_or(false) {
    //     all_passed = false;
    // }
    checks.push(mcp_check);

    // Check Hooks configuration
    let hooks_check = check_hooks_configuration();
    // Hooks are optional, so don't fail overall status
    // if !hooks_check["passed"].as_bool().unwrap_or(false) {
    //     all_passed = false;
    // }
    checks.push(hooks_check);

    // Determine if there are any real failures (not just warnings)
    let has_failures = checks.iter().any(|check| {
        let status = check["status"].as_str().unwrap_or("");
        status.contains("✗ FAIL")
    });

    let result = json!({
        "summary": if all_passed { "✓ All checks passed" } else if has_failures { "✗ Some checks failed" } else { "⚠ Some optional features not configured" },
        "overall_status": if all_passed { "healthy" } else if has_failures { "unhealthy" } else { "warnings" },
        "checks": checks
    });

    println!("{}", serde_json::to_string_pretty(&result)?);

    // Only exit with error code if there are actual failures, not just warnings
    if has_failures {
        std::process::exit(1);
    }

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
    dry_run: bool,
    force: bool,
    diagnose: bool,
    config_path: Option<String>,
    project_dir: Option<String>,
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
        dry_run,
        force,
        config_path: config_path.map(PathBuf::from),
        project_dir: project_dir.map(PathBuf::from),
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

            if !dry_run {
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
            }

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

/// Print task context in a human-friendly tree format
fn print_task_context(ctx: &TaskContext) -> Result<()> {
    let task = &ctx.task;

    // Status badge
    let status_badge = match task.status.as_str() {
        "done" => "✓",
        "doing" => "→",
        "todo" => "○",
        _ => "?",
    };

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
            let status = match ancestor.status.as_str() {
                "done" => "✓",
                "doing" => "→",
                "todo" => "○",
                _ => "?",
            };
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
            let status = match child.status.as_str() {
                "done" => "✓",
                "doing" => "→",
                "todo" => "○",
                _ => "?",
            };
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
            let status = match sibling.status.as_str() {
                "done" => "✓",
                "doing" => "→",
                "todo" => "○",
                _ => "?",
            };
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
            let status = match task.status.as_str() {
                "done" => "✓",
                "doing" => "→",
                "todo" => "○",
                _ => "?",
            };
            println!("  • #{} {} {}", task.id, task.name, status);
        }
    }

    if !ctx.dependencies.blocked_by_tasks.is_empty() {
        println!("\nBlocked by this task:");
        for task in &ctx.dependencies.blocked_by_tasks {
            let status = match task.status.as_str() {
                "done" => "✓",
                "doing" => "→",
                "todo" => "○",
                _ => "?",
            };
            println!("  • #{} {} {}", task.id, task.name, status);
        }
    }

    Ok(())
}

/// Print task tree in a hierarchical format (for dry-run mode)
fn print_task_tree(task: &intent_engine::plan::TaskTree, indent: usize) {
    let prefix = "  ".repeat(indent);
    println!("{}• {}", prefix, task.name);

    if let Some(spec) = &task.spec {
        println!("{}  Spec: {}", prefix, spec);
    }

    if let Some(priority) = &task.priority {
        println!("{}  Priority: {}", prefix, priority.as_str());
    }

    if let Some(depends_on) = &task.depends_on {
        if !depends_on.is_empty() {
            println!("{}  Depends on: {}", prefix, depends_on.join(", "));
        }
    }

    if let Some(children) = &task.children {
        for child in children {
            print_task_tree(child, indent + 1);
        }
    }
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

async fn handle_dashboard_command(dashboard_cmd: DashboardCommands) -> Result<()> {
    use chrono::Utc;
    use intent_engine::dashboard::{daemon, registry::*};

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

            // Load or create registry
            let mut registry = ProjectRegistry::load()?;

            // Check if already running using HTTP health check
            if let Some(existing) = registry.find_by_path(&project_path) {
                if check_dashboard_health(existing.port).await {
                    println!("Dashboard already running for this project:");
                    println!("  Port: {}", existing.port);
                    if let Some(pid) = existing.pid {
                        println!("  PID: {}", pid);
                    }
                    println!("  URL: http://127.0.0.1:{}", existing.port);
                    return Ok(());
                } else {
                    // Dashboard not responding, clean up stale state
                    tracing::info!(
                        "Cleaning up stale Dashboard state for {}",
                        project_path.display()
                    );
                    daemon::delete_pid_file(existing.port).ok();
                    registry.unregister(&project_path);
                }
            }

            // Allocate port (always 11391, or custom if specified)
            let allocated_port = if let Some(custom_port) = port {
                // Custom port specified - check if available
                if !ProjectRegistry::is_port_available(custom_port) {
                    return Err(IntentError::InvalidInput(format!(
                        "Port {} is already in use",
                        custom_port
                    )));
                }
                custom_port
            } else {
                // Use default fixed port (11391)
                registry.allocate_port()?
            };

            // Register project
            let registered_project = RegisteredProject {
                path: project_path.clone(),
                name: project_name.clone(),
                port: allocated_port,
                pid: None, // Will be set after server starts
                started_at: Utc::now().to_rfc3339(),
                db_path: db_path.clone(),
                mcp_connected: false,
                mcp_last_seen: None,
                mcp_agent: None,
            };

            registry.register(registered_project);
            registry.save()?;

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

                // Update registry with current PID
                let current_pid = std::process::id();
                if let Some(project) = registry.find_by_path_mut(&project_path) {
                    project.pid = Some(current_pid);
                    registry.save()?;
                }
                daemon::write_pid_file(allocated_port, current_pid)?;

                // Run server (blocks until terminated)
                let result = server.run().await;

                // Cleanup on exit
                daemon::delete_pid_file(allocated_port).ok();
                registry.unregister(&project_path);
                registry.save().ok();

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

                    // Pass project path via environment variable to ensure child process
                    // can find the project even if current_dir is not inherited correctly
                    // (fixes macOS setsid issue where std::env::current_dir() fails in child)
                    cmd.env("INTENT_ENGINE_PROJECT_DIR", &project_path);

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

                    // Pass project path via environment variable for consistency
                    cmd.env("INTENT_ENGINE_PROJECT_DIR", &project_path);

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

                // Update registry with PID
                if let Some(project) = registry.find_by_path_mut(&project_path) {
                    project.pid = Some(pid);
                    registry.save()?;
                }
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
                    registry.unregister(&project_path);
                    registry.save().ok();
                    return Err(IntentError::InvalidInput(
                        "Failed to start dashboard server".to_string(),
                    ));
                }

                Ok(())
            }
        },

        DashboardCommands::Stop { all } => {
            let mut registry = ProjectRegistry::load()?;

            if all {
                // Stop all dashboards
                let projects: Vec<_> = registry.list_all().to_vec();
                let mut stopped_count = 0;

                for project in projects {
                    // Check if dashboard is actually running via HTTP health check
                    let is_healthy = check_dashboard_health(project.port).await;

                    if is_healthy {
                        if let Some(pid) = project.pid {
                            if let Err(e) = daemon::stop_process(pid) {
                                eprintln!("Failed to stop {} (PID {}): {}", project.name, pid, e);
                            } else {
                                println!("Stopped dashboard for: {}", project.name);
                                stopped_count += 1;
                            }
                        } else {
                            eprintln!("Dashboard running but no PID recorded for {}", project.name);
                        }
                    } else {
                        tracing::debug!(
                            "Dashboard for {} not responding, cleaning up stale state",
                            project.name
                        );
                    }

                    daemon::delete_pid_file(project.port).ok();
                    registry.unregister(&project.path);
                }

                registry.save()?;
                println!("Stopped {} dashboard(s)", stopped_count);
            } else {
                // Stop dashboard for current project
                let project_ctx = ProjectContext::load_or_init().await?;
                let project_path = project_ctx.root.clone();

                if let Some(project) = registry.find_by_path(&project_path) {
                    let port = project.port;
                    let pid = project.pid;

                    // Check if dashboard is actually running via HTTP health check
                    if check_dashboard_health(port).await {
                        if let Some(pid) = pid {
                            daemon::stop_process(pid)?;
                            println!("Stopped dashboard (PID: {})", pid);
                        } else {
                            println!("Dashboard running but no PID recorded");
                        }
                    } else if let Some(pid) = pid {
                        println!("Dashboard not responding (stale PID: {}), cleaning up", pid);
                    } else {
                        println!("Dashboard not running");
                    }

                    daemon::delete_pid_file(port).ok();
                    registry.unregister(&project_path);
                    registry.save()?;
                } else {
                    eprintln!("No dashboard running for current project");
                    return Err(IntentError::InvalidInput(
                        "No dashboard instance found".to_string(),
                    ));
                }
            }

            Ok(())
        },

        DashboardCommands::Status { all } => {
            let mut registry = ProjectRegistry::load()?;

            // Clean up unhealthy dashboards via HTTP health checks
            registry.cleanup_unhealthy_dashboards().await;
            registry.save()?;

            if all {
                // Show all dashboards
                let projects = registry.list_all();

                if projects.is_empty() {
                    println!("No dashboard instances registered");
                    return Ok(());
                }

                println!("Dashboard instances:");
                for project in projects {
                    // Check dashboard health via HTTP
                    let status = if check_dashboard_health(project.port).await {
                        if let Some(pid) = project.pid {
                            format!("✓ Running (PID: {})", pid)
                        } else {
                            "✓ Running".to_string()
                        }
                    } else {
                        "✗ Stopped".to_string()
                    };

                    println!("\n  Project: {}", project.name);
                    println!("    Path: {}", project.path.display());
                    println!("    Port: {}", project.port);
                    println!("    Status: {}", status);
                    println!("    Started: {}", project.started_at);
                    println!("    URL: http://127.0.0.1:{}", project.port);
                }
            } else {
                // Show status for current project
                let project_ctx = ProjectContext::load_or_init().await?;
                let project_path = project_ctx.root.clone();

                if let Some(project) = registry.find_by_path(&project_path) {
                    // Check dashboard health via HTTP
                    let status = if check_dashboard_health(project.port).await {
                        if let Some(pid) = project.pid {
                            format!("✓ Running (PID: {})", pid)
                        } else {
                            "✓ Running".to_string()
                        }
                    } else {
                        "✗ Stopped".to_string()
                    };

                    println!("Dashboard status:");
                    println!("  Project: {}", project.name);
                    println!("  Port: {}", project.port);
                    println!("  Status: {}", status);
                    println!("  URL: http://127.0.0.1:{}", project.port);
                } else {
                    println!("No dashboard running for current project");
                }
            }

            Ok(())
        },

        DashboardCommands::List => {
            let mut registry = ProjectRegistry::load()?;

            // Clean up unhealthy dashboards via HTTP health checks
            registry.cleanup_unhealthy_dashboards().await;
            registry.save()?;

            let projects = registry.list_all();

            if projects.is_empty() {
                println!("No dashboard instances registered");
                return Ok(());
            }

            println!("Registered dashboard instances:");
            println!("{:<30} {:<8} {:<12} PATH", "PROJECT", "PORT", "STATUS");
            println!("{}", "-".repeat(80));

            for project in projects {
                // Check dashboard health via HTTP
                let status = if check_dashboard_health(project.port).await {
                    "Running"
                } else {
                    "Stopped"
                };

                println!(
                    "{:<30} {:<8} {:<12} {}",
                    project.name,
                    project.port,
                    status,
                    project.path.display()
                );
            }

            Ok(())
        },

        DashboardCommands::Open => {
            let registry = ProjectRegistry::load()?;
            let project_ctx = ProjectContext::load_or_init().await?;
            let project_path = project_ctx.root.clone();

            if let Some(project) = registry.find_by_path(&project_path) {
                // Check if dashboard is running via HTTP health check
                if !check_dashboard_health(project.port).await {
                    eprintln!("Dashboard is not running");
                    eprintln!("Start it with: ie dashboard start");
                    return Err(IntentError::InvalidInput(
                        "Dashboard not running".to_string(),
                    ));
                }

                let url = format!("http://127.0.0.1:{}", project.port);
                println!("Opening dashboard: {}", url);

                daemon::open_browser(&url)?;
            } else {
                eprintln!("No dashboard registered for current project");
                eprintln!("Start it with: ie dashboard start");
                return Err(IntentError::InvalidInput(
                    "No dashboard instance found".to_string(),
                ));
            }

            Ok(())
        },
    }
}
