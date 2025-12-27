# Intent-Engine

**中文 | [English](README.md)**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)

---

> **AI 会遗忘。你不应该反复提醒它。**

## 问题

每次和 AI 开始新会话：

```
第一天："帮我做登录系统"
       AI 工作出色，做了很多聪明的决策...
       [会话结束]

第二天："继续做登录系统"
       AI："什么登录系统？我完全不记得了。"
```

AI 有强大的推理能力，只是记不住事。

## 解决方案

```bash
cargo install intent-engine
```

现在你的 AI 能记住一切——跨天、跨周、跨月。

```
第一天："帮我做登录系统"
       AI 创建任务、工作、记录决策 → 保存到磁盘

第二天："继续做登录系统"
       AI 读取记忆 → "接着做...我们选了 JWT + HS256，
       token 生成已完成，下一步：OAuth 集成"
```

**无云端。零配置。纯粹的持久记忆。**

---

## 工作原理

Intent-Engine 给 AI 一套简单协议：

```bash
ie status              # 我在做什么？
ie plan                # 创建或更新任务（JSON stdin）
ie log decision "..."  # 记录为什么做这个选择
ie search "登录"       # 查找相关历史
```

AI 开始会话时运行 `ie status`，一切都回来了：
- 当前任务及其上下文
- 所有祖先任务（更大的蓝图）
- 决策历史（每个选择背后的"为什么"）

**一条命令，完整的上下文恢复。**

---

## 集成

### Claude Code（一键安装）

```bash
/plugin marketplace add wayfind/intent-engine
/plugin install intent-engine@intent-engine
```

搞定。插件包含：
- **Hook**：自动安装 ie、初始化项目、会话启动时运行 `ie status`
- **Skill**：引导 Claude 用 `ie plan` 替代 TodoWrite

### 手动安装

如果你偏好手动配置：

```bash
# 1. 安装二进制
cargo install intent-engine
# 或: brew install wayfind/tap/intent-engine
# 或: npm install -g @m3task/intent-engine

# 2. 添加系统提示词
claude --append-system-prompt "Use ie plan instead of TodoWrite. Commands: ie status, echo '{...}'|ie plan, ie log, ie search"
```

### 其他 AI 助手

任何有 CLI 权限的 AI 都可以直接使用 `ie` 命令。

---

## 更深层的理念

大多数工具追踪的是**发生了什么**（提交、日志、事件）。

Intent-Engine 追踪的是**你想做什么**以及**为什么**。

```
Git:           "修改了 auth.rs 第 42 行"
Intent-Engine: "为了无状态 API 的可扩展性，选择 JWT 而非 Session"
```

代码会变。意图永存。

---

## 核心功能

- **层级任务** — 把大目标拆解成小目标
- **决策历史** — 每个"为什么"都有记录
- **跨会话记忆** — 从上次离开的地方继续
- **本地存储** — 一切都在 `~/.intent-engine/`，无云端
- **Dashboard UI** — 在 `localhost:11391` 可视化进度

---

## 快速参考

```bash
# 安装
cargo install intent-engine
# 或：brew install wayfind/tap/intent-engine
# 或：npm install -g @m3task/intent-engine

# 核心命令
ie status                    # 当前上下文
ie search "todo doing"       # 查找未完成的工作
echo '{"tasks":[...]}' | ie plan   # 创建/更新任务
ie log decision "选了 X"     # 记录决策
ie dashboard open            # 可视化 UI
```

---

## 文档

- [快速开始](QUICKSTART.md) — 5 分钟上手
- [CLAUDE.md](CLAUDE.md) — 给 AI 助手
- [命令参考](docs/zh-CN/guide/command-reference-full.md) — 所有命令

---

## 许可证

MIT 或 Apache-2.0，任选。

---

**给你的 AI 它应得的记忆。**

```bash
cargo install intent-engine
```
