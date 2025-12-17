# Migration Guide: v0.9.x → v0.10.0

## Overview

Intent-Engine v0.10.0 represents a major architectural shift from MCP-based to system prompt-based AI integration. This guide will help you migrate smoothly.

---

## 🎯 What Changed?

### Removed: MCP Server

**Before (v0.9.x)**:
```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "ie",
      "args": ["mcp-server"],
      "env": {...}
    }
  }
}
```

**After (v0.10.0)**:
- ❌ MCP server removed entirely
- ✅ System prompt integration (automatic)
- ✅ Dashboard auto-start (seamless)

### Why This Change?

**Problems with MCP Approach**:
1. **Complex Setup**: Users had to manually configure `mcp-server.json` files
2. **Multiple Binaries**: Separate CLI and MCP server processes
3. **Sync Issues**: CLI and MCP operations could drift out of sync
4. **Cognitive Load**: Users needed to understand JSON-RPC, process management, etc.

**Benefits of System Prompt Approach**:
1. **Zero Configuration**: Works out of the box with Claude Code
2. **Single Binary**: One `ie` binary for everything
3. **Automatic Dashboard**: Dashboard starts automatically when needed
4. **Real-Time Sync**: CLI → Dashboard notifications are instant
5. **Simpler Mental Model**: Just use CLI commands naturally

---

## 📋 Migration Checklist

### Step 1: Update Intent-Engine

```bash
# Using Homebrew (macOS/Linux)
brew upgrade intent-engine

# Or build from source
git pull origin main
cargo install --path .
```

### Step 2: Remove MCP Configuration

**Claude Code**:
```bash
# Remove MCP server configuration
# Edit: ~/.config/Claude/claude_desktop_config.json (Linux)
#       ~/Library/Application Support/Claude/claude_desktop_config.json (macOS)
#       %APPDATA%\Claude\claude_desktop_config.json (Windows)

# Delete the "intent-engine" entry from "mcpServers"
```

**Claude Desktop**:
- Same process as Claude Code - remove MCP configuration

### Step 3: Verify Setup

```bash
# Check version
ie --version
# Should show: intent-engine 0.10.0 (or higher)

# Test basic functionality
ie init
ie add "Test task"
ie start 1
ie done

# Verify Dashboard auto-start
ie ls
# Dashboard should start automatically in background

# Check Dashboard status
ie dashboard status
```

### Step 4: Use Enhanced Help System

```bash
# New guide command replaces need for external docs
ie guide ai            # AI integration patterns
ie guide todowriter    # TodoWriter migration guide
ie guide workflow      # Core workflows
ie guide patterns      # Usage examples
```

---

## 🔄 Feature Comparison

| Feature | v0.9.x (MCP) | v0.10.0 (System Prompt) |
|---------|--------------|-------------------------|
| **Setup Complexity** | High (manual config) | None (automatic) |
| **AI Integration** | MCP tools | System prompt + CLI |
| **Dashboard** | Manual start | Auto-start |
| **CLI → Dashboard Sync** | Database only | Database + HTTP notifications |
| **Learning Curve** | Steep (MCP concepts) | Gentle (standard CLI) |
| **Binary Count** | 1 (dual-mode) | 1 (unified) |
| **Configuration Files** | `mcp-server.json` required | None required |

---

## 🛠️ New Features in v0.10.0

### 1. Dashboard Auto-Start

```bash
# Dashboard starts automatically when you use any command
ie add "New task"
# ✓ Dashboard started in background (PID: 12345)

# Check status
ie dashboard status
# Dashboard running: http://127.0.0.1:11391 (PID: 12345)
```

**Cross-Platform Support**:
- **Unix/Linux/macOS**: Fork-based daemon with SIGTERM handling
- **Windows**: Detached process with CREATE_NO_WINDOW flag
- **PID Management**: Automatic stale PID cleanup, health checks

### 2. Real-Time CLI → Dashboard Sync

```bash
# CLI operations instantly update Dashboard UI
ie add "Task A"          # Dashboard shows new task immediately
ie start 1               # Dashboard highlights focused task
ie log decision "..."    # Dashboard shows new event in real-time
```

**Technical Details**:
- Fire-and-forget HTTP notifications (500ms timeout)
- Non-blocking CLI operations
- WebSocket broadcast to all connected clients

### 3. Enhanced Guide System

```bash
# Get context-rich guidance for AI assistants
ie guide ai         # 345-line AI integration guide
ie guide todowriter # TodoWriter → Intent-Engine migration
ie guide workflow   # 6 core workflow patterns
ie guide patterns   # 8 practical usage examples
```

**Design Philosophy**:
- Optimized for AI consumption (no token waste)
- Covers common mistakes and anti-patterns
- Includes code examples and decision trees

---

## 🤖 AI Assistant Integration

### For Claude Code Users

**Before (v0.9.x)**:
1. Install MCP server
2. Configure `claude_desktop_config.json`
3. Restart Claude Code
4. Use MCP tools directly

**After (v0.10.0)**:
1. Install Intent-Engine
2. Start using it naturally in conversations
3. Claude Code reads system prompt automatically
4. Dashboard provides real-time UI feedback

**Example Session**:
```
User: "Help me implement authentication"

Claude: "I'll use Intent-Engine to track this work..."
        [Uses: ie add "Implement authentication"]
        [Uses: ie start 1]
        [Claude implements the feature]
        [Uses: ie log decision "Chose JWT because..."]
        [Uses: ie done]

Dashboard: [Shows all changes in real-time]
```

### For Other AI Assistants

The system prompt approach works with any AI assistant that can:
1. Execute shell commands
2. Read command output
3. Understand markdown/text guides

