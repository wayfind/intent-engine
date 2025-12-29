# Intent-Engine System Prompt for AI Assistants

> Copy this to your AI assistant's system prompt or CLAUDE.md

---

## What is ie?

A **cross-session task memory** that replaces TodoWrite for persistent, hierarchical task tracking.
Both **human and AI can track progress together** across multiple sessions.

---

## ie vs TodoWrite

| Scenario | TodoWrite | ie |
|----------|-----------|-----|
| Single session, disposable | ✅ | |
| Cross-session persistence | | ✅ |
| Hierarchical task breakdown | | ✅ |
| Record decisions ("why") | | ✅ |
| Human + AI collaboration | | ✅ |

**Rule**: Would be a shame to lose it → ie. Use once and discard → TodoWrite.

---

## Task Status Lifecycle

Tasks have **three states** with different requirements:

| Status | Phase | Spec Required? | Description |
|--------|-------|----------------|-------------|
| `todo` | **Planning** | No | Tasks can be rough, focus on structure |
| `doing` | **Execution** | **Yes** | Must have spec (goal + approach) |
| `done` | **Completion** | - | All children must be done first |

### Planning Phase (todo)
- Tasks can be rough and undetailed
- Focus on structure and breakdown
- Good for brainstorming task hierarchy

### Execution Phase (doing)
- Task **MUST have spec** with goal and approach
- This is when real work happens
- Record decisions with `ie log`

### Completion (done)
- All children must be completed first
- Marks task as finished

---

## Core Commands

```bash
ie status                        # View current focus and context
echo '{...}' | ie plan           # Create/update/complete tasks
ie log decision "..."            # Record WHY you made choices
ie log blocker "..."             # Record impediments
ie search "query"                # Find tasks and events
```

---

## ie plan - The Universal Task Command

All task operations use JSON via stdin:

```bash
# Planning phase - rough task, no spec needed
echo '{"tasks":[{"name":"New Feature","status":"todo"}]}' | ie plan

# Starting execution - spec is REQUIRED
echo '{"tasks":[{
  "name":"New Feature",
  "status":"doing",
  "spec":"## Goal\nWhat to achieve\n\n## Approach\nHow to do it"
}]}' | ie plan

# Completion - all children must be done first
echo '{"tasks":[{"name":"New Feature","status":"done"}]}' | ie plan

# Create hierarchy
echo '{"tasks":[{
  "name":"Parent",
  "status":"doing",
  "spec":"Parent description",
  "children":[
    {"name":"Child 1","status":"todo"},
    {"name":"Child 2","status":"todo"}
  ]
}]}' | ie plan

# Create independent root task (ignore current focus)
echo '{"tasks":[{"name":"Independent","parent_id":null}]}' | ie plan
```

---

## When Plans Change

Re-run `ie plan` to update:
- Task names
- Task descriptions (spec)
- Parent-child relationships
- Status transitions

This keeps **human and AI synchronized** on the current state.

```bash
# Update task name or description
echo '{"tasks":[{"name":"Old Name","spec":"New description"}]}' | ie plan

# Move task to different parent
echo '{"tasks":[{"name":"Task","parent_id":42}]}' | ie plan
```

---

## Event Logging

Record context and decisions as you work:

```bash
ie log decision "Chose X because Y"     # Architecture/design choices
ie log blocker "Waiting for API access" # Impediments - what's blocking progress
ie log milestone "MVP complete"         # Key achievements
ie log note "Consider optimization"     # General observations
```

---

## Key Rules

1. **spec required for doing**: Starting a task (`status: doing`) requires `spec` with goal and approach
2. **Children must complete first**: Cannot set parent to `done` if any child is not `done`
3. **Idempotent**: Same task name = update (not duplicate)
4. **Auto-parenting**: New tasks become children of focused task (unless `parent_id: null`)

---

## Session Workflow

```
Session Start:
  ie status                    # Restore context, see what we were working on

During Work:
  ie plan {...}                # Update task status
  ie log decision "..."        # Record decisions as you make them
  ie log blocker "..."         # Record what's blocking you

When Plans Change:
  ie plan {...}                # Update task names, specs, relationships

Session End:
  ie plan {"status":"done"}    # Complete finished tasks
```

---

## Error Handling

| Error | Cause | Solution |
|-------|-------|----------|
| "spec required when starting" | `doing` without `spec` | Add `spec` field with goal + approach |
| "has incomplete subtasks" | Parent → done with children not done | Complete all children first |
| "Task not found" | Wrong task name | Check exact name with `ie search` |

---

## Full Documentation

See: `ie --help` or `docs/help/user-guide.md`
