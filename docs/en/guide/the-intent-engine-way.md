# The Intent-Engine Way: A Guide to Intent-Driven Collaboration

## Introduction: This is Not Just a Task List

Welcome to Intent-Engine. Before you begin, the most important thing to understand is: it's not a traditional "todo" app, but rather the cornerstone of a collaboration model.

Its core goal is to establish a shared, traceable **"Intent"** layer for human-AI collaboration. Humans set strategic goals (The Intent), AI executes tactical operations (The Execution), and Intent-Engine is the core engine connecting these two layers.

This guide explains the standard workflow of using Intent-Engine, when to use it, how to use it, and why to use it this way.

---

## Step 1: Capture Intent (When & How to `task add`)

### When

When an idea or requirement becomes "complex enough," it should be captured as an Intent-Engine task. Trigger conditions include:

- **Multi-step tasks**: When you or AI foresee that completing this requirement needs multiple independent operations
- **Requires context**: When task execution requires extensive background information, discussion history, or specific specifications
- **Long-cycle work**: When the task cannot be completed in a single interaction or session and needs to be interrupted and resumed
- **Collaboration node**: When task completion requires multiple rounds of Q&A, feedback, and intervention between humans and AI

A smart AI Agent should be trained to recognize these signals and proactively suggest to humans:

> "This seems like a complex task. I recommend creating an Intent-Engine task to track it. Do you agree?"

### How

Use `intent-engine task add`. The key is the quality of the `spec`.

```bash
# Pass detailed, structured requirements via pipe to --spec-stdin
echo "# Goal: Implement OAuth2 login

## Requirements:
- Support Google and GitHub
- Keep password login as fallback
- Token validity: 7 days, support refresh

## Technical Constraints:
- Use OAuth2 PKCE flow
- Frontend-backend separation architecture" | intent-engine task add --name "Implement OAuth2 login" --spec-stdin
```

### Why

This is the starting point of the entire process. A clear, structured spec sets a clear "axiom" for AI. This fundamentally reduces errors and rework caused by unclear requirement understanding. We transform vague conversations into clear, executable intent.

---

## Step 2: Activate Intent (When & How to `task start`)

### When

When you or AI decide to officially begin working on a captured intent. This is a clear "work start" signal.

### How

Always use `intent-engine task start <ID> --with-events`.

```bash
# AI decides to start task #42
intent-engine task start 42 --with-events
```

### Why

This isn't just changing status to `doing`. The `start` command is a carefully designed atomic operation that does at least three critical things:

1. **Declare ownership**: Updates task status to `doing`, informing all collaborators (including other AIs or humans) "I'm working on this task"
2. **Focus attention**: Automatically points the system's workspace focus (`current_task_id`) to this task, providing a baseline for all subsequent operations
3. **Load context**: Returns complete task `spec` and `events_summary` in one call. This allows AI to get all target information and historical background needed to start work in a single call, extremely efficient

---

## Step 2.5: Smart Planning (When & How to `pick-next`) 🆕

### When

When AI discovers multiple problems that need handling, and these problems:

- **Already evaluated**: Each problem's complexity and priority are clear
- **Need sorting**: Need to automatically decide processing order, not by creation order
- **Capacity management**: Need to control number of simultaneous tasks (WIP limit)

### How

First create tasks and evaluate them, then use `pick-next` for smart selection:

```bash
# 1. AI discovers 5 issues in code review
intent-engine task add --name "Fix null pointer exception"
intent-engine task add --name "Optimize database query"
intent-engine task add --name "Fix memory leak"
intent-engine task add --name "Update outdated dependencies"
intent-engine task add --name "Add error logging"

# 2. AI evaluates complexity (1-10) and priority for each task
intent-engine task update 1 --complexity 3 --priority 10  # Null pointer: simple but urgent
intent-engine task update 2 --complexity 7 --priority 8   # Database: complex and important
intent-engine task update 3 --complexity 9 --priority 10  # Memory: complex but urgent
intent-engine task update 4 --complexity 5 --priority 5   # Dependencies: medium
intent-engine task update 5 --complexity 2 --priority 3   # Logging: simple not urgent

# 3. Smart select top 3 tasks (by priority DESC, complexity ASC)
intent-engine task pick-next --max-count 3 --capacity 5
# Result: Will select task 1 (P10/C3), 3 (P10/C9), 2 (P8/C7)
```

