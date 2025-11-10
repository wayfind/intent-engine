# Intent-Engine
**为人机协作，编织清晰的思路**

>
> 将您和 AI 伙伴短暂、易失的协作瞬间，沉淀为项目可追溯、可恢复的永恒智慧
>
 
**中文 | [English](README.en.md)**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wayfind/intent-engine/branch/main/graph/badge.svg)](https://codecov.io/gh/wayfind/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![Documentation](https://docs.rs/intent-engine/badge.svg)](https://docs.rs/intent-engine)

---

## 🎯 这是什么？

Intent-Engine 是一个命令行工具 + 数据库系统，用于记录、追踪和回顾**战略意图**。它为人类与 AI 的协作提供了一个共享的、可追溯的"意图层"。

**核心特点：**
- 📝 **战略级任务管理**：关注 What（做什么）和 Why（为什么），而不仅仅是 How（怎么做）
- 🧠 **AI 的外部长时记忆**：跨会话持久化决策历史和上下文
- 🌳 **层级化问题分解**：支持无限层级的父子任务关系
- 📊 **结构化决策追踪**：每个关键决策都被记录为事件流
- 🔄 **JSON 原生接口**：完美适配 AI 工具集成

---

## 👥 为谁设计？

### 主要用户

1. **人类开发者**：设定战略目标，记录项目意图
2. **AI Agent**：理解目标、执行任务、记录决策过程
3. **人机协作团队**：在长期项目中保持上下文同步

### 使用场景

- ✅ 需要 AI 在多个会话间持续工作的复杂项目
- ✅ 需要追溯"为什么做这个决策"的技术项目
- ✅ 需要分解大型任务为子任务树的系统工程
- ✅ 需要 AI 自主管理工作优先级的自动化流程

---

## 💡 解决什么痛点？

### 对人类的价值

**传统任务管理工具（Jira/Linear）的问题：**
- ❌ 聚焦于战术执行（Sprint、Story Points），缺少战略层
- ❌ 需要大量手动维护（状态更新、评论）
- ❌ 不适合 AI 集成（Web UI 为主）

**Intent-Engine 的解决方案：**
- ✅ 战略意图层：每个任务包含完整的 **规格说明（spec）** 和 **决策历史（events）**
- ✅ 自动化友好：AI 可以自主创建、更新、切换任务
- ✅ CLI + JSON：完美的 AI 工具链集成

### 对 AI的价值

**Claude Code TodoWrite 的局限性：**
- ❌ **会话级**：仅存在于当前对话，会话结束即消失
- ❌ **无历史**：无法追溯之前的决策和思考过程
- ❌ **平铺结构**：缺少层级关系，难以管理复杂任务

**Intent-Engine 的优势：**
- ✅ **项目级**：持久化到 SQLite 数据库，跨会话永久保存
- ✅ **可追溯**：完整的事件流记录每个决策的上下文
- ✅ **层级化**：任务树结构，强制完成子任务才能完成父任务
- ✅ **原子操作**：`start`、`pick-next`、`spawn-subtask`、`switch` 等命令节省 50-70% Token

---

## 📊 与其他工具的本质区别

| 维度 | Intent-Engine | Claude Code TodoWrite | Jira/Linear |
|------|---------------|----------------------|-------------|
| **核心哲学** | 战略意图层：What + Why | 战术执行层：What (临时) | 任务追踪层：What + When |
| **主要用户** | 人类 ↔ AI（双向协作） | AI 内部使用（单向） | 人类团队（协作） |
| **生命周期** | 项目级（跨会话、持久化） | 会话级（临时、易失） | 项目级（持久化） |
| **数据结构** | 任务树 + 事件流 + 规格说明 | 平铺列表（无层级） | 工作流 + 字段 + 评论 |
| **历史追溯** | ✅ 完整决策历史（events）| ❌ 无历史记录 | ⚠️ 有评论但无结构化决策 |
| **交互模式** | CLI + JSON（AI友好） | Tool Call（AI专用） | Web UI（人类友好） |
| **粒度定位** | 粗粒度（战略里程碑） | 细粒度（当前步骤） | 中粒度（Sprint任务） |
| **核心价值** | AI的外部长时记忆 | AI的工作记忆（短期） | 团队的工作协调 |

### 典型使用场景对比

**Intent-Engine：** "实现用户认证系统"（包含完整的技术规格、决策历史、子任务树）
- 生命周期：数天到数周
- AI 可以在任何时候通过 `task start --with-events` 恢复上下文并继续工作

**TodoWrite：** "修改 auth.rs 文件"（当前会话的临时步骤）
- 生命周期：当前会话
- 会话结束后消失，无法恢复

**Jira：** "PROJ-123: 添加 OAuth2 支持"（团队分配的具体任务）
- 生命周期：一个 Sprint（1-2周）
- 需要手动更新状态和进度

---

## 🚀 快速开始

### 1. 安装

```bash
# 方式 1: Cargo Install（推荐）
cargo install intent-engine

# 方式 2: 下载预编译二进制
# 访问 https://github.com/wayfind/intent-engine/releases

# 验证安装
intent-engine --version
```

> 📖 **详细安装指南**：查看 [INSTALLATION.md](docs/zh-CN/guide/installation.md) 了解所有安装方式、故障排除和集成选项。

### 2. 5 分钟体验核心功能

```bash
# 1. 添加一个战略任务
echo "实现 JWT 认证，支持刷新 Token，有效期 7 天" | \
  intent-engine task add --name "实现用户认证" --spec-stdin

# 2. 开始任务并查看上下文
intent-engine task start 1 --with-events

# 3. 在工作中发现子问题？创建子任务并自动切换
intent-engine task spawn-subtask --name "配置 JWT 密钥"

# 4. 记录关键决策（子任务已是当前任务）
echo "选择使用 HS256 算法，密钥存储在环境变量中" | \
  intent-engine event add --type decision --data-stdin

# 5. 完成子任务
intent-engine task done

# 6. 切回父任务并完成
intent-engine task switch 1
intent-engine task done

# 7. 生成工作报告
intent-engine report --since 1d --summary-only
```

> 💡 **更详细的教程**：参见 [QUICKSTART.md](docs/zh-CN/guide/quickstart.md)

---

## ✨ 核心特性

- **项目感知**：自动向上查找 `.intent-engine` 目录，感知项目根目录
- **惰性初始化**：写入命令自动初始化项目，无需手动 init
- **任务树管理**：支持无限层级的父子任务关系
- **决策历史**：完整的事件流记录（decision、blocker、milestone 等）
- **智能推荐**：`pick-next` 基于上下文推荐下一个任务
- **原子操作**：`start`、`switch`、`spawn-subtask` 等命令节省 50-70% Token
- **🔍 FTS5 搜索引擎**：GB 级任务量下毫秒级响应，独特的 snippet 函数用 `**` 高亮匹配词，对 Agent 上下文极度友好
- **JSON 输出**：所有命令输出结构化 JSON，完美集成 AI 工具

---

## 📚 文档导航

### 🎯 核心文档
- [**INTERFACE_SPEC.md**](docs/INTERFACE_SPEC.md) - **接口规范** (权威定义)
- [**QUICKSTART.md**](QUICKSTART.md) - 5 分钟快速上手

### 🚀 新用户入门
- [**The Intent-Engine Way**](docs/zh-CN/guide/the-intent-engine-way.md) - 设计哲学和协作模式（强烈推荐）
- [**Installation Guide**](docs/zh-CN/guide/installation.md) - 详细安装指南和故障排除

### 🤖 AI 集成
- [**AI Quick Guide**](docs/zh-CN/guide/ai-quick-guide.md) - AI 客户端速查手册
- [**MCP Server**](docs/zh-CN/integration/mcp-server.md) - 集成到 Claude Code/Desktop
- [**Claude Skill**](.claude-code/intent-engine.skill.md) - 轻量级 Claude Code 集成

### 📖 深度学习
- [**Command Reference**](docs/zh-CN/guide/command-reference.md) - 完整命令参考
- [**Task Workflow Analysis**](docs/zh-CN/technical/task-workflow-analysis.md) - Token 优化策略详解
- [**Performance Report**](docs/zh-CN/technical/performance.md) - 性能基准测试
- [**Security Testing**](docs/zh-CN/technical/security.md) - 安全性测试报告
- [**MCP Tools Sync**](docs/zh-CN/technical/mcp-tools-sync.md) - MCP 工具同步系统

### 👥 贡献者
- [**Contributing Guide**](docs/zh-CN/contributing/contributing.md) - 如何贡献代码
- [**Release Process**](docs/zh-CN/contributing/publish-to-crates-io.md) - 发布流程

---

## 🌟 Intent-Engine 的独特价值

### 1. 人机协作的记忆共享层
- 人类设定战略意图（What + Why）
- AI 执行战术任务（How）
- Intent-Engine 记录整个过程

### 2. 跨会话的上下文恢复
- AI 可以随时通过 `task start --with-events` 恢复完整上下文
- 无需人类重复解释背景

### 3. 决策可追溯性
- 每个关键决策都被记录（`event add --type decision`）
- 未来可以回顾"为什么当时选择了方案 A 而不是方案 B"

### 4. 层级化的问题分解
- 支持无限层级的父子任务
- 强制完成所有子任务才能完成父任务

---

## 🛠️ 技术栈

- **语言**：Rust 2021
- **CLI**：clap 4.5
- **数据库**：SQLite with sqlx 0.7
- **异步运行时**：tokio 1.35
- **全文搜索**：SQLite FTS5

---

## 🔧 开发设置

### 首次克隆项目后（贡献者必读）

为了避免 CI 格式检查失败，请立即运行：

```bash
./scripts/setup-git-hooks.sh
```

这会安装 git pre-commit hooks，在每次提交前自动运行 `cargo fmt`，确保代码格式符合规范。

### 开发工具命令

项目提供了 Makefile 简化常用操作：

```bash
make help          # 显示所有可用命令
make fmt           # 格式化代码
make check         # 运行格式化、clippy 和测试
make test          # 运行所有测试
make setup-hooks   # 安装 git hooks（同上述脚本）
```

> 📖 **详细说明**：查看 [scripts/README.md](scripts/README.md) 了解完整的开发工作流和自动化工具。

---

## 🧪 测试

Intent-Engine 包含完整的测试体系：

```bash
# 运行所有测试
cargo test

# 运行性能测试
cargo test --test performance_tests -- --ignored

# 查看测试覆盖率
cargo tarpaulin
```

**测试统计**：116 个测试全部通过 ✅
- 47 个单元测试
- 22 个 CLI 集成测试
- 10 个特殊字符安全性测试
- 37 个性能测试

---

## 📄 许可证

本项目采用 MIT 或 Apache-2.0 双许可证。

- MIT License - 详见 [LICENSE-MIT](LICENSE-MIT)
- Apache License 2.0 - 详见 [LICENSE-APACHE](LICENSE-APACHE)

---


**下一步**：阅读 [The Intent-Engine Way](docs/zh-CN/guide/the-intent-engine-way.md) 深入理解设计哲学，或直接查看 [QUICKSTART.md](QUICKSTART.md) 开始使用。
