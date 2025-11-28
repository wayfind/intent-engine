# Intent-Engine MCP Server 配置指南

本指南介绍如何将 Intent-Engine 作为 MCP (Model Context Protocol) 服务器添加到 Claude Code 中。

## 前置条件

1. **Rust 工具链**: 用于构建 MCP 服务器二进制文件
2. **Claude Code/Claude Desktop**: 支持 MCP 的 AI 助手应用

> **注意**: Intent-Engine 使用 **Rust 原生 MCP 服务器**,无需 Python 依赖,性能更优,启动更快。

## 安装方式

### 方式一: 快速安装 (推荐)

```bash
# 克隆或下载 Intent-Engine
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine

# 构建并安装 (单一二进制，包含 CLI 和 MCP 服务器)
cargo install --path .

# 运行自动配置脚本
./scripts/install/install-mcp-server.sh
```

自动配置脚本会:
- ✅ 检测操作系统并找到正确的配置目录
- ✅ 自动定位 MCP 服务器二进制文件
- ✅ 备份现有配置 (如果存在)
- ✅ 创建或更新 `mcp_servers.json` 配置

### 方式二: 手动配置

#### 步骤 1: 构建 MCP 服务器

```bash
# 从源码构建 (单一二进制，包含 CLI 和 MCP 服务器)
cargo build --release

# 安装到用户路径 (推荐)
cargo install --path .
# 安装后路径: ~/.cargo/bin/intent-engine

# 或者复制到系统路径
sudo cp target/release/intent-engine /usr/local/bin/
```

#### 步骤 2: 配置 Claude Code

编辑 Claude Code 的 MCP 配置文件:

- **macOS/Linux/WSL**: `~/.claude.json`
- **Windows**: `%APPDATA%\Claude\.claude.json`

> **⚠️ 版本说明**: Claude Code v2.0.37+ 在 Linux/macOS/WSL 上使用 `~/.claude.json` 作为主配置文件。
> 早期版本可能使用不同的路径，如 `~/.claude/mcp_servers.json` 或 `~/.config/claude-code/mcp_servers.json`。
> 如果安装后 MCP 工具未显示，请验证您的 Claude Code 版本和配置文件位置。

添加 Intent-Engine 服务器配置:

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "/home/user/.cargo/bin/intent-engine",
      "args": ["mcp-server"],
      "description": "Strategic intent and task workflow management for human-AI collaboration"
    }
  }
}
```

**配置说明**:
- `command`: Intent-Engine 二进制文件的绝对路径
  - 使用 `cargo install` 安装: `~/.cargo/bin/intent-engine`
  - 复制到系统路径: `/usr/local/bin/intent-engine`
- `args`: 必须包含 `["mcp-server"]` 以启动 MCP 服务器模式
- 项目目录会自动检测（基于 `.git`, `Cargo.toml` 等项目标记）
- 使用绝对路径确保可靠性

#### 步骤 3: 重启 Claude Code

重启 Claude Code 以加载新的 MCP 服务器。

## 验证安装

### 手动测试 MCP 服务器

```bash
# 测试 JSON-RPC 接口 (从项目目录运行)
cd /path/to/your/project
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
  ie mcp-server

