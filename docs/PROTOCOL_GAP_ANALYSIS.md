# Protocol Gap Analysis: Current Implementation vs IEP v1.0

**Date**: 2025-11-23
**Protocol Version**: 1.0
**Status**: Work in Progress

---

## Executive Summary

This document analyzes the gap between the current Intent-Engine WebSocket implementation and the Intent-Engine Protocol (IEP) v1.0 specification.

**Overall Assessment**:
- ✅ **60% Compliant** - Core messaging and endpoints exist
- 🔧 **30% Needs Enhancement** - Missing hello/welcome, version negotiation, error handling
- 📋 **10% Not Implemented** - Goodbye, full state machine, compliance tests

---

## 1. Message Format Compliance

### Protocol Requirement (Section 2.2)

```json
{
  "version": "1.0",
  "type": "message_type",
  "payload": { ... },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

### Current Implementation

**Dashboard → MCP** (`src/dashboard/websocket.rs:111-118`):
```rust
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum McpResponse {
    Registered { success: bool },
    Pong,
}
```

**MCP → Dashboard** (`src/mcp/ws_client.rs:22-29`):
```rust
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum McpMessage {
    Register { project: ProjectInfo },
    Ping,
}
```

**Dashboard → UI** (`src/dashboard/websocket.rs:120-132`):
```rust
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum UiMessage {
    Init { projects: Vec<ProjectInfo> },
    ProjectOnline { project: ProjectInfo },
    ProjectOffline { project_path: String },
    Ping,
}
```

### Gap Analysis

| Field | Protocol | Current | Status |
|-------|----------|---------|--------|
| `version` | Required | ❌ Missing | 🔧 Need to add |
| `type` | Required | ✅ Present | ✅ Compliant |
| `payload` | Optional | ⚠️ Inline | 🔧 Need to wrap |
| `timestamp` | Required | ❌ Missing | 🔧 Need to add |

**Issue**: Current messages use `#[serde(tag = "type")]` which inlines the payload, not nesting it under a `payload` field.

**Example - Current**:
```json
{
  "type": "register",
  "project": { "path": "...", "name": "..." }
}
```

