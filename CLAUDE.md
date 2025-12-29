# Intent-Engine: AI Long-Term Task Memory System

**Version**: 0.10
**Target**: Claude Code, Claude Desktop, and AI assistants

---

## Claude Code Plugin Installation

Install the plugin for automatic session integration:

```bash
# 1. Add marketplace
claude plugin marketplace add wayfind/origin-task

# 2. Install plugin
claude plugin install intent-engine
```

After installation, the plugin automatically:
- Runs `ie status` at every session start
- Shows your current focused task and progress
- Auto-installs `ie` CLI via npm if not found

**Manual CLI Installation** (if needed):

```bash
npm install -g @m3task/intent-engine
# or: cargo install intent-engine
# or: brew install wayfind/tap/intent-engine
```

---

## Quick Decision: TodoWrite vs ie

**You already have TodoWrite** (built into Claude Code). When to use ie?

| Scenario | Use TodoWrite | Use ie |
|----------|---------------|--------|
| Simple checklist within a single session | ✅ | |
| Cross-session project work | | ✅ |
| Need to record "why I made this decision" | | ✅ |
| Complex multi-level task breakdown | | ✅ |
| Need to review decision history later | | ✅ |
| Temporary tasks, no need to preserve | ✅ | |

**Simple rule**:
- **Would be a shame to lose it** → use ie
- **Use once and discard** → use TodoWrite

---

## Session Start Standard Action

**At the start of each new session, run**:

```bash
ie status
```

This tells you:
- What is the currently focused task (if any)
- Which subtasks remain to be completed
- Where you left off last time
- Sibling task progress

**When no focused task exists**, `ie status` shows all root tasks, helping you choose where to start.

---

## Task Management Decision Tree

When a user request involves tasks, follow these rules:

```
Does user request involve tasks?
    │
    ├─ Need to preserve across sessions?
    │   ├─ No → TodoWrite
    │   └─ Yes → ie ✓
    │
    ├─ Has multiple sub-steps (3+)?
    │   ├─ 1-2 steps → TodoWrite
    │   └─ 3+ steps → ie (use hierarchical structure) ✓
    │
    ├─ Need to record "why I did this"?
    │   ├─ No → TodoWrite
    │   └─ Yes → ie (use ie log for decisions) ✓
    │
    └─ Is this an existing ie project?
        └─ Run ie status to check
            ├─ Has in-progress tasks → continue with ie ✓
            └─ No tasks → decide based on above rules
```

---

## Core Commands Quick Reference

| Command | Purpose | Example |
|---------|---------|---------|
| `ie status [id]` | View task context | `ie status` or `ie status 42` |
| `ie plan` | Create/update/complete tasks | `echo '{"tasks":[...]}' \| ie plan` |
| `ie log <type> <msg>` | Record decisions/blockers/milestones | `ie log decision "Chose JWT"` |
| `ie search <query>` | Search tasks and events | `ie search "todo doing"` |

---

## Authoritative Specification

> **IMPORTANT**: This guide is a practical summary derived from the authoritative specification.
>
> **Single Source of Truth**: `docs/spec-03-interface-current.md`
>
> The spec-03-interface-current.md document is the **foundational blueprint** that defines:
> - ✅ All CLI command signatures and behaviors
> - ✅ JSON output formats and data structures
> - ✅ Data models and their exact field names
> - ✅ Atomic operation semantics
> - ✅ Output format specifications
> - ✅ Interface stability guarantees (SemVer)
>
> **In case of any conflict or ambiguity**, the spec-03-interface-current.md takes precedence.

---

## What is Intent-Engine?

Intent-Engine is your **external long-term memory** for strategic task management. Think of it as:

- **Your Task Brain**: Persistent, hierarchical task tracking across sessions
- **Context Keeper**: Full history of decisions, blockers, and milestones
- **Smart Assistant**: Recommends next tasks based on focus and priority

---

## Core Concept: Focus-Driven Workflow

