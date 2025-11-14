# The Intent-Engine Way: A Guide to Intent-Driven Collaboration

## 介绍：这不仅仅是一个任务列表

欢迎使用 Intent-Engine。在开始使用之前，最重要的一点是理解：它不是一个传统的"待办事项"应用，而是一个协作模式的基石。

它的核心目标是为人类与 AI 的协作，建立一个共享的、可追溯的**"意图"**层。人类负责设定战略目标（The Intent），AI 则围绕这个目标展开战术执行（The Execution），而 Intent-Engine 就是连接这两层的核心引擎。

本指南将阐述使用 Intent-Engine 的标准工作流程，解释何时使用、如何使用以及为何这样使用。

---

## 第 1 步：捕获意图 (When & How to `task add`)

### 时机 (When)

当一个想法或需求变得"足够复杂"时，就应该将其捕获为一个 Intent-Engine 任务。触发条件包括：

- **多步骤任务**: 当你或 AI 预见到完成这个需求需要多个独立的操作步骤
- **需要上下文**: 当任务的执行需要依赖大量的背景信息、讨论历史或特定的规格说明
- **长周期工作**: 当任务不可能在一次交互或一个会话中完成，需要被中断和恢复
- **协作节点**: 当任务的完成需要人类和 AI 之间进行多轮的问答、反馈和干预

一个聪明的 AI Agent 应该被训练成能够识别这些信号，并主动向人类提议：

> "这似乎是一个复杂的任务，我建议创建一个 Intent-Engine 任务来跟踪它，您同意吗？"

### 方法 (How)

使用 `ie task add`。关键在于 `spec` 的质量。

```bash
# 将详细的、结构化的需求通过管道传递给 --spec-stdin
echo "# 目标: 实现 OAuth2 登录

## 需求:
- 支持 Google 和 GitHub
- 保留密码登录作为后备方案
- Token 有效期 7 天，支持刷新

## 技术约束:
- 使用 OAuth2 PKCE 流程
- 前后端分离架构" | ie task add --name "实现 OAuth2 登录" --spec-stdin
```

### 原因 (Why)

这是整个流程的起点。一个清晰的、结构化的 spec 就是为 AI 设定了一条明确的"公理"。这可以从根本上减少后续因需求理解不清而导致的错误和返工。我们将模糊的对话，转化为了清晰、可执行的意图。

---

## 第 2 步：激活意图 (When & How to `task start`)

### 时机 (When)

当你或 AI 决定要正式开始着手处理一个已捕获的意图时。这是一个明确的"工作开始"信号。

### 方法 (How)

永远使用 `ie task start <ID> --with-events`。

```bash
# AI 决定开始处理任务 #42
ie task start 42 --with-events
```

### 原因 (Why)

这不仅仅是把状态改成 `doing`。`start` 命令是一个精心设计的原子操作，它至少做了三件至关重要的事：

1. **宣告所有权**: 将任务状态更新为 `doing`，告知所有协作者（包括其他 AI 或人类）"这个任务我正在处理"
2. **聚焦注意力**: 自动将系统的工作区焦点（`current_task_id`）指向这个任务，为所有后续的操作提供基准
3. **加载上下文**: 一步到位地返回任务的完整 `spec` 和 `events_summary`。这使得 AI 可以在一次调用中，就获取开始工作所需的所有目标信息和历史背景，极其高效

---

## 第 2.5 步：智能规划 (When & How to `pick-next`) 🆕

### 时机 (When)

当 AI 发现了多个需要处理的问题，并且这些问题：

- **已被评估**: 每个问题的复杂度和优先级已经明确
- **需要排序**: 需要自动决定处理顺序，而不是按创建顺序处理
- **容量管理**: 需要控制同时进行的任务数量（WIP 限制）

### 方法 (How)

先创建任务并评估，然后使用 `pick-next` 智能选择：

```bash
# 1. AI 在代码审查中发现 5 个问题
ie task add --name "修复空指针异常"
ie task add --name "优化数据库查询"
ie task add --name "修复内存泄漏"
ie task add --name "更新过期依赖"
ie task add --name "添加错误日志"

# 2. AI 评估每个任务的复杂度（1-10）和优先级
ie task update 1 --complexity 3 --priority 10  # 空指针：简单但紧急
ie task update 2 --complexity 7 --priority 8   # 数据库：复杂且重要
ie task update 3 --complexity 9 --priority 10  # 内存：复杂但紧急
ie task update 4 --complexity 5 --priority 5   # 依赖：中等
ie task update 5 --complexity 2 --priority 3   # 日志：简单不紧急

# 3. 智能选择前 3 个任务（按优先级降序，复杂度升序）
ie task pick-next --max-count 3 --capacity 5
# 结果：会选择任务 1（P10/C3）、3（P10/C9）、2（P8/C7）
```

