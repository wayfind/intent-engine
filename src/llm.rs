use crate::cli_handlers::config_commands::{config_get, config_set};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

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
Output ONLY the markdown summary, no preamble or explanation."#,
            task_name, original_spec_text, events_text
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
}
