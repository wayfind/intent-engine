# 日志系统指南

**版本**: 0.6.4+  
**状态**: Stable

---

## 概述

intent-engine 提供统一的文件日志系统，用于记录 Dashboard 和 MCP Server 的运行状态、调试信息和错误诊断。

### 主要特性

- ✅ **自动文件日志**: Dashboard daemon 和 MCP Server 自动记录日志到文件
- ✅ **日志轮转**: 每日自动轮转，防止日志文件无限增长
- ✅ **自动清理**: 定期清理过期日志（默认保留 7 天）
- ✅ **结构化格式**: JSON 格式（MCP Server）或纯文本格式（Dashboard）
- ✅ **调试支持**: 完整的 DEBUG 级别日志用于问题排查

---

## 日志文件位置

所有日志文件存储在：

```
~/.intent-engine/logs/
├── dashboard.log           # 当前 Dashboard 日志
├── dashboard.log.2025-11-22  # 轮转的旧日志
├── mcp-server.log          # 当前 MCP Server 日志
└── mcp-server.log.2025-11-21 # 轮转的旧日志
```

### 按运行模式分类

| 模式 | 日志文件 | 格式 | 说明 |
|------|---------|------|------|
| **Dashboard (daemon)** | `dashboard.log` | 纯文本 | 后台服务日志，包含 WebSocket 连接、项目注册等 |
| **MCP Server** | `mcp-server.log` | JSON | MCP 协议日志，便于机器解析 |

---

## 日志格式

### Dashboard 日志格式

纯文本格式，每行包含：

```
2025-11-22T07:18:55.869164+00:00 INFO intent_engine::dashboard::registry: Registry saved and verified successfully
```

**字段说明**:
- **时间戳**: RFC3339 格式（UTC）
- **级别**: ERROR, WARN, INFO, DEBUG
- **目标模块**: Rust 模块路径
- **消息内容**: 日志消息

### MCP Server 日志格式

JSON 格式，每行一个 JSON 对象：

```json
{
  "timestamp": "2025-11-22T07:18:55.869164Z",
  "level": "DEBUG",
  "fields": {
    "message": "Connected to Dashboard at ws://127.0.0.1:11391/ws/mcp"
  },
  "target": "intent_engine::mcp::ws_client"
}
```

**优势**: 便于使用 `jq` 等工具解析和过滤。

---

## 查看日志

### 查看最新日志

```bash
# Dashboard 日志
tail -f ~/.intent-engine/logs/dashboard.log

# MCP Server 日志（带 JSON 格式化）
tail -f ~/.intent-engine/logs/mcp-server.log | jq .
```

### 搜索特定内容

```bash
# 查找错误
grep ERROR ~/.intent-engine/logs/dashboard.log

# 查找 MCP 操作日志
jq 'select(.fields.message | contains("Dashboard"))' ~/.intent-engine/logs/mcp-server.log
```

### 查看轮转的旧日志

```bash
# 列出所有日志文件
ls -lht ~/.intent-engine/logs/

# 查看昨天的日志
cat ~/.intent-engine/logs/dashboard.log.2025-11-21
```

---

## 日志轮转和清理

### 轮转机制

- **策略**: 每日轮转 (daily rotation)
- **格式**: `{name}.log.YYYY-MM-DD`
- **时机**: 每天午夜（UTC）自动轮转
- **当前日志**: 始终为 `{name}.log` (不带日期后缀)

**示例**:
```
dashboard.log           ← 当前日志（今天）
dashboard.log.2025-11-22 ← 昨天的日志
dashboard.log.2025-11-21 ← 前天的日志
```

### 自动清理

Dashboard 启动时自动清理超过保留期的日志文件。

**默认配置**:
- 保留期: 7 天
- 清理对象: 仅 `*.log.YYYY-MM-DD` 格式的文件
- 其他文件: 不受影响

**自定义保留期**:

```bash
# 设置保留 14 天
export IE_LOG_RETENTION_DAYS=14
ie dashboard start
```

### 手动清理

```bash
# 删除 30 天前的日志
find ~/.intent-engine/logs -name "*.log.*" -mtime +30 -delete

# 查看将被删除的文件（不实际删除）
find ~/.intent-engine/logs -name "*.log.*" -mtime +7 -ls
```

---

## 环境变量配置

### IE_LOG_RETENTION_DAYS

控制日志保留天数（Dashboard 模式）。

```bash
# 保留 30 天
export IE_LOG_RETENTION_DAYS=30
ie dashboard start
```

