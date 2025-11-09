# Intent-Engine Quick Start Guide

**[中文](QUICKSTART.md) | English**

**Experience Intent-Engine's core features from scratch in 5 minutes.**

---

## Prerequisites

Required:
- Rust and Cargo ([Installation Guide](https://rustup.rs/))
- Or pre-compiled binary downloaded

---

## Step 1: Installation (1 minute)

```bash
# Method 1: Install using Cargo (Recommended)
cargo install intent-engine

# Method 2: Download pre-compiled binary
# Visit https://github.com/wayfind/intent-engine/releases
# Download the version for your platform

# Verify installation
intent-engine --version
```

> 💡 **Tip**: For detailed installation options, see [INSTALLATION.md](docs/en/guide/installation.md)

---

## Step 2: Create Your First Task (1 minute)

```bash
# Add a strategic task (with specifications)
echo "Implement JWT authentication
- Support token generation and validation
- Token validity: 7 days
- Support token refresh
- Use HS256 algorithm" | \
  intent-engine task add --name "Implement user authentication" --spec-stdin

# Example output:
# {
#   "id": 1,
#   "name": "Implement user authentication",
#   "status": "todo",
#   ...
# }
```

**What happened?**
- ✅ Intent-Engine automatically created `.intent-engine/project.db` in current directory (or parent)
- ✅ Task saved to SQLite database
- ✅ Specification (spec) fully recorded

---

## Step 3: Start Task and View Context (30 seconds)

```bash
# Start task and get complete context
intent-engine task start 1 --with-events

# Example output:
# {
#   "id": 1,
#   "name": "Implement user authentication",
#   "spec": "Implement JWT authentication\n- Support token generation and validation\n...",
#   "status": "doing",  # Status updated to doing
#   ...
# }
```

**What happened?**
- ✅ Task status changed from `todo` to `doing`
- ✅ Task set as "current task"
- ✅ Returns complete specification and event history (if any)

---

## Step 4: Discover Sub-problem During Work (1 minute)

```bash
# During implementation, discover need to configure JWT secret first
intent-engine task spawn-subtask --name "Configure JWT secret storage"

# Example output:
# {
#   "id": 2,
#   "parent_id": 1,  # Parent task auto-set
#   "name": "Configure JWT secret storage",
#   "status": "doing",  # Auto-started
#   ...
# }
```

**What happened?**
- ✅ Created subtask (parent_id = 1)
- ✅ Subtask automatically entered `doing` status
- ✅ Current task automatically switched to subtask

---

## Step 5: Record Key Decisions (30 seconds)

```bash
# Record your decision process (to current task)
echo "Decided to store JWT secret in environment variables
Reasons:
1. Avoid hardcoding secrets in code
2. Easy to use different secrets in different environments
3. Complies with 12-Factor App principles" | \
  intent-engine event add --type decision --data-stdin

# Example output:
# {
#   "id": 1,
#   "task_id": 2,
#   "log_type": "decision",
#   "discussion_data": "Decided to store JWT secret in environment variables\n...",
#   "timestamp": "2025-11-08T..."
# }
```

**What happened?**
- ✅ Decision recorded to event stream
- ✅ Can trace "why this decision was made" in the future
- ✅ AI can recover complete context via `--with-events`

---

## Step 6: Complete Subtask and Switch Back to Parent (30 seconds)

```bash
# Complete subtask
intent-engine task done

# Switch back to parent task
intent-engine task switch 1

# Output includes complete context of parent task
```

---

## Step 7: Complete Parent Task (30 seconds)

```bash
# Complete parent task
intent-engine task done

# If there are incomplete subtasks, system will error:
# Error: Cannot complete task 1: it has incomplete subtasks

# After all subtasks complete, can successfully complete parent task
```

---

## Step 8: Generate Work Report (30 seconds)

```bash
# Generate concise work summary (recommended, saves tokens)
intent-engine report --since 1d --summary-only

# Example output:
# {
#   "summary": {
#     "total_count": 2,
#     "todo_count": 0,
#     "doing_count": 0,
#     "done_count": 2
#   }
# }

# Generate detailed report
intent-engine report --since 1d
```

---

## 🎉 Congratulations!

You've completed Intent-Engine's core workflow:

1. ✅ Create strategic task (with specifications)
2. ✅ Start task and get context
3. ✅ Discover sub-problem and create subtask
4. ✅ Record key decisions
5. ✅ Complete tasks (enforced subtask completion check)
6. ✅ Generate work report

---

## Next Steps

### 🚀 Advanced Features

1. **Smart Task Selection**: Batch process multiple tasks
   ```bash
   # Create multiple tasks
   intent-engine task add --name "Task A"
   intent-engine task add --name "Task B"
   intent-engine task add --name "Task C"

   # Set priority and complexity
   intent-engine task update 1 --priority 10 --complexity 3
   intent-engine task update 2 --priority 8 --complexity 7
   intent-engine task update 3 --priority 5 --complexity 2

   # Smart selection (by priority DESC, complexity ASC)
   intent-engine task pick-next --max-count 3
   ```

2. **Full-text Search**: Quickly find tasks
   ```bash
   intent-engine report --filter-name "authentication" --summary-only
   intent-engine report --filter-spec "JWT" --summary-only
   ```

3. **Event Types**: Record different types of events
   - `decision` - Key decisions
   - `blocker` - Encountered obstacles
   - `milestone` - Milestones
   - `discussion` - Discussion records
   - `note` - General notes

### 📚 Deep Learning

- [**The Intent-Engine Way**](docs/en/guide/the-intent-engine-way.md) - Understand design philosophy and best practices
- [**AI Quick Guide**](docs/en/guide/ai-quick-guide.md) - AI client quick reference
- [**Complete Command Reference**](docs/en/guide/command-reference-full.md) - Detailed documentation for all commands

### 🔧 Integrate with AI Tools

- [**MCP Server**](docs/en/integration/mcp-server.md) - Integrate with Claude Code/Desktop
- [**Claude Skill**](.claude-code/intent-engine.skill.md) - Lightweight integration method

### 💻 Pre-contribution Setup

If you want to contribute code to Intent-Engine, please install git hooks first:

```bash
./scripts/setup-git-hooks.sh
```

This automatically formats code before each commit, preventing CI check failures. For more development tool commands, see [scripts/README.md](scripts/README.md).

---

## FAQ

**Q: What's the difference between Intent-Engine and regular todo tools?**

A: Intent-Engine focuses on the **strategic intent layer** (What + Why), not just the tactical execution layer (What). Each task includes complete specifications, decision history, and hierarchical relationships—it's AI's external long-term memory.

**Q: Why do we need `--with-events`?**

A: This returns the task's event history, helping AI (or humans) quickly recover context and understand "what decisions were made before."

**Q: When to use `spawn-subtask` vs `task add --parent`?**

A:
- `spawn-subtask`: When **working on a task** and discover a sub-problem, completes "create + start + switch" in one step
- `task add --parent`: Plan subtasks in advance, but don't start immediately

**Q: Where is data stored?**

A: `.intent-engine/project.db` (SQLite database), located in project root directory.

---

**Start using Intent-Engine now and make AI your strategic execution partner!** 🚀
