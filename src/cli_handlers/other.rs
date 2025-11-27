use crate::cli::{CurrentAction, EventCommands};
use crate::cli_handlers::read_stdin;
use crate::cli_handlers::{check_dashboard_status, check_mcp_connections};
use crate::error::{IntentError, Result};
use crate::events::EventManager;
use crate::project::ProjectContext;
use crate::report::ReportManager;
use crate::sql_constants;
use crate::workspace::WorkspaceManager;
use sqlx::Row;
use std::path::PathBuf;

pub async fn handle_current_command(
    set: Option<i64>,
    command: Option<CurrentAction>,
) -> Result<()> {
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

pub async fn handle_report_command(
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

pub async fn handle_event_command(cmd: EventCommands) -> Result<()> {
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

pub async fn handle_search_command(
    query: &str,
    include_tasks: bool,
    include_events: bool,
    limit: Option<i64>,
) -> Result<()> {
    use crate::search::SearchManager;

    let ctx = ProjectContext::load_or_init().await?;
    let search_mgr = SearchManager::new(&ctx.pool);

    let results = search_mgr
        .unified_search(query, include_tasks, include_events, limit)
        .await?;

    println!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}

pub async fn handle_doctor_command() -> Result<()> {
    use serde_json::json;

    let mut checks = vec![];

    // 1. Database Path Resolution
    let db_path_info = ProjectContext::get_database_path_info();
    checks.push(json!({
        "check": "Database Path Resolution",
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

pub async fn handle_init_command(at: Option<String>, force: bool) -> Result<()> {
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

pub async fn handle_session_restore(
    include_events: usize,
    workspace: Option<String>,
) -> Result<()> {
    use crate::session_restore::SessionRestoreManager;

    // If workspace path is specified, change to that directory
    if let Some(ws_path) = workspace {
        std::env::set_current_dir(&ws_path)?;
    }

    // Try to load project context
    let ctx = match ProjectContext::load().await {
        Ok(ctx) => ctx,
        Err(_) => {
            // Workspace not found
            let result = crate::session_restore::SessionRestoreResult {
                status: crate::session_restore::SessionStatus::Error,
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
                error_type: Some(crate::session_restore::ErrorType::WorkspaceNotFound),
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

pub async fn handle_setup(
    target: Option<String>,
    scope: &str,
    force: bool,
    config_path: Option<String>,
) -> Result<()> {
    use crate::setup::claude_code::ClaudeCodeSetup;
    use crate::setup::{SetupModule, SetupOptions, SetupScope};

    println!("Intent-Engine Unified Setup");
    println!("============================\n");

    // Parse scope
    let setup_scope: SetupScope = scope.parse()?;

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
        use crate::setup::interactive::SetupWizard;

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

/// Check SessionStart hook configuration and effectiveness
pub fn check_session_start_hook() -> serde_json::Value {
    use crate::setup::common::get_home_dir;
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

pub fn handle_logs_command(
    mode: Option<String>,
    level: Option<String>,
    since: Option<String>,
    until: Option<String>,
    limit: Option<usize>,
    follow: bool,
    export: String,
) -> Result<()> {
    use crate::logs::{
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
