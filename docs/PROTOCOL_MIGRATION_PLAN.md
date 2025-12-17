# Protocol Migration Plan: IEP v1.0 Compliance

**Date**: 2025-11-23
**Target**: Full IEP v1.0 compliance
**Estimated Timeline**: 2-3 weeks

---

## Overview

This document outlines a **phased, testable migration plan** to bring Intent-Engine into full compliance with the Intent-Engine Protocol (IEP) v1.0.

**Key Principles**:
- ✅ **Incremental**: Each phase is independently testable
- ✅ **Backward Compatible**: Phases 1-2 don't break existing functionality
- ✅ **Test-Driven**: Write tests first, then implement
- ✅ **Protocol-First**: Code must conform to spec, not vice versa

---

## Phase 0: Quick Wins (Day 1, 2-3 hours)

These changes are low-risk, high-value improvements that can be done immediately.

### 0.1 Web UI: Remove Max Reconnection Attempts

**File**: `static/js/app.js`

**Current** (line 11):
```javascript
const WS_MAX_RECONNECT_ATTEMPTS = 10;
```

**Change to**:
```javascript
const WS_MAX_RECONNECT_ATTEMPTS = Infinity; // Protocol: unlimited retries
```

**Also remove** (lines 281, 287, 292):
```javascript
if (wsReconnectAttempts < WS_MAX_RECONNECT_ATTEMPTS) {
    // ...
} else {
    console.error('✗ Maximum reconnection attempts reached...');
}
```

**Change to**:
```javascript
// Always reconnect (unlimited retries per protocol)
const delayIndex = Math.min(wsReconnectAttempts, WS_RECONNECT_DELAYS.length - 1);
const delay = WS_RECONNECT_DELAYS[delayIndex];
console.log(`⟳ Reconnecting in ${delay/1000}s... (attempt ${wsReconnectAttempts + 1})`);

wsReconnectAttempts++;
setTimeout(connectToDashboardWebSocket, delay);
```

---

### 0.2 Web UI: Add Jitter to Backoff

**File**: `static/js/app.js`

**Current** (line 283):
```javascript
const delay = WS_RECONNECT_DELAYS[delayIndex];
```

**Change to**:
```javascript
const baseDelay = WS_RECONNECT_DELAYS[delayIndex];
const jitter = Math.random() * 1000; // 0-1000ms jitter per protocol
const delay = baseDelay + jitter;
```

---

### 0.3 Web UI: Fix Heartbeat Timeout

**File**: `static/js/app.js`

**Current** (line 14):
```javascript
const WS_HEARTBEAT_TIMEOUT = 60000; // 60 seconds
```

**Change to**:
```javascript
const WS_HEARTBEAT_TIMEOUT = 90000; // 90 seconds (protocol: 3 missed pings)
```

---

### 0.4 Web UI: Align Backoff Delays with Protocol

**File**: `static/js/app.js`

**Current** (line 12):
```javascript
const WS_RECONNECT_DELAYS = [1000, 2000, 5000, 10000, 30000]; // ms
```

**Change to**:
```javascript
const WS_RECONNECT_DELAYS = [1000, 2000, 4000, 8000, 16000, 32000]; // Protocol: 1,2,4,8,16,32s
```

---

### 0.5 Add Protocol Version Constant

**Files**:
- `src/dashboard/websocket.rs`
- `src/mcp/ws_client.rs`
- `static/js/app.js`

**Add constant**:
```rust
// Rust
const PROTOCOL_VERSION: &str = "1.0";
```

```javascript
// JavaScript
const PROTOCOL_VERSION = "1.0";
```

**Test**: Verify constants exist (no behavior change yet).

---

### Phase 0 Acceptance Criteria

- [ ] Web UI reconnects indefinitely
- [ ] Backoff delays match protocol: 1,2,4,8,16,32s
- [ ] Jitter adds 0-1000ms randomness
- [ ] Heartbeat timeout is 90s
- [ ] Protocol version constant defined in all files