### 原因 (Why)

这体现了"让 AI 专注于思考，让系统负责调度"的理念：

- **Token 节省**: 一次调用完成"查询 todo → 评估容量 → 排序 → 批量更新"，节省 60-70% API 调用
- **决策一致性**: 使用统一的算法（优先级 DESC，复杂度 ASC）确保决策逻辑的可预测性
- **容量保护**: 自动执行 WIP 限制，防止同时开启过多任务导致的效率下降

---

## 第 3 步：执行与记录 (The Execution Loop & `event add`)

这是 Intent-Engine 模式的核心。AI 在执行任务时，会进入一个"感知-思考-行动-记录"的循环。

### 时机 (When to record events)

在执行循环中的每一个关键节点，都必须使用 `ie event add` 进行记录。关键节点包括：

- **做出重要决策时** (`--type decision`): "我选择了库 A 而不是库 B，原因是..."
- **遇到障碍时** (`--type blocker`): "我需要 API 密钥，无法继续"
- **收到人类反馈时** (`--type discussion`): "人类已确认依赖安装完毕"
- **完成一个里程碑时** (`--type milestone`): "数据库迁移脚本已编写完成，等待测试"
- **一次尝试失败后** (`--type note`): "执行 Action A 失败，错误是...，下一步将尝试 Action B"

### 方法 (How)

交替使用"工具箱"中的各类工具，并将关键思考过程写回 Intent-Engine。

```bash
# 1. AI 感知环境 (使用底层工具)
git status
ls -R

# 2. AI 做出决策并行动 (例如：修改文件)
# ... a series of file edits ...

# 3. AI 记录其关键决策 (使用 Intent-Engine)
echo "重构了 token 验证逻辑。

原因：原有逻辑未正确处理过期 token。

改进：
- 添加了 token 过期时间检查
- 实现了自动刷新机制
- 增加了单元测试覆盖" | ie event add --task-id 42 --type decision --data-stdin
```

### 原因 (Why)

Intent-Engine 是 AI 的**外部长时记忆**。AI 的上下文窗口是有限的，它会"遗忘"。`events` 表将 AI 短暂的思考过程，转化为永久的、可查询的项目知识。这能：

- **防止重复犯错**: AI 可以回顾历史，知道哪些路走不通
- **支持中断与恢复**: 任何协作者都可以通过读取 event 历史，无缝地接手工作
- **实现人机协作**: event 是 AI 向人类"请求帮助"和接收"场外指导"的唯一通道
- **提供审计追踪**: 为事后复盘提供了"当时到底发生了什么"的精确记录

---

## 第 3.5 步：处理子问题 (When & How to `spawn-subtask`) 🆕

### 时机 (When)

在执行过程中，当 AI 发现：

- **前置依赖**: 当前任务依赖某个子问题的解决
- **问题分解**: 发现任务过于复杂，需要分解为更小的单元
- **递归发现**: 在处理子任务时又发现了更细的子问题

### 方法 (How)

使用 `spawn-subtask` 在当前任务下创建并切换到子任务：

```bash
# AI 正在处理任务 #42: 实现 OAuth2 登录
ie task start 42 --with-events

# 在实现过程中发现需要先配置 OAuth 应用
ie task spawn-subtask --name "在 Google 和 GitHub 配置 OAuth 应用"

# 这会自动：
# 1. 创建子任务（parent_id = 42）
# 2. 将子任务状态设为 doing
# 3. 切换当前任务到子任务
# 4. 返回子任务详情

# 在配置 OAuth 应用时，又发现需要先申请域名验证
echo "需要先完成域名所有权验证才能创建 OAuth 应用" | \
  ie event add --task-id <child-task-id> --type blocker --data-stdin

ie task spawn-subtask --name "完成域名所有权验证"

# 完成最深层的子任务（spawn-subtask 后该任务已是焦点）
ie task done

# 切回父任务继续
ie task switch <child-task-id>
ie task done  # 完成当前焦点任务

# 最终完成根任务
ie task switch 42
ie task done  # 完成任务42
```

### 原因 (Why)

这强制执行了"先完成子任务才能完成父任务"的业务规则：

- **保持层级清晰**: 避免平铺大量任务，难以理解依赖关系
- **原子切换**: 一步完成创建、启动、设为当前任务，节省 Token
- **强制完整性**: 系统会检查所有子任务是否完成，防止遗漏

---

## 第 3.6 步：任务切换 (When & How to `switch`) 🆕

### 时机 (When)

当需要在多个进行中的任务之间切换时：

- **暂停当前任务**: 处理更紧急的任务
- **并行工作**: 在等待外部反馈时切换到其他任务
- **任务树导航**: 在父任务和子任务之间来回切换

### 方法 (How)

使用 `switch` 在任务间快速切换，并获取完整上下文：

