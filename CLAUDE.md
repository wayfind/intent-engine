# Intent-Engine: AI Long-Term Memory System

> **Give your AI the memory it deserves.**

---

## The Challenge

AI assistants are incredibly capable within a single conversation. But when sessions end:

```
Day 1: "Let's build authentication"
       AI works brilliantly, makes smart decisions...
       [session ends]

Day 2: "Continue authentication"
       AI: "What authentication? I have no memory of this."
```

**The challenge isn't capability—it's continuity.**

User tasks span days, weeks, months. AI conversations are ephemeral.

---

## The Solution: External Memory

Intent-Engine gives AI persistent memory across sessions:

| Challenge | Solution | How ie Helps |
|-----------|----------|--------------|
| Sessions are ephemeral | Persistent task state | `ie status` restores full context |
| Decisions get lost | Decision history | `ie log` records the "why" |
| Context needs repeating | External memory | `ie search` retrieves past work |
| Complex work needs structure | Hierarchical tasks | `ie plan` organizes intent |

**One command restores everything:**

```bash
ie status
# Returns: current task, goal, approach, decision history, subtasks
```

---

## 6 Principles of Effective AI Collaboration

### 1. Intent Anchoring

> Clear goals lead to better outcomes.

```
Vague:                          Clear:
"I'm helping you code..."       "I'm on #42: Implement JWT auth"
(What code? Why? Goal?)         "Goal: Users access protected resources"
                                "Current: Implementing validation middleware"
```

**When AI knows exactly what it's working on, it stays focused.**

### 2. Decomposition = Understanding

> Breaking down problems reveals their structure.

```
Good decomposition reveals:
├─ Dependencies between components
├─ Where key decisions lie
├─ Risks and uncertainties
└─ True work distribution
```

**If you can decompose it well, you understand it well.**

### 3. Decision Transparency

> Record the "why" behind every choice.

```
Two weeks later, user asks: "Why HS256?"

Without record:
  AI: "...I don't know"

With record:
  AI: checks history → "Single-app scenario, asymmetric encryption adds unnecessary complexity"
```

**Decision logs are messages to your future self.**

### 4. Focus Discipline

> One thing at a time. Finish before moving on.

```
Scattered:                      Focused:
"Working on" 5 things           Current focus: #42
Each half-done                  Depth-first: complete subtasks first
Easy to get lost                Report blocks: log when stuck
```

**Focus isn't "I'm looking at" - it's "I commit to complete".**

### 5. Verifiable Completion

> Know what "done" looks like before you start.

```
Vague:                          Clear:
"Implement user auth"           Completion criteria:
 ↓                              1. POST /auth/login returns JWT
"I implemented it" (really?)    2. Protected routes validate token
                                3. Expired token returns 401
                                4. All tests pass
```

**Clear criteria prevent premature completion claims.**

### 6. Context Recovery

> Externalize everything needed to continue later.

```
Session recovery test:
  If this session ends now, can I:
  1. Know what we were doing?
  2. Find relevant decisions?
  3. Continue without user repeating everything?
```

**If yes, the context is properly externalized.**

---

## What ie Really Is

```
TodoWrite = Sticky notes     ie = External brain
Use and discard              Persistent memory
Single session               Cross-session
No structure                 Hierarchical
No history                   Traceable
```

### Command Meanings

| Command | Function | Purpose |
|---------|----------|---------|
| `ie status` | View current state | **Context recovery** - restore working memory |
| `ie plan` | Batch create/update tasks | **Intent persistence** - externalize goals (batch) |
| `ie task create` | Create a single task | **Task creation** - with metadata, deps, owner |
| `ie task get` | Get task details | **Task inspection** - with events and context |
| `ie task update` | Update a task | **Task mutation** - any field, metadata, deps |
| `ie task list` | List/filter tasks | **Task discovery** - filter, sort, tree view |
| `ie task delete` | Delete a task | **Task cleanup** - with optional cascade |
| `ie task start` | Start a task | **Focus + status** - sets doing and focuses |
| `ie task done` | Complete a task | **Completion** - by ID or current focus |
| `ie task next` | Suggest next task | **Prioritization** - context-aware pick |
| `ie log` | Record events | **Decision history** - capture the "why" |
| `ie search` | Find history | **Memory retrieval** - access past context |

---

## Quick Decision: ie vs TodoWrite

| Scenario | TodoWrite | ie |
|----------|-----------|-----|
| Single session, disposable | ✓ | |
| Cross-session work | | ✓ |
| Need to record "why I decided this" | | ✓ |
| Complex multi-level breakdown | | ✓ |
| Need to review decision history | | ✓ |

