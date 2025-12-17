# ie doctor 命令增强功能

## 新增检查项

### 1. MCP 配置检查

检查 `~/.claude.json` 中的 MCP 服务器配置。

**检查项**：
- ✅ 配置文件是否存在
- ✅ `mcpServers.intent-engine` 是否配置
- ✅ binary 路径是否正确
- ✅ binary 是否可执行
- ✅ 环境变量配置是否正确

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

**可能的状态**：
- `✓ PASS`: MCP 配置正确且 binary 可执行
- `⚠ WARNING`: MCP 未配置或 binary 存在但不可执行
- `✗ FAIL`: Binary 路径错误或不存在

### 2. Hooks 配置检查

检查用户级和项目级的 hooks 配置。

**检查项**：
- ✅ Hook 脚本是否存在
- ✅ Hook 脚本是否可执行
- ✅ `settings.json` 是否配置了 SessionStart hook
- ✅ 同时检查用户级和项目级配置

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

**可能的状态**：
- `✓ PASS`: 用户级或项目级有一个配置正确
- `⚠ WARNING`: 两个级别都未配置

## 使用方法

```bash
# 运行健康检查
ie doctor

# 查看 MCP 配置状态
ie doctor | jq '.checks[] | select(.check == "MCP Configuration")'

# 查看 Hooks 配置状态
ie doctor | jq '.checks[] | select(.check == "Hooks Configuration")'
```

## 诊断和修复建议

### MCP 配置问题

**问题**: Binary 路径错误
```json
{
  "status": "✗ FAIL",
  "details": {
    "message": "Binary not found at configured path",
    "binary_path": "/old/path/to/intent-engine"
  }
}
```

**解决方案**：
```bash
# 重新运行 setup 以更新配置
ie setup --target claude-code --force
```

### Hooks 配置问题

**问题**: Hooks 未配置
```json
{
  "status": "⚠ WARNING",
  "details": {
    "message": "Hooks not configured. Run 'ie setup --target claude-code' to configure"
  }
}
```

**解决方案**：
```bash
# 配置用户级 hooks
ie setup --target claude-code

# 或配置项目级 hooks
ie setup --target claude-code --scope project
```

## 完整的 doctor 输出示例

```json
{
  "summary": "✓ All checks passed",
  "overall_status": "healthy",
  "checks": [
    {
      "check": "System Information",
      "status": "✓ PASS",
      "details": "OS: linux, Arch: x86_64"
    },
    {
      "check": "SQLite",
      "status": "✓ PASS",
      "details": "SQLite version: 3.46.0"
    },
    {
      "check": "Database Connection",
      "status": "✓ PASS",
      "details": "Connected to database, 19 tasks found"
    },
    {
      "check": "Intent Engine Version",
      "status": "✓ PASS",
      "details": "v0.3.3"
    },
    {
      "check": "Database Path Resolution",
      "status": "✓ INFO",
      "details": { ... }
    },
    {
      "check": "MCP Configuration",
      "status": "✓ PASS",
      "passed": true,
      "details": { ... }
    },
    {
      "check": "Hooks Configuration",
      "status": "✓ PASS",
      "passed": true,
      "details": { ... }
    }
  ]
}
```

## 健康度评估

doctor 命令会综合所有检查项给出总体健康状态：

- **healthy**: 所有必需检查项通过
- **unhealthy**: 存在失败的检查项

**注意**: Hooks 配置是可选的，即使未配置也不会导致整体状态为 unhealthy。

## 与 setup 命令的集成

doctor 命令会在检查失败时提供修复建议：

```bash
# 如果 MCP 未配置，doctor 会提示
ie doctor
# 输出: "message": "MCP not configured. Run 'ie setup --target claude-code' to configure"

# 按照提示运行 setup
ie setup --target claude-code

# 再次验证
ie doctor
# 输出: "status": "✓ PASS"
```

## 实现细节

### MCP 检查逻辑

1. 检查 `~/.claude.json` 是否存在
2. 解析 JSON 配置
3. 查找 `mcpServers.intent-engine` 配置
4. 验证 binary 路径的有效性
5. 检查文件是否可执行（Unix 平台）
6. 验证环境变量配置

### Hooks 检查逻辑

1. 同时检查用户级（`~/.claude/`）和项目级（`./.claude/`）
2. 验证 hook 脚本存在且可执行
3. 检查 `settings.json` 中的 SessionStart 配置
4. 只要有一个级别配置正确即通过

## 相关文件

- `src/main.rs:426-703` - 检查函数实现
- `src/setup/claude_code.rs` - Setup 模块
- `src/setup/common.rs` - 公共工具函数

## 版本信息

- 添加版本: v0.3.3
- 相关任务: #18, #19
