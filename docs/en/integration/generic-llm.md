# Generic LLM Tool Integration Guide

This guide explains how to integrate Intent-Engine into any AI tool, whether GPT, Claude, Gemini, or other LLMs.

---

## Core Principle

Intent-Engine interacts with AI tools via **CLI + JSON**:

1. AI tool calls `ie` commands via `Bash`/`Shell` capability
2. Intent-Engine returns results in structured format
3. AI parses output and continues working

**Key Advantages:**
- No need for specialized plugins or extensions
- Works with any AI tool that supports Shell command execution
- Complete feature coverage through 4 core commands

---

## Prerequisites

1. **Intent-Engine installed and in PATH**
   ```bash
   ie --version
   ```

2. **AI tool supports executing Shell commands**
   - GPT: Code Interpreter / Advanced Data Analysis
   - Claude: Bash tool (via Anthropic API)
   - Gemini: Code execution capability
   - Others: Any environment with Shell access

---

## Integration Steps

### Step 1: Prepare System Prompt

Add to your AI tool's System Prompt or Custom Instructions:

```markdown
# Intent-Engine Integration (v0.10.0)

You have access to Intent-Engine, a strategic intent tracking system for human-AI collaboration.

## When to Use

Create a task when work requires:
- Multiple steps or sessions
- Extensive context/specifications
- Decision history tracking
- Hierarchical problem decomposition

## Core Commands

### Restore Context (Always First)
\`\`\`bash
ie status              # What am I working on?
ie status 42           # View specific task
\`\`\`

### Create/Update/Complete Tasks
\`\`\`bash
# All task operations through ie plan with JSON stdin
echo '{"tasks":[{
  "name": "Task name",
  "status": "doing",
  "spec": "Goal and approach description"
}]}' | ie plan
\`\`\`

### Record Events
\`\`\`bash
ie log decision "Chose X because..."
ie log blocker "Stuck on..."
ie log milestone "Completed..."
ie log note "Observation..."
\`\`\`

### Search History
\`\`\`bash
ie search "todo doing"    # Unfinished tasks
ie search "keyword"       # Full-text search
\`\`\`

## Key Rules

1. Run `ie status` at session start (amnesia recovery)
2. `status:doing` requires `spec` (goal + approach)
3. Complete all children before marking parent `done`
4. Same task name = update (idempotent)

Full guide: docs/en/guide/ai-quick-guide.md
```

### Step 2: Activate in Conversation

When you need to use Intent-Engine, explicitly tell the AI:

```
Please use Intent-Engine to track this task: implement user authentication system
```

Or:

```
Let's track this work with Intent-Engine. Please create a task for
implementing the user authentication system.
```

### Step 3: Verify Integration

Test if AI can correctly use Intent-Engine:

**Test conversation example:**

```
You: I need to refactor the database query layer, please track this task with Intent-Engine.

AI: I'll create an Intent-Engine task to track this refactoring.

[Executes command]
echo '{"tasks":[{
  "name": "Refactor database query layer",
  "status": "doing",
  "spec": "## Goal\nUnify and optimize database queries\n\n## Approach\n- Unify query interface\n- Add connection pool management\n- Implement query caching\n- Add slow query logging"
}]}' | ie plan

[Output]
✓ Plan executed successfully
Created: 1 tasks
Task ID mapping:
  Refactor database query layer → #1

AI: Task created (ID: 1). Let me check the current context.

[Executes command]
ie status

[AI continues working...]
```

---

## Best Practices

### 1. When to Create Tasks

**Recommended to create tasks:**
- Work expected to require multiple conversations
- Need to record "why we did this" decisions
- Complex tasks involving multiple related sub-problems

**Not recommended to create tasks:**
- One-off simple questions (e.g., "how to install Python")
- Pure information queries (e.g., "what is JWT")

### 2. How to Write Specifications (Spec)

A good specification should include:

```markdown
## Goal
[Briefly describe what to implement]

## Approach
- [Step or technique 1]
- [Step or technique 2]
- ...

## Constraints
- [Technology choices]
- [Architecture requirements]
- [Performance targets]
```

**Example:**

```bash
echo '{"tasks":[{
  "name": "Implement JWT authentication",
  "status": "doing",
  "spec": "## Goal\nJWT-based user authentication system\n\n## Approach\n- Support user registration and login\n- Token validity: 7 days\n- Support token refresh\n- Password encryption using bcrypt\n\n## Constraints\n- Use Rust + Actix-Web\n- JWT library: jsonwebtoken\n- Database: PostgreSQL"
}]}' | ie plan
```

