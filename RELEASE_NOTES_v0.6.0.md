# 🚀 Release v0.6.0 - Plan Interface and Dashboard Enhancements

**Release Date**: November 21, 2025
**Status**: Experimental (Pre-1.0)

---

## ⭐ Highlights

### 🎯 Plan Interface - Declarative Task Management

The biggest addition in v0.6.0 is the new **Plan Interface**, enabling declarative, batch-oriented task creation:

```bash
# Create entire task structures in one atomic operation
cat > project.json <<'JSON'
{
  "tasks": [
    {
      "name": "User Authentication",
      "spec": "Implement full auth system",
      "priority": "high",
      "children": [
        {"name": "JWT Implementation"},
        {"name": "OAuth2 Integration", "depends_on": ["JWT Implementation"]}
      ]
    }
  ]
}
JSON

ie plan < project.json
```

**Key Features:**
- ✅ **Batch Operations**: Create entire task trees in one call
- ✅ **Idempotent**: Run same plan multiple times → same result
- ✅ **Declarative**: Describe what you want, not how to do it
- ✅ **Smart Dependencies**: Automatic name-based resolution with cycle detection
- ✅ **Transaction-Based**: All-or-nothing atomicity
- ✅ **Well Documented**: Comprehensive guide in [docs/PLAN_INTERFACE_GUIDE.md](docs/PLAN_INTERFACE_GUIDE.md)

### 🔌 MCP WebSocket Integration

Real-time connection between Dashboard and MCP server:
- WebSocket-based communication for live updates
- Automatic reconnection with exponential backoff
- Enhanced session tracking and registry management

### 🎨 Dashboard UI Redesign

Complete visual overhaul with sci-fi themed interface:
- Modern, futuristic design aesthetic
- Improved user experience and visual hierarchy
- Enhanced task visualization
- Fixed port allocation (11391) for better reliability

---

## 📦 What's New

### Added

- **New CLI Command**: `ie plan` - Declarative task creation from JSON/stdin
- **New MCP Tool**: `plan` - Same functionality accessible via MCP
- **New Documentation**:
  - [docs/PLAN_INTERFACE_GUIDE.md](docs/PLAN_INTERFACE_GUIDE.md) - Complete plan interface guide
  - [docs/roadmap.md](docs/roadmap.md) - Project roadmap
  - [docs/ai_feedback/](docs/ai_feedback/) - AI tool integration paradigms
- **New Source Files**:
  - `src/plan.rs` (2028 lines) - Plan interface implementation
  - `src/mcp/ws_client.rs` - WebSocket client for MCP integration
  - `tests/cascade_tests.rs` - Cascade behavior tests
  - `tests/focus_switching_tests.rs` - Focus switching tests

### Changed

- **Dependencies**: Added WebSocket and async support
  - `axum` now includes `ws` feature
  - Added `tokio-tungstenite` 0.21
  - Added `futures-util` 0.3
  - Added `reqwest` with JSON support
- **Dashboard**: Fixed port to 11391 (previously dynamic allocation)
- **Interface Version**: Updated to 0.6 (from 0.5)
- **Documentation**: Updated AGENT.md, CLAUDE.md, and README.md with plan interface guidance

### Fixed

- **MCP → Dashboard WebSocket Connection**: Resolved cross-session connection failures
- **Project Boundary Logic**: Better support for non-project startup scenarios
- **Dashboard Daemon**: Improved process detachment using `setsid` on Unix
- **Process Management**: Enhanced PID file and registry synchronization

---

## 🔧 Technical Details

### Architecture

- **Plan Interface**: Uses Tarjan's SCC algorithm for dependency cycle detection
- **Transaction Model**: All plan operations are atomic (all-or-nothing)
- **Name-Based Identity**: Tasks identified by names, automatic ID resolution
- **Idempotency**: Same input produces same output (safe to retry)

### Testing

- ✅ All 280 tests passing
- ✅ New integration tests for focus switching
- ✅ Cascade behavior test coverage
- ✅ Plan interface comprehensive test suite

### Performance

- Fast dependency resolution with efficient graph algorithms
- Optimized WebSocket reconnection strategy
- No performance regression on existing features

---

## 📚 Migration Guide

### For Existing Users

**No Breaking Changes!** v0.6.0 is fully backward compatible:
- All existing CLI commands work as before
- All existing MCP tools unchanged (except new `plan` tool)
- Existing workflows continue to work

### Adopting Plan Interface

Consider migrating batch task creation to the new plan interface:

**Before (imperative):**
```bash
ie task add "Parent Task"
PARENT_ID=$(ie task list | grep "Parent" | awk '{print $1}')
ie task add "Child 1" --parent $PARENT_ID
ie task add "Child 2" --parent $PARENT_ID
ie task add-dependency --blocked "Child 2" --blocking "Child 1"
```

**After (declarative):**
```bash
ie plan <<'JSON'
{
  "tasks": [
    {
      "name": "Parent Task",
      "children": [
        {"name": "Child 1"},
        {"name": "Child 2", "depends_on": ["Child 1"]}
      ]
    }
  ]
}
JSON
```

See [PLAN_INTERFACE_GUIDE.md](docs/PLAN_INTERFACE_GUIDE.md) for complete migration examples.

---

## 🎯 What's Next

### v0.6.1 (Planned)
- Idempotent updates for plan interface
- Enhanced error messages
- Performance optimizations

### v0.7.0 (Future)
- Interface simplification based on feedback
- Additional plan interface capabilities
- Dashboard enhancements

See [docs/roadmap.md](docs/roadmap.md) for the full roadmap.

---

## 📖 Documentation

- **Quick Start**: [README.md](README.md)
- **AI Integration**: [CLAUDE.md](CLAUDE.md) and [AGENT.md](AGENT.md)
- **Plan Interface**: [docs/PLAN_INTERFACE_GUIDE.md](docs/PLAN_INTERFACE_GUIDE.md)
- **Full Changelog**: [CHANGELOG.md](CHANGELOG.md)
- **Interface Spec**: [docs/spec-03-interface-current.md](docs/spec-03-interface-current.md)

---

## 🙏 Acknowledgments

Thanks to all contributors and users who provided feedback to make this release possible!

Special thanks to Claude for assistance in development and documentation.

---

## 📥 Installation

### From Release Binaries (Recommended)
Download the appropriate binary for your platform from the [releases page](https://github.com/wayfind/intent-engine/releases/tag/v0.6.0).

### Via cargo-binstall
```bash
cargo binstall intent-engine@0.6.0
```

### From Source
```bash
cargo install --git https://github.com/wayfind/intent-engine --tag v0.6.0
```

---

**Full Changelog**: https://github.com/wayfind/intent-engine/compare/v0.4.0...v0.6.0
