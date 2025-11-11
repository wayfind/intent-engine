use clap::Parser;
use intent_engine::cli::{Cli, Commands, EventCommands, TaskCommands};
use intent_engine::error::{IntentError, Result};
use intent_engine::events::EventManager;
use intent_engine::project::ProjectContext;
use intent_engine::report::ReportManager;
use intent_engine::tasks::TaskManager;
use intent_engine::workspace::WorkspaceManager;
use sqlx::Row;
use std::io::{self, Read};

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

    if let Err(e) = run().await {
        let error_response = e.to_error_response();
        eprintln!("{}", serde_json::to_string_pretty(&error_response).unwrap());
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Task(task_cmd) => handle_task_command(task_cmd).await?,
        Commands::Current { set } => handle_current_command(set).await?,
        Commands::Report {
            since,
            status,
            filter_name,
            filter_spec,
            format: _,
            summary_only,
        } => handle_report_command(since, status, filter_name, filter_spec, summary_only).await?,
        Commands::Event(event_cmd) => handle_event_command(event_cmd).await?,
        Commands::Doctor => handle_doctor_command().await?,
        Commands::McpServer => {
            // Run MCP server - this never returns unless there's an error
            // io::Error is automatically converted to IntentError::IoError via #[from]
            intent_engine::mcp::run().await?;
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

            let parent_opt = parent.map(Some);
            let task = task_mgr
                .update_task(
                    id,
                    name.as_deref(),
                    spec.as_deref(),
                    parent_opt,
                    status.as_deref(),
                    complexity,
                    priority,
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

        TaskCommands::Find { status, parent } => {
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

        TaskCommands::Search { query } => {
            let ctx = ProjectContext::load().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let results = task_mgr.search_tasks(&query).await?;
            println!("{}", serde_json::to_string_pretty(&results)?);
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
    }

    Ok(())
}

async fn handle_current_command(set: Option<i64>) -> Result<()> {
    if let Some(task_id) = set {
        let ctx = ProjectContext::load_or_init().await?;
        let workspace_mgr = WorkspaceManager::new(&ctx.pool);

        let response = workspace_mgr.set_current_task(task_id).await?;
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        let ctx = ProjectContext::load().await?;
        let workspace_mgr = WorkspaceManager::new(&ctx.pool);

        let response = workspace_mgr.get_current_task().await?;
        println!("{}", serde_json::to_string_pretty(&response)?);
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

    let result = json!({
        "summary": if all_passed { "✓ All checks passed" } else { "✗ Some checks failed" },
        "overall_status": if all_passed { "healthy" } else { "unhealthy" },
        "checks": checks
    });

    println!("{}", serde_json::to_string_pretty(&result)?);

    if !all_passed {
        std::process::exit(1);
    }

    Ok(())
}