**默认值**: `7`

### IE_DASHBOARD_LOG_FILE

强制启用 Dashboard 文件日志（主要用于测试）。

```bash
# 即使在前台模式也写入日志文件
export IE_DASHBOARD_LOG_FILE=1
ie dashboard start --foreground
```

**用途**: 调试、测试场景

### RUST_LOG

控制日志级别和过滤。

```bash
# 启用 DEBUG 级别
export RUST_LOG=debug
ie dashboard start

# 只显示 registry 模块的日志
export RUST_LOG=intent_engine::dashboard::registry=debug
ie dashboard start
```

**可用级别**: `error`, `warn`, `info`, `debug`, `trace`

---

## 常见问题排查

### 找不到日志文件

**问题**: `~/.intent-engine/logs/` 目录不存在

**解决**:
- Dashboard 或 MCP Server 会在首次启动时自动创建目录
- 手动创建: `mkdir -p ~/.intent-engine/logs`

### 日志文件为空

**问题**: 日志文件存在但没有内容

**可能原因**:
1. **日志级别过高**: MCP Server 大部分日志是 DEBUG 级别
   - **解决**: 已在 v0.6.4+ 默认启用 DEBUG 级别

2. **进程刚启动**: 等待几秒让日志写入
   - **解决**: 稍等片刻，触发一些操作（如 `ie task list`）

### 日志文件过大

**问题**: 日志文件占用太多磁盘空间

**解决**:
1. 检查轮转是否工作: `ls -lh ~/.intent-engine/logs/`
2. 减少保留期: `export IE_LOG_RETENTION_DAYS=3`
3. 手动清理旧日志: `find ~/.intent-engine/logs -name "*.log.*" -mtime +7 -delete`

### MCP Server 日志看不到操作细节

**问题**: 日志中看不到具体的 MCP 操作

**原因**: 日志级别为 INFO（部分日志是 DEBUG）

**解决**:
```bash
# 临时启用 DEBUG 级别
export RUST_LOG=debug
# 重启 Claude Code 以重新启动 MCP Server
```

---

## 日志内容说明

### Dashboard 日志包含

- ✅ WebSocket 服务器启动/关闭
- ✅ MCP 客户端连接/断开
- ✅ 项目注册/注销
- ✅ Registry 文件读写和验证
- ✅ 错误和警告信息

### MCP Server 日志包含

- ✅ Dashboard 连接状态
- ✅ WebSocket 通信
- ✅ 项目注册信息
- ✅ 数据库操作（DEBUG 级别）
- ✅ Registry 备份和验证

---

## 最佳实践

### 1. 定期检查日志

```bash
# 每周查看一次错误日志
grep ERROR ~/.intent-engine/logs/dashboard.log
```

### 2. 问题排查时启用 DEBUG

```bash
export RUST_LOG=debug
ie dashboard stop
ie dashboard start
```

### 3. 使用 jq 解析 JSON 日志

```bash
# 安装 jq
sudo apt install jq  # Ubuntu/Debian
brew install jq      # macOS

# 查看所有 ERROR 级别日志
jq 'select(.level == "ERROR")' ~/.intent-engine/logs/mcp-server.log

# 查看特定时间范围
jq 'select(.timestamp > "2025-11-22T07:00:00Z")' ~/.intent-engine/logs/mcp-server.log
```

### 4. 归档重要日志

```bash
# 压缩保存
tar -czf logs-backup-$(date +%Y%m%d).tar.gz ~/.intent-engine/logs/*.log
```

---

## 技术细节

### 日志库

- **tracing**: 结构化日志框架
- **tracing-subscriber**: 订阅者和格式化
- **tracing-appender**: 文件输出和轮转

### 轮转实现

使用 `tracing-appender::rolling::daily`:
- 基于日期的文件命名
- 午夜（UTC）自动切换文件
- 零停机时间

### 清理实现

- 扫描 `~/.intent-engine/logs/` 目录
- 检查文件名匹配 `*.log.YYYY-MM-DD` 模式
- 比较文件修改时间与保留期
- 删除超期文件，记录清理统计

---

## 相关资源

- [故障排查指南](../troubleshooting.md)
- [MCP Server 集成](../integration/mcp-server.md)
- [Dashboard 使用](./quickstart.md#dashboard)

---

**有问题？** 查看 [GitHub Issues](https://github.com/david-d-h/intent-engine/issues) 或提交新问题。
