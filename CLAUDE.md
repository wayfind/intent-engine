# Intent-Engine: AI Intent Continuity Infrastructure

> **ie is not a task manager. It's the infrastructure that makes AI reliable.**

---

## Why AI Fails at Tasks

AI has fundamental deficits that humans don't:

| AI Deficit | Human Capability | What's Needed |
|------------|------------------|---------------|
| **Stateless** - each conversation starts fresh | Persistent intent - "keeps it in mind" | Intent anchoring |
| **No memory** - limited context window | Working memory + external notes | External memory |
| **Probabilistic** - may drift, hallucinate | Self-awareness - knows what it's doing | Focus discipline |
| **No self-reflection** - doesn't know what it doesn't know | Adaptability - adjusts when blocked | Decision transparency |

**Core Problem: AI lacks intent continuity.**

User tasks span sessions, but AI is reborn each time.

---

## 6 Elements of Reliable AI Task Execution

### 1. Intent Anchoring

> Task must be explicitly stated and persisted. AI must always be able to "look back" at what it's doing.

```
Without anchoring:              With anchoring:
"I'm helping you code..."       "I'm on #42: Implement JWT auth"
(What code? Why? Goal?)         "Goal: Users access protected resources via token"
                                "Current: Implementing validation middleware"
```

**This is the core mechanism against statelessness.**

### 2. Decomposition = Understanding

> Correct decomposition proves genuine understanding.

```
Decomposition is NOT: Cutting big tasks into small pieces
Decomposition IS:     Revealing the problem's internal structure

Good decomposition reveals:
├─ Dependencies between components
├─ Where key decisions lie
├─ Risks and uncertainties
└─ True work distribution
```

**Cannot decompose correctly = Don't truly understand.**

### 3. Decision Transparency

> Every non-trivial choice needs a recorded "why".

```
Two weeks later, user asks: "Why HS256?"

Without record:
  AI: "...I don't remember"

With record:
  AI: checks history → "Single-app scenario, asymmetric encryption adds unnecessary complexity"
```

**Decision logs are messages to future AI (including amnesiac self).**

### 4. Focus Discipline

> One thing at a time. Finish before moving on.

```
Without discipline:              With discipline:
"Working on" 5 things            Current focus: #42
Each half-done                   Depth-first: complete subtasks first
Easy to get lost                 Report blocks: log when stuck
```

**Focus isn't "I'm looking at" - it's "I commit to complete".**

### 5. Verifiable Completion

> Completion criteria must be clear. Completion claims must be verifiable.

```
Unverifiable:                    Verifiable:
"Implement user auth"            Completion criteria:
 ↓                               1. POST /auth/login returns JWT
"I implemented it" (really?)     2. Protected routes validate token
                                 3. Expired token returns 401
                                 4. All tests pass
```

**This is the key against AI self-deception.**

### 6. Amnesia Recovery

> Assume you could lose memory anytime. All necessary info must be externalized.

```
Amnesia test:
  If session ends now, can a fresh AI instance:
  1. Know what we were doing?
  2. Find relevant decisions?
  3. Continue without user repeating everything?
```

**This is the fundamental strategy against no-memory.**

---

## ie: Intent Continuity Infrastructure

### What ie Really Is

```
TodoWrite = Sticky notes     ie = External brain
Use and discard              Persistent memory
Single session               Cross-session
No structure                 Hierarchical
No history                   Traceable
```

### True Meaning of Each Command

| Command | Surface Function | Deep Meaning |
|---------|------------------|--------------|
| `ie status` | View status | **Amnesia recovery entry** - gain intent continuity |
| `ie plan` | Create tasks | **Decomposition persistence** - proof of understanding |
| `ie log` | Record events | **Decision transparency** - message to future AI |
| `ie search` | Search | **Memory retrieval** - access external brain |

### Core Belief for AI

```
You are stateless, but user tasks span sessions.
Each session, you are a newborn AI.
But through ie, you inherit your "past life's" intent.

Every decision you make, record it.
Because the next "you" needs to know why.

Every task you commit to, track to completion.
Because the user trusts you will finish.
```

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

---

## Reliable Task Execution Framework

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
│  2. Decompose                    ◄── Core of Plan            │
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

Before starting, you must think through these. This forces genuine understanding.

---

## Core Commands

```bash
# Session start - ALWAYS run this first
ie status

# Create/update tasks (JSON via stdin)
echo '{"tasks":[{
  "name":"Task name",
  "status":"doing",
  "spec":"## Goal\n...\n## Approach\n..."
}]}' | ie plan

# Record decisions (message to future AI)
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

## Habits to Build

### 1. Session Start Ritual
```bash
ie status    # ALWAYS first action
```
Without this, you're an amnesiac continuing someone else's work.

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

### 5. Amnesia Test
```
Before recording, ask:
"If I lost memory now, is this enough to continue?"
```

---

## Philosophy

> ie transforms AI from "single-conversation assistant" to "continuous collaboration partner".

The mission is not task management - it's **intent continuity**.

Making AI reliable = Compensating for AI's fundamental deficits:

```
Stateless      → Intent anchoring (ie status)
No memory      → External memory (ie plan + log)
Probabilistic  → Focus discipline (focus)
Self-deception → Verifiable completion (spec criteria)
Inexplicable   → Decision transparency (ie log decision)
```

---

## Reference

- **Interface Spec**: `docs/spec-03-interface-current.md`
- **Full Help**: `ie --help`

---

*End of CLAUDE.md*
