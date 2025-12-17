# Intent-Engine 下一阶段路线图

**版本**: 0.6.x - 1.0.0
**讨论日期**: 2025-11-20
**状态**: 设计完成，待实施

---

## 📋 概述

本路线图基于 Intent-Engine 当前架构和 depends-on 特性的深入分析，规划了下一阶段的两大核心发展方向：

1. **多 Sub-Agent 并行工作机制** - 支持多个 AI Agent 同时工作
2. **Agent MCP 接口优化** - 简化接口，提升 Agent 使用体验

---

## 🎯 方向一：多 Sub-Agent 并行工作机制

### 背景与动机

当前 Intent-Engine 采用单焦点模型：
- 全局唯一的 `current_task_id`
- 任何时刻只有一个任务处于 `doing` 状态
- 适合单 Agent 深度优先工作模式

**未来需求**：Claude Code 等 AI 平台可能同时启动多个 Sub-Agent 并行工作：
```
Agent A: 实现前端界面 (#100, doing)
Agent B: 实现后端 API (#200, doing)
Agent C: 编写集成测试 (#300, doing)
```

### 核心设计决策

#### 1. Doing 状态级联 ✅

**设计原则**：子任务 doing → 所有祖先任务自动 doing

```
用户认证模块 (#100, doing, not focus)
├─ JWT 实现 (#101, doing, not focus)
│  └─ 令牌生成 (#102, doing, focus) ← Agent 当前工作位置
└─ OAuth2 实现 (#103, todo)
```

**语义明确化**：

| 概念 | 含义 | 数量约束 |
|------|------|----------|
| **doing** | 任务及其子孙正在被执行 | 可多个（一条工作链） |
| **focus** | Agent 当前实际工作的叶子任务 | 有且只有一个 |

**优势**：
- ✅ 状态语义清晰：doing = 工作范围，focus = 当前位置
- ✅ 自动级联维护：子任务完成时，自动判断父任务状态
- ✅ 符合深度优先思维：自然表达工作上下文

#### 2. 暂停语义 ✅

**决策**：接受多个 doing 状态任务同时存在（暂停中）

```
项目根任务 (#1, doing)
├─ 前端开发 (#10, doing, paused) ← 暂停，可被唤醒
├─ 后端开发 (#20, doing, focus)  ← 当前工作
└─ 测试编写 (#30, todo)
```

**影响**：
- `ie task list doing` 会显示多个 doing 任务
- UI 需要区分 focus 和 paused doing
- 展示示例：`"✓ #10 (doing, paused)"` vs `"→ #20 (doing, focus)"`

#### 3. 接口简化：只用 start 命令 ✅

**决策**：不需要显式 pause/resume/switch 命令，只用 `task_start` 自动管理

```rust
// 当前 focus: #10
task_start(20)
  → #10: doing, focus → doing, paused  // 自动暂停
  → #20: todo → doing, focus           // 新焦点
  → #20 的所有祖先: todo → doing       // 级联

// 未来某时重新开始 #10
task_start(10)
  → #20: doing, focus → doing, paused  // 自动暂停
  → #10: doing, paused → doing, focus  // 唤醒
```

**优势**：
- 接口极简，Agent 只需要知道 "我要开始做这个任务"
- 自动管理暂停/唤醒逻辑
- 符合 Agent 自然思维

**实现伪代码**：
```rust
fn task_start(new_id: TaskId) -> Result<()> {
    // 1. 暂停当前焦点任务
    if let Some(old_focus) = workspace.current_task_id {
        task.set_paused(old_focus);
    }

    // 2. 设置新焦点
    task.set_status(new_id, Status::Doing);
    task.set_focus(new_id);

    // 3. 级联祖先为 doing
    cascade_ancestors_to_doing(new_id);

    Ok(())
}
```

#### 4. Done 验证机制 🔍

**决策**：未来引入专门的 "Done 验证 Agent"

**当前问题**：
- Agent 主观判断任务完成 → 可能遗漏检查项
- 缺乏客观验证标准

