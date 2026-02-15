//! Utility functions for CLI handlers
//!
//! Helper functions for reading stdin, formatting status badges, and printing task contexts.

use crate::db::models::{EventsSummary, Task, TaskContext};
use crate::error::Result;
use std::io::{self, Read};

/// Read from stdin with proper encoding handling (especially for Windows PowerShell)
pub fn read_stdin() -> Result<String> {
    #[cfg(windows)]
    {
        use encoding_rs::GBK;

        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)?;

        // First try UTF-8
        if let Ok(s) = String::from_utf8(buffer.clone()) {
            return Ok(s.trim().to_string());
        }

        // Fall back to GBK decoding (common in Chinese Windows PowerShell)
        let (decoded, _, had_errors) = GBK.decode(&buffer);
        if !had_errors {
            tracing::debug!(
                "Successfully decoded stdin from GBK encoding (Chinese Windows detected)"
            );
            Ok(decoded.trim().to_string())
        } else {
            // If GBK also fails, return the UTF-8 lossy version
            tracing::warn!(
                "Failed to decode stdin from both UTF-8 and GBK, using lossy UTF-8 conversion"
            );
            Ok(String::from_utf8_lossy(&buffer).trim().to_string())
        }
    }

    #[cfg(not(windows))]
    {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer.trim().to_string())
    }
}

/// Get a status badge icon for task status (arrow style, used in `ie status`)
pub fn get_status_badge(status: &str) -> &'static str {
    match status {
        "done" => "✓",
        "doing" => "→",
        "todo" => "○",
        _ => "?",
    }
}

/// Get a status icon for task status (bullet style, used in tree/list views)
pub fn status_icon(status: &str) -> &'static str {
    match status {
        "todo" => "○",
        "doing" => "●",
        "done" => "✓",
        _ => "?",
    }
}

/// Print tasks in a hierarchical tree format
pub fn print_task_tree(tasks: &[crate::db::models::Task]) {
    use std::collections::HashMap;

    // Build parent -> children map
    let mut children_map: HashMap<Option<i64>, Vec<&crate::db::models::Task>> = HashMap::new();
    for task in tasks {
        children_map.entry(task.parent_id).or_default().push(task);
    }

    fn print_subtree(
        children_map: &HashMap<Option<i64>, Vec<&crate::db::models::Task>>,
        parent_id: Option<i64>,
        indent: &str,
        _is_last: bool,
    ) {
        if let Some(children) = children_map.get(&parent_id) {
            for (i, task) in children.iter().enumerate() {
                let is_last_child = i == children.len() - 1;
                let connector = if indent.is_empty() {
                    ""
                } else if is_last_child {
                    "└─ "
                } else {
                    "├─ "
                };
                let icon = status_icon(&task.status);
                let priority_info = task
                    .priority
                    .map(|p| format!(" [P{}]", p))
                    .unwrap_or_default();

                println!(
                    "  {}{}{} #{} {}{}",
                    indent, connector, icon, task.id, task.name, priority_info
                );

                let new_indent = if indent.is_empty() {
                    "".to_string()
                } else if is_last_child {
                    format!("{}   ", indent)
                } else {
                    format!("{}│  ", indent)
                };
                print_subtree(children_map, Some(task.id), &new_indent, is_last_child);
            }
        }
    }

    // Start with root-level tasks (parent is None or parent not in our set)
    let task_ids: std::collections::HashSet<i64> = tasks.iter().map(|t| t.id).collect();
    let roots: Vec<&crate::db::models::Task> = tasks
        .iter()
        .filter(|t| t.parent_id.is_none() || !task_ids.contains(&t.parent_id.unwrap_or(-1)))
        .collect();

    for (i, task) in roots.iter().enumerate() {
        let _is_last = i == roots.len() - 1;
        let icon = status_icon(&task.status);
        let priority_info = task
            .priority
            .map(|p| format!(" [P{}]", p))
            .unwrap_or_default();
        println!("  {} #{} {}{}", icon, task.id, task.name, priority_info);
        print_subtree(&children_map, Some(task.id), "  ", _is_last);
    }
}