**Simple rule:**
- Would be a shame to lose → ie
- Use once and discard → TodoWrite

**ie task vs ie plan:**
- Single task operation (create, update, delete) → `ie task` (preferred)
- Batch create/update multiple tasks at once → `ie plan`

---

## Task Execution Framework

```
┌─────────────────────────────────────────────────────────────┐
│                      User Intent                             │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Understand                                               │
│     Ask: Do I truly understand this problem?                 │
│     Output: Clear goal statement                             │
│     Verify: Can I restate in my own words? Boundaries clear? │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Decompose                                                │
│     Ask: What's the structure of this problem?               │
│     Output: Task tree (hierarchical intent decomposition)    │
│     Verify: Does decomposition reveal dependencies?          │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  3. Commit                                                   │
│     Ask: Which task do I commit to complete?                 │
│     Output: Clear focus (one at a time)                      │
│     Verify: Does spec contain verifiable completion criteria?│
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  4. Execute                                                  │
│     Depth-first, report blocks, record decisions             │
│     Output: Code changes + decision logs                     │
│     Verify: Check against completion criteria                │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  5. Verify                                                   │
│     Ask: Are all completion criteria met?                    │
│     Output: Verification result                              │
│     Mark: status:done                                        │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
                      Back to step 3
                     (next task)
```

---

## Task Status Lifecycle

| Status | Phase | Spec Required? | Meaning |
|--------|-------|----------------|---------|
| `todo` | **Planning** | No | Tasks can be rough, focus on structure |
| `doing` | **Execution** | **Yes** | Commitment - must have goal + approach |
| `done` | **Completion** | - | All children must be done first |

### Why spec is required for doing?

**spec is not "description" - it's "execution contract":**
- Goal: What does completion look like?
- Approach: How will you achieve it?
- Boundary: What's NOT included?

Before starting, think through these. This ensures clarity before action.

---

## Core Commands

```bash
# Session start - ALWAYS run this first
ie status

# Individual task operations (preferred for single tasks)
ie task create "Task name"                          # Create task
ie task create "Subtask" --parent 42                # Create subtask
ie task get 42 --with-context                       # View task details
ie task update 42 --status doing --priority 1       # Update task
ie task update 42 --metadata type=epic              # Set metadata
ie task start 42                                    # Start task (doing + focus)
ie task done                                        # Complete focused task
ie task done 42                                     # Complete specific task
ie task next                                        # Suggest next task
ie task list --status todo                          # List todo tasks
ie task list --tree                                 # Show task tree
ie task delete 42 --cascade                         # Delete task + children

# Batch create/update tasks (JSON via stdin)
echo '{"tasks":[{
  "name":"Task name",
  "status":"doing",
  "spec":"## Goal\n...\n## Approach\n..."
}]}' | ie plan

# Record decisions (message to future self)
ie log decision "Chose X because Y"
ie log blocker "Waiting for Z"

# Search history
ie search "todo doing"    # Unfinished tasks
ie search "keyword"       # Full-text search
```

---

## Key Rules

1. **spec required for doing** - Starting task requires spec with goal + approach
2. **Children complete first** - Cannot mark parent done until all children done
3. **Idempotent** - Same task name = update (not duplicate)
4. **Auto-parenting** - New tasks become children of focused task (unless `parent_id: null`)

---

## Best Practices

### 1. Session Start Ritual
```bash
ie status    # ALWAYS first action
```
This restores your working context from the previous session.

### 2. Think Before Starting
```
Before status:doing, ask:
- What's the goal? (completion criteria)
- What's my approach?
- What's NOT included?

Write these in spec.
```

### 3. Record Decisions Immediately
```bash
# Whenever you make a non-trivial choice
ie log decision "Chose X because..."
```
Two weeks later, someone will ask "why X?" - make sure you can answer.

### 4. Depth-First Completion
```
Don't jump between tasks.
Complete subtasks before siblings.
If blocked, log it: ie log blocker "..."
```

### 5. Context Recovery Test
```
Before ending a session, ask:
"Is everything needed to continue recorded?"
```

---

## Philosophy

> ie transforms AI from "single-conversation assistant" to "continuous collaboration partner".

The mission is **intent continuity** - giving AI the memory infrastructure to work reliably across sessions.

```
Ephemeral sessions  → Persistent context (ie status)
Lost decisions      → Decision history (ie log)
Repeated context    → Searchable memory (ie search)
Vague goals         → Structured intent (ie plan)
```

---

## Reference

- **Interface Spec**: `docs/spec-03-interface-current.md`
- **Full Help**: `ie --help`

---

*End of CLAUDE.md*
