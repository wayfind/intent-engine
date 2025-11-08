# 通用 LLM 工具集成指南

本指南说明如何将 Intent-Engine 集成到任意 AI 工具中，无论是 GPT、Claude、Gemini 还是其他 LLM。

---

## 核心原理

Intent-Engine 通过 **CLI + JSON** 的方式与 AI 工具交互：

1. AI 工具通过 `Bash`/`Shell` 能力调用 `intent-engine` 命令
2. Intent-Engine 返回 JSON 格式的结果
3. AI 解析 JSON 并继续工作

**关键优势：**
- ✅ 无需专门的插件或扩展
- ✅ 适用于任何支持 Shell 命令执行的 AI 工具
- ✅ 完全的功能覆盖（与 MCP Server 相同）

---

## 前置要求

1. **Intent-Engine 已安装并在 PATH 中**
   ```bash
   intent-engine --version
   ```

2. **AI 工具支持执行 Shell 命令**
   - GPT: Code Interpreter / Advanced Data Analysis
   - Claude: Bash tool（通过 Anthropic API）
   - Gemini: Code execution capability
   - 其他：任何有 Shell 访问的环境

---

## 集成步骤

### 步骤 1：准备 System Prompt

在你的 AI 工具的 System Prompt 或 Custom Instructions 中添加：

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

Full guide: docs/zh-CN/guide/ai-quick-guide.md
```

### 步骤 2：在对话中激活

在需要使用 Intent-Engine 时，明确告诉 AI：

```
请使用 Intent-Engine 追踪这个任务：实现用户认证系统
```

或者：

```
Let's track this work with Intent-Engine. Please create a task for
implementing the user authentication system.
\`\`\`

### 步骤 3：验证集成

测试 AI 是否能正确使用 Intent-Engine：

**测试对话示例：**

```
You: 我需要重构数据库查询层，请用 Intent-Engine 追踪这个任务。

AI: 我来创建一个 Intent-Engine 任务来追踪这次重构。

[执行命令]
echo "重构数据库查询层
- 统一查询接口
- 添加连接池管理
- 实现查询缓存
- 添加慢查询日志" | \
  intent-engine task add --name "重构数据库查询层" --spec-stdin

[输出]
{
  "id": 1,
  "name": "重构数据库查询层",
  "status": "todo",
  ...
}

AI: 任务已创建（ID: 1）。让我开始这个任务并查看上下文。

[执行命令]
intent-engine task start 1 --with-events

[AI 继续工作...]
```

---

## 最佳实践

### 1. 任务创建时机

**推荐创建任务：**
- ✅ 预计需要多次对话才能完成的工作
- ✅ 需要记录"为什么这样做"的决策
- ✅ 涉及多个相关子问题的复杂任务

**不推荐创建任务：**
- ❌ 一次性的简单问题（如"如何安装 Python"）
- ❌ 纯信息查询（如"什么是 JWT"）

### 2. 规格说明（Spec）的写法

好的规格说明应该包含：

```markdown
# 目标
[简要描述要实现什么]

# 需求
- [具体需求 1]
- [具体需求 2]
- ...

# 技术约束
- [技术选型]
- [架构要求]
- [性能目标]

# 参考资料
- [相关文档链接]
```

**示例：**

```bash
echo "# 目标
实现基于 JWT 的用户认证系统

# 需求
- 支持用户注册和登录
- Token 有效期 7 天
- 支持 Token 刷新
- 密码使用 bcrypt 加密

# 技术约束
- 使用 Rust + Actix-Web
- JWT 库使用 jsonwebtoken
- 数据库使用 PostgreSQL

# 参考资料
- RFC 7519 (JWT)
- https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html" | \
  intent-engine task add --name "实现 JWT 认证" --spec-stdin
```

### 3. 事件记录策略

**什么时候记录事件：**

| 事件类型 | 使用场景 | 示例 |
|---------|---------|------|
| `decision` | 做出关键技术决策 | "决定使用 Redis 作为缓存，而不是 Memcached" |
| `blocker` | 遇到阻塞问题 | "需要等待 API 密钥审批" |
| `milestone` | 完成重要阶段 | "完成核心逻辑，单元测试通过" |
| `discussion` | 记录讨论结果 | "与团队讨论后确定使用微服务架构" |
| `note` | 一般备注 | "发现性能瓶颈在数据库查询" |

### 4. 任务层级设计

**推荐层级：**

```
根任务（战略目标）
├── 子任务 1（战术步骤）
│   ├── 子任务 1.1（具体实现）
│   └── 子任务 1.2
├── 子任务 2
│   ├── 子任务 2.1
│   │   └── 子任务 2.1.1（递归分解）
│   └── 子任务 2.2
└── 子任务 3
```

**避免过深层级：**
- ✅ 1-3 层：推荐
- ⚠️ 4-5 层：可接受
- ❌ 6+ 层：过度复杂，考虑重新设计

---

## AI 工作流示例

### 场景 1：发现多个问题的代码审查

```bash
# 1. AI 发现 5 个问题，批量创建任务
intent-engine task add --name "修复空指针异常"
intent-engine task add --name "优化数据库查询"
intent-engine task add --name "修复内存泄漏"
intent-engine task add --name "更新过期依赖"
intent-engine task add --name "添加错误日志"

