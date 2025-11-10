# Intent-Engine Interface Specification

**Version**: 0.1
**Last Updated**: 2024-11-09
**Status**: Experimental (Pre-1.0)

---

> **🚨 CRITICAL: Version Update Policy**
>
> This document defines the **interface contract version** (major.minor format).
>
> **WHEN TO UPDATE THE VERSION:**
> - ✅ **Breaking changes** → Increment major (0.1 → 1.0)
> - ✅ **New features/tools** → Increment minor (0.1 → 0.2)
> - ❌ **Bug fixes only** → Do NOT update (keep 0.1)
>
> **THIS VERSION IS THE SOURCE OF TRUTH**
> - All other files (CLAUDE.md, mcp-server.json) follow this version
> - Cargo.toml may have additional patch version (e.g., 0.1.12)
> - When you update this version, you MUST update Cargo.toml's minor version
>
> **FOR AI ASSISTANTS:**
> If you modify this file and change the interface (add/remove/modify CLI commands,
> MCP tools, or data models), you MUST increment the version number above and
> remind the user to run the version sync workflow.

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
├── status: String { "todo", "doing", "done" }
├── complexity: Integer (optional, nullable)
├── priority: Integer (optional, nullable, 1=highest)
├── parent_id: Integer (optional, nullable)
├── first_todo_at: Timestamp (when first set to todo)
├── first_doing_at: Timestamp (when first set to doing)
└── first_done_at: Timestamp (when first set to done)

Event
├── id: Integer (auto-increment)
├── task_id: Integer (required)
├── timestamp: Timestamp
├── log_type: String { "decision", "blocker", "milestone", "note" }
└── discussion_data: String (markdown)

Workspace State
└── current_task_id: Integer (nullable)
```

**Key Design Principles**:
- **Focus-Driven**: Most commands operate on `current_task_id` (the "focused" task)
- **Priority Model**: Lower number = higher priority (1 is highest, optional field)
- **Lifecycle Timestamps**: Track first occurrence of each status for analysis
- **Atomic Operations**: Commands like `start`, `switch`, `done` combine multiple steps

### 1.2 Status Transitions

```
todo → doing → done
  ↑      ↓
  └──────┘
```

**Transition Rules**:
- `start <ID>`: Set task to `doing` + set as current
- `switch <ID>`: (Previous doing → todo) + Set new task to `doing` + set as current
- `done`: Current task → `done` + clear current (requires all children done)

### 1.3 Project Initialization and Smart Root Inference

**Philosophy**: Intent-Engine's initialization is designed to be completely transparent and frictionless. Users should focus on expressing intent (e.g., `task add`) rather than performing setup work (e.g., `init`). There is no public `init` command.

**Smart Lazy Initialization**: The system automatically initializes on the first write operation, intelligently inferring the project root directory rather than blindly using the current working directory (CWD).

#### Trigger Conditions

Initialization logic is triggered when **all** of the following conditions are met:
1. A write-type CLI command is executed (e.g., `task add`, `task spawn-subtask`)
2. No `.intent-engine` folder exists in CWD or any parent directory

If `.intent-engine` already exists, it is used directly without re-initialization. If a read-only command is executed and no `.intent-engine` is found, an error is returned.

#### Root Directory Inference Algorithm

The system follows this algorithm to determine where to initialize:

**Step 1: Define Project Root Markers (Priority Order)**

The system uses a hardcoded priority list of common project markers:

1. `.git` (Git - highest priority)
2. `.hg` (Mercurial)
3. `package.json` (Node.js)
4. `Cargo.toml` (Rust)
5. `pyproject.toml` (Python PEP 518)
6. `go.mod` (Go Modules)
7. `pom.xml` (Maven - Java)
8. `build.gradle` (Gradle - Java/Kotlin)

**Step 2: Recursive Upward Search**

Starting from CWD, traverse upward to the filesystem root (`/`), checking each directory for markers in priority order.

**Step 3: First Match Determines Root**

The first directory containing any marker becomes the project root. Search terminates immediately upon finding a marker.

**Step 4: Initialize in Determined Root**

Create `.intent-engine/project.db` in the determined project root directory.

**Step 5: Fallback Mechanism**

If no markers are found after reaching filesystem root:
- Use CWD as the project root (fallback)
- Print a warning to stderr:
  ```
  Warning: Could not determine a project root based on common markers (e.g., .git, package.json).
           Initialized Intent-Engine in the current directory '/path/to/cwd'.
           For predictable behavior, it's recommended to initialize from a directory containing a root marker.
  ```

#### Example Scenarios

**Scenario 1: Git Repository**
```
Structure: /home/user/my-app/.git
           /home/user/my-app/src/components/

Command: cd /home/user/my-app/src/components && ie task add --name "Fix button"

