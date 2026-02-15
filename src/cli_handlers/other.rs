// Note: CurrentAction and EventCommands removed in v0.10.1 CLI simplification
// These functions are kept for potential Dashboard/MCP use but not exposed in CLI
// use crate::cli::{CurrentAction, EventCommands};
use crate::cli_handlers::read_stdin;
use crate::error::{IntentError, Result};
use crate::events::EventManager;
use crate::project::ProjectContext;
use crate::report::ReportManager;
use crate::tasks::TaskManager;
use crate::time_utils::parse_date_filter;
use crate::workspace::WorkspaceManager;
use std::path::PathBuf;

// Stub types for deprecated CLI commands (no longer in cli.rs)
#[allow(dead_code)]
pub enum CurrentAction {
    Set { task_id: i64 },
    Clear,
}

#[allow(dead_code)]
pub enum EventCommands {
    Add {
        task_id: Option<i64>,
        log_type: String,
        data_stdin: bool,
    },
    List {
        task_id: Option<i64>,
        log_type: Option<String>,
        since: Option<String>,
        limit: Option<i64>,
    },
}

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
        let response = workspace_mgr.set_current_task(task_id, None).await?;
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
            let response = workspace_mgr.set_current_task(task_id, None).await?;
            println!("✓ Switched to task #{}", task_id);
            println!("{}", serde_json::to_string_pretty(&response)?);
        },
        Some(CurrentAction::Clear) => {
            eprintln!("⚠️  Warning: 'ie current clear' is a low-level atomic command.");
            eprintln!("   For normal use, prefer 'ie task done' or 'ie task switch' which ensures data consistency.");
            eprintln!();
            workspace_mgr.clear_current_task(None).await?;
            println!("✓ Current task cleared");
        },
        None => {
            // Default: display current task in JSON format
            let response = workspace_mgr.get_current_task(None).await?;
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
            let project_path = ctx.root.to_string_lossy().to_string();
            let event_mgr = EventManager::with_project_path(&ctx.pool, project_path);

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
                // Fall back to current_task_id from sessions table for this session
                let session_id = crate::workspace::resolve_session_id(None);
                let current_task_id: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
                    "SELECT current_task_id FROM sessions WHERE session_id = ?",
                )
                .bind(&session_id)
                .fetch_optional(&ctx.pool)
                .await?
                .flatten();

                current_task_id
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

/// Check if query is a #ID format (e.g., "#123", "#1")
/// Returns Some(id) if it's a task ID query, None otherwise
fn parse_task_id_query(query: &str) -> Option<i64> {
    let query = query.trim();

    // Must start with # and have at least one digit after
    if !query.starts_with('#') || query.len() < 2 {
        return None;
    }

    // The rest must be all digits
    let id_part = &query[1..];
    id_part.parse::<i64>().ok()
}