---

## Phase 1: Protocol Message Wrapper (Days 2-4, 12-16 hours)

### Objective

Wrap all messages in protocol-compliant format:
```json
{
  "version": "1.0",
  "type": "message_type",
  "payload": { ... },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

---

### 1.1 Create Base Message Struct (Rust)

**New File**: `src/protocol/mod.rs`

```rust
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub version: String,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    pub timestamp: String, // ISO 8601
}

impl ProtocolMessage {
    pub fn new(msg_type: impl Into<String>, payload: Option<Value>) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_string(),
            r#type: msg_type.into(),
            payload,
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    pub fn parse_payload<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        match &self.payload {
            Some(p) => serde_json::from_value(p.clone()),
            None => Err(serde_json::Error::custom("No payload")),
        }
    }
}

// Helper macros for creating messages
#[macro_export]
macro_rules! protocol_msg {
    ($type:expr) => {
        ProtocolMessage::new($type, None)
    };
    ($type:expr, $payload:expr) => {
        ProtocolMessage::new($type, Some(serde_json::to_value($payload)?))
    };
}
```

**Add to `src/lib.rs`**:
```rust
pub mod protocol;
```

---

### 1.2 Update Dashboard Message Types

**File**: `src/dashboard/websocket.rs`

**Replace** (lines 100-132):
```rust
/// Message types from MCP to Dashboard
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum McpMessage {
    #[serde(rename = "register")]
    Register { project: ProjectInfo },
    #[serde(rename = "ping")]
    Ping,
}
```

**With**:
```rust
use crate::protocol::{ProtocolMessage, PROTOCOL_VERSION};

/// Payload for register message
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterPayload {
    pub project: ProjectInfo,
}

/// Payload for registered message
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisteredPayload {
    pub success: bool,
    pub project_path: String,
}

// Remove old enums, use ProtocolMessage directly
```

**Update message handling** (line 187):
```rust
match serde_json::from_str::<ProtocolMessage>(&text) {
    Ok(msg) => {
        // Check version
        if !is_compatible_version(&msg.version) {
            let error = protocol_msg!("error", ErrorPayload {
                code: "unsupported_version".to_string(),
                message: format!("Version {} not supported", msg.version),
                details: None,
            });
            let _ = tx.send(Message::Text(serde_json::to_string(&error)?));
            continue;
        }

        match msg.r#type.as_str() {
            "register" => {
                let payload: RegisterPayload = msg.parse_payload()?;
                handle_register(payload.project, &state, &tx).await;
            },
            "ping" => {
                let pong = protocol_msg!("pong");
                let _ = tx.send(Message::Text(serde_json::to_string(&pong)?));
            },
            _ => {
                tracing::warn!("Unknown message type: {}", msg.r#type);
            }
        }
    },
    Err(e) => {
        tracing::warn!("Failed to parse protocol message: {}", e);
        let error = protocol_msg!("error", ErrorPayload {
            code: "invalid_message".to_string(),
            message: e.to_string(),
            details: None,
        });
        let _ = tx.send(Message::Text(serde_json::to_string(&error)?));
    }
}
```

---

### 1.3 Update MCP Client

**File**: `src/mcp/ws_client.rs`

**Replace message enums** with:
```rust
use crate::protocol::{ProtocolMessage, protocol_msg};

// Remove McpMessage enum

// Send register message (line 93-101)
let register = protocol_msg!("register", RegisterPayload {
    project: project_info.clone(),
});
write
    .send(Message::Text(serde_json::to_string(&register)?))
    .await?;