# 应该返回包含 13 个工具的 JSON 响应
# 项目目录会自动通过 .git, Cargo.toml 等标记检测
```

### 在 Claude Code 中验证

启动 Claude Code 后,你应该能看到以下 **13 个 Intent-Engine MCP 工具**:

**任务管理工具**:
- `task_add` - 创建战略任务
- `task_start` - 开始任务 (原子操作: 设为 doing + 设为当前任务)
- `task_pick_next` - 智能推荐下一个任务
- `task_spawn_subtask` - 创建子任务并切换 (原子操作)
- `task_done` - 完成任务 (验证所有子任务已完成)
- `task_update` - 更新任务属性
- `task_list` - 列出任务,支持过滤、排序和分页
  - 支持 `status`、`parent` 过滤
  - 通过 `limit` 和 `offset` 分页
  - 排序选项: `id`、`priority`、`time`、`focus_aware`
  - 返回 `PaginatedTasks`,包含 `has_more` 标志
- `task_get` - 获取任务详细信息

**事件追踪工具**:
- `event_add` - 记录决策/阻碍/里程碑 (AI 的外部长时记忆)
- `event_list` - 列出任务的事件历史 (支持按类型和时间过滤)

**搜索工具**:
- `search` - 跨任务和事件的统一全文搜索
  - FTS5 全文搜索,带片段高亮
  - 通过 `limit` 和 `offset` 支持分页
  - 返回 `PaginatedSearchResults`,包含任务/事件分别计数

**工作流工具**:
- `current_task_get` - 获取当前聚焦的任务
- `report_generate` - 生成工作报告
- `plan` - 声明式批量任务创建,支持依赖关系

## 使用示例

安装完成后,Claude Code 可以自动使用 Intent-Engine:

### 基本任务创建和管理

```
你: "帮我重构认证系统"

Claude: 我会创建一个任务来追踪这项工作。
[使用 task_add 工具]
[使用 task_start 工具并获取事件历史]

"我已经开始了任务 #42。基于代码分析,我看到三个主要部分:
1. 密码哈希 (当前使用 MD5,需要升级到 bcrypt)
2. 会话管理 (没有过期机制)
3. OAuth 集成 (缺失)

让我为每个部分创建子任务..."
[使用 task_spawn_subtask 为每个领域创建子任务]
```

### 分页和过滤

```
你: "显示所有正在进行的任务,按优先级排序"

Claude: 我会列出你的 doing 任务并按优先级排序。
[使用 task_list: {status: 'doing', sort_by: 'priority', limit: 20, offset: 0}]

"你有 15 个正在进行的任务。以下是按优先级排序的前 20 个:
- 任务 #42: 重构认证系统 (优先级: high)
- 任务 #58: 优化数据库查询 (优先级: high)
- 任务 #71: 更新文档 (优先级: medium)
..."

[响应包含分页元数据: total_count: 15, has_more: false]
```

### 带分页的搜索

```
你: "找到所有与 JWT 认证相关的任务和讨论"

Claude: 我会在任务和事件中搜索。
[使用 search: {query: "JWT 认证", limit: 20, offset: 0}]

"找到 8 个任务和 12 个事件与 JWT 认证相关:

任务:
- 任务 #42: 实现基于 JWT 的认证 (在 spec 中匹配)
- 任务 #45: 配置 JWT 密钥轮换 (在 name 中匹配)

事件:
- 任务 #42 的决策: '选择 HS256 用于 JWT 签名...'
- 任务 #45 的阻碍: 'JWT 密钥环境变量...'
..."

[响应包含: total_tasks: 8, total_events: 12, has_more: false]
```

## 技术优势

### 为什么选择 Rust 原生实现?

| 特性 | Rust 原生 MCP 服务器 | Python 包装器 (旧版) |
|------|---------------------|---------------------|
| **启动速度** | < 10ms | 300-500ms |
| **内存占用** | ~5MB | ~30-50MB |
| **依赖** | 零依赖 | 需要 Python 3.7+ |
| **性能** | 原生性能 | 进程间通信开销 |
| **维护性** | 单一代码库 | 双重维护 |

### 架构说明

```
Claude Code (客户端)
      │
      ├─ JSON-RPC 2.0 over stdio ─┐
      │                           │
      ▼                           ▼
ie mcp-server ─────> SQLite
  (Rust 原生,单一二进制)     (.intent-engine/project.db)
