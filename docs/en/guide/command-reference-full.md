# Intent-Engine

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wayfind/intent-engine/branch/main/graph/badge.svg)](https://codecov.io/gh/wayfind/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![Documentation](https://docs.rs/intent-engine/badge.svg)](https://docs.rs/intent-engine)

Intent-Engine is a minimalist, project-specific command-line database service designed to record, track, and review human strategic intent. It's a core powerhouse in the AI collaborator's toolbox, helping answer two key questions: "Where are we going? (What)" and "Why are we going there? (Why)".

> 📖 **New User?** We recommend reading [The Intent-Engine Way](the-intent-engine-way.md) first to understand Intent-Engine's design philosophy and collaboration model. This document is a technical reference; that guide explains "why" and "when" to use it.

## Core Features

- **Simple Initialization**: Use `ie init` to initialize a directory, or let write commands auto-initialize in the current directory
- **Lazy Initialization**: Write commands automatically initialize in the current directory if needed
- **Task Management**: Support for task CRUD, hierarchical relationships, status tracking
  - **Priority and Complexity**: Support for task evaluation and sorting 🆕
  - **Smart Recommendation**: `pick-next` recommends next task based on context 🆕
  - **Subtask Management**: `spawn-subtask` atomically creates and switches 🆕
  - **Task Switching**: `switch` flexibly switches between multiple tasks 🆕
- **Event Logging**: Records task-related decisions, discussions, and milestones
- **Workspace State**: Tracks currently active task
- **Smart Reports**: Supports FTS5 full-text search and time range filtering
- **Token Optimization**: Atomic operations reduce API calls by 60-70% 🆕
- **JSON Output**: All output is structured JSON, easy for AI and tool integration

## Installation

> 📖 **Complete Installation Guide**: See [INSTALLATION.md](installation.md) for detailed instructions on all installation methods, troubleshooting, and maintainer release process.

### Method 1: Cargo Install (Recommended) 🚀

If you already have Rust and Cargo installed, this is the simplest installation method:

```bash
# Install latest version from crates.io
cargo install intent-engine

# Verify installation
ie --version
```

**Don't have Rust?** Install Rust first:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Method 2: Homebrew (macOS/Linux) 🍺

```bash
# Coming soon
brew install wayfind/tap/intent-engine
```

### Method 3: cargo-binstall (Fast Pre-compiled Binary Install) ⚡

Use [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) to directly install pre-compiled binaries, much faster than building from source:

```bash
# Install cargo-binstall (if not already installed)
cargo install cargo-binstall

# Install intent-engine (automatically downloads from GitHub Releases)
cargo binstall intent-engine
```

### Method 4: Download Pre-compiled Binaries

Download the binary for your platform from [GitHub Releases](https://github.com/wayfind/intent-engine/releases):

- **Linux**: `intent-engine-linux-x86_64.tar.gz` or `intent-engine-linux-aarch64.tar.gz`
- **macOS**: `intent-engine-macos-x86_64.tar.gz` or `intent-engine-macos-aarch64.tar.gz`
- **Windows**: `intent-engine-windows-x86_64.zip`

```bash
# Extract and install
tar xzf intent-engine-*.tar.gz
sudo mv intent-engine /usr/local/bin/

# Verify installation
ie --version
```

### Method 5: Build from Source

```bash
# Clone repository
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine

# Build and install
cargo install --path .

# Or build manually
cargo build --release
sudo cp target/release/intent-engine /usr/local/bin/
```

### Method 6: Integrate with Claude Code via System Prompt (v0.10.0+)

Intent-Engine integrates with Claude Code using a system prompt approach (MCP removed in v0.10.0).

```bash
# No installation needed - zero configuration!
# Claude Code automatically detects Intent-Engine when installed
ie --version  # Should be v0.10.0+
```

For detailed configuration instructions, see [System Prompt Guide](../integration/claude-code-system-prompt.md).

### Method 7: As Claude Code Skill

For lightweight integration, configure Intent-Engine as a Claude Code skill:

```bash
# Skill configuration file is included in the project
# Location: .claude-code/intent-engine.skill.md
# Claude Code will automatically recognize it
```

## Quick Start

### Typical Workflow

```bash
# 1. Add main task
ie task add --name "Implement user authentication feature" | jq -r '.id'
# Output: 1

# 2. Start task and view details
ie task start 1 --with-events

# 3. Discover problem, create subtask
ie task spawn-subtask --name "Fix password validation bug"

# 4. Record key decision
echo "Decided to use bcrypt instead of MD5" | ie event add --task-id 2 --type decision --data-stdin

# 5. Complete subtask
ie task done

# 6. Switch back to parent task
ie task switch 1

# 7. Complete parent task
ie task done

# 8. Generate work report
ie report --since 1d --summary-only
```

## Command Reference

### Task Management Commands

#### `task add` - Add Task

Create new task, supporting parent task and specification.

**Usage:**
```bash
ie task add --name <NAME> [OPTIONS]
```

**Parameters:**
- `--name <NAME>` - Task name (required)
- `--parent <ID>` - Parent task ID (optional)
- `--spec-stdin` - Read specification from stdin (optional)

**Examples:**
```bash
# Add simple task
ie task add --name "Implement user login"

# Add task with specification
echo "Use JWT token, valid for 7 days, support refresh" | \
  ie task add --name "JWT authentication" --spec-stdin

# Add subtask
ie task add --name "Write unit tests" --parent 1

# Read specification from file
cat design.md | ie task add --name "Design review" --spec-stdin
```

**Output Example:**
```json
{
  "id": 1,
  "parent_id": null,
  "name": "Implement user login",
  "status": "todo",
  "priority": 0,
  "first_todo_at": "2025-11-06T10:00:00Z",
  "first_doing_at": null,
  "first_done_at": null
}
```

---

#### `task list` - Find Tasks

Find tasks, supporting filtering by status and parent.

**Usage:**
```bash
ie task list [OPTIONS]
```

**Parameters:**
- `--status <STATUS>` - Filter by status: todo/doing/done (optional)
- `--parent <PARENT>` - Filter by parent: task ID or "null" (optional)

**Examples:**
```bash
# Find all tasks
ie task list

# Find tasks in progress
ie task list doing

# Find completed tasks
ie task list done

# Find all subtasks of specific parent task
ie task list --parent 1

# Find all root tasks (no parent)
ie task list --parent null

# Combined query: find subtasks of task 1 that are in progress
ie task list doing --parent 1
```

**Output Example:**
```json
[
  {
    "id": 1,
    "parent_id": null,
    "name": "Implement user login",
    "status": "doing",
    "priority": 5,
    "complexity": 7,
    "first_todo_at": "2025-11-06T10:00:00Z",
    "first_doing_at": "2025-11-06T10:30:00Z",
    "first_done_at": null
  },
  {
    "id": 2,
    "parent_id": 1,
    "name": "Write unit tests",
    "status": "todo",
    "priority": 3,
    "first_todo_at": "2025-11-06T11:00:00Z",
    "first_doing_at": null,
    "first_done_at": null
  }
]
```

---

#### `task get` - Get Task Details

Get detailed information for a single task, optionally including associated event summary.

**Usage:**
```bash
ie task get <ID> [OPTIONS]
```

**Parameters:**
- `<ID>` - Task ID (required)
- `--with-events` - Include event summary (optional)

**Examples:**
```bash
# Get basic information
ie task get 1

# Get complete information with event summary
ie task get 1 --with-events

# Extract specific fields using jq
ie task get 1 | jq -r '.name'
ie task get 1 --with-events | jq '.events_summary'
```

**Output Example (without events):**
```json
{
  "id": 1,
  "parent_id": null,
  "name": "Implement user login",
  "spec": "Use JWT token, valid for 7 days",
  "status": "doing",
  "complexity": 7,
  "priority": 5,
  "first_todo_at": "2025-11-06T10:00:00Z",
  "first_doing_at": "2025-11-06T10:30:00Z",
  "first_done_at": null
}
```

**Output Example (with events):**
```json
{
  "task": {
    "id": 1,
    "name": "Implement user login",
    "status": "doing",
    "..."
  },
  "events_summary": {
    "total_count": 3,
    "by_type": {
      "decision": 2,
      "blocker": 1
    },
    "recent_events": [
      {
        "id": 3,
        "log_type": "decision",
        "discussion_data": "Decided to use bcrypt instead of MD5",
        "timestamp": "2025-11-06T11:00:00Z"
      }
    ]
  }
}
```

---

#### `task update` - Update Task

Update task properties, including name, parent, status, complexity and priority.

**Usage:**
```bash
ie task update <ID> [OPTIONS]
```

**Parameters:**
- `<ID>` - Task ID (required)
- `--name <NAME>` - New name (optional)
- `--parent <PARENT_ID>` - New parent task ID (optional)
- `--status <STATUS>` - New status: todo/doing/done (optional)
- `--complexity <1-10>` - Task complexity 1-10 (optional)
- `--priority <N>` - Task priority, higher number = higher priority (optional)
- `--spec-stdin` - Read new specification from stdin (optional)

**Examples:**
```bash
# Update task name
ie task update 1 --name "Implement OAuth2 login"

# Set task complexity and priority
ie task update 1 --complexity 8 --priority 10

# Update task status
ie task update 1 --status doing

# Change parent task
ie task update 3 --parent 2

# Update specification
echo "New implementation: use OAuth2 + PKCE" | \
  ie task update 1 --spec-stdin

# Combined update
ie task update 1 \
  --name "Optimize login performance" \
  --complexity 5 \
  --priority 8 \
  --status doing
```

**Output Example:**
```json
{
  "id": 1,
  "parent_id": null,
  "name": "Implement OAuth2 login",
  "status": "doing",
  "complexity": 8,
  "priority": 10,
  "first_todo_at": "2025-11-06T10:00:00Z",
  "first_doing_at": "2025-11-06T10:30:00Z",
  "first_done_at": null
}
```

---

#### `task start` - Start Task

Atomic operation: update task status to "doing" and set as current task.

**Usage:**
```bash
ie task start <ID> [OPTIONS]
```

**Parameters:**
- `<ID>` - Task ID (required)
- `--with-events` - Include event summary (optional)

**Examples:**
```bash
# Start task
ie task start 1

# Start task and get historical context
ie task start 1 --with-events

# Typical AI workflow: understand background before starting
ie task start 1 --with-events | jq '.events_summary.recent_events'
```

**Output Example:**
```json
{
  "id": 1,
  "name": "Implement user login",
  "status": "doing",
  "first_doing_at": "2025-11-06T10:30:00Z",
  "..."
}
```

---

#### `task done` - Complete Task

Atomic operation: check if all subtasks are complete, then mark task as "done".

This command does not accept any ID parameter. It operates on the current focused task (`current_task_id`) only.

**Usage:**
```bash
ie task done
```

**Parameters:**
- None (operates on current focused task)

**Examples:**
```bash
# Complete task
ie task done

# If there are incomplete subtasks, returns error
ie task done
# Error: Cannot complete task 1: it has incomplete subtasks
```

**Output Example:**
```json
{
  "id": 1,
  "name": "Implement user login",
  "status": "done",
  "first_done_at": "2025-11-06T12:00:00Z",
  "..."
}
```

---

#### `task del` - Delete Task

Delete task and all its subtasks (cascade delete).

**Usage:**
```bash
ie task del <ID>
```

**Parameters:**
- `<ID>` - Task ID (required)

**Examples:**
```bash
# Delete task
ie task del 1

# Deletion cascades to all subtasks
ie task del 1  # Deletes task 1 and all its subtasks
```

**Output Example:**
```json
{
  "success": true,
  "message": "Task 1 deleted"
}
```

---

#### `task pick-next` - Intelligently Recommend Next Task 🆕

Based on a context-aware priority model, intelligently recommends the single most appropriate task to work on next. This command is non-interactive and does not modify task status.

**Core Philosophy**: Complete the current ongoing topic depth-first, then start a new topic.

**Usage:**
```bash
ie task pick-next [--format <FORMAT>]
```

**Parameters:**
- `--format <FORMAT>` - Output format (default: `text`)
  - `text`: Human-friendly guidance format
  - `json`: Structured JSON format for AI Agents

**Smart Recommendation Logic:**
1. **First Priority**: Subtasks of current focused task (depth-first)
   - Find all `status=todo` subtasks of `current_task_id`
   - Sort by `priority ASC` (lower number = higher priority), then `id ASC`
2. **Second Priority**: Top-level tasks (breadth-first)
   - Find all `parent_id IS NULL` and `status=todo` tasks
   - Sort by `priority ASC`, then `id ASC`
3. **No Recommendation**: Return appropriate empty state response with exit code 1

**Examples:**

```bash
# Text format (default) - Human-friendly
ie task pick-next

# Output example:
# Based on your current focus, the recommended next task is:
#
# [ID: 43] [Priority: 1] [Status: todo]
# Name: Design database schema for user identities
#
# To start working on it, run:
#   ie task start 43

# JSON format - AI Agent friendly
ie task pick-next --format json
```

**JSON Output Example (with recommendation):**
```json
{
  "suggestion_type": "FOCUSED_SUB_TASK",
  "task": {
    "id": 43,
    "parent_id": 4,
    "name": "Design database schema for user identities",
    "spec": "Detailed specification content...",
    "status": "todo",
    "priority": 1,
    "complexity": null,
    "first_todo_at": "2025-11-08T10:30:00Z",
    "first_doing_at": null,
    "first_done_at": null
  }
}
```

**JSON Output Example (empty state - no tasks):**
```json
{
  "suggestion_type": "NONE",
  "reason_code": "NO_TASKS_IN_PROJECT",
  "message": "No tasks found in this project. Your intent backlog is empty."
}
```

**JSON Output Example (empty state - all completed):**
```json
{
  "suggestion_type": "NONE",
  "reason_code": "ALL_TASKS_COMPLETED",
  "message": "Project Complete! All intents have been realized."
}
```

**Suggestion Types:**
- `FOCUSED_SUB_TASK` - Recommends subtask of current focused task
- `TOP_LEVEL_TASK` - Recommends top-level task
- `NONE` - No recommendation (with reason_code explaining why)

**Exit Codes:**
- `0` - Successfully found recommended task
- `1` - No recommendation (empty state)

**Use Cases:**
- AI Agents get next task to work on at the start of each session
- Human users check system-recommended next steps
- Automation scripts make decisions based on recommended tasks

---

#### `task spawn-subtask` - Create Subtask and Switch 🆕

Create subtask under current task and automatically switch to new subtask (atomic operation).

**Usage:**
```bash
ie task spawn-subtask --name <NAME> [OPTIONS]
```

**Parameters:**
- `--name <NAME>` - Subtask name (required)
- `--spec-stdin` - Read specification from stdin (optional)

**Prerequisites:**
- Must have current task (set via `current --set` or `task start`)

**Atomic Operation Flow:**
1. Check current task
2. Create subtask (parent_id = current_task_id)
3. Set subtask status to doing
4. Set subtask as current task

**Examples:**
```bash
# 1. First start a parent task
ie task start 1

# 2. Discover need to handle sub-problem during work
ie task spawn-subtask --name "Fix dependency version conflict"

# 3. Subtask with specification
echo "Need to upgrade tokio to 1.35" | \
  ie task spawn-subtask --name "Upgrade dependencies" --spec-stdin

# Typical scenario: recursive problem decomposition
ie task start 1  # Start: implement user authentication
ie task spawn-subtask --name "Implement password encryption"  # Discover sub-problem
ie task spawn-subtask --name "Choose encryption algorithm"  # Discover even finer sub-problem
ie task done  # Complete: choose encryption algorithm
ie task switch 2  # Switch back: implement password encryption
ie task done  # Complete: implement password encryption
ie task switch 1  # Switch back: implement user authentication
ie task done  # Complete: implement user authentication
```

**Output Example:**
```json
{
  "id": 2,
  "parent_id": 1,
  "name": "Fix dependency version conflict",
  "status": "doing",
  "priority": 0,
  "first_todo_at": "2025-11-06T10:35:00Z",
  "first_doing_at": "2025-11-06T10:35:00Z",
  "first_done_at": null
}
```

**Use Cases:**
- AI discovers sub-problems that need solving first while working on task
- Keep task hierarchy clear, avoid flattening all tasks
- Enforce completing subtasks before completing parent task

---

#### `task switch` - Switch Task 🆕

Atomic operation: update task status to doing (if not already) and set as current task.

**Usage:**
```bash
ie task switch <ID>
```

**Parameters:**
- `<ID>` - Task ID (required)

**Atomic Operation Flow:**
1. Verify task exists
2. If status is not doing, update to doing
3. Set as current task
4. Return task details and event summary

**Examples:**
```bash
# Switch to task 2
ie task switch 2

# Switch between multiple tasks
ie task start 1
ie task spawn-subtask --name "Subtask A"
ie task spawn-subtask --name "Subtask B"
ie task switch 2  # Switch back to subtask A
ie task done
ie task switch 3  # Switch to subtask B

# View context after switching
ie task switch 5 | jq '.events_summary'
```

**Output Example:**
```json
{
  "task": {
    "id": 2,
    "parent_id": 1,
    "name": "Implement password encryption",
    "status": "doing",
    "first_doing_at": "2025-11-06T10:40:00Z",
    "..."
  },
  "events_summary": {
    "total_count": 2,
    "by_type": {
      "decision": 1,
      "milestone": 1
    },
    "recent_events": [...]
  }
}
```

**Use Cases:**
- Switch between multiple parallel tasks
- Pause current task to handle more urgent task
- Switch back to parent task after completing subtask

---

#### `task search` - Full-Text Search Tasks 🆕

Use FTS5 full-text search to find content in all tasks' name and spec fields, returning a relevance-sorted result list.

**Usage:**
```bash
ie task search <QUERY>
```

**Parameters:**
- `<QUERY>` - Search query string (required), supports FTS5 advanced syntax

**FTS5 Advanced Syntax:**
- `authentication` - Simple keyword search
- `"user login"` - Exact phrase search
- `authentication AND bug` - Contains both words
- `JWT OR OAuth` - Contains either word
- `authentication NOT critical` - Contains authentication but not critical
- `auth*` - Prefix matching (e.g., auth, authentication, authorize)

**Features:**
- Searches both name and spec fields
- Returns results with highlighted snippets (using `**` markers)
- Automatically sorted by relevance
- Millisecond-level query performance (based on FTS5 index)

**Examples:**
```bash
# Simple search
ie task search "authentication"

# Search for tasks containing JWT
ie task search "JWT"

# Advanced search: contains both keywords
ie task search "authentication AND bug"

# Search for either keyword
ie task search "JWT OR OAuth"

# Exclude specific keyword
ie task search "bug NOT critical"

# Prefix matching
ie task search "auth*"

# Exact phrase search
ie task search '"user login flow"'

# Combine with jq to view results
ie task search "authentication" | jq '.[].task | {id, name, status}'

# View match snippets
ie task search "JWT" | jq '.[].match_snippet'
```

**Output Example:**
```json
[
  {
    "id": 5,
    "parent_id": 1,
    "name": "Authentication bug fix",
    "spec": "Fix the JWT token validation bug in the authentication middleware",
    "status": "todo",
    "complexity": 5,
    "priority": 8,
    "first_todo_at": "2025-11-06T10:00:00Z",
    "first_doing_at": null,
    "first_done_at": null,
    "match_snippet": "...Fix the **JWT** token validation bug in the **authentication** middleware..."
  },
  {
    "id": 12,
    "parent_id": null,
    "name": "Implement OAuth2 authentication",
    "spec": "Add OAuth2 support for third-party authentication",
    "status": "doing",
    "priority": 10,
    "first_todo_at": "2025-11-05T15:00:00Z",
    "first_doing_at": "2025-11-06T09:00:00Z",
    "first_done_at": null,
    "match_snippet": "Implement OAuth2 **authentication**"
  }
]
```

**match_snippet Field Explanation:**
- Text snippet extracted from the matching field (spec or name)
- Uses `**keyword**` to mark highlighted matches
- Uses `...` to indicate omitted content
- Prioritizes spec matches; if spec doesn't match, shows name matches

**Use Cases:**
- Quickly find tasks containing specific keywords
- Locate related tasks in large projects
- Search for previous decisions and technical approaches
- AI context lookup
- Find related tasks during code review

**Difference from `task list`:**
- `task list`: Exact filtering (by status, parent), returns complete task list
- `task search`: Full-text search (by content keywords), returns results with match snippets, sorted by relevance

---

#### `task depends-on` - Add Task Dependency

Create a dependency between two tasks.

**Usage:**
```bash
ie task depends-on <BLOCKED_TASK_ID> <BLOCKING_TASK_ID>
```

**Parameters:**
- `<BLOCKED_TASK_ID>` - Task that has the dependency (blocked task)
- `<BLOCKING_TASK_ID>` - Task that must be completed first (blocking task)

**Logic**: `depends-on A B` means Task A depends on Task B (Task B must be completed before Task A can start)

**Examples:**
```bash
# Task 42 depends on Task 41 (41 must complete before 42 can start)
ie task depends-on 42 41

# Real scenario: API client implementation depends on authentication completion
ie task add --name "Implement authentication system"   # Task 1
ie task add --name "Implement API client"            # Task 2
ie task depends-on 2 1                               # Task 2 depends on Task 1

# Verify dependency
ie task start 2  # Will fail if Task 1 is not completed
```

**Output Example:**
```json
{
  "success": true,
  "dependency": {
    "blocked_task_id": 42,
    "blocking_task_id": 41,
    "message": "Task 42 now depends on Task 41"
  }
}
```

**Use Cases:**
- Define task completion order
- Ensure prerequisites are met
- Project dependency management

---

### Event Logging Commands

#### `event add` - Add Event

Record event for task (decisions, blockers, milestones, etc.).

**Usage:**
```bash
ie event add [--task-id <ID>] --type <TYPE> --data-stdin
```

**Parameters:**
- `--task-id <ID>` - Task ID (optional, uses current task if omitted)
- `--type <TYPE>` - Event type (required), suggested values:
  - `decision` - Key decision
  - `blocker` - Encountered obstacle
  - `milestone` - Milestone
  - `discussion` - Discussion record
  - `note` - General note
- `--data-stdin` - Read event content from stdin (required)

**Examples:**
```bash
# Record to current task (concise workflow)
echo "Decided to use bcrypt instead of MD5 for password encryption" | \
  ie event add --type decision --data-stdin

# Record to specific task (flexible workflow)
echo "Found bcrypt library fails to compile on Windows, need alternative" | \
  ie event add --task-id 1 --type blocker --data-stdin

# Record milestone to current task
echo "Completed core encryption logic, all unit tests passing" | \
  ie event add --type milestone --data-stdin

# Record from file to specific task
cat discussion_notes.md | \
  ie event add --task-id 1 --type discussion --data-stdin

# Record long text to current task
echo "After research, compared the following options:
1. bcrypt - Industry standard, but poor Windows compatibility
2. argon2 - More secure, but higher performance overhead
3. scrypt - Balanced approach

Final decision: Use argon2, accept performance overhead" | \
  ie event add --type decision --data-stdin
```

**Output Example:**
```json
{
  "id": 1,
  "task_id": 1,
  "timestamp": "2025-11-06T11:00:00Z",
  "log_type": "decision",
  "discussion_data": "Decided to use bcrypt instead of MD5 for password encryption"
}
```

---

#### `event list` - List Events

List event history for specified task.

**Usage:**
```bash
ie event list --task-id <ID> [OPTIONS]
```

**Parameters:**
- `--task-id <ID>` - Task ID (required)
- `--limit <N>` - Limit returned count (optional, default returns all)

**Examples:**
```bash
# List all events
ie event list --task-id 1

# View only most recent 5
ie event list --task-id 1 --limit 5

# View only decision type events
ie event list --task-id 1 | jq '.[] | select(.log_type == "decision")'

# View latest decision
ie event list --task-id 1 --limit 10 | \
  jq '.[] | select(.log_type == "decision") | .discussion_data' | head -1

# Used when AI recovers context
ie event list --task-id 1 --limit 10 | \
  jq '[.[] | {type: .log_type, data: .discussion_data, time: .timestamp}]'
```

**Output Example:**
```json
[
  {
    "id": 3,
    "task_id": 1,
    "timestamp": "2025-11-06T12:00:00Z",
    "log_type": "milestone",
    "discussion_data": "Completed core encryption logic"
  },
  {
    "id": 2,
    "task_id": 1,
    "timestamp": "2025-11-06T11:30:00Z",
    "log_type": "blocker",
    "discussion_data": "Found bcrypt library fails to compile on Windows"
  },
  {
    "id": 1,
    "task_id": 1,
    "timestamp": "2025-11-06T11:00:00Z",
    "log_type": "decision",
    "discussion_data": "Decided to use bcrypt for password encryption"
  }
]
```

---

### Workspace Commands

#### `current` - Current Task Management

View or set currently active task.

**Usage:**
```bash
# View current task
ie current

# Set current task
ie current --set <ID>
```

**Parameters:**
- `--set <ID>` - Set current task ID (optional)

**Examples:**
```bash
# View current task
ie current

# Set current task
ie current --set 2

# View current task name
ie current | jq -r '.task.name'

# Check if there is current task
ie current &>/dev/null && echo "Has current task" || echo "No current task"

# Clear current task (currently requires manual database operation)
# Note: Usually not needed, start/switch/spawn-subtask will auto-update
```

**Output Example (with current task):**
```json
{
  "current_task_id": 2,
  "task": {
    "id": 2,
    "parent_id": 1,
    "name": "Implement password encryption",
    "status": "doing",
    "..."
  }
}
```

**Output Example (no current task):**
```json
{
  "current_task_id": null,
  "message": "No current task"
}
```

---

## System Utility Commands

#### `setup` - Unified Setup Command

Unified configuration interface for AI tool integrations, supporting both hook installation and MCP server configuration.

**Usage:**
```bash
ie setup [OPTIONS]
```

**Options:**
- `--target <TARGET>` - Target tool: claude-code, gemini-cli, codex
- `--scope <SCOPE>` - Installation scope: user (default recommended), project, or both
- `--dry-run` - Preview mode, show what would be done without executing
- `--force` - Force overwrite existing configuration
- `--config-path <CONFIG_PATH>` - Custom config file path (advanced)

**Features:**
- Support for user-level or project-level installation
- Atomic operations with rollback on failure
- Built-in connectivity testing

**Examples:**
```bash
# Setup Claude Code integration at user level (recommended)
ie setup --target claude-code

# Preview what will be done
ie setup --target claude-code --dry-run

# Force reinstall
ie setup --target claude-code --force

# Project-level installation
ie setup --target claude-code --scope project
```

**Output Example:**
```json
{
  "success": true,
  "target": "claude-code",
  "scope": "user",
  "actions": [
    "Created MCP server configuration",
    "Tested connectivity",
    "Updated Claude Code config"
  ]
}
```

---

#### `doctor` - System Health Check

Check system health and dependencies.

**Usage:**
```bash
ie doctor
```

**Features:**
- Verify Intent-Engine installation
- Check MCP server configuration
- Test database connectivity
- Validate Claude Code integration status
- Provide repair suggestions

**Examples:**
```bash
# Run system health check
ie doctor

# Example output:
# ✓ Intent-Engine installation OK
# ✓ SQLite database connection OK
# ⚠️  MCP server not configured
# 💡 Suggestion: Run 'ie setup --target claude-code'
```

---

### Report Commands

#### `report` - Generate Work Report

Generate task work report, supporting time range, status filtering and full-text search.

**Usage:**
```bash
ie report [OPTIONS]
```

**Parameters:**
- `--summary-only` - Generate summary only (recommended, saves tokens)
- `--since <DURATION>` - Time range: 1h/6h/1d/7d/30d (optional)
- `--status <STATUS>` - Filter by status: todo/doing/done (optional)
- `--filter-name <KEYWORD>` - Search by task name (FTS5) (optional)
- `--filter-spec <KEYWORD>` - Search by specification (FTS5) (optional)

**Examples:**
```bash
# Generate complete report
ie report

# Generate summary only (recommended)
ie report --summary-only

# View last 1 day of work
ie report --since 1d --summary-only

# View last 7 days of work
ie report --since 7d --summary-only

# View completed tasks
ie report --status done --summary-only

# View tasks in progress
ie report --status doing --summary-only

# Search for tasks containing "authentication"
ie report --filter-name "authentication" --summary-only

# Search for tasks with "JWT" in specification
ie report --filter-spec "JWT" --summary-only

# Combined query: authentication-related tasks completed in last 7 days
ie report --since 7d --status done --filter-name "authentication" --summary-only

# AI generate daily report
ie report --since 1d --summary-only | \
  jq -r '.summary | "Completed \(.done_count) tasks today, \(.doing_count) in progress"'

# View task details
ie report --since 7d | jq '.tasks[] | {name, status, started: .first_doing_at}'
```

**Output Example (summary-only):**
```json
{
  "summary": {
    "total_count": 15,
    "todo_count": 5,
    "doing_count": 3,
    "done_count": 7,
    "time_range": {
      "since": "7d",
      "from": "2025-10-30T10:00:00Z",
      "to": "2025-11-06T10:00:00Z"
    }
  },
  "filters": {
    "status": null,
    "name_keyword": null,
    "spec_keyword": null
  }
}
```

**Output Example (full report):**
```json
{
  "summary": {
    "total_count": 3,
    "todo_count": 1,
    "doing_count": 1,
    "done_count": 1
  },
  "tasks": [
    {
      "id": 1,
      "name": "Implement user authentication",
      "status": "done",
      "first_todo_at": "2025-11-06T10:00:00Z",
      "first_doing_at": "2025-11-06T10:30:00Z",
      "first_done_at": "2025-11-06T12:00:00Z"
    },
    {
      "id": 2,
      "name": "Write unit tests",
      "status": "doing",
      "first_todo_at": "2025-11-06T11:00:00Z",
      "first_doing_at": "2025-11-06T11:30:00Z",
      "first_done_at": null
    },
    {
      "id": 3,
      "name": "Performance optimization",
      "status": "todo",
      "first_todo_at": "2025-11-06T12:00:00Z",
      "first_doing_at": null,
      "first_done_at": null
    }
  ]
}
```

---

## Real-World Scenario Examples

### Scenario 1: AI Discovers Multiple Issues and Batch Processes

```bash
# 1. AI discovers 5 issues during code review
ie task add --name "Fix null pointer exception"
ie task add --name "Optimize database query"
ie task add --name "Update outdated dependencies"
ie task add --name "Fix memory leak"
ie task add --name "Add error logging"

# 2. AI evaluates priority for each task (lower number = higher priority)
ie task update 1 --priority 1   # Null pointer: most urgent
ie task update 2 --priority 2   # Database: second priority
ie task update 3 --priority 5   # Dependencies: medium
ie task update 4 --priority 1   # Memory: most urgent
ie task update 5 --priority 10  # Logging: not urgent

# 3. Get smart recommendation
ie task pick-next --format json
# Result: Recommends task 1 (priority=1, smallest ID)

# 4. Start processing recommended task
ie task start 1
echo "Cause: Did not check for null return value" | ie event add --task-id 1 --type note --data-stdin
ie task done  # Complete current focused task

# 5. Get next recommendation
ie task pick-next --format json
# Result: Recommends task 4 (priority=1, second smallest ID)

ie task start 4
echo "Decision: Use smart pointers to avoid memory leak" | ie event add --task-id 4 --type decision --data-stdin
ie task done

# 6. Generate report
ie report --since 1d --summary-only
```

### Scenario 2: Recursive Task Decomposition

```bash
# 1. Start a major task
ie task add --name "Implement payment system"
ie task start 1 --with-events

# 2. Discover need to do authentication first
ie task spawn-subtask --name "Integrate third-party payment API"
# Current task automatically switches to task 2

# 3. While integrating API, discover need to configure keys first
ie task spawn-subtask --name "Configure payment keys and callback URL"
# Current task automatically switches to task 3

# 4. Complete deepest subtask
echo "Configured Stripe API keys in backend" | ie event add --task-id 3 --type milestone --data-stdin
ie task done

# 5. Switch back to parent task and continue
ie task switch 2
echo "API integration complete, tests passing" | ie event add --task-id 2 --type milestone --data-stdin
ie task done

# 6. Complete root task
ie task switch 1
ie task done

# 7. View task hierarchy
ie task find --parent null  # Root tasks
ie task find --parent 1     # Subtasks
```

### Scenario 3: Parallel Task Management

```bash
# 1. Create multiple independent tasks
ie task add --name "Frontend: Implement login page"
ie task add --name "Backend: Implement API endpoints"
ie task add --name "Docs: Update API documentation"

# 2. Get recommendation and start first task
ie task pick-next --format json
# Recommends: task 1
ie task start 1

# 3. Switch between tasks
# ... do some frontend work ...
echo "Completed UI layout" | ie event add --task-id 1 --type milestone --data-stdin

ie task switch 2
# ... do some backend work ...
echo "Completed database models" | ie event add --task-id 2 --type milestone --data-stdin

ie task switch 3
# ... update docs ...
ie task done

# 4. View progress
ie report --status doing
```

## Project Structure

```
veobd/
├── src/
│   ├── main.rs          # Main entry and command dispatch
│   ├── lib.rs           # Library entry
│   ├── cli.rs           # CLI command definitions
│   ├── error.rs         # Error type definitions
│   ├── project.rs       # Project context discovery
│   ├── tasks.rs         # Task management logic
│   ├── events.rs        # Event logging logic
│   ├── workspace.rs     # Workspace state management
│   ├── report.rs        # Report generation logic
│   ├── test_utils.rs    # Test utilities
│   └── db/
│       ├── mod.rs       # Database connection and migration
│       └── models.rs    # Data model definitions
├── tests/               # Integration tests
│   ├── cli_tests.rs
│   ├── performance_tests.rs
│   ├── special_chars_tests.rs
│   └── cli_special_chars_tests.rs
├── benches/             # Performance benchmarks
│   └── performance.rs
├── Cargo.toml           # Project configuration
├── README.md            # Main documentation
├── PERFORMANCE.md       # Performance report
├── SPECIAL_CHARS.md     # Special character handling docs
└── .intent-engine/      # Project data directory (auto-created)
    └── project.db       # SQLite database
```

## Database Schema

### tasks table
- `id`: Task ID (primary key, auto-increment)
- `parent_id`: Parent task ID (optional, foreign key)
- `name`: Task name (required)
- `spec`: Task specification (optional)
- `status`: Task status (todo/doing/done, default todo)
- `complexity`: Task complexity (1-10, optional) 🆕
- `priority`: Task priority (integer, default 0) 🆕
- `first_todo_at`: First time set to todo
- `first_doing_at`: First time set to doing
- `first_done_at`: First time set to done

### events table
- `id`: Event ID
- `task_id`: Associated task ID
- `timestamp`: Event timestamp
- `log_type`: Event type (decision/blocker/milestone, etc.)
- `discussion_data`: Event detailed content

### workspace_state table
- `key`: State key (e.g., current_task_id)
- `value`: State value

## AI Client Usage Guide

### Task Lifecycle SOP

#### Basic Workflow
1. **Start task**: Use `task start <ID> --with-events` to get context
2. **Discover sub-problem**: Use `task spawn-subtask --name "sub-problem"` to create and switch
3. **Record key information**: Use `event add` to record decisions, blockers and milestones
4. **Complete task**: Use `task done` to mark complete (automatically checks subtasks)

#### Batch Problem Processing Workflow 🆕
1. **Discover problems**: Batch create todo tasks
2. **Evaluate tasks**: Use `task update` to set priority (lower number = higher priority)
3. **Smart recommendation**: Use `task pick-next` to get next task to work on
4. **Start task**: Use `task start` to begin recommended task
5. **Repeat**: After completion, call `pick-next` again for next recommendation

### Token Optimization Strategy 🆕

Using new atomic operation commands can significantly reduce token consumption:

| Traditional Workflow | Token Cost | Optimized Workflow | Token Cost | Savings |
|---------------------|------------|-------------------|------------|---------|
| find + get | 2 calls | `pick-next --format json` | 1 call | **50%** |
| add + start + set current | 3 calls | `spawn-subtask` | 1 call | **67%** |
| update + set current + get | 3 calls | `switch` | 1 call | **67%** |

### Relationship with Native Task System

- **Intent-Engine tasks**: Strategic intent, coarse granularity, long lifecycle
- **Native tasks (/todos)**: Tactical steps, fine granularity, short lifecycle

Intent-Engine tasks drive creation of native tasks.

### Best Practices

#### When Starting Work
1. Use `task start --with-events` to get goals and historical context
2. If multiple issues discovered, create todo tasks and set priority/complexity
3. Use `task pick-next` to automatically select optimal task order

#### During Work
1. Use `spawn-subtask` when discovering sub-problems, keep hierarchy clear
2. Use `event add` to record thinking process when making key decisions
3. Use `task switch` to flexibly switch between multiple tasks

#### When Ending Work
1. Use `report --summary-only` to generate efficient summary (saves tokens)
2. Use `report --since 1d` to view day's work progress

#### When Resuming Work
1. Use `current` to view currently active task
2. Use `task get <ID> --with-events` to get complete context
3. Use `event list` to refresh memory

## Technology Stack

- **Language**: Rust 2021
- **CLI**: clap 4.5
- **Database**: SQLite with sqlx 0.7
- **Async Runtime**: tokio 1.35
- **Serialization**: serde + serde_json
- **Full-text Search**: SQLite FTS5

## Testing

Intent-Engine includes a complete testing system ensuring code quality and reliability.

### Running Tests

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test cli_tests

# Run specific test
cargo test test_add_task
```

### Test Coverage

- **Unit Tests** (47 tests):
  - Task management: 30 tests (CRUD, hierarchy, status management, circular dependency detection, priority/complexity, pick_next, spawn_subtask, switch)
  - Event logging: 6 tests (add, list, filter)
  - Workspace state: 5 tests (get, set, update)
  - Report generation: 6 tests (summary, full, filter, FTS5 search)

- **Integration Tests** (22 CLI tests):
  - Basic CRUD operation tests
  - Task state transition tests
  - Task hierarchy and parent-child relationship tests
  - Project awareness and context discovery tests (4)
  - New workflow tests: pick-next, spawn-subtask, switch (4)
  - JSON output format validation

- **Special Character Tests** (10 CLI tests + unit tests):
  - SQL injection protection tests
  - Unicode and Emoji support tests
  - Edge cases and extreme input tests

**Total**: 116 tests all passing ✅

### Test Architecture

- `src/test_utils.rs`: Test helper tools and context management
- `tests/cli_tests.rs`: CLI integration tests
- `#[cfg(test)]` modules in each module: Unit tests

All tests use temporary databases ensuring test isolation and repeatability.

## Performance Testing

Intent-Engine includes a complete performance testing suite verifying system behavior under extreme conditions.

### Running Performance Tests

```bash
# Run all performance tests (takes significant time)
cargo test --test performance_tests -- --ignored --nocapture

# Run specific performance test
cargo test --test performance_tests test_deep_task_hierarchy -- --ignored --nocapture
cargo test --test performance_tests test_massive_tasks_10k -- --ignored --nocapture

# Run benchmarks
cargo bench --bench performance
```

### Performance Metrics Summary

- **Deep Hierarchy**: Supports 100+ level task hierarchies, creation ~343ms, query <1ms
- **Massive Tasks**: 10,000 tasks creation ~33s, find ~257ms
- **Massive Events**: 10,000 events per task, limited query <32ms
- **FTS5 Search**: Search across 5,000 tasks, average ~32ms
- **Report Generation**: 5,000 task summary-only report ~137ms

For detailed performance report, see [PERFORMANCE.md](../technical/performance.md).

### Performance Test Coverage

- Deep task hierarchy tests (100, 500 levels)
- Massive task tests (10,000, 50,000 tasks)
- Massive event tests (10,000 events)
- Wide hierarchy tests (1,000 subtasks)
- FTS5 full-text search performance
- Report generation performance (summary-only vs full report)
- Concurrent operation tests
- State transition stress tests

## Special Characters and Security Testing

Intent-Engine is comprehensively tested for special characters and edge cases ensuring system security and robustness.

### Test Coverage

**Security Tests** (37 unit tests + 10 integration tests):
- ✅ SQL injection protection (single quotes, UNION SELECT, comment markers, etc.)
- ✅ Unicode support (Chinese, Japanese, Arabic, mixed languages)
- ✅ Emoji support (including compound emojis and flags)
- ✅ JSON special characters (quotes, backslashes, control characters)
- ✅ Extreme length inputs (10,000+ characters)
- ✅ Edge cases (empty strings, pure whitespace, single character)
- ✅ Shell metacharacters, Markdown, HTML tags
- ✅ URLs, file paths, regex metacharacters

### Running Tests

```bash
# Run special character unit tests
cargo test --test special_chars_tests

# Run CLI special character integration tests
cargo test --test cli_special_chars_tests
```

### Security Guarantees

- **SQL Injection**: Complete protection (using parameterized queries)
- **Command Injection**: Doesn't execute external commands, no risk
- **Internationalization**: Full Unicode and Emoji support
- **Data Integrity**: Preserves user input originality

For detailed information, see [SPECIAL_CHARS.md](../technical/security.md).

## Related Documentation

Intent-Engine provides a series of documents helping you understand and use the system from different perspectives:

### Core Documentation

- **[AI Quick Guide](ai-quick-guide.md)** - AI quick reference ⚡
  - Super concise usage guide
  - Suitable for system prompt
  - Command quick reference and anti-patterns

- **[The Intent-Engine Way](the-intent-engine-way.md)** - Collaboration philosophy and workflow guide 🌟
  - When, how, why to use each command
  - Complete workflow examples
  - Core principles and anti-patterns
  - Recommended for new users to read first

- **[README.md](README.md)** (this document) - Complete technical reference
  - Detailed usage of all commands
  - 60+ real examples
  - Database schema explanation

### Integration Documentation

- **[System Prompt Guide](../integration/claude-code-system-prompt.md)** - Zero-config Claude Code integration (v0.10.0+) 🔧
  - System prompt setup
  - Zero configuration required
  - Best Claude integration experience

- **[Claude Code Skill](../../../.claude-code/intent-engine.skill.md)** - Skill configuration
  - Lightweight alternative integration

- **[Generic CLI Integration](../integration/generic-llm.md)** - Integrate with any AI tool
  - Works with any AI tool that supports CLI commands
  - Quick start examples
  - Common patterns

### Technical Documentation

- **[Task Workflow Analysis](../technical/task-workflow-analysis.md)** - In-depth technical analysis
  - Token optimization strategy details
  - 11 test scenario designs
  - Implementation details and ROI analysis

- **[PERFORMANCE.md](../technical/performance.md)** - Performance testing report
  - Massive data performance metrics
  - Stress test results

- **[SPECIAL_CHARS.md](../technical/security.md)** - Security testing report
  - SQL injection protection verification
  - Unicode and special character support

## License

MIT License

## Contributing

Issues and Pull Requests welcome!