// Parse responses
match serde_json::from_str::<ProtocolMessage>(&text) {
    Ok(msg) if msg.r#type == "registered" => {
        let payload: RegisteredPayload = msg.parse_payload()?;
        if payload.success {
            tracing::debug!("Successfully registered");
        } else {
            anyhow::bail!("Registration rejected");
        }
    },
    Ok(msg) if msg.r#type == "pong" => {
        tracing::debug!("Received pong");
    },
    Ok(msg) if msg.r#type == "error" => {
        let payload: ErrorPayload = msg.parse_payload()?;
        tracing::error!("Protocol error: {} - {}", payload.code, payload.message);
    },
    _ => {
        tracing::warn!("Unknown message: {}", text);
    }
}
```

---

### 1.4 Update Web UI

**File**: `static/js/app.js`

**Create helper function**:
```javascript
function createProtocolMessage(type, payload = null) {
    return {
        version: PROTOCOL_VERSION,
        type: type,
        payload: payload,
        timestamp: new Date().toISOString()
    };
}

function parseProtocolMessage(data) {
    const msg = JSON.parse(data);

    // Version check
    if (msg.version && !isCompatibleVersion(msg.version)) {
        console.error('Incompatible protocol version:', msg.version);
        return null;
    }

    return msg;
}

function isCompatibleVersion(version) {
    const [major] = version.split('.');
    return major === '1'; // Accept any 1.x
}
```

**Update message sending** (line 388):
```javascript
const pong = createProtocolMessage('pong');
dashboardWebSocket.send(JSON.stringify(pong));
```

**Update message parsing** (line 256):
```javascript
const msg = parseProtocolMessage(event.data);
if (!msg) return;

switch (msg.type) {
    case 'init':
        handleInitMessage(msg.payload.projects);
        break;
    case 'project_online':
        handleProjectOnline(msg.payload.project);
        break;
    case 'project_offline':
        handleProjectOffline(msg.payload.project_path);
        break;
    case 'ping':
        const pong = createProtocolMessage('pong');
        dashboardWebSocket.send(JSON.stringify(pong));
        break;
    case 'error':
        console.error('Protocol error:', msg.payload);
        break;
    default:
        console.warn('Unknown message type:', msg.type);
}
```

---

### Phase 1 Acceptance Criteria

- [ ] All messages have `version`, `type`, `payload`, `timestamp` fields
- [ ] Dashboard validates protocol version
- [ ] MCP validates protocol version
- [ ] Web UI validates protocol version
- [ ] Incompatible versions rejected with `error` message
- [ ] All existing functionality still works
- [ ] Unit tests pass for message serialization

---

## Phase 2: Connection Layer (Days 5-8, 16-20 hours)

### Objective

Implement full connection handshake and lifecycle management.

---

### 2.1 Add `hello` / `welcome` Handshake

**Dashboard** (`src/dashboard/websocket.rs`):

```rust
// State tracking per connection
enum ConnectionState {
    WaitingHello,
    Connected { entity_type: String, entity_id: String },
}

// Handle hello message
"hello" => {
    let payload: HelloPayload = msg.parse_payload()?;

    // Validate entity type
    if !["web_ui", "mcp_server"].contains(&payload.entity_type.as_str()) {
        let error = protocol_msg!("error", ErrorPayload {
            code: "invalid_entity_type".to_string(),
            message: format!("Unknown entity type: {}", payload.entity_type),
            details: None,
        });
        let _ = tx.send(Message::Text(serde_json::to_string(&error)?));
        continue;
    }

    // Send welcome
    let welcome = protocol_msg!("welcome", WelcomePayload {
        session_id: uuid::Uuid::new_v4().to_string(),
        server_time: chrono::Utc::now().to_rfc3339(),
    });
    let _ = tx.send(Message::Text(serde_json::to_string(&welcome)?));

    // Update connection state
    connection_state = ConnectionState::Connected {
        entity_type: payload.entity_type,
        entity_id: payload.entity_id,
    };
},
```

**MCP Client** (`src/mcp/ws_client.rs`):

```rust
// Send hello immediately after connection
let hello = protocol_msg!("hello", HelloPayload {
    entity_type: "mcp_server".to_string(),
    entity_id: normalized_project_path.to_string_lossy().to_string(),
    capabilities: vec![],
});
write.send(Message::Text(serde_json::to_string(&hello)?)).await?;

