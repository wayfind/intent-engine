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
ie task start <ID> --with-events  # ATOMIC: status→doing + set current + get context
```

### Create & Switch to Subtask
```bash
ie task spawn-subtask --name "X"  # ATOMIC: create + status→doing + switch
```

### Switch Tasks
```bash
ie task switch <ID>  # ATOMIC: status→doing + set current + get events
```

### Smart Batch Selection
```bash
ie task pick-next --max-count 3  # ATOMIC: query + sort + batch transition
```

### Record Critical Moments
```bash
# To current task (concise)
echo "Decision/blocker/milestone..." | \
  ie event add --type decision --data-stdin

# To specific task (flexible)
echo "Decision/blocker/milestone..." | \
  ie event add --task-id <ID> --type decision --data-stdin
```

### Complete (Enforces Hierarchy)
```bash
ie task done  # Fails if subtasks incomplete
```

### Get Summary (Token-Efficient)
```bash
ie report --since 7d --summary-only  # Returns stats only, not full tasks
```

## Workflow Pattern

```bash
# 1. Create task with rich spec
echo "Multi-line markdown spec..." | \
  ie task add --name "Implement OAuth2" --spec-stdin

# 2. Start & load context (returns spec + event history)
ie task start 1 --with-events

# 3. Execute + record key decisions (to current task)
echo "Chose Passport.js for OAuth strategies" | \
  ie event add --type decision --data-stdin

# 4. Hit sub-problem? Create & auto-switch
ie task spawn-subtask --name "Configure Google OAuth app"

# 5. Complete child (current task), switch back to parent
ie task done
ie task switch 1

# 6. Complete parent
ie task done
```

## Batch Problem Handling

```bash
# Discovered 5 bugs? Create all, then smartly select:
for bug in A B C D E; do
  ie task add --name "Fix bug $bug"
done

# Evaluate each
ie task update 1 --complexity 3 --priority 10  # Simple+urgent
ie task update 2 --complexity 8 --priority 10  # Complex+urgent
ie task update 3 --complexity 2 --priority 5   # Simple+normal

# Auto-select by: priority DESC, complexity ASC
ie task pick-next --max-count 3
# → Selects: #1 (P10/C3), #3 (P5/C2), #2 (P10/C8)
```

## Event Types

- `decision` - Chose X over Y because...
- `blocker` - Stuck, need human help
- `milestone` - Completed phase X
- `discussion` - Captured conversation
- `note` - General observation

## Advanced Pattern: Replace Intermediate Files

AI workflows often create temporary files (scratchpad.md, plan.md, etc.). Intent-Engine offers a superior alternative.

### Core Mapping

| Traditional File | Intent-Engine Way | Advantage |
|-----------------|-------------------|-----------|
| `requirements.md` | Task Spec (`--spec-stdin`) | Bound to task, auto-loaded on start |
| `scratchpad.md` | Event (`type: note`) | Timestamped, always linked to task |
| `plan.md` | Subtasks | Trackable status, dynamic plan |
| `error_log.txt` | Event (`type: blocker`) | Explicitly marked, easy to review |
| `design_v2.md` | Task Spec (new task) | Design + execution unified |

### Example: Debug Analysis

```bash
# ❌ Old way: Create debug_analysis.md
cat > debug_analysis_task5.md <<EOF
Browser console findings:
1. Button click doesn't trigger network request
2. Console error: TypeError...
EOF

# ✅ New way: Store directly in event stream
echo "Browser console findings:
1. Button click doesn't trigger network request
2. Console error: TypeError...
3. Initial diagnosis: event binding failed" | \
  ie event add --type note --data-stdin  # Uses current task
```

### Example: Technical Proposal

```bash
# ❌ Old way: Create design_v2.md awaiting approval
cat > design_v2.md <<EOF
V2 Refactor: Replace Class Components with React Hooks
EOF

# ✅ New way: Create subtask, spec is the proposal
cat design_v2.md | \
  ie task spawn-subtask --name "Execute V2 refactor" --spec-stdin
# Auto-switches to new task, spec already loaded
```

### Transformative Benefits

1. **Token Efficiency**:
   - Old: Read entire scratchpad.md (includes irrelevant info)
   - New: Precise `event list --task-id 5 --limit 10`

2. **Query Power**:
   - Old: "How did I solve this before?" → manual file hunting
   - New: `task search "TypeError" --status done` → instant location

3. **Clean Workspace**:
   - Old: Project root flooded with `temp_xxx.md`
   - New: All thought fragments in `.intent-engine/project.db`

4. **Auto-Linking**:
   - Old: Manually tag task ID in filename (`bug_5_analysis.md`)
   - New: Database foreign keys auto-link, zero maintenance

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
ie current                          # What am I working on?
ie task find --status doing         # All active tasks
ie task search "keyword"            # Search tasks by content (FTS5)
ie event list --task-id <ID> --limit 5  # Recent context
ie report --since 1d --summary-only     # Today's summary
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
