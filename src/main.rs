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
        }

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
        }

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
        }

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
        }

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
        }

        TaskCommands::Start { id, with_events } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let task = task_mgr.start_task(id, with_events).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        }

        TaskCommands::Done { id } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let task = task_mgr.done_task(id).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        }

        TaskCommands::PickNext {
            max_count,
            capacity,
        } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let tasks = task_mgr.pick_next_tasks(max_count, capacity).await?;
            println!("{}", serde_json::to_string_pretty(&tasks)?);
        }

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
        }

        TaskCommands::Switch { id } => {
            let ctx = ProjectContext::load_or_init().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let task = task_mgr.switch_to_task(id).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        }
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

            let event = event_mgr.add_event(task_id, &log_type, &data).await?;
            println!("{}", serde_json::to_string_pretty(&event)?);
        }

        EventCommands::List { task_id, limit } => {
            let ctx = ProjectContext::load().await?;
            let event_mgr = EventManager::new(&ctx.pool);

            let events = event_mgr.list_events(task_id, limit).await?;
            println!("{}", serde_json::to_string_pretty(&events)?);
        }
    }

    Ok(())
}

fn read_stdin() -> Result<String> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer.trim().to_string())
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
        }
        Ok(None) | Err(_) => {
            all_passed = false;
            checks.push(json!({
                "check": "SQLite",
                "status": "✗ FAIL",
                "details": "Unable to query SQLite version"
            }));
        }
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
                }
                Err(e) => {
                    all_passed = false;
                    checks.push(json!({
                        "check": "Database Connection",
                        "status": "✗ FAIL",
                        "details": format!("Database query failed: {}", e)
                    }));
                }
            }
        }
        Err(e) => {
            all_passed = false;
            checks.push(json!({
                "check": "Database Initialization",
                "status": "✗ FAIL",
                "details": format!("Failed to initialize database: {}", e)
            }));
        }
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
