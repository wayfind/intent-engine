/// Guide command handler - AI learning content
///
/// This module provides rich contextual guides optimized for AI assistants
/// to learn Intent-Engine workflows and best practices.

use crate::cli::GuideCommands;
use crate::error::Result;

const AI_GUIDE: &str = include_str!("../../system-prompt.txt");

const PLAN_GUIDE: &str = r#"# Plan Command - Complete Guide

## 🎯 核心原理

`plan` 是唯一用于任务管理的命令，采用声明式批量操作：

```
JSON Plan → ie plan → 任务创建/更新 + 自动状态同步
```

**关键特性**：
- ✅ **幂等性**: 相同输入多次运行结果一致（按 name 更新）
- ✅ **批量操作**: 一次创建/更新多个任务
- ✅ **层级结构**: 支持父子任务嵌套（children）
- ✅ **依赖管理**: 自动处理任务依赖（depends_on）
- ✅ **状态控制**: 设置 todo/doing/done 状态
- ✅ **自动聚焦**: 可指定一个 doing 任务自动聚焦

---

## 📝 基础用法 (类似 TodoWriter)

### 最简示例 - 创建单个任务

```bash
echo '{"tasks":[{"name":"实现用户登录"}]}' | ie plan
```

### TodoWriter 风格 - 状态管理

**类似 TodoWriter 的用法**：
```bash
# 等同于 TodoWriter 的 todos 列表
echo '{
  "tasks": [
    {"name": "设计数据库schema", "status": "done"},
    {"name": "实现API接口", "status": "doing", "active_form": "正在实现API接口"},
    {"name": "编写单元测试", "status": "todo"},
    {"name": "部署到生产环境", "status": "todo"}
  ]
}' | ie plan
```

**关键差异**：
| TodoWriter | Intent-Engine |
|-----------|---------------|
| `status: "completed"` | `status: "done"` |
| `status: "in_progress"` | `status: "doing"` + `active_form` |
| `status: "pending"` | `status: "todo"` |

---

## 🌳 层级结构 (TodoWriter 无此功能)

### 父子任务 - 使用 children

```bash
echo '{
  "tasks": [{
    "name": "用户认证功能",
    "status": "doing",
    "children": [
      {"name": "JWT token生成", "status": "done"},
      {"name": "登录API接口", "status": "doing"},
      {"name": "token验证中间件", "status": "todo"}
    ]
  }]
}' | ie plan
```

**层级优势**：
- 复杂任务自动分解
- 子任务完成后才能完成父任务
- Dashboard UI 显示树状结构

---

## 🔗 依赖管理 (TodoWriter 无此功能)

### 使用 depends_on 指定依赖

```bash
echo '{
  "tasks": [
    {"name": "设计API接口"},
    {"name": "实现后端逻辑", "depends_on": ["设计API接口"]},
    {"name": "开发前端页面", "depends_on": ["设计API接口"]},
    {"name": "集成测试", "depends_on": ["实现后端逻辑", "开发前端页面"]}
  ]
}' | ie plan
```

**依赖效果**：
- 自动检测循环依赖
- 被依赖的任务必须先完成
- `ie next` 会推荐无依赖的任务

---

## 🎯 自动聚焦 (关键特性)

### 指定 doing 任务聚焦

```bash
echo '{
  "tasks": [
    {"name": "任务A", "status": "todo"},
    {"name": "任务B", "status": "doing"},  ← 自动聚焦到这个
    {"name": "任务C", "status": "todo"}
  ]
}' | ie plan
```

**重要**: 一次只能有一个 `doing` 任务（单一聚焦原则）

---

## 🔄 幂等更新 (强大特性)

### 按 name 更新已有任务

```bash
# 第一次运行 - 创建任务
echo '{"tasks":[{"name":"实现登录","status":"todo"}]}' | ie plan

# 第二次运行 - 更新状态
echo '{"tasks":[{"name":"实现登录","status":"doing"}]}' | ie plan

# 第三次运行 - 标记完成
echo '{"tasks":[{"name":"实现登录","status":"done"}]}' | ie plan
```

**用途**：
- 进度同步（从其他系统导入）
- 批量更新状态
- 安全重试（不会重复创建）

---

## 📊 常见模式

### 模式1: Sprint 规划