Intent-Engine works like your brain - **one focused task at a time**:

```
┌──────────────────────────────────────┐
│  Workspace State                     │
│  current_task_id: 42                 │  ← "What am I working on?"
└──────────────────────────────────────┘
           │
           ▼
    ┌────────────┐
    │  Task 42   │  ← The Focused Task (doing + current)
    │  "Impl auth"│
    └────┬───┬───┘
         │   │
    ┌────▼┐ ┌▼────┐
    │T43  │ │T44  │  ← Subtasks (depth-first priority)
    │JWT  │ │OAuth│
    └─────┘ └─────┘
```

**Important**: The system supports **multiple 'doing' tasks** simultaneously for hierarchical workflows. However, only **one task is focused** (current_task_id) at any time.

---

## CLI Commands (v0.10.0)

> **Simplified 6-command CLI** - All task operations go through `plan`

### Core Commands

| Command | Purpose | Example |
|---------|---------|---------|
| `ie plan` | Create/update tasks (from stdin JSON) | `echo '{"tasks":[...]}' \| ie plan` |
| `ie log <type> <message>` | Record events | `ie log decision "Chose JWT"` |
| `ie search <query>` | Search tasks and events | `ie search "todo doing"` |
| `ie status [id]` | View task context | `ie status` or `ie status 42` |
| `ie init` | Initialize project | `ie init` |
| `ie dashboard <cmd>` | Dashboard management | `ie dashboard start` |
| `ie doctor` | Check system health | `ie doctor` |

### Plan Command - The Universal Tool

`ie plan` handles ALL task operations through JSON:

```bash
# Create tasks
echo '{"tasks":[{"name":"Implement auth","status":"doing"}]}' | ie plan

# Update task status
echo '{"tasks":[{"name":"Implement auth","status":"done"}]}' | ie plan

# Create hierarchical tasks
echo '{"tasks":[{
  "name":"Parent task",
  "status":"doing",
  "children":[
    {"name":"Subtask 1","status":"todo"},
    {"name":"Subtask 2","status":"todo"}
  ]
}]}' | ie plan
```

### @file Syntax - Including File Content

Use `@file(path)` to include content from a file into task description (spec):

```bash
# Write detailed description to a temp file
cat > /tmp/task-desc.md << 'EOF'
## Goal
Implement user authentication with JWT

## Approach
- Use HS256 algorithm
- Token expiry: 24h
- Refresh token: 7d
EOF

# Create task with description from file
echo '{"tasks":[{
  "name":"Implement auth",
  "status":"doing",
  "spec":"@file(/tmp/task-desc.md)"
}]}' | ie plan
# File is automatically deleted after successful plan execution

# Keep the file (don't delete)
echo '{"tasks":[{"name":"Task","spec":"@file(/tmp/desc.md, keep)"}]}' | ie plan
```

### Description Requirement

**Tasks must have a description (spec) when starting (status: doing):**

```bash
# ❌ This will fail - no spec when starting
echo '{"tasks":[{"name":"My Task","status":"doing"}]}' | ie plan

# ✅ This works - spec provided
echo '{"tasks":[{"name":"My Task","status":"doing","spec":"Goal: ..."}]}' | ie plan

# ✅ Creating todo tasks without spec is OK (will show warning)
echo '{"tasks":[{"name":"My Task","status":"todo"}]}' | ie plan
```

**Rationale:** Before starting a task, you should know:
- What is the goal
- How you plan to approach it

**Status indicators in `ie status`:**
- Tasks without description show ⚠️ marker
- This helps track which tasks need more context

### Completion Requirement

**Parent tasks cannot be completed until all children are done:**

```bash
# ❌ This will fail - child is not complete
echo '{"tasks":[{"name":"Parent Task","status":"done"}]}' | ie plan
# Error: Cannot complete task 'Parent Task': has incomplete subtasks

# ✅ Complete children first, then parent
echo '{"tasks":[{"name":"Child Task","status":"done"}]}' | ie plan
echo '{"tasks":[{"name":"Parent Task","status":"done"}]}' | ie plan
```

