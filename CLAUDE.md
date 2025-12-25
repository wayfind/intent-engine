# Intent-Engine: AI 长期任务记忆系统

**Version**: 0.10
**Target**: Claude Code, Claude Desktop, and AI assistants

---

## ⚡ 快速决策：TodoWrite vs ie

**你已经有了 TodoWrite**（Claude Code 内置）。何时用 ie？

| 场景 | 用 TodoWrite | 用 ie |
|------|-------------|-------|
| 单次会话内的简单清单 | ✅ | |
| 跨 session 的项目工作 | | ✅ |
| 需要记录"为什么这么决定" | | ✅ |
| 复杂的多层级任务分解 | | ✅ |
| 需要后续回顾决策历史 | | ✅ |
| 临时性、不需要保留的任务 | ✅ | |

**简单规则**：
- **会丢了可惜** → 用 ie
- **用完即弃** → 用 TodoWrite

---

## 🚀 Session 开始标准动作

**每次新 session 开始时，先运行**：

```bash
ie status
```

这会告诉你：
- 当前聚焦的任务是什么（如果有）
- 有哪些子任务待完成
- 上次工作到哪里了
- 兄弟任务的进度

**无聚焦任务时**，`ie status` 会显示所有根任务，帮你选择从哪里开始。

---

## 🧠 任务管理决策树

当用户请求涉及任务时，按以下规则决策：

```
用户请求涉及任务？
    │
    ├─ 是否需要跨 session 保留？
    │   ├─ 否 → TodoWrite
    │   └─ 是 → ie ✓
    │
    ├─ 是否有多个子步骤（3+）？
    │   ├─ 1-2 步 → TodoWrite
    │   └─ 3+ 步 → ie（利用层级结构）✓
    │
    ├─ 是否需要记录"为什么这么做"？
    │   ├─ 否 → TodoWrite
    │   └─ 是 → ie（用 ie log 记录决策）✓
    │
    └─ 是否是已存在的 ie 项目？
        └─ 运行 ie status 检查
            ├─ 有进行中的任务 → 继续用 ie ✓
            └─ 无任务 → 根据上述规则决定
```

---

## 🔧 核心命令速查

| 命令 | 用途 | 示例 |
|------|------|------|
| `ie status [id]` | 查看任务上下文 | `ie status` 或 `ie status 42` |
| `ie plan` | 创建/更新/完成任务 | `echo '{"tasks":[...]}' \| ie plan` |
| `ie log <type> <msg>` | 记录决策/阻塞/里程碑 | `ie log decision "选择 JWT"` |
| `ie search <query>` | 搜索任务和事件 | `ie search "todo doing"` |

---

## 📖 Authoritative Specification

> **IMPORTANT**: This guide is a practical summary derived from the authoritative specification.
>
> **Single Source of Truth**: `docs/spec-03-interface-current.md`
>
> The spec-03-interface-current.md document is the **foundational blueprint** that defines:
> - ✅ All CLI command signatures and behaviors
> - ✅ JSON output formats and data structures
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
    │  Task 42   │  ← The Focused Task (doing + current)
    │  "Impl auth"│
    └────┬───┬───┘
         │   │
    ┌────▼┐ ┌▼────┐
    │T43  │ │T44  │  ← Subtasks (depth-first priority)
    │JWT  │ │OAuth│
    └─────┘ └─────┘
```

**Important**: The system supports **multiple 'doing' tasks** simultaneously for hierarchical workflows. However, only **one task is focused** (current_task_id) at any time. Tasks that are 'doing' but not current are effectively "paused/pending" until you switch back to them.

---

## 🛠️ CLI Commands (v0.10.0)

> **Simplified 6-command CLI** - All task operations go through `plan`

### Core Commands

| Command | Purpose | Example |
|---------|---------|---------|
| `ie plan` | Create/update tasks (from stdin JSON) | `echo '{"tasks":[...]}' \| ie plan` |
| `ie log <type> <message>` | Record events | `ie log decision "Chose JWT"` |
| `ie search <query>` | Search tasks and events | `ie search "todo doing"` |
| `ie init` | Initialize project | `ie init` |
| `ie dashboard <cmd>` | Dashboard management | `ie dashboard start` |
| `ie doctor` | Check system health | `ie doctor` |

### Plan Command - The Universal Tool

`ie plan` handles ALL task operations through JSON:

```bash
# Create tasks
echo '{"tasks":[{"name":"Implement auth","status":"doing"}]}' | ie plan

# Update task status
echo '{"tasks":[{"name":"Implement auth","status":"done"}]}' | ie plan

# Create hierarchical tasks
echo '{"tasks":[{
  "name":"Parent task",
  "status":"doing",
  "children":[
    {"name":"Subtask 1","status":"todo"},
    {"name":"Subtask 2","status":"todo"}
  ]
}]}' | ie plan
```

### Log Command - Event Recording

```bash
ie log decision "Chose HS256 for JWT signing"
ie log blocker "API rate limit hit"
ie log milestone "MVP feature complete"
ie log note "Consider caching optimization"
ie log decision "message" --task 42  # Target specific task
```

### Search Command - Smart Query

```bash
ie search "todo doing"           # Status filter (unfinished tasks)
ie search "JWT authentication"   # FTS5 full-text search
ie search "API AND client"       # Boolean operators
```

---

## 🎨 Typical Usage Patterns

### Pattern 1: Starting Fresh
```
User: "Help me implement user authentication"