```bash
# 正在处理前端任务 #5
ie task switch 5

# 突然发现后端 API 有问题，需要先修复
ie task switch 12  # 切换到后端任务

# switch 会自动：
# 1. 将任务 #12 状态更新为 doing（如果不是）
# 2. 设置 #12 为当前任务
# 3. 返回任务详情和事件摘要

# 查看切换后的上下文
# 输出会包含 events_summary，帮助 AI 快速恢复记忆

# 修复完成，切回前端任务
ie task switch 5
```

### 原因 (Why)

这是对 AI 工作记忆的有效管理：

- **原子操作**: 将"获取任务 → 更新状态 → 设为当前 → 获取事件"合并为一次调用
- **上下文恢复**: 自动返回 events_summary，帮助 AI 快速回忆"这个任务我做到哪儿了"
- **状态一致性**: 确保每次切换都会正确更新任务状态和工作区焦点

---

## 第 4 步：完成意图 (When & How to `task done`)

### 时机 (When)

当 `spec` 中定义的所有目标都已达成，且所有子任务（如果有）都已完成后。

### 方法 (How)

使用 `ie task done`（不需要 ID 参数，自动完成当前焦点任务）。

**工作流:**
```bash
# 方式 1: 使用 switch/start 命令（自动设置焦点）
ie task switch 42
ie task done

# 方式 2: 手动设置焦点
ie current --set 42
ie task done
```

**新的响应格式:**

命令会返回一个包含三部分的智能响应：
```json
{
  "completed_task": {
    "id": 42,
    "name": "实现用户认证",
    "status": "done"
  },
  "workspace_status": {
    "current_task_id": null
  },
  "next_step_suggestion": {
    "type": "PARENT_IS_READY",
    "message": "All sub-tasks of parent #10 'User System' are now complete...",
    "parent_task_id": 10,
    "parent_task_name": "User System"
  }
}
```

如果任务还有未完成的子任务，系统会返回错误：
```json
{
  "error": "UNCOMPLETED_CHILDREN"
}
```

### 原因 (Why)

`done` 是一个内置了安全检查的原子操作：
- **强制先决条件**: 必须设置为当前焦点任务，确保操作意图明确
- **子任务检查**: 强制执行"必须先完成所有子任务"的业务规则
- **智能建议**: 返回上下文感知的下一步建议，帮助 AI 和用户理解工作流
- **清空焦点**: 自动清空 `current_task_id`，避免误操作已完成的任务

它不是一个简单的状态变更，而是对"这个意图连同其所有子意图都已圆满达成"的最终确认，同时提供智能的后续行动建议。

---

## 第 5 步：回顾与洞察 (When & How to `report`)

### 时机 (When)

在需要生成周期性报告（如周报）、进行项目复盘或对特定类型的工作（如 Bug 修复）进行效率分析时。

### 方法 (How)

使用 `ie report`，并**优先使用 `--summary-only`**。

```bash
# AI 需要为周报生成摘要
ie report --since 7d --status done --summary-only

# 输出示例（小巧的 JSON 摘要）:
# {
#   "summary": {
#     "total_count": 23,
#     "todo_count": 5,
#     "doing_count": 3,
#     "done_count": 15
#   }
# }

# AI 接收到这个小巧的 JSON 摘要后，再用自然语言将其扩展成一份完整的报告
```

更多查询示例：

```bash
# 查看最近 1 天的所有任务（含详情）
ie report --since 1d

# 查看所有进行中的任务
ie report --status doing --summary-only

# 搜索与"认证"相关的已完成任务
ie report --filter-name "认证" --status done --summary-only

# 组合查询：最近 30 天完成的数据库优化工作
ie report --since 30d --status done --filter-spec "数据库" --summary-only
```

### 原因 (Why)

这体现了**"将计算留在数据源"**的最佳实践。AI 的强项是语言和推理，而不是数据聚合。

让 Intent-Engine 在内部高效地完成所有统计计算，只将最终的、高价值的"洞察"结果返回给 AI，可以：

- **极大节省 Token 消耗**: `--summary-only` 只返回统计数字，而不是所有任务详情
- **降低成本**: 更少的 Token 意味着更低的 API 成本
- **提升质量**: AI 将其宝贵的上下文空间用于更高质量的思考和创作，而不是数据处理

---

## 完整工作流示例

### 场景：AI 发现代码审查中的多个问题

