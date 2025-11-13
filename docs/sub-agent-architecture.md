# Intent-Engine: Sub-Agent Architecture Specification

**版本**: 1.0
**状态**: 设计提案 - 待技术验证
**目标**: 通过任务级上下文隔离，彻底解决长上下文记忆衰减问题
**前置依赖**: Phase 1 (焦点恢复) 必须先实施并验证

---

## 1. 核心理念

### 1.1 哲学基础

**最好的记忆强化，就是不需要记忆。**

与其让一个 Agent 在 100 轮对话中努力记住最初的目标，不如让每个任务都有一个专注的、短生命周期的 Agent。这是一种**"上下文重置"**策略，用架构设计解决认知限制。

### 1.2 架构概览

```
┌─────────────────────────────────────────────────┐
│  Main Agent (项目级协调者)                       │
│  - 接收用户需求                                  │
│  - 分解和管理任务                                │
│  - 调度 Sub-Agent                               │
│  - 处理简单任务（混合模式）                       │
└──────────────┬──────────────────────────────────┘
               │
               ├──→ [Sub-Agent #1: Task 42]
               │    上下文: Task 42 的 spec + events
               │    目标: 单一任务完成
               │    生命周期: 10-30 轮对话
               │
               ├──→ [Sub-Agent #2: Task 43]
               │    上下文: Task 43 的 spec + events
               │    目标: 单一任务完成
               │    生命周期: 15-30 轮对话
               │
               └──→ [Sub-Agent #3: Task 44]
                    ...
```

---

## 2. Main Agent 职责（混合模式）

### 2.1 核心职责

**战略层面**:
- ✅ 接收用户需求，理解意图
- ✅ 使用 Intent-Engine 分解任务
- ✅ 管理任务生命周期（add, spawn, 调度）
- ✅ 决定哪些任务需要 Sub-Agent，哪些自己处理

**战术层面**（有限参与）:
- ✅ 处理简单、单步的任务
- ✅ 快速修复和微调
- ✅ 回答用户的即时问题
- ❌ 不处理复杂、多步骤的实现

### 2.2 决策规则：何时启动 Sub-Agent？

```rust
fn should_spawn_sub_agent(task: &Task) -> bool {
    // 规则1: 任务复杂度
    if task.estimated_steps > 5 { return true; }

    // 规则2: 任务已有子任务
    if !task.children.is_empty() { return true; }

    // 规则3: 任务有明确的 spec（战略性）
    if task.spec.len() > 100 { return true; }

    // 规则4: 用户明确要求专注处理
    if user_requested_focused_work { return true; }

    // 否则，Main Agent 自己处理
    false
}
```

**示例判断**:

| 任务类型 | 示例 | 谁处理？ |
|---------|------|---------|
| 简单查询 | "列出所有 todo 任务" | Main Agent |
| 快速修复 | "修复这个拼写错误" | Main Agent |
| 复杂功能 | "实现JWT认证系统" | Sub-Agent |
| 有子任务的任务 | "用户认证"（已分解为3个子任务） | Sub-Agent（为每个子任务） |

---

## 3. Sub-Agent 规范

### 3.1 生命周期

```
创建 → 上下文注入 → 工作执行 → 任务完成 → 状态持久化 → 销毁
```

**阶段详解**:

1. **创建**: Main Agent 调用宿主环境 API 创建新 Agent 实例
2. **上下文注入**: 注入任务的 spec、parent context、events history
3. **工作执行**: Sub-Agent 专注于单一任务，10-30 轮对话
4. **任务完成**: Sub-Agent 调用 `ie task done`
5. **状态持久化**: 所有进展、决策已通过 `ie event add` 记录
6. **销毁**: Main Agent 回收 Sub-Agent

### 3.2 上下文注入模板

