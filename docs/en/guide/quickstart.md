# Intent-Engine Quick Start Guide

**[中文](../../zh-CN/guide/quickstart.md) | English**

**5 minutes to experience Intent-Engine's core features.**

---

## Prerequisites

- Rust and Cargo ([Installation Guide](https://rustup.rs/))
- Or pre-compiled binary from [releases](https://github.com/wayfind/intent-engine/releases)

---

## Step 1: Installation (1 minute)

```bash
# Method 1: Using Cargo (Recommended)
cargo install intent-engine

# Method 2: Using Homebrew (macOS/Linux)
brew install wayfind/tap/intent-engine

# Method 3: Using npm
npm install -g @origintask/intent-engine

# Verify installation
ie --version
```

---

## Step 2: Create Your First Task (1 minute)

```bash
# Create a task with description (spec)
echo '{"tasks":[{
  "name": "Implement user authentication",
  "status": "doing",
  "spec": "## Goal\nUsers can authenticate via JWT tokens\n\n## Approach\n- Use HS256 algorithm\n- Token validity: 7 days\n- Support refresh tokens"
}]}' | ie plan

# Output:
# ✓ Plan executed successfully
# Created: 1 tasks
# Task ID mapping:
#   Implement user authentication → #1
```

**What happened?**
- Intent-Engine auto-initialized in the current directory
- Created `.intent-engine/project.db` (SQLite database)
- Task saved with full specification
- Task set as current focus (status: doing)

> **Note**: Tasks with `status: doing` require a `spec` (description). This ensures you know the goal before starting work.

---

## Step 3: Check Current Status (30 seconds)

```bash
ie status

# Output shows:
# - Current focused task
# - Task specification
# - Parent/child relationships (if any)
# - Event history
```

**This is the "amnesia recovery" command** - run it at the start of every session.

---

## Step 4: Break Down Into Subtasks (1 minute)

```bash
# Add subtasks to the current task
echo '{"tasks":[
  {"name": "Design JWT token schema", "status": "todo"},
  {"name": "Implement token validation", "status": "todo"},
  {"name": "Add refresh mechanism", "status": "todo"}
]}' | ie plan

# Subtasks are automatically added under the focused parent
```

**What happened?**
- 3 subtasks created under the parent task (#1)
- Auto-parenting: new tasks become children of the focused task
- Use `"parent_id": null` to create independent root tasks

---

## Step 5: Record a Decision (30 seconds)

```bash
# Record why you made a choice
ie log decision "Chose HS256 over RS256 - single app, no need for asymmetric keys"

# Output:
# ✓ Event recorded
#   Type: decision
#   Task: #1
```

**Decision logs are messages to future AI** (including your amnesiac future self).

Other event types: `blocker`, `milestone`, `note`

---

## Step 6: Complete a Subtask (30 seconds)

```bash
# Start working on a subtask
echo '{"tasks":[{"name": "Design JWT token schema", "status": "doing", "spec": "Define token structure and claims"}]}' | ie plan

# ... do the work ...

# Mark it complete
echo '{"tasks":[{"name": "Design JWT token schema", "status": "done"}]}' | ie plan
```

**Key rule**: Parent tasks cannot be marked `done` until all children are complete.

---

## Step 7: Search Your History (30 seconds)

```bash
# Find unfinished tasks
ie search "todo doing"

# Search by content
ie search "JWT authentication"

# Find recent decisions
ie search "decision"
```

---

## Congratulations!

You've learned Intent-Engine's core workflow:

1. **ie status** - Restore context (always first)
2. **ie plan** - Create, update, complete tasks (JSON stdin)
3. **ie log** - Record decisions and events
4. **ie search** - Find tasks and history

---

## Command Summary

| Command | Purpose | Example |
|---------|---------|---------|
| `ie status` | View current context | `ie status` or `ie status 42` |
| `ie plan` | Task operations | `echo '{"tasks":[...]}' \| ie plan` |
| `ie log <type> <msg>` | Record events | `ie log decision "chose X"` |
| `ie search <query>` | Search | `ie search "todo doing"` |

---

## Next Steps

### Advanced Features

1. **Hierarchical Tasks**: Use `children` in JSON for nested structures
2. **Priority**: Add `"priority": "high"` (critical/high/medium/low)
3. **Dashboard**: Run `ie dashboard start` for visual UI at `localhost:11391`

### Documentation

- [CLAUDE.md](../../../CLAUDE.md) - For AI assistants (the "why")
- [Command Reference](command-reference-full.md) - All commands in detail
- [System Prompt Guide](../integration/claude-code-system-prompt.md) - Claude Code integration

---

## FAQ

**Q: What's the difference from todo apps?**

A: Intent-Engine tracks **strategic intent** (What + Why), not just tasks. Each task has specifications, decision history, and hierarchy - it's AI's external long-term memory.

**Q: Where is data stored?**

A: `.intent-engine/project.db` (SQLite) in the directory where you first ran a command.

**Q: Why must `doing` tasks have a spec?**

A: You should know the goal and approach before starting work. This prevents "working on something" without clarity.

---

**Start using Intent-Engine - give your AI the memory it deserves!**