// Wait for welcome
match read.next().await {
    Some(Ok(Message::Text(text))) => {
        let msg: ProtocolMessage = serde_json::from_str(&text)?;
        if msg.r#type == "welcome" {
            tracing::debug!("Received welcome from Dashboard");
            // Now send register
        } else {
            anyhow::bail!("Expected welcome, got: {}", msg.r#type);
        }
    },
    _ => anyhow::bail!("Connection closed during handshake"),
}
```

**Web UI** (`static/js/app.js`):

```javascript
dashboardWebSocket.onopen = async () => {
    console.log('✓ WebSocket connected, sending hello...');

    const hello = createProtocolMessage('hello', {
        entity_type: 'web_ui',
        entity_id: generateClientId(), // UUID for this tab
        capabilities: ['real_time_updates']
    });

    dashboardWebSocket.send(JSON.stringify(hello));

    // Wait for welcome before proceeding
    // (handle in onmessage)
};

function handleDashboardMessage(msg) {
    switch (msg.type) {
        case 'welcome':
            console.log('✓ Received welcome, session:', msg.payload.session_id);
            wsReconnectAttempts = 0;
            onWelcomeReceived(); // Fetch project info, etc.
            break;
        // ... rest
    }
}
```

---

### 2.2 Add `goodbye` Message

**MCP Client** (`src/mcp/ws_client.rs`):

```rust
// Add shutdown handler
pub async fn shutdown(&self) {
    let goodbye = protocol_msg!("goodbye", GoodbyePayload {
        reason: "shutdown".to_string(),
    });

    if let Some(write) = &mut self.write {
        let _ = write.send(Message::Text(serde_json::to_string(&goodbye).unwrap())).await;
    }
}

// Call on SIGTERM/SIGINT
// (integrate with existing shutdown logic in server.rs)
```

**Dashboard**:

```rust
"goodbye" => {
    let payload: GoodbyePayload = msg.parse_payload()?;
    tracing::info!("Client goodbye: {}", payload.reason);
    // Clean shutdown (no need to wait for timeout)
    break;
},
```

**Web UI**:

```javascript
window.addEventListener('beforeunload', () => {
    if (dashboardWebSocket && dashboardWebSocket.readyState === WebSocket.OPEN) {
        const goodbye = createProtocolMessage('goodbye', {
            reason: 'page_unload'
        });
        dashboardWebSocket.send(JSON.stringify(goodbye));
    }
});
```

---

### 2.3 Add Heartbeat Timeout Tracking

**Dashboard** (`src/dashboard/websocket.rs`):

```rust
// Track last activity timestamp
let mut last_activity = tokio::time::Instant::now();

// In heartbeat task, check timeout
let mut heartbeat_task = tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;

        // Check if client is alive (90s timeout = 3 missed pongs)
        if last_activity.elapsed() > Duration::from_secs(90) {
            tracing::warn!("Client heartbeat timeout - no activity for 90s");
            break; // Close connection
        }

        // Send ping
        let ping = protocol_msg!("ping");
        if tx.send(Message::Text(serde_json::to_string(&ping).unwrap())).is_err() {
            break;
        }
    }
});

