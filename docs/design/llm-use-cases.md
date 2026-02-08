# LLM Integration Use Cases

## Overview

Intent-Engine integrates LLM capabilities to **augment task management intelligence**, not replace core functionality. The LLM acts as an analytical assistant that helps maintain task clarity over time.

---

## Use Case 1: Task Structure Analysis & Reorganization Hints

### Problem

During long-term task management:
- Task hierarchies become misaligned with actual work
- Dependencies drift as understanding evolves
- Subtasks outgrow their parents
- Related tasks scatter across the tree

**Example**:
```
#42 Implement Auth [doing]
  ├─ #43 Design database schema [done]
  ├─ #44 JWT implementation [done]
  └─ #45 Add tests [todo]

#50 Fix login bug [doing]  ← Should be under #42
#51 OAuth support [todo]   ← Should be under #42
```

### Solution

**LLM analyzes task structure at key moments and suggests reorganization.**

#### Trigger Points

1. **After completing a task** - Check if parent needs restructuring
2. **Before starting a task** - Detect if task belongs elsewhere
3. **On `ie status`** - Periodic structure health check
4. **Explicit check**: `ie plan analyze` - User-requested analysis

#### What LLM Analyzes

```rust
struct TaskStructureAnalysis {
    current_tree: TaskTree,
    events_history: Vec<Event>,
    recent_decisions: Vec<DecisionLog>,
}
```

LLM examines:
- Task names and descriptions (semantic similarity)
- Event logs (actual work done vs. planned)
- Dependency relationships (are they still valid?)
- Parent-child alignment (does hierarchy make sense?)

#### Output Format

```bash
$ ie task done 44

✅ Task completed: #44 JWT implementation

💡 Structure Analysis Suggestion:
   Detected 2 orphaned tasks that may belong here:
   • #50 "Fix login bug" - relates to authentication
   • #51 "OAuth support" - extends auth system

   Reorganize?
     ie task update 50 --parent 42
     ie task update 51 --parent 42
```

**Key Design Principles**:
- ✅ **Suggestion, not automatic** - Agent/user decides
- ✅ **Context-aware** - Uses full task + event history
- ✅ **Actionable** - Provides exact commands to run
- ✅ **Non-blocking** - Doesn't interrupt workflow

---

## Use Case 2: Event-to-Task Description Synthesis

### Problem

Tasks evolve during execution:
- Initial description becomes stale
- Events capture actual decisions/blockers/pivots
- Final state differs from initial plan
- Future search/review needs updated context

**Example**:
```
Task #42: Implement JWT auth
Initial description: "Add JWT-based authentication"

Events accumulated:
  [decision] Chose HS256 over RS256 (simpler for MVP)
  [blocker] bcrypt incompatible with WASM, switched to argon2
  [milestone] Login endpoint working
  [note] Added refresh token rotation for security

Current description: Still just "Add JWT-based authentication" ❌
```

### Solution

**When marking task as done, LLM synthesizes events into updated description.**

#### Trigger Point

**Automatically on `ie task done`** (if LLM configured)

#### What LLM Receives

```rust
struct TaskCompletionContext {
    task: Task,
    original_description: String,
    all_events: Vec<Event>,  // chronological
    completion_criteria: Option<String>,
}
```

#### What LLM Generates

```markdown
## Goal
Implement JWT-based authentication for API endpoints

## Approach
- Chose HS256 algorithm (sufficient for single-server deployment)
- Used argon2 for password hashing (WASM-compatible)
- Implemented refresh token rotation for enhanced security

## Key Decisions
- Rejected RS256: Added complexity not justified for MVP
- Switched from bcrypt: WASM compatibility blocker

## Outcome
Working login/refresh endpoints with secure token management
```

#### Implementation Flow

**For AI-created tasks** (owner = "ai"):
```bash
$ ie task done 42

Completing task #42...
🤖 Synthesizing task summary from 12 events...

Updated task description:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## Goal
Implement JWT-based authentication for API endpoints

## Approach
- Chose HS256 (sufficient for MVP)
- Used argon2 (WASM-compatible)

## Key Decisions
- Rejected RS256: unnecessary complexity

## Outcome
Working login/refresh with token rotation
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Task description updated automatically
```

**For human-created tasks** (owner = "human" or custom):
```bash
$ ie task done 42

Completing task #42...
🤖 Synthesizing task summary from 12 events...

Suggested task description:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
[same content as above]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Apply this summary? [Y/n]
```

