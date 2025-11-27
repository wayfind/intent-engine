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
        use std::io::Read;

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
}