**未来机制流程**：
```
Agent A: task_done()
  ↓
触发 Done 验证 Agent
  ↓
验证步骤：
  1. 读取任务 spec 和验收标准
  2. 检查代码变更（git diff）
  3. 运行测试（cargo test）
  4. 验证文档更新
  5. 返回 pass/fail
  ↓
If pass:  真正标记为 done
If fail:  创建 blocker event，任务回到 doing
```

**需要新建独立根任务讨论**：
- 验收标准的定义格式（checklist？可执行脚本？）
- 验证 Agent 的触发时机（自动？手动？）
- 验证失败的处理流程
- 与 CI/CD 的集成策略
- 人工审核的角色

### 多 Agent 并行技术方案

#### 方案 A（优先）：无头模式 + 管道

```bash
# Agent A
echo '{"task": "implement frontend"}' | claude -p intent-engine

# Agent B (并行)
echo '{"task": "implement backend"}' | claude -p intent-engine
```

**优势**：
- ✅ 简单，使用现有 Claude CLI
- ✅ 每个 Agent 独立进程，天然隔离
- ✅ 通过 stdin/stdout 通信

**挑战**：
- ❓ 需要 Claude CLI 支持无头模式
- ❓ 如何协调多个 Agent 的输出

#### 方案 B（最终）：Claude Agent SDK

```python
from claude_sdk import Agent, Workspace

workspace = Workspace(project="intent-engine")
agent_a = Agent(workspace, task="frontend")
agent_b = Agent(workspace, task="backend")

await asyncio.gather(
    agent_a.run(),
    agent_b.run()
)
```

**优势**：
- ✅ 完全控制 Agent 生命周期
- ✅ 统一管理 workspace 状态
- ✅ 支持复杂协调逻辑

**挑战**：
- ❓ 依赖 Claude SDK 发布时间
- ❓ 需要编写协调层代码

### 待建立的独立根任务

基于本次讨论，以下话题需要独立深入设计：

1. **多 Sub-Agent 并行架构设计**
   - Agent 间通信机制（共享 DB？消息队列？）
   - 冲突检测和解决（同时修改同一文件）
   - 进度聚合和展示
   - 错误处理策略
   - 依赖协调机制

2. **Done 验证机制设计**
   - 验收标准定义
   - 验证 Agent 实现
   - 失败处理流程

---

## 🎯 方向二：Agent MCP 接口优化

### 背景与动机

Intent-Engine 当前提供 14+ MCP 工具：
- **任务 CRUD**: task_add, task_update, task_delete
- **工作流**: task_start, task_done, task_switch, task_spawn_subtask, task_pick_next
- **查询**: task_list, task_get, task_context, current_task_get
- **依赖**: task_add_dependency
- **事件**: event_add, event_list
- **搜索**: search, report_generate

**当前痛点**：

1. **原子性差** - 创建复杂任务结构需要多次调用
   ```javascript
   // 需要 7 次 MCP 调用才能建立结构
   const parent = await task_add({name: "实现认证"})
   const jwt = await task_add({name: "JWT", parent_id: parent.id})
   const login = await task_add({name: "登录", parent_id: parent.id})
   // ...
   await task_add_dependency({blocked: docs.id, blocking: parent.id})
   await task_start({task_id: parent.id})
   ```

2. **接口冗余** - add/update/delete 功能重叠，增加认知负担

3. **Agent 认知负担高** - 14 个工具，每个 2-5 个参数，决策点过多

### 核心设计决策：Plan 接口 v2

#### 设计哲学：参考 TodoWrite

**TodoWrite 的启示**：
```typescript
// TodoWrite 极简设计
TodoWrite({
  todos: [
    {content: "任务1", status: "completed", activeForm: "..."},
    {content: "任务2", status: "in_progress", activeForm: "..."}
  ]
})
```

**关键特点**：
- ✅ 直接传入期望状态
- ✅ 系统自动 diff 和应用变更
- ✅ 没有 mode/id/operation 等元概念
- ✅ 声明式，符合直觉

#### Plan 接口设计

**核心 API**：