### Why

This embodies the philosophy "let AI focus on thinking, let system handle scheduling":

- **Token saving**: One call completes "query todo → evaluate capacity → sort → batch update", saves 60-70% API calls
- **Decision consistency**: Uses unified algorithm (priority DESC, complexity ASC) ensuring predictable decision logic
- **Capacity protection**: Automatically enforces WIP limits, prevents efficiency drop from opening too many tasks simultaneously

---

## Step 3: Execute and Record (The Execution Loop & `event add`)

This is the core of the Intent-Engine pattern. When AI executes tasks, it enters a "perceive-think-act-record" loop.

### When (When to record events)

At every key node in the execution loop, you must use `intent-engine event add` to record. Key nodes include:

- **Making important decisions** (`--type decision`): "I chose library A over library B because..."
- **Encountering obstacles** (`--type blocker`): "I need API key, cannot continue"
- **Receiving human feedback** (`--type discussion`): "Human confirmed dependency installation complete"
- **Completing a milestone** (`--type milestone`): "Database migration script written, awaiting tests"
- **After an attempt fails** (`--type note`): "Executing Action A failed, error is..., next will try Action B"

### How

Alternate between using various tools in the "toolbox" and writing key thinking process back to Intent-Engine.

```bash
# 1. AI perceives environment (using underlying tools)
git status
ls -R

# 2. AI makes decision and acts (e.g., modify files)
# ... a series of file edits ...

# 3. AI records its key decisions (using Intent-Engine)
echo "Refactored token validation logic.

Reason: Original logic did not properly handle expired tokens.

Improvements:
- Added token expiration time check
- Implemented auto-refresh mechanism
- Increased unit test coverage" | intent-engine event add --task-id 42 --type decision --data-stdin
```

### Why

Intent-Engine is AI's **external long-term memory**. AI's context window is limited, it will "forget". The `events` table transforms AI's transient thinking process into permanent, queryable project knowledge. This enables:

- **Prevent repeating mistakes**: AI can review history, know which paths don't work
- **Support interrupt and resume**: Any collaborator can seamlessly take over work by reading event history
- **Enable human-AI collaboration**: Event is the only channel for AI to "request help" from humans and receive "external guidance"
- **Provide audit trail**: Provides precise record of "what actually happened" for post-mortem reviews

---

## Step 3.5: Handle Sub-problems (When & How to `spawn-subtask`) 🆕

### When

During execution, when AI discovers:

- **Prerequisite dependency**: Current task depends on solving a sub-problem
- **Problem decomposition**: Discovers task is too complex, needs decomposition into smaller units
- **Recursive discovery**: Discovers even finer sub-problems while handling subtasks

### How

Use `spawn-subtask` to create and switch to a subtask under current task:

```bash
# AI is working on task #42: Implement OAuth2 login
intent-engine task start 42 --with-events

# During implementation, discovers need to configure OAuth app first
intent-engine task spawn-subtask --name "Configure OAuth app on Google and GitHub"

# This automatically:
# 1. Creates subtask (parent_id = 42)
# 2. Sets subtask status to doing
# 3. Switches current task to subtask
# 4. Returns subtask details

# While configuring OAuth app, discovers need to apply for domain verification first
echo "Need to complete domain ownership verification before creating OAuth app" | \
  intent-engine event add --task-id <child-task-id> --type blocker --data-stdin

intent-engine task spawn-subtask --name "Complete domain ownership verification"

# Complete deepest subtask
intent-engine task done <grandchild-task-id>

# Switch back to parent task and continue
intent-engine task switch <child-task-id>
intent-engine task done <child-task-id>

# Finally complete root task
intent-engine task switch 42
intent-engine task done
```

### Why

This enforces the business rule "must complete subtasks before completing parent task":

- **Keep hierarchy clear**: Avoid flattening many tasks, making dependencies hard to understand
- **Atomic switch**: Completes create, start, set as current task in one step, saves tokens
- **Enforce completeness**: System checks if all subtasks are complete, prevents omissions

---

## Step 3.6: Task Switching (When & How to `switch`) 🆕

### When

