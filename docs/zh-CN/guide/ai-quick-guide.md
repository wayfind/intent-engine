# Intent-Engine: AI Quick Reference

**Purpose**: Strategic intent tracking for human-AI collaboration. Not a todo list—a shared memory layer for long-term, complex work.

## When to Use

Create a task when work requires:
- Multiple steps
- Extensive context/spec
- Session interruptions
- Human-AI collaboration

## Core Commands (Atomic = Single Call)

### Start Work
```bash
intent-engine task start <ID> --with-events  # ATOMIC: status→doing + set current + get context
```

### Create & Switch to Subtask
```bash
intent-engine task spawn-subtask --name "X"  # ATOMIC: create + status→doing + switch
```

### Switch Tasks
```bash
intent-engine task switch <ID>  # ATOMIC: status→doing + set current + get events
```

### Smart Batch Selection
```bash
intent-engine task pick-next --max-count 3  # ATOMIC: query + sort + batch transition
```

### Record Critical Moments
```bash
echo "Decision/blocker/milestone..." | \
  intent-engine event add --task-id <ID> --type decision --data-stdin
```

### Complete (Enforces Hierarchy)
```bash
intent-engine task done <ID>  # Fails if subtasks incomplete
```

### Get Summary (Token-Efficient)
```bash
intent-engine report --since 7d --summary-only  # Returns stats only, not full tasks
```

## Workflow Pattern

```bash
# 1. Create task with rich spec
echo "Multi-line markdown spec..." | \
  intent-engine task add --name "Implement OAuth2" --spec-stdin

# 2. Start & load context (returns spec + event history)
intent-engine task start 1 --with-events

# 3. Execute + record key decisions
echo "Chose Passport.js for OAuth strategies" | \
  intent-engine event add --task-id 1 --type decision --data-stdin

# 4. Hit sub-problem? Create & auto-switch
intent-engine task spawn-subtask --name "Configure Google OAuth app"

# 5. Complete child, switch back to parent
intent-engine task done 2
intent-engine task switch 1

# 6. Complete parent
intent-engine task done 1
```

## Batch Problem Handling

```bash
# Discovered 5 bugs? Create all, then smartly select:
for bug in A B C D E; do
  intent-engine task add --name "Fix bug $bug"
done

# Evaluate each
intent-engine task update 1 --complexity 3 --priority 10  # Simple+urgent
intent-engine task update 2 --complexity 8 --priority 10  # Complex+urgent
intent-engine task update 3 --complexity 2 --priority 5   # Simple+normal

# Auto-select by: priority DESC, complexity ASC
intent-engine task pick-next --max-count 3
# → Selects: #1 (P10/C3), #3 (P5/C2), #2 (P10/C8)
```

## Event Types

- `decision` - Chose X over Y because...
- `blocker` - Stuck, need human help
- `milestone` - Completed phase X
- `discussion` - Captured conversation
- `note` - General observation

## Token Optimization

| Old Way | Calls | Atomic | Calls | Saving |
|---------|-------|--------|-------|--------|
| query+update+set current | 3 | pick-next | 1 | 67% |
| create+start+set current | 3 | spawn-subtask | 1 | 67% |
| update+set+get events | 3 | switch | 1 | 67% |
| query all+filter+format | many | report --summary-only | 1 | 90%+ |

## Key Rules

1. **Always use --with-events** when starting/switching tasks (loads context)
2. **Always use --summary-only** for reports (unless debugging)
3. **Record all key decisions** via `event add` (your external memory)
4. **Use atomic commands** (start, switch, spawn-subtask, pick-next)
5. **Respect hierarchy** (complete children before parents)

## Status Flow

```
todo → (start/pick-next/spawn-subtask) → doing → (done) → done
       ↑                                    ↓
       └────────────── (switch) ────────────┘
```

## Quick Checks

```bash
intent-engine current                          # What am I working on?
intent-engine task find --status doing         # All active tasks
intent-engine event list --task-id <ID> --limit 5  # Recent context
intent-engine report --since 1d --summary-only     # Today's summary
```

## Anti-Patterns

❌ Don't manually chain: `task update <ID> --status doing && current --set <ID>`
✅ Do use atomic: `task start <ID> --with-events`

❌ Don't forget to record decisions
✅ Do log every key choice via `event add`

❌ Don't use `report` without `--summary-only` for routine checks
✅ Do use `--summary-only` (saves 90% tokens)

## Philosophy

Intent-Engine is AI's **strategic memory**. Context window = short-term. Events table = long-term. Tasks = goals. Commands = how we achieve them together.

---

**Full docs**: [the-intent-engine-way.md](the-intent-engine-way.md), [README.md](../../../README.md)
