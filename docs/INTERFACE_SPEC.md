# Intent-Engine Interface Specification

**Version**: 0.1.9
**Last Updated**: 2024-11-09
**Status**: Stable

---

## Overview

Intent-Engine provides three primary interfaces for task and intent management:

1. **CLI Interface** - Command-line tool for human operators
2. **MCP Interface** - Model Context Protocol for AI assistants
3. **Rust Library API** - Direct library integration

This document serves as the **authoritative specification** for all public interfaces.

---

## 1. Core Concepts

### 1.1 Data Model

```
Task
├── id: Integer (auto-increment)
├── name: String (required)
├── spec: String (markdown, optional)
├── status: Enum { todo, doing, done }
├── complexity: Enum { trivial, simple, moderate, complex, epic }
├── priority: Enum { low, medium, high, urgent }
├── parent_id: Integer (optional)
├── created_at: Timestamp
└── updated_at: Timestamp

Event
├── id: Integer (auto-increment)
├── task_id: Integer (required)
├── event_type: Enum { decision, blocker, milestone, note }
├── data: String (markdown)
└── created_at: Timestamp

Workspace State
└── current_task_id: Integer (nullable)
```

### 1.2 Status Transitions

```
todo → doing → done
  ↑      ↓
  └──────┘
```

**Rules**:
- `start`: `todo` → `doing`
- `done`: `doing` → `done` (only if all children are `done`)
- `switch`: `doing` → `todo` + new task → `doing`

---

## 2. CLI Interface

### 2.1 Task Commands

#### `task add`
**Purpose**: Create a new task

**Signature**:
```bash
intent-engine task add \
  --name <NAME> \
  [--spec <SPEC> | --spec-stdin] \
  [--parent-id <PARENT_ID>] \
  [--complexity <COMPLEXITY>] \
  [--priority <PRIORITY>]
```

**Parameters**:
- `name` (required): Task name
- `spec` (optional): Detailed specification in markdown
- `spec-stdin` (optional): Read spec from stdin
- `parent-id` (optional): Parent task ID for subtasks
- `complexity` (optional): trivial | simple | moderate | complex | epic
- `priority` (optional): low | medium | high | urgent

**Output**: JSON
```json
{
  "id": 42,
  "name": "Implement authentication",
  "status": "todo",
  "created_at": "2024-11-09T10:00:00Z"
}
```

**Exit Codes**:
- `0`: Success
- `1`: Invalid parameters
- `2`: Database error

---

#### `task start`
**Purpose**: Start working on a task (atomic operation)

**Signature**:
```bash
intent-engine task start <TASK_ID> [--with-events]
```

**Parameters**:
- `task_id` (required): Task ID to start
- `with-events` (optional): Include event history in output

**Behavior** (atomic):
1. Set task status to `doing`
2. Set as current task in workspace
3. Return full task context

**Output**: JSON
```json
{
  "task": {
    "id": 42,
    "name": "Implement authentication",
    "status": "doing",
    "spec": "..."
  },
  "events": [...]  // if --with-events
}
```

---

#### `task done`
**Purpose**: Complete current task

**Signature**:
```bash
intent-engine task done
```

**Behavior** (atomic):
1. Verify all subtasks are `done`
2. Set current task status to `done`
3. Clear current task in workspace

**Validation**:
- Fails if current task has incomplete subtasks
- Fails if no current task

**Output**: JSON
```json
{
  "id": 42,
  "status": "done",
  "completed_at": "2024-11-09T11:00:00Z"
}
```

---

#### `task spawn-subtask`
**Purpose**: Create subtask under current task and switch to it

**Signature**:
```bash
intent-engine task spawn-subtask \
  --name <NAME> \
  [--spec <SPEC> | --spec-stdin]
```

**Behavior** (atomic):
1. Create subtask with `parent_id` = current task
2. Switch to new subtask (set as current)

**Requirements**:
- Must have a current task
- Current task becomes parent

---

#### `task pick-next`
**Purpose**: Intelligently recommend next tasks

**Signature**:
```bash
intent-engine task pick-next \
  [--limit <N>] \
  [--complexity <COMPLEXITY>] \
  [--priority <PRIORITY>]
```

**Algorithm**:
1. Filter tasks with `status = todo`
2. Apply complexity/priority filters
3. Sort by: priority DESC, complexity ASC
4. Return top N tasks

---

#### `task find`
**Purpose**: Search tasks with filters

**Signature**:
```bash
intent-engine task find \
  [--status <STATUS>] \
  [--complexity <COMPLEXITY>] \
  [--priority <PRIORITY>] \
  [--parent-id <PARENT_ID>] \
  [--name-pattern <PATTERN>]
```

---

#### `task search`
**Purpose**: Full-text search using FTS5

**Signature**:
```bash
intent-engine task search <QUERY> \
  [--limit <N>] \
  [--snippet]
```

**Features**:
- Search in task name + spec
- Supports FTS5 syntax (AND, OR, NEAR, etc.)
- `--snippet`: Return highlighted matches with `**`

---

### 2.2 Event Commands

#### `event add`
**Purpose**: Record event for current task

**Signature**:
```bash
intent-engine event add \
  --type <TYPE> \
  [--data <DATA> | --data-stdin]
```

**Parameters**:
- `type` (required): decision | blocker | milestone | note
- `data`: Event description in markdown

---

#### `event list`
**Purpose**: List events for a task

**Signature**:
```bash
intent-engine event list <TASK_ID>
```

---

### 2.3 Report Commands

#### `report`
**Purpose**: Generate analysis and reports

**Signature**:
```bash
intent-engine report \
  [--since <DURATION>] \
  [--status <STATUS>] \
  [--summary-only]
```

