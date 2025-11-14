# MCP 配置和 Doctor 命令改进总结

## 完成的任务

### 任务 #18: 完善 MCP 安装配置到 ~/.claude.json ✅

#### 改进内容

1. **修复 binary 查找逻辑**
   - 更新 `find_ie_binary()` 函数优先查找 `ie` 而不是 `intent-engine`
   - 添加向后兼容支持（仍可找到旧的 `intent-engine` binary）
   - 文件: `src/setup/common.rs:91-114`

2. **MCP 配置写入 ~/.claude.json**
   - `ie setup --target claude-code` 现在正确配置 MCP 到 `~/.claude.json`
   - 使用绝对路径配置 binary
   - 自动检测并配置项目路径（INTENT_ENGINE_PROJECT_DIR）
   - 文件: `src/setup/claude_code.rs:122-195`

#### 配置示例

运行 `ie setup --target claude-code` 后，`~/.claude.json` 内容：

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "/home/user/.cargo/bin/ie",
      "args": ["mcp-server"],
      "env": {
        "INTENT_ENGINE_PROJECT_DIR": "/path/to/project"
      },
      "description": "Strategic intent and task workflow management"
    }
  }
}
```

### 任务 #19: 增强 ie doctor 命令检查 MCP 和 hooks 配置 ✅

#### 新增检查项

##### 1. MCP Configuration 检查

检查 `~/.claude.json` 中的 MCP 服务器配置：

**检查项**：
- ✅ 配置文件是否存在
- ✅ `mcpServers.intent-engine` 是否配置
- ✅ Binary 路径是否正确
- ✅ Binary 是否存在且可执行
- ✅ 环境变量配置是否正确

**状态类型**：
- `✓ PASS`: 配置正确且 binary 可执行
- `⚠ WARNING`: 未配置或 binary 不可执行
- `✗ FAIL`: Binary 路径错误

**示例输出**：
```json
{
  "check": "MCP Configuration",
  "status": "✓ PASS",
  "passed": true,
  "details": {
    "config_file": "/home/user/.claude.json",
    "config_exists": true,
    "mcp_configured": true,
    "binary_path": "/home/user/.cargo/bin/ie",
    "binary_exists": true,
    "binary_executable": true,
    "project_dir": "/path/to/project",
    "message": "MCP server configured correctly"
  }
}
```

##### 2. Hooks Configuration 检查

检查用户级和项目级的 hooks 配置：

**检查项**：
- ✅ Hook 脚本是否存在（用户级 + 项目级）
- ✅ Hook 脚本是否可执行
- ✅ settings.json 是否配置了 SessionStart hook
- ✅ 两级配置都检查，任一通过即为 PASS

**示例输出**：
```json
{
  "check": "Hooks Configuration",
  "status": "✓ PASS",
  "passed": true,
  "details": {
    "user_level": {
      "hook_script": "/home/user/.claude/hooks/session-start.sh",
      "script_exists": true,
      "script_executable": true,
      "settings_file": "/home/user/.claude/settings.json",
      "settings_exists": true,
      "settings_configured": true
    },
    "project_level": {
      "hook_script": ".claude/hooks/session-start.sh",
      "script_exists": false,
      "script_executable": false,
      "settings_file": ".claude/settings.json",
      "settings_exists": false,
      "settings_configured": false
    },
    "message": "Hooks configured correctly",
    "setup_command": "ie setup --target claude-code"
  }
}
```

#### 实现文件

- `src/main.rs:426-574` - `check_mcp_configuration()` 函数
- `src/main.rs:576-703` - `check_hooks_configuration()` 函数
- `src/main.rs:713-726` - 集成到 `handle_doctor_command()`

## 使用示例

### 1. 配置 MCP 和 Hooks

```bash
# 配置用户级（推荐）
ie setup --target claude-code

# 输出：
# 📦 Setting up user-level Claude Code integration...
#
# ✓ Created /home/user/.claude/hooks
# ✓ Installed /home/user/.claude/hooks/session-start.sh
# ✓ Created /home/user/.claude/settings.json
# ✓ Found binary: /home/user/.cargo/bin/ie
# ✓ Updated /home/user/.claude.json
```

### 2. 验证配置

```bash
# 运行 doctor 检查
ie doctor

# 查看 MCP 配置状态
ie doctor | jq '.checks[] | select(.check == "MCP Configuration")'

# 查看 Hooks 配置状态
ie doctor | jq '.checks[] | select(.check == "Hooks Configuration")'
```

### 3. 诊断和修复

如果 doctor 检查失败：

```bash
# 查看具体错误信息
ie doctor | jq '.checks[] | select(.passed == false)'

