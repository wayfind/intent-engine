# 在 Claude Code 中使用 Intent-Engine（System Prompt 方式）

**版本**: v0.10.0+
**更新日期**: 2025-12-16

---

## 概述

从 v0.10.0 开始，Intent-Engine 使用 **system prompt** 替代 MCP 来集成 Claude Code。优势：

- ✅ **零上下文开销** - 无 MCP 工具定义
- ✅ **简单设置** - 一条命令配置完成
- ✅ **完整 CLI 访问** - 直接执行 shell 命令
- ✅ **可离线使用** - 无需后台进程

---

## 快速开始

### 1. 安装 Intent-Engine

```bash
cargo install intent-engine
# 或
brew install intent-engine
```

### 2. 初始化项目

```bash
cd your-project
ie init
```

### 3. 配置 Claude Code

```bash
# 方案 A: 追加 system prompt（如果持久化）
claude --append-system-prompt "$(cat /path/to/intent-engine/system-prompt.txt)"

# 方案 B: 使用 hook 脚本（如果不持久化）
# 见下方"Hook 脚本设置"部分
```

### 4. 验证设置

询问 Claude Code：
```
"什么是 intent-engine，如何使用？"
```

Claude 应该回答关于 IE 命令和使用模式的信息。

---

## System Prompt 持久化

### 测试持久化

检查 `--append-system-prompt` 是否在会话间持久：

1. 执行上面的 append 命令
2. 重启 Claude Code
3. 询问 Claude："你知道 intent-engine 吗？"

**如果 Claude 记得**: System prompt 是持久的 ✅
**如果 Claude 不记得**: 使用 hook 脚本方法 ❌

---

## Hook 脚本设置（如果不持久化）

如果 system prompt 不持久化，使用 Claude Code hook：

### 方法 1: Session Start Hook

创建 `.claude/hooks/session-start.sh`:

```bash
#!/usr/bin/env bash
# 在会话启动时加载 Intent-Engine system prompt

SYSTEM_PROMPT_FILE="${HOME}/.intent-engine/system-prompt.txt"

if [ ! -f "$SYSTEM_PROMPT_FILE" ]; then
    # 从安装目录复制
    IE_ROOT=$(dirname $(which ie))/../share/intent-engine
    if [ -f "$IE_ROOT/system-prompt.txt" ]; then
        mkdir -p "${HOME}/.intent-engine"
        cp "$IE_ROOT/system-prompt.txt" "$SYSTEM_PROMPT_FILE"
    fi
fi

if [ -f "$SYSTEM_PROMPT_FILE" ]; then
    export CLAUDE_SYSTEM_PROMPT="$(cat $SYSTEM_PROMPT_FILE)"
fi
```

添加执行权限：
```bash
chmod +x .claude/hooks/session-start.sh
```

### 方法 2: 环境变量

添加到 shell 配置文件（`~/.bashrc`, `~/.zshrc`）：

```bash
# Intent-Engine system prompt
if [ -f "$HOME/.intent-engine/system-prompt.txt" ]; then
    export CLAUDE_SYSTEM_PROMPT="$(cat $HOME/.intent-engine/system-prompt.txt)"
fi
```

重新加载 shell：
```bash
source ~/.bashrc  # 或 ~/.zshrc
```

---

## 使用示例

### Plan-First 工作流（主要方式 - 90%）⭐

```bash
# 使用状态工作流创建任务
echo '{
  "tasks": [{
    "name": "实现用户认证",
    "status": "doing",
    "priority": "high",
    "children": [
      {"name": "设计 JWT schema", "status": "todo"},
      {"name": "实现 token 生成", "status": "todo"}
    ]
  }]
}' | ie plan

# Plan 自动设置焦点，开始工作
ie log decision "使用 JWT 实现无状态认证"

# 完成时更新状态
echo '{"tasks": [{"name": "设计 JWT schema", "status": "done"}]}' | ie plan
```

为什么用 plan-first:
- ✅ 幂等（可安全重复执行）
- ✅ 内置状态跟踪
- ✅ 强制层级结构
- ✅ 自动聚焦 "doing" 任务

### 传统工作流（高级用法 - 10%）

```bash
# 用于单个快速任务或动态工作流
ie add "快速修复"
ie start 1
ie done
```

### 上下文恢复

```bash
# 使用 plan 恢复工作
ie search "认证"
echo '{"tasks": [{"name": "实现用户认证", "status": "doing"}]}' | ie plan
ie event list --task-id 1  # 查看决策
```

---

## 故障排查

### Claude 不识别 IE 命令

**症状**: Claude 回复"我不知道 intent-engine"

**解决方案**：
1. 验证 system prompt 已加载：
   ```bash
   cat ~/.intent-engine/system-prompt.txt
   ```
2. 检查 Claude Code 配置
3. 尝试使用 hook 脚本方法

### 命令不工作

**症状**: `ie` 命令未找到

**解决方案**：
1. 验证安装：
   ```bash
   which ie
   ie --version
   ```
