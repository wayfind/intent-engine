//! `ie-neo4j` — Intent-Engine with Neo4j graph database backend.
//!
//! This binary provides the same CLI interface as `ie`, but stores all data
//! in Neo4j instead of SQLite. It reuses the same types (Task, Event, etc.)
//! and CLI definitions from the main intent-engine crate.
//!
//! Handler functions are shared with the SQLite backend via generic traits
//! (TaskBackend, WorkspaceBackend, EventBackend, PlanBackend, SearchBackend).
//!
//! Usage:
//!   NEO4J_URI="neo4j+s://..." NEO4J_PASSWORD="..." ie-neo4j status

use clap::Parser;
use intent_engine::cli::{Cli, Commands};
use intent_engine::cli_handlers::{
    handle_log, handle_search_command, handle_status, handle_task_command, print_plan_result,
    read_stdin,
};
use intent_engine::error::{IntentError, Result};
use intent_engine::neo4j::Neo4jContext;
use intent_engine::plan::{cleanup_included_files, process_file_includes, PlanRequest};

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
        } => {
            let ctx = Neo4jContext::connect().await?;
            let task_mgr = ctx.task_manager();
            handle_search_command(
                &task_mgr, &query, tasks, events, limit, offset, since, until, &format,
            )
            .await?;
        },

        _ => {
            eprintln!("Command not yet implemented for Neo4j backend.");
            eprintln!("Currently supported: ie-neo4j status, ie-neo4j task *, ie-neo4j log, ie-neo4j plan, ie-neo4j search");
            std::process::exit(1);
        },
    }

    Ok(())
}