/// Safely truncate a UTF-8 string to a maximum number of characters
/// Returns the truncated string with "..." appended if truncation occurred
fn truncate_str(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

/// Check if query is a status keyword combination (todo, doing, done)
/// Returns Some(statuses) if it's a status query, None otherwise
fn parse_status_keywords(query: &str) -> Option<Vec<String>> {
    let query_lower = query.to_lowercase();
    let words: Vec<&str> = query_lower.split_whitespace().collect();

    // Must have at least one word
    if words.is_empty() {
        return None;
    }

    // All words must be status keywords
    let valid_statuses = ["todo", "doing", "done"];
    let mut statuses: Vec<String> = Vec::new();

    for word in words {
        if valid_statuses.contains(&word) {
            if !statuses.iter().any(|s| s == word) {
                statuses.push(word.to_string());
            }
        } else {
            // Found a non-status word, not a status query
            return None;
        }
    }

    Some(statuses)
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_search_command(
    query: &str,
    include_tasks: bool,
    include_events: bool,
    limit: Option<i64>,
    offset: Option<i64>,
    since: Option<String>,
    until: Option<String>,
    format: &str,
) -> Result<()> {
    use crate::search::SearchManager;
    use chrono::{DateTime, Utc};

    let ctx = ProjectContext::load_or_init().await?;

    // Parse date filters
    let since_dt: Option<DateTime<Utc>> = if let Some(ref s) = since {
        Some(parse_date_filter(s).map_err(IntentError::InvalidInput)?)
    } else {
        None
    };

    let until_dt: Option<DateTime<Utc>> = if let Some(ref u) = until {
        Some(parse_date_filter(u).map_err(IntentError::InvalidInput)?)
    } else {
        None
    };

    // Check if query is a #ID format (e.g., "#123", "#1")
    if let Some(task_id) = parse_task_id_query(query) {
        let task_mgr = TaskManager::new(&ctx.pool);
        match task_mgr.get_task(task_id).await {
            Ok(task) => {
                if format == "json" {
                    println!("{}", serde_json::to_string_pretty(&task)?);
                } else {
                    let status_icon = match task.status.as_str() {
                        "todo" => "○",
                        "doing" => "●",
                        "done" => "✓",
                        _ => "?",
                    };
                    let parent_info = task
                        .parent_id
                        .map(|p| format!(" (parent: #{})", p))
                        .unwrap_or_default();
                    let priority_info = task
                        .priority
                        .map(|p| format!(" [P{}]", p))
                        .unwrap_or_default();
                    println!("Task #{}", task.id);
                    println!(
                        "  {} {}{}{}",
                        status_icon, task.name, parent_info, priority_info
                    );
                    if let Some(spec) = &task.spec {
                        if !spec.is_empty() {
                            println!("  Spec: {}", spec);
                        }
                    }
                    println!("  Owner: {}", task.owner);
                    if let Some(ts) = task.first_todo_at {
                        print!("  todo: {} ", ts.format("%Y-%m-%d %H:%M:%S"));
                    }
                    if let Some(ts) = task.first_doing_at {
                        print!("doing: {} ", ts.format("%Y-%m-%d %H:%M:%S"));
                    }
                    if let Some(ts) = task.first_done_at {
                        print!("done: {}", ts.format("%Y-%m-%d %H:%M:%S"));
                    }
                    if task.first_todo_at.is_some()
                        || task.first_doing_at.is_some()
                        || task.first_done_at.is_some()
                    {
                        println!();
                    }
                }
                return Ok(());
            },
            Err(_) => {
                // Task not found, fall through to FTS5 search
                // (user might be searching for "#123" as text)
            },
        }
    }

    // Check if query is a status keyword combination
    if let Some(statuses) = parse_status_keywords(query) {
        // Use TaskManager::find_tasks for status filtering
        let task_mgr = TaskManager::new(&ctx.pool);

        // Collect tasks for each status
        // When date filters are used, fetch more tasks initially
        // (we'll apply limit after filtering)
        const DATE_FILTER_FETCH_LIMIT: i64 = 10_000;
        let fetch_limit = if since_dt.is_some() || until_dt.is_some() {
            Some(DATE_FILTER_FETCH_LIMIT)
        } else {
            limit
        };

        let mut all_tasks = Vec::new();
        for status in &statuses {
            let result = task_mgr
                .find_tasks(Some(status), None, None, fetch_limit, offset)
                .await?;
            all_tasks.extend(result.tasks);
        }

        // Apply date filters based on status
        if since_dt.is_some() || until_dt.is_some() {
            all_tasks.retain(|task| {
                // Determine which timestamp to use based on task status
                let timestamp = match task.status.as_str() {
                    "done" => task.first_done_at,
                    "doing" => task.first_doing_at,
                    _ => task.first_todo_at, // todo or unknown
                };

                // If no timestamp, exclude from date-filtered results
                let Some(ts) = timestamp else {
                    return false;
                };

                // Check since filter
                if let Some(ref since) = since_dt {
                    if ts < *since {
                        return false;
                    }
                }

                // Check until filter
                if let Some(ref until) = until_dt {
                    if ts > *until {
                        return false;
                    }
                }

                true
            });
        }

        // Sort by priority, then by id
        all_tasks.sort_by(|a, b| {
            let pri_a = a.priority.unwrap_or(999);
            let pri_b = b.priority.unwrap_or(999);
            pri_a.cmp(&pri_b).then_with(|| a.id.cmp(&b.id))
        });

        // Apply limit if specified
        let limit = limit.unwrap_or(100) as usize;
        if all_tasks.len() > limit {
            all_tasks.truncate(limit);
        }

        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&all_tasks)?);
        } else {
            // Text format: status filter results
            let status_str = statuses.join(", ");
            let date_filter_str = match (&since, &until) {
                (Some(s), Some(u)) => format!(" (from {} to {})", s, u),
                (Some(s), None) => format!(" (since {})", s),
                (None, Some(u)) => format!(" (until {})", u),
                (None, None) => String::new(),
            };
            println!(
                "Tasks with status [{}]{}: {} found",
                status_str,
                date_filter_str,
                all_tasks.len()
            );
            println!();
            for task in &all_tasks {
                let status_icon = match task.status.as_str() {
                    "todo" => "○",
                    "doing" => "●",
                    "done" => "✓",
                    _ => "?",
                };
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
                    status_icon, task.id, task.name, parent_info, priority_info
                );
                if let Some(spec) = &task.spec {
                    if !spec.is_empty() {
                        println!("      Spec: {}", truncate_str(spec, 60));
                    }
                }
                println!("      Owner: {}", task.owner);
                if let Some(ts) = task.first_todo_at {
                    print!("      todo: {} ", ts.format("%m-%d %H:%M:%S"));
                }
                if let Some(ts) = task.first_doing_at {
                    print!("doing: {} ", ts.format("%m-%d %H:%M:%S"));
                }
                if let Some(ts) = task.first_done_at {
                    print!("done: {}", ts.format("%m-%d %H:%M:%S"));
                }
                if task.first_todo_at.is_some()
                    || task.first_doing_at.is_some()
                    || task.first_done_at.is_some()
                {
                    println!();
                }
            }
        }
        return Ok(());
    }

    // Regular FTS5 search
    if since_dt.is_some() || until_dt.is_some() {
        eprintln!("Warning: --since/--until are ignored for fulltext search (only apply to status keyword queries)");
    }
    let search_mgr = SearchManager::new(&ctx.pool);

    let results = search_mgr
        .search(query, include_tasks, include_events, limit, offset, false)
        .await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        use crate::db::models::SearchResult;

        // Text format: FTS5 search results
        println!(
            "Search: \"{}\" → {} tasks, {} events (limit: {}, offset: {})",
            query, results.total_tasks, results.total_events, results.limit, results.offset
        );
        println!();

        for result in &results.results {
            match result {
                SearchResult::Task {
                    task,
                    match_field,
                    match_snippet,
                } => {
                    let status_icon = match task.status.as_str() {
                        "todo" => "○",
                        "doing" => "●",
                        "done" => "✓",
                        _ => "?",
                    };
                    let parent_info = task
                        .parent_id
                        .map(|p| format!(" (parent: #{})", p))
                        .unwrap_or_default();
                    let priority_info = task
                        .priority
                        .map(|p| format!(" [P{}]", p))
                        .unwrap_or_default();
                    println!(
                        "  {} #{} {} [match: {}]{}{}",
                        status_icon, task.id, task.name, match_field, parent_info, priority_info
                    );
                    if let Some(spec) = &task.spec {
                        if !spec.is_empty() {
                            println!("      Spec: {}", truncate_str(spec, 60));
                        }
                    }
                    if !match_snippet.is_empty() {
                        println!("      Snippet: {}", match_snippet);
                    }
                    println!("      Owner: {}", task.owner);
                    if let Some(ts) = task.first_todo_at {
                        print!("      todo: {} ", ts.format("%m-%d %H:%M:%S"));
                    }
                    if let Some(ts) = task.first_doing_at {
                        print!("doing: {} ", ts.format("%m-%d %H:%M:%S"));
                    }
                    if let Some(ts) = task.first_done_at {
                        print!("done: {}", ts.format("%m-%d %H:%M:%S"));
                    }
                    if task.first_todo_at.is_some()
                        || task.first_doing_at.is_some()
                        || task.first_done_at.is_some()
                    {
                        println!();
                    }
                },
                SearchResult::Event {
                    event,
                    task_chain,
                    match_snippet,
                } => {
                    let icon = match event.log_type.as_str() {
                        "decision" => "💡",
                        "blocker" => "🚫",
                        "milestone" => "🎯",
                        _ => "📝",
                    };
                    println!(
                        "  {} #{} [{}] (task #{}) {}",
                        icon,
                        event.id,
                        event.log_type,
                        event.task_id,
                        event.timestamp.format("%Y-%m-%d %H:%M:%S")
                    );
                    println!("      Message: {}", event.discussion_data);
                    if !match_snippet.is_empty() {
                        println!("      Snippet: {}", match_snippet);
                    }
                    if !task_chain.is_empty() {
                        let chain_str: Vec<String> = task_chain
                            .iter()
                            .map(|t| format!("#{} {}", t.id, t.name))
                            .collect();
                        println!("      Task chain: {}", chain_str.join(" → "));
                    }
                },
            }
        }

        if results.has_more {
            println!();
            println!(
                "  ... more results available (use --offset {})",
                results.offset + results.limit
            );
        }
    }
    Ok(())
}

