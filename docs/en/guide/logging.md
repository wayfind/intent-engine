# Logging System Guide

**Version**: 0.6.4+  
**Status**: Stable

---

## Overview

intent-engine provides a unified file logging system for recording runtime status, debug information, and error diagnostics for Dashboard and MCP Server.

### Key Features

- ✅ **Automatic file logging**: Dashboard daemon and MCP Server automatically log to files
- ✅ **Log rotation**: Daily automatic rotation prevents unlimited file growth
- ✅ **Auto cleanup**: Periodically removes expired logs (default: 7 days retention)
- ✅ **Structured format**: JSON (MCP Server) or plain text (Dashboard)
- ✅ **Debug support**: Full DEBUG level logs for troubleshooting

---

## Log File Locations

All log files are stored in:

```
~/.intent-engine/logs/
├── dashboard.log           # Current Dashboard log
├── dashboard.log.2025-11-22  # Rotated old log
├── mcp-server.log          # Current MCP Server log
└── mcp-server.log.2025-11-21 # Rotated old log
```

### By Running Mode

| Mode | Log File | Format | Description |
|------|---------|--------|-------------|
| **Dashboard (daemon)** | `dashboard.log` | Plain text | Background service logs, includes WebSocket connections, project registry, etc. |
| **MCP Server** | `mcp-server.log` | JSON | MCP protocol logs, machine-parseable |

---

## Log Formats

### Dashboard Log Format

Plain text format, each line contains:

```
2025-11-22T07:18:55.869164+00:00 INFO intent_engine::dashboard::registry: Registry saved and verified successfully
```

**Fields**:
- **Timestamp**: RFC3339 format (UTC)
- **Level**: ERROR, WARN, INFO, DEBUG
- **Target module**: Rust module path
- **Message**: Log message content

### MCP Server Log Format

JSON format, one JSON object per line:

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

**Advantage**: Easy to parse and filter using tools like `jq`.

---

## Viewing Logs

### View Latest Logs

```bash
# Dashboard logs
tail -f ~/.intent-engine/logs/dashboard.log

# MCP Server logs (with JSON formatting)
tail -f ~/.intent-engine/logs/mcp-server.log | jq .
```

### Search for Specific Content

```bash
# Find errors
grep ERROR ~/.intent-engine/logs/dashboard.log

# Find MCP operation logs
jq 'select(.fields.message | contains("Dashboard"))' ~/.intent-engine/logs/mcp-server.log
```

### View Rotated Old Logs

```bash
# List all log files
ls -lht ~/.intent-engine/logs/

# View yesterday's log
cat ~/.intent-engine/logs/dashboard.log.2025-11-21
```

---

## Log Rotation and Cleanup

### Rotation Mechanism

- **Strategy**: Daily rotation
- **Format**: `{name}.log.YYYY-MM-DD`
- **Timing**: Automatically rotates at midnight (UTC)
- **Current log**: Always named `{name}.log` (no date suffix)

**Example**:
```
dashboard.log           ← Current log (today)
dashboard.log.2025-11-22 ← Yesterday's log
dashboard.log.2025-11-21 ← Day before yesterday
```

### Auto Cleanup

Dashboard automatically cleans up expired logs on startup.

**Default configuration**:
- Retention: 7 days
- Cleanup target: Only `*.log.YYYY-MM-DD` format files
- Other files: Unaffected

**Custom retention period**:

```bash
# Set 14-day retention
export IE_LOG_RETENTION_DAYS=14
ie dashboard start
```

### Manual Cleanup

```bash
# Delete logs older than 30 days
find ~/.intent-engine/logs -name "*.log.*" -mtime +30 -delete

# View files that would be deleted (dry run)
find ~/.intent-engine/logs -name "*.log.*" -mtime +7 -ls
```

---

## Environment Variables

### IE_LOG_RETENTION_DAYS

Controls log retention days (Dashboard mode).

```bash
# Retain for 30 days
export IE_LOG_RETENTION_DAYS=30
ie dashboard start
```

**Default**: `7`

### IE_DASHBOARD_LOG_FILE

Force enable Dashboard file logging (mainly for testing).

