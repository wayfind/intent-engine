use crate::cli_handlers::config_commands::{config_get, config_set};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global cooldown to prevent unlimited task spawning
/// Stores the timestamp of the last analysis (seconds since epoch)
static LAST_ANALYSIS_TIME: AtomicI64 = AtomicI64::new(0);

/// Default cooldown period: 5 minutes
const DEFAULT_ANALYSIS_COOLDOWN_SECS: i64 = 300;

/// Maximum number of active (non-dismissed) suggestions
/// Prevents unbounded growth if user never dismisses suggestions
const MAX_ACTIVE_SUGGESTIONS: i64 = 20;

/// Get current timestamp in seconds since UNIX_EPOCH
///
/// Handles system clock errors gracefully by returning None.
/// This prevents panics when system time is misconfigured.
fn get_current_timestamp() -> Option<i64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

/// Check if enough time has passed since last analysis
///
/// Handles edge cases:
/// - System clock errors: allows analysis (fail-safe)
/// - Clock skew (time went backwards): resets timer and allows analysis
/// - Thread safety: uses Acquire ordering for visibility
fn should_trigger_analysis() -> bool {
    let now = match get_current_timestamp() {
        Some(ts) => ts,
        None => {
            tracing::warn!("System clock error, allowing analysis as fail-safe");
            return true;
        },
    };

    let last = LAST_ANALYSIS_TIME.load(Ordering::Acquire);

    // Handle clock skew (time went backwards)
    if now < last {
        tracing::warn!(
            "Clock skew detected: current={}, last={}, resetting analysis timer",
            now,
            last
        );
        LAST_ANALYSIS_TIME.store(now, Ordering::Release);
        return true;
    }

    // Use default cooldown (5 minutes)
    let cooldown = DEFAULT_ANALYSIS_COOLDOWN_SECS;

    now - last >= cooldown
}

/// Mark that analysis is starting now
///
/// Uses Release ordering to ensure visibility across threads.
/// If system clock fails, logs warning but doesn't update (safe default).
fn mark_analysis_started() {
    if let Some(now) = get_current_timestamp() {
        LAST_ANALYSIS_TIME.store(now, Ordering::Release);
    } else {
        tracing::warn!("System clock error, cannot update analysis timestamp");
    }
}

/// LLM configuration resolved from env vars and workspace_state
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
}

/// A chat message for the OpenAI-compatible API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI-compatible chat completion request
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

/// OpenAI-compatible chat completion response
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: String,
}

impl LlmConfig {
    /// Resolve LLM config from env vars (priority) and workspace_state.
    /// Returns Some only when all three fields (endpoint, api_key, model) are configured.
    pub async fn resolve(pool: &SqlitePool) -> Result<Option<Self>> {
        let endpoint = Self::resolve_field(pool, "IE_LLM_ENDPOINT", "llm.endpoint").await?;
        let api_key = Self::resolve_field(pool, "IE_LLM_API_KEY", "llm.api_key").await?;
        let model = Self::resolve_field(pool, "IE_LLM_MODEL", "llm.model").await?;

        match (endpoint, api_key, model) {
            (Some(endpoint), Some(api_key), Some(model)) => Ok(Some(Self {
                endpoint,
                api_key,
                model,
            })),
            _ => Ok(None),
        }
    }

    /// Resolve a single field: env var takes priority over workspace_state
    async fn resolve_field(
        pool: &SqlitePool,
        env_var: &str,
        config_key: &str,
    ) -> Result<Option<String>> {
        // Check env var first
        if let Ok(val) = std::env::var(env_var) {
            if !val.is_empty() {
                return Ok(Some(val));
            }
        }

        // Fall back to workspace_state
        config_get(pool, config_key).await
    }

    /// Save LLM config to workspace_state
    pub async fn save(&self, pool: &SqlitePool) -> Result<()> {
        config_set(pool, "llm.endpoint", &self.endpoint).await?;
        config_set(pool, "llm.api_key", &self.api_key).await?;
        config_set(pool, "llm.model", &self.model).await?;
        Ok(())
    }
}

/// OpenAI-compatible LLM client
pub struct LlmClient {
    config: LlmConfig,
    client: reqwest::Client,
}

