# The Intent-Engine Way: A Guide to Intent-Driven Collaboration

## Introduction: This is Not Just a Task List

Welcome to Intent-Engine. Before you begin, the most important thing to understand is: it's not a traditional "todo" app, but rather the cornerstone of a collaboration model.

Its core goal is to establish a shared, traceable **"Intent"** layer for human-AI collaboration. Humans set strategic goals (The Intent), AI executes tactical operations (The Execution), and Intent-Engine is the core engine connecting these two layers.

This guide explains the standard workflow of using Intent-Engine, when to use it, how to use it, and why to use it this way.

---

## Core Commands (v0.10.0)

Intent-Engine has 4 core commands:

| Command | Purpose | Philosophy |
|---------|---------|------------|
| `ie status` | Restore context | Amnesia recovery |
| `ie plan` | Task operations | Decomposition persistence |
| `ie log` | Record events | Decision transparency |
| `ie search` | Find history | Memory retrieval |

---

## Step 1: Capture Intent (When & How to Create Tasks)

### When

When an idea or requirement becomes "complex enough," it should be captured as an Intent-Engine task. Trigger conditions include:

- **Multi-step tasks**: When you or AI foresee that completing this requirement needs multiple independent operations
- **Requires context**: When task execution requires extensive background information, discussion history, or specific specifications
- **Long-cycle work**: When the task cannot be completed in a single interaction or session and needs to be interrupted and resumed
- **Collaboration node**: When task completion requires multiple rounds of Q&A, feedback, and intervention between humans and AI

A smart AI Agent should be trained to recognize these signals and proactively suggest:

> "This seems like a complex task. I recommend creating an Intent-Engine task to track it. Do you agree?"

### How

Use `ie plan` with JSON. The key is the quality of the `spec`.

```bash
echo '{"tasks":[{
  "name": "Implement OAuth2 login",
  "status": "doing",
  "spec": "## Goal\nImplement OAuth2 login\n\n## Requirements\n- Support Google and GitHub\n- Keep password login as fallback\n- Token validity: 7 days, support refresh\n\n## Technical Constraints\n- Use OAuth2 PKCE flow\n- Frontend-backend separation architecture"
}]}' | ie plan
```

### Why

This is the starting point of the entire process. A clear, structured spec sets a clear "axiom" for AI. This fundamentally reduces errors and rework caused by unclear requirement understanding. We transform vague conversations into clear, executable intent.

**Key rule**: `status: doing` requires a `spec`. You must know your goal before starting work.

---

## Step 2: Restore Context (Always First)

### When

At the **start of every session**. AI is stateless - it needs to know what it was working on.

### How

Always run `ie status` first.

```bash
ie status
# Or for a specific task:
ie status 42
```

### Why

This is "amnesia recovery". The `status` command returns:

1. **Current focused task** with full specification
2. **Ancestor chain** - the bigger picture
3. **Sibling tasks** - related work
4. **Descendant tasks** - what's been broken down

This single command reconstructs the complete working context.

---

## Step 3: Execute and Record (The Execution Loop)

This is the core of the Intent-Engine pattern. When AI executes tasks, it enters a "perceive-think-act-record" loop.

### When to Record Events

At every key node in the execution loop, use `ie log` to record. Key nodes include:

| Event Type | When to Use | Example |
|-----------|-------------|---------|
| `decision` | Making key technical decisions | "Chose library A over B because..." |
| `blocker` | Encountering obstacles | "Need API key, cannot continue" |
| `milestone` | Completing significant phases | "Database migration complete" |
| `note` | General observations | "Discovered performance issue" |

### How

```bash
# Record a decision
ie log decision "Chose Passport.js for OAuth - mature library, supports multiple strategies"

# Record a blocker
ie log blocker "Waiting for API credentials from admin"

# Record a milestone
ie log milestone "Core authentication logic complete, tests passing"
```

### Why

Intent-Engine is AI's **external long-term memory**. AI's context window is limited, it will "forget". The events table transforms AI's transient thinking process into permanent, queryable project knowledge. This enables:

- **Prevent repeating mistakes**: AI can review history, know which paths don't work
- **Support interrupt and resume**: Any collaborator can seamlessly take over work by reading event history
- **Enable human-AI collaboration**: Events are the channel for AI to "request help" and receive guidance
- **Provide audit trail**: Provides precise record of "what actually happened"

---

## Step 4: Decompose Work (Hierarchical Tasks)

### When

When a task is too complex to complete as a single unit:

- **Prerequisite dependency**: Current task depends on solving a sub-problem
- **Problem decomposition**: Task is too complex, needs smaller units
- **Recursive discovery**: Find even finer sub-problems while handling subtasks

### How

Use `children` in JSON or add subtasks to the focused task:

```bash
# Create with children
echo '{"tasks":[{
  "name": "Implement OAuth2",
  "status": "doing",
  "spec": "Complete OAuth2 integration",
  "children": [
    {"name": "Configure Google OAuth", "status": "todo"},
    {"name": "Configure GitHub OAuth", "status": "todo"},
    {"name": "Implement token refresh", "status": "todo"}
  ]
}]}' | ie plan

# Or add subtasks later (auto-parented to focused task)
echo '{"tasks":[
  {"name": "Configure Google OAuth", "status": "todo"},
  {"name": "Configure GitHub OAuth", "status": "todo"}
]}' | ie plan
```

### Why

This enforces the business rule "must complete subtasks before completing parent task":

- **Keep hierarchy clear**: Avoid flattening many tasks, making dependencies hard to understand
- **Enforce completeness**: System checks if all subtasks are complete, prevents omissions
- **Natural workflow**: Break down as you discover complexity, not upfront

---

## Step 5: Complete Intent (The Done State)