```

## 故障排查

### MCP 服务器未显示在 Claude Code 中

**检查清单**:
1. 确认 MCP 配置文件路径正确:
   ```bash
   # Linux/macOS/WSL (Claude Code v2.0.37+)
   cat ~/.claude.json

   # Windows PowerShell (Claude Code v2.0.37+)
   Get-Content $env:APPDATA\Claude\.claude.json

   # 注意: 早期版本可能使用不同的路径:
   # - ~/.claude/mcp_servers.json
   # - ~/.config/claude-code/mcp_servers.json
   ```

2. 验证 JSON 语法有效:
   ```bash
   # 使用 jq 验证 JSON
   jq . ~/.claude.json
   ```

3. 检查二进制文件存在且可执行:
   ```bash
   which intent-engine
   # 应该输出: ~/.cargo/bin/intent-engine

   # 测试运行 (需要指定项目目录)
   cd /path/to/your/project
   echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
     ie mcp-server
   ```

4. 查看 Claude Code 日志:
   ```bash
   # macOS/Linux
   tail -f ~/.claude/logs/mcp.log

   # Windows
   # 查看 %APPDATA%\Claude\logs\
   ```

5. **重启 Claude Code** (必须!)

### 权限问题

```bash
# 确保二进制文件可执行
chmod +x ~/.cargo/bin/intent-engine

# 或者
chmod +x /usr/local/bin/intent-engine
```

### 配置文件路径问题

如果使用相对路径或 `~` 符号可能无法工作,请使用**绝对路径**:

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "/home/username/.cargo/bin/intent-engine",
      "args": ["mcp-server"]
    }
  }
}
```

### 测试 MCP 服务器是否正常工作

```bash
# 完整的测试命令 (从项目目录运行)
cd /path/to/your/project
cat << 'EOF' | ie mcp-server
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
EOF

# 期望输出: 包含 13 个工具的 JSON 响应
# 如果有错误,会在 stderr 输出
# 项目目录会自动检测 (基于 .git, Cargo.toml 等标记)
```

## 卸载

### 移除 MCP 服务器配置

1. 编辑 `~/.claude.json` (或您版本对应的配置文件)
2. 从 `mcpServers` 部分删除 `"intent-engine"` 配置项
3. 重启 Claude Code

### 卸载二进制文件

```bash
# 如果使用 cargo install 安装
cargo uninstall intent-engine

# 如果手动复制到系统路径
sudo rm /usr/local/bin/intent-engine
```

## 相关资源

- [CLAUDE.md](../../../CLAUDE.md) - Claude 集成完整指南
- [INTERFACE_SPEC.md](../../INTERFACE_SPEC.md) - 接口规范 (权威文档)
- [MCP 工具同步系统](../technical/mcp-tools-sync.md) - 维护和测试
- [README.md](../../../README.md) - 项目主页

## 高级配置

### 为不同项目使用不同的 Intent-Engine 数据库

Intent-Engine 支持多项目隔离,每个项目有自己的数据库:

```
/home/user/project-a/.intent-engine/project.db  # 项目 A 的任务
/home/user/project-b/.intent-engine/project.db  # 项目 B 的任务
```

**配置方式**: 使用单一配置即可,Intent-Engine 会根据 Claude Code 的当前工作目录自动发现项目:

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "/home/user/.cargo/bin/intent-engine",
      "args": ["mcp-server"]
    }
  }
}
```

**自动项目发现机制**:
- 如果在项目目录中 (检测到 `.git`, `Cargo.toml` 等标记) → 使用该项目的数据库
- 如果不在项目中 → 向上搜索最近的 `.intent-engine/` 目录
- 数据隔离完全自动,无需手动配置

### 与 Claude Desktop 配合使用

Intent-Engine MCP 服务器同样适用于 Claude Desktop。配置文件路径:

- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

配置格式相同:

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "/home/user/.cargo/bin/intent-engine",
      "args": ["mcp-server"]
    }
  }
}
```

**注意**: 项目目录会根据 Claude Desktop 的工作目录自动检测,无需手动配置。