// Update last_activity on any received message
match receiver.next().await {
    Some(Ok(msg)) => {
        last_activity = tokio::time::Instant::now(); // Reset timeout
        // ... handle message
    }
}
```

---

### Phase 2 Acceptance Criteria

- [ ] All clients send `hello` before other messages
- [ ] Dashboard responds with `welcome`
- [ ] Version mismatch rejected in `hello`
- [ ] `goodbye` sent on clean shutdown
- [ ] Dashboard detects heartbeat timeout (90s)
- [ ] MCP detects heartbeat timeout (90s)
- [ ] Manual test: Dashboard restart → MCP/UI reconnect → handshake works

---

## Phase 3: MCP Reconnection (Days 9-12, CRITICAL)

### Objective

Implement full state machine and reconnection loop in MCP Server.

---

### 3.1 Refactor MCP Client to Loop

**File**: `src/mcp/ws_client.rs`

**Current structure** (problematic):
```rust
pub async fn connect_to_dashboard(...) -> Result<()> {
    let (ws, _) = connect_async(url).await?;
    let (write, read) = ws.split();

    // Spawn tasks and EXIT
    tokio::spawn(async move { /* ping loop */ });
    tokio::spawn(async move { /* read loop */ });

    Ok(()) // Function returns, tasks run in background
}
```

**Problem**: If connection lost, tasks die, no retry.

**New structure** (protocol compliant):
```rust
pub async fn connect_to_dashboard_with_retry(
    project_path: PathBuf,
    db_path: PathBuf,
    agent: Option<String>,
) -> Result<()> {
    let mut retry_count = 0;
    let delays = [1, 2, 4, 8, 16, 32]; // seconds

    loop {
        match connect_and_run(&project_path, &db_path, &agent).await {
            Ok(()) => {
                // Connection closed gracefully, retry
                tracing::info!("Dashboard connection closed, reconnecting...");
                retry_count = 0; // Reset on graceful close
            },
            Err(e) => {
                // Connection failed, backoff and retry
                if retry_count < delays.len() {
                    let delay = delays[retry_count];
                    let jitter = rand::random::<u64>() % 1000; // 0-1000ms
                    let total_delay = Duration::from_secs(delay) + Duration::from_millis(jitter);

                    tracing::warn!(
                        "Dashboard connection error: {}. Retrying in {:?}... (attempt {})",
                        e, total_delay, retry_count + 1
                    );

                    tokio::time::sleep(total_delay).await;
                    retry_count += 1;
                } else {
                    // Max backoff reached, keep retrying at 32s
                    let jitter = rand::random::<u64>() % 1000;
                    let total_delay = Duration::from_secs(32) + Duration::from_millis(jitter);

                    tracing::warn!(
                        "Dashboard connection error: {}. Retrying in {:?}...",
                        e, total_delay
                    );

                    tokio::time::sleep(total_delay).await;
                }
            }
        }
    }
}

async fn connect_and_run(
    project_path: &PathBuf,
    db_path: &PathBuf,
    agent: &Option<String>,
) -> Result<()> {
    // Validate temp directory
    // ... (existing code)

    // Connect
    let url = "ws://127.0.0.1:11391/ws/mcp";
    let (ws_stream, _) = connect_async(url).await
        .context("Failed to connect to Dashboard")?;

    let (mut write, mut read) = ws_stream.split();

    // Send hello
    let hello = protocol_msg!("hello", HelloPayload {
        entity_type: "mcp_server".to_string(),
        entity_id: project_path.to_string_lossy().to_string(),
        capabilities: vec![],
    });
    write.send(Message::Text(serde_json::to_string(&hello)?)).await?;

    // Wait for welcome
    match read.next().await {
        Some(Ok(Message::Text(text))) => {
            let msg: ProtocolMessage = serde_json::from_str(&text)?;
            if msg.r#type != "welcome" {
                anyhow::bail!("Expected welcome, got {}", msg.r#type);
            }
        },
        _ => anyhow::bail!("No welcome received"),
    }

    // Send register
    let register = protocol_msg!("register", RegisterPayload {
        project: project_info.clone(),
    });
    write.send(Message::Text(serde_json::to_string(&register)?)).await?;

    // Wait for registered
    match read.next().await {
        Some(Ok(Message::Text(text))) => {
            let msg: ProtocolMessage = serde_json::from_str(&text)?;
            if msg.r#type != "registered" {
                anyhow::bail!("Expected registered, got {}", msg.r#type);
            }
            let payload: RegisteredPayload = msg.parse_payload()?;
            if !payload.success {
                anyhow::bail!("Registration rejected");
            }
        },
        _ => anyhow::bail!("No registered response"),
    }

    tracing::info!("✓ Successfully registered with Dashboard");

    // Now run connection (ping + read loops)
    run_connection(write, read).await
}

