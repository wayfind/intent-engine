use crate::cli::TaskCommands;
use crate::cli_handlers::read_stdin;
use crate::cli_handlers::utils::print_task_context;
use crate::error::{IntentError, Result};
use crate::project::ProjectContext;
use crate::tasks::TaskManager;
use crate::workspace::WorkspaceManager;

pub async fn handle_task_command(cmd: TaskCommands) -> Result<()> {
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

            let task = task_mgr
                .add_task(&name, spec.as_deref(), parent, Some("ai"))
                .await?; // Some("ai") = AI-created task (CLI)
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
                Some(p) => Some(crate::priority::PriorityLevel::parse_to_int(p)?),
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

        TaskCommands::List {
            status,
            parent,
            sort_by,
            limit,
            offset,
        } => {
            let ctx = ProjectContext::load().await?;
            let task_mgr = TaskManager::new(&ctx.pool);

            let parent_opt = parent.map(|p| {
                if p == "null" {
                    None
                } else {
                    p.parse::<i64>().ok()
                }
            });

            // Parse sort_by parameter
            use crate::db::models::TaskSortBy;
            let sort_by_parsed = match sort_by.as_deref() {
                Some("id") => Some(TaskSortBy::Id),
                Some("priority") => Some(TaskSortBy::Priority),
                Some("time") => Some(TaskSortBy::Time),
                Some("focus") => Some(TaskSortBy::FocusAware),
                None => None, // Use default from find_tasks
                Some(other) => {
                    return Err(IntentError::InvalidInput(format!(
                        "Invalid sort_by value '{}'. Valid values: id, priority, time, focus",
                        other
                    )));
                },
            };

            let result = task_mgr
                .find_tasks(status.as_deref(), parent_opt, sort_by_parsed, limit, offset)
                .await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
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

            let task = task_mgr.done_task(false).await?; // false = human caller (CLI)
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

            let dependency =
                crate::dependencies::add_dependency(&ctx.pool, blocking_task_id, blocked_task_id)
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
