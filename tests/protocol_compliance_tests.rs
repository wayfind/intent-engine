//! Protocol Compliance Tests
//!
//! These tests verify that the WebSocket implementation complies with
//! the Intent-Engine Protocol v1.0 specification (docs/INTENT_ENGINE_PROTOCOL.md).
//!
//! Tests are organized by protocol sections:
//! - T1: hello/welcome handshake (Section 4.1.1-4.1.2)
//! - T2: ping/pong heartbeat (Section 4.1.3)
//! - T3: reconnection with exponential backoff (Section 3.3)
//! - T4: Dashboard restart recovery (Section 5)
//! - T5: Multi-client broadcast (Section 4.3)

mod common;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Protocol version from specification
const PROTOCOL_VERSION: &str = "1.0";

/// Protocol message wrapper (matches spec Section 2.2)
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

    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Empty payload for messages without data
#[derive(Debug, Serialize, Deserialize)]
struct EmptyPayload {}

/// Hello message payload (Section 4.1.1)
#[derive(Debug, Serialize, Deserialize)]
struct HelloPayload {
    entity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<Vec<String>>,
}

/// Welcome message payload (Section 4.1.2)
#[derive(Debug, Serialize, Deserialize)]
struct WelcomePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<Vec<String>>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Setup async test environment with initialized project
async fn setup_async_test_env() -> tempfile::TempDir {
    use intent_engine::project::ProjectContext;
    use std::fs;

    let temp_dir = tempfile::TempDir::new().unwrap();

    // Create .git marker
    fs::create_dir(temp_dir.path().join(".git")).unwrap();

    // Initialize project
    ProjectContext::initialize_project_at(temp_dir.path().to_path_buf())
        .await
        .expect("Failed to initialize test project");

    // Verify database exists
    let db_path = temp_dir.path().join(".intent-engine").join("project.db");
    assert!(db_path.exists(), "Database was not created");

    temp_dir
}

/// Start Dashboard server in the background for testing
fn start_test_dashboard(project_path: &Path) -> Child {
    let stdout_path =
        std::env::temp_dir().join(format!("dashboard-protocol-{}.stdout", std::process::id()));
    let stderr_path =
        std::env::temp_dir().join(format!("dashboard-protocol-{}.stderr", std::process::id()));

    let stdout_file = std::fs::File::create(&stdout_path).expect("Failed to create stdout log");
    let stderr_file = std::fs::File::create(&stderr_path).expect("Failed to create stderr log");

    let child = Command::new(common::ie_binary())
        .arg("dashboard")
        .arg("start")
        .current_dir(project_path)
        .env("RUST_LOG", "debug")
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .expect("Failed to start Dashboard");

    eprintln!("Test Dashboard started with PID: {:?}", child.id());
    eprintln!("  stdout: {}", stdout_path.display());
    eprintln!("  stderr: {}", stderr_path.display());

    child
}

/// Wait for Dashboard to be ready (polling port 11391)
async fn wait_for_dashboard_ready() {
    for attempt in 0..30 {
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Try to connect to health endpoint
        match reqwest::get("http://127.0.0.1:11391/api/health").await {
            Ok(response) if response.status().is_success() => {
                eprintln!("Dashboard ready after {}ms", attempt * 100);
                return;
            },
            _ => continue,
        }
    }

    panic!("Dashboard did not become ready within 3 seconds");
}