pub async fn handle_doctor_command() -> Result<()> {
    use crate::cli_handlers::dashboard::{check_dashboard_health, DASHBOARD_PORT};

    // Get database path info
    let db_path_info = ProjectContext::get_database_path_info();

    // Print database location
    println!("Database:");
    if let Some(db_path) = &db_path_info.final_database_path {
        println!("  {}", db_path);
    } else {
        println!("  Not found");
    }
    println!();

    // Print ancestor directories with databases
    let dirs_with_db: Vec<&String> = db_path_info
        .directories_checked
        .iter()
        .filter(|d| d.has_intent_engine)
        .map(|d| &d.path)
        .collect();

    if !dirs_with_db.is_empty() {
        println!("Ancestor directories with databases:");
        for dir in dirs_with_db {
            println!("  {}", dir);
        }
    } else {
        println!("Ancestor directories with databases: None");
    }
    println!();

    // Check dashboard status
    print!("Dashboard: ");
    let dashboard_health = check_dashboard_health(DASHBOARD_PORT).await;
    if dashboard_health {
        println!("Running (http://127.0.0.1:{})", DASHBOARD_PORT);
    } else {
        println!("Not running (start with 'ie dashboard start')");
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
                recommended_task: None,
                top_pending_tasks: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // parse_task_id_query tests
    // ============================================================================

    #[test]
    fn test_parse_task_id_query_valid() {
        assert_eq!(parse_task_id_query("#1"), Some(1));
        assert_eq!(parse_task_id_query("#123"), Some(123));
        assert_eq!(parse_task_id_query("#999999"), Some(999999));
    }

    #[test]
    fn test_parse_task_id_query_with_whitespace() {
        assert_eq!(parse_task_id_query("  #1  "), Some(1));
        assert_eq!(parse_task_id_query("\t#42\n"), Some(42));
    }

    #[test]
    fn test_parse_task_id_query_invalid() {
        // Not starting with #
        assert_eq!(parse_task_id_query("123"), None);
        assert_eq!(parse_task_id_query("task"), None);

        // Only #
        assert_eq!(parse_task_id_query("#"), None);

        // # followed by non-digits
        assert_eq!(parse_task_id_query("#abc"), None);
        assert_eq!(parse_task_id_query("#1a"), None);
        assert_eq!(parse_task_id_query("#a1"), None);

        // Mixed content
        assert_eq!(parse_task_id_query("#123 task"), None);
        assert_eq!(parse_task_id_query("task #123"), None);

        // Negative numbers (technically parsed, but task IDs are positive in practice)
        // Note: i64::parse accepts negative, so #-1 returns Some(-1)
        assert_eq!(parse_task_id_query("#-1"), Some(-1));

        // Empty
        assert_eq!(parse_task_id_query(""), None);
    }

    // ============================================================================
    // truncate_str tests (UTF-8 safe truncation)
    // ============================================================================

    #[test]
    fn test_truncate_str_ascii() {
        // Short string - no truncation
        assert_eq!(truncate_str("hello", 10), "hello");

        // Exact length - no truncation
        assert_eq!(truncate_str("hello", 5), "hello");

        // Needs truncation
        assert_eq!(truncate_str("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_str_chinese() {
        // Short Chinese string - no truncation
        assert_eq!(truncate_str("你好", 10), "你好");

        // Chinese string needs truncation
        // "根据覆盖缺口分析补充" = 10 chars, truncate to 8 means keep 5 + "..."
        let chinese = "根据覆盖缺口分析补充";
        let result = truncate_str(chinese, 8);
        assert_eq!(result, "根据覆盖缺...");
        assert!(!result.contains('\u{FFFD}')); // No replacement chars
    }

    #[test]
    fn test_truncate_str_mixed() {
        // Mixed ASCII and Chinese
        let mixed = "Task: 实现用户认证功能";
        let result = truncate_str(mixed, 12);
        assert_eq!(result, "Task: 实现用...");
    }

    #[test]
    fn test_truncate_str_edge_cases() {
        // Empty string
        assert_eq!(truncate_str("", 10), "");

        // Max chars less than 3 (edge case for "...")
        assert_eq!(truncate_str("hello", 3), "...");

        // Single char with truncation
        assert_eq!(truncate_str("hello", 4), "h...");
    }

    #[test]
    fn test_truncate_str_emoji() {
        // Emoji (multi-byte UTF-8)
        let emoji = "🚀🎉🔥💡";
        let result = truncate_str(emoji, 4);
        assert_eq!(result, "🚀🎉🔥💡"); // No truncation needed

        let result = truncate_str(emoji, 3);
        assert_eq!(result, "..."); // All replaced by ...
    }

    // ============================================================================
    // parse_status_keywords tests
    // ============================================================================

    #[test]
    fn test_parse_status_keywords_valid() {
        assert_eq!(
            parse_status_keywords("todo"),
            Some(vec!["todo".to_string()])
        );
        assert_eq!(
            parse_status_keywords("doing"),
            Some(vec!["doing".to_string()])
        );
        assert_eq!(
            parse_status_keywords("done"),
            Some(vec!["done".to_string()])
        );
    }

    #[test]
    fn test_parse_status_keywords_multiple() {
        let result = parse_status_keywords("todo doing");
        assert!(result.is_some());
        let statuses = result.unwrap();
        assert!(statuses.contains(&"todo".to_string()));
        assert!(statuses.contains(&"doing".to_string()));
    }

    #[test]
    fn test_parse_status_keywords_case_insensitive() {
        assert_eq!(
            parse_status_keywords("TODO"),
            Some(vec!["todo".to_string()])
        );
        assert_eq!(
            parse_status_keywords("DoInG"),
            Some(vec!["doing".to_string()])
        );
    }

    #[test]
    fn test_parse_status_keywords_invalid() {
        // Mixed with non-status words
        assert_eq!(parse_status_keywords("todo task"), None);
        assert_eq!(parse_status_keywords("search term"), None);

        // Empty
        assert_eq!(parse_status_keywords(""), None);
        assert_eq!(parse_status_keywords("   "), None);
    }

    #[test]
    fn test_parse_status_keywords_dedup() {
        // Duplicate statuses should be deduplicated
        let result = parse_status_keywords("todo todo todo");
        assert!(result.is_some());
        let statuses = result.unwrap();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0], "todo");
    }
}
