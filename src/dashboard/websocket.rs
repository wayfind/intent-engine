// WebSocket support for Dashboard
// Handles real-time communication between MCP servers and UI clients

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Project information sent by MCP servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub db_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

/// MCP connection entry
#[derive(Debug)]
pub struct McpConnection {
    pub tx: tokio::sync::mpsc::UnboundedSender<Message>,
    pub project: ProjectInfo,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// UI connection entry
#[derive(Debug)]
pub struct UiConnection {
    pub tx: tokio::sync::mpsc::UnboundedSender<Message>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// Shared WebSocket state
#[derive(Clone)]
pub struct WebSocketState {
    /// Project path → MCP connection
    pub mcp_connections: Arc<RwLock<HashMap<String, McpConnection>>>,
    /// List of active UI connections
    pub ui_connections: Arc<RwLock<Vec<UiConnection>>>,
}

impl Default for WebSocketState {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketState {
    pub fn new() -> Self {
        Self {
            mcp_connections: Arc::new(RwLock::new(HashMap::new())),
            ui_connections: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Broadcast message to all UI connections
    pub async fn broadcast_to_ui(&self, message: &str) {
        let connections = self.ui_connections.read().await;
        for conn in connections.iter() {
            let _ = conn.tx.send(Message::Text(message.to_string()));
        }
    }

    /// Get list of currently connected projects from Registry
    pub async fn get_online_projects(&self) -> Vec<ProjectInfo> {
        // Load from Registry to get accurate mcp_connected status
        // This ensures UI gets complete project list even if WebSocket connections haven't been established yet
        match crate::dashboard::registry::ProjectRegistry::load() {
            Ok(registry) => registry
                .projects
                .iter()
                .filter(|p| p.mcp_connected)
                .map(|p| ProjectInfo {
                    name: p.name.clone(),
                    path: p.path.display().to_string(),
                    db_path: p.db_path.display().to_string(),
                    agent: p.mcp_agent.clone(),
                })
                .collect(),
            Err(e) => {
                tracing::warn!("Failed to load registry for online projects: {}", e);
                Vec::new()
            },
        }
    }
}

/// Message types from MCP to Dashboard
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum McpMessage {
    #[serde(rename = "register")]
    Register { project: ProjectInfo },
    #[serde(rename = "ping")]
    Ping,
}

/// Message types from Dashboard to MCP
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum McpResponse {
    #[serde(rename = "registered")]
    Registered { success: bool },
    #[serde(rename = "pong")]
    Pong,
}

/// Message types from Dashboard to UI
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum UiMessage {
    #[serde(rename = "init")]
    Init { projects: Vec<ProjectInfo> },
    #[serde(rename = "project_online")]
    ProjectOnline { project: ProjectInfo },
    #[serde(rename = "project_offline")]
    ProjectOffline { project_path: String },
    #[serde(rename = "ping")]
    Ping,
}

/// Handle MCP WebSocket connections
pub async fn handle_mcp_websocket(
    ws: WebSocketUpgrade,
    State(state): State<WebSocketState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_mcp_socket(socket, state))
}

async fn handle_mcp_socket(socket: WebSocket, state: WebSocketState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Spawn task to forward messages from channel to WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Variables to track this connection
    let mut project_path: Option<String> = None;

    // Clone state for use inside recv_task
    let state_for_recv = state.clone();

    // Clone tx for heartbeat task
    let heartbeat_tx = tx.clone();

    // Spawn heartbeat task - send ping every 30 seconds
    let mut heartbeat_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let ping_msg = McpResponse::Pong; // Use Pong as keepalive for MCP
            if heartbeat_tx
                .send(Message::Text(serde_json::to_string(&ping_msg).unwrap()))
                .is_err()
            {
                // Connection closed
                break;
            }
            tracing::trace!("Sent heartbeat to MCP client");
        }
    });

    // Handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Parse incoming message
                    match serde_json::from_str::<McpMessage>(&text) {
                        Ok(McpMessage::Register { project }) => {
                            tracing::info!("MCP registering project: {}", project.name);

                            let path = project.path.clone();
                            let project_path_buf = std::path::PathBuf::from(&path);

                            // Validate project path - reject temporary directories (Defense Layer 5)
                            // This prevents test environments from polluting the Dashboard registry
                            let normalized_path = project_path_buf
                                .canonicalize()
                                .unwrap_or_else(|_| project_path_buf.clone());

                            let temp_dir = std::env::temp_dir();
                            let is_temp_path = normalized_path.starts_with(&temp_dir);

                            if is_temp_path {
                                tracing::warn!(
                                    "Rejecting MCP registration for temporary/invalid path: {}",
                                    path
                                );

                                // Send rejection response
                                let response = McpResponse::Registered { success: false };
                                let _ = tx
                                    .send(Message::Text(serde_json::to_string(&response).unwrap()));
                                continue; // Skip registration
                            }

                            // Store connection
                            let conn = McpConnection {
                                tx: tx.clone(),
                                project: project.clone(),
                                connected_at: chrono::Utc::now(),
                            };

                            state_for_recv
                                .mcp_connections
                                .write()
                                .await
                                .insert(path.clone(), conn);
                            project_path = Some(path.clone());

                            // Update Registry immediately to set mcp_connected=true
                            match crate::dashboard::registry::ProjectRegistry::load() {
                                Ok(mut registry) => {
                                    if let Err(e) = registry.register_mcp_connection(
                                        &project_path_buf,
                                        project.agent.clone(),
                                    ) {
                                        tracing::warn!(
                                            "Failed to update Registry for MCP connection: {}",
                                            e
                                        );
                                    } else {
                                        tracing::info!(
                                            "✓ Updated Registry: {} is now mcp_connected=true",
                                            project.name
                                        );
                                    }
                                },
                                Err(e) => {
                                    tracing::warn!("Failed to load Registry: {}", e);
                                },
                            }

                            // Send confirmation
                            let response = McpResponse::Registered { success: true };
                            let _ =
                                tx.send(Message::Text(serde_json::to_string(&response).unwrap()));

                            // Broadcast to UI clients
                            let ui_msg = UiMessage::ProjectOnline { project };
                            state_for_recv
                                .broadcast_to_ui(&serde_json::to_string(&ui_msg).unwrap())
                                .await;
                        },
                        Ok(McpMessage::Ping) => {
                            // Respond with pong
                            let response = McpResponse::Pong;
                            let _ =
                                tx.send(Message::Text(serde_json::to_string(&response).unwrap()));
                        },
                        Err(e) => {
                            tracing::warn!("Failed to parse MCP message: {}", e);
                        },
                    }
                },
                Message::Close(_) => {
                    break;
                },
                _ => {},
            }
        }

        project_path
    });

    // Wait for any task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
            heartbeat_task.abort();
        }
        project_path_result = (&mut recv_task) => {
            send_task.abort();
            heartbeat_task.abort();
            if let Ok(Some(path)) = project_path_result {
                // Clean up connection
                state.mcp_connections.write().await.remove(&path);

                // Update Registry immediately to set mcp_connected=false
                let project_path_buf = std::path::PathBuf::from(&path);
                match crate::dashboard::registry::ProjectRegistry::load() {
                    Ok(mut registry) => {
                        if let Err(e) = registry.unregister_mcp_connection(&project_path_buf) {
                            tracing::warn!("Failed to update Registry for MCP disconnection: {}", e);
                        } else {
                            tracing::info!("✓ Updated Registry: {} is now mcp_connected=false", path);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load Registry: {}", e);
                    }
                }

                // Notify UI clients
                let ui_msg = UiMessage::ProjectOffline { project_path: path.clone() };
                state
                    .broadcast_to_ui(&serde_json::to_string(&ui_msg).unwrap())
                    .await;

                tracing::info!("MCP disconnected: {}", path);
            }
        }
        _ = (&mut heartbeat_task) => {
            send_task.abort();
            recv_task.abort();
        }
    }
}

/// Handle UI WebSocket connections
pub async fn handle_ui_websocket(
    ws: WebSocketUpgrade,
    State(state): State<WebSocketState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ui_socket(socket, state))
}

async fn handle_ui_socket(socket: WebSocket, state: WebSocketState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Spawn task to forward messages from channel to WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Send initial project list
    let projects = state.get_online_projects().await;
    let init_msg = UiMessage::Init { projects };
    let _ = tx.send(Message::Text(serde_json::to_string(&init_msg).unwrap()));

    // Register this UI connection
    let conn = UiConnection {
        tx: tx.clone(),
        connected_at: chrono::Utc::now(),
    };
    let conn_index = {
        let mut connections = state.ui_connections.write().await;
        connections.push(conn);
        connections.len() - 1
    };

    tracing::info!("UI client connected");

    // Clone tx for heartbeat task
    let heartbeat_tx = tx.clone();

    // Spawn heartbeat task - send ping every 30 seconds
    let mut heartbeat_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let ping_msg = UiMessage::Ping;
            if heartbeat_tx
                .send(Message::Text(serde_json::to_string(&ping_msg).unwrap()))
                .is_err()
            {
                // Connection closed
                break;
            }
            tracing::trace!("Sent heartbeat ping to UI client");
        }
    });

    // Handle incoming messages (mostly just keep-alive and pong)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // UI can send pong or other messages
                    tracing::trace!("Received from UI: {}", text);
                },
                Message::Pong(_) => {
                    tracing::trace!("Received pong from UI");
                },
                Message::Close(_) => {
                    break;
                },
                _ => {},
            }
        }
    });

    // Wait for any task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
            heartbeat_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
            heartbeat_task.abort();
        }
        _ = (&mut heartbeat_task) => {
            send_task.abort();
            recv_task.abort();
        }
    }

    // Clean up UI connection
    state.ui_connections.write().await.swap_remove(conn_index);
    tracing::info!("UI client disconnected");
}