```bash
# 1. 捕获意图 - AI 发现 5 个问题
ie task add --name "修复空指针异常 in UserService"
ie task add --name "优化数据库查询性能"
ie task add --name "修复内存泄漏问题"
ie task add --name "更新过期的依赖包"
ie task add --name "添加错误日志记录"

# 2. 评估 - AI 分析每个问题的复杂度和优先级
ie task update 1 --complexity 3 --priority 10
ie task update 2 --complexity 7 --priority 8
ie task update 3 --complexity 9 --priority 10
ie task update 4 --complexity 5 --priority 5
ie task update 5 --complexity 2 --priority 3

# 3. 智能规划 - 自动选择最优任务顺序
ie task pick-next --max-count 3 --capacity 5
# 系统选择：任务 1(P10/C3)、3(P10/C9)、2(P8/C7)

# 4. 执行第一个任务
ie task switch 1

# 4.1 记录决策
echo "问题原因：UserService.getUser() 未检查返回值是否为 null
修复方案：添加 Optional 包装和空值检查
影响范围：3 个调用点" | \
  ie event add --task-id 1 --type decision --data-stdin

# 4.2 执行修复
# ... 修改代码 ...

# 4.3 完成任务（任务1当前是焦点，直接完成）
ie task done

# 5. 处理第二个任务（包含子任务）
ie task switch 3

# 5.1 发现需要先诊断问题
echo "需要先使用 profiler 定位内存泄漏源" | \
  ie event add --task-id 3 --type blocker --data-stdin

ie task spawn-subtask --name "使用 Valgrind 分析内存使用"

# 5.2 完成诊断（子任务当前是焦点，直接完成）
echo "发现问题：WebSocket 连接未正确关闭" | \
  ie event add --task-id <subtask-id> --type milestone --data-stdin
ie task done

# 5.3 切回并完成主任务
ie task switch 3
# ... 修复代码 ...
ie task done

# 6. 生成工作报告
ie report --since 1d --summary-only
```

---

## 核心原则总结

### 1. 意图优先 (Intent-First)
不要让 AI 漫无目的地执行。先明确意图（task），再开始行动。

### 2. 记录一切关键决策 (Record Everything Critical)
AI 的记忆会消失，但 Intent-Engine 不会。每个重要决策都应该被记录。

### 3. 原子操作优先 (Prefer Atomic Operations)
优先使用 `start`、`pick-next`、`spawn-subtask`、`switch`、`done` 这些复合命令，而不是手动组合多个底层操作。

### 4. 层级清晰 (Clear Hierarchy)
使用父子任务保持工作结构清晰。大任务分解为小任务，小任务完成后才能完成大任务。

### 5. 上下文即王道 (Context is King)
始终使用 `--with-events` 获取完整上下文。AI 需要知道"为什么"和"如何"，而不仅仅是"是什么"。

### 6. Token 效率 (Token Efficiency)
使用 `--summary-only`、原子操作、智能选择等机制，最大化每个 Token 的价值。

---

## 反模式警示

### ❌ 不要：直接操作状态
```bash
# 错误：手动组合多个操作
ie task update 42 --status doing
ie current --set 42
ie task get 42 --with-events
```

### ✅ 应该：使用原子操作
```bash
# 正确：一步到位
ie task start 42 --with-events
```

---

### ❌ 不要：平铺所有任务
```bash
# 错误：所有子问题都创建为独立的根任务
ie task add --name "实现 OAuth2"
ie task add --name "配置 Google OAuth"
ie task add --name "配置 GitHub OAuth"
ie task add --name "实现 token 刷新"
```

### ✅ 应该：使用层级结构
```bash
# 正确：使用父子关系
ie task add --name "实现 OAuth2"
ie task start 1
ie task spawn-subtask --name "配置 Google OAuth"
ie task done  # spawn-subtask 后子任务自动成为焦点
ie task spawn-subtask --name "配置 GitHub OAuth"
ie task done  # 完成当前焦点任务
ie task spawn-subtask --name "实现 token 刷新"
ie task done  # 完成当前焦点任务
ie task switch 1  # 切回父任务
ie task done  # 完成父任务
```

---

### ❌ 不要：忘记记录关键决策
```bash
# 错误：AI 做了重要决定但未记录
# ... 选择了库 A ...
# ... 直接继续下一步 ...
```

### ✅ 应该：记录所有关键节点
```bash
# 正确：记录决策过程
echo "选择使用 Passport.js 而不是手写 OAuth 逻辑

原因：
- 成熟稳定，社区支持好
- 支持多种策略
- 减少维护负担

权衡：
- 增加依赖
- 需要学习其 API" | \
  ie event add --task-id 1 --type decision --data-stdin
```

---

## 结语

Intent-Engine 不仅仅是一个工具，它是一种协作哲学的实现。

它将人类的战略思维与 AI 的执行能力有机结合，通过清晰的意图捕获、严格的状态管理和完整的历史记录，使得人机协作变得**可追溯**、**可恢复**、**可扩展**。

掌握"The Intent-Engine Way"，就是掌握了与 AI 协作的艺术。

---

**下一步**: 阅读完整的命令参考 ([README.md](../../../README.md)) 和技术分析 ([task-workflow-analysis.md](../../technical/task-workflow-analysis.md))。
