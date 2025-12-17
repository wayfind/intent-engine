# Intent-Engine Logging System Guide

**Version**: 0.6.6
**Last Updated**: 2025-11-23

---

## Overview

Intent-Engine provides a comprehensive logging system for tracking application behavior across different modes:
- **Dashboard** - Background service logs
- **MCP Server** - AI assistant integration logs
- **CLI** - Command-line operation logs

All logs are automatically stored in `~/.intent-engine/logs/` directory.

---

## Log Files

### Location

```
~/.intent-engine/logs/
├── dashboard.log          # Dashboard daemon logs
├── dashboard.log.YYYY-MM-DD  # Rotated dashboard logs
├── mcp-server.log         # MCP Server logs
├── mcp-server.log.YYYY-MM-DD # Rotated MCP Server logs
└── cli.log                # CLI command logs (optional)
```

### Format

**Dashboard/CLI logs** (Text format):
```
2025-11-23T07:05:51.332508805+00:00  INFO intent_engine::dashboard::server: Dashboard server listening on 127.0.0.1:11391
```

**MCP Server logs** (JSON format):
```json
{"timestamp":"2025-11-23T07:11:01.722215Z","level":"INFO","fields":{"message":"Connecting to Dashboard (attempt 1)..."},"target":"intent_engine::mcp::ws_client"}
```

---

## Viewing Logs

### Using `ie logs` Command

The `ie logs` command provides a unified interface to query and view logs.

#### Basic Usage

```bash
# Show recent logs from all modes (last 24 hours by default)
ie logs

# Show recent dashboard logs
ie logs --mode dashboard

# Show recent MCP Server logs
ie logs --mode mcp-server
```

#### Filtering by Level

```bash
# Show only errors
ie logs --level error

# Show errors and warnings from dashboard
ie logs --mode dashboard --level warn

# Show debug logs
ie logs --mode mcp-server --level debug
```

Supported levels: `error`, `warn`, `info`, `debug`, `trace`

#### Filtering by Time

```bash
# Show logs from last hour
ie logs --since 1h

# Show logs from last 24 hours
ie logs --since 24h

# Show logs from last 7 days
ie logs --since 7d

# Combine with mode and level
ie logs --mode dashboard --level error --since 1h
```

Time formats:
- `Xs` - seconds (e.g., `30s`)
- `Xm` - minutes (e.g., `30m`)
- `Xh` - hours (e.g., `24h`)
- `Xd` - days (e.g., `7d`)

#### Limiting Results

```bash
# Show only latest 10 entries
ie logs --limit 10

# Combine with filters
ie logs --mode dashboard --limit 50
```

#### Real-time Monitoring

```bash
# Follow logs in real-time (like tail -f)
ie logs --follow
ie logs -f

# Follow specific mode
ie logs --mode dashboard --follow

# Follow and filter by level
ie logs --level error --follow
```

Press `Ctrl+C` to stop following.

#### Export Formats

```bash
# Export as JSON (default: text)
ie logs --export json

# Save to file
ie logs --mode dashboard --export json > dashboard.json
ie logs --mode dashboard > dashboard.txt
```

---

## Log Rotation

### Built-in Rotation

Intent-Engine uses **daily log rotation** by default. Each day, a new log file is created with the date suffix:
- `dashboard.log` - Current day
- `dashboard.log.2025-11-22` - Previous day

### Using logrotate (Recommended for Linux)

For production deployments on Linux, we recommend using `logrotate` for more advanced rotation:

**Installation**:
```bash
sudo cp docs/deployment/logrotate.conf /etc/logrotate.d/intent-engine
sudo chmod 644 /etc/logrotate.d/intent-engine
```

**Test configuration**:
```bash
sudo logrotate -d /etc/logrotate.d/intent-engine
```

**Force rotation** (for testing):
```bash
sudo logrotate -f /etc/logrotate.d/intent-engine
```

**Configuration** (`docs/deployment/logrotate.conf`):
- Rotates daily
- Keeps 7 days of logs
- Compresses old logs
- Sends SIGHUP to dashboard/MCP server to reopen log files

### Manual Cleanup

Old log files (`.log.YYYY-MM-DD`) can be safely deleted:
```bash
# Remove logs older than 7 days
find ~/.intent-engine/logs -name "*.log.*" -mtime +7 -delete
```

---

## Log Levels

### Default Levels by Mode

| Mode | Default Level | Description |
|------|--------------|-------------|
| CLI | INFO | User-facing operations |
| Dashboard | INFO | Service status and operations |
| MCP Server | DEBUG | Detailed debugging for AI tools |
| Test | DEBUG | Maximum verbosity for testing |

### Changing Log Level

**Via Environment Variable** (applies to all modes):
```bash
export RUST_LOG=debug
ie dashboard start

# Or for specific module
export RUST_LOG=intent_engine::dashboard=debug
ie dashboard start
```

**Via CLI Flags** (CLI mode only):
```bash
# Verbose output (DEBUG level)
ie -v task list

# Very verbose (TRACE level)
ie -vv task list

# Quiet mode (ERROR level only)
ie -q task list
```

---

## Common Use Cases

### Debugging Dashboard Issues

```bash
# Check if dashboard started successfully
ie logs --mode dashboard --since 10m

# Look for errors during startup
ie logs --mode dashboard --level error

# Monitor dashboard in real-time
ie logs --mode dashboard --follow
```

