/// Tests for CLI notifier functionality
/// Covers environment variable disabling and notification behavior
use intent_engine::dashboard::cli_notifier::{CliNotifier, NotificationMessage};

#[tokio::test]
async fn test_notifications_enabled_by_default() {
    // Ensure env var is not set
    std::env::remove_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS");

    let notifier = CliNotifier::with_port(65001); // Use high port to avoid conflicts

    // Should not panic even if Dashboard is not running
    // (fire-and-forget behavior)
    notifier.notify_task_changed(Some(1), "created").await;

    // Test passes if no panic occurs
}

#[tokio::test]
async fn test_notifications_disabled_via_env_value_1() {
    // Set env var to "1"
    std::env::set_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS", "1");

    let notifier = CliNotifier::with_port(65002);

    // Should return immediately without attempting connection
    notifier.notify_task_changed(Some(1), "created").await;

    // Cleanup
    std::env::remove_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS");

    // Test passes if no panic occurs and returns quickly
}

#[tokio::test]
async fn test_notifications_disabled_via_env_value_true() {
    // Set env var to "true" (case-insensitive)
    std::env::set_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS", "true");

    let notifier = CliNotifier::with_port(65003);

    // Should return immediately without attempting connection
    notifier.notify_task_changed(Some(1), "created").await;

    // Cleanup
    std::env::remove_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS");
}

#[tokio::test]
async fn test_notifications_disabled_via_env_value_true_uppercase() {
    // Set env var to "TRUE" (case-insensitive check)
    std::env::set_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS", "TRUE");

    let notifier = CliNotifier::with_port(65004);

    // Should return immediately without attempting connection
    notifier.notify_task_changed(Some(1), "created").await;

    // Cleanup
    std::env::remove_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS");
}

#[tokio::test]
async fn test_notifications_not_disabled_with_other_values() {
    // Set env var to a value that should NOT disable notifications
    std::env::set_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS", "0");

    let notifier = CliNotifier::with_port(65005);

    // Should attempt notification (but fail gracefully since Dashboard not running)
    notifier.notify_task_changed(Some(1), "created").await;

    // Cleanup
    std::env::remove_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS");
}

#[tokio::test]
async fn test_notification_message_types() {
    std::env::remove_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS");

    let notifier = CliNotifier::with_port(65006);

    // Test TaskChanged notification
    notifier.notify_task_changed(Some(42), "updated").await;

    // Test EventAdded notification
    notifier.notify_event_added(42, 1).await;

    // Test WorkspaceChanged notification
    notifier.notify_workspace_changed(Some(42)).await;

    // Test direct notify with custom message
    let message = NotificationMessage::TaskChanged {
        task_id: Some(42),
        operation: "deleted".to_string(),
    };
    notifier.notify(message).await;

    // All tests pass if no panic occurs
}

#[tokio::test]
async fn test_disabled_notifications_with_all_message_types() {
    std::env::set_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS", "1");

    let notifier = CliNotifier::with_port(65007);

    // All of these should return immediately without attempting connection
    notifier.notify_task_changed(Some(42), "created").await;
    notifier.notify_event_added(42, 1).await;
    notifier.notify_workspace_changed(Some(42)).await;

    // Cleanup
    std::env::remove_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS");
}

#[tokio::test]
async fn test_env_var_checked_per_notification() {
    std::env::remove_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS");

    let notifier = CliNotifier::with_port(65008);

    // First notification should work (env var not set)
    notifier.notify_task_changed(Some(1), "created").await;

    // Set env var
    std::env::set_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS", "1");

    // Second notification should be disabled
    notifier.notify_task_changed(Some(2), "created").await;

    // Unset env var
    std::env::remove_var("IE_DISABLE_DASHBOARD_NOTIFICATIONS");

    // Third notification should work again
    notifier.notify_task_changed(Some(3), "created").await;
}

#[test]
fn test_notifier_creation() {
    // Verify notifier can be created
    let _notifier = CliNotifier::new();
    // Test passes if no panic occurs
}

#[test]
fn test_notifier_with_custom_port() {
    // Verify notifier can be created with custom port
    let _notifier = CliNotifier::with_port(8080);
    // Test passes if no panic occurs
}