```typescript
interface PlanRequest {
  tasks: TaskTree[]
  // 仅此而已！系统自动处理一切
}

interface TaskTree {
  // 基础信息
  name: string                    // 任务名（唯一标识）
  spec?: string                   // 规格说明
  priority?: "critical" | "high" | "medium" | "low"

  // 层级关系（直接嵌套）
  children?: TaskTree[]           // 子任务树

  // 依赖关系（名称引用）
  depends_on?: string[]           // 依赖的任务名称列表

  // 可选：更新模式
  task_id?: number                // 如果提供，强制更新指定任务
}
```

**使用示例**：

```typescript
// 场景1: 创建任务树
plan({
  tasks: [
    {
      name: "用户认证系统",
      priority: "high",
      spec: "实现完整的认证流程",
      children: [
        {
          name: "JWT 令牌实现",
          spec: "HS256 算法，1小时过期"
        },
        {
          name: "OAuth2 集成",
          spec: "支持 Google、GitHub"
        }
      ]
    },
    {
      name: "API 客户端",
      depends_on: ["用户认证系统"],  // 名称引用依赖
      children: [
        {name: "HTTP 客户端封装"},
        {name: "认证拦截器"}
      ]
    }
  ]
})
```

#### 系统处理逻辑

```rust
fn plan(request: PlanRequest) -> Result<PlanResult> {
    // BEGIN TRANSACTION

    // 1. 名称查找：已存在的任务
    let existing: HashMap<String, TaskId> = db.find_tasks_by_names(
        extract_all_names(&request.tasks)
    );

    // 2. 自动分类：create vs update
    for task in flatten_tree(&request.tasks) {
        if existing.contains_key(&task.name) {
            // 已存在 → UPDATE
            update_task(existing[&task.name], task);
        } else {
            // 不存在 → CREATE
            create_task(task);
        }
    }

    // 3. 建立关系
    build_parent_child_relations(&request.tasks);
    build_dependencies(&request.tasks, &existing);

    // 4. 验证 DAG（循环依赖检测）
    validate_no_cycles()?;

    // COMMIT
    Ok(PlanResult { task_id_map, ... })
}
```

#### 关键优化点

**1. 去除临时 ID**

```typescript
// ❌ 之前：需要管理临时 ID
{
  id: "temp-auth",
  parent_id: "temp-root",
  children: ["temp-jwt", "temp-oauth"]
}

// ✅ 现在：直接嵌套
{
  name: "认证系统",
  children: [
    {name: "JWT"},
    {name: "OAuth"}
  ]
}
```

**2. 去除 mode 参数**

```typescript
// ❌ 之前：Agent 需要决定模式
plan({mode: "create", tasks: [...]})
plan({mode: "update", tasks: [...]})

// ✅ 现在：系统自动判断
plan({tasks: [...]})  // 名称存在→更新，不存在→创建
```

**3. 名称引用依赖**

```typescript
// ❌ 之前：临时 ID 引用
dependencies: [
  {blocked: "temp-api", blocking: "temp-auth"}
]

// ✅ 现在：名称引用
{
  name: "API客户端",
  depends_on: ["认证系统"]  // 清晰直观
}
```

**4. 幂等性**

```typescript
// 同样的 plan 调用多次 → 结果相同
plan({
  tasks: [{name: "任务A", spec: "v1"}]
})

plan({
  tasks: [{name: "任务A", spec: "v2"}]  // 自动更新 spec
})
```

#### 边界情况处理

**名称冲突**（策略：自动合并）：
```rust
plan({tasks: [
  {name: "任务A", priority: "high"},
  {name: "任务A", spec: "新规格"}
]})
// 结果：任务A {priority: high, spec: "新规格"}
```

**依赖不存在**：
```rust
plan({tasks: [
  {name: "API", depends_on: ["不存在的任务"]}
]})
// 错误：Dependency '不存在的任务' not found in plan or database
```

**循环依赖**：
```rust
plan({tasks: [
  {name: "A", depends_on: ["B"]},
  {name: "B", depends_on: ["A"]}
]})
// 错误：Circular dependency: A → B → A
```

