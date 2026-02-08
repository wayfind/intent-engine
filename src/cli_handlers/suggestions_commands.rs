use crate::error::Result;
use crate::llm::{
    clear_dismissed_suggestions, dismiss_all_suggestions, dismiss_suggestion,
    get_active_suggestions,
};
use crate::project::ProjectContext;
use serde_json::json;

pub async fn handle_list(format: &str) -> Result<()> {
    let ctx = ProjectContext::load_or_init().await?;
    let suggestions = get_active_suggestions(&ctx.pool).await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&suggestions)?);
    } else {
        if suggestions.is_empty() {
            println!("No active suggestions.");
            return Ok(());
        }

        println!("💡 Active Suggestions ({})", suggestions.len());
        println!();

        for suggestion in &suggestions {
            let type_emoji = match suggestion.suggestion_type.as_str() {
                "task_structure" => "🔄",
                "event_synthesis" => "📝",
                "error" => "⚠️",
                _ => "ℹ️",
            };

            println!(
                "{} Suggestion #{} ({})",
                type_emoji, suggestion.id, suggestion.suggestion_type
            );
            println!(
                "   Created: {}",
                suggestion.created_at.format("%Y-%m-%d %H:%M")
            );
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("{}", suggestion.content);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!();
        }

        println!("To dismiss: ie suggestions dismiss <id>");
        println!("To dismiss all: ie suggestions dismiss --all");
    }

    Ok(())
}

pub async fn handle_dismiss(id: Option<i64>, all: bool, format: &str) -> Result<()> {
    let ctx = ProjectContext::load_or_init().await?;

    let count = if all {
        dismiss_all_suggestions(&ctx.pool).await?
    } else if let Some(suggestion_id) = id {
        dismiss_suggestion(&ctx.pool, suggestion_id).await?;
        1
    } else {
        use crate::error::IntentError;
        return Err(IntentError::InvalidInput(
            "Must provide either suggestion ID or --all flag.\n\
             Usage:\n\
               ie suggestions dismiss 5\n\
               ie suggestions dismiss --all"
                .to_string(),
        ));
    };

    if format == "json" {
        println!("{}", json!({ "dismissed": count }));
    } else {
        if count == 0 {
            println!("No suggestions to dismiss.");
        } else {
            println!("✅ Dismissed {} suggestion(s)", count);
        }
    }

    Ok(())
}

pub async fn handle_clear(format: &str) -> Result<()> {
    let ctx = ProjectContext::load_or_init().await?;
    let count = clear_dismissed_suggestions(&ctx.pool).await?;

    if format == "json" {
        println!("{}", json!({ "cleared": count }));
    } else {
        if count == 0 {
            println!("No dismissed suggestions to clear.");
        } else {
            println!("🗑️  Cleared {} dismissed suggestion(s)", count);
        }
    }

    Ok(())
}
