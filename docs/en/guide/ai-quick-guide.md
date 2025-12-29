# Intent-Engine: AI Quick Reference

**Purpose**: Strategic intent tracking for human-AI collaboration. Not a todo list—a shared memory layer for long-term, complex work.

## When to Use

Create a task when work requires:
- Multiple steps or sessions
- Extensive context/specification
- Decision tracking ("why did I choose X?")
- Human-AI collaboration

## Core Commands (v0.10.0)

### 1. Restore Context (Always First)
```bash
ie status              # What am I working on? Full context recovery
ie status 42           # View specific task context
```

### 2. Create/Update/Complete Tasks
```bash
# All task operations go through `ie plan` with JSON stdin
echo '{"tasks":[...]}' | ie plan
```

### 3. Record Events
```bash
ie log decision "Chose X over Y because..."
ie log blocker "Stuck on API rate limit"
ie log milestone "Phase 1 complete"
ie log note "Consider caching later"
```

### 4. Search History
```bash
ie search "todo doing"       # Find unfinished tasks
ie search "JWT auth"         # Full-text search
ie search "decision"         # Find decisions
```

## Workflow Pattern

```bash
# 1. Create task with spec (status:doing requires spec)
echo '{"tasks":[{
  "name": "Implement OAuth2",
  "status": "doing",
  "spec": "## Goal\nUsers authenticate via OAuth\n\n## Approach\nUse Passport.js"
}]}' | ie plan

# 2. Check current context
ie status

# 3. Record key decision
ie log decision "Chose Passport.js for OAuth - mature library, good docs"

# 4. Break down into subtasks (auto-parented to focused task)
echo '{"tasks":[
  {"name": "Configure Google OAuth", "status": "todo"},
  {"name": "Configure GitHub OAuth", "status": "todo"},
  {"name": "Implement callback handler", "status": "todo"}
]}' | ie plan

# 5. Start working on subtask
echo '{"tasks":[{
  "name": "Configure Google OAuth",
  "status": "doing",
  "spec": "Set up Google Cloud Console, get credentials"
}]}' | ie plan

# 6. Complete subtask
echo '{"tasks":[{"name": "Configure Google OAuth", "status": "done"}]}' | ie plan

# 7. Complete all subtasks, then parent
echo '{"tasks":[{"name": "Implement OAuth2", "status": "done"}]}' | ie plan
```

## JSON Task Format

```json
{
  "tasks": [
    {
      "name": "Task name (required, unique identifier)",
      "status": "todo|doing|done",
      "spec": "Description (required for doing)",
      "priority": "critical|high|medium|low",
      "parent_id": null,
      "children": [...]
    }
  ]
}
```

### Key Fields

| Field | Required | Notes |
|-------|----------|-------|
| `name` | Yes | Unique identifier, used for updates |
| `status` | No | `todo` (default), `doing`, `done` |
| `spec` | For `doing` | Goal + approach description |
| `priority` | No | `critical`, `high`, `medium`, `low` |
| `parent_id` | No | `null` = root task, omit = auto-parent to focus |
| `children` | No | Nested subtask array |

## Event Types

| Type | When to Use |
|------|-------------|
| `decision` | Chose X over Y because... |
| `blocker` | Stuck, need help or info |
| `milestone` | Completed significant phase |
| `note` | General observation |

## Key Rules

1. **`ie status` first** — Always run at session start (amnesia recovery)
2. **`doing` needs spec** — Must have goal + approach before starting
3. **Children first** — Parent can't be `done` until all children are `done`
4. **Same name = update** — `ie plan` is idempotent, won't create duplicates
5. **Auto-parenting** — New tasks become children of focused task (use `parent_id: null` for root)

## Common Patterns

### Create Independent Root Task
```bash
echo '{"tasks":[{
  "name": "Unrelated bug fix",
  "status": "todo",
  "parent_id": null
}]}' | ie plan
```

### Hierarchical Task with Children
```bash
echo '{"tasks":[{
  "name": "User Authentication",
  "status": "doing",
  "spec": "Complete auth system",
  "children": [
    {"name": "JWT tokens", "status": "todo"},
    {"name": "Session management", "status": "todo"},
    {"name": "Password reset", "status": "todo"}
  ]
}]}' | ie plan
```

### Update Task Priority
```bash
echo '{"tasks":[{
  "name": "Existing task",
  "priority": "critical"
}]}' | ie plan
```

### Find Unfinished Work
```bash
ie search "todo doing"
```

## Anti-Patterns

| Don't | Do |
|-------|-----|
| Start `doing` without spec | Always include goal + approach |
| Forget to record decisions | `ie log decision "..."` immediately |
| Create tasks without checking focus | `ie status` first |
| Mark parent done with incomplete children | Complete all children first |

## Session Workflow

```
Session Start:
  ie status                    # Restore context

During Work:
  ie plan (create/update)      # Task operations
  ie log decision "..."        # Record choices

Session End:
  ie plan (status:done)        # Complete finished work
  ie status                    # Verify state
```

## Philosophy

Intent-Engine is AI's **external brain**:
- **ie status** = Amnesia recovery
- **ie plan** = Decomposition persistence
- **ie log** = Decision transparency
- **ie search** = Memory retrieval

---

**Full docs**: [CLAUDE.md](../../../CLAUDE.md), [quickstart.md](quickstart.md)
