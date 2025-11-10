# Intent-Engine: AI Agent Guide

**Version**: 0.1.9
**Purpose**: This document helps AI agents understand Intent-Engine's core concepts and interface design

---

## 🔧 Important: Code Formatting

**CRITICAL**: This project enforces code formatting with `cargo fmt`.

### Automatic Setup (No Action Needed)

Git hooks are **automatically installed** on your first `cargo build`. The pre-commit hook will:
- Run `cargo fmt --all` before every commit
- Auto-add formatted files to your commit
- Prevent unformatted code from being committed

### Verification

Check that hooks are installed:
```bash
ls -la .git/hooks/pre-commit  # Should exist and be executable
```

### Manual Override

If you need to bypass the hook (not recommended):
```bash
git commit --no-verify
```

To manually install hooks:
```bash
./scripts/setup-git-hooks.sh
```

To disable automatic installation:
```bash
export SKIP_GIT_HOOKS_SETUP=1
cargo build
```

### How It Works

1. **First build**: `build.rs` detects no hooks → installs them automatically
2. **Every commit**: Pre-commit hook → runs `cargo fmt` → adds changes → continues commit
3. **CI validation**: Runs `cargo fmt --all -- --check` to ensure compliance

---

## 📖 Authoritative Specification

> **CRITICAL**: Before working with Intent-Engine, understand the specification hierarchy:
>
> **Single Source of Truth**: `docs/INTERFACE_SPEC.md`
>
> The INTERFACE_SPEC.md document is the **authoritative blueprint** for all Intent-Engine interfaces:
> - ✅ **CLI Interface**: Command signatures, parameters, atomic behaviors
> - ✅ **MCP Interface**: Tool definitions, JSON-RPC protocols
> - ✅ **Rust API**: Public types, function signatures
> - ✅ **Data Models**: Exact field names, types, lifecycle semantics
> - ✅ **Guarantees**: SemVer stability, breaking change policies
>
> **This AGENT.md is a derived guide** that explains concepts and patterns.
> **In case of any conflict**, defer to INTERFACE_SPEC.md.
>
> When implementing features, writing tests, or building integrations:
> 1. Read INTERFACE_SPEC.md first to understand the contract
> 2. Use this guide to understand the philosophy and patterns
> 3. Validate your work against the spec's requirements

---

## 🎯 Core Philosophy

Intent-Engine is built on the **Focus-Driven Architecture** principle:

1. **Single Focus Point**: `current_task_id` is the central concept
2. **Atomic Operations**: Commands combine multiple steps (start, switch, done)
3. **Context-Aware Intelligence**: Recommendations based on current focus
4. **Hierarchical Task Trees**: Parent-child relationships with enforced completion order
5. **Persistent Memory**: Cross-session state in SQLite database

---

## 📊 Data Model

### Task
```
Task {
  id: i64,
  name: String,
  spec: Option<String>,              // Markdown specification
  status: String,                     // "todo", "doing", "done"
  priority: Option<i32>,              // 1 = highest priority
  complexity: Option<i32>,            // Optional complexity rating
  parent_id: Option<i64>,             // Parent task for subtasks
  first_todo_at: Option<Timestamp>,   // Lifecycle tracking
  first_doing_at: Option<Timestamp>,
  first_done_at: Option<Timestamp>
}
```

### Event
```
Event {
  id: i64,
  task_id: i64,
  timestamp: Timestamp,
  log_type: String,                   // "decision", "blocker", "milestone", "note"
  discussion_data: String             // Markdown content
}
```

### Workspace State
```
current_task_id: Option<i64>          // The focused task (or null)
```

---

## 🔑 Key Design Principles

### 1. Focus-Driven Operations

Most operations work on `current_task_id`:

```bash
# NO explicit ID needed - uses current_task_id
task done
event add --type decision --data-stdin
task spawn-subtask --name "Configure JWT"
```

### 2. Atomic Operations

Commands perform multiple steps atomically:

**`task start <ID>`**:
1. Set task status to `doing`
2. Set as `current_task_id`
3. Return full context with events

**`task switch <ID>`**:
1. Previous task: `doing` → `todo`
2. New task: `todo` → `doing`
3. Update `current_task_id`

**`task done`** (NO parameters):
1. Verify all subtasks are `done`
2. Current task → `done`
3. Clear `current_task_id`

### 3. Context-Aware Intelligence

**`task pick-next`** uses depth-first strategy:

**Priority 1**: Subtasks of current focused task
```sql
SELECT * FROM tasks
WHERE parent_id = current_task_id AND status = 'todo'
ORDER BY priority ASC NULLS LAST
LIMIT 1
```

**Priority 2**: Top-level tasks (if no focused subtasks)
```sql
SELECT * FROM tasks
WHERE parent_id IS NULL AND status = 'todo'
ORDER BY priority ASC NULLS LAST
LIMIT 1
```

---

## 🛠️ Essential Commands

### Task Management

| Command | Purpose | Parameters | Focus-Driven? |
|---------|---------|------------|---------------|
| `task add` | Create task | `--name`, `--parent`, `--spec-stdin` | ❌ |
| `task start <ID>` | Start task | `<TASK_ID>`, `--with-events` | Sets focus |
| `task done` | Complete current | None | ✅ Yes |
| `task switch <ID>` | Switch focus | `<TASK_ID>` | Changes focus |
| `task spawn-subtask` | Create + switch | `--name`, `--spec-stdin` | ✅ Yes |
| `task pick-next` | Recommend next | `--format` | Context-aware |
| `task find` | Filter by metadata | `--status`, `--parent` | ❌ |
| `task search` | Full-text search | `<QUERY>`, `--snippet` | ❌ |

