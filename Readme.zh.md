# Intent-Engine

**中文 | [English](README.md)**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wayfind/intent-engine/branch/main/graph/badge.svg)](https://codecov.io/gh/wayfind/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)

---

## 这是什么？

**Intent-Engine 是 AI 的外部长期记忆。**

可以理解为你和 AI 助手（比如 Claude）之间的**共享笔记本**，专门用于**跨会话**工作：
- 你写下**战略目标**（"做一个登录系统"）
- AI 写下**决策过程**（"选择 JWT 因为..."）
- **第二天、下周、甚至下个月** —— 你们都能完整恢复之前的工作
- 🖥️ **Dashboard UI** 让你随时可视化跟踪任务进度

```
┌─────────────────────────────────────────────────────┐
│  没有 Intent-Engine                                 │
├─────────────────────────────────────────────────────┤
│  第一天:                                            │
│  你: "我们来做个登录功能"                            │
│  AI: "好的！"[在 context window 中工作、做决策]      │
│  --- 会话结束，context 清空 ---                     │
│                                                     │
│  第二天:                                            │
│  你: "继续做登录功能"                               │
│  AI: "什么登录？我没有之前的上下文..."              │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│  有了 Intent-Engine                                 │
├─────────────────────────────────────────────────────┤
│  第一天:                                            │
│  你: "我们来做个登录功能"                            │
│  AI: "好的！"[创建任务、做决策、写入持久化存储]     │
│  你: [打开 Dashboard UI 看到任务进度]               │
│  --- 会话结束，但数据保存到磁盘 ---                 │
│                                                     │
│  第二天:                                            │
│  你: "继续做登录功能"                               │
│  AI: [从磁盘读取] "接着做...我们选了 JWT，做完了     │
│      基础认证，昨天决定用 HS256 算法..."            │
└─────────────────────────────────────────────────────┘
```

---

## 为什么需要这个？

### 🤔 问题所在

**短期记忆 vs 长期记忆的差距**

AI 助手有两种"记忆"：

| 类型 | 机制 | 持久性 | 典型用途 |
|------|------|--------|---------|
| **短期记忆** | Context Window（Claude 的对话窗口） | ❌ 会话结束即清空 | 当前对话中的工作 |
| **长期记忆** | 外部存储（Intent-Engine） | ✅ 永久保存到磁盘 | 跨天、跨周的项目 |

**当前的痛点**：
- ❌ 你昨天和 AI 讨论的架构决策，今天 AI 完全忘了
- ❌ 你上周开始的功能，这周 AI 要从头问起
- ❌ 复杂项目变成一堆"请回忆一下我们之前..."的对话
- ❌ 人类无法可视化追踪 AI 的长期工作进度

### ✅ 解决方案

**Intent-Engine = 长期记忆 + Dashboard UI**

```
┌──────────────────────────────────────────┐
│  Claude Context Window                   │  ← 短期记忆
│  (会话内有效)                             │
│  - 当前对话的所有工具调用                 │
│  - AI 自然知道"刚才做了什么"              │
└──────────┬───────────────────────────────┘
           │ [会话结束时清空]
           ▼
┌──────────────────────────────────────────┐
│  Intent-Engine                           │  ← 长期记忆
│  (永久持久化到磁盘)                       │
│  - 任务树和规格说明                       │
│  - 所有决策历史和理由                     │
│  - 跨会话的完整上下文                     │
└──────────┬───────────────────────────────┘
           │
           ▼
┌──────────────────────────────────────────┐
│  Dashboard UI                            │  ← 人类可视化追踪
│  (实时展示任务进度)                       │
│  - 任务树可视化                           │
│  - 决策历史时间线                         │
│  - 进度追踪和状态更新                     │
└──────────────────────────────────────────┘
```

**Intent-Engine 让你获得**：
- ✅ **跨会话恢复** —— 昨天的工作，今天无缝接上
- ✅ **决策历史追踪** —— 每个"为什么"都有记录
- ✅ **长期项目支持** —— 几天、几周、甚至几个月的开发
- ✅ **Dashboard 可视化** —— 人类通过 UI 追踪长期任务，比命令行更直观

---

## vs. TodoWriter：收益远大于损失

### 对比分析

| 维度 | TodoWriter | Intent-Engine |
|------|-----------|---------------|
| **持久性** | ❌ 会话结束即丢失 | ✅ 永久保存到磁盘 |
| **跨会话** | ❌ 每次从头开始 | ✅ 第二天完整恢复 |
| **决策历史** | ❌ 没有 | ✅ 完整事件日志 |
| **长期项目** | ❌ 仅当前会话 | ✅ 几周、几个月 |
| **命令行 todo 提示** | ✅ 实时显示 | ❌ 无（用 Dashboard 代替） |
| **人类追踪方式** | 看命令行 | 🖥️ **Dashboard UI** |

### 权衡结果

**损失**：
- 命令行中的实时 todo 提示（TodoWriter 的特色功能）

**获得**：
- ✅ 长期记忆和跨会话恢复
- ✅ 完整的决策历史追踪
- ✅ Dashboard UI 可视化追踪（比命令行 todo 更强大）
- ✅ AI 自主决策能力（用数据而非硬编码推荐）

**结论：用 Dashboard UI 替代命令行 todo 提示，收益极大**

```
TodoWriter 方案:
┌─────────────────────────────────────────┐
│  命令行                                  │
│  ┌─────────────────────────────────────┐│
│  │ TODO:                               ││
│  │ [x] 设计数据库                       ││
│  │ [ ] 实现 API                        ││  ← 仅当前会话可见
│  │ [ ] 写测试                          ││
│  └─────────────────────────────────────┘│
│  （会话结束后消失）                       │
└─────────────────────────────────────────┘

Intent-Engine 方案:
┌─────────────────────────────────────────┐
│  Dashboard UI                           │
│  ┌─────────────────────────────────────┐│
│  │ 📊 用户登录系统                      ││
│  │ ├── ✅ 设计数据库 (done)            ││
│  │ │   └─ 决策: 选择 PostgreSQL        ││  ← 永久可见
│  │ ├── 🔄 实现 API (doing)             ││  ← 可视化追踪
│  │ │   └─ 决策: 选择 REST over GraphQL ││  ← 决策历史
│  │ └── ⏳ 写测试 (todo)                ││
│  └─────────────────────────────────────┘│
│  （永久保存，跨会话可见）                 │
└─────────────────────────────────────────┘
```

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

**每次进入 Claude Code，AI 会自动：**

```bash
# 1. 查找未完成的任务（恢复长期记忆）
ie search "todo doing"
# → 返回所有待办和进行中的任务

# 2. AI 自主分析和决策
# AI 看到所有任务数据后，自己推理：
#   - 哪些任务有 blocker？
#   - 哪些任务优先级高？
#   - 用户最关心什么？
# → AI 自己做出最优判断，不需要系统"推荐"

# 3. 启动任务（声明式更新状态）
echo '{"tasks":[{"name":"实现用户认证","status":"doing"}]}' | ie plan
# → 更新任务状态为 doing
# → 自动获取祖先任务信息（完整的长期记忆）

# 4. 工作中记录决策（写入长期记忆）
ie log decision "选择 JWT 而不是 Session，因为需要无状态 API"
ie log blocker "等待产品确认密码重置流程"
ie log note "已完成 token 生成逻辑"

# 5. 完成时更新状态
echo '{"tasks":[{"name":"实现用户认证","status":"done"}]}' | ie plan
```

**关键理念**：
- 🧠 **AI 负责智能决策** —— Intent-Engine 只提供数据，不"推荐"
- 📊 **系统负责持久化** —— 完整、准确、结构化的长期记忆
- 🖥️ **Dashboard 负责展示** —— 人类通过 UI 追踪，比命令行更直观

### 人类追踪：Dashboard UI

**启动 Dashboard**：

```bash
# 自动启动（任何 ie 命令都会触发）
ie ls

# 或手动启动
ie dashboard start

# 打开浏览器查看
ie dashboard open
# → 打开 http://127.0.0.1:11391
```

**Dashboard 功能**：
- 📊 任务树可视化
- ⏱️ 决策历史时间线
- 🔄 实时状态更新（CLI 操作自动同步）
- 🔍 搜索和过滤

```
┌──────────────────────────────────────────────────────┐
│  Intent-Engine Dashboard                             │
├──────────────────────────────────────────────────────┤
│                                                      │
│  📁 用户登录系统                         [high] doing │
│  ├── 📄 实现 JWT 生成和验证              [done] ✅    │
│  │   └─ 📝 决策: 选择 HS256 算法                     │
│  ├── 📄 添加 OAuth2 集成                 [doing] 🔄  │
│  │   └─ ⚠️ 阻塞: 等待 Google API 密钥               │
│  └── 📄 实现会话管理                     [todo] ⏳    │
│                                                      │
│  最近事件                                            │
│  ─────────────────────────────────────────────────── │
│  12:30  decision  选择 JWT + HS256                   │
│  11:45  note      已完成 token 生成逻辑              │
│  10:20  blocker   等待 Google API 密钥               │
│                                                      │
└──────────────────────────────────────────────────────┘
```

---

## 示例：跨会话工作

```bash
# === 第一天下午 3 点 ===
你："帮我做一个用户登录系统"
AI: [判断是长期目标，创建任务树]

echo '{
  "tasks": [{
    "name": "做一个用户登录系统",
    "status": "doing",
    "spec": "JWT 令牌、OAuth2 支持、7 天会话",
    "children": [
      {"name": "实现 JWT 生成和验证"},
      {"name": "添加 OAuth2 集成"},
      {"name": "实现会话管理"}
    ]
  }]
}' | ie plan

AI: [开始实现第一个子任务]
    ie log decision "选择 HS256 算法，因为简单且足够安全"
    ie log note "已完成 token 生成，待验证逻辑"

你: [打开 Dashboard UI]
    # → 看到任务树、决策历史、实时进度

# [你下班了，会话结束]

# === 第二天早上 9 点，新会话 ===
你："继续登录系统"
AI: ie search "todo doing"
    # ✅ 找到任务 "做一个用户登录系统" (doing)
    # ✅ 看到昨天的决策 "选择 HS256..."
    # ✅ 看到进度 "已完成 token 生成"

    "好的，我看到昨天我们选择了 JWT + HS256，
     已经完成 token 生成，现在让我继续实现验证逻辑..."

你: [打开 Dashboard UI]
    # → 看到完整的历史，包括昨天的所有决策

# [无缝恢复工作，没有任何信息丢失]
```

---

## 与 AI 助手集成

### 与 Claude Code 配合（零配置！）

如果你用 **Claude Code**（Anthropic 官方 CLI），Intent-Engine **完全自动化**：

**安装后立即可用：**

1. 安装 Intent-Engine：`cargo install intent-engine`
2. 在项目里启动 Claude Code
3. **无需任何配置！**系统提示词会让 Claude 自动：
   - 每次新会话开始时执行 `ie search "todo doing"`
   - 发现未完成任务时主动询问是否继续
   - 自主分析所有任务数据，做出最优决策
   - 持续用 `ie log` 记录关键决策到长期记忆
   - 完成后自动更新任务状态

**工作流示例：**

```
# 场景 1: 有未完成任务（恢复长期记忆）
你：[打开 Claude Code]
Claude：[自动执行 ie search "todo doing"]
        "我发现有 3 个待办任务：
         1. 实现用户认证 (todo)
         2. 重构数据库层 (doing)
         3. 修复登录 bug (todo, 优先级高)

         基于当前状态，我建议继续 #2 重构数据库层，
         因为它已经在进行中且是架构基础。要继续吗？"
你："好的"
Claude：[执行 ie plan 恢复任务]
        [读取所有历史决策和上下文]
        "好，我看到之前决定用 Repository 模式，
         已经完成了 User 模型，现在继续 Order 模型..."

# 场景 2: 没有任务（静默等待）
你：[打开 Claude Code] "帮我实现一个 REST API"
Claude：[判断这是长期目标]
        [自动执行 ie plan 创建任务树]
        "我创建了任务'实现 REST API'，包含 4 个子任务，
         你可以在 Dashboard UI 中查看进度..."
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

每个"为什么"都被记录到**长期记忆**：

```bash
ie log decision "选择 PostgreSQL 而不是 MongoDB，因为需要 ACID 保证"
ie log blocker "等待团队的设计审批"
ie log milestone "MVP 完成，可以测试了"
ie log note "性能测试结果：QPS 达到 1000"
```

**这些记录会永久保存**，并在 Dashboard UI 中展示完整时间线。

### 🎯 声明式工作流：ie plan 是核心

**ie plan 是统一的任务管理接口**

```bash
# 1️⃣ 创建任务
echo '{"tasks":[{"name":"新功能","spec":"...","priority":"high"}]}' | ie plan

# 2️⃣ 更新任务（幂等性，可以多次运行）
echo '{"tasks":[{"name":"新功能","spec":"更新后的内容"}]}' | ie plan

# 3️⃣ 启动任务（声明式更新状态）
echo '{"tasks":[{"name":"新功能","status":"doing"}]}' | ie plan

# 4️⃣ 完成任务
echo '{"tasks":[{"name":"新功能","status":"done"}]}' | ie plan
```

### 🖥️ Dashboard UI

人类通过 Dashboard 追踪长期任务：

```bash
ie dashboard start   # 启动 Dashboard
ie dashboard open    # 打开浏览器
ie dashboard status  # 查看状态
```

---

## 实际使用场景

### ✅ 多天/多周开发项目

**问题**：复杂功能需要分多次会话完成，每次恢复都要重新解释上下文
**方案**：Intent-Engine 自动恢复，Dashboard 可视化追踪

### ✅ 代码重构

**问题**：AI 建议改动，但你需要稍后验证或向团队解释
**方案**：决策历史在 Dashboard 中完整展示

### ✅ 向 AI 学习

**问题**：AI 做技术选择时的推理过程，几天后你就忘了
**方案**：Dashboard 时间线成为学习文档

### ✅ 团队交接

**问题**：项目交给其他成员时，上下文丢失
**方案**：分享 Dashboard 链接，完整决策历史可见

---

## 文档

### 快速链接

- 📖 **[快速开始指南](QUICKSTART.md)** - 5 分钟教程
- 🔧 **[安装指南](docs/zh-CN/guide/installation.md)** - 所有安装方式
- 🤖 **[Claude Code 集成](docs/zh-CN/integration/claude-code-system-prompt.md)** - 零配置设置
- 📚 **[完整命令参考](docs/zh-CN/guide/command-reference-full.md)** - 所有命令说明
- 🖥️ **[Dashboard 用户指南](docs/dashboard-user-guide.md)** - UI 使用说明

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

**问：用 Intent-Engine 代替 TodoWriter，损失了什么？获得了什么？**
答：
- **损失**：命令行中的实时 todo 提示
- **获得**：长期记忆、跨会话恢复、决策追踪、Dashboard UI 可视化
- **结论**：Dashboard UI 比命令行 todo 提示更强大，收益远大于损失

**问：人类怎么追踪 AI 的工作进度？**
答：通过 **Dashboard UI**。运行 `ie dashboard open` 打开浏览器，可以看到：
- 任务树和层级关系
- 所有决策的完整时间线
- 实时状态更新
- 搜索和过滤功能

这比 TodoWriter 的命令行提示更直观、更强大。

**问：Intent-Engine 和 Context Window 有什么关系？**
答：它们是**互补**的关系：
- **Context Window（短期记忆）**：当前会话内的所有内容，会话结束即清空
- **Intent-Engine（长期记忆）**：永久保存到磁盘，跨天、跨周、跨月

在同一个会话内，AI 不需要查询 Intent-Engine 就知道"刚才做了什么"（因为在 Context Window 里）。但新会话开始时，AI 需要 Intent-Engine 来恢复"昨天/上周做了什么"。

**问：AI 怎么知道应该做哪个任务？**
答：AI 用 `ie search "todo doing"` 获取所有待办任务的数据，然后**自己推理和决策**。Intent-Engine 不会"推荐"你做什么，它只提供完整、准确的数据，让 AI 自主判断。

**问：ie plan 和 ie start 有什么关系？**
答：在 v0.10.0+ 中，**没有单独的 start 命令**。`ie plan` 配合 `status: "doing"` 就是启动任务的声明式方式。

**问：我需要懂 Rust 吗？**
答：不需要！Intent-Engine 是预编译的二进制文件，直接安装使用。

**问：会把数据发送到云端吗？**
答：不会。所有东西都存在你电脑的 `~/.intent-engine/` 目录里，完全离线工作。Dashboard 也是本地运行。

**问：免费吗？**
答：是的，完全开源且永久免费。

**问：支持哪些 AI 助手？**
答：任何有 CLI 权限的：Claude Code（最佳）、自定义 GPT、Gemini CLI、Cursor 等。

**问：和 git 有什么不同？**
答：Git 追踪**代码变更**。Intent-Engine 追踪**战略决策和上下文**。它们互补。

---

**准备好给 AI 长期记忆了吗？**

```bash
cargo install intent-engine
ie --version
ie dashboard open  # 打开 Dashboard UI
```

从我们的 [5 分钟快速开始 →](QUICKSTART.md) 开始吧
