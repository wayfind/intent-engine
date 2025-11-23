// WebSocket support for Dashboard
// Handles real-time communication between MCP servers and UI clients

/// Intent-Engine Protocol Version
pub const PROTOCOL_VERSION: &str = "1.0";

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

/// Protocol message wrapper - wraps all WebSocket messages with version and timestamp
#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolMessage<T> {
    /// Protocol version (e.g., "1.0")
    pub version: String,
    /// Message type identifier
    #[serde(rename = "type")]
    pub message_type: String,
    /// Message payload
    pub payload: T,
    /// ISO 8601 timestamp when message was created
    pub timestamp: String,
}

impl<T> ProtocolMessage<T>
where
    T: Serialize,
{
    /// Create a new protocol message with current timestamp
    pub fn new(message_type: impl Into<String>, payload: T) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_string(),
            message_type: message_type.into(),
            payload,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl<T> ProtocolMessage<T>
where
    T: for<'de> Deserialize<'de>,
{
    /// Deserialize from JSON string with version validation
    pub fn from_json(json: &str) -> Result<Self, String> {
        let msg: Self = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse protocol message: {}", e))?;

        // Validate protocol version (major version must match)
        let expected_major = PROTOCOL_VERSION.split('.').next().unwrap_or("1");
        let received_major = msg.version.split('.').next().unwrap_or("0");

        if expected_major != received_major {
            return Err(format!(
                "Protocol version mismatch: expected {}, got {}",
                PROTOCOL_VERSION, msg.version
            ));
        }

        Ok(msg)
    }
}

/// Project information sent by MCP servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub db_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Whether this project has an active MCP connection
    pub mcp_connected: bool,
    /// Whether the Dashboard serving this project is online
    pub is_online: bool,
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

    /// Get list of all online projects from in-memory state
    pub async fn get_online_projects(&self) -> Vec<ProjectInfo> {
        // Read from in-memory MCP connections
        let connections = self.mcp_connections.read().await;

        connections
            .values()
            .map(|conn| {
                let mut project = conn.project.clone();
                project.mcp_connected = true; // All projects in the map are connected
                project
            })
            .collect()
    }

    /// Get list of all online projects
    /// Always includes the current Dashboard project plus all MCP-connected projects
    /// This is the single source of truth for project status
    pub async fn get_online_projects_with_current(
        &self,
        current_project_name: &str,
        current_project_path: &std::path::Path,
        current_db_path: &std::path::Path,
        _port: u16,
    ) -> Vec<ProjectInfo> {
        let connections = self.mcp_connections.read().await;
        let current_path_str = current_project_path.display().to_string();

        let mut projects = Vec::new();

        // 1. Always add current Dashboard project first
        // Check if this project also has an MCP connection
        let current_has_mcp = connections
            .values()
            .any(|conn| conn.project.path == current_path_str);

        projects.push(ProjectInfo {
            name: current_project_name.to_string(),
            path: current_path_str.clone(),
            db_path: current_db_path.display().to_string(),
            agent: None, // Dashboard itself doesn't have an agent name
            mcp_connected: current_has_mcp,
            is_online: true, // Dashboard is online (serving this response)
        });

        // 2. Add all other MCP-connected projects (excluding current project to avoid duplication)
        for conn in connections.values() {
            if conn.project.path != current_path_str {
                let mut project = conn.project.clone();
                project.mcp_connected = true;
                project.is_online = true; // MCP connection means project is online
                projects.push(project);
            }
        }

        projects
    }
}

// ============================================================================
// Payload Structures (used inside ProtocolMessage)
// ============================================================================

/// Payload for MCP register message
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterPayload {
    pub project: ProjectInfo,
}

/// Payload for MCP registered response
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisteredPayload {
    pub success: bool,
}

/// Empty payload for ping/pong messages
#[derive(Debug, Serialize, Deserialize)]
pub struct EmptyPayload {}

/// Payload for UI init message
#[derive(Debug, Serialize, Deserialize)]
pub struct InitPayload {
    pub projects: Vec<ProjectInfo>,
}

/// Payload for UI project_online message
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectOnlinePayload {
    pub project: ProjectInfo,
}

/// Payload for UI project_offline message
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectOfflinePayload {
    pub project_path: String,
}

/// Payload for hello message (client → server)
#[derive(Debug, Serialize, Deserialize)]
pub struct HelloPayload {
    /// Client entity type ("mcp" or "ui")
    pub entity_type: String,
    /// Client capabilities (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,
}

/// Payload for welcome message (server → client)
#[derive(Debug, Serialize, Deserialize)]
pub struct WelcomePayload {
    /// Server capabilities
    pub capabilities: Vec<String>,
    /// Session ID
    pub session_id: String,
}