/// Connect to Dashboard WebSocket and return the stream
async fn connect_to_dashboard_ws() -> (
    futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) {
    let url = "ws://127.0.0.1:11391/ws/mcp";
    let (ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to Dashboard WebSocket");

    ws_stream.split()
}

/// Receive and parse a protocol message
async fn recv_protocol_message<T: for<'de> Deserialize<'de>>(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) -> ProtocolMessage<T> {
    let msg = read
        .next()
        .await
        .expect("Stream ended")
        .expect("Failed to read message");

    match msg {
        Message::Text(text) => {
            serde_json::from_str(&text).expect("Failed to parse protocol message")
        },
        _ => panic!("Expected text message, got {:?}", msg),
    }
}

// ============================================================================
// T1: Hello/Welcome Handshake Test (Section 4.1.1-4.1.2)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_hello_welcome_handshake_mcp_client() {
    // Setup test environment
    let temp_dir = setup_async_test_env().await;
    let project_path = temp_dir.path();

    // Start Dashboard
    let mut dashboard = start_test_dashboard(project_path);
    wait_for_dashboard_ready().await;

    // Connect to Dashboard WebSocket
    let (mut write, mut read) = connect_to_dashboard_ws().await;

    // Step 1: Send hello message (MCP Server entity)
    let hello = ProtocolMessage::new(
        "hello",
        HelloPayload {
            entity_type: "mcp_server".to_string(),
            capabilities: Some(vec![]),
        },
    );

    write
        .send(Message::Text(hello.to_json().unwrap()))
        .await
        .expect("Failed to send hello");

    eprintln!("✓ Sent hello message");

    // Step 2: Expect welcome response
    let welcome: ProtocolMessage<WelcomePayload> = recv_protocol_message(&mut read).await;

    assert_eq!(welcome.message_type, "welcome", "Expected welcome message");
    assert_eq!(
        welcome.version, PROTOCOL_VERSION,
        "Protocol version mismatch"
    );

    // Session ID is optional but should be present
    if let Some(session_id) = &welcome.payload.session_id {
        assert!(!session_id.is_empty(), "Session ID should not be empty");
        eprintln!("✓ Received welcome with session_id: {}", session_id);
    } else {
        eprintln!("✓ Received welcome (no session_id)");
    }

    // Cleanup
    dashboard.kill().expect("Failed to kill Dashboard");
    let _ = dashboard.wait(); // Prevent zombie process
}

#[tokio::test]
#[serial]
async fn test_hello_welcome_handshake_web_ui_client() {
    // Setup test environment
    let temp_dir = setup_async_test_env().await;
    let project_path = temp_dir.path();

    // Start Dashboard
    let mut dashboard = start_test_dashboard(project_path);
    wait_for_dashboard_ready().await;

    // Connect to Dashboard WebSocket (UI endpoint)
    let url = "ws://127.0.0.1:11391/ws/ui";
    let (ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to Dashboard WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Step 1: Send hello message (Web UI entity)
    let hello = ProtocolMessage::new(
        "hello",
        HelloPayload {
            entity_type: "web_ui".to_string(),
            capabilities: None,
        },
    );

    write
        .send(Message::Text(hello.to_json().unwrap()))
        .await
        .expect("Failed to send hello");

    eprintln!("✓ Sent hello message (web_ui)");

    // Step 2: Expect welcome response
    let welcome: ProtocolMessage<WelcomePayload> = recv_protocol_message(&mut read).await;

    assert_eq!(welcome.message_type, "welcome", "Expected welcome message");
    assert_eq!(
        welcome.version, PROTOCOL_VERSION,
        "Protocol version mismatch"
    );

    eprintln!("✓ Received welcome message");

    // Cleanup
    dashboard.kill().expect("Failed to kill Dashboard");
    let _ = dashboard.wait(); // Prevent zombie process
}

// ============================================================================
// T2: Ping/Pong Heartbeat Test (Section 4.1.3)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_heartbeat_ping_pong() {
    // This test verifies:
    // 1. Dashboard sends ping within 35 seconds (30s interval + tolerance)
    // 2. Client responds with pong
    // 3. Connection remains alive

    // Setup test environment
    let temp_dir = setup_async_test_env().await;
    let project_path = temp_dir.path();

    // Start Dashboard
    let mut dashboard = start_test_dashboard(project_path);
    wait_for_dashboard_ready().await;

    // Connect to Dashboard WebSocket
    let (mut write, mut read) = connect_to_dashboard_ws().await;

    // Complete hello/welcome handshake
    let hello = ProtocolMessage::new(
        "hello",
        HelloPayload {
            entity_type: "mcp_server".to_string(),
            capabilities: Some(vec![]),
        },
    );

    write
        .send(Message::Text(hello.to_json().unwrap()))
        .await
        .expect("Failed to send hello");

    // Receive welcome
    let _welcome: ProtocolMessage<WelcomePayload> = recv_protocol_message(&mut read).await;
    eprintln!("✓ Handshake completed");

    // Wait for ping (Dashboard sends ping every 30s, allow up to 35s)
    eprintln!("⏱ Waiting for ping from Dashboard (max 35s)...");

    let ping_timeout = Duration::from_secs(35);
    let ping_result = tokio::time::timeout(ping_timeout, async {
        loop {
            let msg: ProtocolMessage<serde_json::Value> = recv_protocol_message(&mut read).await;
            if msg.message_type == "ping" {
                return msg;
            }
            eprintln!("  Skipping non-ping message: {}", msg.message_type);
        }
    })
    .await;

    match ping_result {
        Ok(ping) => {
            eprintln!("✓ Received ping from Dashboard");
            assert_eq!(ping.message_type, "ping", "Expected ping message");
            assert_eq!(ping.version, PROTOCOL_VERSION, "Protocol version mismatch");

            // Respond with pong
            let pong = ProtocolMessage::new("pong", EmptyPayload {});
            write
                .send(Message::Text(pong.to_json().unwrap()))
                .await
                .expect("Failed to send pong");

            eprintln!("✓ Sent pong response");

            // Verify connection remains alive by sending another message
            let hello2 = ProtocolMessage::new(
                "hello",
                HelloPayload {
                    entity_type: "mcp_server".to_string(),
                    capabilities: Some(vec![]),
                },
            );
            write
                .send(Message::Text(hello2.to_json().unwrap()))
                .await
                .expect("Connection should still be alive");

            eprintln!("✓ Connection remains alive after pong");
        },
        Err(_) => {
            panic!("Did not receive ping within 35 seconds");
        },
    }

    // Cleanup
    dashboard.kill().expect("Failed to kill Dashboard");
    let _ = dashboard.wait(); // Prevent zombie process
}

