use crate::backend::PlanBackend;
use crate::error::Result;
use crate::plan::PlanResult;

/// Format and display the result of a plan execution.
///
/// The caller is responsible for:
/// - Reading stdin and parsing JSON
/// - Processing @file directives
/// - Constructing the plan executor with appropriate project path / default parent
/// - Executing the plan via `PlanBackend::execute`
/// - Cleaning up included files
///
/// This function only handles the output formatting, which is shared across backends.
pub fn print_plan_result(result: &PlanResult, format: &str) -> Result<()> {
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else if result.success {
        println!("Plan executed successfully");
        println!();
        println!("Created: {} tasks", result.created_count);
        println!("Updated: {} tasks", result.updated_count);
        if result.deleted_count > 0 {
            println!("Deleted: {} tasks", result.deleted_count);
        }
        if result.cascade_deleted_count > 0 {
            println!("Cascade deleted: {} tasks", result.cascade_deleted_count);
        }
        println!("Dependencies: {}", result.dependency_count);
        println!();
        println!("Task ID mapping:");
        for (name, id) in &result.task_id_map {
            println!("  {} -> #{}", name, id);
        }

        if !result.warnings.is_empty() {
            println!();
            println!("Warnings:");
            for warning in &result.warnings {
                println!("  - {}", warning);
            }
        }

        if let Some(focused) = &result.focused_task {
            println!();
            println!("Current focus:");
            println!("  ID: {}", focused.task.id);
            println!("  Name: {}", focused.task.name);
            println!("  Status: {}", focused.task.status);
            if let Some(parent_id) = focused.task.parent_id {
                println!("  Parent: #{}", parent_id);
            }
            if let Some(priority) = focused.task.priority {
                println!("  Priority: {}", priority);
            }
            if let Some(spec) = &focused.task.spec {
                println!("  Spec: {}", spec);
            }
            println!("  Owner: {}", focused.task.owner);
            if let Some(ts) = focused.task.first_todo_at {
                println!("  First todo: {}", ts.format("%Y-%m-%d %H:%M:%S UTC"));
            }
            if let Some(ts) = focused.task.first_doing_at {
                println!("  First doing: {}", ts.format("%Y-%m-%d %H:%M:%S UTC"));
            }
            if let Some(ts) = focused.task.first_done_at {
                println!("  First done: {}", ts.format("%Y-%m-%d %H:%M:%S UTC"));
            }

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
                            event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                            event.discussion_data
                        );
                    }
                }
            }
        }
    } else {
        eprintln!("Plan failed");
        if let Some(error) = &result.error {
            eprintln!("Error: {}", error);
        }
        std::process::exit(1);
    }

    Ok(())
}

/// Execute a plan and print the result.
///
/// Convenience function that combines execution and formatting.
pub async fn execute_and_print(
    plan_mgr: &impl PlanBackend,
    request: &crate::plan::PlanRequest,
    format: &str,
) -> Result<PlanResult> {
    let result = plan_mgr.execute(request).await?;
    print_plan_result(&result, format)?;
    Ok(result)
}