You:
1. Create task with ie plan
2. Search for context: ie search "authentication"
3. Update status to 'doing': ie plan with status update
4. Begin work and record decisions with ie log
```

### Pattern 2: Breaking Down Work
```
User: "Let's add authentication"

You:
1. Create parent task with subtasks using ie plan:
   echo '{"tasks":[{
     "name":"Implement authentication",
     "status":"doing",
     "children":[
       {"name":"Design JWT schema","status":"todo"},
       {"name":"Implement token validation","status":"todo"}
     ]
   }]}' | ie plan
2. Update subtask status as you work:
   echo '{"tasks":[{"name":"Design JWT schema","status":"doing"}]}' | ie plan
3. Complete subtask:
   echo '{"tasks":[{"name":"Design JWT schema","status":"done"}]}' | ie plan
```

### Pattern 3: Recording Decisions
```
While implementing JWT:

You: "I chose HS256 algorithm because..."
     ie log decision "Chose HS256 for performance and simplicity"
```

### Pattern 4: Resuming Work
```
User: "Let's continue with authentication"

You:
1. ie search "todo doing"       # Check unfinished tasks
2. ie search "authentication"   # Find specific tasks
3. Update status to continue:
   echo '{"tasks":[{"name":"Implement authentication","status":"doing"}]}' | ie plan
4. Continue from where you left off
```

### Pattern 5: Switching Context
```
User: "Let's pause auth and fix that bug"

You:
1. ie log note "Pausing auth to handle bug #123"
2. Create/update bug fix task:
   echo '{"tasks":[{"name":"Fix bug #123","status":"doing"}]}' | ie plan
3. Fix the bug
4. Mark done and return:
   echo '{"tasks":[
     {"name":"Fix bug #123","status":"done"},
     {"name":"Implement authentication","status":"doing"}
   ]}' | ie plan
```

### Pattern 6: Working with Dependencies
```
User: "Implement the API client, but it depends on authentication"

You:
1. Create both tasks with dependency:
   echo '{"tasks":[
     {"name":"Implement authentication","status":"doing"},
     {"name":"Implement API client","status":"todo","depends_on":["Implement authentication"]}
   ]}' | ie plan
2. Complete auth first, then API client becomes unblocked
```

### Pattern 7: Smart Search
```
User: "What decisions did we make on authentication?"

You:
1. ie search "authentication decision"  # FTS5 search
2. Review and summarize the decisions
```

---

## 💡 Best Practices

### 1. Use Status-Based Workflow
```
❌ DON'T: Forget to update status
✅ DO:    echo '{"tasks":[{"name":"Task","status":"doing"}]}' | ie plan
```

### 2. Use Hierarchical Decomposition
```
❌ DON'T: Flat list of 10 implementation steps
✅ DO:    Parent task with 3-4 logical subtasks
```

### 3. Record Important Decisions
```
❌ DON'T: Just implement without context
✅ DO:    ie log decision "Chose X because..."
```

### 4. Use Search for Context
```
❌ DON'T: Start without checking history
✅ DO:    ie search "todo doing" before starting
```

### 5. Keep Tasks Updated
```
❌ DON'T: Forget to mark tasks done
✅ DO:    Update status promptly via ie plan
```

---

## ⚠️ Common Mistakes

### Mistake 1: Forgetting to update status
```
❌ Work on task without updating status

✅ echo '{"tasks":[{"name":"My Task","status":"doing"}]}' | ie plan
   # ... do work ...
   echo '{"tasks":[{"name":"My Task","status":"done"}]}' | ie plan
```

### Mistake 2: Using search incorrectly
```
❌ ie search "status:doing"  # WRONG - not a filter syntax

✅ ie search "todo doing"    # Status keywords only → filter mode
✅ ie search "JWT auth"      # Contains non-status words → FTS5 search
```

### Mistake 3: Creating duplicate tasks
```
❌ Run same ie plan twice → creates duplicates? NO!

✅ ie plan is idempotent - same name = update, not create
```

### Mistake 4: Completing parent with incomplete children
```
❌ Mark parent done while children are still todo

✅ Complete all children first, then parent:
   echo '{"tasks":[
     {"name":"Child 1","status":"done"},
     {"name":"Child 2","status":"done"},
     {"name":"Parent","status":"done"}
   ]}' | ie plan
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
- "Help me implement X" → Create task via `ie plan`, track work
- "What's next?" → Use `ie search "todo doing"`
- "Why did we...?" → Use `ie search` for events
- "Continue authentication" → Update status via `ie plan`

### Task Lifecycle

```
User Request
    │
    ▼
ie plan (create) ──────────┐
    │                      │ (strategic planning)
    ▼                      │
ie plan (status:doing) ────┤
    │                      │ (active work)
    ├── ie log             │
    ├── ie plan (children) │
    │                      │
    ▼                      │
ie plan (status:done) ─────┘
```