```bash
echo '{
  "tasks": [{
    "name": "Sprint 10: 用户系统重构",
    "priority": "high",
    "spec": "2025年1月15-28日",
    "children": [
      {
        "name": "用户注册流程",
        "priority": "high",
        "children": [
          {"name": "邮箱验证"},
          {"name": "密码哈希"},
          {"name": "欢迎邮件"}
        ]
      },
      {
        "name": "用户资料页面",
        "priority": "medium",
        "depends_on": ["用户注册流程"]
      }
    ]
  }]
}' | ie plan
```

### 模式2: 快速记录当前进度

```bash
# 批量创建待办 + 标记当前进度
echo '{
  "tasks": [
    {"name": "实现缓存层", "status": "done"},
    {"name": "优化数据库查询", "status": "doing", "active_form": "正在优化查询"},
    {"name": "添加性能监控", "status": "todo"},
    {"name": "编写压测脚本", "status": "todo"}
  ]
}' | ie plan
```

### 模式3: Bug 修复追踪

```bash
echo '{
  "tasks": [{
    "name": "修复生产环境登录超时",
    "priority": "critical",
    "status": "doing",
    "children": [
      {"name": "复现问题", "status": "done"},
      {"name": "定位根因", "status": "doing"},
      {"name": "编写修复补丁", "status": "todo"},
      {"name": "回归测试", "status": "todo"}
    ]
  }]
}' | ie plan
```

### 模式4: 技术债务管理

```bash
echo '{
  "tasks": [{
    "name": "Q1 技术债务清理",
    "priority": "medium",
    "children": [
      {"name": "重构认证模块", "priority": "high"},
      {"name": "升级依赖库", "priority": "medium"},
      {"name": "优化测试覆盖率", "priority": "low"}
    ]
  }]
}' | ie plan
```

---

## ⚡ 性能优化建议

### ✅ 推荐: 批量操作
```bash
# 一次创建10个任务
echo '{"tasks":[...10 tasks...]}' | ie plan
```

### ❌ 避免: 逐个调用
```bash
# 不推荐 - 10次数据库操作
for task in ...; do
  echo "{\"tasks\":[{\"name\":\"$task\"}]}" | ie plan
done
```

---

## 🔍 输出格式

### JSON 格式 (--format json)
```bash
echo '{"tasks":[{"name":"test"}]}' | ie plan --format json
```

输出结构：
```json
{
  "success": true,
  "created_count": 1,
  "updated_count": 0,
  "dependency_count": 0,
  "task_id_map": {
    "test": 42
  },
  "focused_task": {
    "task": {...},
    "events_summary": {...}
  }
}
```

### 文本格式 (默认)
```
✓ Plan executed successfully

Created: 3 tasks
Updated: 1 tasks
Dependencies: 2

Task ID mapping:
  实现登录 → #42
  设计数据库 → #43
  编写测试 → #44

✓ Current focus:
  ID: 42
  Name: 实现登录
  Status: doing
```

---

## 🚫 常见错误

### 错误1: 多个 doing 任务
```bash
# ❌ 错误 - 一次只能有一个 doing
echo '{
  "tasks": [
    {"name": "A", "status": "doing"},
    {"name": "B", "status": "doing"}  ← 会报错
  ]
}' | ie plan
```

### 错误2: 循环依赖
```bash
# ❌ 错误 - A依赖B，B依赖A
echo '{
  "tasks": [
    {"name": "A", "depends_on": ["B"]},
    {"name": "B", "depends_on": ["A"]}  ← 会报错
  ]
}' | ie plan
```

### 错误3: 无效的 status
```bash
# ❌ 错误 - 只能是 todo/doing/done
echo '{"tasks":[{"name":"test","status":"pending"}]}' | ie plan
```

---

## 🎓 最佳实践

1. **保持简单**: 从平面列表开始，需要时再加层级
2. **批量操作**: 尽量一次性创建相关任务
3. **明确命名**: 使用清晰的任务名（如 "实现JWT认证" 而非 "做认证"）
4. **合理层级**: 2-3层足够，避免过深嵌套
5. **状态同步**: 使用 plan 更新进度而非手动修改数据库

---

## 📚 配合其他命令

```bash
# 1. 创建任务结构
echo '{...}' | ie plan

# 2. 搜索任务
ie search "登录"

# 3. 记录决策
ie log decision "选择JWT而非Session"

# 4. 查看进度
ie guide ai  # AI集成指南会显示当前状态
```

---

## 💡 从 TodoWriter 迁移

**TodoWriter 模式**：
```typescript
TodoWrite({
  todos: [
    {content: "Task 1", status: "in_progress", activeForm: "Working on Task 1"},
    {content: "Task 2", status: "pending"}
  ]
});
```

