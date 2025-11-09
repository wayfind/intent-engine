# Intent-Engine

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wayfind/intent-engine/branch/main/graph/badge.svg)](https://codecov.io/gh/wayfind/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![Documentation](https://docs.rs/intent-engine/badge.svg)](https://docs.rs/intent-engine)

Intent-Engine 是一个极简的、项目专属的命令行数据库服务，专门用于记录、追踪和回顾人类的战略意图。它是 AI 协作者工具箱中的核心动力，帮助回答"我们要去哪里？(What)"和"为什么要去那里？(Why)"这两个关键问题。

> 📖 **新用户？** 推荐先阅读 [The Intent-Engine Way](docs/zh-CN/guide/the-intent-engine-way.md)，了解 Intent-Engine 的设计哲学和协作模式。本文档是技术参考，那份指南解释"为什么"和"何时"使用。

## 核心特性

- **项目感知**: 自动向上查找 `.intent-engine` 目录，感知项目根目录
- **惰性初始化**: 写入命令自动初始化项目，无需手动 init
- **任务管理**: 支持任务的增删改查、层级关系、状态跟踪
  - **优先级和复杂度**: 支持任务评估和排序 🆕
  - **智能推荐**: `pick-next` 基于上下文推荐下一个任务 🆕
  - **子任务管理**: `spawn-subtask` 原子创建并切换 🆕
  - **任务切换**: `switch` 在多任务间灵活切换 🆕
- **事件日志**: 记录任务相关的决策、讨论和里程碑
- **工作区状态**: 跟踪当前正在处理的任务
- **智能报告**: 支持 FTS5 全文搜索和时间范围过滤
- **Token 优化**: 原子操作减少 60-70% API 调用 🆕
- **JSON 输出**: 所有输出均为结构化 JSON，便于 AI 和工具集成

## 安装

> 📖 **完整安装指南**: 查看 [INSTALLATION.md](docs/zh-CN/guide/installation.md) 了解所有安装方式的详细说明、故障排除和维护者发布流程。

### 方式 1: Cargo Install（推荐）🚀

如果你已经安装了 Rust 和 Cargo，这是最简单的安装方式：

```bash
# 从 crates.io 安装最新版本
cargo install intent-engine

# 验证安装
intent-engine --version
```

**没有 Rust？** 先安装 Rust：
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 方式 2: Homebrew（macOS/Linux）🍺

```bash
# 即将支持
brew install wayfind/tap/intent-engine
```

### 方式 3: cargo-binstall（快速安装预编译二进制）⚡

使用 [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) 直接安装预编译二进制，比从源码编译快得多：

```bash
# 安装 cargo-binstall（如果还没有）
cargo install cargo-binstall

# 安装 intent-engine（自动从 GitHub Releases 下载）
cargo binstall intent-engine
```

### 方式 4: 下载预编译二进制

从 [GitHub Releases](https://github.com/wayfind/intent-engine/releases) 下载适合你平台的二进制文件：

- **Linux**: `intent-engine-linux-x86_64.tar.gz` 或 `intent-engine-linux-aarch64.tar.gz`
- **macOS**: `intent-engine-macos-x86_64.tar.gz` 或 `intent-engine-macos-aarch64.tar.gz`
- **Windows**: `intent-engine-windows-x86_64.zip`

```bash
# 解压并安装
tar xzf intent-engine-*.tar.gz
sudo mv intent-engine /usr/local/bin/

# 验证安装
intent-engine --version
```

### 方式 5: 从源码构建

```bash
# 克隆仓库
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine

# 构建并安装
cargo install --path .

# 或者手动构建
cargo build --release
sudo cp target/release/intent-engine /usr/local/bin/
```

### 方式 6: 作为 MCP Server 集成到 Claude Code

Intent-Engine 可以作为 MCP (Model Context Protocol) server 集成到 Claude Code，提供原生工具支持。

```bash
# 自动安装 MCP server
./scripts/install/install-mcp-server.sh

# 重启 Claude Code
```

详细配置说明请参见 [MCP_SETUP.md](docs/zh-CN/integration/mcp-server.md)。

### 方式 7: 作为 Claude Code Skill

对于轻量级集成，可以将 Intent-Engine 配置为 Claude Code skill：

```bash
# Skill 配置文件已包含在项目中
# 位置：.claude-code/intent-engine.skill.md
# Claude Code 会自动识别
```

## 快速开始

### 典型工作流程

```bash
# 1. 添加主任务
intent-engine task add --name "实现用户认证功能" | jq -r '.id'
# 输出: 1

# 2. 开始任务并查看详情
intent-engine task start 1 --with-events

# 3. 发现问题，创建子任务
intent-engine task spawn-subtask --name "修复密码验证 bug"

# 4. 记录关键决策
echo "决定使用 bcrypt 替代 MD5" | intent-engine event add --task-id 2 --type decision --data-stdin

# 5. 完成子任务（子任务已是焦点，直接完成）
intent-engine task done

# 6. 切换回父任务
intent-engine task switch 1

# 7. 完成父任务（父任务现在是焦点，直接完成）
intent-engine task done

# 8. 生成工作报告
intent-engine report --since 1d --summary-only
```

## 命令参考

### 任务管理命令

#### `task add` - 添加任务

创建新任务，支持指定父任务和规格说明。

**用法:**
```bash
intent-engine task add --name <NAME> [OPTIONS]
```

**参数:**
- `--name <NAME>` - 任务名称（必需）
- `--parent <ID>` - 父任务 ID（可选）
- `--spec-stdin` - 从标准输入读取规格说明（可选）

**示例:**
```bash
# 添加简单任务
intent-engine task add --name "实现用户登录"

# 添加带规格说明的任务
echo "使用 JWT token，有效期 7 天，支持刷新" | \
  intent-engine task add --name "JWT 认证" --spec-stdin

# 添加子任务
intent-engine task add --name "编写单元测试" --parent 1

# 从文件读取规格
cat design.md | intent-engine task add --name "设计评审" --spec-stdin
```

**输出示例:**
```json
{
  "id": 1,
  "parent_id": null,
  "name": "实现用户登录",
  "status": "todo",
  "priority": 0,
  "first_todo_at": "2025-11-06T10:00:00Z",
  "first_doing_at": null,
  "first_done_at": null
}
```

---

#### `task find` - 查找任务

查找任务，支持按状态、父任务筛选。

**用法:**
```bash
intent-engine task find [OPTIONS]
```

**参数:**
- `--status <STATUS>` - 按状态筛选：todo/doing/done（可选）
- `--parent <PARENT>` - 按父任务筛选：任务 ID 或 "null"（可选）

**示例:**
```bash
# 查找所有任务
intent-engine task find

# 查找正在进行的任务
intent-engine task find --status doing

# 查找已完成的任务
intent-engine task find --status done

# 查找特定父任务的所有子任务
intent-engine task find --parent 1

# 查找所有根任务（无父任务）
intent-engine task find --parent null

# 组合查询：查找任务 1 下正在进行的子任务
intent-engine task find --parent 1 --status doing
```

**输出示例:**
```json
[
  {
    "id": 1,
    "parent_id": null,
    "name": "实现用户登录",
    "status": "doing",
    "priority": 5,
    "complexity": 7,
    "first_todo_at": "2025-11-06T10:00:00Z",
    "first_doing_at": "2025-11-06T10:30:00Z",
    "first_done_at": null
  },
  {
    "id": 2,
    "parent_id": 1,
    "name": "编写单元测试",
    "status": "todo",
    "priority": 3,
    "first_todo_at": "2025-11-06T11:00:00Z",
    "first_doing_at": null,
    "first_done_at": null
  }
]
```

---

#### `task get` - 获取任务详情

获取单个任务的详细信息，可选包含关联事件摘要。

**用法:**
```bash
intent-engine task get <ID> [OPTIONS]
```

**参数:**
- `<ID>` - 任务 ID（必需）
- `--with-events` - 包含事件摘要（可选）

**示例:**
```bash
# 获取基本信息
intent-engine task get 1

# 获取包含事件摘要的完整信息
intent-engine task get 1 --with-events

# 使用 jq 提取特定字段
intent-engine task get 1 | jq -r '.name'
intent-engine task get 1 --with-events | jq '.events_summary'
```

**输出示例（不带事件）:**
```json
{
  "id": 1,
  "parent_id": null,
  "name": "实现用户登录",
  "spec": "使用 JWT token，有效期 7 天",
  "status": "doing",
  "complexity": 7,
  "priority": 5,
  "first_todo_at": "2025-11-06T10:00:00Z",
  "first_doing_at": "2025-11-06T10:30:00Z",
  "first_done_at": null
}
```

**输出示例（带事件）:**
```json
{
  "task": {
    "id": 1,
    "name": "实现用户登录",
    "status": "doing",
    "..."
  },
  "events_summary": {
    "total_count": 3,
    "by_type": {
      "decision": 2,
      "blocker": 1
    },
    "recent_events": [
      {
        "id": 3,
        "log_type": "decision",
        "discussion_data": "决定使用 bcrypt 替代 MD5",
        "timestamp": "2025-11-06T11:00:00Z"
      }
    ]
  }
}
```

---

#### `task update` - 更新任务

更新任务的属性，包括名称、父任务、状态、复杂度和优先级。

**用法:**
```bash
intent-engine task update <ID> [OPTIONS]
```

**参数:**
- `<ID>` - 任务 ID（必需）
- `--name <NAME>` - 新名称（可选）
- `--parent <PARENT_ID>` - 新父任务 ID（可选）
- `--status <STATUS>` - 新状态：todo/doing/done（可选）
- `--complexity <1-10>` - 任务复杂度 1-10（可选）
- `--priority <N>` - 任务优先级，数值越大越优先（可选）
- `--spec-stdin` - 从标准输入读取新规格说明（可选）

**示例:**
```bash
# 更新任务名称
intent-engine task update 1 --name "实现 OAuth2 登录"

# 设置任务复杂度和优先级
intent-engine task update 1 --complexity 8 --priority 10

# 更新任务状态
intent-engine task update 1 --status doing

# 更改父任务
intent-engine task update 3 --parent 2

# 更新规格说明
echo "新的实现方案：使用 OAuth2 + PKCE" | \
  intent-engine task update 1 --spec-stdin

# 组合更新
intent-engine task update 1 \
  --name "优化登录性能" \
  --complexity 5 \
  --priority 8 \
  --status doing
```

**输出示例:**
```json
{
  "id": 1,
  "parent_id": null,
  "name": "实现 OAuth2 登录",
  "status": "doing",
  "complexity": 8,
  "priority": 10,
  "first_todo_at": "2025-11-06T10:00:00Z",
  "first_doing_at": "2025-11-06T10:30:00Z",
  "first_done_at": null
}
```

---

#### `task start` - 开始任务

原子操作：将任务状态更新为 "doing" 并设置为当前任务。

**用法:**
```bash
intent-engine task start <ID> [OPTIONS]
```

**参数:**
- `<ID>` - 任务 ID（必需）
- `--with-events` - 包含事件摘要（可选）

**示例:**
```bash
# 开始任务
intent-engine task start 1

# 开始任务并获取历史上下文
intent-engine task start 1 --with-events

# 典型 AI 工作流：开始任务前了解背景
intent-engine task start 1 --with-events | jq '.events_summary.recent_events'
```

**输出示例:**
```json
{
  "id": 1,
  "name": "实现用户登录",
  "status": "doing",
  "first_doing_at": "2025-11-06T10:30:00Z",
  "..."
}
```

---

#### `task done` - 完成当前焦点任务

原子性地完成当前焦点任务。此命令不接受 ID 参数，其操作目标永远是 `current_task_id` 所指向的任务。

**用法:**
```bash
intent-engine task done
```

**参数:**
- 无参数（只对当前焦点任务生效）

**前置条件:**
- 必须有任务被设置为焦点（通过 `current --set <ID>` 或 `task start/switch` 命令）
- 当前任务的所有子任务必须已完成

**行为:**
在一个事务中完成以下操作：
1. 检查当前焦点任务的所有子任务是否为 done
2. 将当前焦点任务的状态更新为 done
3. 清空 `current_task_id`，使工作区返回"未聚焦"状态

**工作流:**
完成一个非焦点任务的标准流程：
```bash
intent-engine current --set <ID>  # 设置焦点
intent-engine task done           # 完成焦点任务
```

**示例:**
```bash
# 1. 设置任务为焦点
intent-engine current --set 1

# 2. 完成任务
intent-engine task done

# 3. 如果有未完成的子任务，会返回错误
intent-engine current --set 2
intent-engine task done
# 错误: UNCOMPLETED_CHILDREN
```

**输出示例:**
```json
{
  "completed_task": {
    "id": 1,
    "name": "实现用户登录",
    "status": "done",
    "first_done_at": "2025-11-06T12:00:00Z"
  },
  "workspace_status": {
    "current_task_id": null
  },
  "next_step_suggestion": {
    "type": "PARENT_IS_READY",
    "message": "All sub-tasks of parent #5 'User Authentication' are now complete. The parent task is ready for your attention.",
    "parent_task_id": 5,
    "parent_task_name": "User Authentication"
  }
}
```

**Next Step Suggestion 类型:**

- **PARENT_IS_READY**: 父任务的所有子任务都已完成，父任务已准备就绪
- **SIBLING_TASKS_REMAIN**: 父任务还有其他未完成的子任务
- **TOP_LEVEL_TASK_COMPLETED**: 完成了一个有子任务的顶级任务
- **NO_PARENT_CONTEXT**: 完成了一个独立的任务（还有其他任务待完成）
- **WORKSPACE_IS_CLEAR**: 所有任务都已完成，项目完成

---

#### `task del` - 删除任务

删除任务及其所有子任务（级联删除）。

**用法:**
```bash
intent-engine task del <ID>
```

**参数:**
- `<ID>` - 任务 ID（必需）

**示例:**
```bash
# 删除任务
intent-engine task del 1

# 删除会级联到所有子任务
intent-engine task del 1  # 同时删除任务 1 及其所有子任务
```

**输出示例:**
```json
{
  "success": true,
  "message": "Task 1 deleted"
}
```

---

#### `task pick-next` - 智能推荐下一个任务 🆕

基于上下文感知的优先级模型，智能推荐当前最应该处理的单个任务。该命令是非交互式的，不会修改任务状态。

**核心哲学**: 深度优先地完成当前正在进行的主题，然后再开启新的主题。

**用法:**
```bash
intent-engine task pick-next [--format <FORMAT>]
```

**参数:**
- `--format <FORMAT>` - 输出格式（默认：`text`）
  - `text`: 人类友好的引导格式
  - `json`: 结构化的 JSON 格式，适合 AI Agent

**智能推荐逻辑:**
1. **第一优先级**: 当前焦点任务的子任务（深度优先）
   - 查找 `current_task_id` 的所有 `status=todo` 的子任务
   - 按 `priority ASC`（数字越小优先级越高）、`id ASC` 排序
2. **第二优先级**: 顶级任务（广度优先）
   - 查找所有 `parent_id IS NULL` 且 `status=todo` 的任务
   - 按 `priority ASC`、`id ASC` 排序
3. **无推荐**: 返回适当的空状态响应，退出码为 1

**示例:**

```bash
# Text 格式（默认）- 人类友好
intent-engine task pick-next

# 输出示例：
# Based on your current focus, the recommended next task is:
#
# [ID: 43] [Priority: 1] [Status: todo]
# Name: Design database schema for user identities
#
# To start working on it, run:
#   ie task start 43

# JSON 格式 - AI Agent 友好
intent-engine task pick-next --format json
```

**JSON 输出示例（有推荐）:**
```json
{
  "suggestion_type": "FOCUSED_SUB_TASK",
  "task": {
    "id": 43,
    "parent_id": 4,
    "name": "Design database schema for user identities",
    "spec": "详细规范内容...",
    "status": "todo",
    "priority": 1,
    "complexity": null,
    "first_todo_at": "2025-11-08T10:30:00Z",
    "first_doing_at": null,
    "first_done_at": null
  }
}
```

**JSON 输出示例（空状态 - 项目为空）:**
```json
{
  "suggestion_type": "NONE",
  "reason_code": "NO_TASKS_IN_PROJECT",
  "message": "No tasks found in this project. Your intent backlog is empty."
}
```

**JSON 输出示例（空状态 - 全部完成）:**
```json
{
  "suggestion_type": "NONE",
  "reason_code": "ALL_TASKS_COMPLETED",
  "message": "Project Complete! All intents have been realized."
}
```

**建议类型:**
- `FOCUSED_SUB_TASK` - 推荐当前焦点任务的子任务
- `TOP_LEVEL_TASK` - 推荐顶级任务
- `NONE` - 无推荐（配合 reason_code 说明原因）

**退出码:**
- `0` - 成功找到推荐任务
- `1` - 无推荐（空状态）

**使用场景:**
- AI Agent 在每次工作开始时获取下一个应该处理的任务
- 人类用户查看系统推荐的下一步工作
- 自动化脚本基于推荐任务进行决策

---

#### `task spawn-subtask` - 创建子任务并切换 🆕

在当前任务下创建子任务，并自动切换到新子任务（原子操作）。

**用法:**
```bash
intent-engine task spawn-subtask --name <NAME> [OPTIONS]
```

**参数:**
- `--name <NAME>` - 子任务名称（必需）
- `--spec-stdin` - 从标准输入读取规格说明（可选）

**前置条件:**
- 必须有当前任务（通过 `current --set` 或 `task start` 设置）

**原子操作流程:**
1. 检查当前任务
2. 创建子任务（parent_id = current_task_id）
3. 将子任务状态设为 doing
4. 设置子任务为当前任务

**示例:**
```bash
# 1. 先开始一个父任务
intent-engine task start 1

# 2. 在工作中发现需要处理子问题
intent-engine task spawn-subtask --name "修复依赖版本冲突"

# 3. 带规格说明的子任务
echo "需要升级 tokio 到 1.35" | \
  intent-engine task spawn-subtask --name "升级依赖" --spec-stdin

# 典型场景：递归问题分解
intent-engine task start 1  # 开始：实现用户认证（自动成为焦点）
intent-engine task spawn-subtask --name "实现密码加密"  # 发现子问题（自动切换为焦点）
intent-engine task spawn-subtask --name "选择加密算法"  # 又发现更细的子问题（自动切换为焦点）
intent-engine task done  # 完成：选择加密算法（当前焦点）
intent-engine task switch 2  # 切回：实现密码加密
intent-engine task done  # 完成：实现密码加密（当前焦点）
intent-engine task switch 1  # 切回：实现用户认证
intent-engine task done  # 完成：实现用户认证（当前焦点）
```

**输出示例:**
```json
{
  "id": 2,
  "parent_id": 1,
  "name": "修复依赖版本冲突",
  "status": "doing",
  "priority": 0,
  "first_todo_at": "2025-11-06T10:35:00Z",
  "first_doing_at": "2025-11-06T10:35:00Z",
  "first_done_at": null
}
```

**使用场景:**
- AI 在处理任务时发现需要先解决的子问题
- 保持任务层级清晰，避免平铺所有任务
- 强制完成子任务后才能完成父任务

---

#### `task switch` - 切换任务 🆕

原子操作：将任务状态更新为 doing（如果不是）并设置为当前任务。

**用法:**
```bash
intent-engine task switch <ID>
```

**参数:**
- `<ID>` - 任务 ID（必需）

**原子操作流程:**
1. 验证任务存在
2. 如果状态不是 doing，更新为 doing
3. 设置为当前任务
4. 返回任务详情和事件摘要

**示例:**
```bash
# 切换到任务 2
intent-engine task switch 2

# 在多个任务间切换
intent-engine task start 1
intent-engine task spawn-subtask --name "子任务 A"
intent-engine task spawn-subtask --name "子任务 B"
intent-engine task switch 2  # 切回子任务 A
intent-engine task done  # 完成当前焦点任务（子任务 A）
intent-engine task switch 3  # 切到子任务 B

# 查看切换后的上下文
intent-engine task switch 5 | jq '.events_summary'
```

**输出示例:**
```json
{
  "task": {
    "id": 2,
    "parent_id": 1,
    "name": "实现密码加密",
    "status": "doing",
    "first_doing_at": "2025-11-06T10:40:00Z",
    "..."
  },
  "events_summary": {
    "total_count": 2,
    "by_type": {
      "decision": 1,
      "milestone": 1
    },
    "recent_events": [...]
  }
}
```

**使用场景:**
- 在多个并行任务间切换
- 暂停当前任务去处理更紧急的任务
- 完成子任务后切回父任务

---

#### `task search` - 全文搜索任务 🆕

使用 FTS5 全文搜索在所有任务的 name 和 spec 字段中查找内容，返回按相关性排序的结果列表。

**用法:**
```bash
intent-engine task search <QUERY>
```

**参数:**
- `<QUERY>` - 搜索查询字符串（必需），支持 FTS5 高级语法

**FTS5 高级语法:**
- `authentication` - 简单关键词搜索
- `"user login"` - 精确短语搜索
- `authentication AND bug` - 同时包含两个词
- `JWT OR OAuth` - 包含任一词
- `authentication NOT critical` - 包含 authentication 但不包含 critical
- `auth*` - 前缀匹配（如 auth, authentication, authorize）

**特性:**
- 搜索 name 和 spec 两个字段
- 返回带有高亮片段的结果（使用 `**` 标记关键词）
- 按相关性自动排序
- 毫秒级查询性能（基于 FTS5 索引）

**示例:**
```bash
# 简单搜索
intent-engine task search "authentication"

# 搜索包含 JWT 的任务
intent-engine task search "JWT"

# 高级搜索：同时包含两个关键词
intent-engine task search "authentication AND bug"

# 搜索任一关键词
intent-engine task search "JWT OR OAuth"

# 排除特定关键词
intent-engine task search "bug NOT critical"

# 前缀匹配
intent-engine task search "auth*"

# 精确短语搜索
intent-engine task search '"user login flow"'

# 组合使用 jq 查看结果
intent-engine task search "authentication" | jq '.[].task | {id, name, status}'

# 查看匹配片段
intent-engine task search "JWT" | jq '.[].match_snippet'
```

**输出示例:**
```json
[
  {
    "id": 5,
    "parent_id": 1,
    "name": "Authentication bug fix",
    "spec": "Fix the JWT token validation bug in the authentication middleware",
    "status": "todo",
    "complexity": 5,
    "priority": 8,
    "first_todo_at": "2025-11-06T10:00:00Z",
    "first_doing_at": null,
    "first_done_at": null,
    "match_snippet": "...Fix the **JWT** token validation bug in the **authentication** middleware..."
  },
  {
    "id": 12,
    "parent_id": null,
    "name": "Implement OAuth2 authentication",
    "spec": "Add OAuth2 support for third-party authentication",
    "status": "doing",
    "priority": 10,
    "first_todo_at": "2025-11-05T15:00:00Z",
    "first_doing_at": "2025-11-06T09:00:00Z",
    "first_done_at": null,
    "match_snippet": "Implement OAuth2 **authentication**"
  }
]
```

**match_snippet 字段说明:**
- 从匹配字段（spec 或 name）中提取的文本片段
- 使用 `**关键词**` 标记高亮匹配的词
- 使用 `...` 表示省略的内容
- 优先显示 spec 的匹配，如果 spec 没有匹配则显示 name 的匹配

**使用场景:**
- 快速查找包含特定关键词的任务
- 在大型项目中定位相关任务
- 搜索之前的决策和技术方案
- AI 查找相关上下文时使用
- 代码审查时查找相关任务

**与 `task find` 的区别:**
- `task find`: 精确过滤（按 status、parent），返回完整任务列表
- `task search`: 全文搜索（按内容关键词），返回带匹配片段的结果，按相关性排序

---

### 事件日志命令

#### `event add` - 添加事件

为任务记录事件（决策、障碍、里程碑等）。

**用法:**
```bash
intent-engine event add [--task-id <ID>] --type <TYPE> --data-stdin
```

**参数:**
- `--task-id <ID>` - 任务 ID（可选，如省略则使用当前任务）
- `--type <TYPE>` - 事件类型（必需），建议值：
  - `decision` - 关键决策
  - `blocker` - 遇到的障碍
  - `milestone` - 里程碑
  - `discussion` - 讨论记录
  - `note` - 一般备注
- `--data-stdin` - 从标准输入读取事件内容（必需）

**示例:**
```bash
# 记录到当前任务（简洁工作流）
echo "决定使用 bcrypt 而不是 MD5 进行密码加密" | \
  intent-engine event add --type decision --data-stdin

# 记录到指定任务（灵活工作流）
echo "发现 bcrypt 库在 Windows 上编译失败，需要寻找替代方案" | \
  intent-engine event add --task-id 1 --type blocker --data-stdin

# 记录里程碑到当前任务
echo "完成核心加密逻辑，通过所有单元测试" | \
  intent-engine event add --type milestone --data-stdin

# 从文件记录到指定任务
cat discussion_notes.md | \
  intent-engine event add --task-id 1 --type discussion --data-stdin

# 记录长文本到当前任务
echo "经过调研，比较了以下方案：
1. bcrypt - 业界标准，但 Windows 兼容性差
2. argon2 - 更安全，但性能开销大
3. scrypt - 平衡方案

最终决定：使用 argon2，接受性能开销" | \
  intent-engine event add --type decision --data-stdin
```

**输出示例:**
```json
{
  "id": 1,
  "task_id": 1,
  "timestamp": "2025-11-06T11:00:00Z",
  "log_type": "decision",
  "discussion_data": "决定使用 bcrypt 而不是 MD5 进行密码加密"
}
```

---

#### `event list` - 列出事件

列出指定任务的事件历史。

**用法:**
```bash
intent-engine event list --task-id <ID> [OPTIONS]
```

**参数:**
- `--task-id <ID>` - 任务 ID（必需）
- `--limit <N>` - 限制返回数量（可选，默认返回所有）

**示例:**
```bash
# 列出所有事件
intent-engine event list --task-id 1

# 只看最近 5 条
intent-engine event list --task-id 1 --limit 5

# 只看决策类型的事件
intent-engine event list --task-id 1 | jq '.[] | select(.log_type == "decision")'

# 查看最新的决策
intent-engine event list --task-id 1 --limit 10 | \
  jq '.[] | select(.log_type == "decision") | .discussion_data' | head -1

# AI 恢复上下文时使用
intent-engine event list --task-id 1 --limit 10 | \
  jq '[.[] | {type: .log_type, data: .discussion_data, time: .timestamp}]'
```

**输出示例:**
```json
[
  {
    "id": 3,
    "task_id": 1,
    "timestamp": "2025-11-06T12:00:00Z",
    "log_type": "milestone",
    "discussion_data": "完成核心加密逻辑"
  },
  {
    "id": 2,
    "task_id": 1,
    "timestamp": "2025-11-06T11:30:00Z",
    "log_type": "blocker",
    "discussion_data": "发现 bcrypt 库在 Windows 上编译失败"
  },
  {
    "id": 1,
    "task_id": 1,
    "timestamp": "2025-11-06T11:00:00Z",
    "log_type": "decision",
    "discussion_data": "决定使用 bcrypt 进行密码加密"
  }
]
```

---

### 工作区命令

#### `current` - 当前任务管理

查看或设置当前正在处理的任务。

**用法:**
```bash
# 查看当前任务
intent-engine current

# 设置当前任务
intent-engine current --set <ID>
```

**参数:**
- `--set <ID>` - 设置当前任务 ID（可选）

**示例:**
```bash
# 查看当前任务
intent-engine current

# 设置当前任务
intent-engine current --set 2

# 查看当前任务名称
intent-engine current | jq -r '.task.name'

# 检查是否有当前任务
intent-engine current &>/dev/null && echo "有当前任务" || echo "无当前任务"

# 清除当前任务（目前需要手动操作数据库）
# 注意：通常不需要清除，start/switch/spawn-subtask 会自动更新
```

**输出示例（有当前任务）:**
```json
{
  "current_task_id": 2,
  "task": {
    "id": 2,
    "parent_id": 1,
    "name": "实现密码加密",
    "status": "doing",
    "..."
  }
}
```

**输出示例（无当前任务）:**
```json
{
  "current_task_id": null,
  "message": "No current task"
}
```

---

### 报告命令

#### `report` - 生成工作报告

生成任务工作报告，支持时间范围、状态筛选和全文搜索。

**用法:**
```bash
intent-engine report [OPTIONS]
```

**参数:**
- `--summary-only` - 仅生成摘要（推荐，节省 Token）
- `--since <DURATION>` - 时间范围：1h/6h/1d/7d/30d（可选）
- `--status <STATUS>` - 按状态筛选：todo/doing/done（可选）
- `--filter-name <KEYWORD>` - 按任务名称搜索（FTS5）（可选）
- `--filter-spec <KEYWORD>` - 按规格说明搜索（FTS5）（可选）

**示例:**
```bash
# 生成完整报告
intent-engine report

# 仅生成摘要（推荐）
intent-engine report --summary-only

# 查看最近 1 天的工作
intent-engine report --since 1d --summary-only

# 查看最近 7 天的工作
intent-engine report --since 7d --summary-only

# 查看已完成的任务
intent-engine report --status done --summary-only

# 查看正在进行的任务
intent-engine report --status doing --summary-only

# 搜索包含"认证"的任务
intent-engine report --filter-name "认证" --summary-only

# 搜索规格中包含"JWT"的任务
intent-engine report --filter-spec "JWT" --summary-only

# 组合查询：最近 7 天完成的认证相关任务
intent-engine report --since 7d --status done --filter-name "认证" --summary-only

# AI 生成日报
intent-engine report --since 1d --summary-only | \
  jq -r '.summary | "今日完成 \(.done_count) 个任务，进行中 \(.doing_count) 个"'

# 查看任务详情
intent-engine report --since 7d | jq '.tasks[] | {name, status, started: .first_doing_at}'
```

**输出示例（summary-only）:**
```json
{
  "summary": {
    "total_count": 15,
    "todo_count": 5,
    "doing_count": 3,
    "done_count": 7,
    "time_range": {
      "since": "7d",
      "from": "2025-10-30T10:00:00Z",
      "to": "2025-11-06T10:00:00Z"
    }
  },
  "filters": {
    "status": null,
    "name_keyword": null,
    "spec_keyword": null
  }
}
```

**输出示例（完整报告）:**
```json
{
  "summary": {
    "total_count": 3,
    "todo_count": 1,
    "doing_count": 1,
    "done_count": 1
  },
  "tasks": [
    {
      "id": 1,
      "name": "实现用户认证",
      "status": "done",
      "first_todo_at": "2025-11-06T10:00:00Z",
      "first_doing_at": "2025-11-06T10:30:00Z",
      "first_done_at": "2025-11-06T12:00:00Z"
    },
    {
      "id": 2,
      "name": "编写单元测试",
      "status": "doing",
      "first_todo_at": "2025-11-06T11:00:00Z",
      "first_doing_at": "2025-11-06T11:30:00Z",
      "first_done_at": null
    },
    {
      "id": 3,
      "name": "性能优化",
      "status": "todo",
      "first_todo_at": "2025-11-06T12:00:00Z",
      "first_doing_at": null,
      "first_done_at": null
    }
  ]
}
```

---

## 实际场景示例

### 场景 1：AI 发现多个问题并批量处理

```bash
# 1. AI 在代码审查中发现 5 个问题
intent-engine task add --name "修复空指针异常"
intent-engine task add --name "优化数据库查询"
intent-engine task add --name "更新过期依赖"
intent-engine task add --name "修复内存泄漏"
intent-engine task add --name "添加错误日志"

# 2. AI 评估每个任务的优先级（数字越小越优先）
intent-engine task update 1 --priority 1   # 空指针：最紧急
intent-engine task update 2 --priority 2   # 数据库：第二优先
intent-engine task update 3 --priority 5   # 依赖：中等
intent-engine task update 4 --priority 1   # 内存：最紧急
intent-engine task update 5 --priority 10  # 日志：不紧急

# 3. 获取智能推荐
intent-engine task pick-next --format json
# 结果：会推荐任务 1（priority=1，ID 最小）

# 4. 开始处理推荐的任务
intent-engine task start 1
echo "原因：未检查 null 返回值" | intent-engine event add --task-id 1 --type note --data-stdin
intent-engine task done

# 5. 继续获取下一个推荐
intent-engine task pick-next --format json
# 结果：推荐任务 4（priority=1，ID 第二小）

intent-engine task start 4
echo "决定使用智能指针避免内存泄漏" | intent-engine event add --task-id 4 --type decision --data-stdin
intent-engine task done

# 6. 生成报告
intent-engine report --since 1d --summary-only
```

### 场景 2：递归任务分解

```bash
# 1. 开始一个大任务
intent-engine task add --name "实现支付系统"
intent-engine task start 1 --with-events

# 2. 发现需要先做认证
intent-engine task spawn-subtask --name "集成第三方支付 API"
# 当前任务自动切换到任务 2

# 3. 在集成 API 时发现需要先配置密钥
intent-engine task spawn-subtask --name "配置支付密钥和回调地址"
# 当前任务自动切换到任务 3

# 4. 完成最深层的子任务（子任务 3 当前是焦点）
echo "已在后台配置 Stripe API 密钥" | intent-engine event add --task-id 3 --type milestone --data-stdin
intent-engine task done

# 5. 切回父任务继续
intent-engine task switch 2
echo "API 集成完成，测试通过" | intent-engine event add --task-id 2 --type milestone --data-stdin
intent-engine task done

# 6. 完成根任务
intent-engine task switch 1
intent-engine task done

# 7. 查看任务层级
intent-engine task find --parent null  # 根任务
intent-engine task find --parent 1     # 子任务
```

### 场景 3：并行任务管理

```bash
# 1. 创建多个独立任务
intent-engine task add --name "前端：实现登录页面"
intent-engine task add --name "后端：实现 API 接口"
intent-engine task add --name "文档：更新 API 文档"

# 2. 获取推荐并开始第一个任务
intent-engine task pick-next --format json
# 推荐：任务 1
intent-engine task start 1

# 3. 在任务间切换
# ... 做一些前端工作 ...
echo "完成 UI 布局" | intent-engine event add --task-id 1 --type milestone --data-stdin

intent-engine task switch 2
# ... 做一些后端工作 ...
echo "完成数据库模型" | intent-engine event add --task-id 2 --type milestone --data-stdin

intent-engine task switch 3
# ... 更新文档 ...
intent-engine task done

# 4. 查看进度
intent-engine report --status doing
```

## 项目结构

```
veobd/
├── src/
│   ├── main.rs          # 主入口和命令分发
│   ├── lib.rs           # 库入口
│   ├── cli.rs           # CLI 命令定义
│   ├── error.rs         # 错误类型定义
│   ├── project.rs       # 项目上下文发现
│   ├── tasks.rs         # 任务管理逻辑
│   ├── events.rs        # 事件日志逻辑
│   ├── workspace.rs     # 工作区状态管理
│   ├── report.rs        # 报告生成逻辑
│   ├── test_utils.rs    # 测试工具
│   └── db/
│       ├── mod.rs       # 数据库连接和迁移
│       └── models.rs    # 数据模型定义
├── tests/               # 集成测试
│   ├── cli_tests.rs
│   ├── performance_tests.rs
│   ├── special_chars_tests.rs
│   └── cli_special_chars_tests.rs
├── benches/             # 性能基准测试
│   └── performance.rs
├── Cargo.toml           # 项目配置
├── README.md            # 主文档
├── PERFORMANCE.md       # 性能报告
├── SPECIAL_CHARS.md     # 特殊字符处理文档
└── .intent-engine/      # 项目数据目录（自动创建）
    └── project.db       # SQLite 数据库
```

## 数据库模式

### tasks 表
- `id`: 任务 ID（主键，自增）
- `parent_id`: 父任务 ID（可选，外键）
- `name`: 任务名称（必需）
- `spec`: 任务规格说明（可选）
- `status`: 任务状态（todo/doing/done，默认 todo）
- `complexity`: 任务复杂度（1-10，可选）🆕
- `priority`: 任务优先级（整数，默认 0）🆕
- `first_todo_at`: 首次设为 todo 的时间
- `first_doing_at`: 首次设为 doing 的时间
- `first_done_at`: 首次设为 done 的时间

### events 表
- `id`: 事件 ID
- `task_id`: 关联的任务 ID
- `timestamp`: 事件时间戳
- `log_type`: 事件类型（decision/blocker/milestone 等）
- `discussion_data`: 事件详细内容

### workspace_state 表
- `key`: 状态键（如 current_task_id）
- `value`: 状态值

## AI 客户端使用指南

### 任务生命周期 SOP

#### 基础工作流
1. **开始任务**: 使用 `task start <ID> --with-events` 获取上下文
2. **发现子问题**: 使用 `task spawn-subtask --name "子问题"` 创建并切换
3. **记录关键信息**: 使用 `event add` 记录决策、障碍和里程碑
4. **完成任务**: 使用 `task done` 标记完成（自动检查子任务）

#### 批量问题处理工作流 🆕
1. **发现问题**: 批量创建 todo 任务
2. **评估任务**: 使用 `task update` 设置 priority（数字越小优先级越高）
3. **智能推荐**: 使用 `task pick-next` 获取下一个应该处理的任务
4. **开始任务**: 使用 `task start` 开始推荐的任务
5. **重复**: 完成后再次调用 `pick-next` 获取下一个推荐

### Token 优化策略 🆕

使用新增的原子操作命令可以显著减少 Token 消耗：

| 传统工作流 | Token 消耗 | 优化工作流 | Token 消耗 | 节省 |
|-----------|-----------|-----------|-----------|------|
| find + get | 2 次调用 | `pick-next --format json` | 1 次调用 | **50%** |
| add + start + set current | 3 次调用 | `spawn-subtask` | 1 次调用 | **67%** |
| update + set current + get | 3 次调用 | `switch` | 1 次调用 | **67%** |

### 与原生任务系统的关系

- **Intent-Engine 任务**: 战略意图，粒度粗，生命周期长
- **原生任务 (/todos)**: 战术步骤，粒度细，生命周期短

Intent-Engine 任务驱动原生任务的创建。

### 最佳实践

#### 工作开始时
1. 使用 `task pick-next --format json` 获取推荐任务
2. 使用 `task start <ID> --with-events` 开始推荐的任务
3. 如果发现多个新问题，创建 todo 任务并设置优先级（数字越小越优先）

#### 工作过程中
1. 发现子问题时使用 `spawn-subtask`，保持层级清晰
2. 在做出关键决策时使用 `event add` 记录思考过程
3. 使用 `task switch` 在多个任务间灵活切换

#### 工作结束时
1. 使用 `task done` 完成当前任务
2. 使用 `task pick-next` 获取下一个推荐任务
3. 使用 `report --summary-only` 生成高效总结（节省 Token）

#### 恢复工作时
1. 使用 `current` 查看当前正在处理的任务
2. 使用 `task pick-next` 获取推荐的下一个任务
3. 使用 `task get <ID> --with-events` 获取完整上下文

## 技术栈

- **语言**: Rust 2021
- **CLI**: clap 4.5
- **数据库**: SQLite with sqlx 0.7
- **异步运行时**: tokio 1.35
- **序列化**: serde + serde_json
- **全文搜索**: SQLite FTS5

## 测试

Intent-Engine 包含完整的测试体系，确保代码质量和可靠性。

### 运行测试

```bash
# 运行所有测试
cargo test

# 仅运行单元测试
cargo test --lib

# 仅运行集成测试
cargo test --test cli_tests

# 运行特定测试
cargo test test_add_task
```

### 测试覆盖

- **单元测试** (47 个测试):
  - 任务管理：30 个测试（CRUD、层级、状态管理、循环依赖检测、优先级/复杂度、pick_next、spawn_subtask、switch）
  - 事件日志：6 个测试（添加、列出、过滤）
  - 工作区状态：5 个测试（获取、设置、更新）
  - 报告生成：6 个测试（摘要、全量、过滤、FTS5 搜索）

- **集成测试** (22 个 CLI 测试):
  - 基础 CRUD 操作测试
  - 任务状态转换测试
  - 任务层级和父子关系测试
  - 项目感知和上下文发现测试（4 个）
  - 新增工作流测试：pick-next、spawn-subtask、switch（4 个）
  - JSON 输出格式验证

- **特殊字符测试** (10 个 CLI 测试 + 单元测试):
  - SQL 注入防护测试
  - Unicode 和 Emoji 支持测试
  - 边界情况和极端输入测试

**总计**: 116 个测试全部通过 ✅

### 测试架构

- `src/test_utils.rs`: 测试辅助工具和上下文管理
- `tests/cli_tests.rs`: CLI 集成测试
- 每个模块内部的 `#[cfg(test)]` 模块：单元测试

所有测试使用临时数据库，确保测试隔离和可重复性。

## 性能测试

Intent-Engine 包含完整的性能测试套件，验证系统在极端条件下的表现。

### 运行性能测试

```bash
# 运行所有性能测试（需要较长时间）
cargo test --test performance_tests -- --ignored --nocapture

# 运行特定性能测试
cargo test --test performance_tests test_deep_task_hierarchy -- --ignored --nocapture
cargo test --test performance_tests test_massive_tasks_10k -- --ignored --nocapture

# 运行基准测试
cargo bench --bench performance
```

### 性能指标摘要

- **深度层级**: 支持 100+ 层任务层级，创建 ~343ms，查询 <1ms
- **海量任务**: 10,000 个任务创建 ~33s，查找 ~257ms
- **海量事件**: 单任务 10,000 个事件，限制查询 <32ms
- **FTS5 搜索**: 5,000 个任务中搜索，平均 ~32ms
- **报告生成**: 5,000 任务 summary-only 报告 ~137ms

详细性能报告请参见 [PERFORMANCE.md](docs/zh-CN/technical/performance.md)。

### 性能测试覆盖

- 深度任务层级测试（100、500 层）
- 海量任务测试（10,000、50,000 个任务）
- 海量事件测试（10,000 个事件）
- 宽度层级测试（1,000 个子任务）
- FTS5 全文搜索性能
- 报告生成性能（summary-only vs 完整报告）
- 并发操作测试
- 状态转换压力测试

## 特殊字符和安全性测试

Intent-Engine 经过全面的特殊字符和边界情况测试，确保系统的安全性和鲁棒性。

### 测试覆盖

**安全性测试** (37 个单元测试 + 10 个集成测试):
- ✅ SQL 注入防护（单引号、UNION SELECT、注释符等）
- ✅ Unicode 支持（中文、日文、阿拉伯文、混合语言）
- ✅ Emoji 支持（包括复合 emoji 和国旗）
- ✅ JSON 特殊字符（引号、反斜杠、控制字符）
- ✅ 极端长度输入（10,000+ 字符）
- ✅ 边界情况（空字符串、纯空格、单字符）
- ✅ Shell 元字符、Markdown、HTML 标签
- ✅ URL、文件路径、正则表达式元字符

### 运行测试

```bash
# 运行特殊字符单元测试
cargo test --test special_chars_tests

# 运行 CLI 特殊字符集成测试
cargo test --test cli_special_chars_tests
```

### 安全保证

- **SQL 注入**: 完全防护（使用参数化查询）
- **命令注入**: 不执行外部命令，无风险
- **国际化**: 完全支持 Unicode 和 Emoji
- **数据完整性**: 保持用户输入原始性

详细信息请参见 [SPECIAL_CHARS.md](docs/zh-CN/technical/security.md)。

## 相关文档

Intent-Engine 提供了一系列文档，帮助你从不同角度理解和使用系统：

### 核心文档

- **[AI Quick Guide](docs/zh-CN/guide/ai-quick-guide.md)** - AI 快速参考 ⚡
  - 超级简洁的使用指南
  - 适合作为 system prompt
  - 命令速查表和反模式

- **[The Intent-Engine Way](docs/zh-CN/guide/the-intent-engine-way.md)** - 协作哲学和工作流指南 🌟
  - 何时、如何、为何使用每个命令
  - 完整的工作流示例
  - 核心原则和反模式
  - 推荐新用户首先阅读

- **[README.md](README.md)** (本文档) - 完整的技术参考
  - 所有命令的详细用法
  - 60+ 个实际示例
  - 数据库模式说明

### 集成文档

- **[MCP Setup Guide](docs/zh-CN/integration/mcp-server.md)** - MCP Server 安装指南 🔧
  - Claude Code MCP 集成
  - 自动安装脚本
  - 故障排除

- **[Claude Code Skill](.claude-code/intent-engine.skill.md)** - Skill 配置
  - 轻量级 Claude Code 集成
  - 快速开始示例
  - 常用模式

### 技术文档

- **[Task Workflow Analysis](docs/zh-CN/technical/task-workflow-analysis.md)** - 深度技术分析
  - Token 优化策略详解
  - 11 个测试场景设计
  - 实现细节和 ROI 分析

- **[PERFORMANCE.md](docs/zh-CN/technical/performance.md)** - 性能测试报告
  - 海量数据性能指标
  - 压力测试结果

- **[SPECIAL_CHARS.md](docs/zh-CN/technical/security.md)** - 安全性测试报告
  - SQL 注入防护验证
  - Unicode 和特殊字符支持

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