async fn run_connection(
    mut write: impl SinkExt<Message> + Unpin,
    mut read: impl StreamExt<Item = Result<Message, tungstenite::Error>> + Unpin,
) -> Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Spawn ping task
    let ping_tx = tx.clone();
    let ping_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let ping = protocol_msg!("ping");
            if ping_tx.send(ping).is_err() {
                break;
            }
        }
    });

    // Spawn send task
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            if write.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Main read loop (foreground, not spawned!)
    let mut last_activity = tokio::time::Instant::now();

    while let Some(result) = read.next().await {
        match result {
            Ok(Message::Text(text)) => {
                last_activity = tokio::time::Instant::now();

                let msg: ProtocolMessage = serde_json::from_str(&text)?;
                match msg.r#type.as_str() {
                    "pong" => {
                        tracing::trace!("Received pong");
                    },
                    "ping" => {
                        // Respond to server ping
                        let pong = protocol_msg!("pong");
                        tx.send(pong)?;
                    },
                    "goodbye" => {
                        tracing::info!("Dashboard sent goodbye");
                        break; // Clean shutdown
                    },
                    _ => {
                        tracing::debug!("Received: {}", msg.r#type);
                    }
                }
            },
            Ok(Message::Close(_)) => {
                tracing::info!("Dashboard closed connection");
                break;
            },
            Err(e) => {
                tracing::warn!("WebSocket error: {}", e);
                break;
            },
            _ => {}
        }

        // Check heartbeat timeout
        if last_activity.elapsed() > Duration::from_secs(90) {
            tracing::warn!("Heartbeat timeout - no message for 90s");
            break;
        }
    }

    // Cleanup tasks
    ping_task.abort();
    send_task.abort();

    Ok(())
}
```

---

### 3.2 Update MCP Server Startup

**File**: `src/mcp/server.rs`

**Replace** (line 104-121):
```rust
// Spawn Dashboard registration task
tokio::spawn(async move {
    if let Err(e) = ws_client::connect_to_dashboard(...).await {
        tracing::warn!("Failed to connect to Dashboard: {}", e);
    }
});
```

**With**:
```rust
// Spawn Dashboard connection task (runs forever with retry)
tokio::spawn(async move {
    ws_client::connect_to_dashboard_with_retry(...).await
        .expect("Dashboard connection loop exited");
});
```

---

### Phase 3 Acceptance Criteria

- [ ] MCP connects to Dashboard on startup
- [ ] MCP reconnects if Dashboard restarts
- [ ] MCP reconnects if network interrupted
- [ ] Reconnection uses exponential backoff (1,2,4,8,16,32s + jitter)
- [ ] Reconnection is unlimited (never gives up)
- [ ] Manual test: Kill Dashboard → restart → MCP reconnects within 32s
- [ ] Manual test: Restart MCP → reconnects immediately

---

## Phase 4: Remove Registry Dependency (Days 13-15)

### Objective

Dashboard uses only in-memory state, no Registry file.

---

### 4.1 Update `get_online_projects()`

**File**: `src/dashboard/websocket.rs`

**Replace** (lines 75-97):
```rust
pub async fn get_online_projects(&self) -> Vec<ProjectInfo> {
    match crate::dashboard::registry::ProjectRegistry::load() {
        // ... reads from file
    }
}
```

**With**:
```rust
pub async fn get_online_projects(&self) -> Vec<ProjectInfo> {
    let connections = self.mcp_connections.read().await;
    connections
        .values()
        .map(|conn| conn.project.clone())
        .collect()
}
```

---

### 4.2 Remove Registry Writes

**File**: `src/dashboard/websocket.rs`

**Remove** (lines 233-254):
```rust
// Update Registry immediately to set mcp_connected=true
match crate::dashboard::registry::ProjectRegistry::load() {
    Ok(mut registry) => {
        registry.register_mcp_connection(...)?;
        // ...
    }
}
```

**Remove** (lines 303-316):
```rust
// Update Registry immediately to set mcp_connected=false
match crate::dashboard::registry::ProjectRegistry::load() {
    // ...
}
```

---

### 4.3 Make Registry Optional (Backward Compatibility)

**Add environment variable**:
```rust
const ENABLE_REGISTRY: bool = std::env::var("INTENT_ENGINE_ENABLE_REGISTRY").is_ok();