```bash
# Write to log file even in foreground mode
export IE_DASHBOARD_LOG_FILE=1
ie dashboard start --foreground
```

**Use case**: Debugging, testing scenarios

### RUST_LOG

Controls log level and filtering.

```bash
# Enable DEBUG level
export RUST_LOG=debug
ie dashboard start

# Only show logs from registry module
export RUST_LOG=intent_engine::dashboard::registry=debug
ie dashboard start
```

**Available levels**: `error`, `warn`, `info`, `debug`, `trace`

---

## Troubleshooting

### Log Files Not Found

**Issue**: `~/.intent-engine/logs/` directory doesn't exist

**Solution**:
- Dashboard or MCP Server will auto-create directory on first startup
- Manual creation: `mkdir -p ~/.intent-engine/logs`

### Empty Log Files

**Issue**: Log file exists but has no content

**Possible causes**:
1. **Log level too high**: Most MCP Server logs are DEBUG level
   - **Solution**: DEBUG level enabled by default in v0.6.4+

2. **Process just started**: Wait a few seconds for logs to be written
   - **Solution**: Wait a moment, trigger some operations (like `ie task list`)

### Log Files Too Large

**Issue**: Log files consuming too much disk space

**Solution**:
1. Check if rotation is working: `ls -lh ~/.intent-engine/logs/`
2. Reduce retention: `export IE_LOG_RETENTION_DAYS=3`
3. Manual cleanup: `find ~/.intent-engine/logs -name "*.log.*" -mtime +7 -delete`

### MCP Server Logs Missing Operation Details

**Issue**: Can't see specific MCP operations in logs

**Cause**: Log level is INFO (some logs are DEBUG)

**Solution**:
```bash
# Temporarily enable DEBUG level
export RUST_LOG=debug
# Restart Claude Code to restart MCP Server
```

---

## Log Content

### Dashboard Logs Include

- ✅ WebSocket server start/stop
- ✅ MCP client connect/disconnect
- ✅ Project registration/unregistration
- ✅ Registry file read/write and verification
- ✅ Errors and warnings

### MCP Server Logs Include

- ✅ Dashboard connection status
- ✅ WebSocket communication
- ✅ Project registration info
- ✅ Database operations (DEBUG level)
- ✅ Registry backup and verification

---

## Best Practices

### 1. Regularly Check Logs

```bash
# Check error logs weekly
grep ERROR ~/.intent-engine/logs/dashboard.log
```

### 2. Enable DEBUG When Troubleshooting

```bash
export RUST_LOG=debug
ie dashboard stop
ie dashboard start
```

### 3. Use jq to Parse JSON Logs

```bash
# Install jq
sudo apt install jq  # Ubuntu/Debian
brew install jq      # macOS

# View all ERROR level logs
jq 'select(.level == "ERROR")' ~/.intent-engine/logs/mcp-server.log

# View specific time range
jq 'select(.timestamp > "2025-11-22T07:00:00Z")' ~/.intent-engine/logs/mcp-server.log
```

### 4. Archive Important Logs

```bash
# Compress and save
tar -czf logs-backup-$(date +%Y%m%d).tar.gz ~/.intent-engine/logs/*.log
```

---

## Technical Details

### Logging Libraries

- **tracing**: Structured logging framework
- **tracing-subscriber**: Subscribers and formatters
- **tracing-appender**: File output and rotation

### Rotation Implementation

Uses `tracing-appender::rolling::daily`:
- Date-based file naming
- Automatic file switching at midnight (UTC)
- Zero downtime

### Cleanup Implementation

- Scans `~/.intent-engine/logs/` directory
- Checks filenames matching `*.log.YYYY-MM-DD` pattern
- Compares file modification time with retention period
- Deletes expired files, logs cleanup statistics

---

## Related Resources

- [Troubleshooting Guide](../troubleshooting.md)
- [MCP Server Integration](../integration/mcp-server.md)
- [Dashboard Usage](./quickstart.md#dashboard)

---

**Questions?** Check [GitHub Issues](https://github.com/david-d-h/intent-engine/issues) or submit a new one.
