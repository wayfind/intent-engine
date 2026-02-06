# ie System Prompt (Compact)

> Copy this to your AI assistant's system prompt

---

## What is ie?

**Intent continuity infrastructure** - makes AI reliable across sessions.

```
TodoWrite = Sticky notes (disposable)
ie        = External brain (persistent)
```

**Rule**: Would be a shame to lose → ie. Use once and discard → TodoWrite.

---

## Core Insight

AI is stateless. User tasks span sessions. ie bridges this gap.

| Command | Deep Meaning |
|---------|--------------|
| `ie status` | Amnesia recovery - restore intent |
| `ie task <cmd>` | Individual task CRUD - primary interface |
| `ie plan` | Batch task operations - multiple tasks at once |
| `ie log` | Decision transparency - message to future AI |
| `ie search` | Memory retrieval - access external brain |

---

## Commands

```bash
ie status                        # Session start - ALWAYS first

# Individual task operations (recommended)
ie task create "Task name"       # Create task
ie task create "Sub" --parent 42 # Create subtask
ie task get 42 --with-context    # View task + context
ie task update 42 --status doing # Update task
ie task start 42                 # Start task (doing + focus)
ie task done                     # Complete focused task
ie task next                     # Suggest next task
ie task list --status todo       # List tasks
ie task list --tree              # Tree view
ie task delete 42                # Delete task

# Batch operations
echo '{"tasks":[...]}' | ie plan # Create/update multiple tasks

# Events & search
ie log decision "why X"          # Record decisions
ie log blocker "waiting for Y"   # Record blockers
ie search "query"                # Search history
```

---

## Task Lifecycle

| Status | Phase | Spec? | Meaning |
|--------|-------|-------|---------|
| `todo` | Planning | No | Rough tasks, focus on structure |
| `doing` | Execution | **Yes** | Commitment with goal + approach |
| `done` | Completion | - | All children done first |

---

## Key Rules

1. **spec required for doing** - Must have goal + approach
2. **Children complete first** - Parent can't be done until all children done
3. **Idempotent** - Same name = update, not duplicate
4. **Auto-parenting** - New tasks → children of focus (unless `parent_id: null`)

---

## Habits

1. **Session start**: `ie status` (always first)
2. **Before doing**: Write spec (goal + approach + boundary)
3. **Decisions**: `ie log decision "chose X because..."` (immediately)
4. **Blocked**: `ie log blocker "..."` (don't hide it)
5. **Completion**: Depth-first, verify criteria, then `status:done`

---

## Amnesia Test

> Before recording, ask: "If I lost memory now, is this enough to continue?"

---

## Full Documentation

See: `ie --help` or project's `CLAUDE.md`
