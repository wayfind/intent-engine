# Intent-Engine Protocol (IEP) Specification

**Version**: 1.0
**Status**: Draft
**Last Updated**: 2025-11-23

---

## 1. Overview

### 1.1 Purpose

The Intent-Engine Protocol (IEP) defines a **stable, versioned communication protocol** for real-time coordination between unstable distributed components in the Intent-Engine system.

### 1.2 Design Philosophy

**Core Principle**: The protocol is the contract; implementations are transient.

- **Protocol Stability**: This specification is fixed for version 1.0. Changes require version bump.
- **Implementation Freedom**: Any component may have bugs and be fixed without protocol changes.
- **Entity Instability**: All entities (Web UI, Dashboard, MCP Server) can go offline/online anytime.
- **Self-Healing**: Automatic discovery, reconnection, and state recovery.

### 1.3 Architecture Model

```
┌─────────────┐       WebSocket        ┌─────────────────┐
│   Web UI    │◄─────/ws/ui───────────►│                 │
│ (Multi)     │                         │   Dashboard     │
└─────────────┘                         │   Server        │
                                        │   (Single)      │
┌─────────────┐       WebSocket        │                 │
│ MCP Server  │◄─────/ws/mcp──────────►│                 │
│ (Multi)     │                         └─────────────────┘
└─────────────┘
```

**Three Entity Types**:
- **Web UI (WUI)**: Browser instances, multiple, unstable
- **Dashboard Server (DS)**: Central node, single instance, unstable
- **MCP Server (MS)**: Project instances, multiple, unstable

**Topology**: Star (DS is hub)

---

## 2. Transport Layer

### 2.1 WebSocket Endpoints

| Endpoint | Client | Server | Purpose |
|----------|--------|--------|---------|
| `/ws/ui` | Web UI | Dashboard | UI real-time updates |
| `/ws/mcp` | MCP Server | Dashboard | Project registration & events |

### 2.2 Message Format

All messages are **JSON** objects with this structure:

```json
{
  "version": "1.0",
  "type": "message_type",
  "payload": { ... },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

**Required Fields**:
- `version` (string): Protocol version (e.g., "1.0")
- `type` (string): Message type (see Section 4)
- `timestamp` (string): ISO 8601 UTC timestamp

**Optional Fields**:
- `payload` (object): Message-specific data
- `id` (string): Correlation ID for request-response

---

## 3. Connection State Machine

### 3.1 States

```
DISCONNECTED ──connect()──► CONNECTING ──success──► CONNECTED
     ▲                           │                       │
     │                         fail                   error/close
     │                           │                       │
     └───────────────────────────┴───────► RECONNECTING ┘
                                                  │
                                              backoff timer
                                                  │
                                                  ▼
                                            (retry connect)