**Supported Platforms**:
- ✅ Claude Code (native)
- ✅ Claude Desktop (native)
- ✅ Cursor (via shell commands)
- ✅ Aider (via shell commands)
- ✅ Generic LLM CLI tools

---

## 🔧 Troubleshooting

### Issue: "Dashboard not starting automatically"

**Symptoms**:
```bash
ie add "Test"
# No dashboard auto-start message
```

**Solution**:
```bash
# Check if Dashboard is already running
ie dashboard status

# If not running, check logs
ie logs --mode dashboard --level debug

# Manually start Dashboard
ie dashboard start --daemon

# Verify health
curl http://127.0.0.1:11391/api/health
```

### Issue: "CLI changes not reflected in Dashboard"

**Symptoms**:
- CLI commands succeed
- Dashboard UI doesn't update

**Solution**:
```bash
# Check Dashboard status
ie dashboard status

# Check logs for notification errors
ie logs --mode dashboard --level warn --since 1h

# Verify notification endpoint
curl -X POST http://127.0.0.1:11391/api/internal/cli-notify \
  -H "Content-Type: application/json" \
  -d '{"type":"task_changed","task_id":1,"operation":"test"}'
```

### Issue: "Old MCP configuration conflicts"

**Symptoms**:
- Claude Code shows "MCP server connection error"
- Intent-Engine works in CLI but not in Claude

**Solution**:
```bash
# 1. Remove MCP configuration completely
# Edit Claude's config file and delete "intent-engine" entry

# 2. Restart Claude Code

# 3. Verify clean state
ps aux | grep "ie mcp-server"  # Should show nothing

# 4. Test with fresh session
ie init --force
ie add "Test"
```

---

## 📚 Updated Documentation Structure

**Removed**:
- `docs/*/integration/mcp-server.md` (MCP setup guide)
- `docs/*/technical/mcp-tools-sync.md` (MCP maintenance guide)
- `mcp-server.json` (MCP schema file)

**Added**:
- `MIGRATION_v0.10.0.md` (this file)
- `ie guide ai` (embedded guide)
- `ie guide todowriter` (embedded guide)
- `ie guide workflow` (embedded guide)
- `ie guide patterns` (embedded guide)

**Updated**:
- `README.md` - Removed MCP section, updated integration guide
- `CLAUDE.md` - Updated to reflect system prompt approach
- `AGENT.md` - Removed MCP interface documentation

---

## 🎓 Learning Resources

### Quick Start

```bash
# 1. Initialize project
cd ~/my-project
ie init

# 2. Create first task
ie add "Implement feature X"

# 3. Start working
ie start 1

# 4. Track decision
ie log decision "Chose approach A because..."

# 5. Complete
ie done

# 6. View Dashboard
ie dashboard open
```

### Recommended Reading Order

1. **Quick Start**: `ie guide ai` - Core concepts and commands
2. **Workflows**: `ie guide workflow` - Focus-driven patterns
3. **Examples**: `ie guide patterns` - Real-world scenarios
4. **Migration**: `ie guide todowriter` - If coming from TodoWriter

### Advanced Topics

```bash
# Hierarchical task breakdown
ie add "Feature X" --priority high
ie start 1
ie add "Subtask A" --parent 1
ie start 2
ie done
ie next  # Recommends next subtask

# Dependencies
ie add "Build API"
ie add "Build Frontend"
ie task depends-on 2 1  # Frontend depends on API

# Event filtering
ie event list --type decision --since 7d
ie search "authentication AND JWT"
```

---

## 🚀 What's Next?

### Planned Features (v0.11.0+)

- **Git Integration**: Auto-commit on `ie done`
- **Plugin System**: Custom commands and hooks
- **Multi-Project Dashboard**: View all projects in one UI
- **Export/Import**: Share task trees with team
- **AI Templates**: Pre-configured workflows for common tasks

### Community

- **GitHub**: https://github.com/your-org/intent-engine
- **Issues**: Report bugs or request features
- **Discussions**: Share workflows and tips

---

## 📝 Changelog Summary

### Added
- ✅ Dashboard auto-start with cross-platform daemon mode
- ✅ CLI → Dashboard real-time HTTP notifications
- ✅ `ie guide` command with 4 comprehensive guides
- ✅ System prompt embedded in binary (345 lines)
- ✅ Fire-and-forget notification pattern

### Changed
- 🔄 AI integration: MCP → System Prompt
- 🔄 Architecture: Dual-mode binary → CLI-native
- 🔄 Configuration: Manual JSON → Zero-config

### Removed
- ❌ MCP server mode (`ie mcp-server`)
- ❌ MCP configuration files
- ❌ MCP-specific documentation
- ❌ JSON-RPC protocol handling

### Deprecated
- None (clean break from v0.9.x)

---

## ❓ FAQ

**Q: Do I need to reconfigure anything?**
A: No. Just upgrade the binary and remove old MCP config.

**Q: Will my existing database work?**
A: Yes. Database schema is unchanged.

**Q: Can I still use the Dashboard manually?**
A: Yes. Use `ie dashboard start` or let it auto-start.

**Q: What if I prefer the MCP approach?**
A: v0.9.x will remain available, but v0.10.0+ is recommended.

**Q: Does this work with Claude Desktop?**
A: Yes, through system prompt. Remove MCP config and use naturally.

**Q: How do I verify the migration succeeded?**
A: Run `ie guide ai` - if you see the guide, you're good!

---

**Migration Date**: 2025-12-16
**Version**: 0.10.0
**Migration Difficulty**: Low (mostly configuration removal)
**Estimated Time**: 5-10 minutes

*For technical details, see [RELEASE_NOTES_v0.10.0.md](RELEASE_NOTES_v0.10.0.md)*