**Intent-Engine 等价**：
```bash
echo '{
  "tasks": [
    {"name": "Task 1", "status": "doing", "active_form": "Working on Task 1"},
    {"name": "Task 2", "status": "todo"}
  ]
}' | ie plan
```

**关键改进**：
- ✅ 持久化存储（TodoWriter 仅内存）
- ✅ 层级结构（TodoWriter 仅平面列表）
- ✅ 依赖管理（TodoWriter 无）
- ✅ Dashboard 可视化（TodoWriter 无）

---

## 🔗 相关命令

- `ie log` - 记录事件（决策、阻塞、里程碑）
- `ie search` - 搜索任务和事件
- `ie guide ai` - AI集成完整指南
- `ie guide todo-writer` - TodoWriter 详细迁移指南

---

**核心理念**: Plan 是声明式的，告诉系统"我要什么"而非"怎么做"。
"#;

const TODOWRITER_GUIDE: &str = r#"# TodoWriter → Intent-Engine Migration Guide

Intent-Engine provides a **superior replacement** for TodoWriter with enhanced features:

## Key Advantages

### 1. Batch Task Creation
**TodoWriter**:
```
TodoWrite with individual task calls
```

**Intent-Engine**:
```bash
echo '{"tasks": [
  {"name": "Parent", "children": [
    {"name": "Child 1"},
    {"name": "Child 2"}
  ]}
]}' | ie plan
```

### 2. Status Management
- `todo`: Not started
- `doing`: In progress (with `active_form` for UI)
- `done`: Completed

### 3. Real-Time Dashboard Sync
- CLI operations instantly update Dashboard UI
- WebSocket-based live updates
- No polling required

### 4. Event History
Track **why** not just **what**:
```bash
ie log decision "Chose JWT because stateless"
ie log blocker "API rate limit blocking feature"
ie log milestone "MVP complete, ready for testing"
```

### 5. Hierarchical Tasks
```json
{
  "tasks": [{
    "name": "Implement Auth",
    "status": "doing",
    "children": [
      {"name": "JWT Setup", "status": "done"},
      {"name": "OAuth Integration", "status": "todo"}
    ]
  }]
}
```

### 6. Dependencies
```json
{
  "tasks": [
    {"name": "Build API"},
    {"name": "Build UI", "depends_on": ["Build API"]}
  ]
}
```

## Migration Checklist

✅ Replace `TodoWrite` calls with `ie plan`
✅ Use `status` field instead of separate lists
✅ Add `active_form` for better UX
✅ Track decisions with `ie log decision`
✅ Use hierarchical structure for complex tasks
✅ Define dependencies when needed

## Example: Full Migration

**Before (TodoWriter)**:
```typescript
TodoWrite({
  todos: [
    {content: "Task 1", status: "pending"},
    {content: "Task 2", status: "in_progress"},
    {content: "Task 3", status: "completed"}
  ]
});
```

**After (Intent-Engine)**:
```bash
echo '{
  "tasks": [
    {"name": "Task 1", "status": "todo"},
    {"name": "Task 2", "status": "doing", "active_form": "Working on Task 2"},
    {"name": "Task 3", "status": "done"}
  ]
}' | ie plan
```

## Pro Tips

1. **Idempotent**: Safe to run `ie plan` multiple times (updates by name)
2. **Focus**: Use `ie start <id>` to focus on one task
3. **Context**: Use `ie current` to see current focus
4. **Recovery**: Use `ie search` to find tasks after breaks
5. **History**: Use `ie event list` to review decisions

## See Also
- `ie guide ai` - AI integration patterns
- `ie guide workflow` - Core workflows
- `ie guide patterns` - Usage examples
"#;

const WORKFLOW_GUIDE: &str = r#"# Intent-Engine Core Workflows

## 1. Focus-Driven Single-Task Execution

**Principle**: Work on ONE task at a time

```bash
# Start a task (sets focus)
ie start 42

# Check current focus
ie current

# Complete current task
ie done

# Get recommendation for next task
ie next
```

**Why**: Prevents context switching, improves completion rate

## 2. Hierarchical Task Breakdown

**Pattern**: Break complex tasks into manageable subtasks

```bash
# Create parent task
ie add "Implement Authentication"
# Returns: Task #42

# Start working on it
ie start 42

# Break it down (creates subtask and focuses on it)
ie add "Design JWT schema" --parent 42
# Returns: Task #43

ie start 43
# Work on subtask...
ie done

# Get next subtask
ie next
# Recommends: Task #44 or back to #42
```