```

### 3.2 State Transitions

| From | Event | To | Action |
|------|-------|-----|--------|
| DISCONNECTED | connect() | CONNECTING | Open WebSocket |
| CONNECTING | open | CONNECTED | Send `hello` |
| CONNECTING | error | RECONNECTING | Start backoff timer |
| CONNECTED | close/error | RECONNECTING | Start backoff timer |
| RECONNECTING | timer | CONNECTING | Retry connection |

### 3.3 Reconnection Strategy

**Exponential Backoff with Jitter**:

```
delays = [1, 2, 4, 8, 16, 32] seconds (capped at 32s)
actual_delay = base_delay + random(0, 1000ms)
max_attempts = unlimited (keep trying forever)
```

**Rationale**:
- Entities may be offline for extended periods (e.g., Dashboard restart)
- Unlimited retries ensure eventual reconnection
- Jitter prevents thundering herd

---

## 4. Message Types

### 4.1 Connection Layer

#### 4.1.1 `hello` (Client → Server)

Sent immediately after WebSocket connection opens.

```json
{
  "version": "1.0",
  "type": "hello",
  "payload": {
    "entity_type": "web_ui" | "mcp_server",
    "entity_id": "unique_identifier",
    "capabilities": ["feature_1", "feature_2"]
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

**Fields**:
- `entity_type`: "web_ui" or "mcp_server"
- `entity_id`: Unique identifier (e.g., browser tab ID, project path)
- `capabilities`: Optional feature flags

#### 4.1.2 `welcome` (Server → Client)

Response to `hello`, confirms connection established.

```json
{
  "version": "1.0",
  "type": "welcome",
  "payload": {
    "session_id": "uuid",
    "server_time": "2025-11-23T03:00:00Z"
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

#### 4.1.3 `ping` / `pong` (Bidirectional)

Heartbeat to detect dead connections.

```json
{
  "version": "1.0",
  "type": "ping",
  "timestamp": "2025-11-23T03:00:00Z"
}
```

**Timing**: Every 30 seconds
**Timeout**: 90 seconds (3 missed pings → close connection)

#### 4.1.4 `goodbye` (Bidirectional)

Graceful shutdown notification.

```json
{
  "version": "1.0",
  "type": "goodbye",
  "payload": {
    "reason": "shutdown" | "restart" | "error"
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

---

### 4.2 Registration Layer (MCP Server only)

#### 4.2.1 `register` (MS → DS)

Register project with Dashboard.

```json
{
  "version": "1.0",
  "type": "register",
  "payload": {
    "project": {
      "path": "/absolute/path/to/project",
      "name": "project-name",
      "db_path": "/path/to/project.db",
      "agent": "optional_agent_name"
    }
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

**Validation Rules**:
- `path` must be absolute
- `path` must NOT be in temp directory (defense layer)
- `name` derived from path if not provided

#### 4.2.2 `registered` (DS → MS)

Confirmation of registration.

```json
{
  "version": "1.0",
  "type": "registered",
  "payload": {
    "success": true,
    "project_path": "/absolute/path/to/project"
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

If rejected:

```json
{
  "version": "1.0",
  "type": "registered",
  "payload": {
    "success": false,
    "error": "invalid_path",
    "message": "Path is in temporary directory"
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

---

### 4.3 State Synchronization Layer

#### 4.3.1 `init` (DS → WUI)

Initial state sent to Web UI after connection.

```json
{
  "version": "1.0",
  "type": "init",
  "payload": {
    "projects": [
      {
        "path": "/path/to/project",
        "name": "project-name",
        "db_path": "/path/to/db",
        "agent": "agent-name",
        "mcp_connected": true
      }
    ]
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

**Source**: Dashboard's in-memory state (NOT Registry file)

#### 4.3.2 `project_online` (DS → WUI)

Broadcast when MCP Server connects.

```json
{
  "version": "1.0",
  "type": "project_online",
  "payload": {
    "project": {
      "path": "/path/to/project",
      "name": "project-name",
      "db_path": "/path/to/db",
      "agent": "agent-name",
      "mcp_connected": true
    }
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

#### 4.3.3 `project_offline` (DS → WUI)

Broadcast when MCP Server disconnects.

```json
{
  "version": "1.0",
  "type": "project_offline",
  "payload": {
    "project_path": "/path/to/project"
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

---

### 4.4 Real-Time Data Layer (Future)

These message types are **reserved for future use** (v1.1+):

#### 4.4.1 `event_update` (MS → DS → WUI)

Notify UI when new event is added to task.

```json
{
  "version": "1.0",
  "type": "event_update",
  "payload": {
    "project_path": "/path/to/project",
    "task_id": 42,
    "event": {
      "id": 123,
      "type": "decision",
      "data": "...",
      "timestamp": "2025-11-23T03:00:00Z"
    }
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

**Use Case**: MCP tool call adds event → UI auto-refreshes

#### 4.4.2 `task_update` (MS → DS → WUI)

Notify UI when task status changes.

```json
{
  "version": "1.0",
  "type": "task_update",
  "payload": {
    "project_path": "/path/to/project",
    "task": {
      "id": 42,
      "name": "Task name",
      "status": "doing",
      "updated_at": "2025-11-23T03:00:00Z"
    }
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

---

### 4.5 Error Handling

#### 4.5.1 `error` (Bidirectional)

Generic error notification.

```json
{
  "version": "1.0",
  "type": "error",
  "payload": {
    "code": "error_code",
    "message": "Human-readable error",
    "details": { ... }
  },
  "timestamp": "2025-11-23T03:00:00Z"
}
```

**Error Codes**:
- `invalid_message`: Malformed JSON or missing required fields
- `unsupported_version`: Protocol version mismatch
- `registration_failed`: Project registration rejected
- `internal_error`: Server-side error

---

## 5. State Recovery Mechanisms

### 5.1 Dashboard Restart

**Scenario**: Dashboard goes offline and restarts.

**Recovery**:
1. Dashboard starts, listens on `/ws/mcp` and `/ws/ui`
2. MCP Servers detect connection loss → RECONNECTING state
3. MCP Servers retry with exponential backoff
4. Upon reconnection, send `hello` + `register` again
5. Dashboard rebuilds in-memory state from registrations
6. Web UI reconnects, receives `init` with current state

**Key**: Dashboard has NO persistent state (no Registry file needed)

### 5.2 MCP Server Restart

**Scenario**: MCP Server crashes or is manually restarted.

**Recovery**:
1. Dashboard detects WebSocket close → broadcasts `project_offline`
2. Web UI shows project as offline (gray)
3. MCP Server restarts → connects to Dashboard
4. Sends `hello` + `register`
5. Dashboard broadcasts `project_online`
6. Web UI shows project as online (green)

**Key**: Project state persists in SQLite DB, not in Dashboard

### 5.3 Web UI Refresh

**Scenario**: User refreshes browser tab.

**Recovery**:
1. Web UI reads project list from `localStorage`
2. Connects to Dashboard → sends `hello`
3. Dashboard sends `init` with current online projects
4. Web UI merges: localStorage (history) + init (online status)
5. Renders projects with correct green/gray indicators

**Key**: `localStorage` stores project list, WebSocket provides online status

---

## 6. Protocol Versioning

### 6.1 Version Negotiation

**Version String**: `"1.0"`, `"1.1"`, `"2.0"`

**Handshake**:
1. Client sends `hello` with `version: "1.0"`
2. Server checks compatibility
3. If compatible: sends `welcome`
4. If incompatible: sends `error` with `unsupported_version`

### 6.2 Compatibility Rules

- **Major version change (1.x → 2.x)**: Breaking changes, no compatibility
- **Minor version change (1.0 → 1.1)**: Backward-compatible additions (new message types)
- **Patch version (future)**: Bug fixes, no protocol changes

### 6.3 Deprecation Policy

- New message types: Add in minor version (e.g., 1.0 → 1.1)
- Remove message types: Major version bump (e.g., 1.x → 2.0)
- Deprecation period: Minimum 6 months notice

---

## 7. Implementation Guidelines

### 7.1 Must Implement

**All Entities**:
- Connection state machine (Section 3)
- `hello`, `welcome`, `ping`, `pong`, `goodbye` (Section 4.1)
- Reconnection with exponential backoff (Section 3.3)
- Error handling (Section 4.5)

**MCP Server**:
- `register`, `registered` (Section 4.2)
- Path validation (reject temp directories)

**Dashboard Server**:
- In-memory state management (no persistent Registry)
- Broadcast to all Web UI connections
- Handle multiple MCP Server connections

**Web UI**:
- `init`, `project_online`, `project_offline` (Section 4.3)
- localStorage for project history
- Merge online status from WebSocket

### 7.2 Should Implement

- Structured logging with correlation IDs
- Metrics (connection count, message rate, latency)
- Health checks (separate from WebSocket)

### 7.3 Must NOT

- ❌ Add custom message types without protocol version bump
- ❌ Rely on file-based state synchronization (Registry)
- ❌ Assume entities are stable (always plan for disconnection)
- ❌ Use `localhost` hardcoded (should be configurable)

---

## 8. Security Considerations

### 8.1 Authentication

**Current**: None (localhost-only deployment)

**Future**:
- Token-based auth in `hello` message
- TLS for remote deployments

### 8.2 Validation

**All Entities Must**:
- Validate JSON schema before processing
- Reject messages with invalid `version`
- Sanitize file paths (reject temp directories)
- Rate-limit messages (prevent DoS)

---

## 9. Testing & Compliance

### 9.1 Protocol Compliance Tests

Implementations MUST pass these tests:

1. **Connection Test**: Establish connection, exchange `hello`/`welcome`
2. **Heartbeat Test**: Send/receive `ping`/`pong` every 30s
3. **Reconnection Test**: Kill connection, verify exponential backoff
4. **State Recovery Test**: Restart Dashboard, verify MCP re-registration
5. **Broadcast Test**: Register MCP, verify all Web UIs receive `project_online`

### 9.2 Test Harness

A reference test harness will be provided in `tests/protocol/`:

- Mock Dashboard server
- Mock MCP client
- Mock Web UI client
- Scenario-based tests (reconnection, state recovery, etc.)

---

## 10. Examples

### 10.1 MCP Server Connection Flow

```
MS                           DS                           WUI
│                            │                            │
├─(connect ws://DS/ws/mcp)──►│                            │
│◄──(WebSocket open)─────────┤                            │
│                            │                            │
├─hello─────────────────────►│                            │
│◄─welcome────────────────────┤                            │
│                            │                            │
├─register───────────────────►│                            │
│◄─registered─────────────────┤                            │
│                            ├─project_online────────────►│
│                            │                            │
│◄─ping───────────────────────┤                            │
├─pong───────────────────────►│                            │
│                            │                            │
(30s later)                  │                            │
│◄─ping───────────────────────┤                            │
├─pong───────────────────────►│                            │
│                            │                            │
```

### 10.2 Dashboard Restart Flow

```
MS                           DS                           WUI
│                            │                            │
├─ping────────────────X      │ (Dashboard crashes)        │
│                                                         │
(detect timeout)                                          │
├─(enter RECONNECTING)       │                            │
│                            │                            │
(wait 1s)                    │                            │
├─(retry connect)────────X   │ (still down)               │
│                            │                            │
(wait 2s)                    │                            │
├─(retry connect)────────X   │ (still down)               │
│                            │                            │
(wait 4s)                    │                            │
│                            ● (Dashboard restarts)       │
│                            │                            │
├─(retry connect)───────────►│                            │
│◄──(WebSocket open)─────────┤                            │
│                            │                            │
├─hello─────────────────────►│                            │
│◄─welcome────────────────────┤                            │
│                            │                            │
├─register───────────────────►│ (state rebuilt)           │
│◄─registered─────────────────┤                            │
│                            │                            │
```

---

## 11. Future Extensions (Post-1.0)

These features are NOT in v1.0 but planned for future versions:

### v1.1 (Real-Time Data Sync)
- `event_update`: Push new events to UI
- `task_update`: Push task status changes to UI
- Bi-directional sync: UI can send commands (e.g., mark task done)

### v1.2 (Multi-Dashboard)
- Dashboard clustering (multiple instances)
- Leader election
- State replication

### v2.0 (Breaking Changes)
- Remove Registry file entirely
- gRPC transport option (in addition to WebSocket)
- Binary protocol (Protocol Buffers)

---

## 12. References

- [WebSocket Protocol (RFC 6455)](https://tools.ietf.org/html/rfc6455)
- [JSON Schema](https://json-schema.org/)
- [ISO 8601 Timestamps](https://en.wikipedia.org/wiki/ISO_8601)

---

## Changelog

### 1.0 (2025-11-23)
- Initial protocol specification
- Connection layer (hello, welcome, ping, pong, goodbye)
- Registration layer (register, registered)
- State sync layer (init, project_online, project_offline)
- Reconnection strategy (exponential backoff)
- State recovery mechanisms

---

**End of Specification**