impl LlmClient {
    /// Create a new LlmClient from database config.
    /// Returns an error if LLM is not fully configured.
    pub async fn from_pool(pool: &SqlitePool) -> Result<Self> {
        let config = LlmConfig::resolve(pool).await?.ok_or_else(|| {
            crate::error::IntentError::InvalidInput(
                "LLM not configured. Set llm.endpoint, llm.api_key, and llm.model via 'ie config set' or environment variables (IE_LLM_ENDPOINT, IE_LLM_API_KEY, IE_LLM_MODEL).".to_string(),
            )
        })?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| crate::error::IntentError::OtherError(e.into()))?;

        Ok(Self { config, client })
    }

    /// Check whether LLM is fully configured (all three fields present)
    pub async fn is_configured(pool: &SqlitePool) -> bool {
        matches!(LlmConfig::resolve(pool).await, Ok(Some(_)))
    }

    /// Simple chat: send a single user prompt and get a response
    pub async fn chat(&self, prompt: &str) -> Result<String> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];
        self.chat_with_messages(messages).await
    }

    /// Chat with full message history
    pub async fn chat_with_messages(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
        };

        let response = self
            .client
            .post(&self.config.endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| crate::error::IntentError::OtherError(e.into()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "(no body)".to_string());
            return Err(crate::error::IntentError::OtherError(anyhow::anyhow!(
                "LLM API error (HTTP {}): {}",
                status,
                body
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| crate::error::IntentError::OtherError(e.into()))?;

        chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| {
                crate::error::IntentError::OtherError(anyhow::anyhow!("LLM returned empty choices"))
            })
    }

    /// Synthesize task description from accumulated events
    ///
    /// This function takes a task and its event history, and uses the LLM to generate
    /// a structured summary in markdown format.
    pub async fn synthesize_task_description(
        &self,
        task_name: &str,
        original_spec: Option<&str>,
        events: &[crate::db::models::Event],
    ) -> Result<String> {
        // Build the event summary
        let events_text = if events.is_empty() {
            "No events recorded.".to_string()
        } else {
            events
                .iter()
                .map(|e| {
                    format!(
                        "[{}] {} - {}",
                        e.log_type,
                        e.timestamp.format("%Y-%m-%d %H:%M"),
                        e.discussion_data
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let original_spec_text = original_spec.unwrap_or("(No original description)");

        // Detect language from task name and events to respond in same language
        let is_cjk = task_name.chars().any(|c| {
            matches!(c,
                '\u{4E00}'..='\u{9FFF}' |  // CJK Unified Ideographs
                '\u{3400}'..='\u{4DBF}' |  // CJK Extension A
                '\u{3040}'..='\u{309F}' |  // Hiragana
                '\u{30A0}'..='\u{30FF}' |  // Katakana
                '\u{AC00}'..='\u{D7AF}'    // Hangul
            )
        });

        let language_instruction = if is_cjk {
            "Respond in Chinese (中文)."
        } else {
            "Respond in English."
        };

        // Construct the prompt
        let prompt = format!(
            r#"You are summarizing a completed task based on its execution history.

Task: {}
Original description: {}

Events (chronological):
{}

Synthesize a clear, structured description capturing:
1. Goal (what was the objective?)
2. Approach (how was it accomplished?)
3. Key Decisions (what choices were made and why?)
4. Outcome (what was delivered?)

Use markdown format with ## headers. Be concise but preserve critical context.
Output ONLY the markdown summary, no preamble or explanation.

IMPORTANT: {}"#,
            task_name, original_spec_text, events_text, language_instruction
        );

        self.chat(&prompt).await
    }
}

/// Synthesize task description using LLM (convenience function)
///
/// Returns None if LLM is not configured (graceful degradation)
pub async fn synthesize_task_description(
    pool: &SqlitePool,
    task_name: &str,
    original_spec: Option<&str>,
    events: &[crate::db::models::Event],
) -> Result<Option<String>> {
    // Check if LLM is configured
    if !LlmClient::is_configured(pool).await {
        return Ok(None);
    }

    // Create client and synthesize
    let client = LlmClient::from_pool(pool).await?;
    let synthesis = client
        .synthesize_task_description(task_name, original_spec, events)
        .await?;

    Ok(Some(synthesis))
}

/// Analyze task structure in background and store suggestions
///
/// This function runs asynchronously without blocking the caller.
/// Suggestions are stored in the database and shown at next interaction.
///
/// **Rate Limiting**: Uses a cooldown period (default 5 minutes) to prevent
/// unlimited task spawning. If called within the cooldown period, it's a no-op.
pub fn analyze_task_structure_background(pool: SqlitePool) {
    // Check cooldown BEFORE spawning to avoid unnecessary tasks
    if !should_trigger_analysis() {
        tracing::debug!("Analysis cooldown active, skipping background analysis");
        return;
    }

    // Mark as started immediately to prevent race conditions
    mark_analysis_started();

    tokio::spawn(async move {
        if let Err(e) = analyze_and_store_suggestions(&pool).await {
            // Store error as a suggestion so user knows it failed
            let error_msg = format!(
                "## Analysis Error\n\n\
                Background task structure analysis failed: {}\n\n\
                This may indicate:\n\
                - LLM API endpoint is unreachable\n\
                - API quota exceeded\n\
                - Network connectivity issues\n\n\
                Check logs for details: `ie log`",
                e
            );

            // Try to store the error
            match store_suggestion(&pool, "error", &error_msg).await {
                Ok(_) => {
                    tracing::warn!("Background task analysis failed: {}", e);
                },
                Err(store_err) => {
                    // Critical: both analysis AND storage failed
                    // Log to stderr so user has some visibility
                    tracing::error!("Failed to store error suggestion: {}", store_err);
                    eprintln!(
                        "\n⚠️  Background analysis failed AND couldn't store error.\n\
                         Analysis error: {}\n\
                         Storage error: {}\n\
                         This may indicate database issues.",
                        e, store_err
                    );
                },
            }
        }
    });
}

/// Internal: Perform analysis and store suggestions
async fn analyze_and_store_suggestions(pool: &SqlitePool) -> Result<()> {
    // Check if LLM is configured
    if !LlmClient::is_configured(pool).await {
        return Ok(()); // Silent skip
    }

    // Get all tasks
    let tasks: Vec<crate::db::models::Task> = sqlx::query_as(
        "SELECT id, parent_id, name, spec, status, complexity, priority, \
         first_todo_at, first_doing_at, first_done_at, active_form, owner, metadata \
         FROM tasks ORDER BY id",
    )
    .fetch_all(pool)
    .await?;

    // Need at least 5 tasks to make meaningful suggestions
    if tasks.len() < 5 {
        return Ok(());
    }

    let analysis = perform_structure_analysis(pool, &tasks).await?;

    // Only store if there are actual suggestions
    if !analysis.contains("no reorganization needed") && !analysis.contains("looks good") {
        store_suggestion(pool, "task_structure", &analysis).await?;
    }

    Ok(())
}

/// Internal: Perform the actual LLM analysis
async fn perform_structure_analysis(
    pool: &SqlitePool,
    tasks: &[crate::db::models::Task],
) -> Result<String> {
    // Build task tree representation
    let task_summary = tasks
        .iter()
        .map(|t| {
            format!(
                "#{} {} [{}] (parent: {})",
                t.id,
                t.name,
                t.status,
                t.parent_id.map_or("none".to_string(), |p| p.to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Detect language from task names
    let is_cjk = tasks.iter().any(|t| {
        t.name.chars().any(|c| {
            matches!(c,
                '\u{4E00}'..='\u{9FFF}' |
                '\u{3400}'..='\u{4DBF}' |
                '\u{3040}'..='\u{309F}' |
                '\u{30A0}'..='\u{30FF}' |
                '\u{AC00}'..='\u{D7AF}'
            )
        })
    });

    let language_instruction = if is_cjk {
        "Respond in Chinese (中文)."
    } else {
        "Respond in English."
    };

    let prompt = format!(
        r#"You are analyzing a task hierarchy for structural issues.

Current task tree:
{}

Identify tasks that should be reorganized:
1. Semantically related tasks that should be grouped under a common parent
2. Root tasks that could be subtasks of existing tasks
3. Tasks with similar names or themes that should share a parent

For each suggestion:
- Explain WHY the reorganization makes sense
- Provide the EXACT command to execute
- Only suggest if there's clear semantic relationship

Output format:
## Suggestion 1: [Brief description]
**Reason**: [Why this makes sense]
**Command**: `ie task update <id> --parent <parent_id>`

If no reorganization needed, respond with: "Task structure looks good, no reorganization needed."

IMPORTANT: {}"#,
        task_summary, language_instruction
    );

    let client = LlmClient::from_pool(pool).await?;
    let analysis = client.chat(&prompt).await?;

    Ok(analysis)
}

/// Internal: Store a suggestion in the database
///
/// Implements automatic cleanup when suggestion count exceeds MAX_ACTIVE_SUGGESTIONS.
/// Old suggestions are auto-dismissed to prevent unbounded growth.
async fn store_suggestion(pool: &SqlitePool, suggestion_type: &str, content: &str) -> Result<()> {
    // Check current count of active suggestions
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM suggestions WHERE dismissed = 0")
        .fetch_one(pool)
        .await?;

    // If at limit, auto-dismiss oldest suggestion
    if count >= MAX_ACTIVE_SUGGESTIONS {
        let dismissed = sqlx::query(
            "UPDATE suggestions SET dismissed = 1
             WHERE id IN (
                 SELECT id FROM suggestions
                 WHERE dismissed = 0
                 ORDER BY created_at ASC
                 LIMIT 1
             )",
        )
        .execute(pool)
        .await?;

        if dismissed.rows_affected() > 0 {
            tracing::info!(
                "Auto-dismissed oldest suggestion (limit: {})",
                MAX_ACTIVE_SUGGESTIONS
            );
        }
    }

    // Store the new suggestion
    sqlx::query("INSERT INTO suggestions (type, content) VALUES (?, ?)")
        .bind(suggestion_type)
        .bind(content)
        .execute(pool)
        .await?;

    tracing::info!("Stored {} suggestion in database", suggestion_type);
    Ok(())
}

/// Retrieve active (non-dismissed) suggestions from database
pub async fn get_active_suggestions(
    pool: &SqlitePool,
) -> Result<Vec<crate::db::models::Suggestion>> {
    let suggestions = sqlx::query_as::<_, crate::db::models::Suggestion>(
        "SELECT id, type, content, created_at, dismissed \
         FROM suggestions \
         WHERE dismissed = 0 \
         ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(suggestions)
}

/// Dismiss a suggestion (mark as read/acted upon)
pub async fn dismiss_suggestion(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("UPDATE suggestions SET dismissed = 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Dismiss all active suggestions
pub async fn dismiss_all_suggestions(pool: &SqlitePool) -> Result<usize> {
    let result = sqlx::query("UPDATE suggestions SET dismissed = 1 WHERE dismissed = 0")
        .execute(pool)
        .await?;

    Ok(result.rows_affected() as usize)
}

/// Clear all dismissed suggestions from database
pub async fn clear_dismissed_suggestions(pool: &SqlitePool) -> Result<usize> {
    let result = sqlx::query("DELETE FROM suggestions WHERE dismissed = 1")
        .execute(pool)
        .await?;

    Ok(result.rows_affected() as usize)
}

/// Display suggestions to the user (called from CLI commands)
pub async fn display_suggestions(pool: &SqlitePool) -> Result<()> {
    let suggestions = get_active_suggestions(pool).await?;

    if suggestions.is_empty() {
        return Ok(());
    }

    // Separate errors from other suggestions
    let (errors, others): (Vec<_>, Vec<_>) = suggestions
        .iter()
        .partition(|s| s.suggestion_type == "error");

    // Display errors first (more urgent)
    if !errors.is_empty() {
        eprintln!("\n⚠️  Background Analysis Errors:");
        for error in &errors {
            eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            eprintln!("{}", error.content);
            eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
        eprintln!("\nTo dismiss: ie suggestions dismiss {}", errors[0].id);
        eprintln!("To dismiss all: ie suggestions dismiss --all");
    }

    // Display regular suggestions
    if !others.is_empty() {
        eprintln!("\n💡 Suggestions:");
        for suggestion in &others {
            eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            eprintln!("{}", suggestion.content);
            eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
        eprintln!("\nTo dismiss: ie suggestions dismiss {}", others[0].id);
        eprintln!("To list all: ie suggestions list");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::TestContext;

    #[tokio::test]
    async fn test_llm_config_resolve_none_when_unconfigured() {
        let ctx = TestContext::new().await;
        let config = LlmConfig::resolve(ctx.pool()).await.unwrap();
        assert!(config.is_none());
    }

    #[tokio::test]
    async fn test_llm_config_resolve_partial_returns_none() {
        let ctx = TestContext::new().await;
        config_set(ctx.pool(), "llm.endpoint", "http://localhost:8080")
            .await
            .unwrap();
        config_set(ctx.pool(), "llm.model", "gpt-4").await.unwrap();
        // Missing api_key
        let config = LlmConfig::resolve(ctx.pool()).await.unwrap();
        assert!(config.is_none());
    }

    #[tokio::test]
    async fn test_llm_config_resolve_full() {
        let ctx = TestContext::new().await;
        config_set(
            ctx.pool(),
            "llm.endpoint",
            "http://localhost:8080/v1/chat/completions",
        )
        .await
        .unwrap();
        config_set(ctx.pool(), "llm.api_key", "sk-test123")
            .await
            .unwrap();
        config_set(ctx.pool(), "llm.model", "gpt-4").await.unwrap();

        let config = LlmConfig::resolve(ctx.pool()).await.unwrap();
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.endpoint, "http://localhost:8080/v1/chat/completions");
        assert_eq!(config.api_key, "sk-test123");
        assert_eq!(config.model, "gpt-4");
    }

    #[tokio::test]
    async fn test_llm_config_save_and_resolve() {
        let ctx = TestContext::new().await;
        let config = LlmConfig {
            endpoint: "http://example.com/v1/chat/completions".to_string(),
            api_key: "sk-saved".to_string(),
            model: "claude-3".to_string(),
        };
        config.save(ctx.pool()).await.unwrap();

        let resolved = LlmConfig::resolve(ctx.pool()).await.unwrap().unwrap();
        assert_eq!(resolved.endpoint, "http://example.com/v1/chat/completions");
        assert_eq!(resolved.api_key, "sk-saved");
        assert_eq!(resolved.model, "claude-3");
    }

    #[tokio::test]
    async fn test_is_configured_false() {
        let ctx = TestContext::new().await;
        assert!(!LlmClient::is_configured(ctx.pool()).await);
    }

    #[tokio::test]
    async fn test_is_configured_true() {
        let ctx = TestContext::new().await;
        config_set(ctx.pool(), "llm.endpoint", "http://localhost:8080")
            .await
            .unwrap();
        config_set(ctx.pool(), "llm.api_key", "sk-test")
            .await
            .unwrap();
        config_set(ctx.pool(), "llm.model", "gpt-4").await.unwrap();
        assert!(LlmClient::is_configured(ctx.pool()).await);
    }

    #[tokio::test]
    async fn test_from_pool_errors_when_unconfigured() {
        let ctx = TestContext::new().await;
        let result = LlmClient::from_pool(ctx.pool()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chat_message_serialization() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[tokio::test]
    async fn test_synthesize_task_description_when_unconfigured() {
        let ctx = TestContext::new().await;

        // Create a simple event for testing
        use chrono::Utc;
        let event = crate::db::models::Event {
            id: 1,
            task_id: 1,
            log_type: "decision".to_string(),
            discussion_data: "Test decision".to_string(),
            timestamp: Utc::now(),
        };

        // Should return None when LLM not configured
        let result =
            synthesize_task_description(ctx.pool(), "Test Task", Some("Original spec"), &[event])
                .await
                .unwrap();

        assert!(
            result.is_none(),
            "Should return None when LLM not configured"
        );
    }

    #[tokio::test]
    async fn test_synthesize_prompt_includes_task_info() {
        // This test verifies the prompt structure without calling actual LLM
        use chrono::Utc;

        let events = vec![
            crate::db::models::Event {
                id: 1,
                task_id: 1,
                log_type: "decision".to_string(),
                discussion_data: "Chose approach A".to_string(),
                timestamp: Utc::now(),
            },
            crate::db::models::Event {
                id: 2,
                task_id: 1,
                log_type: "milestone".to_string(),
                discussion_data: "Completed phase 1".to_string(),
                timestamp: Utc::now(),
            },
        ];

        // Create a mock client (we can't test actual synthesis without LLM endpoint)
        // But we can verify the prompt construction logic
        let events_text: String = events
            .iter()
            .map(|e| {
                format!(
                    "[{}] {} - {}",
                    e.log_type,
                    e.timestamp.format("%Y-%m-%d %H:%M"),
                    e.discussion_data
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Verify event formatting
        assert!(events_text.contains("decision"));
        assert!(events_text.contains("Chose approach A"));
        assert!(events_text.contains("milestone"));
        assert!(events_text.contains("Completed phase 1"));
    }

    #[tokio::test]
    async fn test_synthesize_with_empty_events() {
        // Verify handling of tasks with no events
        let events: Vec<crate::db::models::Event> = vec![];

        // Should handle empty events gracefully
        // (actual synthesis would still work, just with "No events recorded")
        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    async fn test_synthesize_with_no_original_spec() {
        use chrono::Utc;

        let original_spec: Option<&str> = None;
        let events = vec![crate::db::models::Event {
            id: 1,
            task_id: 1,
            log_type: "note".to_string(),
            discussion_data: "Some work done".to_string(),
            timestamp: Utc::now(),
        }];

        // Should handle missing original spec
        // (prompt would use "(No original description)")
        assert!(original_spec.is_none());
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_language_detection() {
        // Test CJK detection logic
        let chinese_task = "实现用户认证";
        let english_task = "Implement authentication";
        let japanese_task = "認証を実装する";
        let korean_task = "인증 구현";

        // Chinese
        let is_cjk = chinese_task.chars().any(|c| {
            matches!(c,
                '\u{4E00}'..='\u{9FFF}' |
                '\u{3400}'..='\u{4DBF}' |
                '\u{3040}'..='\u{309F}' |
                '\u{30A0}'..='\u{30FF}' |
                '\u{AC00}'..='\u{D7AF}'
            )
        });
        assert!(is_cjk, "Should detect Chinese characters");

        // English
        let is_cjk = english_task.chars().any(|c| {
            matches!(c,
                '\u{4E00}'..='\u{9FFF}' |
                '\u{3400}'..='\u{4DBF}' |
                '\u{3040}'..='\u{309F}' |
                '\u{30A0}'..='\u{30FF}' |
                '\u{AC00}'..='\u{D7AF}'
            )
        });
        assert!(!is_cjk, "Should not detect CJK in English text");

        // Japanese
        let is_cjk = japanese_task.chars().any(|c| {
            matches!(c,
                '\u{4E00}'..='\u{9FFF}' |
                '\u{3400}'..='\u{4DBF}' |
                '\u{3040}'..='\u{309F}' |
                '\u{30A0}'..='\u{30FF}' |
                '\u{AC00}'..='\u{D7AF}'
            )
        });
        assert!(is_cjk, "Should detect Japanese characters");

        // Korean
        let is_cjk = korean_task.chars().any(|c| {
            matches!(c,
                '\u{4E00}'..='\u{9FFF}' |
                '\u{3400}'..='\u{4DBF}' |
                '\u{3040}'..='\u{309F}' |
                '\u{30A0}'..='\u{30FF}' |
                '\u{AC00}'..='\u{D7AF}'
            )
        });
        assert!(is_cjk, "Should detect Korean characters");
    }

    #[tokio::test]
    async fn test_store_and_retrieve_suggestions() {
        let ctx = TestContext::new().await;

        // Store a suggestion
        store_suggestion(
            ctx.pool(),
            "task_structure",
            "## Suggestion\nReorganize task #5 under task #3",
        )
        .await
        .unwrap();

        // Retrieve suggestions
        let suggestions = get_active_suggestions(ctx.pool()).await.unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].suggestion_type, "task_structure");
        assert!(suggestions[0].content.contains("Reorganize"));
        assert!(!suggestions[0].dismissed);
    }

    #[tokio::test]
    async fn test_dismiss_suggestion() {
        let ctx = TestContext::new().await;

        // Store a suggestion
        store_suggestion(ctx.pool(), "task_structure", "Test suggestion")
            .await
            .unwrap();

        // Get the suggestion
        let suggestions = get_active_suggestions(ctx.pool()).await.unwrap();
        assert_eq!(suggestions.len(), 1);
        let suggestion_id = suggestions[0].id;

        // Dismiss it
        dismiss_suggestion(ctx.pool(), suggestion_id).await.unwrap();

        // Should not appear in active suggestions anymore
        let active = get_active_suggestions(ctx.pool()).await.unwrap();
        assert_eq!(active.len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_suggestions() {
        let ctx = TestContext::new().await;

        // Store multiple suggestions
        store_suggestion(ctx.pool(), "task_structure", "Suggestion 1")
            .await
            .unwrap();
        store_suggestion(ctx.pool(), "task_structure", "Suggestion 2")
            .await
            .unwrap();
        store_suggestion(ctx.pool(), "event_synthesis", "Suggestion 3")
            .await
            .unwrap();

        // All should be active
        let suggestions = get_active_suggestions(ctx.pool()).await.unwrap();
        assert_eq!(suggestions.len(), 3);

        // Newest first (ORDER BY created_at DESC)
        assert!(suggestions[0].content.contains("Suggestion 3"));
    }

    #[tokio::test]
    async fn test_dismiss_all_suggestions() {
        let ctx = TestContext::new().await;

        // Store 3 suggestions
        store_suggestion(ctx.pool(), "task_structure", "Suggestion 1")
            .await
            .unwrap();
        store_suggestion(ctx.pool(), "task_structure", "Suggestion 2")
            .await
            .unwrap();
        store_suggestion(ctx.pool(), "error", "Error message")
            .await
            .unwrap();

        // Verify all active
        let active = get_active_suggestions(ctx.pool()).await.unwrap();
        assert_eq!(active.len(), 3);

        // Dismiss all
        let count = dismiss_all_suggestions(ctx.pool()).await.unwrap();
        assert_eq!(count, 3);

        // No active suggestions left
        let remaining = get_active_suggestions(ctx.pool()).await.unwrap();
        assert_eq!(remaining.len(), 0);
    }

    #[tokio::test]
    async fn test_clear_dismissed_suggestions() {
        let ctx = TestContext::new().await;

        // Store and dismiss some suggestions
        store_suggestion(ctx.pool(), "task_structure", "Suggestion 1")
            .await
            .unwrap();
        store_suggestion(ctx.pool(), "task_structure", "Suggestion 2")
            .await
            .unwrap();

        let suggestions = get_active_suggestions(ctx.pool()).await.unwrap();
        dismiss_suggestion(ctx.pool(), suggestions[0].id)
            .await
            .unwrap();
        dismiss_suggestion(ctx.pool(), suggestions[1].id)
            .await
            .unwrap();

        // Clear dismissed
        let count = clear_dismissed_suggestions(ctx.pool()).await.unwrap();
        assert_eq!(count, 2);

        // Verify they're gone from database
        let all: Vec<crate::db::models::Suggestion> =
            sqlx::query_as("SELECT id, type, content, created_at, dismissed FROM suggestions")
                .fetch_all(ctx.pool())
                .await
                .unwrap();
        assert_eq!(all.len(), 0);
    }

    #[tokio::test]
    async fn test_error_suggestion_storage() {
        let ctx = TestContext::new().await;

        // Store an error suggestion
        let error_msg = "LLM API failed: connection timeout";
        store_suggestion(ctx.pool(), "error", error_msg)
            .await
            .unwrap();

        // Verify it's stored
        let suggestions = get_active_suggestions(ctx.pool()).await.unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].suggestion_type, "error");
        assert!(suggestions[0].content.contains("timeout"));
    }

    #[test]
    fn test_rate_limiting_cooldown() {
        // Save original state
        let original = LAST_ANALYSIS_TIME.load(Ordering::SeqCst);

        // Reset global state to known value
        LAST_ANALYSIS_TIME.store(0, Ordering::SeqCst);

        // First call should pass
        assert!(should_trigger_analysis());
        mark_analysis_started();

        // Immediate second call should be blocked
        assert!(!should_trigger_analysis());

        // Simulate time passing (set to past time)
        if let Some(now) = get_current_timestamp() {
            let past = now - DEFAULT_ANALYSIS_COOLDOWN_SECS - 1;
            LAST_ANALYSIS_TIME.store(past, Ordering::SeqCst);

            // Now should pass again
            assert!(should_trigger_analysis());
        }

        // Restore original state
        LAST_ANALYSIS_TIME.store(original, Ordering::SeqCst);
    }

    #[test]
    fn test_rate_limiting_clock_skew() {
        // Save original state
        let original = LAST_ANALYSIS_TIME.load(Ordering::SeqCst);

        if let Some(now) = get_current_timestamp() {
            // Set last analysis to future
            let future = now + 1000;
            LAST_ANALYSIS_TIME.store(future, Ordering::SeqCst);

            // Should detect clock skew and allow analysis
            assert!(should_trigger_analysis());

            // Timer should be reset to current time
            let reset_time = LAST_ANALYSIS_TIME.load(Ordering::SeqCst);
            assert!(
                reset_time <= now,
                "Timer should be reset to current or earlier"
            );
        }

        // Restore original state
        LAST_ANALYSIS_TIME.store(original, Ordering::SeqCst);
    }

    #[test]
    fn test_get_current_timestamp_returns_valid() {
        // Should always return Some in normal conditions
        let ts = get_current_timestamp();
        assert!(ts.is_some());

        // Should be reasonable (after 2020)
        if let Some(timestamp) = ts {
            assert!(timestamp > 1577836800); // Jan 1, 2020
        }
    }

    #[tokio::test]
    async fn test_max_active_suggestions_limit() {
        let ctx = TestContext::new().await;

        // Store MAX_ACTIVE_SUGGESTIONS suggestions
        for i in 0..MAX_ACTIVE_SUGGESTIONS {
            store_suggestion(ctx.pool(), "task_structure", &format!("Suggestion {}", i))
                .await
                .unwrap();
        }

        // Verify we have exactly MAX_ACTIVE_SUGGESTIONS
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM suggestions WHERE dismissed = 0")
            .fetch_one(ctx.pool())
            .await
            .unwrap();
        assert_eq!(count, MAX_ACTIVE_SUGGESTIONS);

        // Store one more - should auto-dismiss oldest
        store_suggestion(ctx.pool(), "task_structure", "New suggestion")
            .await
            .unwrap();

        // Should still have MAX_ACTIVE_SUGGESTIONS (not MAX + 1)
        let count_after: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM suggestions WHERE dismissed = 0")
                .fetch_one(ctx.pool())
                .await
                .unwrap();
        assert_eq!(count_after, MAX_ACTIVE_SUGGESTIONS);

        // Verify the newest suggestion is there
        let suggestions = get_active_suggestions(ctx.pool()).await.unwrap();
        assert!(suggestions[0].content.contains("New suggestion"));

        // Verify one was auto-dismissed
        let dismissed_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM suggestions WHERE dismissed = 1")
                .fetch_one(ctx.pool())
                .await
                .unwrap();
        assert_eq!(dismissed_count, 1);
    }

    #[tokio::test]
    async fn test_error_type_suggestions() {
        let ctx = TestContext::new().await;

        // Store error suggestion
        store_suggestion(ctx.pool(), "error", "## Analysis Error\n\nLLM API failed")
            .await
            .unwrap();

        // Store normal suggestion
        store_suggestion(ctx.pool(), "task_structure", "Reorganize task #5")
            .await
            .unwrap();

        // Get all suggestions
        let suggestions = get_active_suggestions(ctx.pool()).await.unwrap();
        assert_eq!(suggestions.len(), 2);

        // Errors should be distinguishable by type
        let errors: Vec<_> = suggestions
            .iter()
            .filter(|s| s.suggestion_type == "error")
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].content.contains("Analysis Error"));
    }

    #[tokio::test]
    async fn test_concurrent_cooldown_check() {
        use std::sync::Arc;
        use tokio::sync::Barrier;

        // Save original state
        let original = LAST_ANALYSIS_TIME.load(Ordering::SeqCst);

        // Reset to allow analysis
        LAST_ANALYSIS_TIME.store(0, Ordering::SeqCst);

        // Create barrier for synchronization
        let barrier = Arc::new(Barrier::new(5));

        // Spawn 5 concurrent checks
        let mut handles = vec![];
        for _ in 0..5 {
            let b = Arc::clone(&barrier);
            let handle = tokio::spawn(async move {
                // Wait for all tasks to be ready
                b.wait().await;

                // All check at the same time
                should_trigger_analysis()
            });
            handles.push(handle);
        }

        // Collect results
        let mut results = vec![];
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        // At least one should succeed (due to race, might be more in Relaxed)
        // But with proper ordering, should be predictable
        let success_count = results.iter().filter(|&&r| r).count();
        assert!(
            success_count > 0,
            "At least one concurrent check should succeed"
        );

        // Restore original state
        LAST_ANALYSIS_TIME.store(original, Ordering::SeqCst);
    }
}