### Event Recording

| Command | Purpose | Parameters | Focus-Driven? |
|---------|---------|------------|---------------|
| `event add` | Record event | `--type`, `--task-id?`, `--data-stdin` | ✅ Yes (if no --task-id) |
| `event list <ID>` | List events | `<TASK_ID>` | ❌ |

### Workspace

| Command | Purpose | Parameters |
|---------|---------|------------|
| `current` | Get focused task | None |
| `current --set <ID>` | Set focus | `<TASK_ID>` |
| `report` | Generate report | `--since`, `--status`, `--summary-only` |

---

## 🎨 Output Formats

All commands output **JSON by default**.

### Standard Task Output
```json
{
  "id": 42,
  "name": "Implement authentication",
  "status": "doing",
  "spec": "Use JWT with 7-day expiry...",
  "parent_id": null,
  "priority": 1,
  "first_doing_at": "2024-11-09T10:05:00Z"
}
```

### Special Output Structures

**TaskWithEvents** (from `task start --with-events`):
```json
{
  "task": { ... },
  "events_summary": {
    "total_count": 3,
    "recent_events": [...]
  }
}
```

**SearchResult** (from `task search --snippet`):
```json
{
  "task_id": 42,
  "name": "Implement **authentication**",
  "spec_snippet": "Use **JWT** with refresh tokens...",
  "rank": 0.95
}
```

**PickNextResult** (from `task pick-next --format json`):
```json
{
  "recommended_task": { ... },
  "reason": "subtask_of_current",
  "context": {
    "current_task_id": 42,
    "strategy": "depth_first"
  }
}
```

---

## 🔄 Typical Workflow

### Starting Work
```bash
# See what's available
task pick-next

# Start the recommended task
task start 42 --with-events

# Review context and spec
# Task 42 is now current_task_id
```

### During Work
```bash
# Discover a subtask is needed
task spawn-subtask --name "Configure JWT secret"
# Creates subtask and automatically switches to it

# Record decision
echo "Using HS256 algorithm" | event add --type decision --data-stdin

# Complete subtask
task done
# Automatically clears current_task_id
```

### Continuing Parent Task
```bash
# Switch back to parent
task switch 42

# Or let pick-next suggest next subtask
task pick-next  # Will recommend other subtasks of 42

# Complete parent when all children done
task done
```

---

## ⚠️ Common Pitfalls

### ❌ DON'T: Pass ID to `task done`
```bash
task done 42  # WRONG - command takes no parameters
```

### ✅ DO: Use focus-driven approach
```bash
task start 42   # Set as current
task done       # Complete current
```

### ❌ DON'T: Use `find` for text search
```bash
task find --name-pattern "auth"  # WRONG - find is for metadata only
```

### ✅ DO: Use `search` for text
```bash
task search "auth AND jwt"  # Correct
```

### ❌ DON'T: Forget to set current task before `done`
```bash
task done  # ERROR if no current task
```

### ✅ DO: Always start or switch first
```bash
task start 42
task done  # Works
```

---

## 🧪 Testing Guidelines

### When writing tests, ensure:

1. **Focus State**: Set up `current_task_id` before testing focus-driven commands
2. **Atomic Verification**: Check all steps of atomic operations complete
3. **Hierarchy Rules**: Parent tasks can't be done until children are done
4. **Event Flexibility**: Test both `--task-id` and current-task modes
5. **Search vs Find**: Separate tests for metadata filtering vs text search

### Test Data Setup
```bash
# Create task hierarchy
task add --name "Parent"               # ID 1
task start 1
task spawn-subtask --name "Child 1"    # ID 2, now current
task done                               # Child 1 done
task spawn-subtask --name "Child 2"    # ID 3, now current
task done                               # Child 2 done
task switch 1                           # Back to parent
task done                               # Parent done (all children done)
```

---

## 📚 Key Documents

- **Authoritative Spec**: `docs/INTERFACE_SPEC.md`
- **MCP Integration**: `docs/*/integration/mcp-server.md`
- **Tool Sync**: `docs/*/technical/mcp-tools-sync.md`
- **Test Files**: `tests/interface_spec_test.rs`, `tests/mcp_tools_sync_test.rs`

---

## 🎓 Design Rationale

### Why Focus-Driven?

1. **Cognitive Load**: Humans naturally focus on one task at a time
2. **Context Retention**: No need to constantly specify "which task"
3. **Atomic Safety**: Focus prevents accidental operations on wrong tasks
4. **AI Efficiency**: Reduces token usage (no repetitive ID passing)

### Why Atomic Operations?

1. **Consistency**: Multiple steps succeed or fail together
2. **Simplicity**: One command does "the right thing"
3. **Safety**: No partial state changes
4. **Ergonomics**: Fewer commands to remember

### Why Hierarchical Tasks?

1. **Problem Decomposition**: Natural way to break down complex work
2. **Forced Completion**: Parent can't complete until children done
3. **Context Awareness**: `pick-next` recommends subtasks first
4. **Progress Tracking**: Tree structure shows work breakdown

---

## 🔮 Future Considerations (Post-1.0)

- **Multiple Workspaces**: Separate focus contexts for different projects
- **Task Templates**: Reusable task structures with subtasks
- **Time Tracking**: Automatic time spent per task/status
- **Dependency Graphs**: Tasks depend on other tasks (not just parent-child)
- **Collaborative State**: Multiple users with separate focus points

---

**Last Updated**: 2024-11-09
**Spec Version**: 0.1.9
**Status**: Experimental (Pre-1.0)