```markdown
You are a Sub-Agent working on a specific task within a larger project.

**YOUR MISSION**: Task #{task_id} - "{task_name}"

**PARENT CONTEXT**:
- Parent Task: #{parent_id} "{parent_name}"
- Your role: {role_description}

**TASK SPECIFICATION**:
{task.spec}

**DECISION HISTORY** (from previous work on this task):
{events_summary}

**YOUR GOAL**:
Complete this single task. When done, call `ie task done`.
You have access to all development tools. Focus on quality.

**IMPORTANT**:
- If you discover this task needs to be broken down, use `ie task spawn-subtask`
- Record key decisions: `ie event add --type decision --data "..."`
- Record blockers: `ie event add --type blocker --data "..."`
- Your work will be reviewed by Main Agent after completion

**WORKSPACE STATE**:
Current task is already set to #{task_id} by Main Agent.
```

### 3.3 Sub-Agent 的权限

**可以做**:
- ✅ 使用所有开发工具（Read, Write, Edit, Bash, etc.）
- ✅ 使用 Intent-Engine 的 event 系列命令
- ✅ 使用 `ie task spawn-subtask` 分解任务
- ✅ 使用 `ie task done` 完成任务
- ✅ 查询 Intent-Engine 状态（current, context, list, search）

**不应该做**:
- ❌ 不应该切换到其他任务（`ie task switch`）
- ❌ 不应该启动新的顶层任务（`ie task add`）
- ❌ 不应该调度其他 Sub-Agent

---

## 4. 工作流示例

### 4.1 场景：实现用户认证功能

```bash
# === Main Agent ===
用户: "帮我实现一个完整的用户认证系统"

Main Agent 分析:
  - 这是一个复杂任务（estimated_steps > 10）
  - 需要分解为子任务
  - 每个子任务由 Sub-Agent 处理

Main Agent 执行:
  → ie task add "Implement user authentication system" \
      --spec "Complete auth with JWT, password hashing, session management"
  → ie task start 42
  → ie task spawn-subtask "JWT token generation" \
      --spec "Use jsonwebtoken crate, HS256, 7-day expiry"
  → ie task spawn-subtask "Password hashing" \
      --spec "Use argon2, secure salt generation"
  → ie task spawn-subtask "Session management" \
      --spec "In-memory store, 24h timeout"

Main Agent 决策:
  - Task 43 (JWT) 是复杂任务 → 启动 Sub-Agent

# === Sub-Agent for Task 43 ===
Launch Sub-Agent with context:
  Task ID: 43
  Name: "JWT token generation"
  Spec: "Use jsonwebtoken crate, HS256, 7-day expiry"
  Parent: 42 "Implement user authentication system"

Sub-Agent #43 开始工作:
  → 调研 jsonwebtoken crate 文档
  → ie event add --type note --data "Comparing jsonwebtoken vs jose-jwt"
  → ie event add --type decision --data "Chose jsonwebtoken: better maintained, simpler API"
  → 设计 token 结构 (claims, expiry)
  → 实现 generate_token() 函数
  → 实现 validate_token() 函数
  → 写单元测试
  → ie event add --type milestone --data "JWT generation and validation complete, tests passing"
  → ie task done

Sub-Agent #43 结束

# === Main Agent 继续 ===
Main Agent 收到完成信号:
  → ie task pick-next
  → 推荐: Task 44 (Password hashing)
  → 启动 Sub-Agent for Task 44

# === Sub-Agent for Task 44 ===
Launch Sub-Agent with context:
  Task ID: 44
  Name: "Password hashing"
  ...

Sub-Agent #44 开始工作:
  → 调研 argon2 vs bcrypt
  → ...
```

### 4.2 场景：Sub-Agent 发现需要进一步分解

```bash
# Sub-Agent #43 工作中
Sub-Agent #43:
  → 开始实现 JWT token generation
  → 发现任务比预期复杂，需要分解

Sub-Agent #43 执行:
  → ie task spawn-subtask "Token signing logic"
  → ie task spawn-subtask "Token validation logic"
  → ie task spawn-subtask "Token refresh mechanism"

Sub-Agent #43 继续:
  → 当前焦点自动切换到第一个子任务
  → 逐个完成这些子任务
  → 所有子任务完成后，完成自己（Task 43）
  → ie task done
```