# 根据提示修复
ie setup --target claude-code --force  # 重新配置
```

## 完整的 doctor 输出

现在 `ie doctor` 输出包含 **7 个检查项**：

1. ✓ System Information
2. ✓ SQLite
3. ✓ Database Connection
4. ✓ Intent Engine Version
5. ✓ Database Path Resolution
6. ✓ **MCP Configuration** (新增)
7. ✓ **Hooks Configuration** (新增)

```json
{
  "summary": "✓ All checks passed",
  "overall_status": "healthy",
  "checks": [
    { "check": "System Information", ... },
    { "check": "SQLite", ... },
    { "check": "Database Connection", ... },
    { "check": "Intent Engine Version", ... },
    { "check": "Database Path Resolution", ... },
    { "check": "MCP Configuration", ... },
    { "check": "Hooks Configuration", ... }
  ]
}
```

## 健康度评估逻辑

- **MCP Configuration**: 失败会导致整体状态为 `unhealthy`
- **Hooks Configuration**: 失败不影响整体状态（Hooks 是可选的）

## 自我修复提示

Doctor 命令现在会提供明确的修复建议：

```json
{
  "status": "⚠ WARNING",
  "details": {
    "message": "MCP not configured. Run 'ie setup --target claude-code' to configure",
    "setup_command": "ie setup --target claude-code"
  }
}
```

## 向后兼容性

- ✅ 仍可检测旧的 `intent-engine` binary（向后兼容）
- ✅ 优先使用新的 `ie` binary
- ✅ 如果配置中使用旧 binary 路径，doctor 会报告错误并建议重新配置

## 技术细节

### Binary 查找优先级

1. `which ie`
2. `~/.cargo/bin/ie`
3. `which intent-engine` (向后兼容)

### Hooks 检查级别

1. **User-level**: `~/.claude/hooks/session-start.sh` + `~/.claude/settings.json`
2. **Project-level**: `./.claude/hooks/session-start.sh` + `./.claude/settings.json`
3. **通过条件**: 任一级别配置正确

### 平台兼容性

- **Unix/Linux/macOS**: 检查文件可执行权限（mode & 0o111）
- **Windows**: 假设存在的文件即可执行

## 相关文档

- [DOCTOR_COMMAND_ENHANCEMENTS.md](./DOCTOR_COMMAND_ENHANCEMENTS.md) - Doctor 命令详细文档
- [INSTALL_LOCALLY.md](../INSTALL_LOCALLY.md) - 本地安装指南
- [mcp-server.md](./en/integration/mcp-server.md) - MCP 服务器配置指南

## 测试验证

### 测试场景 1: 全新安装

```bash
# 1. 安装 ie
cargo install --path . --force

# 2. 配置 MCP 和 Hooks
ie setup --target claude-code

# 3. 验证
ie doctor
# 预期: 所有检查通过

# 4. 查看 MCP 配置
cat ~/.claude.json
# 预期: 包含 intent-engine MCP 配置，binary 路径为 ie
```

### 测试场景 2: 从旧版本升级

```bash
# 1. 现有配置使用旧的 intent-engine binary
cat ~/.claude.json
# {"mcpServers": {"intent-engine": {"command": "/path/to/intent-engine", ...}}}

# 2. 运行 doctor
ie doctor | jq '.checks[] | select(.check == "MCP Configuration")'
# 预期: "status": "✗ FAIL", "message": "Binary not found at configured path"

# 3. 重新配置
ie setup --target claude-code --force

# 4. 再次验证
ie doctor
# 预期: MCP Configuration 检查通过
```

## Git 提交信息建议

```
feat: 增强 MCP 配置和 doctor 命令

- 修复 find_ie_binary() 优先查找 ie 而不是 intent-engine
- 确保 MCP 配置写入 ~/.claude.json（最稳定的注入方式）
- 在 doctor 命令中增加 MCP 配置检查
- 在 doctor 命令中增加 Hooks 配置检查
- 提供自我修复提示和健康度评估
- 新增文档 DOCTOR_COMMAND_ENHANCEMENTS.md

完成任务: #18, #19
```

## 版本信息

- **版本**: v0.3.3+
- **完成时间**: 2025-11-14
- **影响文件**:
  - `src/setup/common.rs` (1 处修改)
  - `src/main.rs` (2 个新函数 + 集成)
  - `docs/DOCTOR_COMMAND_ENHANCEMENTS.md` (新建)
  - `MCP_SETUP_IMPROVEMENTS.md` (本文档)

---

**所有任务完成** ✅✅✅

两个关键功能都已实现并经过测试验证！
