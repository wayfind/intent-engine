# Intent-Engine

**中文 | [English](README.md)**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)

> **AI 编程助手的持久记忆。**

---

## AI 总是忘记。每一次。

```
第一天："帮我做认证系统"
       AI 工作出色，做了很多聪明的决策...
       [会话结束]

第二天："继续做认证"
       AI："什么认证？"
```

你经历过。我们都经历过。

---

## 一条命令改变一切

```bash
ie status
```

现在你的 AI 记住了：

```
第二天："继续做认证"
       AI："恢复任务 #42：JWT 认证。
            已完成：token 生成、验证。
            下一步：refresh token 轮换。
            决策：选择 HS256，因为单服务更简单。"
```

**完整上下文。瞬间恢复。**

---

## 但这不仅仅是"记忆"

想想开发过程中实际发生的事：

| 场景 | 没有 Intent-Engine | 有 Intent-Engine |
|------|-------------------|------------------|
| 会话结束 | 上下文丢失 | ✓ 已持久化 |
| 工具崩溃 | 进度消失 | ✓ 可恢复 |
| 电脑重启 | 从头开始 | ✓ 立即恢复 |
| 一周之后 | "我在做什么？" | ✓ 完整历史 |
| 多个 agent | 混乱 | ✓ 隔离会话 |
| 复杂项目 | 上下文爆炸 | ✓ Focus-driven |

**这不是记忆。这是可靠 AI 工作流的基础设施。**

---

## 为什么 Intent-Engine 有效

### 极简足迹

| 方面 | Intent-Engine | 典型方案 |
|------|---------------|----------|
| 上下文开销 | ~200 tokens | 数千 |
| 集成方式 | 系统提示词 / Hook | 重量级 MCP 服务器 |
| 运行时 | 单二进制 | 后台守护进程 |

AI 只获取所需。不多不少。

### 久经考验的技术栈

| 组件 | 选择 | 原因 |
|------|------|------|
| 语言 | Rust | 内存安全，高性能 |
| 存储 | SQLite | 零配置，可靠 |
| 搜索 | FTS5 | GB 级，毫秒响应 |
| 位置 | 纯本地 | 数据永远是你的 |

---

## 更大的图景：长时任务

这是 AI agent 领域的未解难题：**持续数天或数周的任务**。

单会话 AI 无法处理。Intent-Engine 可以。

```
为期一周的重构项目：

├── Agent A (session: "api")    → focus: #12 REST 接口
├── Agent B (session: "db")     → focus: #15 Schema 迁移
└── Agent C (session: "test")   → focus: #18 集成测试
                                  depends_on: [#12, #15]
```

**四大能力协同工作：**

| 挑战 | 解决方案 |
|------|----------|
| 中断 | 持久记忆 |
| 多 agent | Session 隔离 |
| 调度 | 依赖图 |
| 上下文爆炸 | Focus-driven 检索 |

每个 agent 维护隔离的 focus。编排器读取 `depends_on` 进行并行调度。状态跨崩溃、重启、跨天持久化。

**结果：可靠的多日、多 agent 工作流。**

---

## 开始使用

**Claude Code（一条命令）：**

```
/plugin marketplace add wayfind/origin-task
/plugin install intent-engine@wayfind/origin-task
```

完成。插件自动处理二进制安装和集成。

**其他方式：**

```bash
# 安装
brew install wayfind/tap/intent-engine  # 或 npm, cargo

# 使用
ie status                         # 恢复上下文
echo '{"tasks":[...]}' | ie plan  # 创建任务
ie log decision "选了 X"          # 记录决策
ie search "关键词"                # 搜索历史
```

---

## 工作原理

```
会话开始 → ie status → 上下文恢复
                       ↓
工作中   → ie plan   → 任务更新
         → ie log    → 决策记录
                       ↓
中断     → 状态自动持久化
                       ↓
下次会话 → ie status → 从上次离开的地方继续
```

---

## 文档

- [快速开始](docs/zh-CN/guide/quickstart.md)
- [CLAUDE.md](CLAUDE.md) — AI 集成指南
- [命令参考](docs/zh-CN/guide/command-reference-full.md)

---

## 许可证

MIT 或 Apache-2.0

---

**给你的 AI 它应得的记忆。**