Result: .intent-engine created at /home/user/my-app/ (where .git is located)
```

**Scenario 2: No Markers (Fallback)**
```
Structure: /home/user/scripts/ (no markers)

Command: cd /home/user/scripts && ie task add --name "Refactor script"

Result: .intent-engine created at /home/user/scripts/ (CWD fallback)
        Warning printed to stderr
```

**Scenario 3: Multiple Markers (Priority)**
```
Structure: /home/user/project/.git
           /home/user/project/nested/Cargo.toml

Command: cd /home/user/project/nested/deep && ie task add --name "Test"

Result: .intent-engine created at /home/user/project/ (.git has higher priority)
```

#### Error Handling

- **Permission Errors**: If directory/file creation fails due to permissions, the command fails with a clear filesystem-related error message
- **Read-Only Commands**: If a read command is executed without an existing `.intent-engine`, return `NOT_A_PROJECT` error

---

## 2. CLI Interface

### 2.1 Task Commands

#### `task add`
**Purpose**: Create a new task

**Signature**:
```bash
intent-engine task add \
  --name <NAME> \
  [--spec-stdin] \
  [--parent <PARENT_ID>]
```

**Parameters**:
- `--name <NAME>` (required): Task name
- `--spec-stdin` (optional): Read spec from stdin
- `--parent <PARENT_ID>` (optional): Parent task ID for subtasks

**Output**: JSON
```json
{
  "id": 42,
  "name": "Implement authentication",
  "status": "todo",
  "parent_id": null,
  "first_todo_at": "2024-11-09T10:00:00Z"
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
- `<TASK_ID>` (required): Task ID to start
- `--with-events` (optional): Include event history in output

**Atomic Behavior**:
1. Set task status to `doing` (and set `first_doing_at` if first time)
2. Set as current task in workspace
3. Return full task context

**Output**: JSON
```json
{
  "task": {
    "id": 42,
    "name": "Implement authentication",
    "status": "doing",
    "spec": "...",
    "first_doing_at": "2024-11-09T10:05:00Z"
  },
  "events_summary": {
    "total_count": 3,
    "recent_events": [...]
  }
}
```

---

#### `task done`
**Purpose**: Complete the current focused task

**Signature**:
```bash
intent-engine task done
```

**No Parameters** - This command is **strictly focus-driven** and operates only on `current_task_id`.

**Prerequisites**:
- A task must be set as current (via `task start` or `current --set`)
- All subtasks must have status `done`

**Atomic Behavior**:
1. Verify all subtasks are `done` (fails if not)
2. Set current task status to `done` (and set `first_done_at` if first time)
3. Clear `current_task_id` in workspace

**Output**: JSON
```json
{
  "completed_task": {
    "id": 42,
    "name": "Implement authentication",
    "status": "done",
    "first_done_at": "2024-11-09T11:00:00Z"
  },
  "workspace_status": {
    "current_task_id": null
  }
}
```

**Error Cases**:
- No current task: `"error": "No current task set"`
- Incomplete subtasks: `"error": "Cannot complete task: has incomplete subtasks"`

---

#### `task spawn-subtask`
**Purpose**: Create subtask under current task and switch to it

**Signature**:
```bash
intent-engine task spawn-subtask \
  --name <NAME> \
  [--spec-stdin]
```

**Parameters**:
- `--name <NAME>` (required): Subtask name
- `--spec-stdin` (optional): Read spec from stdin

**Prerequisites**:
- Must have a current task set

**Atomic Behavior**:
1. Create subtask with `parent_id` = current task ID
2. Switch to new subtask (set as current, mark as `doing`)

**Output**: JSON
```json
{
  "subtask": {
    "id": 43,
    "name": "Configure JWT secret",
    "parent_id": 42,
    "status": "doing"
  },
  "parent_task": {
    "id": 42,
    "name": "Implement authentication"
  }
}
```

---

#### `task switch`
**Purpose**: Switch focus to a different task

**Signature**:
```bash
intent-engine task switch <TASK_ID>
```

**Parameters**:
- `<TASK_ID>` (required): Task ID to switch to

**Atomic Behavior**:
1. If there's a current task in `doing` status, set it back to `todo`
2. Set new task status to `doing` (and set `first_doing_at` if first time)
3. Update `current_task_id` to new task

**Output**: JSON
```json
{
  "previous_task": {
    "id": 42,
    "status": "todo"
  },
  "current_task": {
    "id": 43,
    "name": "Configure JWT secret",
    "status": "doing"
  }
}
```

---

#### `task pick-next`
**Purpose**: Intelligently recommend the next task to work on

**Signature**:
```bash
intent-engine task pick-next [--format <FORMAT>]
```

**Parameters**:
- `--format <FORMAT>` (optional): Output format (`text` or `json`, default: `text`)

**Algorithm** (Context-Aware, Depth-First):

**Priority 1**: Subtasks of current focused task
```
IF current_task_id IS SET:
  SELECT * FROM tasks
  WHERE parent_id = current_task_id
    AND status = 'todo'
  ORDER BY priority ASC NULLS LAST, id ASC
  LIMIT 1
```

**Priority 2**: Top-level tasks (if no focused subtasks)
```
IF Priority 1 returns empty:
  SELECT * FROM tasks
  WHERE parent_id IS NULL
    AND status = 'todo'
  ORDER BY priority ASC NULLS LAST, id ASC
  LIMIT 1
```

**Output** (text format):
```
📋 Recommended next task:
  #43: Configure JWT secret (priority: 1)
  Parent: #42 Implement authentication

🎯 Reason: Subtask of current focused task
💡 To start: intent-engine task start 43
```

**Output** (json format):
```json
{
  "recommended_task": {
    "id": 43,
    "name": "Configure JWT secret",
    "priority": 1,
    "parent_id": 42
  },
  "reason": "subtask_of_current",
  "context": {
    "current_task_id": 42,
    "strategy": "depth_first"
  }
}
```

**Empty State Response**:
```json
{
  "recommended_task": null,
  "reason": "no_todo_tasks",
  "suggestion": "All tasks are done! Use 'task add' to create new tasks."
}
```

---

#### `task find`
**Purpose**: Filter tasks by structured metadata

**Signature**:
```bash
intent-engine task find \
  [--status <STATUS>] \
  [--parent <PARENT_ID>]
```

**Parameters**:
- `--status <STATUS>` (optional): Filter by status (`todo`, `doing`, `done`)
- `--parent <PARENT_ID>` (optional): Filter by parent ID (use `"null"` for root tasks)

**Design Note**: `task find` handles **structured filtering only**. For text search, use `task search`.

**Output**: JSON (array of tasks)
```json
[
  {
    "id": 42,
    "name": "Implement authentication",
    "status": "doing",
    "parent_id": null
  },
  ...
]
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

**Parameters**:
- `<QUERY>` (required): FTS5 search query
- `--limit <N>` (optional): Maximum results to return
- `--snippet` (optional): Return highlighted matches with `**`

**Search Scope**: Searches in both `name` and `spec` fields.

**FTS5 Syntax Support**:
- AND: `auth AND jwt`
- OR: `auth OR oauth`
- NEAR: `auth NEAR/5 token`
- Phrases: `"user authentication"`

**Output** (with `--snippet`):
```json
[
  {
    "task_id": 42,
    "name": "Implement **authentication**",
    "spec_snippet": "Use **JWT** with refresh tokens...",
    "rank": 0.95
  }
]
```

---

#### `task get`
**Purpose**: Get task by ID

**Signature**:
```bash
intent-engine task get <TASK_ID>
```

**Output**: JSON (single task with full details)

---

#### `task update`
**Purpose**: Update task properties

**Signature**:
```bash
intent-engine task update <TASK_ID> \
  [--name <NAME>] \
  [--status <STATUS>] \
  [--spec-stdin]
```

---

### 2.2 Event Commands

#### `event add`
**Purpose**: Record event for a task

**Signature**:
```bash
intent-engine event add \
  --type <TYPE> \
  [--task-id <TASK_ID>] \
  [--data-stdin]
```

**Parameters**:
- `--type <TYPE>` (required): Event type (`decision`, `blocker`, `milestone`, `note`)
- `--task-id <TASK_ID>` (optional): Task ID to attach event to
  - **If omitted**: Uses current focused task
  - **If current task not set**: Returns error
- `--data-stdin` (optional): Read event data from stdin

**Design Note**: This command supports **flexible event recording**:
- **During active work**: Omit `--task-id` to record events for current task
- **Cross-task insights**: Use `--task-id` to record events for any task (e.g., project retrospectives)

**Output**: JSON
```json
{
  "id": 123,
  "task_id": 42,
  "log_type": "decision",
  "discussion_data": "Chose HS256 algorithm...",
  "timestamp": "2024-11-09T10:30:00Z"
}
```

---

#### `event list`
**Purpose**: List events for a task

**Signature**:
```bash
intent-engine event list <TASK_ID>
```

**Output**: JSON (array of events)

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
- `--since <DURATION>`: Time duration (e.g., `"7d"`, `"2h"`, `"30m"`)
- `--status <STATUS>`: Filter by status
- `--summary-only`: Return summary statistics only

**Output**: JSON
```json
{
  "summary": {
    "total_tasks": 50,
    "tasks_by_status": {
      "todo": 20,
      "doing": 5,
      "done": 25
    },
    "total_events": 150,
    "date_range": {
      "from": "2024-11-02T00:00:00Z",
      "to": "2024-11-09T11:00:00Z"
    }
  },
  "tasks": [...],
  "events": [...]
}
```

---

### 2.4 Workspace Commands

#### `current`
**Purpose**: Get or set current task

**Signature**:
```bash
# Get current task
intent-engine current

# Set current task
intent-engine current --set <TASK_ID>
```

**Output** (get):
```json
{
  "current_task_id": 42,
  "task": {
    "id": 42,
    "name": "Implement authentication",
    "status": "doing"
  }
}
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
use intent_engine::tasks::TaskManager;

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
    ).await?;

    // Start task (sets as current and doing)
    let started = task_mgr.start_task(task.id, true).await?;

    // Complete current task (focus-driven)
    // Note: complete_task operates on current_task_id from workspace
    task_mgr.complete_current_task().await?;

    Ok(())
}
```

### 4.3 API Documentation

Full API documentation available at: https://docs.rs/intent-engine

---

## 5. Output Formats

### 5.1 JSON Output (Default)

All CLI commands output structured JSON by default.

**Standard Success Format**:
```json
{
  // Command-specific data
  "id": 42,
  "name": "Task name",
  ...
}
```

**Error Format**:
```json
{
  "error": "Descriptive error message",
  "code": "ERROR_CODE"  // Optional
}
```

### 5.2 Special Output Structures

#### SearchResult (from `task search --snippet`)
```json
{
  "task_id": 42,
  "name": "Text with **highlighted** matches",
  "spec_snippet": "Snippet with **highlighted** terms...",
  "rank": 0.95
}
```

#### PickNextResult (from `task pick-next`)
```json
{
  "recommended_task": { ... },
  "reason": "subtask_of_current" | "top_level_task" | "no_todo_tasks",
  "context": { ... }
}
```

---

## 6. Interface Guarantees

### 6.1 Semantic Versioning

Intent-Engine follows [SemVer 2.0](https://semver.org/):

- **MAJOR** version: Breaking interface changes
- **MINOR** version: Backward-compatible additions
- **PATCH** version: Backward-compatible bug fixes

### 6.2 Stability Guarantees

| Version | CLI Interface | MCP Interface | Rust API | Status |
|---------|--------------|---------------|----------|--------|
| 0.1.x   | Experimental | Experimental  | Experimental | Current |
| 1.0.x   | Stable       | Stable        | Stable | Future |

**Current Status (0.1.9)**: All interfaces are **experimental** and may change.

**Experimental means**:
- Interface may change without major version bump
- Breaking changes documented in CHANGELOG
- No long-term compatibility guarantee

### 6.3 Deprecation Policy (Post-1.0)

For stable versions (≥1.0):
1. Deprecated features marked in documentation
2. Warning messages shown for 2 minor versions
3. Removal only in next major version

Example:
```
1.0.0: Feature X works normally
1.1.0: Feature X marked deprecated (warning shown)
1.2.0: Feature X still works (warning shown)
2.0.0: Feature X removed
```

---

## 7. Validation & Testing

### 7.1 Interface Consistency Tests

```bash
# Verify CLI commands match spec
cargo test --test cli_spec_test