/// Print a concise task summary
pub fn print_task_summary(task: &Task) {
    let icon = status_icon(&task.status);
    println!("  {} #{} {}", icon, task.id, task.name);
    println!("  Status: {}", task.status);
    if let Some(pid) = task.parent_id {
        println!("  Parent: #{}", pid);
    }
    if let Some(p) = task.priority {
        println!("  Priority: {}", p);
    }
    if let Some(spec) = &task.spec {
        if !spec.is_empty() {
            println!("  Spec: {}", spec);
        }
    }
    println!("  Owner: {}", task.owner);
    if let Some(af) = &task.active_form {
        println!("  Active form: {}", af);
    }
    if let Some(meta) = &task.metadata {
        println!("  Metadata: {}", meta);
    }
}

/// Print task context in a human-friendly tree format
pub fn print_task_context(ctx: &TaskContext) {
    let icon = status_icon(&ctx.task.status);
    println!("\n{} Task #{}: {}", icon, ctx.task.id, ctx.task.name);
    println!("Status: {}", ctx.task.status);

    if let Some(spec) = &ctx.task.spec {
        println!("\nSpec:");
        for line in spec.lines() {
            println!("  {}", line);
        }
    }

    // Print parent chain
    if !ctx.ancestors.is_empty() {
        println!("\nParent Chain:");
        for (i, ancestor) in ctx.ancestors.iter().enumerate() {
            let indent = "  ".repeat(i + 1);
            println!(
                "{}└─ {} #{}: {}",
                indent,
                status_icon(&ancestor.status),
                ancestor.id,
                ancestor.name
            );
        }
    }

    // Print children
    if !ctx.children.is_empty() {
        println!("\nChildren:");
        for child in &ctx.children {
            println!(
                "  {} #{}: {}",
                status_icon(&child.status),
                child.id,
                child.name
            );
        }
    }

    // Print siblings
    if !ctx.siblings.is_empty() {
        println!("\nSiblings:");
        for sibling in &ctx.siblings {
            println!(
                "  {} #{}: {}",
                status_icon(&sibling.status),
                sibling.id,
                sibling.name
            );
        }
    }

    // Print dependencies (blocking tasks)
    if !ctx.dependencies.blocking_tasks.is_empty() {
        println!("\nDepends on:");
        for dep in &ctx.dependencies.blocking_tasks {
            println!("  {} #{}: {}", status_icon(&dep.status), dep.id, dep.name);
        }
    }

    // Print dependents (blocked by tasks)
    if !ctx.dependencies.blocked_by_tasks.is_empty() {
        println!("\nBlocks:");
        for dep in &ctx.dependencies.blocked_by_tasks {
            println!("  {} #{}: {}", status_icon(&dep.status), dep.id, dep.name);
        }
    }

    println!();
}

