# Intent-Engine 
  
   **中文 | [English](README.en.md)**
  
[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wayfind/intent-engine/branch/main/graph/badge.svg)](https://codecov.io/gh/wayfind/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![Documentation](https://docs.rs/intent-engine/badge.svg)](https://docs.rs/intent-engine)


**为人机协作，编织清晰的思路**

> AI 的外部长时记忆 + 战略任务管理系统
>
> 将您和 AI 伙伴短暂、易失的协作瞬间，沉淀为项目可追溯、可恢复的永恒智慧



## 🎯 这是什么？

**项目级持久化任务系统** + **完整决策历史追踪** = AI 的外部长时记忆

- 📝 **战略意图层**：关注 What（做什么）和 Why（为什么），而非 How（怎么做）
- 🧠 **跨会话记忆**：持久化到 SQLite，任何时候都能恢复完整上下文
- 🌳 **层级任务树**：支持无限层级的父子任务，自然的问题分解
- 📊 **决策历史**：每个关键决策都被记录为事件流，可追溯、可回顾
- 🔄 **AI 原生**：CLI + JSON + MCP 协议，为 AI 工具链深度优化

---

## 💡 解决什么痛点？

### Claude Code TodoWrite 的局限

❌ **会话级生命周期** - 对话结束即消失，无法跨会话
❌ **无决策历史** - 不知道为什么做某个决定
❌ **平铺结构** - 难以管理复杂的层级任务

### Intent-Engine 的解决方案

✅ **项目级持久化** - 永久保存在 `.intent-engine/project.db`
✅ **完整事件流** - 记录每个 decision/blocker/milestone
✅ **任务树结构** - 自然的层级分解 + 强制子任务完成验证
✅ **原子操作** - `start`/`spawn-subtask`/`switch` 等命令节省 50-70% Token

---

## 🚀 快速开始

### 安装

```bash
# 方式 1: Cargo Install（推荐）
cargo install intent-engine

# 方式 2: 预编译二进制
# 访问 https://github.com/wayfind/intent-engine/releases

# 验证安装
intent-engine --version
```

### 5 分钟核心体验

```bash
# 1. 添加任务（自动初始化项目）
echo "使用 JWT 认证，支持刷新 Token" | \
  intent-engine task add --name "实现用户认证" --spec-stdin

# 2. 开始任务
intent-engine task start 1 --with-events

# 3. 发现子问题？创建子任务并自动切换
intent-engine task spawn-subtask --name "配置 JWT 密钥"

# 4. 记录决策
echo "选择 HS256 算法，密钥存储在环境变量" | \
  intent-engine event add --type decision --data-stdin

# 5. 完成子任务，获取下一步建议
intent-engine task done
intent-engine task pick-next
```

> 💡 **详细教程**: [Quickstart Guide](docs/zh-CN/guide/quickstart.md) | [The Intent-Engine Way](docs/zh-CN/guide/the-intent-engine-way.md)

---

## 🔌 MCP 集成：与 Claude Code/Desktop 无缝集成

一键安装脚本：

```bash
# 克隆并安装
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine
cargo install --path .

# 自动配置 MCP 服务器
./scripts/install/install-mcp-server.sh
```

安装后，Claude 可以自动使用 **13 个 MCP 工具** 来管理任务和记录决策，无需手动运行命令。

> 📖 **详细指南**: [MCP 服务器集成](docs/zh-CN/integration/mcp-server.md) | [AI 集成完整指南](CLAUDE.md)

---

## ✨ 核心特性

### v0.2 新功能 (2025-11)
- **🔗 任务依赖系统**：定义任务依赖关系，自动阻止依赖未满足的任务启动
- **📊 智能事件查询**：按类型和时间过滤事件，大幅节省 Token 和处理时间
- **🎯 优先级枚举**：人性化的优先级接口 (`critical`/`high`/`medium`/`low`)
- **📝 命令更名**：`task find` → `task list` 更清晰直观

### 核心功能
- **🔍 智能项目检测**：自动向上查找 `.git`/`Cargo.toml` 等标记，确定项目根目录
- **⚡ 惰性初始化**：写入命令自动初始化，无需手动 `init`
- **🎯 聚焦工作流**：`current_task_id` 概念，大部分命令操作当前聚焦任务
- **🤖 智能推荐**：`pick-next` 基于深度优先策略推荐下一个任务
- **🔍 FTS5 全文搜索**：GB 级任务量下毫秒级响应，`**` 高亮匹配词
- **📦 零依赖部署**：单一静态链接二进制，无需 Python/Node 环境
- **🚀 Rust 原生 MCP**：启动 < 10ms，内存占用 ~5MB

---

## 📚 文档导航

### 🎯 核心文档
- [**接口规范**](docs/INTERFACE_SPEC.md) - CLI/MCP/Rust API 权威定义
- [**设计哲学**](docs/zh-CN/guide/the-intent-engine-way.md) - Intent-Engine Way 深入解读

### 🚀 用户指南
- [安装指南](docs/zh-CN/guide/installation.md) - 所有安装方式 + 故障排除
- [快速开始](docs/zh-CN/guide/quickstart.md) - 详细教程和最佳实践
- [命令参考](docs/zh-CN/guide/command-reference-full.md) - 完整命令说明

### 🤖 AI 集成
- [Claude 集成指南](CLAUDE.md) - AI 助手完整集成手册
- [MCP 服务器](docs/zh-CN/integration/mcp-server.md) - Claude Code/Desktop 配置
- [通用 LLM 集成](docs/zh-CN/integration/generic-llm.md) - 其他 AI 工具集成

### 🔧 技术文档
- [性能基准](docs/zh-CN/technical/performance.md) - 性能测试和优化
- [安全测试](docs/zh-CN/technical/security.md) - 安全性验证
- [MCP 工具同步](docs/zh-CN/technical/mcp-tools-sync.md) - 维护者指南

### 👥 贡献者
- [贡献指南](docs/zh-CN/contributing/contributing.md) - 如何贡献代码
- [发布流程](docs/zh-CN/contributing/publish-to-crates-io.md) - 发布到 crates.io

---

## 🧪 质量保证

- **240+ 测试全部通过** ✅ (单元测试 + 集成测试 + 性能测试)
- **85% 代码覆盖率** 📊 持续集成验证
- **安全性测试** 🛡️ 特殊字符、SQL 注入、路径遍历防护
- **跨平台验证** 🖥️ Linux/macOS/Windows 自动化测试

---

## 🛠️ 技术栈

- **语言**: Rust 2021 Edition
- **CLI**: clap 4.5 (声明式命令行)
- **数据库**: SQLite + sqlx 0.7 (异步查询)
- **全文搜索**: SQLite FTS5 (毫秒级搜索)
- **异步运行时**: tokio 1.35

---

## 🌟 与其他工具的本质区别

| 维度 | Intent-Engine | Claude Code TodoWrite | Jira/Linear |
|------|---------------|----------------------|-------------|
| **生命周期** | 项目级（永久） | 会话级（临时） | 项目级（永久） |
| **核心用户** | 人类 ↔ AI | AI 内部 | 人类团队 |
| **决策历史** | ✅ 完整事件流 | ❌ 无记录 | ⚠️ 评论（非结构化） |
| **任务结构** | 层级树 + 规格 | 平铺列表 | 工作流 + 字段 |
| **AI 集成** | CLI + JSON + MCP | Tool Call | ❌ 不支持 |

**典型使用场景**:
- **Intent-Engine**: "实现用户认证系统"（数天，完整上下文，可恢复）
- **TodoWrite**: "修改 auth.rs"（当前会话，临时步骤）
- **Jira**: "PROJ-123: OAuth2 支持"（Sprint 任务，人工维护）

---

## 📄 许可证

本项目采用 MIT 或 Apache-2.0 双许可证。

- MIT License - 详见 [LICENSE-MIT](LICENSE-MIT)
- Apache License 2.0 - 详见 [LICENSE-APACHE](LICENSE-APACHE)

---

**下一步**: 阅读 [The Intent-Engine Way](docs/zh-CN/guide/the-intent-engine-way.md) 深入理解设计哲学，或查看 [Quickstart](docs/zh-CN/guide/quickstart.md) 开始使用。