// Wrap all Registry calls
if ENABLE_REGISTRY {
    if let Err(e) = registry.save() {
        tracing::warn!("Failed to save Registry: {}", e);
    }
}
```

**Update documentation**:
- Registry is deprecated
- Set `INTENT_ENGINE_ENABLE_REGISTRY=1` for legacy behavior
- Will be removed in v2.0

---

### Phase 4 Acceptance Criteria

- [ ] Dashboard starts with empty project list
- [ ] MCP connects → project appears in list
- [ ] MCP disconnects → project disappears from list
- [ ] Web UI refresh → receives current online projects from memory
- [ ] Manual test: Restart Dashboard → project list empty → MCP reconnects → list restored
- [ ] No Registry file reads/writes (unless env var set)

---

## Phase 5: Testing & Validation (Days 16-20)

### Objective

Comprehensive protocol compliance testing.

---

### 5.1 Protocol Compliance Tests

**New file**: `tests/protocol/compliance.rs`

```rust
#[tokio::test]
async fn test_hello_welcome_handshake() {
    // Start mock Dashboard
    let dashboard = MockDashboard::new().await;

    // Connect client
    let client = TestClient::connect(dashboard.url()).await;

    // Client sends hello
    client.send_hello("test_entity", "test_id").await;

    // Client receives welcome
    let welcome = client.expect_welcome().await;
    assert!(welcome.session_id.len() > 0);
}

#[tokio::test]
async fn test_version_negotiation() {
    let dashboard = MockDashboard::new().await;
    let client = TestClient::connect(dashboard.url()).await;

    // Send unsupported version
    client.send_hello_with_version("99.0").await;

    // Expect error
    let error = client.expect_error().await;
    assert_eq!(error.code, "unsupported_version");
}

#[tokio::test]
async fn test_mcp_reconnection() {
    let dashboard = MockDashboard::new().await;
    let mcp = McpClient::start(dashboard.url()).await;

    // Wait for connection
    dashboard.expect_register("test_project").await;

    // Kill Dashboard
    dashboard.shutdown().await;

    // Restart Dashboard
    let dashboard2 = MockDashboard::new_at_same_port(dashboard.port()).await;

    // MCP should reconnect within 5 seconds
    dashboard2.expect_register_within("test_project", Duration::from_secs(5)).await;
}

