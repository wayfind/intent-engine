/// CLI Notifier - HTTP client for notifying Dashboard of CLI operations
///
/// This module provides a simple HTTP notification mechanism to inform the
/// Dashboard when CLI commands modify the database. The Dashboard will then
/// broadcast changes to connected WebSocket clients for real-time UI updates.
use std::time::Duration;

/// Default Dashboard port
const DASHBOARD_PORT: u16 = 11391;

/// Notification message types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationMessage {
    /// Task was created/updated/deleted
    TaskChanged {
        task_id: Option<i64>,
        operation: String,
        /// Project path that sent this notification
        project_path: Option<String>,
    },
    /// Event was added
    EventAdded {
        task_id: i64,
        event_id: i64,
        /// Project path that sent this notification
        project_path: Option<String>,
    },
    /// Workspace state changed (current_task_id updated)
    WorkspaceChanged {
        current_task_id: Option<i64>,
        /// Project path that sent this notification
        project_path: Option<String>,
    },
}

/// CLI Notifier for sending notifications to Dashboard
pub struct CliNotifier {
    base_url: String,
    client: reqwest::Client,
}

impl CliNotifier {
    /// Create a new CLI notifier
    pub fn new() -> Self {
        Self::with_port(DASHBOARD_PORT)
    }

    /// Create a CLI notifier with custom port (for testing)
    pub fn with_port(port: u16) -> Self {
        let base_url = format!("http://127.0.0.1:{}", port);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(100)) // Short timeout - don't block CLI
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { base_url, client }
    }

    /// Send a notification to Dashboard (fire-and-forget, non-blocking)
    pub async fn notify(&self, message: NotificationMessage) {
        // Check: Environment variable to disable notifications
        if std::env::var("IE_DISABLE_DASHBOARD_NOTIFICATIONS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            tracing::debug!(
                "Dashboard notifications disabled via IE_DISABLE_DASHBOARD_NOTIFICATIONS"
            );
            return; // Skip all notification logic
        }

        let url = format!("{}/api/internal/cli-notify", self.base_url);

        // Fire-and-forget: don't block CLI command on Dashboard response
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.post(&url).json(&message).send().await {
                tracing::debug!("Failed to notify Dashboard: {}", e);
                // Silently ignore errors - Dashboard might not be running
            }
        });
    }

    /// Notify about task change
    pub async fn notify_task_changed(
        &self,
        task_id: Option<i64>,
        operation: &str,
        project_path: Option<String>,
    ) {
        self.notify(NotificationMessage::TaskChanged {
            task_id,
            operation: operation.to_string(),
            project_path,
        })
        .await;
    }

    /// Notify about event added
    pub async fn notify_event_added(
        &self,
        task_id: i64,
        event_id: i64,
        project_path: Option<String>,
    ) {
        self.notify(NotificationMessage::EventAdded {
            task_id,
            event_id,
            project_path,
        })
        .await;
    }

    /// Notify about workspace change
    pub async fn notify_workspace_changed(
        &self,
        current_task_id: Option<i64>,
        project_path: Option<String>,
    ) {
        self.notify(NotificationMessage::WorkspaceChanged {
            current_task_id,
            project_path,
        })
        .await;
    }
}

impl Default for CliNotifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notifier_creation() {
        let notifier = CliNotifier::new();
        assert_eq!(notifier.base_url, "http://127.0.0.1:11391");
    }

    #[test]
    fn test_notifier_with_custom_port() {
        let notifier = CliNotifier::with_port(8080);
        assert_eq!(notifier.base_url, "http://127.0.0.1:8080");
    }

    #[tokio::test]
    async fn test_notify_non_blocking() {
        // This should not panic even if Dashboard is not running
        let notifier = CliNotifier::with_port(65000); // Port not in use
        notifier
            .notify_task_changed(Some(42), "created", Some("/test/path".to_string()))
            .await;

        // Should return immediately (fire-and-forget)
        // No assertions needed - test passes if no panic
    }
}