**Parameters**:
- `since`: Duration (e.g., "7d", "2h", "30m")
- `status`: Filter by status
- `summary-only`: Return summary statistics only

---

### 2.4 Workspace Commands

#### `current`
**Purpose**: Get/set current task

**Signature**:
```bash
# Get current task
intent-engine current

# Set current task
intent-engine current --set <TASK_ID>
```

---

## 3. MCP Interface

### 3.1 Protocol

**Protocol**: JSON-RPC 2.0 over stdio
**Schema Version**: 0.1.9
**Schema File**: `mcp-server.json`

### 3.2 Available Tools

| Tool Name | Purpose | Maps to CLI |
|-----------|---------|-------------|
| `task_add` | Create task | `task add` |
| `task_start` | Start task | `task start` |
| `task_pick_next` | Recommend tasks | `task pick-next` |
| `task_spawn_subtask` | Create subtask | `task spawn-subtask` |
| `task_switch` | Switch task | `task switch` |
| `task_done` | Complete task | `task done` |
| `task_update` | Update task | `task update` |
| `task_find` | Filter tasks | `task find` |
| `task_get` | Get task by ID | `task get` |
| `event_add` | Record event | `event add` |
| `event_list` | List events | `event list` |
| `current_task_get` | Get current task | `current` |
| `report_generate` | Generate report | `report` |

### 3.3 Tool Schema Reference

All MCP tools follow the schema defined in `mcp-server.json`.

**Example Tool Call**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "task_add",
    "arguments": {
      "name": "Implement auth",
      "spec": "Use JWT with 7-day expiry"
    }
  }
}
```

**Response Format**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [{
      "type": "text",
      "text": "{\"id\": 42, \"name\": \"Implement auth\", ...}"
    }]
  }
}
```

---

## 4. Rust Library API

### 4.1 Core Modules

```rust
use intent_engine::{
    tasks::TaskManager,
    events::EventManager,
    report::ReportManager,
    workspace::WorkspaceManager,
    project::ProjectContext,
};
```

### 4.2 Example Usage

```rust
use intent_engine::project::ProjectContext;
use intent_engine::tasks::{TaskManager, TaskStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize project context
    let ctx = ProjectContext::current_or_init().await?;

    // Create task manager
    let task_mgr = TaskManager::new(ctx.clone());

    // Add a task
    let task = task_mgr.add_task(
        "Implement authentication",
        Some("Use JWT..."),
        None,  // no parent
        None,  // default complexity
        None,  // default priority
    ).await?;

    // Start task
    let started = task_mgr.start_task(task.id, true).await?;

    // Complete task
    task_mgr.complete_task(task.id).await?;

    Ok(())
}
```

### 4.3 API Documentation

Full API documentation available at: https://docs.rs/intent-engine

---

## 5. Output Formats

### 5.1 JSON Output (Default)

All CLI commands output structured JSON by default.

**Standard Fields**:
```json
{
  "success": true,
  "data": { ... },
  "timestamp": "2024-11-09T10:00:00Z"
}
```

**Error Format**:
```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid task ID"
  }
}
```

### 5.2 Format Flag

Future versions may support `--format` flag:
- `json` (default)
- `yaml`
- `table`
- `markdown`

---

## 6. Interface Guarantees

### 6.1 Semantic Versioning

Intent-Engine follows [SemVer 2.0](https://semver.org/):

- **MAJOR** version: Breaking interface changes
- **MINOR** version: Backward-compatible additions
- **PATCH** version: Backward-compatible bug fixes

### 6.2 Stability Guarantees

| Version | CLI Interface | MCP Interface | Rust API |
|---------|--------------|---------------|----------|
| 0.1.x   | Experimental | Experimental  | Experimental |
| 1.0.x   | Stable       | Stable        | Stable |

**Current Status (0.1.9)**: All interfaces are **experimental** and may change.

### 6.3 Deprecation Policy

For stable versions (≥1.0):
1. Deprecated features marked in documentation
2. Warning messages shown for 2 minor versions
3. Removal only in next major version

---

## 7. Validation & Testing

### 7.1 Interface Consistency Tests

```bash
# Verify CLI commands match spec
cargo test --test cli_spec_test

# Verify MCP tools match spec
cargo test --test mcp_tools_sync_test

# Verify Rust API matches spec
cargo test --doc
```

### 7.2 Contract Testing

Future: Add contract tests to ensure interface stability across versions.

---

## 8. Migration Guide

### 8.1 Breaking Changes

All breaking changes documented in `CHANGELOG.md` with migration guide.

### 8.2 Version Matrix

| Intent-Engine Version | CLI Version | MCP Schema Version | Min Rust API |
|----------------------|-------------|-------------------|--------------|
| 0.1.9                | 0.1.9       | 0.1.9             | 0.1.9        |

---

## 9. Related Documents

- **API Reference**: `docs/*/guide/command-reference-full.md`
- **MCP Schema**: `mcp-server.json`
- **Rust API Docs**: https://docs.rs/intent-engine
- **Changelog**: `CHANGELOG.md`
- **MCP Sync System**: `docs/*/technical/mcp-tools-sync.md`

---

## 10. Maintenance

This specification is maintained as the **single source of truth** for Intent-Engine interfaces.

**Update Process**:
1. Update this spec first for any interface changes
2. Update implementation to match spec
3. Update `mcp-server.json` if MCP tools changed
4. Run `./scripts/sync-mcp-tools.sh`
5. Run tests: `cargo test --test mcp_tools_sync_test`

**Automated Sync**:
- Version number synced by `scripts/sync-mcp-tools.sh`
- Tool list validated by `tests/mcp_tools_sync_test.rs`

---

**Specification Version**: 0.1.9
**Maintained by**: Intent-Engine Contributors
**License**: MIT OR Apache-2.0
