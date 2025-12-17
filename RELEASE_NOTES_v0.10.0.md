# Release Notes: Intent-Engine v0.10.0

**Release Date**: 2025-12-16
**Codename**: "System Prompt"
**Type**: Major Release (Breaking Changes)

---

## 🎯 Overview

Intent-Engine v0.10.0 represents a fundamental architectural shift from MCP (Model Context Protocol) to an embedded system prompt approach. This release dramatically simplifies installation, eliminates configuration overhead, and provides a superior user experience through Dashboard auto-start and real-time synchronization.

**Key Highlights**:
- ✅ **Zero Configuration**: No setup required - works out of the box
- ✅ **Dashboard Auto-Start**: Automatically starts in background when needed
- ✅ **Real-Time Sync**: CLI operations instantly update Dashboard UI
- ✅ **Enhanced Help System**: Built-in AI guides accessible via `ie guide`
- ✅ **Simpler Architecture**: Single binary, no external dependencies

---

## 🔥 Breaking Changes

### MCP Server Removed

The MCP (Model Context Protocol) server has been completely removed in favor of an embedded system prompt approach.

**What This Means**:
- ❌ `ie mcp-server` command no longer exists
- ❌ `mcp-server.json` configuration file no longer used
- ❌ MCP setup/configuration no longer required

**Migration Path**:
1. Upgrade to v0.10.0: `cargo install --force intent-engine`
2. Remove MCP configuration from Claude Code/Desktop
3. That's it! System prompt integration is automatic

> 📖 **Detailed Migration Guide**: See [MIGRATION_v0.10.0.md](MIGRATION_v0.10.0.md)

### Configuration Changes

- **Removed**: All MCP-related configuration
- **Removed**: `mcp-server.json` schema file
- **Added**: Embedded 345-line system prompt (automatic)

---

## ✨ New Features

### 1. Dashboard Auto-Start (Phase 3)

Dashboard now automatically starts in daemon mode when you use any CLI command.

**Features**:
- Cross-platform daemon mode (Unix fork, Windows detached process)
- PID file management with automatic stale cleanup
- Health checks with 3-second timeout
- Graceful degradation if start fails

**Usage**:
```bash
# Dashboard starts automatically on first command
ie add "New task"
# ✓ Dashboard started in background (PID: 12345)

# Check status
ie dashboard status
# Dashboard running: http://127.0.0.1:11391 (PID: 12345)

# Manual control still available
ie dashboard start --daemon
ie dashboard stop
```

**Platform Support**:
- ✅ Linux (fork + SIGTERM)
- ✅ macOS (fork + SIGTERM)
- ✅ Windows (detached process + CREATE_NO_WINDOW)
- ✅ WSL (fork + SIGTERM)

### 2. Real-Time CLI → Dashboard Sync (Phase 4)

CLI operations now trigger instant Dashboard UI updates via HTTP notifications.

**How It Works**:
- CLI commands → HTTP POST to Dashboard (fire-and-forget, 500ms timeout)
- Dashboard → WebSocket broadcast to all connected clients
- Non-blocking design (CLI never waits for Dashboard)

**Supported Notifications**:
- `TaskChanged`: Task created/updated/deleted
- `EventAdded`: Event logged to task
- `WorkspaceChanged`: Focus changed (current_task_id)

**Example**:
```bash
# Terminal
ie add "Implement auth"    # Creates task
ie start 1                 # Sets focus
ie log decision "..."      # Adds event

# Dashboard UI updates in real-time (no refresh needed)
```

### 3. Enhanced Help System (`ie guide`)

Built-in AI guides replace external documentation for common workflows.

**Commands**:
```bash
ie guide ai          # AI integration patterns (345 lines)
ie guide todo-writer # TodoWriter → Intent-Engine migration
ie guide workflow    # Core workflow patterns (6 scenarios)
ie guide patterns    # Real-world usage examples (8 patterns)
```

**Content**:
- Optimized for AI consumption (no token waste)
- Covers common mistakes and anti-patterns
- Includes code examples and decision trees
- Always up-to-date (embedded in binary)

**Example**:
```bash
$ ie guide ai | head -20
# Intent-Engine: AI Quick Reference

Intent-Engine is your **external long-term memory** for strategic task management across sessions.

## Core Concepts

### 1. Focus-Driven Workflow
- **One task focused at a time** (current_task_id)
- Multiple tasks can be 'doing', but only ONE is "current" (focused)
- Tasks not current are effectively "paused"
...
```

### 4. Embedded System Prompt

345-line AI guide embedded directly in the binary.

**Content**:
- All CLI commands with usage examples
- Focus-driven workflow patterns
- Hierarchical task decomposition
- Event tracking best practices
- Common mistakes and solutions

**Advantages**:
- No external files or configuration
- Always consistent with binary version
- Zero maintenance overhead
- Works offline

---

## 🔧 Improvements

