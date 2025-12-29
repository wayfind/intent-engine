# Intent-Engine 快速上手指南

**中文 | [English](../../en/guide/quickstart.md)**

**5 分钟体验 Intent-Engine 的核心功能。**

---

## 前提条件

- Rust 和 Cargo（[安装指南](https://rustup.rs/)）
- 或从 [releases](https://github.com/wayfind/intent-engine/releases) 下载预编译二进制

---

## 第 1 步：安装（1 分钟）

```bash
# 方式 1：使用 Cargo（推荐）
cargo install intent-engine

# 方式 2：使用 Homebrew（macOS/Linux）
brew install wayfind/tap/intent-engine

# 方式 3：使用 npm
npm install -g @origintask/intent-engine

# 验证安装
ie --version
```

---

## 第 2 步：创建第一个任务（1 分钟）

```bash
# 创建带描述（spec）的任务
echo '{"tasks":[{
  "name": "实现用户认证",
  "status": "doing",
  "spec": "## 目标\n用户可通过 JWT 令牌认证\n\n## 方案\n- 使用 HS256 算法\n- 令牌有效期 7 天\n- 支持刷新令牌"
}]}' | ie plan

# 输出：
# ✓ Plan executed successfully
# Created: 1 tasks
# Task ID mapping:
#   实现用户认证 → #1
```

**发生了什么？**
- Intent-Engine 在当前目录自动初始化
- 创建了 `.intent-engine/project.db`（SQLite 数据库）
- 任务已保存，包含完整规格说明
- 任务被设为当前焦点（status: doing）

> **注意**：`status: doing` 的任务必须有 `spec`（描述）。这确保你在开始工作前知道目标。

---

## 第 3 步：查看当前状态（30 秒）

```bash
ie status

# 输出显示：
# - 当前聚焦的任务
# - 任务规格说明
# - 父子关系（如果有）
# - 事件历史
```

**这是"失忆恢复"命令** - 每次会话开始时运行。

---

## 第 4 步：分解为子任务（1 分钟）

```bash
# 给当前任务添加子任务
echo '{"tasks":[
  {"name": "设计 JWT 令牌结构", "status": "todo"},
  {"name": "实现令牌验证", "status": "todo"},
  {"name": "添加刷新机制", "status": "todo"}
]}' | ie plan

# 子任务自动添加到聚焦的父任务下
```

**发生了什么？**
- 在父任务（#1）下创建了 3 个子任务
- 自动归属：新任务成为聚焦任务的子任务
- 使用 `"parent_id": null` 创建独立的根任务

---

## 第 5 步：记录决策（30 秒）

```bash
# 记录你做选择的原因
ie log decision "选择 HS256 而非 RS256 - 单应用场景，不需要非对称加密"

# 输出：
# ✓ Event recorded
#   Type: decision
#   Task: #1
```

**决策日志是给未来 AI 的消息**（包括失忆的未来自己）。

其他事件类型：`blocker`、`milestone`、`note`

---

## 第 6 步：完成子任务（30 秒）

```bash
# 开始处理子任务
echo '{"tasks":[{"name": "设计 JWT 令牌结构", "status": "doing", "spec": "定义令牌结构和声明"}]}' | ie plan

# ... 执行工作 ...

# 标记完成
echo '{"tasks":[{"name": "设计 JWT 令牌结构", "status": "done"}]}' | ie plan
```

**关键规则**：父任务必须等所有子任务完成后才能标记为 `done`。

---

## 第 7 步：搜索历史（30 秒）

```bash
# 查找未完成的任务
ie search "todo doing"

# 按内容搜索
ie search "JWT 认证"

# 查找最近的决策
ie search "decision"
```

---

## 恭喜！

你已经学会了 Intent-Engine 的核心工作流：

1. **ie status** - 恢复上下文（总是第一步）
2. **ie plan** - 创建、更新、完成任务（JSON 标准输入）
3. **ie log** - 记录决策和事件
4. **ie search** - 搜索任务和历史

---

## 命令速查

| 命令 | 用途 | 示例 |
|------|------|------|
| `ie status` | 查看当前上下文 | `ie status` 或 `ie status 42` |
| `ie plan` | 任务操作 | `echo '{"tasks":[...]}' \| ie plan` |
| `ie log <类型> <消息>` | 记录事件 | `ie log decision "选择了 X"` |
| `ie search <查询>` | 搜索 | `ie search "todo doing"` |

---

## 下一步

### 进阶功能

1. **层级任务**：在 JSON 中使用 `children` 创建嵌套结构
2. **优先级**：添加 `"priority": "high"`（critical/high/medium/low）
3. **仪表盘**：运行 `ie dashboard start` 在 `localhost:11391` 查看可视化界面

### 文档

- [CLAUDE.md](../../../CLAUDE.md) - 给 AI 助手（理解"为什么"）
- [命令参考](command-reference-full.md) - 所有命令详解
- [系统提示词指南](../integration/claude-code-system-prompt.md) - Claude Code 集成

---

## 常见问题

**Q：和普通待办应用有什么区别？**

A：Intent-Engine 追踪**战略意图**（What + Why），而不仅仅是任务。每个任务都有规格说明、决策历史和层级关系 - 这是 AI 的外部长期记忆。

**Q：数据存储在哪里？**

A：`.intent-engine/project.db`（SQLite），位于你首次运行命令的目录。

**Q：为什么 `doing` 任务必须有 spec？**

A：开始工作前应该知道目标和方法。这防止"在做某事"却不清楚在做什么。

---

**开始使用 Intent-Engine - 给你的 AI 它应得的记忆！**