#### CLI 映射

```bash
# YAML 格式（推荐）
cat > plan.yaml <<'YAML'
tasks:
  - name: 认证系统
    priority: high
    children:
      - name: JWT实现
      - name: OAuth实现
  - name: API客户端
    depends_on:
      - 认证系统
YAML

ie plan plan.yaml

# JSON 格式
ie plan plan.json

# Stdin
cat plan.yaml | ie plan --stdin

# 验证模式（不实际执行）
ie plan plan.yaml --dry-run
```

#### 接口对比

| 场景 | 旧方式 | Plan 方式 |
|------|--------|----------|
| 创建单任务 | `task_add` | `plan {tasks:[{name:"A"}]}` |
| 创建树 | 5次 add + spawn | 1次 plan |
| 添加依赖 | `task_add_dependency` | plan 中 depends_on |
| 更新任务 | `task_update` | 再次 plan（幂等）|

**保留简单场景快捷方式**：
```bash
ie task add "快速任务"  # 仍然可用，简单场景

ie plan complex.yaml   # 复杂结构用 plan
```

### 接口简化路线

**目标**：从 14+ 个 MCP 工具简化到 8-10 个核心接口

#### 最终接口设计（1.0）

**工作流（4个）**：
- `task_start` - 开始任务（自动级联，自动暂停旧焦点）
- `task_done` - 完成当前任务
- `task_switch` - 切换焦点（实际调用 start）
- `task_pick_next` - 智能推荐下一个任务

**规划（1个）**：
- `plan` - 声明式创建/更新任务结构

**查询（3个）**：
- `task_list` - 元数据过滤（status, parent）
- `search` - 全文搜索（任务+事件）
- `current_task_get` - 获取当前焦点

**事件（2个）**：
- `event_add` - 记录决策/里程碑/阻塞
- `event_list` - 查询事件历史

#### 废弃接口时间表

| 版本 | 操作 | 说明 |
|------|------|------|
| **0.6.0** | 引入 plan，标记 deprecated | 保留所有现有接口，添加警告 |
| **0.7.0** | 移除 5 个废弃接口 | task_update, task_delete, task_spawn_subtask, task_add_dependency, task_context |
| **1.0.0** | 最终简化 | 稳定 8-10 个核心接口，SemVer 保证 |

**废弃接口**：
- `task_update` → 用 plan (mode=update)
- `task_delete` → 用 plan (mode=replace)
- `task_spawn_subtask` → 用 plan + start
- `task_add_dependency` → 用 plan.dependencies
- `task_context` → 查询类接口足够
- `task_get` → 用 search 或 list

**保留接口**（向后兼容）：
- `task_add` - 简单场景快捷方式

### 关键特性

#### 原子性保证

```rust
plan(...) -> Result<PlanResult> {
    // SQLite Transaction
    BEGIN TRANSACTION;

    // 1. 验证阶段
    validate_no_cycles()?;
    check_dependencies_exist()?;

    // 2. 执行阶段
    create_or_update_tasks()?;
    build_relations()?;

    // 3. 提交或回滚
    COMMIT;  // 全失败策略（All-or-Nothing）
}
```

#### 幂等性

```typescript
// 多次调用相同 plan → 结果相同
plan({tasks: [{name: "A", spec: "v1"}]})
plan({tasks: [{name: "A", spec: "v1"}]})  // 无变化
plan({tasks: [{name: "A", spec: "v2"}]})  // 仅更新 spec
```

#### 验证完整性

验证顺序（快速失败）：

1. **语法验证**（最快）- JSON schema 验证
2. **引用验证**（次快）- parent_id, dependency ID 存在性
3. **图验证**（稍慢）- 循环依赖检测（Tarjan 算法）
4. **数据库验证**（最慢）- task_id 存在性（update 模式）

错误信息示例：
```
❌ Plan validation failed at: Graph Validation

Circular dependency detected:
  Task A (name: "feature-a")
    → depends on Task B (name: "feature-b")
    → depends on Task A (name: "feature-a")

Suggestion: Remove dependency between feature-b and feature-a
```

