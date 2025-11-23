// WebSocket client for MCP → Dashboard communication
// Handles registration and keep-alive for MCP server instances

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Protocol version
const PROTOCOL_VERSION: &str = "1.0";

/// Protocol message wrapper
#[derive(Debug, Serialize, Deserialize)]
struct ProtocolMessage<T> {
    version: String,
    #[serde(rename = "type")]
    message_type: String,
    payload: T,
    timestamp: String,
}

impl<T: Serialize> ProtocolMessage<T> {
    fn new(message_type: impl Into<String>, payload: T) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_string(),
            message_type: message_type.into(),
            payload,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(Into::into)
    }
}

/// Empty payload for ping/pong messages
#[derive(Debug, Serialize, Deserialize)]
struct EmptyPayload {}

/// Payload for registered response
#[derive(Debug, Serialize, Deserialize)]
struct RegisteredPayload {
    success: bool,
}

/// Payload for goodbye message
#[derive(Debug, Serialize, Deserialize)]
struct GoodbyePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// Payload for hello message (client → server)
#[derive(Debug, Serialize, Deserialize)]
struct HelloPayload {
    entity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<Vec<String>>,
}

/// Payload for welcome message (server → client)
#[derive(Debug, Serialize, Deserialize)]
struct WelcomePayload {
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<Vec<String>>,
}

/// Payload for error message (server → client)
#[derive(Debug, Serialize, Deserialize)]
struct ErrorPayload {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

/// Project information sent to Dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub db_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

/// Reconnection delays in seconds (exponential backoff with max)
const RECONNECT_DELAYS: &[u64] = &[1, 2, 4, 8, 16, 32];

/// Start WebSocket client with automatic reconnection
/// This function runs indefinitely, reconnecting on disconnection
pub async fn connect_to_dashboard(
    project_path: PathBuf,
    db_path: PathBuf,
    agent: Option<String>,
) -> Result<()> {
    // Validate project path once at the beginning
    let normalized_project_path = project_path
        .canonicalize()
        .unwrap_or_else(|_| project_path.clone());

    let temp_dir = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());

    if normalized_project_path.starts_with(&temp_dir) {
        tracing::warn!(
            "Skipping Dashboard registration for temporary path: {}",
            normalized_project_path.display()
        );
        return Ok(()); // Silently skip for temp paths
    }

    let mut attempt = 0;

    // Infinite reconnection loop
    loop {
        tracing::info!("Connecting to Dashboard (attempt {})...", attempt + 1);

        match connect_and_run(project_path.clone(), db_path.clone(), agent.clone()).await {
            Ok(()) => {
                // Graceful close - reset attempt counter and retry immediately
                tracing::info!("Dashboard connection closed gracefully, reconnecting...");
                attempt = 0;
                // Small delay before reconnecting
                tokio::time::sleep(Duration::from_secs(1)).await;
            },
            Err(e) => {
                // Connection error - use exponential backoff
                tracing::warn!("Dashboard connection failed: {}. Retrying...", e);

                // Calculate delay with exponential backoff
                let delay_index = std::cmp::min(attempt, RECONNECT_DELAYS.len() - 1);
                let base_delay = RECONNECT_DELAYS[delay_index];

                // Add jitter: ±25% random variance
                let jitter_factor = rand::random::<f64>() * 2.0 - 1.0; // Range: -1.0 to 1.0
                let jitter_ms = (base_delay * 1000) as f64 * 0.25 * jitter_factor;
                let delay_ms = (base_delay * 1000) as f64 + jitter_ms;
                let delay = Duration::from_millis(delay_ms.max(0.0) as u64);

                tracing::info!(
                    "Waiting {:.1}s before retry (base: {}s + jitter: {:.1}s)",
                    delay.as_secs_f64(),
                    base_delay,
                    jitter_ms / 1000.0
                );

                tokio::time::sleep(delay).await;
                attempt += 1;
            },
        }
    }
}

