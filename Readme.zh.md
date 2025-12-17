# Intent-Engine

**中文 | [English](README.md)**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wayfind/intent-engine/branch/main/graph/badge.svg)](https://codecov.io/gh/wayfind/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)

---

## 这是什么？

**Intent-Engine 是 AI 的外部长期记忆。**

可以理解为你和 AI 助手（比如 Claude）之间的**共享笔记本**：
- 你写下**战略目标**（"做一个登录系统"）
- AI 写下**决策过程**（"选择 JWT 因为..."）
- 你们都能**随时接上之前的工作** —— 哪怕是几天或几周后

```
┌─────────────────────────────────────────────────────┐
│  没有 Intent-Engine                                 │
├─────────────────────────────────────────────────────┤
│  你: "我们来做个登录功能"                            │
│  AI: "好的！"[开始写代码]                           │
│  --- 第二天 ---                                     │
│  你: "继续做登录功能"                               │
│  AI: "什么登录？从头开始吧..."                      │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│  有了 Intent-Engine                                 │
├─────────────────────────────────────────────────────┤
│  你: "我们来做个登录功能"                            │
│  AI: "好的！"[创建任务、写代码、记录决策]           │
│  --- 第二天 ---                                     │
│  你: "继续做登录功能"                               │
│  AI: [读取任务历史]"接着做...我们选了 JWT 方案，     │
│      已经做完基础认证，下一步是 OAuth2 集成"        │
└─────────────────────────────────────────────────────┘
```

---

## 为什么需要这个？

### 🤔 问题所在

AI 助手（Claude、GPT 等）是**无状态**的 —— 对话结束就什么都忘了：
- ❌ 你没法让 AI "继续上周的那个功能"
- ❌ 你看不到 AI 为什么做某些决策
- ❌ 复杂项目变成一堆复制粘贴旧对话的混乱

### ✅ 解决方案

Intent-Engine 给 AI 一个存在你电脑上的**持久化记忆**：
- ✅ AI 记得你在做什么、为什么这么做
- ✅ 每个决策都有完整的上下文记录
- ✅ 你可以暂停工作，随时回来，AI 立刻接上

---

## AI 如何自动工作

### 安装（30 秒）

```bash
# 通过 Cargo 安装（Rust 包管理器）
cargo install intent-engine

# 或者下载预编译的二进制文件
# https://github.com/wayfind/intent-engine/releases

# 验证安装
ie --version
```

### AI 的自动工作流程

**每次进入 Claude Code 时，AI 会自动：**

```bash
# 1. 自动查找未完成的任务
ie search "todo doing"
# → 返回所有待办和进行中的任务及其摘要

# 2. AI 分析并选择重要且优先的任务
# → 根据优先级、依赖关系、上下文决定

# 3. 通过 ie plan 启动任务（plan 蕴含 start）
echo '{"tasks":[{"name":"实现用户认证","status":"doing"}]}' | ie plan
# → 更新任务状态为 doing
# → 自动获取祖先任务的长时记忆（完整上下文）

# 4. 工作过程中记录决策
ie log decision "选择 JWT 而不是 Session，因为需要无状态 API"
ie log note "已完成 token 生成逻辑"

# 5. 完成时更新状态
echo '{"tasks":[{"name":"实现用户认证","status":"done"}]}' | ie plan
```

**如果没有未完成的任务：**
- AI 静默等待你的需求
- 当你提出**大目标或长期需求**时，AI 自动启动 `ie plan` 创建新任务

### 示例：人类提出新需求

```bash
# 你说："帮我做一个用户登录系统"
# AI 判断这是大目标，自动执行：

echo '{
  "tasks": [{
    "name": "做一个用户登录系统",
    "status": "doing",
    "spec": "JWT 令牌、OAuth2 支持、会话 7 天有效期",
    "priority": "high",
    "children": [
      {"name": "实现 JWT 生成和验证", "status": "todo"},
      {"name": "添加 OAuth2 集成", "status": "todo"},
      {"name": "实现会话管理", "status": "todo"}
    ]
  }]
}' | ie plan

# plan 执行后自动：
# ✓ 创建任务树（父任务 + 子任务）
# ✓ 父任务设为 doing 状态（开始工作）
# ✓ 获取完整长时记忆（如果有祖先任务）
# ✓ AI 立即开始实现，并持续记录决策
```

---

## 与 AI 助手集成

### 与 Claude Code 配合（零配置！）

如果你用 **Claude Code**（Anthropic 官方 CLI），Intent-Engine **完全自动化**：

**安装后立即可用：**

1. 安装 Intent-Engine：`cargo install intent-engine`
2. 在项目里启动 Claude Code
3. **无需任何配置！**Claude 会自动：
   - 每次会话开始时执行 `ie search "todo doing"`
   - 发现未完成任务时主动询问是否继续
   - 自动通过 `ie plan` 启动任务并获取长时记忆
   - 工作中持续用 `ie log` 记录关键决策
   - 完成后自动更新任务状态

**工作流示例：**

```
# 场景 1: 有未完成任务
你：[打开 Claude Code]
Claude：[自动执行 ie search "todo doing"]
        "我发现有 3 个待办任务：
         1. 实现用户认证 (todo)
         2. 重构数据库层 (doing)
         3. 修复登录 bug (todo, 优先级高)

         我建议继续 #2，要继续吗？"
你："好的"
Claude：[执行 ie plan 启动任务]
        [获取完整上下文和历史]
        "好，我看到之前选择了 Repository 模式...让我继续实现"

# 场景 2: 没有任务
你：[打开 Claude Code] "帮我实现一个 REST API"
Claude：[判断这是大目标]
        [自动执行 ie plan 创建任务树]
        "我创建了任务'实现 REST API'，包含 4 个子任务，现在开始..."
```

📖 [Claude Code 集成详解](docs/zh-CN/integration/claude-code-system-prompt.md)

### 与任何 AI 工具配合

Intent-Engine 只是个 CLI 工具 —— 任何能运行命令的 AI 都能用：
- Gemini CLI
- 自定义 GPT 代理
- Cursor AI
- 任何有 bash 权限的聊天机器人

📖 [通用集成指南](docs/zh-CN/integration/generic-llm.md)

---

## 核心功能

### 🌳 层级任务树

像你思考问题一样，把大问题拆成小问题：

```
做一个登录系统
├── 实现 JWT
│   ├── 生成令牌
│   └── 验证令牌
└── 添加 OAuth2
    ├── Google 登录
    └── GitHub 登录
```

### 📝 决策历史（事件流）

每个"为什么"都被记录下来：

```bash
ie log decision "选择 PostgreSQL 而不是 MongoDB，因为需要 ACID 保证"
ie log blocker "等待团队的设计审批"
ie log milestone "MVP 完成，可以测试了"
```

### 🎯 声明式工作流：ie plan 是核心

**ie plan 不仅仅是创建任务 —— 它也是启动任务的方式**

```bash
# plan 的三重作用：

# 1️⃣ 创建新任务
echo '{"tasks":[{"name":"新任务","spec":"..."}]}' | ie plan

# 2️⃣ 更新已有任务（幂等性）
echo '{"tasks":[{"name":"新任务","spec":"更新后的内容"}]}' | ie plan

# 3️⃣ 启动任务 = 设置 status="doing"
echo '{"tasks":[{"name":"新任务","status":"doing"}]}' | ie plan
# ✓ 任务状态变为 doing
# ✓ 自动获取祖先任务信息（长时记忆）
# ✓ AI 获得完整上下文开始工作
```

**完整工作流程**：

```bash
# 会话开始：AI 自动查找任务
ie search "todo doing"

# 如果找到任务：AI 通过 plan 启动
echo '{"tasks":[{"name":"已存在的任务","status":"doing"}]}' | ie plan

# 如果没有任务：等待新需求，然后 plan 创建并启动
echo '{
  "tasks": [{
    "name": "新的大目标",
    "status": "doing",  # 直接创建为 doing 状态
    "children": [...]
  }]
}' | ie plan

# 工作中：记录决策
ie log decision "选择方案 A 因为性能更好"

# 完成：更新状态
echo '{"tasks":[{"name":"任务名","status":"done"}]}' | ie plan
```

### 📊 进度报告

查看完成了什么：

```bash
ie search "登录"    # 查找所有登录相关的工作
```

---

## Intent-Engine 有什么不同？

### vs. Claude Code TodoWriter

| 功能 | Intent-Engine | TodoWriter |
|------|--------------|------------|
| **持久化** | 保存到磁盘，永不丢失 | 对话结束就丢失 |
| **决策历史** | 完整事件日志，带推理过程 | 没有历史 |
| **AI 恢复工作** | 可以 - 加载完整上下文 | 不行 - 从头开始 |
| **跨会话** | 支持 | 不支持 |
| **最适合** | 战略性、多天的工作 | 当前会话的笔记 |

### vs. Jira / Linear / Asana

| 功能 | Intent-Engine | 项目管理工具 |
|------|--------------|-------------|
| **AI 集成** | 原生 CLI，JSON 输出 | 只有网页界面（手动） |
| **决策追踪** | 结构化事件流 | 非结构化评论 |
| **自动化** | AI 可以创建/更新任务 | 需要手动输入 |
| **重点** | 战略"为什么" + 技术规格 | 战术"什么时候" + 任务分配 |
| **最适合** | 人机协作 | 人类团队协调 |

---

## 实际使用场景

### ✅ 多天开发项目

**问题**：你和 AI 一起开发复杂功能，分多次会话完成
**方案**：Intent-Engine 记住进度、决策和下一步

### ✅ 代码重构

**问题**：AI 建议改动，但你需要稍后验证
**方案**：记录所有重构决策和理由

### ✅ 向 AI 学习

**问题**：AI 做技术选择，但你忘了为什么
**方案**：事件日志成为最佳实践的学习文档

### ✅ 团队交接

**问题**：不同团队成员（或 AI 代理）在同一个项目工作
**方案**：完整的上下文和决策历史对所有人可用

---

## 文档

### 快速链接

- 📖 **[快速开始指南](QUICKSTART.md)** - 5 分钟教程
- 🔧 **[安装指南](docs/zh-CN/guide/installation.md)** - 所有安装方式
- 🤖 **[Claude Code 集成](docs/zh-CN/integration/claude-code-system-prompt.md)** - 零配置设置
- 📚 **[完整命令参考](docs/zh-CN/guide/command-reference-full.md)** - 所有命令说明
- 🧠 **[AI 快速指南](docs/zh-CN/guide/ai-quick-guide.md)** - 给 AI 助手看的

### 架构

- **[CLAUDE.md](CLAUDE.md)** - 给 AI 助手：如何使用 Intent-Engine
- **[AGENT.md](AGENT.md)** - 技术细节：数据模型、原子操作
- **[The Intent-Engine Way](docs/zh-CN/guide/the-intent-engine-way.md)** - 设计哲学

---

## 下载 & 安装

### 预编译二进制

下载适合你平台的版本：
- **[最新版本](https://github.com/wayfind/intent-engine/releases/latest)**
  - Linux (x86_64)
  - macOS (Intel & Apple Silicon)
  - Windows (x86_64)

### 从源码编译

```bash
# 需要 Rust 工具链 (https://rustup.rs)
cargo install intent-engine
```

### 包管理器

```bash
# Homebrew (macOS/Linux)
brew install wayfind/tap/intent-engine

# Cargo (跨平台)
cargo install intent-engine
```

---

## 贡献

我们欢迎贡献！查看 [CONTRIBUTING.md](docs/zh-CN/contributing/contributing.md)

**我们特别需要帮助的方面**：
- 📝 文档改进
- 🐛 Bug 报告和修复
- 🌐 翻译
- 💡 功能建议
- 🧪 更多测试覆盖

---

## 许可证

双重许可，你可以选择：
- MIT 许可证 ([LICENSE-MIT](LICENSE-MIT))
- Apache 许可证 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

---

## 常见问题

**问：AI 是如何自动使用 Intent-Engine 的？**
答：系统提示词会让 AI 在会话开始时自动执行 `ie search "todo doing"`，发现未完成任务就主动询问你是否继续。如果没有任务，AI 会在你提出大目标时自动创建任务树并开始工作。

**问：ie plan 和 ie start 有什么关系？**
答：在 v0.10.0+ 中，**没有单独的 start 命令**。`ie plan` 配合 `status: "doing"` 就是启动任务的方式。plan 会自动获取祖先任务的长时记忆，让 AI 获得完整上下文。

**问：我需要懂 Rust 吗？**
答：不需要！Intent-Engine 是预编译的二进制文件，直接安装使用就行。

**问：会把数据发送到云端吗？**
答：不会。所有东西都存在你电脑的 `~/.intent-engine/` 目录里。

**问：可以不用 AI，只给人类用吗？**
答：可以！对人类来说也是个强大的任务追踪工具。但和 AI 配合使用效果最好。

**问：免费吗？**
答：是的，完全开源且永久免费。

**问：支持哪些 AI 助手？**
答：任何有 CLI 权限的：Claude Code（最佳）、自定义 GPT、Gemini CLI、Cursor 等。

**问：和 git 有什么不同？**
答：Git 追踪**代码变更**。Intent-Engine 追踪**战略决策和上下文**。它们互补。

**问：任务的"长时记忆"是什么意思？**
答：当你用 `ie plan` 启动任务时，系统会自动获取该任务的祖先任务（父任务、祖父任务等）的所有信息和决策历史。这样 AI 就能理解整个项目的上下文，而不只是当前任务。

---

**准备好给 AI 长期记忆了吗？**

```bash
cargo install intent-engine
ie --version
```

从我们的 [5 分钟快速开始 →](QUICKSTART.md) 开始吧
