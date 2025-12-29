# Intent-Engine User Guide

**Version**: 0.10.4
**Last Updated**: 2024-12-27

---

## Table of Contents

1. [What is Intent-Engine?](#what-is-intent-engine)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [Core Concepts](#core-concepts)
5. [Command Reference](#command-reference)
6. [Task Lifecycle](#task-lifecycle)
7. [Validation Rules](#validation-rules)
8. [Best Practices](#best-practices)
9. [Integration Guide](#integration-guide)
10. [Troubleshooting](#troubleshooting)
11. [FAQ](#faq)

---

## What is Intent-Engine?

Intent-Engine is an **AI-native task management system** designed for persistent, hierarchical task tracking across sessions. Unlike ephemeral todo lists, Intent-Engine:

- **Persists across sessions** - Never lose your work context
- **Supports hierarchical tasks** - Break down complex work into subtrees
- **Records decisions** - Track *why* you made choices, not just *what* you did
- **Integrates with AI assistants** - Built for Claude Code, Claude Desktop, and other AI tools

### When to Use Intent-Engine

| Scenario | Use Intent-Engine? |
|----------|-------------------|
| Multi-session project work | ✅ Yes |
| Complex task with 3+ subtasks | ✅ Yes |
| Need to record decisions/rationale | ✅ Yes |
| Simple one-off task | ❌ Use TodoWrite |
| Quick fix, no context needed | ❌ Use TodoWrite |

**Rule of thumb**: If losing the task context would be a shame, use Intent-Engine.

---

## Installation

### Option 1: Cargo (Recommended)
```bash
cargo install intent-engine
```

### Option 2: npm
```bash
npm install -g @m3task/intent-engine
```

### Option 3: Homebrew (macOS)
```bash
brew install wayfind/tap/intent-engine
```

### Verify Installation
```bash
ie --version
ie doctor  # Check system health
```

---

## Quick Start

### 1. Initialize a Project
```bash
cd /path/to/your/project
ie init
```

### 2. Create Your First Task
```bash
echo '{"tasks":[{
  "name":"My First Task",
  "status":"doing",
  "spec":"Learning how to use Intent-Engine"
}]}' | ie plan
```

### 3. Check Status
```bash
ie status
```

### 4. Complete the Task
```bash
echo '{"tasks":[{"name":"My First Task","status":"done"}]}' | ie plan
```

---

## Core Concepts

### Focus-Driven Workflow

Intent-Engine uses a **focus model** where one task is "current" at any time:

```
┌─────────────────────────────────┐
│  Current Focus: Task #42        │
│  "Implement authentication"     │
└─────────────────────────────────┘
         │
    ┌────┴────┐
    ▼         ▼
 Task #43   Task #44
 "JWT"      "OAuth"
```

- **Focus** (`current_task_id`): The task you're actively working on
- **Subtasks**: Child tasks automatically inherit focus context
- **Auto-parenting**: New tasks become children of the focused task

### Task States

| State | Symbol | Description |
|-------|--------|-------------|
| `todo` | 📋 | Planned, not started |
| `doing` | 🔨 | In progress (requires description) |
| `done` | ✅ | Completed (requires all children done) |

### Hierarchical Structure

Tasks can have parent-child relationships:

```
Project Refactor (doing)
├── Design Phase (done)
│   ├── Define API schema (done)
│   └── Write specifications (done)
├── Implementation (doing)    ← Current focus
│   ├── Backend API (doing)
│   └── Frontend UI (todo)
└── Testing (todo)
```

---

## Command Reference

### ie status

View current task context and focus.

```bash
ie status           # Show focused task
ie status 42        # Show specific task #42
ie status --format json  # JSON output
```

**Output includes**:
- Current focused task details
- Ancestor chain (parent hierarchy)
- Sibling tasks (same level)
- Descendant tasks (children and below)

### ie plan

The universal command for all task operations. Accepts JSON via stdin.

#### Create Tasks
```bash
echo '{"tasks":[{"name":"Task Name","status":"todo"}]}' | ie plan
```

#### Start a Task (requires spec)
```bash
echo '{"tasks":[{
  "name":"Task Name",
  "status":"doing",
  "spec":"## Goal\nWhat you want to achieve\n\n## Approach\nHow you plan to do it"
}]}' | ie plan
```

#### Complete a Task
```bash
echo '{"tasks":[{"name":"Task Name","status":"done"}]}' | ie plan
```

#### Create Hierarchy
```bash
echo '{"tasks":[{
  "name":"Parent Task",
  "status":"doing",
  "spec":"Parent description",
  "children":[
    {"name":"Child 1","status":"todo"},
    {"name":"Child 2","status":"todo"}
  ]
}]}' | ie plan
```

#### Create Root Task (Ignore Current Focus)
```bash
echo '{"tasks":[{"name":"Independent Task","parent_id":null}]}' | ie plan
```

#### Assign to Specific Parent
```bash
echo '{"tasks":[{"name":"Child Task","parent_id":42}]}' | ie plan
```

#### Include Description from File
```bash
# Create description file
cat > /tmp/spec.md << 'EOF'
## Goal
Implement user authentication

## Approach
- Use JWT tokens
- HS256 algorithm
- 24h expiry
EOF

# Use @file syntax
echo '{"tasks":[{
  "name":"Auth Task",
  "status":"doing",
  "spec":"@file(/tmp/spec.md)"
}]}' | ie plan
# File is auto-deleted after successful execution

# Keep file after execution
echo '{"tasks":[{"name":"Task","spec":"@file(/tmp/spec.md, keep)"}]}' | ie plan
```

### ie log

Record events associated with tasks.

```bash
ie log decision "Chose PostgreSQL for better JSON support"
ie log blocker "Waiting for API credentials from vendor"
ie log milestone "Authentication module complete"
ie log note "Consider adding rate limiting later"

# Log to specific task
ie log decision "message" --task 42
```

**Event Types**:
| Type | Use For |
|------|---------|
| `decision` | Architecture and design choices |
| `blocker` | Impediments and blockers |
| `milestone` | Key achievements |
| `note` | General observations |

### ie search

Search across tasks and events.

```bash
# Status filter (find unfinished tasks)
ie search "todo doing"

# Full-text search
ie search "authentication JWT"

# Boolean operators
ie search "API AND authentication"
ie search "frontend OR backend"

# Output formats
ie search "query" --format json
```

### ie init

Initialize a new Intent-Engine project.

```bash
ie init                    # Current directory
ie init --at /path/to/dir  # Specific directory
```

### ie dashboard

Manage the web dashboard.

```bash
ie dashboard start         # Start dashboard server
ie dashboard stop          # Stop dashboard server
ie dashboard status        # Check dashboard status
```

### ie doctor

Check system health and dependencies.

```bash
ie doctor
```

---

## Task Lifecycle

### Complete Lifecycle Example

```bash
# 1. Create a new feature task
echo '{"tasks":[{
  "name":"User Authentication",
  "status":"doing",
  "spec":"## Goal\nImplement secure user authentication\n\n## Approach\n- JWT tokens\n- Refresh token rotation"
}]}' | ie plan

# 2. Break down into subtasks
echo '{"tasks":[{
  "name":"User Authentication",
  "children":[
    {"name":"Design token schema"},
    {"name":"Implement login endpoint"},
    {"name":"Add token validation middleware"},
    {"name":"Write tests"}
  ]
}]}' | ie plan

# 3. Record decisions as you work
ie log decision "Chose HS256 for JWT signing - simpler key management"
ie log decision "Set access token expiry to 15 minutes for security"

# 4. Complete subtasks one by one
echo '{"tasks":[{"name":"Design token schema","status":"done"}]}' | ie plan
echo '{"tasks":[{"name":"Implement login endpoint","status":"done"}]}' | ie plan

# 5. Note a blocker
ie log blocker "Redis connection issues - investigating"

# 6. Continue and complete remaining subtasks
echo '{"tasks":[{"name":"Add token validation middleware","status":"done"}]}' | ie plan
echo '{"tasks":[{"name":"Write tests","status":"done"}]}' | ie plan

# 7. Complete parent (only works when all children are done)
echo '{"tasks":[{"name":"User Authentication","status":"done"}]}' | ie plan

# 8. Record milestone
ie log milestone "User Authentication feature complete"
```

---

## Validation Rules

Intent-Engine enforces these rules to maintain data integrity:

### Rule 1: Description Required for Starting

When setting a task to `doing`, you must provide a description (`spec`):

```bash
# ❌ This will fail
echo '{"tasks":[{"name":"Task","status":"doing"}]}' | ie plan
# Error: spec (description) is required when starting a task

# ✅ This works
echo '{"tasks":[{"name":"Task","status":"doing","spec":"Goal and approach"}]}' | ie plan
```

**Rationale**: Before starting work, you should know what you're trying to achieve.

### Rule 2: Children Must Complete Before Parent

A parent task cannot be marked `done` if any child is not `done`:

```bash
# Setup: Parent with incomplete child
echo '{"tasks":[{
  "name":"Parent",
  "status":"doing",
  "spec":"Parent task",
  "children":[{"name":"Child","status":"todo"}]
}]}' | ie plan

# ❌ This will fail
echo '{"tasks":[{"name":"Parent","status":"done"}]}' | ie plan
# Error: Cannot complete task 'Parent': has incomplete subtasks

# ✅ Complete child first, then parent
echo '{"tasks":[{"name":"Child","status":"done"}]}' | ie plan
echo '{"tasks":[{"name":"Parent","status":"done"}]}' | ie plan
```

**Rationale**: Hierarchical consistency - a task isn't done until all its parts are done.

### Rule 3: Idempotent Operations

The same `ie plan` command can be run multiple times safely:

```bash
# Running this twice creates only ONE task
echo '{"tasks":[{"name":"My Task"}]}' | ie plan
echo '{"tasks":[{"name":"My Task"}]}' | ie plan  # Updates, doesn't duplicate
```

---

## Best Practices

### 1. Write Meaningful Descriptions

```bash
# ❌ Too vague
"spec":"Implement feature"

# ✅ Clear goal and approach
"spec":"## Goal\nAdd rate limiting to API endpoints\n\n## Approach\n- Use sliding window algorithm\n- Store counts in Redis\n- Return 429 when exceeded"
```

### 2. Use Hierarchical Decomposition

```bash
# ❌ Flat list of 10 tasks
Task 1, Task 2, Task 3, ... Task 10

# ✅ Logical hierarchy
Parent Task
├── Subtask Group A
│   ├── A.1
│   └── A.2
└── Subtask Group B
    ├── B.1
    └── B.2
```

### 3. Record Decisions Immediately

```bash
# When you make a choice, record it right away
ie log decision "Chose PostgreSQL over MongoDB - need ACID transactions"
```

### 4. Use parent_id for Independent Tasks

```bash
# When creating a task unrelated to current focus
echo '{"tasks":[{"name":"Bug Fix #123","parent_id":null}]}' | ie plan
```

### 5. Keep Status Updated

```bash
# Start of work
echo '{"tasks":[{"name":"Task","status":"doing","spec":"..."}]}' | ie plan

# During work
ie log note "Progress: 50% complete"

# End of work
echo '{"tasks":[{"name":"Task","status":"done"}]}' | ie plan
```

---

## Integration Guide

### Claude Code Integration

Add to your project's `CLAUDE.md`:

```markdown
# Task Management

Use `ie` for task tracking:
- Session start: `ie status`
- Create/update: `echo '{...}' | ie plan`
- Decisions: `ie log decision "..."`
```

### Session Start Hook

Configure Claude's session hook to auto-run `ie status`:

```json
{
  "hooks": {
    "SessionStart": [{
      "hooks": [{
        "type": "command",
        "command": "ie status"
      }]
    }]
  }
}
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `IE_SESSION_ID` | Session identifier for multi-session support |
| `IE_DATABASE_PATH` | Custom database file path |
| `IE_LOG_LEVEL` | Logging verbosity (error, warn, info, debug) |

---

## Troubleshooting

### "spec required when starting a task"

**Cause**: Trying to set `status: doing` without a `spec`.

**Solution**: Add a description:
```bash
echo '{"tasks":[{"name":"Task","status":"doing","spec":"Description here"}]}' | ie plan
```

### "Cannot complete task: has incomplete subtasks"

**Cause**: Trying to complete a parent while children are still `todo` or `doing`.

**Solution**: Complete all children first:
```bash
ie status <parent_id>  # Check which children are incomplete
# Complete each child
echo '{"tasks":[{"name":"Child","status":"done"}]}' | ie plan
```

### "Task not found"

**Cause**: Task name doesn't match exactly.

**Solution**: Check exact name with `ie search` or `ie status`.

### Database Locked

**Cause**: Multiple processes accessing the database.

**Solution**:
```bash
ie doctor  # Check for issues
# If needed, stop dashboard and retry
ie dashboard stop
```

---

## FAQ

### Q: Can I use ie without AI assistants?

Yes! Intent-Engine is a standalone CLI tool. While designed for AI integration, it works perfectly for human-driven task management.

### Q: How do I migrate from another task system?

Export your tasks to JSON and use `ie plan` to import:
```bash
cat tasks.json | ie plan
```

### Q: Where is the data stored?

By default: `.intent-engine/intent.db` in your project root.

### Q: Can multiple team members use the same database?

The database is designed for single-user, local use. For teams, consider separate databases or the web dashboard.

### Q: How do I backup my data?

Copy the `.intent-engine/` directory.

### Q: Can I undo a completed task?

Yes, update the status back:
```bash
echo '{"tasks":[{"name":"Task","status":"doing"}]}' | ie plan
```

---

## Further Reading

- [Plan Command Details](./plan.md)
- [Interface Specification](../spec-03-interface-current.md)
- [Dashboard User Guide](../dashboard-user-guide.md)
- [System Prompt for AI](../system_prompt.md)

---

*End of User Guide*