#[tokio::test]
async fn test_heartbeat_timeout() {
    let dashboard = MockDashboard::new().await;
    let mcp = McpClient::start(dashboard.url()).await;

    // Stop sending pongs
    dashboard.stop_pong_responses().await;

    // MCP should detect timeout within 90s + margin
    tokio::time::sleep(Duration::from_secs(95)).await;

    assert!(mcp.is_disconnected());
}
```

---

### 5.2 Integration Tests

**Test scenarios**:
1. Dashboard starts → MCP starts → Web UI opens → all connected
2. Dashboard restarts → MCP reconnects → Web UI reconnects
3. MCP restarts → Dashboard detects disconnect → MCP re-registers
4. Network interruption simulation (close socket) → auto-recovery
5. Version mismatch → graceful rejection with error message

---

### 5.3 Manual Test Checklist

**Dashboard Restart**:
- [ ] Start Dashboard
- [ ] Start MCP Server (registers successfully)
- [ ] Open Web UI (sees project online)
- [ ] Kill Dashboard
- [ ] Web UI shows "reconnecting" banner
- [ ] MCP logs "connection lost, retrying..."
- [ ] Restart Dashboard
- [ ] MCP reconnects within 32s
- [ ] Web UI reconnects
- [ ] Web UI shows project online again

**MCP Restart**:
- [ ] Dashboard + MCP + UI all running
- [ ] Kill MCP Server
- [ ] Dashboard logs "MCP disconnected"
- [ ] Web UI shows project offline (gray)
- [ ] Restart MCP Server
- [ ] MCP reconnects immediately
- [ ] Web UI shows project online (green)

**Network Failure**:
- [ ] All components running
- [ ] Simulate network partition (firewall/iptables)
- [ ] Both MCP and UI enter reconnection loop
- [ ] Restore network
- [ ] Both reconnect within 32s
- [ ] State fully restored

---

### Phase 5 Acceptance Criteria

- [ ] All protocol compliance tests pass
- [ ] All integration tests pass
- [ ] Manual test checklist 100% passed
- [ ] No Registry file created during tests
- [ ] Logs show proper protocol version in all messages

---

## Post-Migration: Documentation

### Update Docs

- [x] `docs/INTENT_ENGINE_PROTOCOL.md` - Protocol spec (already done)
- [x] `docs/PROTOCOL_GAP_ANALYSIS.md` - Gap analysis (already done)
- [x] `docs/PROTOCOL_MIGRATION_PLAN.md` - This document (in progress)
- [ ] `docs/TESTING_GUIDE.md` - How to run protocol tests
- [ ] `CLAUDE.md` - Update with protocol compliance status
- [ ] `README.md` - Mention protocol-first design

---

## Risk Mitigation

### Rollback Plan

Each phase is independently testable. If issues arise:

1. **Phase 1**: Revert message format changes (git revert)
2. **Phase 2**: Disable hello/welcome (feature flag)
3. **Phase 3**: Keep old MCP client (conditional compilation)
4. **Phase 4**: Re-enable Registry (env var already in place)

### Backward Compatibility

- Phases 1-2: Existing functionality unchanged
- Phase 3: MCP reconnection is addition, not breaking change
- Phase 4: Registry can be re-enabled with env var

---

## Timeline Summary

| Phase | Duration | Effort | Status |
|-------|----------|--------|--------|
| Phase 0: Quick Wins | Day 1 | 2-3h | Ready |
| Phase 1: Message Format | Days 2-4 | 12-16h | Planned |
| Phase 2: Connection Layer | Days 5-8 | 16-20h | Planned |
| Phase 3: MCP Reconnection | Days 9-12 | 16-20h | Planned |
| Phase 4: Remove Registry | Days 13-15 | 8-12h | Planned |
| Phase 5: Testing | Days 16-20 | 16-24h | Planned |

**Total**: 15-20 days (70-95 hours)

---

## Success Metrics

### Technical Metrics

- [ ] 100% message format compliance
- [ ] 100% protocol compliance tests pass
- [ ] 0 Registry file reads/writes (default)
- [ ] Dashboard restart recovery < 32s
- [ ] MCP restart recovery < 5s

### Quality Metrics

- [ ] No breaking changes to existing features
- [ ] All existing tests still pass
- [ ] Code coverage > 80% for protocol code
- [ ] Zero production incidents during rollout

---

## Next Steps

1. **Review this plan** with user
2. **Approve Phase 0** (quick wins)
3. **Start Phase 1** (message format wrapper)
4. **Daily standups** to track progress
5. **Adjust timeline** based on actual velocity

---

**Document Version**: 1.0
**Last Updated**: 2025-11-23