**Key Design Principles**:
- ✅ **Preserves original intent** - Goal stays consistent
- ✅ **Captures reality** - Approach reflects actual work
- ✅ **Extracts insights** - Decisions become searchable
- ✅ **Respects ownership** - AI-created = auto-update, human-created = requires approval
- ✅ **Optional** - User can skip synthesis for human tasks

---

## Technical Architecture

### LLM Prompts

#### Use Case 1: Structure Analysis Prompt

```
You are analyzing a task management tree for structural issues.

Current task just completed: {task.name}
Parent task: {parent.name}
Siblings: {siblings}
Recent decisions: {decision_logs}

Other root tasks in workspace:
{root_tasks}

Identify tasks that semantically belong under the completed task's parent but are currently orphaned or misplaced.

Output JSON:
{
  "suggestions": [
    {
      "task_id": 50,
      "reason": "Related to authentication, should be under #42",
      "command": "ie task update 50 --parent 42"
    }
  ]
}
```

#### Use Case 2: Event Synthesis Prompt

```
You are summarizing a completed task based on its execution history.

Task: {task.name}
Original description: {task.spec}

Events (chronological):
{events}

Synthesize a clear, structured description capturing:
1. Goal (what was the objective?)
2. Approach (how was it accomplished?)
3. Key Decisions (what choices were made and why?)
4. Outcome (what was delivered?)

Use markdown format. Be concise but preserve critical context.
```

### Configuration

```bash
# Enable LLM features
ie config set llm.endpoint "http://localhost:8080/v1/chat/completions"
ie config set llm.api_key "sk-xxx"
ie config set llm.model "gpt-4"

# Feature flags
ie config set llm.enable_structure_analysis "true"
ie config set llm.enable_task_synthesis "true"
```

### Owner Field Constraint

**Critical Design Rule**: Respect task ownership

```rust
// In done_task_by_id implementation
if task.owner == "ai" {
    // Auto-apply LLM synthesis
    task.spec = llm_synthesis;
} else {
    // Human-created task - require approval
    if !prompt_user_approval(&llm_synthesis)? {
        // User declined, keep original spec
        return Ok(result);
    }
    task.spec = llm_synthesis;
}
```

**Rationale**:
- AI agents can freely update their own tasks
- Human tasks represent human intent → require consent
- Prevents AI from overwriting human-crafted descriptions
- Aligns with existing `done_task_by_id` owner protection

**Error Cases**:
```rust
// Attempting to complete human task as AI caller
if task.owner == "human" && caller == "ai" {
    return Err(IntentError::HumanTaskCannotBeCompletedByAI {
        task_id: task.id,
        task_name: task.name,
    });
}
```

### Error Handling

**Graceful degradation**:
- If LLM unavailable → skip analysis/synthesis
- If LLM returns invalid JSON → log warning, continue
- If user disables → respect setting immediately

**No blocking**: LLM failure never prevents core operations.

---

## Implementation Priority

### Phase 1 (v0.13): Use Case 2 - Event Synthesis
**Why first**:
- Simpler trigger (single point: `ie task done`)
- Clear input/output
- Immediate value (better task descriptions)

### Phase 2 (v0.14): Use Case 1 - Structure Analysis
**Why second**:
- More complex (multiple trigger points)
- Requires graph analysis
- Higher sophistication needed

---

## Success Metrics

### Use Case 1
- % of reorganization suggestions accepted by users
- Reduction in "orphaned" tasks over time
- User feedback on suggestion relevance

### Use Case 2
- Task description quality (measured by search hit rate)
- Time saved on manual documentation
- Adoption rate (users who enable synthesis)

---

## Non-Goals

❌ **Automatic task creation** - LLM doesn't create tasks
❌ **Automatic task completion** - LLM doesn't mark tasks done
❌ **Replacing human judgment** - LLM suggests, user decides
❌ **Real-time assistance** - LLM runs at specific triggers only

---

## Summary

**LLM's Role**: Analytical assistant that maintains task clarity

**Core Principle**: **Suggest, don't automate**

**Value Proposition**:
- For AI Agents: Better context continuity across sessions
- For Humans: Less manual grooming, better searchability

This aligns with intent-engine's mission: **External memory for AI agents**.
