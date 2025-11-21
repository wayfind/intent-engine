# Intent-Engine: Claude Integration Guide

**Version**: 0.6.2
**Target**: Claude Code, Claude Desktop, and AI assistants via MCP

---

## 📖 Authoritative Specification

> **IMPORTANT**: This guide is a practical summary derived from the authoritative specification.
>
> **Single Source of Truth**: `docs/spec-03-interface-current.md`
>
> The spec-03-interface-current.md document is the **foundational blueprint** that defines:
> - ✅ All CLI command signatures and behaviors
> - ✅ All MCP tool definitions and interfaces
> - ✅ Data models and their exact field names
> - ✅ Atomic operation semantics
> - ✅ Output format specifications
> - ✅ Interface stability guarantees (SemVer)
>
> **In case of any conflict or ambiguity**, the spec-03-interface-current.md takes precedence.
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

> **Technical details**: See [AGENT.md](AGENT.md#focus-driven-operations) for data models and atomic operation semantics

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

## 🛠️ Essential MCP Tools

> **For detailed technical specifications**, see [AGENT.md](AGENT.md#essential-commands)

### Core Workflow Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| `task_start` | Begin working (sets focus) | `task_id`, `with_events` |
| `task_done` | Complete current task | (no parameters) |
| `task_switch` | Change focus to another task | `task_id` |
| `task_pick_next` | Get smart recommendation | (no parameters) |

### Planning Tools

| Tool | Purpose | Key Parameters | Use Case |
|------|---------|----------------|----------|
| `plan` ⭐ | Declarative batch task creation | `tasks: TaskTree[]` | **Batch operations**, hierarchies, dependencies |
| `task_add` | Create single task (imperative) | `name`, `spec`, `priority` | **Single tasks**, interactive CLI |
| `task_spawn_subtask` | Create and focus on subtask | `name`, `spec` | **Dynamic workflows**, interactive |
| `task_add_dependency` | Add single dependency | `blocked_task_id`, `blocking_task_id` | **Single dependencies**, precise control |

**When to use `plan`**:
- ✅ Creating multiple related tasks at once
- ✅ Complex task hierarchies (parent/child relationships)
- ✅ Tasks with dependencies (automatic cycle detection)
- ✅ Idempotent operations (safe to run multiple times)
- ✅ Importing from external systems (YAML/JSON)

**When to use traditional tools** (`task_add`, etc.):
- ✅ Single task creation
- ✅ Interactive CLI sessions
- ✅ Fine-grained control over each step
- ✅ Simple, straightforward operations

> 💡 **See [PLAN_INTERFACE_GUIDE.md](docs/PLAN_INTERFACE_GUIDE.md) for detailed usage patterns and migration examples**

### Query Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| `task_list` | Filter by status/parent | `status`, `parent` |

### Search and Discovery

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| `search` | Search tasks AND events | `query`, `include_tasks`, `include_events` |

**Search capabilities**:
- Supports FTS5 syntax: `AND`, `OR`, `NOT`, `"phrases"`
- Returns mixed results with task ancestry for events
- Example: `search(query: "JWT AND authentication")`

### Event Tracking

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| `event_add` | Record decision/blocker/note | `type`, `data`, `task_id?` |
| `event_list` | Query events with filters | `task_id?`, `type?`, `since?`, `limit?` |

**Event types**: `decision`, `blocker`, `milestone`, `note`

**Filtering** (new in v0.2):
- By type: `event_list(type: "decision")`
- By time: `event_list(since: "7d")`
- Combined: `event_list(type: "blocker", since: "24h")`

### Workspace and Reporting

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| `current_task_get` | Get focused task | (no parameters) |
| `report_generate` | Generate summary report | `since`, `summary_only` |

### New Features (v0.2+)

**Priority Levels**: Tasks support `critical`, `high`, `medium`, `low`
**Dependencies**: Use `task_add_dependency` to model prerequisites
**Event Filtering**: Filter by type, time range, or both
**Unified Search**: Search across both tasks and events

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
1. search(query: "authentication")
   → Find task ID 42 and related events
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

### Pattern 6: Working with Dependencies (new in v0.2)
```
User: "Implement the API client, but it depends on authentication being done first"

You:
1. task_list(status: "doing")
   → Find current auth task (ID 42)
2. task_add(name: "Implement API client", priority: "high")
   → Creates task ID 50
3. task_add_dependency(blocked_task_id: 50, blocking_task_id: 42)
   → API client now depends on auth completion
4. Continue working on task 42 (auth)
5. When task 42 is done, task_pick_next() will recommend task 50
```

### Pattern 7: Smart Event Filtering (new in v0.2)
```
User: "What decisions did we make on the authentication task?"

You:
1. search(query: "authentication")
   → Find task ID 42 and decision events
2. event_list(task_id: 42, type: "decision")
   → Get only decision events (efficient!)
3. Review and summarize the decisions

Alternative - Recent blockers:
event_list(task_id: 42, type: "blocker", since: "7d")
→ Get blockers from last week only
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

### Mistake 2: Using list for text search
```
❌ task_list(status: "JWT")  # WRONG - list is metadata only (status, parent)

✅ search(query: "JWT")  # Correct - searches tasks and events
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

## 🧠 Mental Model

Think of Intent-Engine as:

1. **Your Notebook** - Persistent task list across sessions
2. **Your Focus Ring** - One task at a time (current_task_id)
3. **Your Memory** - Decision history in events
4. **Your Guide** - Smart recommendations (pick-next)
5. **Your Tree** - Hierarchical problem breakdown

---

## 📚 Key References

- **Interface Spec** (authoritative): `docs/spec-03-interface-current.md`
- **AI Agent Guide** (technical details): `AGENT.md`
- **MCP Schema**: `mcp-server.json`
- **Setup Guide**: `docs/*/integration/mcp-server.md`

> For data models, output formats, and command specifications, see [AGENT.md](AGENT.md)

---

## 🎓 Philosophy

Intent-Engine is designed for **strategic intent tracking**, not tactical todo lists:

- **What + Why** over "How"
- **Persistent context** over ephemeral notes
- **Hierarchical thinking** over flat lists
- **Decision history** over task status
- **Focus** over multitasking

---

**Last Updated**: 2025-11-17
**Spec Version**: 0.6.2
**MCP Tools**: 14 available (search tool for unified search)
**Status**: Experimental (Pre-1.0)
