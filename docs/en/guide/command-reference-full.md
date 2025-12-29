# Intent-Engine Command Reference

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)

Intent-Engine is a minimalist, project-specific command-line tool designed to record, track, and review human strategic intent. It's a core powerhouse in the AI collaborator's toolbox, helping answer two key questions: "Where are we going? (What)" and "Why are we going there? (Why)".

> **New User?** We recommend reading [The Intent-Engine Way](the-intent-engine-way.md) first to understand Intent-Engine's design philosophy. This document is a technical reference.

---

## Installation

### Method 1: Cargo Install (Recommended)

```bash
cargo install intent-engine
ie --version
```

### Method 2: Homebrew (macOS/Linux)

```bash
brew install wayfind/tap/intent-engine
```

### Method 3: npm

```bash
npm install -g @origintask/intent-engine
```

### Method 4: Download Pre-compiled Binaries

Download from [GitHub Releases](https://github.com/wayfind/intent-engine/releases)

### Method 5: Build from Source

```bash
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine
cargo install --path .
```

---

## Core Concepts

### Data Model

```
Task
├── id: Integer (auto-increment)
├── name: String (required, unique identifier)
├── spec: String (description, required for status:doing)
├── status: String { "todo", "doing", "done" }
├── priority: String { "critical", "high", "medium", "low" }
├── parent_id: Integer (optional, nullable)
├── first_todo_at: Timestamp
├── first_doing_at: Timestamp
└── first_done_at: Timestamp

Event
├── id: Integer (auto-increment)
├── task_id: Integer (foreign key)
├── log_type: String { "decision", "blocker", "milestone", "note" }
├── message: String
└── timestamp: Timestamp
```

### Task Status Lifecycle

```
todo → doing → done
```

- **todo**: Task identified, not yet started
- **doing**: Actively working (requires spec)
- **done**: Completed (requires all children done)

### Focus System

Intent-Engine maintains a "current focus" - the task you're actively working on. New tasks without explicit `parent_id` become children of the focused task (auto-parenting).

---

## Commands Overview

| Command | Purpose |
|---------|---------|
| `ie status` | View current task context |
| `ie plan` | Create/update/complete tasks |
| `ie log` | Record events |
| `ie search` | Search tasks and events |
| `ie init` | Initialize project |
| `ie dashboard` | Dashboard management |
| `ie doctor` | System health check |

---

## ie status

**Purpose**: Restore context, view current task and relationships.

### Usage

```bash
ie status [TASK_ID] [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `TASK_ID` | Optional task ID to view (default: current focus) |

### Options

| Option | Description |
|--------|-------------|
| `--format <FORMAT>` | Output format: `human` (default), `json` |

### Examples

```bash
# View current focused task
ie status

# View specific task
ie status 42

# Get JSON output
ie status --format json
```

### Output

Shows:
- Focused task with full spec
- Ancestor chain (parent tasks)
- Sibling tasks
- Descendant tasks (children)
- Event history summary

---

## ie plan

**Purpose**: Create, update, and complete tasks. All task operations go through this command.

### Usage

```bash
echo '{"tasks":[...]}' | ie plan
```

### JSON Format

```json
{
  "tasks": [
    {
      "name": "Task name (required)",
      "status": "todo|doing|done",
      "spec": "Description (required for doing)",
      "priority": "critical|high|medium|low",
      "parent_id": null,
      "children": [...]
    }
  ]
}
```

### Field Reference

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier, used for matching existing tasks |
| `status` | No | `todo` (default), `doing`, `done` |
| `spec` | For `doing` | Goal + approach description |
| `priority` | No | `critical`, `high`, `medium`, `low` |
| `parent_id` | No | `null` = root task, omit = auto-parent to focus |
| `children` | No | Nested subtask array |

### Key Behaviors

1. **Idempotent**: Same name = update existing task, not create duplicate
2. **Auto-parenting**: New tasks become children of focused task (unless `parent_id: null`)
3. **Spec required**: `status: doing` requires a `spec`
4. **Children first**: `status: done` requires all children complete

### Examples

```bash
# Create a task
echo '{"tasks":[{
  "name": "Implement authentication",
  "status": "doing",
  "spec": "## Goal\nUsers can log in\n\n## Approach\nUse JWT"
}]}' | ie plan

# Create with children
echo '{"tasks":[{
  "name": "Parent task",
  "status": "doing",
  "spec": "Main feature",
  "children": [
    {"name": "Subtask 1", "status": "todo"},
    {"name": "Subtask 2", "status": "todo"}
  ]
}]}' | ie plan

# Update task status
echo '{"tasks":[{"name": "Subtask 1", "status": "done"}]}' | ie plan

# Create independent root task
echo '{"tasks":[{
  "name": "Unrelated task",
  "status": "todo",
  "parent_id": null
}]}' | ie plan

# Update priority
echo '{"tasks":[{
  "name": "Existing task",
  "priority": "critical"
}]}' | ie plan
```

### @file Syntax

Include file content in spec:

```bash
# Write description to file
cat > /tmp/desc.md << 'EOF'
## Goal
Implement user authentication

## Approach
- Use JWT tokens
- Add refresh mechanism
EOF

# Reference in plan
echo '{"tasks":[{
  "name": "Auth feature",
  "status": "doing",
  "spec": "@file(/tmp/desc.md)"
}]}' | ie plan
```

---

## ie log

**Purpose**: Record events (decisions, blockers, milestones, notes).

### Usage

```bash
ie log <TYPE> <MESSAGE> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `TYPE` | Event type: `decision`, `blocker`, `milestone`, `note` |
| `MESSAGE` | Event message |