**Rationale:** A task is not truly complete until all its subtasks are done.

### Explicit Parent Assignment (parent_id)

Control task hierarchy explicitly using `parent_id`:

```bash
# Create a root task (ignores focused task auto-parenting)
echo '{"tasks":[{"name":"Independent Task","parent_id":null}]}' | ie plan

# Assign to specific parent by ID
echo '{"tasks":[{"name":"Child Task","parent_id":42}]}' | ie plan

# Move existing task to new parent
echo '{"tasks":[{"name":"Existing Task","parent_id":99}]}' | ie plan
```

**Three-state logic:**
- `parent_id` absent → Default behavior (auto-parent to focused task for new tasks)
- `parent_id: null` → Explicitly create as root task
- `parent_id: 42` → Explicitly set parent to task #42

**Priority:** `children` nesting > `parent_id` > auto-parenting

### Log Command - Event Recording

```bash
ie log decision "Chose HS256 for JWT signing"
ie log blocker "API rate limit hit"
ie log milestone "MVP feature complete"
ie log note "Consider caching optimization"
ie log decision "message" --task 42  # Target specific task
```

### Search Command - Smart Query

```bash
ie search "todo doing"           # Status filter (unfinished tasks)
ie search "JWT authentication"   # FTS5 full-text search
ie search "API AND client"       # Boolean operators
```

---

## Typical Usage Patterns

### Pattern 1: Starting Fresh
```
User: "Help me implement user authentication"

You:
1. Create task with ie plan
2. Search for context: ie search "authentication"
3. Update status to 'doing': ie plan with status update
4. Begin work and record decisions with ie log
```

### Pattern 2: Breaking Down Work
```
User: "Let's add authentication"

You:
1. Create parent task with subtasks using ie plan:
   echo '{"tasks":[{
     "name":"Implement authentication",
     "status":"doing",
     "children":[
       {"name":"Design JWT schema","status":"todo"},
       {"name":"Implement token validation","status":"todo"}
     ]
   }]}' | ie plan
2. Update subtask status as you work
3. Complete subtask when done
```

### Pattern 3: Recording Decisions
```
While implementing JWT:

You: "I chose HS256 algorithm because..."
     ie log decision "Chose HS256 for performance and simplicity"
```

### Pattern 4: Creating Independent Tasks
```
User: "Create a separate task for that bug fix"

You:
# Use parent_id: null to create root task independent of current focus
echo '{"tasks":[{"name":"Fix bug #123","parent_id":null,"status":"todo"}]}' | ie plan
```

### Pattern 5: Resuming Work
```
User: "Let's continue with authentication"

You:
1. ie search "todo doing"       # Check unfinished tasks
2. ie search "authentication"   # Find specific tasks
3. Update status to continue:
   echo '{"tasks":[{"name":"Implement authentication","status":"doing"}]}' | ie plan
```

---

## Best Practices

### 1. Use Status-Based Workflow
```
❌ DON'T: Forget to update status
✅ DO:    echo '{"tasks":[{"name":"Task","status":"doing"}]}' | ie plan
```

### 2. Use Hierarchical Decomposition
```
❌ DON'T: Flat list of 10 implementation steps
✅ DO:    Parent task with 3-4 logical subtasks
```

### 3. Record Important Decisions
```
❌ DON'T: Just implement without context
✅ DO:    ie log decision "Chose X because..."
```

### 4. Use parent_id for Independent Tasks
```
❌ DON'T: Let unrelated tasks become children of current focus
✅ DO:    echo '{"tasks":[{"name":"Unrelated","parent_id":null}]}' | ie plan
```

### 5. Keep Tasks Updated
```
❌ DON'T: Forget to mark tasks done
✅ DO:    Update status promptly via ie plan
```

---

## Common Mistakes

