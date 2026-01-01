# Intent-Engine

**中文 | [English](README.md)**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)

**AI 编程助手的持久记忆。**

---

## AI 总是忘记。每一次。

**没有 Intent-Engine：**
```
第一天："做认证系统"
       AI 工作出色...
       [会话结束]

第二天："继续做认证"
       AI："什么认证？"
```

**有 Intent-Engine：**
```
第一天："做认证系统"
       AI 工作，保存进度...
       [会话结束]

第二天："继续做认证"
       AI："恢复 #42：JWT 认证。
            已完成：token 生成。
            下一步：refresh token。"
```

**一条命令恢复一切：** `ie status`

---

## 不只是记忆 — 是基础设施

实际发生的事：

- **会话结束** → ✓ 已持久化
- **工具崩溃** → ✓ 可恢复
- **一周之后** → ✓ 完整历史
- **多个 agent** → ✓ 隔离
- **复杂项目** → ✓ Focus-driven

---

## 为什么有效

**极简足迹** — ~200 tokens 开销，单二进制，无守护进程

**久经考验** — Rust + SQLite + FTS5，GB 级毫秒响应，纯本地

---

## 更大的图景

> **AI agent 领域的未解难题：持续数天或数周的任务。**

Intent-Engine 提供基础：

```
为期一周的重构：

├── Agent A (session: "api")    → focus: #12 REST 接口
├── Agent B (session: "db")     → focus: #15 Schema 迁移
└── Agent C (session: "test")   → focus: #18 集成测试
                                  depends_on: [#12, #15]
```

- **中断** → 持久记忆
- **多 agent** → Session 隔离
- **调度** → 依赖图 (`depends_on`)
- **上下文爆炸** → Focus-driven 检索

**结果：** 可靠的多日、多 agent 工作流。

---

## 开始使用

**Claude Code** — 一条命令搞定：

```
/plugin marketplace add wayfind/origin-task
/plugin install intent-engine@wayfind/origin-task
```

**手动安装：**

```bash
# 安装（任选一种）
brew install wayfind/tap/intent-engine
npm install -g @origintask/intent-engine
cargo install intent-engine

# 核心命令
ie status                         # 恢复上下文
echo '{"tasks":[...]}' | ie plan  # 创建/更新任务
ie log decision "选了 X"          # 记录决策
ie search "关键词"                # 搜索历史
```

---

## 工作原理

```
会话开始   →  ie status  →  完整上下文恢复
                                  ↓
工作中     →  ie plan    →  任务追踪
           →  ie log     →  决策记录
                                  ↓
中断       →  自动持久化
                                  ↓
下次会话   →  ie status  →  从上次离开处继续
```

---

## 文档

- **[快速开始](docs/zh-CN/guide/quickstart.md)** — 5 分钟上手
- **[CLAUDE.md](CLAUDE.md)** — AI 集成指南
- **[命令参考](docs/zh-CN/guide/command-reference-full.md)** — 完整参考

---

**MIT 或 Apache-2.0** · [GitHub](https://github.com/wayfind/intent-engine)

*给你的 AI 它应得的记忆。*