### Investigating MCP Server Connection Issues

```bash
# Check MCP Server connection logs
ie logs --mode mcp-server --since 1h | grep -i "connect"

# Monitor MCP requests in real-time
ie logs --mode mcp-server --follow --level debug

# Export for analysis
ie logs --mode mcp-server --since 24h --export json > mcp-debug.json
```

### Troubleshooting Registry Issues

```bash
# Look for registry-related errors
ie logs --level error | grep -i "registry"

# Check registry backup operations
ie logs --mode dashboard --since 1h | grep -i "backup"

# See registry verification logs (DEBUG level required)
RUST_LOG=debug ie dashboard start
ie logs --mode dashboard --level debug | grep -i "registry"
```

### Audit Trail

```bash
# Export all recent operations
ie logs --since 7d --export json > audit.json

# Review specific time period
ie logs --since 24h --until "2025-11-23T12:00:00Z"
```

---

## File Logging Behavior

### Dashboard Mode

| Scenario | File Logging | Console Output |
|----------|-------------|----------------|
| Foreground (TTY) | No | Yes (to stdout) |
| Daemon mode | Yes | No (to /dev/null) |
| Force via env var | Yes | Depends |

**Force file logging** (for testing):
```bash
IE_DASHBOARD_LOG_FILE=1 ie dashboard start --foreground
```

### MCP Server Mode

| Scenario | File Logging | Console Output |
|----------|-------------|----------------|
| Always | Yes | Yes (JSON-RPC only) |

MCP Server uses **dual output**:
- Logs go to `~/.intent-engine/logs/mcp-server.log`
- JSON-RPC communication goes to stdout (clean)

This ensures AI tools can read JSON-RPC responses without log noise.

### CLI Mode

| Scenario | File Logging | Console Output |
|----------|-------------|----------------|
| Default | No | Yes |
| Verbose | No | Yes (with timestamps) |

---

## Cleanup and Maintenance

### Automatic Cleanup

Dashboard automatically cleans up old log files on startup:
- Default retention: **7 days**
- Only rotated files (`.log.YYYY-MM-DD`) are removed
- Current log file (`.log`) is never removed

**Configure retention**:
```bash
export IE_LOG_RETENTION_DAYS=14
ie dashboard start
```

### Manual Cleanup

```bash
# Check log directory size
du -sh ~/.intent-engine/logs

# List old log files
find ~/.intent-engine/logs -name "*.log.*" -mtime +7

# Remove old logs
find ~/.intent-engine/logs -name "*.log.*" -mtime +7 -delete
```

---

## Troubleshooting

### No Logs Appear

**Problem**: `ie logs` shows "No log entries found"

**Solutions**:
1. Check if log directory exists:
   ```bash
   ls -la ~/.intent-engine/logs/
   ```

2. Check if Dashboard/MCP Server was run:
   ```bash
   ie dashboard start
   ie logs --mode dashboard
   ```

3. Increase time range:
   ```bash
   ie logs --since 7d
   ```

### Log Parsing Errors

**Problem**: Some log lines not showing in `ie logs`

**Cause**: Non-standard log format or corrupted lines

**Solution**: Check raw log file:
```bash
cat ~/.intent-engine/logs/dashboard.log
```

### Permission Errors

**Problem**: Cannot write to log directory

**Solution**: Check directory permissions:
```bash
ls -ld ~/.intent-engine/logs
chmod 755 ~/.intent-engine/logs
```

---

## Advanced Topics

### Custom Log Formats

For programmatic analysis, use JSON export:
```bash
ie logs --export json | jq '.[] | select(.level == "ERROR")'
```

### Integration with External Tools

**Send logs to syslog**:
```bash
ie logs --mode dashboard --since 1h | logger -t intent-engine
```

**Monitor for errors**:
```bash
# Alert on errors (cron job)
ie logs --level error --since 1h | mail -s "Intent-Engine Errors" admin@example.com
```

**Prometheus metrics** (future enhancement):
```bash
# Count errors by level
ie logs --since 1h --export json | jq 'group_by(.level) | map({level: .[0].level, count: length})'
```

---

## FAQ

### Q: Where are logs stored?

**A**: `~/.intent-engine/logs/` directory. Each mode has its own log file.

### Q: How do I enable debug logging?

**A**: Set `RUST_LOG=debug` environment variable before running the command.

### Q: Do logs consume unlimited disk space?

**A**: No. Daily rotation creates new files, and old files are automatically cleaned up (7-day retention by default). You can also use `logrotate` for more control.

### Q: Can I disable file logging?

**A**: Yes, but not recommended. You can delete the `~/.intent-engine/logs/` directory, but Dashboard daemon mode requires file logging to work.

### Q: How do I view logs from multiple modes at once?

**A**: Omit the `--mode` flag: `ie logs --since 1h`

### Q: Can I query logs by timestamp range?

**A**: Yes, use `--since` and `--until`:
```bash
ie logs --since 24h --until "2025-11-23T12:00:00Z"
```

---

## See Also

- [Deployment Guide](deployment/README.md) - Production deployment recommendations
- [Troubleshooting Guide](troubleshooting.md) - Common issues and solutions
- [logrotate Configuration](deployment/logrotate.conf) - Example logrotate setup

---

**Last Updated**: 2025-11-23
**Version**: 0.6.6