### Mistake 1: Forgetting to update status
```
❌ Work on task without updating status

✅ echo '{"tasks":[{"name":"My Task","status":"doing"}]}' | ie plan
   # ... do work ...
   echo '{"tasks":[{"name":"My Task","status":"done"}]}' | ie plan
```

### Mistake 2: Using search incorrectly
```
❌ ie search "status:doing"  # WRONG - not a filter syntax

✅ ie search "todo doing"    # Status keywords only → filter mode
✅ ie search "JWT auth"      # Contains non-status words → FTS5 search
```

### Mistake 3: Creating duplicate tasks
```
❌ Run same ie plan twice → creates duplicates? NO!

✅ ie plan is idempotent - same name = update, not create
```

### Mistake 4: Unintended auto-parenting
```
❌ New task becomes child of focused task unexpectedly

✅ Use parent_id: null for independent root tasks:
   echo '{"tasks":[{"name":"Independent","parent_id":null}]}' | ie plan
```

### Mistake 5: Completing parent before children
```
❌ Try to complete parent with incomplete children
   echo '{"tasks":[{"name":"Parent","status":"done"}]}' | ie plan
   # Error: has incomplete subtasks

✅ Complete all children first, then parent:
   echo '{"tasks":[{"name":"Child 1","status":"done"}]}' | ie plan
   echo '{"tasks":[{"name":"Child 2","status":"done"}]}' | ie plan
   echo '{"tasks":[{"name":"Parent","status":"done"}]}' | ie plan
```

---

## When to Use Intent-Engine

### ✅ GOOD Use Cases

1. **Multi-session work**
   - "Let's implement authentication" (will take multiple conversations)
   - Complex features that span days

2. **Hierarchical problems**
   - "Design and implement API endpoints" (has multiple sub-steps)
   - Need to break down large tasks

3. **Decision tracking**
   - "Why did we choose approach X?" (record decisions)
   - Project retrospectives

4. **Context recovery**
   - "What were we working on?" (resume after break)
   - "What decisions have we made?" (review history)

### ❌ NOT Ideal For

1. **Single-step tasks**
   - "Fix this typo" (too trivial)
   - Quick one-liners

2. **Exploratory questions**
   - "What is JWT?" (informational only)
   - No actual work being tracked

3. **Temporary context**
   - Current conversation already has context
   - Won't need this information later

---

## Integration Workflow

### With Claude Code

When user says:
- "Help me implement X" → Create task via `ie plan`, track work
- "What's next?" → Use `ie search "todo doing"`
- "Why did we...?" → Use `ie search` for events
- "Continue authentication" → Update status via `ie plan`
- "Create separate task for Y" → Use `parent_id: null` for root task

### Task Lifecycle

```
User Request
    │
    ▼
ie plan (create) ──────────┐
    │                      │ (strategic planning)
    ▼                      │
ie plan (status:doing) ────┤
    │                      │ (active work)
    ├── ie log             │
    ├── ie plan (children) │
    │                      │
    ▼                      │
ie plan (status:done) ─────┘
```

---

## Mental Model

Think of Intent-Engine as:

1. **Your Notebook** - Persistent task list across sessions
2. **Your Focus Ring** - One task at a time (current_task_id)
3. **Your Memory** - Decision history in events (ie log)
4. **Your Search** - Find anything with ie search
5. **Your Tree** - Hierarchical problem breakdown

---

## Key References

- **Interface Spec** (authoritative): `docs/spec-03-interface-current.md`
- **AI Agent Guide** (technical details): `AGENT.md`
- **Plan Command Guide**: `ie plan --help`

---

## Philosophy

Intent-Engine is designed for **strategic intent tracking**, not tactical todo lists:

- **What + Why** over "How"
- **Persistent context** over ephemeral notes
- **Hierarchical thinking** over flat lists
- **Decision history** over task status
- **Focus** over multitasking

---

*End of CLAUDE.md*