When you need to switch between multiple ongoing tasks:

- **Pause current task**: Handle more urgent tasks
- **Parallel work**: Switch to other tasks while waiting for external feedback
- **Task tree navigation**: Navigate back and forth between parent and subtasks

### How

Use `switch` to quickly switch between tasks and get complete context:

```bash
# Currently working on frontend task #5
intent-engine task switch 5

# Suddenly discover backend API has issue, need to fix first
intent-engine task switch 12  # Switch to backend task

# switch will automatically:
# 1. Update task #12 status to doing (if not already)
# 2. Set #12 as current task
# 3. Return task details and event summary

# View context after switch
# Output includes events_summary, helps AI quickly recover memory

# Fix complete, switch back to frontend task
intent-engine task switch 5
```

### Why

This is effective management of AI's working memory:

- **Atomic operation**: Merges "get task → update status → set as current → get events" into one call
- **Context recovery**: Automatically returns events_summary, helps AI quickly recall "where did I leave off on this task"
- **State consistency**: Ensures each switch correctly updates task status and workspace focus

---

## Step 4: Complete Intent (When & How to `task done`)

### When

When all goals defined in `spec` have been achieved, and all subtasks (if any) are complete.

### How

Always use `intent-engine task done`.

```bash
intent-engine task done
```

If the task still has incomplete subtasks, system will return error:

```json
{
  "error": "Cannot complete task 42: it has 2 incomplete subtasks"
}
```

### Why

Like `start`, `done` is also an atomic operation with built-in safety checks. It enforces the core business rule "must complete all subtasks first", ensuring logical consistency of the task tree in Intent-Engine.

It's not a simple status change, but final confirmation that "this intent along with all its sub-intents have been fully achieved."

---

## Step 5: Review and Insight (When & How to `report`)

### When

When you need to generate periodic reports (e.g., weekly reports), conduct project retrospectives, or analyze efficiency for specific types of work (e.g., bug fixes).

### How

Use `intent-engine report`, and **prefer `--summary-only`**.

```bash
# AI needs to generate summary for weekly report
intent-engine report --since 7d --status done --summary-only

# Example output (compact JSON summary):
# {
#   "summary": {
#     "total_count": 23,
#     "todo_count": 5,
#     "doing_count": 3,
#     "done_count": 15
#   }
# }

# AI receives this compact JSON summary, then expands it into a complete report in natural language
```

More query examples:

```bash
# View all tasks from last 1 day (with details)
intent-engine report --since 1d

# View all in-progress tasks
intent-engine report --status doing --summary-only

# Search completed tasks related to "authentication"
intent-engine report --filter-name "authentication" --status done --summary-only

# Combined query: database optimization work completed in last 30 days
intent-engine report --since 30d --status done --filter-spec "database" --summary-only
```

### Why

This embodies the best practice of **"leave computation at data source"**. AI's strength is language and reasoning, not data aggregation.

Letting Intent-Engine efficiently complete all statistical calculations internally, only returning final, high-value "insight" results to AI:

- **Greatly saves token consumption**: `--summary-only` only returns statistics, not all task details
- **Reduces cost**: Fewer tokens means lower API costs
- **Improves quality**: AI uses its precious context space for higher quality thinking and creation, not data processing

---

## Complete Workflow Example

### Scenario: AI Discovers Multiple Issues in Code Review

