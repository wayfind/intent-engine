# Intent-Engine: Claude Integration Guide

**Version**: 0.1.10
**Target**: Claude Code, Claude Desktop, and AI assistants via MCP

---

## 📖 Authoritative Specification

> **IMPORTANT**: This guide is a practical summary derived from the authoritative specification.
>
> **Single Source of Truth**: `docs/INTERFACE_SPEC.md`
>
> The INTERFACE_SPEC.md document is the **foundational blueprint** that defines:
> - ✅ All CLI command signatures and behaviors
> - ✅ All MCP tool definitions and interfaces
> - ✅ Data models and their exact field names
> - ✅ Atomic operation semantics
> - ✅ Output format specifications
> - ✅ Interface stability guarantees (SemVer)
>
> **In case of any conflict or ambiguity**, the INTERFACE_SPEC.md takes precedence.
>
> This CLAUDE.md guide provides practical usage patterns and integration tips,
> but should always align with the authoritative specification.

---

## 🤖 What is Intent-Engine?

Intent-Engine is your **external long-term memory** for strategic task management. Think of it as:

- **Your Task Brain**: Persistent, hierarchical task tracking across sessions
- **Context Keeper**: Full history of decisions, blockers, and milestones
- **Smart Assistant**: Recommends next tasks based on focus and priority

---

## 🎯 Core Concept: Focus-Driven Workflow

Intent-Engine works like your brain - **one focused task at a time**:

```
┌──────────────────────────────────────┐
│  Workspace State                     │
│  current_task_id: 42                 │  ← "What am I working on?"
└──────────────────────────────────────┘
           │
           ▼
    ┌────────────┐
    │  Task 42   │  ← The Focused Task
    │  "Impl auth"│
    └────┬───┬───┘
         │   │
    ┌────▼┐ ┌▼────┐
    │T43  │ │T44  │  ← Subtasks (depth-first priority)
    │JWT  │ │OAuth│
    └─────┘ └─────┘
```

---

## 🛠️ Available MCP Tools

### Task Management

#### `task_add` - Create Strategic Task
```json
{
  "name": "Implement user authentication",
  "spec": "Use JWT with 7-day expiry, refresh tokens, HS256 algorithm"
}
```

**When to use**:
- User gives you a complex requirement
- You need to track work across sessions
- The task has multiple steps or sub-problems

#### `task_start` - Begin Working
```json
{
  "task_id": 42,
  "with_events": true
}
```

**What it does** (atomic):
1. Sets task status to `doing`
2. Makes it the current focused task
3. Returns full context with decision history

**When to use**:
- Starting a new task
- Resuming work after a break
- User asks "what should I work on?"

#### `task_done` - Complete Current Task
```json
{
  // NO parameters - operates on current_task_id
}
```

**Prerequisites**:
- A task must be current/focused
- All subtasks must be done

**What it does** (atomic):
1. Verifies all children are done
2. Marks current task as done
3. Clears current_task_id (unfocuses)

**When to use**:
- Task is complete and verified
- All subtasks are finished
- Ready to move to next task

#### `task_spawn_subtask` - Decompose Problem
```json
{
  "name": "Configure JWT secret key",
  "spec": "Store in environment variables, use strong random value"
}
```

**What it does** (atomic):
1. Creates subtask under current task
2. Switches focus to new subtask
3. Sets new subtask as `doing`

**When to use**:
- Discover a sub-problem while working
- Need to break down current task
- Want to track detailed steps

#### `task_switch` - Change Focus
```json
{
  "task_id": 43
}
```

**What it does** (atomic):
1. Previous task: `doing` → `todo` (pause)
2. New task: `todo` → `doing` (resume)
3. Update focus to new task

**When to use**:
- Pause current work to handle something else
- Return to a previously started task
- User asks to switch context

#### `task_pick_next` - Get Recommendation
```json
{
  // No required parameters
}
```

**Smart algorithm** (depth-first):
1. **First priority**: Subtasks of current focused task
2. **Second priority**: Top-level todo tasks

**When to use**:
- User asks "what should I work on next?"
- Current task is done, need next step
- Starting a new work session

#### `task_find` - Filter by Metadata
```json
{
  "status": "doing",
  "parent": 42
}
```

**When to use**:
- Find all tasks in a specific status
- List subtasks of a parent
- Query structured properties

**NOT for text search** - use `task_search` instead

#### `task_search` - Full-Text Search
```json
{
  "query": "JWT AND authentication",
  "snippet": true
}
```

**Searches**: Both `name` and `spec` fields

**When to use**:
- Find tasks by content/keywords
- Search for specific technical terms
- Locate related work

### Event Recording

#### `event_add` - Record Decisions/Blockers
```json
{
  "type": "decision",  // or "blocker", "milestone", "note"
  "data": "Chose HS256 over RS256 because we don't need key rotation yet",
  "task_id": 42  // Optional - defaults to current task
}
```

**Two modes**:
1. **During work**: Omit `task_id` → records for current task
2. **Retrospective**: Include `task_id` → records for any task

**When to use**:
- Made an important design decision
- Hit a blocker that needs tracking
- Reached a milestone
- Quick note for future reference

### Reporting

#### `report_generate` - Generate Summary
```json
{
  "since": "7d",
  "summary_only": true
}
```

**When to use**:
- User asks "what have we accomplished?"
- Weekly/daily standup summary
- Project status report

#### `current_task_get` - Get Focused Task
```json
{
  // No parameters
}
```

**When to use**:
- Check what task is currently focused
- Understand current context
- Before performing focus-driven operations

---

## 🎨 Typical Usage Patterns

### Pattern 1: Starting Fresh
```
User: "Help me implement user authentication"

You:
1. task_add(name: "Implement user authentication", spec: "...")
2. task_start(task_id: 42, with_events: true)
3. Review the context and begin work
```