### Options

| Option | Description |
|--------|-------------|
| `--task <ID>` | Target specific task (default: current focus) |

### Event Types

| Type | When to Use |
|------|-------------|
| `decision` | Made a key technical choice |
| `blocker` | Stuck on something, need help |
| `milestone` | Completed significant phase |
| `note` | General observation |

### Examples

```bash
# Record decision
ie log decision "Chose JWT over sessions for stateless API"

# Record blocker
ie log blocker "Waiting for API credentials from admin"

# Record milestone
ie log milestone "Core auth logic complete, all tests passing"

# Record note
ie log note "Consider adding rate limiting later"

# Target specific task
ie log decision "Approach X chosen" --task 42
```

---

## ie search

**Purpose**: Search tasks and events using full-text search.

### Usage

```bash
ie search <QUERY> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `QUERY` | Search query |

### Options

| Option | Description |
|--------|-------------|
| `--since <DURATION>` | Filter by time (e.g., `7d`, `1w`, `2025-01-01`) |
| `--until <DURATION>` | Filter by time |

### Query Types

1. **Status filter**: Query contains only status words
   ```bash
   ie search "todo doing"      # Unfinished tasks
   ie search "done"            # Completed tasks
   ```

2. **Full-text search**: Query contains non-status words
   ```bash
   ie search "JWT authentication"
   ie search "API AND client"  # Boolean operators
   ```

### Examples

```bash
# Find unfinished tasks
ie search "todo doing"

# Search by content
ie search "authentication"

# Find decisions
ie search "decision"

# Time-filtered search
ie search "done" --since 7d
ie search "todo" --since 2025-01-01
```

---

## ie init

**Purpose**: Initialize a new Intent-Engine project.

### Usage

```bash
ie init [PATH]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `PATH` | Directory to initialize (default: current directory) |

### Behavior

- Creates `.intent-engine/` directory
- Creates `project.db` SQLite database
- Safe to run multiple times (no-op if already initialized)

### Examples

```bash
# Initialize current directory
ie init

# Initialize specific directory
ie init /path/to/project
```

---

## ie dashboard

**Purpose**: Manage the web dashboard for visual task management.

### Usage

```bash
ie dashboard <COMMAND>
```

### Commands

| Command | Description |
|---------|-------------|
| `start` | Start dashboard server |
| `stop` | Stop dashboard server |
| `status` | Check dashboard status |
| `open` | Open dashboard in browser |

### Examples

```bash
# Start dashboard
ie dashboard start

# Open in browser (default: localhost:11391)
ie dashboard open

# Check status
ie dashboard status

# Stop dashboard
ie dashboard stop
```

---

## ie doctor

**Purpose**: Check system health and dependencies.

### Usage

```bash
ie doctor
```

### Checks

- Database connectivity
- Schema version
- Configuration validity
- Dashboard status

---

## Common Patterns

### Session Start

```bash
# Always first
ie status
```

### Create Task with Subtasks

```bash
echo '{"tasks":[{
  "name": "Main feature",
  "status": "doing",
  "spec": "## Goal\n...\n\n## Approach\n...",
  "children": [
    {"name": "Step 1", "status": "todo"},
    {"name": "Step 2", "status": "todo"},
    {"name": "Step 3", "status": "todo"}
  ]
}]}' | ie plan
```

### Work on Subtask

```bash
# Start subtask
echo '{"tasks":[{
  "name": "Step 1",
  "status": "doing",
  "spec": "Specific goal for this step"
}]}' | ie plan

# Record decisions
ie log decision "Chose approach X because..."

# Complete
echo '{"tasks":[{"name": "Step 1", "status": "done"}]}' | ie plan
```

### Complete Parent

```bash
# Only works after all children done
echo '{"tasks":[{"name": "Main feature", "status": "done"}]}' | ie plan
```

### Create Independent Task

```bash
echo '{"tasks":[{
  "name": "Unrelated bug fix",
  "status": "todo",
  "parent_id": null
}]}' | ie plan
```

### Find Unfinished Work

```bash
ie search "todo doing"
```

---

## Key Rules Summary

| Rule | Description |
|------|-------------|
| **status first** | Run `ie status` at session start |
| **doing needs spec** | `status: doing` requires description |
| **children first** | Parent can't be done until all children done |
| **same name = update** | `ie plan` is idempotent |
| **auto-parenting** | New tasks become children of focus |
| **parent_id: null** | Creates independent root task |

---

## Error Handling

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| "spec required for doing" | No description | Add `spec` field |
| "has incomplete children" | Children not done | Complete all children first |
| "Invalid JSON" | Malformed input | Check JSON syntax |
| "Task not found" | Wrong name/ID | Verify task exists |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `IE_DATABASE_PATH` | Custom database path |
| `IE_SESSION_ID` | Session identifier for multi-session support |

---

## Data Storage

- **Location**: `.intent-engine/project.db`
- **Format**: SQLite database
- **Scope**: Per-project (directory-based)

---

## Next Steps

- [Quick Start](quickstart.md) - 5-minute tutorial
- [The Intent-Engine Way](the-intent-engine-way.md) - Design philosophy
- [AI Quick Guide](ai-quick-guide.md) - AI-focused reference
- [CLAUDE.md](../../../CLAUDE.md) - Core concepts

---

## License

MIT OR Apache-2.0, at your option.