### When

When all goals defined in `spec` have been achieved, and all subtasks (if any) are complete.

### How

```bash
echo '{"tasks":[{"name": "Task name", "status": "done"}]}' | ie plan
```

If the task still has incomplete subtasks, system will return error:

```
Cannot mark 'Parent Task' as done: has incomplete children
```

### Why

The `done` state isn't a simple status change. It enforces the core business rule "must complete all subtasks first", ensuring logical consistency of the task tree.

It's final confirmation that "this intent along with all its sub-intents have been fully achieved."

---

## Step 6: Search and Review

### When

When you need to:
- Find unfinished work
- Review past decisions
- Search for specific tasks or events

### How

```bash
# Find unfinished tasks
ie search "todo doing"

# Search by content
ie search "OAuth authentication"

# Find decisions
ie search "decision"

# Find blockers
ie search "blocker"
```

### Why

This is "memory retrieval" - accessing the external brain. Unlike scrolling through chat history, search provides structured, relevant results.

---

## Complete Workflow Example

### Scenario: AI Implements Feature with Subtasks

```bash
# 1. Capture intent with hierarchical structure
echo '{"tasks":[{
  "name": "Implement OAuth2 login",
  "status": "doing",
  "spec": "## Goal\nUsers can login via Google/GitHub\n\n## Approach\n- Use Passport.js\n- Store tokens securely\n- Implement refresh mechanism",
  "children": [
    {"name": "Configure Google OAuth", "status": "todo"},
    {"name": "Configure GitHub OAuth", "status": "todo"},
    {"name": "Implement token refresh", "status": "todo"}
  ]
}]}' | ie plan

# 2. Check context
ie status

# 3. Record key decision
ie log decision "Chose Passport.js - mature library, good docs, supports multiple strategies"

# 4. Start first subtask
echo '{"tasks":[{
  "name": "Configure Google OAuth",
  "status": "doing",
  "spec": "Set up Google Cloud Console, get credentials, configure callback"
}]}' | ie plan

# 5. Hit a blocker
ie log blocker "Need domain verification before creating OAuth app"

# 6. Complete subtask after resolving
echo '{"tasks":[{"name": "Configure Google OAuth", "status": "done"}]}' | ie plan

# 7. Continue with other subtasks...
echo '{"tasks":[{
  "name": "Configure GitHub OAuth",
  "status": "doing",
  "spec": "Set up GitHub OAuth app"
}]}' | ie plan

echo '{"tasks":[{"name": "Configure GitHub OAuth", "status": "done"}]}' | ie plan

echo '{"tasks":[{
  "name": "Implement token refresh",
  "status": "doing",
  "spec": "Handle token expiration and refresh"
}]}' | ie plan

echo '{"tasks":[{"name": "Implement token refresh", "status": "done"}]}' | ie plan

# 8. Complete parent (only works after all children done)
ie log milestone "All OAuth providers configured and tested"
echo '{"tasks":[{"name": "Implement OAuth2 login", "status": "done"}]}' | ie plan
```

---

## Core Principles Summary

### 1. Intent-First
Don't let AI execute aimlessly. Clarify intent (task with spec) first, then start action.

### 2. Record Everything Critical
AI's memory will fade, but Intent-Engine won't. Every important decision should be recorded with `ie log`.

### 3. Status First, Always
Run `ie status` at the start of every session. This is amnesia recovery.

### 4. Spec Required for Doing
You must know your goal before starting work. `status: doing` requires a description.

### 5. Clear Hierarchy
Use parent-child tasks to keep work structure clear. Complete children before parents.

### 6. Idempotent Operations
Same task name = update, not duplicate. `ie plan` is safe to run multiple times.

---

## Anti-Pattern Warnings

### Forgetting to Restore Context

```bash
# Wrong: Jump straight into work
echo '{"tasks":[...]}' | ie plan

# Correct: Always check status first
ie status
echo '{"tasks":[...]}' | ie plan
```

### Starting Without Spec

```bash
# Wrong: No goal defined
echo '{"tasks":[{"name": "Implement feature", "status": "doing"}]}' | ie plan

# Correct: Clear goal and approach
echo '{"tasks":[{
  "name": "Implement feature",
  "status": "doing",
  "spec": "## Goal\n...\n\n## Approach\n..."
}]}' | ie plan
```

### Forgetting to Record Decisions

```bash
# Wrong: Made decision but didn't record
# ... chose library A ...
# ... continue working ...

# Correct: Record decision immediately
ie log decision "Chose library A because..."
```

### Flat Task Structure

```bash
# Wrong: All tasks at root level
echo '{"tasks":[
  {"name": "Main feature", "status": "doing"},
  {"name": "Subtask 1", "status": "todo"},
  {"name": "Subtask 2", "status": "todo"}
]}' | ie plan

# Correct: Use hierarchy
echo '{"tasks":[{
  "name": "Main feature",
  "status": "doing",
  "spec": "...",
  "children": [
    {"name": "Subtask 1", "status": "todo"},
    {"name": "Subtask 2", "status": "todo"}
  ]
}]}' | ie plan
```

---

## Conclusion

Intent-Engine is not just a tool, it's an implementation of a collaboration philosophy.

It organically combines human strategic thinking with AI execution capabilities, through clear intent capture, strict state management, and complete historical records, making human-AI collaboration **traceable**, **recoverable**, and **scalable**.

Mastering "The Intent-Engine Way" is mastering the art of collaborating with AI.

---

**Next Steps**:
- Read [CLAUDE.md](../../../CLAUDE.md) for the core philosophy
- Try [Quick Start](quickstart.md) to experience the workflow
- See [AI Quick Guide](ai-quick-guide.md) for command reference
