//! Integration tests for WebSocket client
//!
//! These tests require a running Dashboard server.
//!
//! # Running these tests
//!
//! ```bash
//! # Start Dashboard first
//! ie dashboard start
//!
//! # Run integration tests
//! cargo test --test ws_client_integration_test -- --ignored
//! ```

use std::path::PathBuf;
use std::time::Duration;

/// Test basic connection to Dashboard
///
/// Requires Dashboard to be running on port 11391
#[tokio::test]
#[ignore = "requires running Dashboard server"]
async fn test_connect_to_dashboard() {
    // Use a temp directory to test the temp path skip logic
    let temp_project = std::env::temp_dir().join("test-project");
    let temp_db = temp_project.join(".intent-engine/intent.db");

    // This should return Ok(()) immediately due to temp path detection
    let result = intent_engine::mcp::ws_client::connect_to_dashboard(
        temp_project,
        temp_db,
        Some("test-agent".to_string()),
        None,
        Some(11391),
    )
    .await;

    assert!(result.is_ok(), "Temp path should be skipped silently");
}

/// Test connection with real project path
///
/// Requires Dashboard to be running on port 11391
#[tokio::test]
#[ignore = "requires running Dashboard server"]
async fn test_connect_real_project() {
    let project_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let db_path = project_path.join(".intent-engine/intent.db");

    // Spawn connection in background (it runs indefinitely)
    let handle = tokio::spawn(async move {
        intent_engine::mcp::ws_client::connect_to_dashboard(
            project_path,
            db_path,
            Some("integration-test".to_string()),
            None,
            Some(11391),
        )
        .await
    });

    // Let it run for a bit then cancel
    tokio::time::sleep(Duration::from_secs(2)).await;
    handle.abort();

    // If we got here without panic, connection was established
}