# Verify MCP tools match spec
cargo test --test mcp_tools_sync_test

# Verify interface spec is up-to-date
cargo test --test interface_spec_test
```

### 7.2 Automated Sync

- **Version sync**: `scripts/sync-mcp-tools.sh` ensures version consistency
- **Tool list validation**: Tests verify JSON schema matches code implementation
- **CI enforcement**: All tests run on every PR

---

## 8. Migration Guide

### 8.1 Breaking Changes

All breaking changes documented in `CHANGELOG.md` with migration guide.

### 8.2 Version Matrix

| Intent-Engine | CLI Version | MCP Schema | Min Rust API |
|--------------|-------------|------------|--------------|
| 0.1.9        | 0.1.9       | 0.1.9      | 0.1.9        |

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
1. **Spec First**: Update this document for any interface changes
2. **Implementation**: Update code to match spec
3. **MCP Sync**: Update `mcp-server.json` if tools changed
4. **Auto-sync**: Run `./scripts/sync-mcp-tools.sh` for version
5. **Validate**: Run tests (`cargo test --test mcp_tools_sync_test --test interface_spec_test`)
6. **Document**: Update CHANGELOG.md

**Automated Sync**:
- Version synced by `scripts/sync-mcp-tools.sh`
- Tool list validated by `tests/mcp_tools_sync_test.rs`
- Spec consistency verified by `tests/interface_spec_test.rs`

---

**Specification Version**: 0.1.9
**Maintained by**: Intent-Engine Contributors
**License**: MIT OR Apache-2.0