**Why**: Manageable chunks, clear progress tracking

## 3. Context Recovery After Breaks

**Scenario**: Resuming work after hours/days

```bash
# Step 1: Search for your work
ie search "authentication"

# Step 2: Start the task (with history)
ie start 42

# Step 3: Review decision history
ie event list --task-id 42 --type decision

# Step 4: Continue from where you left off
```

**Why**: No context loss, quick ramp-up

## 4. Decision Tracking

**Pattern**: Record WHY you made choices

```bash
# While implementing
ie log decision "Used HS256 algorithm - simpler, sufficient for internal tokens"

# When blocked
ie log blocker "API rate limit 100/min blocking batch import feature"

# At milestones
ie log milestone "Auth MVP complete - JWT generation + validation working"

# General notes
ie log note "Performance: token generation takes ~2ms avg"
```

**Why**: Future context, project history, knowledge transfer

## 5. Batch Task Creation

**Use Case**: Planning a feature with multiple steps

```bash
cat > plan.json <<EOF
{
  "tasks": [{
    "name": "User Authentication Feature",
    "priority": "high",
    "status": "doing",
    "children": [
      {"name": "JWT schema design", "status": "done"},
      {"name": "Token generation endpoint", "status": "doing"},
      {"name": "Token validation middleware", "status": "todo"},
      {"name": "Refresh token logic", "status": "todo"}
    ]
  }]
}
EOF

ie plan < plan.json
```

**Why**: Upfront planning, clear scope, progress tracking

## 6. Dependency Management

**Use Case**: Task B depends on Task A

```bash
# Create tasks
ie add "Build API"        # Returns: 10
ie add "Build Frontend"   # Returns: 11

# Set dependency
ie task depends-on 11 10  # Frontend depends on API

# Try to start frontend
ie start 11
# ERROR: Task 11 is blocked by incomplete tasks: [10]

# Complete API first
ie start 10
# ... work ...
ie done

# Now frontend is unblocked
ie start 11  # Success!
```

**Why**: Enforces correct order, prevents premature work

## Workflow Comparison

| Scenario | TodoWriter | Intent-Engine |
|----------|-----------|---------------|
| Single task | ✅ Manual | ✅ `ie add` |
| Multi-step | ❌ Flat list | ✅ Hierarchy |
| Focus | ❌ None | ✅ `current_task_id` |
| History | ❌ No | ✅ Events |
| Dependencies | ❌ No | ✅ Built-in |
| Recovery | ❌ Manual | ✅ `search` + `start` |

## Best Practices

1. **Start Simple**: Use `ie add` for quick tasks
2. **Break Down**: Use hierarchy for complex work
3. **Track Decisions**: Use events for important choices
4. **Stay Focused**: One task at a time
5. **Review Regularly**: Use `ie report` weekly
"#;

const PATTERNS_GUIDE: &str = r#"# Common Intent-Engine Usage Patterns

## Pattern 1: Multi-Step Feature Implementation

**Scenario**: Implementing a complex feature

```bash
# Step 1: Create feature structure
echo '{
  "tasks": [{
    "name": "Implement Real-Time Notifications",
    "priority": "high",
    "spec": "Add WebSocket-based notifications for task changes",
    "children": [
      {"name": "Design notification message format"},
      {"name": "Implement WebSocket server"},
      {"name": "Add client-side handlers"},
      {"name": "Write integration tests"}
    ]
  }]
}' | ie plan

# Step 2: Start working
ie start 1  # Parent task
ie start 2  # First subtask

# Step 3: Track decisions
ie log decision "Using JSON for messages - simple, debuggable"

# Step 4: Complete and move on
ie done
ie next  # Automatically suggests subtask 3
```

## Pattern 2: Bug Fixing with Context

**Scenario**: Investigating and fixing a bug

```bash
# Create bug task
ie add "Fix: Dashboard not updating on CLI changes"
ie start 42

# Document investigation
ie log note "Reproduced: 'ie add' doesn't trigger Dashboard update"
ie log note "Checked: WebSocket connection is active"
ie log blocker "Missing HTTP notification endpoint"

# Record solution
ie log decision "Added /api/internal/cli-notify endpoint for CLI→Dashboard sync"

# Mark complete
ie done
```

## Pattern 3: Refactoring Tracking