/// Internal function: Connect to Dashboard and run until disconnection
/// Returns Ok(()) on graceful close, Err on connection failure
async fn connect_and_run(
    project_path: PathBuf,
    db_path: PathBuf,
    agent: Option<String>,
) -> Result<()> {
    // Extract project name from path
    let project_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Normalize paths to handle symlinks
    let normalized_project_path = project_path
        .canonicalize()
        .unwrap_or_else(|_| project_path.clone());
    let normalized_db_path = db_path.canonicalize().unwrap_or_else(|_| db_path.clone());

    // Create project info
    let project_info = ProjectInfo {
        path: normalized_project_path.to_string_lossy().to_string(),
        name: project_name,
        db_path: normalized_db_path.to_string_lossy().to_string(),
        agent,
    };

    // Connect to Dashboard WebSocket
    let url = "ws://127.0.0.1:11391/ws/mcp";
    let (ws_stream, _) = connect_async(url)
        .await
        .context("Failed to connect to Dashboard WebSocket")?;

    tracing::debug!("Connected to Dashboard at {}", url);

    let (mut write, mut read) = ws_stream.split();

    // Step 1: Send hello message (Protocol v1.0 handshake)
    let hello_msg = ProtocolMessage::new(
        "hello",
        HelloPayload {
            entity_type: "mcp_server".to_string(),
            capabilities: Some(vec![]),
        },
    );
    write
        .send(Message::Text(hello_msg.to_json()?))
        .await
        .context("Failed to send hello message")?;
    tracing::debug!("Sent hello message");

    // Step 2: Wait for welcome response
    if let Some(Ok(Message::Text(text))) = read.next().await {
        match serde_json::from_str::<ProtocolMessage<WelcomePayload>>(&text) {
            Ok(msg) if msg.message_type == "welcome" => {
                tracing::debug!(
                    "Received welcome from Dashboard (session: {})",
                    msg.payload.session_id
                );
            },
            Ok(msg) => {
                tracing::warn!(
                    "Expected welcome, received: {} (legacy Dashboard?)",
                    msg.message_type
                );
                // Continue anyway for backward compatibility
            },
            Err(e) => {
                tracing::warn!("Failed to parse welcome message: {}", e);
            },
        }
    }

    // Step 3: Send registration message
    let register_msg = ProtocolMessage::new("register", project_info.clone());
    let register_json = register_msg.to_json()?;
    write
        .send(Message::Text(register_json))
        .await
        .context("Failed to send register message")?;

    // Step 4: Wait for registration confirmation
    if let Some(Ok(Message::Text(text))) = read.next().await {
        match serde_json::from_str::<ProtocolMessage<RegisteredPayload>>(&text) {
            Ok(msg) if msg.message_type == "registered" && msg.payload.success => {
                tracing::debug!("Successfully registered with Dashboard");
            },
            Ok(msg) if msg.message_type == "registered" && !msg.payload.success => {
                anyhow::bail!("Dashboard rejected registration");
            },
            _ => {
                tracing::debug!("Unexpected response during registration: {}", text);
            },
        }
    }

    // Spawn read/write task to handle messages and respond to pings
    // Protocol v1.0 Section 4.1.3: Dashboard sends ping, client responds with pong
    tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(msg) =
                        serde_json::from_str::<ProtocolMessage<serde_json::Value>>(&text)
                    {
                        match msg.message_type.as_str() {
                            "ping" => {
                                // Dashboard sent ping - respond with pong
                                tracing::debug!(
                                    "Received ping from Dashboard, responding with pong"
                                );
                                let pong_msg = ProtocolMessage::new("pong", EmptyPayload {});
                                if let Ok(pong_json) = pong_msg.to_json() {
                                    if write.send(Message::Text(pong_json)).await.is_err() {
                                        tracing::warn!(
                                            "Failed to send pong - Dashboard connection lost"
                                        );
                                        break;
                                    }
                                }
                            },
                            "error" => {
                                // Dashboard sent an error
                                if let Ok(error) =
                                    serde_json::from_value::<ErrorPayload>(msg.payload)
                                {
                                    tracing::error!(
                                        "Dashboard error [{}]: {}",
                                        error.code,
                                        error.message
                                    );
                                    if let Some(details) = error.details {
                                        tracing::error!("  Details: {}", details);
                                    }

                                    // Handle critical errors
                                    match error.code.as_str() {
                                        "unsupported_version" => {
                                            tracing::error!(
                                                "Protocol version mismatch - connection will close"
                                            );
                                            break;
                                        },
                                        "invalid_path" => {
                                            tracing::error!("Project path rejected by Dashboard");
                                            break;
                                        },
                                        _ => {
                                            // Non-critical errors, continue
                                        },
                                    }
                                }
                            },
                            "goodbye" => {
                                // Dashboard is closing connection gracefully
                                if let Ok(goodbye) =
                                    serde_json::from_value::<GoodbyePayload>(msg.payload)
                                {
                                    if let Some(reason) = goodbye.reason {
                                        tracing::info!("Dashboard closing connection: {}", reason);
                                    } else {
                                        tracing::info!("Dashboard closing connection gracefully");
                                    }
                                }
                                break;
                            },
                            _ => {
                                tracing::debug!(
                                    "Received message from Dashboard: {} ({})",
                                    msg.message_type,
                                    text
                                );
                            },
                        }
                    } else {
                        tracing::debug!("Received non-protocol message: {}", text);
                    }
                },
                Message::Close(_) => {
                    tracing::info!("Dashboard closed connection");
                    break;
                },
                _ => {},
            }
        }
    });

    Ok(())
}