### Architecture

**Before (v0.9.x)**:
- MCP server via JSON-RPC
- Manual Dashboard start
- Database-only sync

**After (v0.10.0)**:
- System prompt embedded in binary
- Dashboard auto-start daemon
- Real-time HTTP notifications + WebSocket
- Zero configuration

### User Experience

| Aspect | v0.9.x | v0.10.0 | Improvement |
|--------|--------|---------|-------------|
| Setup Steps | 5+ steps | 1 step | 80% reduction |
| Configuration Files | 2 files | 0 files | 100% elimination |
| Learning Curve | High (MCP concepts) | Low (standard CLI) | Significantly easier |
| Dashboard Start | Manual | Automatic | Always available |
| UI Sync Latency | ~1-2s (polling) | < 100ms (HTTP push) | 10-20x faster |

### Performance

- **Startup Time**: Dashboard < 10ms (unchanged)
- **Memory Usage**: ~5MB CLI + ~15MB Dashboard (unchanged)
- **Sync Latency**: < 100ms (HTTP notification, previously 1-2s)
- **Binary Size**: +50KB (system prompt embedded)

---

## 🐛 Bug Fixes

### Test Suite

- **Fixed**: Removed obsolete `test_spec_lists_all_mcp_tools` test
- **Fixed**: Updated interface spec tests to reflect CLI-only approach

### Documentation

- **Fixed**: Removed all MCP references from README.md, CLAUDE.md, AGENT.md
- **Fixed**: Updated integration guides to system prompt approach

---

## 📝 Documentation

### New Documents

- `MIGRATION_v0.10.0.md` - Complete migration guide from v0.9.x
- Embedded guides accessible via `ie guide` command

### Updated Documents

- `README.md` - Replaced MCP section with Claude Code integration
- `CLAUDE.md` - Updated to v0.10 with system prompt approach
- `AGENT.md` - Removed MCP interface documentation

### Removed Documents

- MCP server setup guides (no longer needed)
- MCP tools sync documentation (obsolete)

---

## 🧪 Testing

### Test Coverage

- **Unit Tests**: 163+ tests ✅
- **Integration Tests**: 137 tests ✅
- **Functional Tests**: All scenarios passing ✅
- **Total**: 300+ tests, 100% pass rate

### Verified Platforms

- ✅ Linux (Ubuntu 22.04, Debian 12)
- ✅ macOS (Intel, Apple Silicon)
- ✅ Windows 10/11 (native + WSL2)

---

## 📦 Installation

### Upgrade from v0.9.x

```bash
# 1. Upgrade binary
cargo install --force intent-engine

# 2. Remove MCP configuration
# Edit Claude's config and delete "intent-engine" entry:
# - Linux/macOS: ~/.claude.json
# - Windows: %APPDATA%\Claude\.claude.json

# 3. Restart Claude Code/Desktop

# 4. Verify
ie guide ai
```

### Fresh Install

```bash
# Install from crates.io
cargo install intent-engine

# Verify
ie --version
# intent-engine 0.10.0

# Initialize project
cd ~/my-project
ie init

# Use naturally - Dashboard auto-starts
ie add "First task"
```

---

## 🔗 Resources

### Documentation

- [Migration Guide (v0.9.x → v0.10.0)](MIGRATION_v0.10.0.md)
- [CLAUDE.md](CLAUDE.md) - AI assistant integration guide
- [AGENT.md](AGENT.md) - Technical details and data models
- [README.md](README.md) - Project overview and quick start

### Built-in Guides

```bash
ie guide ai          # AI integration patterns
ie guide todo-writer # TodoWriter migration
ie guide workflow    # Core workflows
ie guide patterns    # Usage examples
```

### Links

- **Repository**: https://github.com/your-org/intent-engine
- **Crates.io**: https://crates.io/crates/intent-engine
- **Documentation**: https://docs.rs/intent-engine
- **Issues**: https://github.com/your-org/intent-engine/issues

---

## 💬 Feedback

We'd love to hear your thoughts on v0.10.0!

- **Report Issues**: GitHub Issues
- **Suggest Features**: GitHub Discussions
- **Share Workflows**: Show us how you use Intent-Engine

---

## 🙏 Acknowledgments

Special thanks to:
- Early adopters who provided feedback on MCP complexity
- Community members who tested v0.10.0 beta
- All contributors who helped shape this release

---

## 📅 What's Next?

### Planned for v0.11.0

- Git integration (auto-commit on `ie done`)
- Plugin system (custom commands and hooks)
- Multi-project Dashboard view
- Export/Import task trees
- AI workflow templates

### Roadmap

See [ROADMAP.md](docs/roadmap.md) for long-term plans.

---

**Upgrade today and experience the simplicity of Intent-Engine v0.10.0!**

```bash
cargo install --force intent-engine
```

*For questions or issues, please open a GitHub issue or discussion.*