# 2. AI 评估优先级和复杂度
intent-engine task update 1 --priority 10 --complexity 3  # 紧急且简单
intent-engine task update 2 --priority 8 --complexity 7   # 重要但复杂
intent-engine task update 3 --priority 10 --complexity 9  # 紧急且复杂
intent-engine task update 4 --priority 5 --complexity 5   # 中等
intent-engine task update 5 --priority 3 --complexity 2   # 不紧急且简单

# 3. 智能选择任务（按优先级降序、复杂度升序）
intent-engine task pick-next --max-count 3
# 会选择：任务1 (P10/C3)、任务3 (P10/C9)、任务2 (P8/C7)

# 4. 逐个处理
intent-engine task switch 1
# ... 修复 ...
echo "原因：未检查 null 返回值" | \
  intent-engine event add --task-id 1 --type note --data-stdin
intent-engine task done 1

# 5. 生成报告
intent-engine report --since 1d --summary-only
```

### 场景 2：递归问题分解

```bash
# 1. 开始大任务
echo "实现完整的支付系统..." | \
  intent-engine task add --name "实现支付系统" --spec-stdin
intent-engine task start 1 --with-events

# 2. 发现子问题
intent-engine task spawn-subtask --name "集成 Stripe API"

# 3. 又发现更细的问题
intent-engine task spawn-subtask --name "配置 Webhook 回调"

# 4. 完成最深层任务
echo "已配置 webhook endpoint" | \
  intent-engine event add --task-id 3 --type milestone --data-stdin
intent-engine task done 3

# 5. 逐层完成
intent-engine task switch 2
intent-engine task done 2
intent-engine task switch 1
intent-engine task done 1
```

---

## 常见问题

### Q: AI 忘记使用 Intent-Engine 怎么办？

**A**: 在 System Prompt 中强调使用规则：

```markdown
IMPORTANT: For all complex, multi-step tasks, you MUST use Intent-Engine
to track strategic intent. Before starting any significant work, create
a task with `intent-engine task add`.
```

### Q: 如何让 AI 自动记录决策？

**A**: 在 System Prompt 中添加：

```markdown
Whenever you make a key technical decision, record it immediately:

echo "Your decision and reasoning..." | \
  intent-engine event add --task-id <current-task-id> --type decision --data-stdin
```

### Q: JSON 输出太长，影响上下文怎么办？

**A**: 使用 `--summary-only` 和 `jq` 过滤：

```bash
# 只获取摘要
intent-engine report --summary-only

# 只提取需要的字段
intent-engine task get 1 | jq '{id, name, status, spec}'

# 只看最近 5 个事件
intent-engine event list --task-id 1 --limit 5
```

### Q: 如何在团队中共享 Intent-Engine 数据？

**A**: SQLite 数据库可以提交到 Git：

```bash
# .gitignore 中确保不忽略 .intent-engine/
!.intent-engine/
!.intent-engine/project.db

# 提交数据库
git add .intent-engine/project.db
git commit -m "Update task database"
```

**注意**: 大型团队可能需要中心化存储方案（未来计划支持）。

---

## 高级用法

### 1. 自定义 AI 提示词模板

为你的 AI 工具创建专用的提示词模板：

```markdown
# Task: {{task_name}}

## Context
{{task_spec}}

## Recent Decisions
{{event_history}}

## Instructions
[你的具体指令]

## Remember
- Record all key decisions
- Use spawn-subtask for sub-problems
- Switch tasks with `task switch`
- Complete with `task done` only when all subtasks are done
```

### 2. 集成到自动化工作流

```bash
#!/bin/bash
# auto-task-report.sh

# 每天自动生成工作报告
intent-engine report --since 1d --summary-only > /tmp/daily-report.json

# 发送到 AI 生成自然语言总结
cat /tmp/daily-report.json | your-ai-cli summarize
```

### 3. 多项目管理

```bash
# 项目 A
cd /path/to/project-a
intent-engine task add --name "Feature X"

# 项目 B
cd /path/to/project-b
intent-engine task add --name "Feature Y"

# 每个项目独立的 .intent-engine/ 数据库
```

---

## 下一步

1. 📖 阅读 [AI Quick Guide](../guide/ai-quick-guide.md) 了解完整命令
2. 🚀 参考 [QUICKSTART.md](../../../QUICKSTART.md) 体验核心功能
3. 💡 学习 [The Intent-Engine Way](../guide/the-intent-engine-way.md) 理解最佳实践

---

**需要帮助？**

- [GitHub Issues](https://github.com/wayfind/intent-engine/issues)
- [Contributing Guide](../contributing/contributing.md)