// ============================================================================
// T3: Reconnection with Exponential Backoff Test (Section 3.3)
// ============================================================================

#[tokio::test]
#[serial]
#[ignore] // TODO: Requires Mock MCP Client infrastructure
async fn test_reconnection_exponential_backoff() {
    // This test will verify:
    // 1. Client reconnects after disconnection
    // 2. Uses exponential backoff: [1s, 2s, 4s, 8s, 16s, 32s]
    // 3. Includes jitter (±25%)
    //
    // Implementation approach:
    // 1. Start Dashboard
    // 2. Connect MCP Client (via subprocess or mock)
    // 3. Kill Dashboard connection
    // 4. Measure reconnection delays
    // 5. Verify exponential pattern with jitter
    //
    // Note: Requires spawning actual MCP Client process or implementing Mock
    todo!("Requires Mock MCP Client infrastructure");
}

// ============================================================================
// T4: Dashboard Restart Recovery Test (Section 5)
// ============================================================================

#[tokio::test]
#[serial]
#[ignore] // TODO: Requires Dashboard restart infrastructure
async fn test_dashboard_restart_recovery() {
    // This test will verify:
    // 1. MCP Client registers with Dashboard
    // 2. Dashboard restarts (shutdown + start)
    // 3. MCP Client auto-reconnects and re-registers
    // 4. Project state is recovered
    //
    // Implementation approach:
    // 1. Start Dashboard #1
    // 2. Connect MCP Client (mock or subprocess)
    // 3. Verify registration
    // 4. Shutdown Dashboard #1
    // 5. Start Dashboard #2 (same port)
    // 6. Verify MCP Client reconnects and re-registers
    //
    // Note: Requires graceful Dashboard shutdown + restart logic
    todo!("Requires Dashboard restart infrastructure");
}

// ============================================================================
// T5: Multi Web UI Broadcast Test (Section 4.3)
// ============================================================================

#[tokio::test]
#[serial]
#[ignore] // TODO: Requires MCP registration message handling
async fn test_multi_web_ui_broadcast() {
    // This test will verify:
    // 1. Multiple Web UIs can connect simultaneously
    // 2. project_online is broadcasted to all UIs when MCP connects
    // 3. project_offline is broadcasted to all UIs when MCP disconnects
    //
    // Implementation approach:
    // 1. Start Dashboard
    // 2. Connect 3 Web UI clients
    // 3. Connect MCP Client (with project registration)
    // 4. Verify all 3 UIs receive project_online
    // 5. Disconnect MCP Client
    // 6. Verify all 3 UIs receive project_offline
    //
    // Note: Requires proper MCP registration flow implementation
    todo!("Requires MCP registration message handling");
}
