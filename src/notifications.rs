/// Unified notification infrastructure for Dashboard WebSocket communication
///
/// This module provides a centralized NotificationSender that handles the common
/// pattern of sending database operation notifications via WebSocket.
use crate::dashboard::websocket::{DatabaseOperationPayload, ProtocolMessage, WebSocketState};
use std::sync::Arc;

/// Centralized notification sender for database operations
///
/// Handles the common 3-step pattern:
/// 1. Create ProtocolMessage from payload
/// 2. Serialize to JSON
/// 3. Send via WebSocket (Dashboard UI)
pub struct NotificationSender {
    ws_state: Option<Arc<WebSocketState>>,
}

impl NotificationSender {
    /// Create a new NotificationSender
    ///
    /// # Arguments
    /// * `ws_state` - Optional WebSocket state for Dashboard UI notifications
    pub fn new(ws_state: Option<Arc<WebSocketState>>) -> Self {
        Self { ws_state }
    }

    /// Send a database operation notification via all available channels
    ///
    /// This method handles:
    /// - Creating a ProtocolMessage wrapper
    /// - Serializing to JSON (with error handling)
    /// - Broadcasting to Dashboard WebSocket (if connected)
    /// - Sending to MCP channel (if connected)
    ///
    /// # Arguments
    /// * `payload` - The database operation payload to send
    pub async fn send(&self, payload: DatabaseOperationPayload) {
        use ProtocolMessage as PM;

        // Step 1: Wrap payload in protocol message
        let msg = PM::new("db_operation", payload);

        // Step 2: Serialize to JSON
        let json = match msg.to_json() {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to serialize notification message");
                return;
            },
        };

        // Step 3: Send via Dashboard WebSocket (if available)
        if let Some(ws) = &self.ws_state {
            ws.broadcast_to_ui(&json).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_sender_new() {
        let sender = NotificationSender::new(None);
        assert!(sender.ws_state.is_none());
    }

    #[tokio::test]
    async fn test_send_with_no_channels() {
        // Should not panic when no channels are configured
        let sender = NotificationSender::new(None);
        let payload = DatabaseOperationPayload {
            operation: "create".to_string(),
            entity: "task".to_string(),
            affected_ids: vec![1],
            data: None,
            project_path: "/test".to_string(),
        };

        sender.send(payload).await; // Should complete without error
    }
}
