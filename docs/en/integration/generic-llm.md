# Generic LLM Tool Integration Guide

This guide explains how to integrate Intent-Engine into any AI tool, whether GPT, Claude, Gemini, or other LLMs.

---

## Core Principle

Intent-Engine interacts with AI tools via **CLI + JSON**:

1. AI tool calls `intent-engine` commands via `Bash`/`Shell` capability
2. Intent-Engine returns results in JSON format
3. AI parses JSON and continues working

**Key Advantages:**
- ✅ No need for specialized plugins or extensions
- ✅ Works with any AI tool that supports Shell command execution
- ✅ Complete feature coverage (same as MCP Server)

---

## Prerequisites

1. **Intent-Engine installed and in PATH**
   ```bash
   intent-engine --version
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
# Intent-Engine Integration

You have access to Intent-Engine, a strategic intent tracking system for human-AI collaboration.

## When to Use

Create a task when work requires:
- Multiple steps or sessions
- Extensive context/specifications
- Decision history tracking
- Hierarchical problem decomposition

## Core Commands

### Start Working
\`\`\`bash
intent-engine task start <ID> --with-events
# Returns: task details + event history + spec
\`\`\`

### Create Subtask
\`\`\`bash
intent-engine task spawn-subtask --name "Subtask name"
# Atomic: create + start + switch
\`\`\`

### Record Decision
\`\`\`bash
echo "Decision details..." | \
  intent-engine event add --task-id <ID> --type decision --data-stdin
\`\`\`

### Complete Task
\`\`\`bash
intent-engine task done <ID>
# Enforces: all subtasks must be done first
\`\`\`

### Generate Report
\`\`\`bash
intent-engine report --since 1d --summary-only
# Token-efficient summary
\`\`\`

## Key Principles

1. Always use `--with-events` when starting/switching tasks
2. Record all key decisions via `event add`
3. Use `spawn-subtask` when discovering sub-problems
4. Use `--summary-only` for reports (saves tokens)

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
echo "Refactor database query layer
- Unify query interface
- Add connection pool management
- Implement query caching
- Add slow query logging" | \
  intent-engine task add --name "Refactor database query layer" --spec-stdin

[Output]
{
  "id": 1,
  "name": "Refactor database query layer",
  "status": "todo",
  ...
}

AI: Task created (ID: 1). Let me start this task and view the context.

[Executes command]
intent-engine task start 1 --with-events

[AI continues working...]
```

---

## Best Practices

### 1. When to Create Tasks

**Recommended to create tasks:**
- ✅ Work expected to require multiple conversations
- ✅ Need to record "why we did this" decisions
- ✅ Complex tasks involving multiple related sub-problems

**Not recommended to create tasks:**
- ❌ One-off simple questions (e.g., "how to install Python")
- ❌ Pure information queries (e.g., "what is JWT")

### 2. How to Write Specifications (Spec)

A good specification should include:

```markdown
# Goal
[Briefly describe what to implement]

# Requirements
- [Specific requirement 1]
- [Specific requirement 2]
- ...

# Technical Constraints
- [Technology choices]
- [Architecture requirements]
- [Performance targets]

# References
- [Related documentation links]
```

**Example:**

```bash
echo "# Goal
Implement JWT-based user authentication system

# Requirements
- Support user registration and login
- Token validity: 7 days
- Support token refresh
- Password encryption using bcrypt

# Technical Constraints
- Use Rust + Actix-Web
- JWT library: jsonwebtoken
- Database: PostgreSQL

# References
- RFC 7519 (JWT)
- https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html" | \
  intent-engine task add --name "Implement JWT authentication" --spec-stdin
```

### 3. Event Recording Strategy

**When to record events:**

| Event Type | Use Case | Example |
|-----------|---------|---------|
| `decision` | Making key technical decisions | "Decided to use Redis for caching instead of Memcached" |
| `blocker` | Encountering blocking issues | "Waiting for API key approval" |
| `milestone` | Completing important phases | "Core logic complete, unit tests passing" |
| `discussion` | Recording discussion results | "After team discussion, decided to use microservice architecture" |
| `note` | General notes | "Discovered performance bottleneck in database queries" |

### 4. Task Hierarchy Design

**Recommended hierarchy:**

```
Root task (strategic goal)
├── Subtask 1 (tactical step)
│   ├── Subtask 1.1 (specific implementation)
│   └── Subtask 1.2
├── Subtask 2
│   ├── Subtask 2.1
│   │   └── Subtask 2.1.1 (recursive decomposition)
│   └── Subtask 2.2
└── Subtask 3
```

**Avoid excessive depth:**
- ✅ 1-3 levels: Recommended
- ⚠️ 4-5 levels: Acceptable
- ❌ 6+ levels: Over-complicated, consider redesigning

---

## AI Workflow Examples