---

## 🧠 Mental Model

Think of Intent-Engine as:

1. **Your Notebook** - Persistent task list across sessions
2. **Your Focus Ring** - One task at a time (current_task_id)
3. **Your Memory** - Decision history in events (ie log)
4. **Your Search** - Find anything with ie search
5. **Your Tree** - Hierarchical problem breakdown

---

## 📚 Key References

- **Interface Spec** (authoritative): `docs/spec-03-interface-current.md`
- **AI Agent Guide** (technical details): `AGENT.md`
- **Plan Command Guide**: `ie plan --help`

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

## 🏗️ Architecture (v0.10.0+)

### Simplified Communication Model

**Previous Architecture (v0.9.0)**:
```
┌─────────────┐     ┌────────────┐     ┌───────────┐
│ Claude Code │────▶│ MCP Server │◀───▶│ Dashboard │◀───▶│ Frontend │
│   Instance  │     │ (per proj) │     │ (central) │     │          │
└─────────────┘     └────────────┘     └───────────┘     └──────────┘
                    Persistent          Heartbeat          WebSocket
                    Connection          Mechanism          Connection
```

**Key Issues**:
- ❌ Each project required separate MCP server process
- ❌ Persistent bidirectional connections (complexity)
- ❌ Heartbeat mechanism needed (overhead)
- ❌ Projects had "online/offline" states
- ❌ Connection failures caused data sync issues

---

**Current Architecture (v0.10.0+)**:
```
┌─────────────┐
│ Claude Code │ via ie CLI
└──────┬──────┘
       │
       ▼ (write)
┌──────────────────┐
│ Local SQLite DB  │
│ (project-local)  │
└──────┬───────────┘
       │
       ▼ (single notification)
┌──────────────────────────┐
│   Global Dashboard       │
│   (one instance)         │
└──────┬───────────────────┘
       │
       ▼ (direct read/write)
┌─────────────────────────────────┐
│ All Project SQLite DBs          │
│ (/project-1/tasks.db)            │
│ (/project-2/tasks.db)            │
│ (/project-N/tasks.db)            │
└──────┬──────────────────────────┘
       │
       ▼ (query)
┌──────────────┐
│   Frontend   │
│   (Vue SPA)  │
└──────────────┘
```

**Key Improvements**:
- ✅ No MCP servers needed
- ✅ No persistent connections
- ✅ No heartbeat overhead
- ✅ No "online/offline" states
- ✅ All CLI operations work offline
- ✅ Dashboard can directly create/modify tasks in any project

---

### Dashboard's New Role

**Previous Role** (v0.9.0):
- Central server receiving data from multiple MCP servers
- Maintained WebSocket connections with frontend
- Relayed commands between AI agents and frontend
- Required projects to be "online" to function

**Current Role** (v0.10.0+):
1. **Passive Observer**
   - Receives unidirectional notifications from CLI operations
   - No active connections needed
   - Lightweight, event-driven updates

2. **Direct Database Accessor**
   - Has direct read/write access to all project SQLite databases
   - Can query any project's tasks, events, workspace state
   - No intermediary layer

3. **Human Task Creation Interface**
   - Humans can create/modify tasks directly via Dashboard UI
   - Dashboard writes directly to project databases
   - AI picks up human-created tasks on next CLI operation

4. **Multi-Project Visualizer**
   - Single dashboard instance monitors all projects
   - Real-time view across entire workspace
   - No per-project server setup needed

---

### Communication Flow

**AI Agent Workflow**:
```
1. AI executes `ie plan` or `ie add`
2. CLI writes to local SQLite database
3. CLI sends single notification to global dashboard (UDP/HTTP)
4. Dashboard updates frontend views
5. No acknowledgment needed (fire-and-forget)
```

**Human Workflow**:
```
1. Human opens Dashboard UI
2. Dashboard queries all project databases directly
3. Human creates/modifies tasks in UI
4. Dashboard writes directly to project SQLite DB
5. AI picks up changes on next CLI read operation
```

**Key Characteristics**:
- **Offline-First**: CLI operations never blocked by network
- **Eventually Consistent**: Dashboard updates async
- **Fault Tolerant**: Lost notifications don't affect data integrity
- **Simple**: Unidirectional data flow

---

### Migration Notes

If migrating from v0.9.0:
1. **Remove MCP Configuration**
   - No need for `mcp-server.json`
   - No MCP server processes to manage

2. **Start Global Dashboard** (optional)
   ```bash
   ie dashboard start
   # Monitors all projects automatically
   ```

3. **All CLI Commands Work Offline**
   - `ie plan`, `ie add`, `ie start`, etc. always work
   - No connection state to worry about

4. **Dashboard is Optional**
   - CLI works independently
   - Dashboard provides visualization only

For detailed migration guide, see [MIGRATION_v0.10.0.md](MIGRATION_v0.10.0.md)

---

*End of CLAUDE.md*
- 把前端启动在1393端口、后端启动在3000端口的开发模式，及其执行命令的细节，记忆下来，每次我说开启本地开发环境，指的就是这两个端口配合。