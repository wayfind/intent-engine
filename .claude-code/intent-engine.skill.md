# Intent-Engine Skill

Strategic intent and task workflow management for human-AI collaboration.

## Quick Start

```bash
# Create a task to capture intent
echo "Implement OAuth2 login with Google and GitHub support" | \
  ie task add --name "OAuth2 Login" --spec-stdin

# Start working on it
ie task start 1 --with-events

# Record a decision
echo "Using Passport.js for OAuth strategy implementation" | \
  ie event add --task-id 1 --type decision --data-stdin

# Complete the task (task 1 is currently focused)
ie task done
```

## Core Workflow

### 1. Capture Strategic Intent

When a requirement is complex enough (multi-step, needs context, long-term):

```bash
ie task add --name "Task Name" [--parent PARENT_ID]
# With specification:
echo "Detailed spec in markdown..." | \
  ie task add --name "Task Name" --spec-stdin
```

### 2. Activate & Get Context (ATOMIC)

Always use this to start work:

```bash
ie task start <ID> --with-events
```

This single call:
- Updates status to 'doing'
- Sets as current task
- Returns full spec + event history

### 3. Smart Task Selection (NEW)

When you have multiple tasks and need to optimize order:

```bash
# First, evaluate tasks
ie task update 1 --complexity 7 --priority 10
ie task update 2 --complexity 3 --priority 8

# Then intelligently pick next batch
ie task pick-next --max-count 3 --capacity 5
```

Selects by: priority DESC, complexity ASC (do important+simple first)

### 4. Handle Sub-problems (ATOMIC, NEW)

When you discover a blocking sub-problem:

```bash
ie task spawn-subtask --name "Subtask Name"
# With spec:
echo "Subtask details..." | \
  ie task spawn-subtask --name "Subtask Name" --spec-stdin
```

This single call:
- Creates subtask under current task
- Sets subtask to 'doing'
- Switches current task to subtask

### 5. Switch Between Tasks (ATOMIC, NEW)

When juggling multiple tasks:

```bash
ie task switch <ID>
```

This single call:
- Updates to 'doing' if needed
- Sets as current task
- Returns context with events

### 6. Record Key Events (AI's External Memory)

Log every critical decision, blocker, or milestone:

```bash
# Decision
echo "Chose library A over B because..." | \
  ie event add --task-id <ID> --type decision --data-stdin

# Blocker
echo "Need API key from team lead" | \
  ie event add --task-id <ID> --type blocker --data-stdin

# Milestone
echo "Database migration complete" | \
  ie event add --task-id <ID> --type milestone --data-stdin
```

Event types: `decision`, `blocker`, `milestone`, `discussion`, `note`

### 7. Complete Task

Only when all objectives achieved and all subtasks done:

```bash
ie task done
```

**Important**: This command operates on the current focused task only. It does not accept an ID parameter.
- If you need to complete a non-current task, first switch to it: `ie task switch <ID>` or `ie current --set <ID>`
- System enforces: parent can't complete until all children are done.

### 8. Generate Reports (Token-Efficient)

Always use `--summary-only` unless you need full details:

```bash
# Weekly summary (recommended)
ie report --since 7d --summary-only

# Full details for specific status
ie report --status done --since 7d

# Search with FTS5
ie report --filter-name "auth" --summary-only
```

## Common Patterns

### Pattern 1: Discover Multiple Issues

```bash
# Create todos
ie task add --name "Fix bug A"
ie task add --name "Fix bug B"
ie task add --name "Fix bug C"

# Evaluate
ie task update 1 --complexity 3 --priority 10
ie task update 2 --complexity 7 --priority 8
ie task update 3 --complexity 2 --priority 5

# Auto-select optimal order
ie task pick-next --max-count 3
```

### Pattern 2: Recursive Problem Decomposition

```bash
# Start parent task
ie task start 1 --with-events

# Discover sub-problem
ie task spawn-subtask --name "Sub-problem A"

# Discover nested sub-problem
ie task spawn-subtask --name "Sub-sub-problem"

# Complete from deepest to shallowest (each spawn-subtask auto-focuses the subtask)
ie task done  # Completes current (sub-sub-problem)
ie task switch 2  # Switch to parent
ie task done  # Completes current (sub-problem A)
ie task switch 1  # Switch to root
ie task done  # Completes current (parent task)
```

### Pattern 3: Recover Context After Interruption

```bash
# Get current task
ie current

# Get full context
ie task get <ID> --with-events

# Review recent events
ie event list --task-id <ID> --limit 10
```

## Why Use Atomic Commands?

| Traditional | Token Cost | Atomic | Token Cost | Savings |
|------------|-----------|--------|-----------|---------|
| find + update + set | 3 calls | pick-next | 1 call | **67%** |
| add + start + set | 3 calls | spawn-subtask | 1 call | **67%** |
| update + set + get | 3 calls | switch | 1 call | **67%** |

## Key Principles

1. **Intent-First**: Create task before executing
2. **Record Everything Critical**: Events are AI's long-term memory
3. **Use Atomic Operations**: Single call > multiple calls
4. **Maintain Hierarchy**: Use parent-child for structure
5. **Always Get Context**: Use `--with-events` when starting/switching
6. **Summary-Only Reports**: Save tokens unless full detail needed

## See Also

- [The Intent-Engine Way](../THE_INTENT_ENGINE_WAY.md) - Full philosophy guide
- [README.md](../README.md) - Complete command reference
- [MCP Setup](../MCP_SETUP.md) - MCP server installation