**Scenario**: Large refactoring project

```bash
# Plan refactoring
echo '{
  "tasks": [{
    "name": "Refactor: MCP Removal",
    "children": [
      {"name": "Remove MCP server code"},
      {"name": "Simplify NotificationSender"},
      {"name": "Update tests"},
      {"name": "Update documentation"}
    ]
  }]
}' | ie plan

# Track as you go
ie start 1
ie start 2  # First subtask

ie log milestone "Deleted 3,700 lines of MCP code"
ie done

ie start 3
ie log decision "Kept WebSocket path, removed MCP channel"
ie done

# Continue...
```

## Pattern 4: Daily Standup Prep

**Scenario**: Preparing for team standup

```bash
# Generate yesterday's report
ie report --since 24h --summary-only

# Review specific task
ie get 42

# Check blockers
ie event list --type blocker --since 24h

# Plan today's work
ie next
```

## Pattern 5: Context Switch Handling

**Scenario**: Urgent bug interrupts feature work

```bash
# Current state
ie current
# Working on: Task #42 "Implement Auth"

# Urgent bug arrives
ie log note "Pausing auth work for urgent bug fix"

# Create and switch to bug
ie add "URGENT: Production login broken"
ie start 99  # New task

# Fix bug
ie log decision "Reverted commit abc123 - broke OAuth flow"
ie done

# Resume original work
ie start 42
ie event list --task-id 42  # Review where you left off
```

## Pattern 6: Sprint Planning

**Scenario**: Planning a 2-week sprint

```bash
# Create sprint structure
echo '{
  "tasks": [
    {
      "name": "Sprint 5: User Management",
      "spec": "Jan 15-28, 2025",
      "priority": "high",
      "children": [
        {
          "name": "User registration flow",
          "priority": "high",
          "children": [
            {"name": "Email validation"},
            {"name": "Password hashing"},
            {"name": "Confirmation email"}
          ]
        },
        {
          "name": "User profile page",
          "priority": "medium",
          "depends_on": ["User registration flow"]
        },
        {
          "name": "Account settings",
          "priority": "low"
        }
      ]
    }
  ]
}' | ie plan

# Track sprint progress
ie ls doing  # What's in progress
ie report --since 7d  # Weekly review
```

## Pattern 7: Knowledge Capture

**Scenario**: Documenting architectural decisions

```bash
# While implementing
ie start 42

ie log decision "Database: Chose SQLite over PostgreSQL
- Reasoning: Simpler deployment, sufficient for current scale
- Trade-off: Limited concurrent writes
- Future: Can migrate to Postgres if needed"

ie log note "Performance benchmark: 10k tasks insert in 2.3s"

ie log milestone "Database schema v2 complete - supports task dependencies"
```

## Pattern 8: Cross-Project Context

**Scenario**: Working on multiple projects

```bash
# Project A
cd ~/work/project-a
ie init
ie add "Implement API endpoint"

# Project B
cd ~/work/project-b
ie init
ie add "Update documentation"

# Each project has isolated database
# Use Dashboard to switch between projects
ie dashboard start
# Dashboard shows all projects
```

## Anti-Patterns (Avoid These)

❌ **Flat Task Lists**: Don't create 20 sibling tasks
✅ **Use Hierarchy**: Group related tasks under parents

❌ **No Decisions**: Don't just complete tasks silently
✅ **Track Why**: Record important decisions

❌ **Stale Tasks**: Don't leave tasks in 'doing' forever
✅ **Complete or Pause**: Mark done or switch away

❌ **Generic Names**: "Fix bug", "Update code"
✅ **Specific Names**: "Fix: Login timeout after 5min", "Update: Add JWT auth to API"

## See Also
- `ie guide ai` - AI assistant patterns
- `ie guide workflow` - Core workflow details
- `ie help` - Command reference
"#;

pub fn handle_guide_command(guide_cmd: GuideCommands) -> Result<()> {
    match guide_cmd {
        GuideCommands::Ai => {
            println!("{}", AI_GUIDE);
        },
        GuideCommands::Plan => {
            println!("{}", PLAN_GUIDE);
        },
        GuideCommands::TodoWriter => {
            println!("{}", TODOWRITER_GUIDE);
        },
        GuideCommands::Workflow => {
            println!("{}", WORKFLOW_GUIDE);
        },
        GuideCommands::Patterns => {
            println!("{}", PATTERNS_GUIDE);
        },
    }

    Ok(())
}