### 3. Event Recording Strategy

**When to record events:**

| Event Type | Use Case | Example |
|-----------|---------|---------|
| `decision` | Making key technical decisions | "Decided to use Redis for caching instead of Memcached" |
| `blocker` | Encountering blocking issues | "Waiting for API key approval" |
| `milestone` | Completing important phases | "Core logic complete, unit tests passing" |
| `note` | General notes | "Discovered performance bottleneck in database queries" |

### 4. Task Hierarchy Design

**Recommended hierarchy:**

```
Root task (strategic goal)
├── Subtask 1 (tactical step)
│   ├── Subtask 1.1 (specific implementation)
│   └── Subtask 1.2
├── Subtask 2
└── Subtask 3
```

**Use `children` for nested structure:**

```bash
echo '{"tasks":[{
  "name": "Implement payment system",
  "status": "doing",
  "spec": "Complete payment integration",
  "children": [
    {"name": "Integrate Stripe API", "status": "todo"},
    {"name": "Handle webhooks", "status": "todo"},
    {"name": "Add payment UI", "status": "todo"}
  ]
}]}' | ie plan
```

---

## AI Workflow Examples

### Scenario 1: Breaking Down Complex Work

```bash
# 1. AI creates main task with subtasks
echo '{"tasks":[{
  "name": "Fix code review issues",
  "status": "doing",
  "spec": "Address all issues from code review",
  "children": [
    {"name": "Fix null pointer exception", "status": "todo"},
    {"name": "Optimize database query", "status": "todo"},
    {"name": "Fix memory leak", "status": "todo"},
    {"name": "Update outdated dependencies", "status": "todo"},
    {"name": "Add error logging", "status": "todo"}
  ]
}]}' | ie plan

# 2. Start working on first issue
echo '{"tasks":[{
  "name": "Fix null pointer exception",
  "status": "doing",
  "spec": "Check for null return values"
}]}' | ie plan

# 3. Record finding
ie log note "Cause: Did not check for null return value from API"

# 4. Complete subtask
echo '{"tasks":[{"name": "Fix null pointer exception", "status": "done"}]}' | ie plan

# 5. Continue with next...
```

### Scenario 2: Recursive Problem Decomposition

```bash
# 1. Start major task
echo '{"tasks":[{
  "name": "Implement payment system",
  "status": "doing",
  "spec": "Complete Stripe payment integration"
}]}' | ie plan

# 2. Discover sub-problem, add subtask
echo '{"tasks":[{
  "name": "Configure webhook callback",
  "status": "doing",
  "spec": "Set up Stripe webhook endpoint"
}]}' | ie plan

# 3. Complete subtask
ie log milestone "Webhook endpoint configured and tested"
echo '{"tasks":[{"name": "Configure webhook callback", "status": "done"}]}' | ie plan

# 4. Complete parent when all children done
echo '{"tasks":[{"name": "Implement payment system", "status": "done"}]}' | ie plan
```

---

## Common Questions

### Q: What if AI forgets to use Intent-Engine?

**A**: Emphasize usage rules in System Prompt:

```markdown
IMPORTANT: For all complex, multi-step tasks, you MUST use Intent-Engine
to track strategic intent. Run `ie status` at session start, use `ie plan`
for task operations.
```

### Q: How to make AI automatically record decisions?

**A**: Add to System Prompt:

```markdown
Whenever you make a key technical decision, record it immediately:
ie log decision "Your decision and reasoning..."
```

### Q: How to handle long JSON output?

**A**: Focus on `ie status` for context and `ie search` for finding specific tasks. The output is designed to be concise.

### Q: How to create independent tasks?

**A**: Use `parent_id: null` to avoid auto-parenting:

```bash
echo '{"tasks":[{
  "name": "Unrelated bug fix",
  "status": "todo",
  "parent_id": null
}]}' | ie plan
```

---

## Next Steps

1. Read [AI Quick Guide](../guide/ai-quick-guide.md) for complete command reference
2. Try [Quick Start](../guide/quickstart.md) to experience core features
3. Learn from [CLAUDE.md](../../../CLAUDE.md) for design philosophy

---

**Need Help?**

- [GitHub Issues](https://github.com/wayfind/intent-engine/issues)