```bash
# 1. Capture intent - AI discovers 5 issues
intent-engine task add --name "Fix null pointer exception in UserService"
intent-engine task add --name "Optimize database query performance"
intent-engine task add --name "Fix memory leak issue"
intent-engine task add --name "Update outdated dependency packages"
intent-engine task add --name "Add error logging"

# 2. Evaluate - AI analyzes complexity and priority for each issue
intent-engine task update 1 --complexity 3 --priority 10
intent-engine task update 2 --complexity 7 --priority 8
intent-engine task update 3 --complexity 9 --priority 10
intent-engine task update 4 --complexity 5 --priority 5
intent-engine task update 5 --complexity 2 --priority 3

# 3. Smart planning - automatically select optimal task order
intent-engine task pick-next --max-count 3 --capacity 5
# System selects: task 1(P10/C3), 3(P10/C9), 2(P8/C7)

# 4. Execute first task
intent-engine task switch 1

# 4.1 Record decision
echo "Problem cause: UserService.getUser() did not check if return value is null
Fix solution: Add Optional wrapping and null check
Impact scope: 3 call sites" | \
  intent-engine event add --task-id 1 --type decision --data-stdin

# 4.2 Execute fix
# ... modify code ...

# 4.3 Complete task
intent-engine task done

# 5. Handle second task (includes subtask)
intent-engine task switch 3

# 5.1 Discover need to diagnose problem first
echo "Need to use profiler to locate memory leak source" | \
  intent-engine event add --task-id 3 --type blocker --data-stdin

intent-engine task spawn-subtask --name "Analyze memory usage with Valgrind"

# 5.2 Complete diagnosis
echo "Problem found: WebSocket connections not properly closed" | \
  intent-engine event add --task-id <subtask-id> --type milestone --data-stdin
intent-engine task done <subtask-id>

# 5.3 Switch back and complete main task
intent-engine task switch 3
# ... fix code ...
intent-engine task done

# 6. Generate work report
intent-engine report --since 1d --summary-only
```

---

## Core Principles Summary

### 1. Intent-First
Don't let AI execute aimlessly. Clarify intent (task) first, then start action.

### 2. Record Everything Critical
AI's memory will fade, but Intent-Engine won't. Every important decision should be recorded.

### 3. Prefer Atomic Operations
Prefer using composite commands like `start`, `pick-next`, `spawn-subtask`, `switch`, `done` instead of manually combining multiple low-level operations.

### 4. Clear Hierarchy
Use parent-child tasks to keep work structure clear. Big tasks decompose into small tasks, small tasks must complete before big tasks can complete.

### 5. Context is King
Always use `--with-events` to get complete context. AI needs to know "why" and "how", not just "what".

### 6. Token Efficiency
Use `--summary-only`, atomic operations, smart selection, and other mechanisms to maximize the value of each token.

---

## Anti-Pattern Warnings

### ❌ Don't: Directly manipulate status
```bash
# Wrong: Manually combine multiple operations
intent-engine task update 42 --status doing
intent-engine current --set 42
intent-engine task get 42 --with-events
```

### ✅ Should: Use atomic operations
```bash
# Correct: One step
intent-engine task start 42 --with-events
```

---

### ❌ Don't: Flatten all tasks
```bash
# Wrong: All sub-problems created as independent root tasks
intent-engine task add --name "Implement OAuth2"
intent-engine task add --name "Configure Google OAuth"
intent-engine task add --name "Configure GitHub OAuth"
intent-engine task add --name "Implement token refresh"
```

### ✅ Should: Use hierarchical structure
```bash
# Correct: Use parent-child relationship
intent-engine task add --name "Implement OAuth2"
intent-engine task start 1
intent-engine task spawn-subtask --name "Configure Google OAuth"
intent-engine task done
intent-engine task spawn-subtask --name "Configure GitHub OAuth"
intent-engine task done
intent-engine task spawn-subtask --name "Implement token refresh"
intent-engine task done
intent-engine task switch 1
intent-engine task done
```

---

### ❌ Don't: Forget to record key decisions
```bash
# Wrong: AI made important decision but didn't record
# ... chose library A ...
# ... directly continue next step ...
```

### ✅ Should: Record all key nodes
```bash
# Correct: Record decision process
echo "Chose to use Passport.js instead of writing OAuth logic from scratch

Reasons:
- Mature and stable, good community support
- Supports multiple strategies
- Reduces maintenance burden

Trade-offs:
- Increases dependencies
- Need to learn its API" | \
  intent-engine event add --task-id 1 --type decision --data-stdin
```

---

## Conclusion

Intent-Engine is not just a tool, it's an implementation of a collaboration philosophy.

It organically combines human strategic thinking with AI execution capabilities, through clear intent capture, strict state management, and complete historical records, making human-AI collaboration **traceable**, **recoverable**, and **scalable**.

Mastering "The Intent-Engine Way" is mastering the art of collaborating with AI.

---

**Next Steps**: Read the complete command reference ([README.en.md](../../../README.en.md)) and technical analysis ([task-workflow-analysis.md](../../technical/task-workflow-analysis.md)).