---

## 🗓️ 实施计划

### Phase 1: 基础 Plan 接口（v0.6.0）

**时间**: Q1 2025

**目标**：
- ✅ 实现 plan 接口（create 模式）
- ✅ 名称查找 + 自动分类逻辑
- ✅ 嵌套树解析
- ✅ 保留所有现有接口
- ✅ 添加 deprecation 警告

**交付物**：
- `plan` MCP 工具（基础版）
- CLI 命令 `ie plan <file>`
- 单元测试覆盖率 > 90%
- 用户文档更新

### Phase 2: 完整功能（v0.6.1）

**时间**: Q2 2025

**目标**：
- ✅ 幂等更新
- ✅ 依赖解析（depends_on）
- ✅ 循环依赖检测
- ✅ CLI 支持 YAML/JSON
- ✅ 验证模式（--dry-run）

**交付物**：
- plan 完整功能实现
- 迁移指南文档
- 性能测试报告

### Phase 3: 接口简化（v0.7.0）

**时间**: Q3 2025

**目标**：
- ✅ 移除 deprecated 接口
- ✅ plan 成为主要创建方式
- ✅ 更新所有文档和示例
- ✅ Agent 集成测试

**交付物**：
- 简化后的 MCP schema
- 完整迁移指南
- 向后兼容性测试

### Phase 4: 多 Agent 支持（v0.8.0）

**时间**: Q4 2025

**目标**：
- ✅ doing 状态级联实现
- ✅ focus vs paused doing 区分
- ✅ task_start 自动暂停逻辑
- ✅ UI 展示优化

**交付物**：
- 多焦点支持
- Dashboard 并行展示
- 协调机制原型

### Phase 5: 稳定 1.0（v1.0.0）

**时间**: 2026 Q1

**目标**：
- ✅ 最终接口锁定（8-10 个）
- ✅ SemVer 保证
- ✅ 性能优化
- ✅ 完整文档和教程

**交付物**：
- 1.0 稳定版发布
- 完整 API 参考
- 最佳实践指南

---

## 📊 成功指标

### 接口简化

- **接口数量**：14+ → 8-10 个 MCP 工具
- **Agent 认知负担**：参数总数减少 40%+
- **调用次数**：复杂场景从 5-7 次减少到 1-2 次

### 多 Agent 支持

- **并发性**：支持 3+ Agent 同时工作
- **状态一致性**：SQLite 事务保证 100% 原子性
- **UI 体验**：清晰展示 focus vs paused 状态

### 开发体验

- **文档覆盖率**：100% 接口有详细文档
- **测试覆盖率**：核心逻辑 > 95%
- **迁移成本**：提供自动化迁移脚本

---

## 🔮 未来展望

### 短期（6-12个月）

1. **Done 验证 Agent**
   - 自动验证任务完成度
   - 集成 CI/CD 流程
   - 人工审核机制

2. **多 Agent 协调机制**
   - 冲突检测和解决
   - 进度聚合展示
   - 依赖自动协调

### 长期（12-24个月）

1. **Agent SDK**
   - Python/TypeScript SDK
   - 编程式 Agent 控制
   - 复杂工作流编排

2. **分布式支持**
   - 跨机器 Agent 协作
   - 分布式任务队列
   - 云端同步

3. **AI 增强**
   - 自动任务分解
   - 智能优先级调整
   - 验收标准生成

---

## 📚 参考文档

- **接口规范**: `docs/spec-03-interface-current.md`
- **MCP Schema**: `mcp-server.json`
- **Agent 指南**: `AGENT.md`
- **Claude 集成**: `CLAUDE.md`

---

## 🤝 贡献

本路线图基于社区反馈和实际使用场景制定。欢迎通过以下方式参与：

- **GitHub Issues**: 提出功能需求或问题
- **Pull Requests**: 贡献代码或文档
- **Discussions**: 参与设计讨论

---

**最后更新**: 2025-11-20
**维护者**: Intent-Engine 核心团队
**版本**: 1.0（路线图初版）
