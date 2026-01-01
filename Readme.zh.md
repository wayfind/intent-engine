# Intent-Engine

**中文 | [English](README.md)**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)

> **AI 编程助手的持久记忆。**

---

## 问题

AI 助手不断丢失上下文：

| 场景 | 后果 |
|------|------|
| 会话结束 | 上下文全部丢失 |
| 工具崩溃 | 进度消失 |
| 电脑重启 | 从零开始 |
| 一周之后 | "我之前在做什么？" |

你浪费时间反复解释。AI 浪费 token 重新理解。

## 解决方案

```bash
# Claude Code 用户：运行这一条命令
/plugin marketplace add wayfind/origin-task
/plugin install intent-engine@wayfind/origin-task
```

现在你的 AI 记住一切 — 跨会话、跨崩溃、跨重启、跨周。

```
第一周，周一：  "做认证系统"
                AI 工作，记录决策 → 保存到本地

第二周，周三：  "继续认证"
                AI 读取记忆 → "恢复任务 #42：JWT 认证。
                已完成：token 生成、验证中间件。
                下一步：refresh token 轮换。
                决策记录：选择 HS256，因为单服务场景更简单。"
```

**一条命令恢复完整上下文。每一次。**

---

## 为什么选择 Intent-Engine

### 上下文友好

| 方面 | Intent-Engine | 典型方案 |
|------|---------------|----------|
| 上下文占用 | ~200 tokens | 数千 tokens |
| 集成方式 | 系统提示词 / Hook / Skill | 重量级 MCP 服务器 |
| 运行足迹 | 单二进制，无守护进程 | 后台进程 |

AI 只获取所需信息，不多不少。

### 高性能

| 组件 | 技术 | 能力 |
|------|------|------|
| 核心 | Rust | 内存安全，零成本抽象 |
| 存储 | SQLite | 久经考验，零配置 |
| 搜索 | FTS5 | GB 级文本，毫秒响应 |
| 隐私 | 纯本地 | 数据永不离开你的机器 |

### 智能任务模型

- **层级化** — 把复杂目标拆解为子任务
- **多 Agent** — 多个 agent 并行工作，各自专注不同任务（session 隔离）
- **依赖图** — `depends_on` 支持 MapReduce 式任务编排
- **可追溯** — 每个决策都有上下文记录
- **可恢复** — 从任何中断点继续

---

## 快速开始

**Claude Code 用户：** 插件一键完成（二进制 + 集成）。

```
/plugin marketplace add wayfind/origin-task
/plugin install intent-engine@wayfind/origin-task
```

**其他用户：** 两步搞定。

```bash
# 第一步：安装二进制
brew install wayfind/tap/intent-engine
# 或：npm install -g @origintask/intent-engine
# 或：cargo install intent-engine

# 第二步：添加到 AI 系统提示词
# "使用 ie 管理任务记忆。会话开始时运行 ie status。"
```

---

## 工作原理

```bash
ie status              # 恢复上下文：当前任务、祖先任务、决策历史
ie plan                # 创建/更新任务（JSON 通过 stdin）
ie log decision "..."  # 记录为什么做这个选择
ie search "关键词"     # 全文搜索所有历史
```

典型 AI 工作流：

```
会话开始 → ie status → 完整上下文恢复
                       ↓
工作中   → ie plan   → 任务创建/更新
         → ie log    → 决策记录
                       ↓
会话结束 → 数据持久化到本地
                       ↓
下次会话 → ie status → 从上次离开的地方继续
```

---

## 安装详情

### 二进制安装

| 方式 | 命令 | 说明 |
|------|------|------|
| Homebrew | `brew install wayfind/tap/intent-engine` | macOS/Linux |
| npm | `npm install -g @origintask/intent-engine` | 跨平台 |
| Cargo | `cargo install intent-engine` | 需要 Rust |
| 直接下载 | `curl -fsSL .../ie-manager.sh \| bash -s install` | 无依赖 |

### AI 工具集成

**Claude Code（插件）**
```
/plugin marketplace add wayfind/origin-task
/plugin install intent-engine@wayfind/origin-task
```

**Claude Code（手动）**

添加到 `~/.claude/CLAUDE.md`：
```markdown
使用 `ie` 管理任务。会话开始时运行 `ie status`。
```

**其他 AI 工具**

添加到系统提示词：
```
使用 ie 管理持久任务记忆。命令：ie status, ie plan, ie log, ie search
```

---

## 命令参考

```bash
ie status                         # 当前上下文
ie search "todo doing"            # 查找未完成的工作
echo '{"tasks":[...]}' | ie plan  # 创建/更新任务
ie log decision "选了 X"          # 记录决策
ie dashboard open                 # 可视化 UI (localhost:11391)
```

---

## 文档

- [快速开始](docs/zh-CN/guide/quickstart.md)
- [CLAUDE.md](CLAUDE.md) — AI 助手指南
- [命令参考](docs/zh-CN/guide/command-reference-full.md)

---

## 许可证

MIT 或 Apache-2.0

---

**给你的 AI 它应得的记忆。**