### Scenario 1: Code Review Finding Multiple Issues

```bash
# 1. AI discovers 5 issues, batch create tasks
intent-engine task add --name "Fix null pointer exception"
intent-engine task add --name "Optimize database query"
intent-engine task add --name "Fix memory leak"
intent-engine task add --name "Update outdated dependencies"
intent-engine task add --name "Add error logging"

# 2. AI evaluates priority and complexity
intent-engine task update 1 --priority 10 --complexity 3  # Urgent and simple
intent-engine task update 2 --priority 8 --complexity 7   # Important but complex
intent-engine task update 3 --priority 10 --complexity 9  # Urgent and complex
intent-engine task update 4 --priority 5 --complexity 5   # Medium
intent-engine task update 5 --priority 3 --complexity 2   # Not urgent and simple

# 3. Smart task selection (by priority DESC, complexity ASC)
intent-engine task pick-next --max-count 3
# Will select: task 1 (P10/C3), task 3 (P10/C9), task 2 (P8/C7)

# 4. Process one by one
intent-engine task switch 1
# ... fix ...
echo "Cause: Did not check for null return value" | \
  intent-engine event add --task-id 1 --type note --data-stdin
intent-engine task done 1

# 5. Generate report
intent-engine report --since 1d --summary-only
```

### Scenario 2: Recursive Problem Decomposition

```bash
# 1. Start major task
echo "Implement complete payment system..." | \
  intent-engine task add --name "Implement payment system" --spec-stdin
intent-engine task start 1 --with-events

# 2. Discover sub-problem
intent-engine task spawn-subtask --name "Integrate Stripe API"

# 3. Discover even finer problem
intent-engine task spawn-subtask --name "Configure webhook callback"

# 4. Complete deepest task
echo "Webhook endpoint configured" | \
  intent-engine event add --task-id 3 --type milestone --data-stdin
intent-engine task done 3

# 5. Complete layer by layer
intent-engine task switch 2
intent-engine task done 2
intent-engine task switch 1
intent-engine task done 1
```

---

## Common Questions

### Q: What if AI forgets to use Intent-Engine?

**A**: Emphasize usage rules in System Prompt:

```markdown
IMPORTANT: For all complex, multi-step tasks, you MUST use Intent-Engine
to track strategic intent. Before starting any significant work, create
a task with `intent-engine task add`.
```

### Q: How to make AI automatically record decisions?

**A**: Add to System Prompt:

```markdown
Whenever you make a key technical decision, record it immediately:

echo "Your decision and reasoning..." | \
  intent-engine event add --task-id <current-task-id> --type decision --data-stdin
```

### Q: JSON output too long, affecting context?

**A**: Use `--summary-only` and `jq` filtering:

```bash
# Get summary only
intent-engine report --summary-only

# Extract only needed fields
intent-engine task get 1 | jq '{id, name, status, spec}'

# View only recent 5 events
intent-engine event list --task-id 1 --limit 5
```

### Q: How to share Intent-Engine data in a team?

**A**: SQLite database can be committed to Git:

```bash
# Ensure .intent-engine/ is not ignored in .gitignore
!.intent-engine/
!.intent-engine/project.db

# Commit database
git add .intent-engine/project.db
git commit -m "Update task database"
```

**Note**: Large teams may need centralized storage solution (planned for future support).

---

## Advanced Usage

### 1. Custom AI Prompt Templates

Create dedicated prompt templates for your AI tool:

```markdown
# Task: {{task_name}}

## Context
{{task_spec}}

## Recent Decisions
{{event_history}}

## Instructions
[Your specific instructions]

## Remember
- Record all key decisions
- Use spawn-subtask for sub-problems
- Switch tasks with `task switch`
- Complete with `task done` only when all subtasks are done
```

### 2. Integration into Automated Workflows

```bash
#!/bin/bash
# auto-task-report.sh

# Auto-generate daily work report
intent-engine report --since 1d --summary-only > /tmp/daily-report.json

# Send to AI to generate natural language summary
cat /tmp/daily-report.json | your-ai-cli summarize
```

### 3. Multi-project Management

```bash
# Project A
cd /path/to/project-a
intent-engine task add --name "Feature X"

# Project B
cd /path/to/project-b
intent-engine task add --name "Feature Y"

# Each project has independent .intent-engine/ database
```

---

## Next Steps

1. 📖 Read [AI Quick Guide](../guide/ai-quick-guide.md) for complete commands
2. 🚀 Refer to [QUICKSTART.en.md](../../../QUICKSTART.en.md) to experience core features
3. 💡 Learn [The Intent-Engine Way](../guide/the-intent-engine-way.md) to understand best practices

---

**Need Help?**

- [GitHub Issues](https://github.com/wayfind/intent-engine/issues)
- [Contributing Guide](../contributing/contributing.md)