/// Payload for goodbye message
#[derive(Debug, Serialize, Deserialize)]
pub struct GoodbyePayload {
    /// Reason for closing (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Payload for error message (Protocol v1.0 Section 4.5)
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorPayload {
    /// Machine-readable error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details (for debugging)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Standard error codes (Protocol v1.0 Section 4.5)
pub mod error_codes {
    pub const UNSUPPORTED_VERSION: &str = "unsupported_version";
    pub const INVALID_MESSAGE: &str = "invalid_message";
    pub const INVALID_PATH: &str = "invalid_path";
    pub const REGISTRATION_FAILED: &str = "registration_failed";
    pub const INTERNAL_ERROR: &str = "internal_error";
}

/// Payload for database operation notifications
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseOperationPayload {
    /// Operation type: create, read, update, delete
    pub operation: String,

    /// Entity type: task, event
    pub entity: String,

    /// List of affected IDs
    pub affected_ids: Vec<i64>,

    /// Full data for create/update operations
    /// Empty for delete operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,

    /// Project path (for multi-project scenarios)
    pub project_path: String,
}

impl DatabaseOperationPayload {
    /// Create a new database operation payload
    pub fn new(
        operation: impl Into<String>,
        entity: impl Into<String>,
        affected_ids: Vec<i64>,
        data: Option<serde_json::Value>,
        project_path: impl Into<String>,
    ) -> Self {
        Self {
            operation: operation.into(),
            entity: entity.into(),
            affected_ids,
            data,
            project_path: project_path.into(),
        }
    }

    /// Helper: Create payload for task created
    pub fn task_created(
        task_id: i64,
        task_data: serde_json::Value,
        project_path: impl Into<String>,
    ) -> Self {
        Self::new(
            "create",
            "task",
            vec![task_id],
            Some(task_data),
            project_path,
        )
    }

    /// Helper: Create payload for task updated
    pub fn task_updated(
        task_id: i64,
        task_data: serde_json::Value,
        project_path: impl Into<String>,
    ) -> Self {
        Self::new(
            "update",
            "task",
            vec![task_id],
            Some(task_data),
            project_path,
        )
    }

    /// Helper: Create payload for task deleted
    pub fn task_deleted(task_id: i64, project_path: impl Into<String>) -> Self {
        Self::new("delete", "task", vec![task_id], None, project_path)
    }

    /// Helper: Create payload for task read
    pub fn task_read(task_id: i64, project_path: impl Into<String>) -> Self {
        Self::new("read", "task", vec![task_id], None, project_path)
    }

    /// Helper: Create payload for event created
    pub fn event_created(
        event_id: i64,
        event_data: serde_json::Value,
        project_path: impl Into<String>,
    ) -> Self {
        Self::new(
            "create",
            "event",
            vec![event_id],
            Some(event_data),
            project_path,
        )
    }

    /// Helper: Create payload for event updated
    pub fn event_updated(
        event_id: i64,
        event_data: serde_json::Value,
        project_path: impl Into<String>,
    ) -> Self {
        Self::new(
            "update",
            "event",
            vec![event_id],
            Some(event_data),
            project_path,
        )
    }

    /// Helper: Create payload for event deleted
    pub fn event_deleted(event_id: i64, project_path: impl Into<String>) -> Self {
        Self::new("delete", "event", vec![event_id], None, project_path)
    }
}

// ============================================================================
// Helper Functions for Sending Protocol Messages
// ============================================================================

/// Send a protocol message through a channel
fn send_protocol_message<T: Serialize>(
    tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    message_type: &str,
    payload: T,
) -> Result<(), String> {
    let protocol_msg = ProtocolMessage::new(message_type, payload);
    let json = protocol_msg
        .to_json()
        .map_err(|e| format!("Failed to serialize message: {}", e))?;

    tx.send(Message::Text(json))
        .map_err(|_| "Failed to send message: channel closed".to_string())
}

/// Handle MCP WebSocket connections
pub async fn handle_mcp_websocket(
    ws: WebSocketUpgrade,
    State(app_state): State<crate::dashboard::server::AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_mcp_socket(socket, app_state.ws_state))
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
    let mut session_welcomed = false; // Track if welcome handshake completed

    // Clone state for use inside recv_task
    let state_for_recv = state.clone();

    // Clone tx for heartbeat task
    let heartbeat_tx = tx.clone();

    // Spawn heartbeat task - send ping every 30 seconds (Protocol v1.0 Section 4.1.3)
    let mut heartbeat_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        // Skip the first tick (which completes immediately)
        interval.tick().await;

        loop {
            interval.tick().await;
            // Send ping to request heartbeat from client
            if send_protocol_message(&heartbeat_tx, "ping", EmptyPayload {}).is_err() {
                // Connection closed
                break;
            }
            tracing::trace!("Sent heartbeat ping to MCP client");
        }
    });

    // Handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Parse incoming protocol message
                    let parsed_msg = match ProtocolMessage::<serde_json::Value>::from_json(&text) {
                        Ok(msg) => msg,
                        Err(e) => {
                            tracing::warn!("Protocol error: {}", e);

                            // Send error message to client
                            let error_code = if e.contains("version mismatch") {
                                error_codes::UNSUPPORTED_VERSION
                            } else {
                                error_codes::INVALID_MESSAGE
                            };

                            let error_payload = ErrorPayload {
                                code: error_code.to_string(),
                                message: e.to_string(),
                                details: None,
                            };

                            let _ = send_protocol_message(&tx, "error", error_payload);
                            continue;
                        },
                    };

                    match parsed_msg.message_type.as_str() {
                        "hello" => {
                            // Parse hello payload
                            let hello: HelloPayload =
                                match serde_json::from_value(parsed_msg.payload.clone()) {
                                    Ok(h) => h,
                                    Err(e) => {
                                        tracing::warn!("Failed to parse hello payload: {}", e);
                                        continue;
                                    },
                                };

                            tracing::info!("Received hello from {} client", hello.entity_type);

                            // Generate session ID
                            let session_id = format!(
                                "{}-{}",
                                hello.entity_type,
                                chrono::Utc::now().timestamp_millis()
                            );

                            // Send welcome response
                            let welcome_payload = WelcomePayload {
                                session_id,
                                capabilities: vec![], // TODO: Add actual capabilities
                            };

                            if send_protocol_message(&tx, "welcome", welcome_payload).is_ok() {
                                session_welcomed = true;
                                tracing::debug!("Sent welcome message");
                            } else {
                                tracing::error!("Failed to send welcome message");
                            }
                        },
                        "register" => {
                            // Check if handshake completed (backward compatibility: allow register without hello for now)
                            if !session_welcomed {
                                tracing::warn!(
                                    "MCP client registered without hello handshake (legacy client detected)"
                                );
                            }

                            // Parse register payload
                            let project: ProjectInfo =
                                match serde_json::from_value(parsed_msg.payload.clone()) {
                                    Ok(p) => p,
                                    Err(e) => {
                                        tracing::warn!("Failed to parse register payload: {}", e);
                                        continue;
                                    },
                                };
                            tracing::info!("MCP registering project: {}", project.name);

                            let path = project.path.clone();
                            let project_path_buf = std::path::PathBuf::from(&path);

                            // Validate project path - reject temporary directories (Defense Layer 5)
                            // This prevents test environments from polluting the Dashboard registry
                            let normalized_path = project_path_buf
                                .canonicalize()
                                .unwrap_or_else(|_| project_path_buf.clone());

                            // IMPORTANT: Canonicalize temp_dir to match normalized_path format (fixes Windows UNC paths)
                            let temp_dir = std::env::temp_dir()
                                .canonicalize()
                                .unwrap_or_else(|_| std::env::temp_dir());
                            let is_temp_path = normalized_path.starts_with(&temp_dir);

                            if is_temp_path {
                                tracing::warn!(
                                    "Rejecting MCP registration for temporary/invalid path: {}",
                                    path
                                );

                                // Send error message
                                let error_payload = ErrorPayload {
                                    code: error_codes::INVALID_PATH.to_string(),
                                    message: "Path is in temporary directory".to_string(),
                                    details: Some(serde_json::json!({"path": path})),
                                };
                                let _ = send_protocol_message(&tx, "error", error_payload);

                                // Send rejection response
                                let _ = send_protocol_message(
                                    &tx,
                                    "registered",
                                    RegisteredPayload { success: false },
                                );
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

                            tracing::info!("✓ MCP connected: {} ({})", project.name, path);

                            // Send confirmation
                            let _ = send_protocol_message(
                                &tx,
                                "registered",
                                RegisteredPayload { success: true },
                            );

                            // Broadcast to UI clients with mcp_connected=true
                            let mut project_info = project.clone();
                            project_info.mcp_connected = true;
                            let ui_msg = ProtocolMessage::new(
                                "project_online",
                                ProjectOnlinePayload {
                                    project: project_info,
                                },
                            );
                            state_for_recv
                                .broadcast_to_ui(&ui_msg.to_json().unwrap())
                                .await;
                        },
                        "pong" => {
                            // Client responded to our ping - heartbeat confirmed
                            tracing::trace!("Received pong from MCP client - heartbeat confirmed");
                        },
                        "goodbye" => {
                            // Client is closing connection gracefully
                            if let Ok(goodbye_payload) =
                                serde_json::from_value::<GoodbyePayload>(parsed_msg.payload)
                            {
                                if let Some(reason) = goodbye_payload.reason {
                                    tracing::info!("MCP client closing connection: {}", reason);
                                } else {
                                    tracing::info!("MCP client closing connection gracefully");
                                }
                            }
                            // Break loop to close connection
                            break;
                        },
                        "db_operation" => {
                            // MCP client is notifying about a database operation
                            // Forward directly to all UI clients for real-time updates
                            tracing::debug!(
                                "Received db_operation from MCP, forwarding to UI clients"
                            );
                            state_for_recv.broadcast_to_ui(&text).await;
                        },
                        _ => {
                            tracing::warn!("Unknown message type: {}", parsed_msg.message_type);
                        },
                    }
                },
                Message::Close(_) => {
                    tracing::info!("MCP client closed WebSocket");
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

                tracing::info!("MCP disconnected: {}", path);

                // Notify UI clients
                let ui_msg = ProtocolMessage::new(
                    "project_offline",
                    ProjectOfflinePayload { project_path: path.clone() },
                );
                state
                    .broadcast_to_ui(&ui_msg.to_json().unwrap())
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
    State(app_state): State<crate::dashboard::server::AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ui_socket(socket, app_state))
}