**关键**: Sub-Agent 有权分解任务，但仍然在自己的"任务树"内工作，不会跳到其他任务。

---

## 5. Agent 间通信协议

### 5.1 Main → Sub: 启动指令

```json
{
  "action": "launch_sub_agent",
  "task_id": 43,
  "context": {
    "task": { "id": 43, "name": "...", "spec": "..." },
    "parent": { "id": 42, "name": "..." },
    "events": [...],
    "initial_prompt": "You are a Sub-Agent working on..."
  }
}
```

### 5.2 Sub → Main: 完成信号

```json
{
  "action": "task_complete",
  "task_id": 43,
  "status": "done",
  "summary": "JWT implementation complete, tests passing",
  "key_events": [
    { "type": "decision", "data": "Chose jsonwebtoken crate" },
    { "type": "milestone", "data": "All tests passing" }
  ]
}
```

### 5.3 Sub → Main: 请求协助

```json
{
  "action": "request_assistance",
  "task_id": 43,
  "issue": "blocked",
  "description": "Need decision: Should we support refresh tokens in this phase?"
}
```

Main Agent 可以：
- 回答问题
- 修改任务 spec
- 或直接接管（暂停 Sub-Agent，自己处理）

---

## 6. 跨会话支持

### 6.1 Sub-Agent 也会跨会话

**场景**:
```
Day 1:
  Main Agent 启动 Sub-Agent for Task 43
  Sub-Agent 工作了 15 轮
  用户: "我得下班了，明天继续"
  → 会话结束，Sub-Agent 状态保存在 Intent-Engine 中

Day 2:
  新会话开始
  焦点恢复: Task 43 仍然是 current_task_id
  Main Agent: "发现 Task 43 未完成，重新启动 Sub-Agent"
  → 启动 Sub-Agent，注入 Task 43 的完整 context + events
  Sub-Agent: "我看到昨天已经完成了 X 和 Y，现在继续 Z"
```

### 6.2 关键：状态持久化

Sub-Agent 的状态**不依赖 Agent 实例本身**，而是依赖 Intent-Engine：

- ✅ 任务状态: `ie current` 告诉我们焦点在哪
- ✅ 工作历史: `ie task context --with-events` 包含所有决策和进展
- ✅ 子任务状态: 子任务的完成情况清晰可见

**这意味着**: Sub-Agent 是"无状态"的，可以随时创建、销毁、重建。状态都在 Intent-Engine 中。

---

## 7. 技术要求

### 7.1 宿主环境必须支持

| 能力 | 描述 | 优先级 |
|-----|------|-------|
| Agent 实例化 | 创建新的 Agent 实例 | P0 |
| 上下文注入 | 在创建时注入初始 prompt | P0 |
| Agent 监控 | 监控 Agent 状态（运行中/完成/失败） | P0 |
| Agent 终止 | 主动终止和回收 Agent | P1 |
| Agent 间通信 | Sub 向 Main 发送消息 | P1 |

### 7.2 验证计划

**第一步**: 调研 Claude Code
```bash
# 查阅 Claude Code 文档
# 搜索关键词: "agent", "sub-agent", "task", "delegation"
# 联系 Anthropic 团队询问 roadmap
```

**第二步**: 原型验证
```bash
# 手动模拟 Sub-Agent 模式
# Main Agent: 分解任务，记录到 Intent-Engine
# 手动启动新会话，模拟 Sub-Agent
# 测试上下文注入是否足够
```

**第三步**: 技术实现
- 如果 Claude Code 支持 → 使用官方 API
- 如果不支持 → 考虑自建 `ie agent` 命令包装

---

## 8. 备选方案：`ie agent` 命令包装