**Example - Protocol Compliant**:
```json
{
  "version": "1.0",
  "type": "register",
  "payload": {
    "project": { "path": "...", "name": "..." }
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

**Action Required**:
1. Create base message wrapper struct:
   ```rust
   #[derive(Serialize, Deserialize)]
   struct ProtocolMessage {
       version: String,
       r#type: String,
       #[serde(skip_serializing_if = "Option::is_none")]
       payload: Option<serde_json::Value>,
       timestamp: String, // ISO 8601
   }
   ```
2. Wrap all existing messages
3. Update all serialization/deserialization code

---

## 2. Connection Layer Messages

### 2.1 `hello` / `welcome` (Section 4.1.1 & 4.1.2)

**Protocol Requirement**: Clients send `hello` immediately after connection, server responds with `welcome`.

**Current Implementation**: ❌ **Not Implemented**

**Current Behavior**:
- MCP Server: Connects → immediately sends `register` (no hello)
  - `src/mcp/ws_client.rs:93-101`
- Dashboard: Receives `register` directly (no welcome check)
  - `src/dashboard/websocket.rs:188-267`
- Web UI: Connects → waits for `init` (no hello handshake)
  - `static/js/app.js:186-249`

**Gap**: No handshake protocol, no entity identification, no capability negotiation.

**Action Required**:
1. Add `hello` message type to all clients (MCP, Web UI)
2. Add `welcome` response to Dashboard
3. Implement handshake sequence:
   ```
   Client                    Server
   │─connect()─────────────►│
   │◄─(WebSocket open)──────┤
   │─hello──────────────────►│
   │◄─welcome────────────────┤
   │─(application messages)─►│
   ```

---

### 2.2 `ping` / `pong` (Section 4.1.3)

**Protocol Requirement**: 30-second heartbeat, 90-second timeout (3 missed pings).

**Current Implementation**: ✅ **Partially Compliant**

**Dashboard → MCP** (`src/dashboard/websocket.rs:164-179`):
- ✅ Sends ping every 30s (uses `Pong` message - naming inconsistency)
- ❌ No timeout mechanism
- ✅ Responds to MCP's `Ping` with `Pong` (269-274)

**MCP → Dashboard** (`src/mcp/ws_client.rs:118-134`):
- ✅ Sends `Ping` every 30s
- ❌ No timeout detection
- ❌ Does not handle connection loss

**Dashboard → UI** (`src/dashboard/websocket.rs:376-391`):
- ✅ Sends `Ping` every 30s
- ❌ No timeout mechanism

**Web UI → Dashboard** (`static/js/app.js:299-312, 384-390`):
- ✅ Responds to `ping` with `pong`
- ✅ Has heartbeat timeout (60s) - but should be 90s per protocol
- ⚠️ Timeout triggers close, not reconnect

**Gap Analysis**:

| Feature | Protocol | Dashboard | MCP | Web UI |
|---------|----------|-----------|-----|--------|
| 30s interval | Required | ✅ | ✅ | N/A (server-driven) |
| 90s timeout | Required | ❌ | ❌ | ⚠️ 60s |
| Bidirectional | Required | ✅ | ✅ | ✅ |

**Action Required**:
1. Dashboard: Add 90s timeout tracking for both MCP and UI connections
2. MCP: Add 90s timeout detection
3. Web UI: Change timeout from 60s → 90s
4. Fix naming: Dashboard should send `Ping`, not `Pong`

---

### 2.3 `goodbye` (Section 4.1.4)

**Protocol Requirement**: Send before graceful shutdown.

**Current Implementation**: ❌ **Not Implemented**

**Current Behavior**:
- All entities just close WebSocket without notification
- Dashboard detects `Message::Close` and cleans up (280-282, 404-406)

**Action Required**:
1. Add `goodbye` message type
2. Send before clean shutdown:
   - MCP Server: On SIGTERM/SIGINT
   - Dashboard: On shutdown
   - Web UI: On `window.beforeunload`
3. Distinguish graceful shutdown from crash

---

## 3. State Machine (Section 3)

### Protocol Requirement

```
DISCONNECTED → CONNECTING → CONNECTED → RECONNECTING
     ▲             │              │
     └─────────────┴──────────────┘
