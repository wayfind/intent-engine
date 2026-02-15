//! `ie-neo4j` — Intent-Engine with Neo4j graph database backend.
//!
//! This binary provides the same CLI interface as `ie`, but stores all data
//! in Neo4j instead of SQLite. It reuses the same types (Task, Event, etc.)
//! and CLI definitions from the main intent-engine crate.
//!
//! Handler functions are shared with the SQLite backend via generic traits
//! (TaskBackend, WorkspaceBackend, EventBackend, PlanBackend).
//! Only `handle_search` remains local (pending SearchBackend trait).
//!
//! Usage:
//!   NEO4J_URI="neo4j+s://..." NEO4J_PASSWORD="..." ie-neo4j status

use clap::Parser;
use intent_engine::cli::{Cli, Commands};
use intent_engine::cli_handlers::utils::{print_task_summary, status_icon};
use intent_engine::cli_handlers::{
    handle_log, handle_status, handle_task_command, print_plan_result, read_stdin,
};
use intent_engine::error::{IntentError, Result};
use intent_engine::neo4j::Neo4jContext;
use intent_engine::plan::{cleanup_included_files, process_file_includes, PlanRequest};
use intent_engine::time_utils::parse_date_filter;

#[tokio::main]
async fn main() {
    #[cfg(windows)]
    if let Err(e) = intent_engine::windows_console::setup_windows_console() {
        eprintln!("Warning: Failed to setup Windows console UTF-8: {}", e);
    }

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        let error_response = e.to_error_response();
        eprintln!("{}", serde_json::to_string_pretty(&error_response).unwrap());
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Status {
            task_id,
            with_events,
            format,
        } => {
            let ctx = Neo4jContext::connect().await?;
            let task_mgr = ctx.task_manager();
            let ws_mgr = ctx.workspace_manager();
            handle_status(&task_mgr, &ws_mgr, task_id, with_events, &format).await?;
        },

        Commands::Task(task_cmd) => {
            let ctx = Neo4jContext::connect().await?;
            let task_mgr = ctx.task_manager();
            let ws_mgr = ctx.workspace_manager();
            handle_task_command(&task_mgr, &ws_mgr, task_cmd).await?;
        },

        Commands::Log {
            event_type,
            message,
            task,
            format,
        } => {
            let ctx = Neo4jContext::connect().await?;
            let event_mgr = ctx.event_manager();
            let ws_mgr = ctx.workspace_manager();
            handle_log(&event_mgr, &ws_mgr, event_type, &message, task, &format).await?;
        },

        Commands::Plan { format } => {
            let json_input = read_stdin()?;
            let mut request: PlanRequest = serde_json::from_str(&json_input)
                .map_err(|e| IntentError::InvalidInput(format!("Invalid JSON: {}", e)))?;

            let file_include_result =
                process_file_includes(&mut request).map_err(IntentError::InvalidInput)?;

            let ctx = Neo4jContext::connect().await?;
            let ws_mgr = ctx.workspace_manager();

            let current = ws_mgr.get_current_task(None).await?;
            let mut executor = ctx.plan_executor();
            if let Some(current_task_id) = current.current_task_id {
                executor = executor.with_default_parent(current_task_id);
            }

            let result = executor.execute(&request).await?;

            if result.success && !file_include_result.files_to_delete.is_empty() {
                cleanup_included_files(&file_include_result.files_to_delete);
            }

            print_plan_result(&result, &format)?;
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
        } => handle_search(query, tasks, events, limit, offset, since, until, format).await?,

        _ => {
            eprintln!("Command not yet implemented for Neo4j backend.");
            eprintln!("Currently supported: ie-neo4j status, ie-neo4j task *, ie-neo4j log, ie-neo4j plan, ie-neo4j search");
            std::process::exit(1);
        },
    }

    Ok(())
}

// ── Search Command (local — pending SearchBackend trait) ────────