如果宿主环境不支持 Sub-Agent 管理，Intent-Engine 可以自己提供一个包装命令：

```bash
# Main Agent 调用
ie agent spawn --task-id 43

# Intent-Engine 内部:
# 1. 调用宿主环境的"新建会话"API（如果有）
# 2. 或者，生成一个"启动脚本"供用户手动执行
# 3. 或者，等待未来的 MCP Sub-Agent 协议

# 输出:
Sub-Agent context for Task #43:
---
[上下文内容]
---

Next steps:
1. Start a new session/agent
2. Paste the above context as initial prompt
3. Work on Task #43 until completion
4. Call `ie task done`
```

---

## 9. 优势与挑战

### 9.1 优势

- ✅ **彻底解决长上下文衰减**: 每个 Agent 只工作 10-30 轮
- ✅ **专注力最强**: Agent 的整个上下文都是关于单一任务
- ✅ **天然隔离**: 任务间不会互相干扰
- ✅ **可并行**: 理论上可以同时运行多个 Sub-Agent（高级特性）
- ✅ **对 Intent-Engine 零侵入**: 只是改变了使用方式
- ✅ **支持跨会话**: 通过 Intent-Engine 的持久化状态

### 9.2 挑战

- ⚠️ **宿主环境支持**: 需要 Claude Code 或类似平台提供 Agent 管理能力
- ⚠️ **Main Agent 复杂度**: 需要承担调度和协调职责
- ⚠️ **用户体验**: 多个 Agent 切换可能让用户感到困惑
- ⚠️ **成本**: 每个 Sub-Agent 都是独立的会话（但对话轮数更少）
- ⚠️ **调试难度**: 跨 Agent 的问题诊断更复杂

---

## 10. 成功指标

### 10.1 技术指标

- **上下文长度**: Sub-Agent 的平均对话轮数应 < 30 轮
- **任务完成率**: Sub-Agent 专注模式下的任务完成率 > 单 Agent 模式
- **记忆准确性**: Sub-Agent 在整个生命周期内 0% 遗忘率

### 10.2 用户体验指标

- **任务质量**: 复杂任务的实现质量是否提升
- **上下文清晰度**: 用户是否感觉 AI 更"专注"
- **跨会话连续性**: 恢复工作时是否无缝衔接

---

## 11. 实施路线图

### 阶段 1: 技术验证（2-4周）
- [ ] 调研 Claude Code 的 Agent 管理能力
- [ ] 手动原型验证（模拟 Sub-Agent 模式）
- [ ] 评估可行性，决定是否继续

### 阶段 2: 最小可行方案（4-6周）
- [ ] 实现 Main Agent 的任务分解逻辑
- [ ] 实现上下文注入模板生成
- [ ] 如果无官方支持，实现 `ie agent` 包装命令

### 阶段 3: 完整实现（6-8周）
- [ ] 完整的 Agent 生命周期管理
- [ ] Agent 间通信协议
- [ ] 跨会话状态恢复
- [ ] 文档和最佳实践

---

## 12. 设计哲学

**核心原则**: 用架构设计解决认知限制

AI 的长上下文记忆衰减是一个**认知特性**，不是缺陷。与其对抗它，不如接受它，并设计一个架构来适应它。

**Sub-Agent 模式的本质**:
- 不是"修复" AI 的记忆问题
- 而是**避免产生记忆问题**
- 通过短生命周期、单一职责的 Agent 设计
- 让每个 Agent 都工作在最佳认知状态

**与 Intent-Engine 的协同**:
- Intent-Engine 提供持久化的战略记忆
- Sub-Agent 提供短期的、高专注度的战术执行
- Main Agent 连接两者，提供协调和决策

---

**文档版本**: 1.0
**最后更新**: 2025-11-13
**状态**: 设计提案 - 需要 Phase 1 验证后再启动
**前置条件**: Speckit Guardian Phase 1 (焦点恢复) 必须先实施