```

### Current Implementation

**Dashboard**: ❌ **No State Machine**
- Just opens connection and handles messages
- No explicit states

**MCP Server** (`src/mcp/ws_client.rs:43-158`): ⚠️ **Partial**
- States: DISCONNECTED (before connect), CONNECTED (after register)
- ❌ No CONNECTING state
- ❌ No RECONNECTING state
- ❌ No reconnection logic (just spawns tasks and exits)

**Web UI** (`static/js/app.js:176-296`): ✅ **Best Implementation**
- States: DISCONNECTED → CONNECTING → CONNECTED → RECONNECTING
- ✅ Reconnection with exponential backoff (280-296)
- ✅ Attempt tracking (wsReconnectAttempts)
- ⚠️ Max attempts = 10 (protocol says unlimited)

**Gap Analysis**:

| Component | States | Reconnection | Status |
|-----------|--------|--------------|--------|
| Dashboard | None | N/A (server) | ✅ Server doesn't need |
| MCP Server | Implicit | ❌ None | 🔧 Critical gap |
| Web UI | Explicit | ✅ Yes | 🔧 Remove max attempts |

**Action Required**:
1. **MCP Server - CRITICAL**: Implement full state machine with reconnection
   - Detect connection loss
   - Enter RECONNECTING state
   - Retry with exponential backoff
   - Re-send `hello` + `register` on reconnect
2. **Web UI**: Remove max attempts limit (unlimited retries)

---

## 4. Reconnection Strategy (Section 3.3)

### Protocol Requirement

```
delays = [1, 2, 4, 8, 16, 32] seconds (capped at 32s)
actual_delay = base_delay + random(0, 1000ms)
max_attempts = unlimited
```

### Current Implementation

**MCP Server**: ❌ **No Reconnection**
- Just spawns ping/read tasks and exits function
- If connection lost: tasks die, no retry

**Web UI** (`static/js/app.js:12`):
```javascript
const WS_RECONNECT_DELAYS = [1000, 2000, 5000, 10000, 30000]; // ms
const WS_MAX_RECONNECT_ATTEMPTS = 10;
```

**Gap Analysis**:

| Feature | Protocol | Web UI | MCP Server |
|---------|----------|--------|------------|
| Delays | 1,2,4,8,16,32s | 1,2,5,10,30s | ❌ None |
| Jitter | Yes | ❌ No | ❌ N/A |
| Max attempts | Unlimited | ⚠️ 10 | ❌ N/A |

**Action Required**:
1. **MCP Server**: Implement full reconnection loop
   ```rust
   loop {
       match connect_with_retry().await {
           Ok(ws) => handle_connection(ws).await,
           Err(e) => continue, // Retry with backoff
       }
   }
   ```
2. **Web UI**:
   - Add jitter: `delay + Math.random() * 1000`
   - Remove max attempts limit
   - Adjust delays to match protocol: `[1, 2, 4, 8, 16, 32]` seconds

---

## 5. Registration Layer (Section 4.2)

### 5.1 `register` / `registered`

**Current Implementation**: ✅ **Mostly Compliant**

**MCP → Dashboard** (`src/mcp/ws_client.rs:93-116`):
- ✅ Sends `register` with project info
- ✅ Waits for `registered` response
- ✅ Handles rejection (success: false)

**Dashboard → MCP** (`src/dashboard/websocket.rs:188-267`):
- ✅ Validates path (rejects temp directories)
- ✅ Sends `registered { success: bool }`
- ✅ Stores connection in memory
- ⚠️ Also writes to Registry (line 233-254) - should be removed

**Gap**:
- Protocol requires `project_path` in response (not just `success`)
- Message format doesn't match protocol wrapper

**Action Required**:
1. Update `registered` response to include `project_path`
2. Wrap in protocol message format
3. Remove Registry write (Phase 3)

---

## 6. State Synchronization Layer (Section 4.3)

### 6.1 `init` (Dashboard → Web UI)

**Current Implementation**: ✅ **Mostly Compliant**

**Dashboard** (`src/dashboard/websocket.rs:355-358`):
```rust
let projects = state.get_online_projects().await;
let init_msg = UiMessage::Init { projects };
```

**Issue**: `get_online_projects()` loads from Registry (line 76-97), not in-memory state.

**Protocol Requirement**: "Dashboard's in-memory state (NOT Registry file)"

**Action Required**:
1. Change `get_online_projects()` to return from `state.mcp_connections`
2. Map `McpConnection` → `ProjectInfo`
3. Remove Registry dependency

---

### 6.2 `project_online` / `project_offline`

**Current Implementation**: ✅ **Compliant**

**Dashboard** (`src/dashboard/websocket.rs:261-267, 318-322`):
- ✅ Broadcasts `project_online` when MCP registers
- ✅ Broadcasts `project_offline` when MCP disconnects
- ✅ Includes correct fields

**Web UI** (`static/js/app.js:376-383`):
- ✅ Handles both message types
- ✅ Updates `onlineProjects` map
- ✅ Re-renders UI

**Gap**: Message format wrapper (version, timestamp)

---

## 7. Error Handling (Section 4.5)

### Protocol Requirement

```json
{
  "version": "1.0",
  "type": "error",
  "payload": {
    "code": "error_code",
    "message": "Human-readable error",
    "details": { ... }
  },
  "timestamp": "..."
}
```

**Current Implementation**: ❌ **Not Implemented**

**Current Behavior**:
- Errors logged with `tracing::warn!()` (e.g., 276, 308)
- No structured error messages sent to clients
- Registration rejection uses `success: false` (not `error` message)

**Action Required**:
1. Define `error` message type
2. Define error codes: `invalid_message`, `unsupported_version`, `registration_failed`, `internal_error`
3. Send structured errors instead of logging only
4. Clients should handle `error` messages

---

## 8. Version Negotiation (Section 6.1)

### Protocol Requirement

- Client sends `hello` with `version: "1.0"`
- Server checks compatibility
- Server responds with `welcome` or `error { code: "unsupported_version" }`

**Current Implementation**: ❌ **Not Implemented**

**Gap**: No version field in any message, no negotiation logic.

**Action Required**:
1. Add `version` field to all messages
2. Implement version check in Dashboard:
   ```rust
   fn is_compatible_version(client_version: &str) -> bool {
       let (major, _) = parse_version(client_version);
       major == 1 // Accept any 1.x
   }
   ```
3. Reject incompatible versions with `error` message

---

## 9. State Recovery (Section 5)

### 9.1 Dashboard Restart (Section 5.1)

**Protocol Requirement**: Dashboard rebuilds state from MCP re-registrations.

**Current Implementation**: ⚠️ **Partially Works, Wrong Approach**

**Current Behavior** (`src/dashboard/websocket.rs:75-97`):
- Dashboard loads state from **Registry file** on startup
- `get_online_projects()` reads Registry

**Problem**: Violates protocol - should use in-memory state only.

**Correct Approach**:
1. Dashboard starts with empty `mcp_connections`
2. MCP Servers reconnect and re-register
3. Dashboard rebuilds state from registrations
4. No file needed

**Action Required**:
1. Remove Registry load in `get_online_projects()`
2. MCP Server must implement reconnection (critical!)
3. Test: Restart Dashboard → MCP auto-reconnects → state restored

---

### 9.2 MCP Server Restart (Section 5.2)

**Protocol Requirement**: MCP reconnects and re-registers.

**Current Implementation**: ❌ **Broken - No Reconnection**

**Current Behavior** (`src/mcp/ws_client.rs:43-158`):
- MCP connects once
- Spawns ping/read tasks
- Function exits
- If connection lost: tasks die, **no retry**

**Problem**: MCP restart → never reconnects → Dashboard never knows it's back online.

**Action Required - CRITICAL**:
1. Keep WebSocket connection alive in foreground (not spawned tasks)
2. Detect connection loss
3. Implement reconnection loop
4. Re-send `hello` + `register` on reconnect

---

### 9.3 Web UI Refresh (Section 5.3)

**Current Implementation**: ✅ **Better than Protocol**

**Web UI** (`static/js/app.js:74-110, 176-249`):
- ✅ Reads project list from `localStorage`
- ✅ Connects to Dashboard
- ✅ Receives `init` with online projects
- ✅ Merges: localStorage (history) + init (online status)
- ✅ Auto-reconnects on connection loss

**Enhancement**: Also polls `/api/health` to detect offline projects coming back (125-170).

**Gap**: Message format wrapper

---

## 10. Registry Dependency (Goal: Remove)

### Current Usage

| Location | Purpose | Line |
|----------|---------|------|
| `websocket.rs:79` | Load projects for `init` | 79-96 |
| `websocket.rs:234` | Write on MCP register | 234-254 |
| `websocket.rs:305` | Write on MCP disconnect | 305-316 |
| `server.rs` (HTTP API) | `/api/projects` endpoint | N/A |

### Protocol Requirement

**Section 5.1**: "Dashboard has NO persistent state (no Registry file needed)"

### Removal Plan (Phase 3)

1. **Phase 1**: Make Registry optional (environment variable)
2. **Phase 2**: Complete message format migration
3. **Phase 3**: Remove all Registry writes
   - `get_online_projects()` → query `mcp_connections` map
   - Remove `register_mcp_connection()` calls
   - Remove `unregister_mcp_connection()` calls
4. **Phase 4**: Remove Registry file entirely
   - Delete `src/dashboard/registry.rs`
   - Remove from `server.rs`

---

## 11. Future Features (Not in Scope for v1.0)

### Real-Time Data Layer (v1.1)

**Protocol Section 4.4**: Reserved message types

- `event_update`: MCP → Dashboard → Web UI (push new events)
- `task_update`: MCP → Dashboard → Web UI (push task changes)

**Current**: Not implemented (expected for v1.1)

**Note**: This is the "信道可以被用来传输各种数据" feature user requested.

---

## 12. Summary of Critical Gaps

### Priority 1 - Blocking Protocol Compliance

| # | Issue | Component | Impact |
|---|-------|-----------|--------|
| 1 | **No reconnection in MCP Server** | `ws_client.rs` | 🔴 Critical - state recovery broken |
| 2 | **Message format wrapper** | All | 🔴 Critical - not protocol compliant |
| 3 | **No hello/welcome** | All | 🟡 High - missing handshake |
| 4 | **Version negotiation** | All | 🟡 High - no forward compatibility |

### Priority 2 - Protocol Violations

| # | Issue | Component | Impact |
|---|-------|-----------|--------|
| 5 | **Registry dependency in init** | `websocket.rs` | 🟡 High - violates stateless requirement |
| 6 | **No goodbye message** | All | 🟢 Medium - nice to have |
| 7 | **No error messages** | All | 🟢 Medium - poor error handling |
| 8 | **Heartbeat timeout missing** | Dashboard, MCP | 🟢 Medium - dead connections not detected |

### Priority 3 - Enhancements

| # | Issue | Component | Impact |
|---|-------|-----------|--------|
| 9 | **Web UI max attempts limit** | `app.js` | 🟢 Low - should be unlimited |
| 10 | **No jitter in backoff** | `app.js` | 🟢 Low - thundering herd risk |
| 11 | **Inconsistent delays** | `app.js` | 🟢 Low - minor protocol deviation |

---

## 13. Migration Phases

### Phase 1: Message Format (1-2 days)
- Add protocol message wrapper
- Update all serialization/deserialization
- Add version field
- Add timestamp field

### Phase 2: Connection Layer (2-3 days)
- Implement hello/welcome handshake
- Add goodbye message
- Implement version negotiation
- Fix heartbeat timeouts
- Add error messages

### Phase 3: State Machine & Reconnection (3-4 days)
- **CRITICAL**: MCP Server reconnection loop
- Web UI: Remove max attempts
- Web UI: Add jitter
- Test all restart scenarios

### Phase 4: Remove Registry (1-2 days)
- Make Registry optional (env var)
- Update `get_online_projects()` to use memory
- Remove Registry writes
- Test state recovery without Registry

### Phase 5: Testing & Documentation (2-3 days)
- Write protocol compliance tests
- Create test harness (mock clients/server)
- Update documentation
- Migration guide

**Total Estimate**: 2-3 weeks for full protocol compliance

---

## 14. Quick Wins (Can Start Immediately)

1. **Add jitter to Web UI** (10 minutes)
2. **Remove Web UI max attempts** (5 minutes)
3. **Fix heartbeat timeout to 90s** (5 minutes)
4. **Add timestamps to logs** (30 minutes)
5. **Environment variable for Registry** (1 hour)

---

## Next Steps

1. **Review this analysis** with user
2. **Prioritize phases** based on user needs
3. **Start with Phase 1** (message format wrapper)
4. **Implement Critical Gap #1** (MCP reconnection) in parallel

---

**Document Version**: 1.0
**Last Updated**: 2025-11-23