### Pattern 2: Breaking Down Work
```
User: "Let's add authentication"

You:
1. task_start(task_id: 42)
2. Analyze the spec
3. task_spawn_subtask(name: "Design JWT schema")
   → Now subtask is focused
4. Work on subtask
5. task_done() when subtask complete
6. task_pick_next() → Recommends next subtask
```

### Pattern 3: Recording Decisions
```
While implementing JWT:

You: "I chose HS256 algorithm because..."
     event_add(type: "decision", data: "Chose HS256 because...")
```

### Pattern 4: Resuming Work
```
User: "Let's continue with authentication"

You:
1. task_search(query: "authentication")
   → Find task ID 42
2. task_start(task_id: 42, with_events: true)
   → Get full context with decision history
3. Review events_summary
4. Continue from where you left off
```

### Pattern 5: Switching Context
```
User: "Let's pause auth and fix that bug"

You:
1. event_add(type: "note", data: "Pausing to handle bug #123")
2. task_switch(task_id: 67)  # Bug fix task
   → Pauses auth, starts bug fix
3. Fix the bug
4. task_done()
5. task_switch(task_id: 42)  # Back to auth
```

---

## 💡 Best Practices

### 1. Always Start Tasks
```
❌ DON'T: task_done() without starting
✅ DO:    task_start(42) then task_done()
```

### 2. Use Hierarchical Decomposition
```
❌ DON'T: Flat list of 10 implementation steps
✅ DO:    Parent task with 3-4 logical subtasks
```

### 3. Record Important Decisions
```
❌ DON'T: Just implement without context
✅ DO:    event_add() for key design choices
```

### 4. Leverage with_events
```
❌ DON'T: Start task without history
✅ DO:    task_start(task_id, with_events: true)
```

### 5. Let pick-next Guide You
```
❌ DON'T: Manually search for next task
✅ DO:    task_pick_next() for smart recommendation
```

---

## ⚠️ Common Mistakes

### Mistake 1: Passing ID to task_done
```
❌ task_done(task_id: 42)  # WRONG - no parameters

✅ task_start(42)           # Set focus first
   task_done()              # Then complete
```

### Mistake 2: Using find for text search
```
❌ task_find(name_pattern: "JWT")  # WRONG - find is metadata only

✅ task_search(query: "JWT")        # Correct
```

### Mistake 3: Not checking current task
```
❌ Assume no task is focused
   task_done()  # ERROR

✅ current_task_get()  # Check first
   If focused: task_done()
   If not: task_start() first
```

### Mistake 4: Trying to complete parent with incomplete children
```
❌ task_start(42)        # Parent
   task_done()           # ERROR: has incomplete subtasks

✅ task_start(42)        # Parent
   task_spawn_subtask()  # Child 1
   task_done()           # Complete child 1
   task_spawn_subtask()  # Child 2
   task_done()           # Complete child 2
   task_switch(42)       # Back to parent
   task_done()           # Now works - all children done
```

---

## 🎯 When to Use Intent-Engine

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

## 🔄 Integration Workflow

### With Claude Code

When user says:
- "Help me implement X" → Create task, track work
- "What's next?" → Use pick-next
- "Why did we...?" → Check events
- "Continue authentication" → Start task, load context

### Task Lifecycle

```
User Request
    │
    ▼
task_add ──────────────┐
    │                  │ (strategic planning)
    ▼                  │
task_start ─────────────┤
    │                  │ (active work)
    ├── event_add      │
    ├── task_spawn_subtask
    │                  │
    ▼                  │
task_done ─────────────┘
```

---

## 📊 Understanding Output

### TaskWithEvents Structure
```json
{
  "task": {
    "id": 42,
    "name": "Implement authentication",
    "status": "doing",
    "spec": "Use JWT...",
    "first_doing_at": "2024-11-09T10:00:00Z"
  },
  "events_summary": {
    "total_count": 5,
    "recent_events": [
      {
        "log_type": "decision",
        "discussion_data": "Chose HS256 algorithm",
        "timestamp": "2024-11-09T10:15:00Z"
      }
    ]
  }
}
```

**Use this to**:
- Understand current task state
- Review decision history
- Resume work with full context

### PickNextResult Structure
```json
{
  "recommended_task": {
    "id": 43,
    "name": "Configure JWT secret",
    "parent_id": 42
  },
  "reason": "subtask_of_current",
  "context": {
    "current_task_id": 42,
    "strategy": "depth_first"
  }
}
```

**Use this to**:
- Recommend next logical step
- Explain why this task is suggested
- Maintain focus on current work tree

---

## 🧠 Mental Model

Think of Intent-Engine as:

1. **Your Notebook** - Persistent task list across sessions
2. **Your Focus Ring** - One task at a time (current_task_id)
3. **Your Memory** - Decision history in events
4. **Your Guide** - Smart recommendations (pick-next)
5. **Your Tree** - Hierarchical problem breakdown

---

## 📚 Key References

- **Full Spec**: `docs/INTERFACE_SPEC.md`
- **Agent Guide**: `AGENT.md`
- **MCP Schema**: `mcp-server.json`
- **Setup**: `docs/*/integration/mcp-server.md`

---

## 🎓 Philosophy

Intent-Engine is designed for **strategic intent tracking**, not tactical todo lists:

- **What + Why** over "How"
- **Persistent context** over ephemeral notes
- **Hierarchical thinking** over flat lists
- **Decision history** over task status
- **Focus** over multitasking

---

**Last Updated**: 2024-11-09
**Spec Version**: 0.1.10
**MCP Tools**: 13 available
**Status**: Experimental (Pre-1.0)