#[allow(clippy::too_many_arguments)]
async fn handle_search(
    query: String,
    include_tasks: bool,
    include_events: bool,
    limit: Option<i64>,
    offset: Option<i64>,
    since: Option<String>,
    until: Option<String>,
    format: String,
) -> Result<()> {
    use chrono::{DateTime, Utc};

    let ctx = Neo4jContext::connect().await?;

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

    // Mode 1: #ID lookup
    if let Some(task_id) = parse_task_id_query(&query) {
        let task_mgr = ctx.task_manager();
        match task_mgr.get_task(task_id).await {
            Ok(task) => {
                if format == "json" {
                    println!("{}", serde_json::to_string_pretty(&task)?);
                } else {
                    println!("Task #{}", task.id);
                    print_task_summary(&task);
                }
                return Ok(());
            },
            Err(_) => {
                // Fall through to fulltext search
            },
        }
    }

    // Mode 2: Status keywords (todo/doing/done)
    if let Some(statuses) = parse_status_keywords(&query) {
        let task_mgr = ctx.task_manager();

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
                let timestamp = match task.status.as_str() {
                    "done" => task.first_done_at,
                    "doing" => task.first_doing_at,
                    _ => task.first_todo_at,
                };
                let Some(ts) = timestamp else {
                    return false;
                };
                if let Some(ref since) = since_dt {
                    if ts < *since {
                        return false;
                    }
                }
                if let Some(ref until) = until_dt {
                    if ts > *until {
                        return false;
                    }
                }
                true
            });
        }

        // Sort by priority then id
        all_tasks.sort_by(|a, b| {
            let pri_a = a.priority.unwrap_or(999);
            let pri_b = b.priority.unwrap_or(999);
            pri_a.cmp(&pri_b).then_with(|| a.id.cmp(&b.id))
        });

        let max = limit.unwrap_or(100) as usize;
        if all_tasks.len() > max {
            all_tasks.truncate(max);
        }

        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&all_tasks)?);
        } else {
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
                let icon = status_icon(&task.status);
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
                    icon, task.id, task.name, parent_info, priority_info
                );
            }
        }
        return Ok(());
    }

    // Mode 3: Fulltext search
    if since_dt.is_some() || until_dt.is_some() {
        eprintln!("Warning: --since/--until are ignored for fulltext search (only apply to status keyword queries)");
    }
    let search_mgr = ctx.search_manager();
    let results = search_mgr
        .search(&query, include_tasks, include_events, limit, offset)
        .await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        use intent_engine::db::models::SearchResult;

        println!(
            "Search: \"{}\" -> {} tasks, {} events (limit: {}, offset: {})",
            query, results.total_tasks, results.total_events, results.limit, results.offset
        );
        println!();

        for result in &results.results {
            match result {
                SearchResult::Task {
                    task,
                    match_snippet,
                    match_field,
                } => {
                    let icon = status_icon(&task.status);
                    println!("  {} #{} {} [{}]", icon, task.id, task.name, task.status);
                    println!("    Match ({}): {}", match_field, match_snippet);
                },
                SearchResult::Event {
                    event,
                    task_chain,
                    match_snippet,
                } => {
                    let chain_str: String = task_chain
                        .iter()
                        .map(|t| format!("#{} {}", t.id, t.name))
                        .collect::<Vec<_>>()
                        .join(" > ");
                    println!(
                        "  [Event] #{} ({}) on task #{}",
                        event.id, event.log_type, event.task_id
                    );
                    if !chain_str.is_empty() {
                        println!("    Chain: {}", chain_str);
                    }
                    println!("    Match: {}", match_snippet);
                },
            }
        }

        if results.has_more {
            println!(
                "\n  ... more results available (use --offset {})",
                results.offset + results.limit
            );
        }
    }

    Ok(())
}

/// Parse a #ID query (e.g., "#123")
fn parse_task_id_query(query: &str) -> Option<i64> {
    let query = query.trim();
    if !query.starts_with('#') || query.len() < 2 {
        return None;
    }
    query[1..].parse::<i64>().ok()
}

/// Check if query is a status keyword combination (todo, doing, done)
fn parse_status_keywords(query: &str) -> Option<Vec<String>> {
    let query_lower = query.to_lowercase();
    let words: Vec<&str> = query_lower.split_whitespace().collect();

    if words.is_empty() {
        return None;
    }

    let valid_statuses = ["todo", "doing", "done"];
    let mut statuses: Vec<String> = Vec::new();

    for word in words {
        if valid_statuses.contains(&word) {
            if !statuses.iter().any(|s| s == word) {
                statuses.push(word.to_string());
            }
        } else {
            return None;
        }
    }

    Some(statuses)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_task_id_query ──────────────────────────────────────

    #[test]
    fn test_parse_task_id_valid() {
        assert_eq!(parse_task_id_query("#123"), Some(123));
        assert_eq!(parse_task_id_query("#1"), Some(1));
        assert_eq!(parse_task_id_query("#0"), Some(0));
        assert_eq!(parse_task_id_query("  #42  "), Some(42));
    }

    #[test]
    fn test_parse_task_id_invalid() {
        assert_eq!(parse_task_id_query("123"), None);
        assert_eq!(parse_task_id_query("#"), None);
        assert_eq!(parse_task_id_query("#abc"), None);
        assert_eq!(parse_task_id_query(""), None);
        assert_eq!(parse_task_id_query("todo"), None);
        assert_eq!(parse_task_id_query("#-1"), Some(-1)); // negative parses as i64
        assert_eq!(parse_task_id_query("#1.5"), None); // float
    }

    // ── parse_status_keywords ────────────────────────────────────

    #[test]
    fn test_parse_status_single() {
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
    fn test_parse_status_multiple() {
        assert_eq!(
            parse_status_keywords("todo doing"),
            Some(vec!["todo".to_string(), "doing".to_string()])
        );
        assert_eq!(
            parse_status_keywords("todo doing done"),
            Some(vec![
                "todo".to_string(),
                "doing".to_string(),
                "done".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_status_case_insensitive() {
        assert_eq!(
            parse_status_keywords("TODO"),
            Some(vec!["todo".to_string()])
        );
        assert_eq!(
            parse_status_keywords("Todo Doing"),
            Some(vec!["todo".to_string(), "doing".to_string()])
        );
    }

    #[test]
    fn test_parse_status_dedup() {
        assert_eq!(
            parse_status_keywords("todo todo"),
            Some(vec!["todo".to_string()])
        );
    }

    #[test]
    fn test_parse_status_not_status() {
        assert_eq!(parse_status_keywords("hello"), None);
        assert_eq!(parse_status_keywords("todo hello"), None);
        assert_eq!(parse_status_keywords(""), None);
        assert_eq!(parse_status_keywords("   "), None);
    }
}
