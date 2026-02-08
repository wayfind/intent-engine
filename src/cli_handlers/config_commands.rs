use crate::cli::ConfigCommands;
use crate::error::{IntentError, Result};
use crate::project::ProjectContext;
use serde_json::json;
use sqlx::SqlitePool;

/// Protected keys that cannot be modified via config commands
const PROTECTED_KEYS: &[&str] = &["schema_version"];

/// Keys whose values should be masked in output
fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    lower.contains("api_key") || lower.contains("secret")
}

/// Mask a sensitive value for display: show first 4 chars + ********
fn mask_value(value: &str) -> String {
    if value.len() <= 4 {
        "********".to_string()
    } else {
        format!("{}...********", &value[..4])
    }
}

/// Handle all `ie config` subcommands
pub async fn handle_config_command(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::Set { key, value, format } => handle_set(&key, &value, &format).await,
        ConfigCommands::Get { key, format } => handle_get(&key, &format).await,
        ConfigCommands::List { prefix, format } => handle_list(prefix.as_deref(), &format).await,
        ConfigCommands::Unset { key, format } => handle_unset(&key, &format).await,
    }
}

async fn handle_set(key: &str, value: &str, format: &str) -> Result<()> {
    if PROTECTED_KEYS.contains(&key) {
        return Err(IntentError::ActionNotAllowed(format!(
            "Cannot modify protected key: '{}'",
            key
        )));
    }

    let ctx = ProjectContext::load_or_init().await?;
    config_set(&ctx.pool, key, value).await?;

    let display_value = if is_sensitive_key(key) {
        mask_value(value)
    } else {
        value.to_string()
    };

    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "key": key,
                "value": display_value,
                "set": true,
            }))?
        );
    } else {
        println!("Set {} = {}", key, display_value);
    }

    Ok(())
}

async fn handle_get(key: &str, format: &str) -> Result<()> {
    let ctx = ProjectContext::load_or_init().await?;
    let value = config_get(&ctx.pool, key).await?;

    match value {
        Some(v) => {
            let display_value = if is_sensitive_key(key) {
                mask_value(&v)
            } else {
                v.clone()
            };

            if format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "key": key,
                        "value": display_value,
                    }))?
                );
            } else {
                println!("{} = {}", key, display_value);
            }
        },
        None => {
            if format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "key": key,
                        "value": null,
                    }))?
                );
            } else {
                println!("{}: (not set)", key);
            }
        },
    }

    Ok(())
}

async fn handle_list(prefix: Option<&str>, format: &str) -> Result<()> {
    let ctx = ProjectContext::load_or_init().await?;
    let entries = config_list(&ctx.pool, prefix).await?;

    if format == "json" {
        let items: Vec<serde_json::Value> = entries
            .iter()
            .map(|(k, v)| {
                let display_value = if is_sensitive_key(k) {
                    mask_value(v)
                } else {
                    v.clone()
                };
                json!({ "key": k, "value": display_value })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "config": items }))?
        );
    } else {
        if entries.is_empty() {
            println!("No configuration entries found.");
        } else {
            for (key, value) in &entries {
                let display_value = if is_sensitive_key(key) {
                    mask_value(value)
                } else {
                    value.clone()
                };
                println!("{} = {}", key, display_value);
            }
        }
    }

    Ok(())
}

async fn handle_unset(key: &str, format: &str) -> Result<()> {
    if PROTECTED_KEYS.contains(&key) {
        return Err(IntentError::ActionNotAllowed(format!(
            "Cannot delete protected key: '{}'",
            key
        )));
    }

    let ctx = ProjectContext::load_or_init().await?;
    let deleted = config_delete(&ctx.pool, key).await?;

    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "key": key,
                "deleted": deleted,
            }))?
        );
    } else if deleted {
        println!("Unset {}", key);
    } else {
        println!("{}: (not found)", key);
    }

    Ok(())
}

// ============================================================================
// Database operations (reuse workspace_state table)
// ============================================================================

pub async fn config_set(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO workspace_state (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn config_get(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    let value: Option<String> =
        sqlx::query_scalar("SELECT value FROM workspace_state WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;
    Ok(value)
}

pub async fn config_list(pool: &SqlitePool, prefix: Option<&str>) -> Result<Vec<(String, String)>> {
    let rows: Vec<(String, String)> = if let Some(p) = prefix {
        let pattern = format!("{}%", p);
        sqlx::query_as(
            "SELECT key, value FROM workspace_state WHERE key != 'schema_version' AND key LIKE ? ORDER BY key",
        )
        .bind(pattern)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT key, value FROM workspace_state WHERE key != 'schema_version' ORDER BY key",
        )
        .fetch_all(pool)
        .await?
    };
    Ok(rows)
}

pub async fn config_delete(pool: &SqlitePool, key: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM workspace_state WHERE key = ?")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::TestContext;

    #[tokio::test]
    async fn test_config_set_and_get() {
        let ctx = TestContext::new().await;
        config_set(ctx.pool(), "test.key", "test_value")
            .await
            .unwrap();

        let value = config_get(ctx.pool(), "test.key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_config_get_nonexistent() {
        let ctx = TestContext::new().await;
        let value = config_get(ctx.pool(), "nonexistent").await.unwrap();
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_config_set_overwrite() {
        let ctx = TestContext::new().await;
        config_set(ctx.pool(), "test.key", "v1").await.unwrap();
        config_set(ctx.pool(), "test.key", "v2").await.unwrap();

        let value = config_get(ctx.pool(), "test.key").await.unwrap();
        assert_eq!(value, Some("v2".to_string()));
    }

    #[tokio::test]
    async fn test_config_list_empty() {
        let ctx = TestContext::new().await;
        let entries = config_list(ctx.pool(), None).await.unwrap();
        // Only schema_version should exist, but it's excluded
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_config_list_with_entries() {
        let ctx = TestContext::new().await;
        config_set(ctx.pool(), "llm.endpoint", "http://localhost:8080")
            .await
            .unwrap();
        config_set(ctx.pool(), "llm.model", "gpt-4").await.unwrap();
        config_set(ctx.pool(), "other.key", "value").await.unwrap();

        let all = config_list(ctx.pool(), None).await.unwrap();
        assert_eq!(all.len(), 3);

        let llm_only = config_list(ctx.pool(), Some("llm.")).await.unwrap();
        assert_eq!(llm_only.len(), 2);
    }

    #[tokio::test]
    async fn test_config_delete() {
        let ctx = TestContext::new().await;
        config_set(ctx.pool(), "to.delete", "value").await.unwrap();

        let deleted = config_delete(ctx.pool(), "to.delete").await.unwrap();
        assert!(deleted);

        let value = config_get(ctx.pool(), "to.delete").await.unwrap();
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_config_delete_nonexistent() {
        let ctx = TestContext::new().await;
        let deleted = config_delete(ctx.pool(), "nonexistent").await.unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_is_sensitive_key() {
        assert!(is_sensitive_key("llm.api_key"));
        assert!(is_sensitive_key("LLM.API_KEY"));
        assert!(is_sensitive_key("some.secret"));
        assert!(!is_sensitive_key("llm.endpoint"));
        assert!(!is_sensitive_key("llm.model"));
    }

    #[test]
    fn test_mask_value() {
        assert_eq!(
            mask_value("sk-65595e731935451cbac9b29b48f0c906"),
            "sk-6...********"
        );
        assert_eq!(mask_value("abc"), "********");
        assert_eq!(mask_value("abcde"), "abcd...********");
    }

    #[test]
    fn test_protected_keys() {
        assert!(PROTECTED_KEYS.contains(&"schema_version"));
    }
}
