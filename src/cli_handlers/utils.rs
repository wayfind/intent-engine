//! Utility functions for CLI handlers
//!
//! Helper functions for reading stdin, formatting status badges, and printing task contexts.

use crate::error::Result;
use crate::tasks::TaskContext;
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

/// Get a status badge icon for task status
pub fn get_status_badge(status: &str) -> &'static str {
    match status {
        "done" => "✓",
        "doing" => "→",
        "todo" => "○",
        _ => "?",
    }
}

/// Print task context in a human-friendly tree format
pub fn print_task_context(ctx: &TaskContext) -> Result<()> {
    // Print task header
    let badge = get_status_badge(&ctx.task.status);
    println!("\n{} Task #{}: {}", badge, ctx.task.id, ctx.task.name);
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
            let ancestor_badge = get_status_badge(&ancestor.status);
            println!(
                "{}└─ {} #{}: {}",
                indent, ancestor_badge, ancestor.id, ancestor.name
            );
        }
    }

    // Print children
    if !ctx.children.is_empty() {
        println!("\nChildren:");
        for child in &ctx.children {
            let child_badge = get_status_badge(&child.status);
            println!("  {} #{}: {}", child_badge, child.id, child.name);
        }
    }

    // Print siblings
    if !ctx.siblings.is_empty() {
        println!("\nSiblings:");
        for sibling in &ctx.siblings {
            let sibling_badge = get_status_badge(&sibling.status);
            println!("  {} #{}: {}", sibling_badge, sibling.id, sibling.name);
        }
    }

    // Print dependencies (blocking tasks)
    if !ctx.dependencies.blocking_tasks.is_empty() {
        println!("\nDepends on:");
        for dep in &ctx.dependencies.blocking_tasks {
            let dep_badge = get_status_badge(&dep.status);
            println!("  {} #{}: {}", dep_badge, dep.id, dep.name);
        }
    }

    // Print dependents (blocked by tasks)
    if !ctx.dependencies.blocked_by_tasks.is_empty() {
        println!("\nBlocks:");
        for dep in &ctx.dependencies.blocked_by_tasks {
            let dep_badge = get_status_badge(&dep.status);
            println!("  {} #{}: {}", dep_badge, dep.id, dep.name);
        }
    }

    println!();
    Ok(())
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
        let result = print_task_context(&ctx);
        assert!(result.is_ok());
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

        let result = print_task_context(&ctx);
        assert!(result.is_ok());
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

        let result = print_task_context(&ctx);
        assert!(result.is_ok());
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

        let result = print_task_context(&ctx);
        assert!(result.is_ok());
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

        let result = print_task_context(&ctx);
        assert!(result.is_ok());
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

        let result = print_task_context(&ctx);
        assert!(result.is_ok());
    }
}
