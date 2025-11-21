// WebSocket client for MCP → Dashboard communication
// Handles registration and keep-alive for MCP server instances

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Project information sent to Dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub db_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

/// Message types sent by MCP client
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum McpMessage {
    #[serde(rename = "register")]
    Register { project: ProjectInfo },
    #[serde(rename = "ping")]
    Ping,
}

/// Response types from Dashboard
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum DashboardResponse {
    #[serde(rename = "registered")]
    Registered { success: bool },
    #[serde(rename = "pong")]
    Pong,
}

/// Start WebSocket client connection to Dashboard
/// This replaces the Registry-based registration mechanism
pub async fn connect_to_dashboard(
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

    // Validate project path - reject temporary directories
    // This prevents test environments from polluting the Dashboard registry
    if normalized_project_path.starts_with("/tmp")
        || normalized_project_path.starts_with(std::env::temp_dir())
    {
        tracing::warn!(
            "Skipping Dashboard registration for temporary path: {}",
            normalized_project_path.display()
        );
        return Ok(()); // Silently skip, don't error - non-fatal for MCP server
    }

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

    // Send registration message
    let register_msg = McpMessage::Register {
        project: project_info.clone(),
    };
    let register_json = serde_json::to_string(&register_msg)?;
    write
        .send(Message::Text(register_json))
        .await
        .context("Failed to send register message")?;

    // Wait for registration confirmation
    if let Some(Ok(Message::Text(text))) = read.next().await {
        match serde_json::from_str::<DashboardResponse>(&text) {
            Ok(DashboardResponse::Registered { success: true }) => {
                tracing::debug!("Successfully registered with Dashboard");
            },
            Ok(DashboardResponse::Registered { success: false }) => {
                anyhow::bail!("Dashboard rejected registration");
            },
            _ => {
                tracing::debug!("Unexpected response during registration: {}", text);
            },
        }
    }

    // Spawn ping task
    let mut write_clone = write;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            let ping_msg = McpMessage::Ping;
            if let Ok(ping_json) = serde_json::to_string(&ping_msg) {
                if write_clone.send(Message::Text(ping_json)).await.is_err() {
                    tracing::warn!("Failed to send ping - Dashboard connection lost");
                    break;
                }
            }
        }
    });

    // Spawn read task to handle pongs and other messages
    tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<DashboardResponse>(&text) {
                    Ok(DashboardResponse::Pong) => {
                        tracing::debug!("Received pong from Dashboard");
                    },
                    _ => {
                        tracing::debug!("Received message from Dashboard: {}", text);
                    },
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