async fn handle_ui_socket(socket: WebSocket, app_state: crate::dashboard::server::AppState) {
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

    // Protocol v1.0 Compliance: Wait for client to send "hello" first
    // The "init" message will be sent after receiving "hello" and sending "welcome"
    // This is handled in the message loop below

    // Register this UI connection
    let conn = UiConnection {
        tx: tx.clone(),
        connected_at: chrono::Utc::now(),
    };
    let conn_index = {
        let mut connections = app_state.ws_state.ui_connections.write().await;
        connections.push(conn);
        connections.len() - 1
    };

    tracing::info!("UI client connected");

    // Clone app_state for use inside recv_task
    let app_state_for_recv = app_state.clone();

    // Clone tx for heartbeat task
    let heartbeat_tx = tx.clone();

    // Spawn heartbeat task - send ping every 30 seconds (Protocol v1.0 Section 4.1.3)
    let mut heartbeat_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        // Skip the first tick (which completes immediately)
        interval.tick().await;

        loop {
            interval.tick().await;
            if send_protocol_message(&heartbeat_tx, "ping", EmptyPayload {}).is_err() {
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
                    // Parse protocol message from UI
                    if let Ok(parsed_msg) =
                        serde_json::from_str::<ProtocolMessage<serde_json::Value>>(&text)
                    {
                        match parsed_msg.message_type.as_str() {
                            "hello" => {
                                // Parse hello payload
                                if let Ok(hello) =
                                    serde_json::from_value::<HelloPayload>(parsed_msg.payload)
                                {
                                    tracing::info!(
                                        "Received hello from {} client",
                                        hello.entity_type
                                    );

                                    // Generate session ID
                                    let session_id = format!(
                                        "{}-{}",
                                        hello.entity_type,
                                        chrono::Utc::now().timestamp_millis()
                                    );

                                    // Send welcome response
                                    let welcome_payload = WelcomePayload {
                                        session_id,
                                        capabilities: vec![],
                                    };

                                    let _ = send_protocol_message(&tx, "welcome", welcome_payload);
                                    tracing::debug!("Sent welcome message to UI");

                                    // Send init after welcome (protocol-compliant flow)
                                    // Re-fetch projects in case state changed
                                    let current_projects = {
                                        let current_project =
                                            app_state_for_recv.current_project.read().await;
                                        let port = app_state_for_recv.port;
                                        app_state_for_recv
                                            .ws_state
                                            .get_online_projects_with_current(
                                                &current_project.project_name,
                                                &current_project.project_path,
                                                &current_project.db_path,
                                                port,
                                            )
                                            .await
                                    };
                                    let _ = send_protocol_message(
                                        &tx,
                                        "init",
                                        InitPayload {
                                            projects: current_projects,
                                        },
                                    );
                                }
                            },
                            "pong" => {
                                tracing::trace!("Received pong from UI");
                            },
                            "goodbye" => {
                                // UI client closing gracefully
                                if let Ok(goodbye_payload) =
                                    serde_json::from_value::<GoodbyePayload>(parsed_msg.payload)
                                {
                                    if let Some(reason) = goodbye_payload.reason {
                                        tracing::info!("UI client closing: {}", reason);
                                    } else {
                                        tracing::info!("UI client closing gracefully");
                                    }
                                }
                                break;
                            },
                            _ => {
                                tracing::trace!(
                                    "Received from UI: {} ({})",
                                    parsed_msg.message_type,
                                    text
                                );
                            },
                        }
                    } else {
                        tracing::trace!("Received non-protocol message from UI: {}", text);
                    }
                },
                Message::Pong(_) => {
                    tracing::trace!("Received WebSocket pong from UI");
                },
                Message::Close(_) => {
                    tracing::info!("UI client closed WebSocket");
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
    app_state
        .ws_state
        .ui_connections
        .write()
        .await
        .swap_remove(conn_index);
    tracing::info!("UI client disconnected");
}