2. 如需要，添加到 PATH：
   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   ```

### Plan Status Constraint 错误

**症状**: 错误 "Only one task can have status='doing'"

**原因**: 同一个 plan 请求中有多个任务的 `status: "doing"`

**解决方案**:
```bash
# ❌ 错误: 两个 doing 任务
echo '{"tasks": [
  {"name": "A", "status": "doing"},
  {"name": "B", "status": "doing"}
]}' | ie plan

# ✅ 正确: 一个 doing 任务
echo '{"tasks": [
  {"name": "A", "status": "doing"},
  {"name": "B", "status": "todo"}
]}' | ie plan
```

原因: 强制单一焦点工作流（每次批量操作只能有一个 doing 任务）

### 层级 Doing 场景（父子任务同时进行）

**症状**: 需要父任务和子任务同时处于 'doing' 状态

**理解**:
- **数据库层级**: 支持多个 'doing' 任务（父任务 + 聚焦的子任务）
- **Plan API 层级**: 每次请求只能有一个 'doing' 任务

**解决方案** - 使用分开的 plan 调用:
```bash
# 步骤 1: 设置父任务为 doing
echo '{"tasks": [{"name": "实现用户认证", "status": "doing"}]}' | ie plan
# 输出: Task ID 42

# 步骤 2: 设置子任务为 doing（单独请求）
echo '{"tasks": [{"name": "设计 JWT schema", "status": "doing"}]}' | ie plan
# 输出: Task ID 43
# 数据库现在有两个 doing: 父任务 (42) + 子任务 (43, 聚焦)
```

**为什么要分开请求**: Plan API 强制每次批量操作只能有一个 'doing' 任务，但数据库支持层级工作流。每次 plan 调用只能标记一个任务为 'doing'。

---

## 最佳实践

### 1. IE 升级后更新 System Prompt

```bash
ie doctor  # 检查更新
# 如果更新了，刷新 system prompt
claude --append-system-prompt "$(cat /path/to/system-prompt.txt)"
```

### 2. 项目特定初始化

在每个项目中运行 `ie init`：
```bash
cd new-project
ie init
```

### 3. 使用 Dashboard 可视化

```bash
ie dashboard start
# 打开 http://localhost:11391
```

### 4. 定期上下文回顾

```bash
ie report --since 7d  # 每周回顾
```

---

## 从 MCP 迁移 (v0.9.0 → v0.10.0)

### 变更内容

- ❌ **移除**: MCP server 和 tools
- ✅ **新增**: System prompt 方式
- ✅ **相同**: 所有 CLI 命令保持不变
- ✅ **简化**: 单向通讯（CLI → Dashboard）

### 架构变化

**旧版 (v0.9.0)**:
```
Claude Code → MCP Server → Dashboard ←→ 前端
              (持久连接、心跳)
```

**新版 (v0.10.0+)**:
```
Claude Code (ie CLI) → 本地 SQLite DB
                            ↓ (单向通知)
                       全局 Dashboard
                            ↓ (直接访问)
                       所有项目 DB ← 前端
```

**关键改进**:
- 无需持久连接
- 无"在线/离线"项目状态
- Dashboard 可直接在任意项目中创建/修改任务
- CLI 操作完全支持离线

### 迁移步骤

1. **卸载旧 MCP 配置**：
   ```bash
   # 从 Claude Code MCP 设置中删除
   # 删除 mcp-server.json 引用
   ```

2. **安装新版本**：
   ```bash
   cargo install intent-engine
   ie --version  # 应为 v0.10.0+
   ```

3. **配置 system prompt**：
   ```bash
   claude --append-system-prompt "$(cat system-prompt.txt)"
   ```

4. **可选 - 启动全局 dashboard**：
   ```bash
   ie dashboard start
   # Dashboard 现在监控所有项目（无需每项目单独服务器）
   ```

5. **验证**：
   询问 Claude："如何使用 intent-engine？"

### 破坏性变更

- MCP tools 不再可用
- 使用 CLI 命令替代（所有功能保留）
- Dashboard 通讯改为单向（CLI → Dashboard）
- 项目不再有"在线/离线"状态

---

## 常见问题

**Q: 还能使用 MCP 吗？**
A: 不能，v0.10.0 移除了 MCP 支持。使用 system prompt + CLI。

**Q: 需要 Dashboard 运行吗？**
A: 不需要，CLI 独立工作。Dashboard 仅用于可视化（可选）。

**Q: 占用多少上下文？**
A: ~345 行（~3-4K tokens），远少于 MCP tools（~15K tokens）。

**Q: 可以自定义 system prompt 吗？**
A: 可以，编辑 `system-prompt.txt` 并重新加载。保持核心部分完整。

**Q: 可以离线使用吗？**
A: 可以，CLI 操作无需网络连接。

---

## 参见

- [快速开始指南](../guide/quickstart.md)
- [命令参考](../guide/command-reference-full.md)
- [迁移指南](../../../MIGRATION_v0.10.0.md)

---

**需要帮助？**
- GitHub Issues: https://github.com/user/intent-engine/issues
- 文档: https://intent-engine.dev