/// Print events summary (recent events with count)
pub fn print_events_summary(summary: &EventsSummary) {
    println!("Events ({}):", summary.total_count);
    for event in summary.recent_events.iter().take(10) {
        println!(
            "  [{}] {} — {}",
            event.log_type,
            event.timestamp.format("%Y-%m-%d %H:%M:%S"),
            event.discussion_data
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{Task, TaskContext, TaskDependencies};

    // Helper function to create a test task with minimal boilerplate
    fn create_test_task(id: i64, name: &str, status: &str, parent_id: Option<i64>) -> Task {
        Task {
            id,
            name: name.to_string(),
            status: status.to_string(),
            spec: None,
            parent_id,
            priority: Some(5),
            complexity: None,
            first_todo_at: None,
            first_doing_at: None,
            first_done_at: None,
            active_form: None,
            owner: "human".to_string(),
            metadata: None,
        }
    }

    #[test]
    fn test_get_status_badge_done() {
        assert_eq!(get_status_badge("done"), "✓");
    }

    #[test]
    fn test_get_status_badge_doing() {
        assert_eq!(get_status_badge("doing"), "→");
    }

    #[test]
    fn test_get_status_badge_todo() {
        assert_eq!(get_status_badge("todo"), "○");
    }

    #[test]
    fn test_get_status_badge_unknown() {
        assert_eq!(get_status_badge("unknown"), "?");
        assert_eq!(get_status_badge(""), "?");
        assert_eq!(get_status_badge("invalid"), "?");
    }

    #[test]
    fn test_status_icon() {
        assert_eq!(status_icon("todo"), "○");
        assert_eq!(status_icon("doing"), "●");
        assert_eq!(status_icon("done"), "✓");
        assert_eq!(status_icon("unknown"), "?");
    }

    #[test]
    fn test_print_task_context_basic() {
        let task = create_test_task(1, "Test Task", "todo", None);

        let ctx = TaskContext {
            task,
            ancestors: vec![],
            children: vec![],
            siblings: vec![],
            dependencies: TaskDependencies {
                blocking_tasks: vec![],
                blocked_by_tasks: vec![],
            },
        };

        // Should not panic and should execute all branches
        print_task_context(&ctx); // should not panic
    }

    #[test]
    fn test_print_task_context_with_spec() {
        let mut task = create_test_task(2, "Task with Spec", "doing", None);
        task.spec = Some("This is a\nmulti-line\nspecification".to_string());

        let ctx = TaskContext {
            task,
            ancestors: vec![],
            children: vec![],
            siblings: vec![],
            dependencies: TaskDependencies {
                blocking_tasks: vec![],
                blocked_by_tasks: vec![],
            },
        };

        print_task_context(&ctx); // should not panic
    }

    #[test]
    fn test_print_task_context_with_children() {
        let task = create_test_task(3, "Parent Task", "doing", None);
        let child1 = create_test_task(4, "Child Task 1", "todo", Some(3));
        let child2 = create_test_task(5, "Child Task 2", "done", Some(3));

        let ctx = TaskContext {
            task,
            ancestors: vec![],
            children: vec![child1, child2],
            siblings: vec![],
            dependencies: TaskDependencies {
                blocking_tasks: vec![],
                blocked_by_tasks: vec![],
            },
        };

        print_task_context(&ctx); // should not panic
    }

    #[test]
    fn test_print_task_context_with_ancestors() {
        let task = create_test_task(6, "Nested Task", "doing", Some(7));
        let parent = create_test_task(7, "Parent Task", "doing", None);

        let ctx = TaskContext {
            task,
            ancestors: vec![parent],
            children: vec![],
            siblings: vec![],
            dependencies: TaskDependencies {
                blocking_tasks: vec![],
                blocked_by_tasks: vec![],
            },
        };

        print_task_context(&ctx); // should not panic
    }

    #[test]
    fn test_print_task_context_with_dependencies() {
        let task = create_test_task(8, "Task with Dependencies", "todo", None);
        let blocker = create_test_task(9, "Blocking Task", "doing", None);
        let blocked = create_test_task(10, "Blocked Task", "todo", None);

        let ctx = TaskContext {
            task,
            ancestors: vec![],
            children: vec![],
            siblings: vec![],
            dependencies: TaskDependencies {
                blocking_tasks: vec![blocker],
                blocked_by_tasks: vec![blocked],
            },
        };

        print_task_context(&ctx); // should not panic
    }

    #[test]
    fn test_print_task_context_with_siblings() {
        let task = create_test_task(11, "Task with Siblings", "doing", Some(12));
        let sibling = create_test_task(13, "Sibling Task", "todo", Some(12));

        let ctx = TaskContext {
            task,
            ancestors: vec![],
            children: vec![],
            siblings: vec![sibling],
            dependencies: TaskDependencies {
                blocking_tasks: vec![],
                blocked_by_tasks: vec![],
            },
        };

        print_task_context(&ctx); // should not panic
    }
}
