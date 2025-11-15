# Intent-Engine

**[中文](Readme.zh.md) | English**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wayfind/intent-engine/branch/main/graph/badge.svg)](https://codecov.io/gh/wayfind/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![Documentation](https://docs.rs/intent-engine/badge.svg)](https://docs.rs/intent-engine)

**Intent-Engine is a minimalist, project-specific strategic intent tracking system designed for human-AI collaboration.**

It's not just another todo list—it's a bridge connecting human strategic thinking with AI execution capabilities, helping answer two critical questions: "Where are we going?" and "Why are we going there?"

---

## 🎯 What is it?

Intent-Engine is a CLI tool + database system for recording, tracking, and reviewing **strategic intents**. It provides a shared, traceable "intent layer" for human-AI collaboration.

**Core Features:**
- 📝 **Strategic Task Management**: Focus on What and Why, not just How
- 🧠 **AI's External Long-term Memory**: Persist decision history and context across sessions
- 🌳 **Hierarchical Problem Decomposition**: Support unlimited levels of parent-child task relationships
- 📊 **Structured Decision Tracking**: Every key decision is recorded as an event stream
- 🔄 **JSON-native Interface**: Perfect for AI tool integration

---

## 👥 Who is it for?

### Primary Users

1. **Human Developers**: Set strategic goals and record project intentions
2. **AI Agents**: Understand objectives, execute tasks, and document decision processes
3. **Human-AI Collaboration Teams**: Maintain context synchronization in long-term projects

### Use Cases

- ✅ Complex projects requiring AI to work continuously across multiple sessions
- ✅ Technical projects needing to trace "why this decision was made"
- ✅ System engineering requiring decomposition of large tasks into subtask trees
- ✅ Automated processes where AI autonomously manages work priorities

---

## 💡 What problems does it solve?

### Value for Humans

**Problems with Traditional Task Management Tools (Jira/Linear):**
- ❌ Focus on tactical execution (Sprints, Story Points), lacking strategic layer
- ❌ Require extensive manual maintenance (status updates, comments)
- ❌ Not suitable for AI integration (primarily Web UI)

**Intent-Engine's Solution:**
- ✅ Strategic intent layer: Each task includes complete **specifications (spec)** and **decision history (events)**
- ✅ Automation-friendly: AI can autonomously create, update, and switch tasks
- ✅ CLI + JSON: Perfect AI toolchain integration

### Value for AI

**Limitations of Claude Code TodoWrite:**
- ❌ **Session-level**: Only exists in current conversation, disappears when session ends
- ❌ **No History**: Cannot trace previous decisions and thought processes
- ❌ **Flat Structure**: Lacks hierarchical relationships, difficult to manage complex tasks

**Intent-Engine's Advantages:**
- ✅ **Project-level**: Persisted to SQLite database, permanently saved across sessions
- ✅ **Traceable**: Complete event stream records context of every decision
- ✅ **Hierarchical**: Task tree structure, enforces completing all subtasks before parent task
- ✅ **Atomic Operations**: Commands like `start`, `pick-next`, `spawn-subtask`, `switch` save 50-70% tokens

---

## 📊 Essential Differences from Other Tools

| Dimension | Intent-Engine | Claude Code TodoWrite | Jira/Linear |
|-----------|---------------|----------------------|-------------|
| **Core Philosophy** | Strategic intent layer: What + Why | Tactical execution layer: What (temporary) | Task tracking layer: What + When |
| **Primary Users** | Humans ↔ AI (bidirectional) | AI internal use (unidirectional) | Human teams (collaborative) |
| **Lifecycle** | Project-level (cross-session, persistent) | Session-level (temporary, volatile) | Project-level (persistent) |
| **Data Structure** | Task tree + Event stream + Specifications | Flat list (no hierarchy) | Workflows + Fields + Comments |
| **History Tracing** | ✅ Complete decision history (events) | ❌ No history | ⚠️ Has comments but no structured decisions |
| **Interaction Mode** | CLI + JSON (AI-friendly) | Tool Call (AI-specific) | Web UI (human-friendly) |
| **Granularity** | Coarse-grained (strategic milestones) | Fine-grained (current steps) | Medium-grained (Sprint tasks) |
| **Core Value** | AI's external long-term memory | AI's working memory (short-term) | Team work coordination |

### Typical Use Case Comparison

**Intent-Engine:** "Implement user authentication system" (includes complete technical specs, decision history, subtask tree)
- Lifecycle: Days to weeks
- AI can resume context anytime via `task start --with-events` and continue working

**TodoWrite:** "Modify auth.rs file" (temporary step in current session)
- Lifecycle: Current session
- Disappears after session ends, cannot be recovered

**Jira:** "PROJ-123: Add OAuth2 support" (specific task assigned to team)
- Lifecycle: One Sprint (1-2 weeks)
- Requires manual status and progress updates

---

## 🚀 Quick Start

### 1. Installation

```bash
# Method 1: Cargo Install (Recommended)
cargo install intent-engine

# Method 2: Download Pre-compiled Binary
# Visit https://github.com/wayfind/intent-engine/releases

# Verify Installation
ie --version
```

> 📖 **Detailed Installation Guide**: See [INSTALLATION.md](docs/en/guide/installation.md) for all installation methods, troubleshooting, and integration options.

### 2. Experience Core Features in 5 Minutes

```bash
# 1. Add a strategic task
echo "Implement JWT authentication with token refresh, 7-day validity" | \
  ie task add --name "Implement user authentication" --spec-stdin

# 2. Start task and view context
ie task start 1 --with-events

# 3. Discover sub-problem during work? Create subtask and auto-switch
ie task spawn-subtask --name "Configure JWT secret key"

# 4. Record key decision (subtask is now current task)
echo "Chose HS256 algorithm, store secret in environment variables" | \
  ie event add --type decision --data-stdin

# 5. Complete subtask
ie task done

# 6. Switch back to parent task and complete
ie task switch 1
ie task done

# 7. Generate work report
ie report --since 1d --summary-only
```

> 💡 **More Detailed Tutorial**: See [QUICKSTART.md](QUICKSTART.en.md)

---

## 🔌 MCP Service: Deep Integration with Claude Code/Desktop

Intent-Engine provides a **Rust-native MCP (Model Context Protocol) server**, enabling Claude Code and Claude Desktop to directly use all Intent-Engine features without manually running commands.

### Why Use MCP Service?

**Traditional CLI Approach** vs **MCP Service**:

| Aspect | CLI Commands | MCP Service |
|--------|--------------|-------------|
| **Usage** | Humans manually execute commands | AI automatically invokes tools |
| **Integration Difficulty** | Need to copy-paste commands | Completely transparent, works out-of-box |
| **Context Awareness** | Need to manually pass task IDs | AI automatically manages current task |
| **Token Efficiency** | Need to output full commands | Atomic operations, save 50-70% |
| **User Experience** | Need to switch between terminal | Seamlessly complete within conversation |

### Quick Installation

**Method 1: Automatic (Recommended)**

```bash
# Install from cargo
cargo install intent-engine

# Auto-configure MCP server for Claude Code
ie setup-mcp

# Or for Claude Desktop
ie setup-mcp --target claude-desktop
```

**Method 2: From Source**

```bash
# Clone the project
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine

# Build and install (unified binary with CLI and MCP server)
cargo install --path .

# Auto-configure for Claude Code/Desktop
ie setup-mcp
# Or use the shell script:
# ./scripts/install/install-mcp-server.sh
```

> **Note**: The `setup-mcp` command automatically detects your OS and configures the correct file path. It targets Claude Code v2.0.37+ by default.

### Manual Configuration

Edit Claude's MCP configuration file:

**Claude Code** (v2.0.37+):
- Linux/macOS/WSL: `~/.claude.json`
- Windows: `%APPDATA%\Claude\.claude.json`

> **Note**: Earlier versions may use `~/.claude/mcp_servers.json` or `~/.config/claude-code/mcp_servers.json`

**Claude Desktop**:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

Add configuration:

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "/home/user/.cargo/bin/intent-engine",
      "args": ["mcp-server"],
      "env": {
        "INTENT_ENGINE_PROJECT_DIR": "/path/to/your/project"
      },
      "description": "Strategic intent and task workflow management"
    }
  }
}
```

Restart Claude Code/Desktop, and you'll see **17 Intent-Engine tools** available.

### MCP Tools List

After installation, Claude can automatically use the following tools:

**Task Management** (12 tools):
- `task_add` - Create strategic task
- `task_add_dependency` - Define task dependencies
- `task_start` - Start task (atomic: set doing + set as current)
- `task_pick_next` - Intelligently recommend next task
- `task_spawn_subtask` - Create subtask and switch (atomic)
- `task_switch` - Switch tasks (atomic: pause current + start new)
- `task_done` - Complete task (validates all subtasks done)
- `task_list` - Find tasks by status/parent (renamed from `task_find`)
- `task_get` - Get detailed task information
- `task_context` - Get task ancestry and subtask tree
- `task_update` - Update task properties
- `task_delete` - Delete a task

**Search & Discovery** (1 tool):
- `unified_search` - Unified full-text search across tasks and events

**Event Tracking** (2 tools):
- `event_add` - Record decisions/blockers/milestones (AI's external long-term memory)
- `event_list` - List event history with filtering

**Workflow** (2 tools):
- `current_task_get` - Get currently focused task
- `report_generate` - Generate work reports

### Usage Example

After installation, the experience in Claude Code:

```
You: "Help me implement a user authentication system"

Claude: I'll use Intent-Engine to track this work.
[Automatically calls task_add to create task #1]
[Automatically calls task_start to begin and get context]

"I've created and started task #1: Implement user authentication system.
Based on project analysis, I suggest breaking it down into these subtasks:

1. JWT Token generation and validation
2. User password hashing storage
3. Refresh Token mechanism

Let me create subtasks for each area..."
[Automatically calls task_spawn_subtask to create subtask #2]
[Begins implementing first subtask]
```

**Key Advantages**:
- ✅ **Zero Manual Operations**: AI automatically manages tasks, no need to copy-paste commands
- ✅ **Context Preservation**: Automatically resume task status and decision history across sessions
- ✅ **Transparent Tracking**: All decisions automatically recorded to event stream
- ✅ **Multi-project Isolation**: Different projects automatically use their own `.intent-engine` databases

### Technical Advantages

Intent-Engine's MCP server uses **Rust native implementation**, compared to traditional Python wrappers:

| Metric | Rust Native | Python Wrapper |
|--------|-------------|----------------|
| **Startup Time** | < 10ms | 300-500ms |
| **Memory Usage** | ~5MB | ~30-50MB |
| **Dependencies** | Zero | Requires Python 3.7+ |
| **Performance** | Native | IPC overhead |

### Verify Installation

```bash
# Manually test MCP server (from project directory)
cd /path/to/your/project
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
  ie mcp-server

# Or using environment variable
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
  INTENT_ENGINE_PROJECT_DIR=/path/to/your/project ie mcp-server

# Should return JSON response with 13 tools
```

### Detailed Documentation

- 📖 [Complete MCP Server Configuration Guide](docs/en/integration/mcp-server.md) - Installation, configuration, troubleshooting
- 🔧 [MCP Tools Sync System](docs/en/technical/mcp-tools-sync.md) - Maintainer's guide
- 📘 [CLAUDE.md](CLAUDE.md) - Complete AI assistant integration guide

---

## ✨ Core Features

### New in v0.4 (2025-11)
- **🔍 Unified Search Engine**: `unified_search` provides full-text search across both tasks and events, retrieving complete context in a single query

### New in v0.2 (2025-11)
- **🔗 Task Dependency System**: Define task dependencies, automatically prevent blocked tasks from starting
- **📊 Smart Event Querying**: Filter events by type and time range, dramatically reduce token usage
- **🎯 Priority Enum**: Human-friendly priority interface (`critical`/`high`/`medium`/`low`)
- **📝 Command Rename**: `task find` → `task list` for better clarity

### Core Capabilities
- **Project Awareness**: Automatically searches upward for `.intent-engine` directory, senses project root
- **Lazy Initialization**: Write commands auto-initialize project, no manual init needed
- **Task Tree Management**: Support unlimited levels of parent-child task relationships
- **Decision History**: Complete event stream recording (decision, blocker, milestone, etc.)
- **Smart Recommendation**: `pick-next` recommends next task based on context
- **Atomic Operations**: Commands like `start`, `switch`, `spawn-subtask` save 50-70% tokens
- **🔍 FTS5 Search Engine**: Millisecond response under GB-scale tasks, unique snippet function highlights matches with `**`, extremely Agent-context-friendly
- **JSON Output**: All commands output structured JSON, perfect for AI tool integration

---

## 📚 Documentation Navigation

### 🎯 Core Documents
- [**INTERFACE_SPEC.md**](docs/INTERFACE_SPEC.md) - **Interface Specification** (Authoritative)
- [**QUICKSTART.md**](QUICKSTART.en.md) - 5-minute quick start

### 🚀 Getting Started
- [**The Intent-Engine Way**](docs/en/guide/the-intent-engine-way.md) - Design philosophy and collaboration patterns (highly recommended)
- [**Installation Guide**](docs/en/guide/installation.md) - Detailed installation guide and troubleshooting

### 🤖 AI Integration
- [**AI Quick Guide**](docs/en/guide/ai-quick-guide.md) - AI client quick reference
- [**MCP Server**](docs/en/integration/mcp-server.md) - Integrate with Claude Code/Desktop
- [**Claude Skill**](.claude-code/intent-engine.skill.md) - Lightweight Claude Code integration

### 📖 Deep Dive
- [**Command Reference**](docs/en/guide/command-reference.md) - Complete command reference
- [**Task Workflow Analysis**](docs/en/technical/task-workflow-analysis.md) - Token optimization strategy explained
- [**Performance Report**](docs/en/technical/performance.md) - Performance benchmarks
- [**Security Testing**](docs/en/technical/security.md) - Security test reports
- [**MCP Tools Sync**](docs/en/technical/mcp-tools-sync.md) - MCP tools synchronization system

### 👥 Contributors
- [**Contributing Guide**](docs/en/contributing/contributing.md) - How to contribute code
- [**Release Process**](docs/en/contributing/publish-to-crates-io.md) - Release workflow

---

## 🌟 Unique Value of Intent-Engine

### 1. Memory Sharing Layer for Human-AI Collaboration
- Humans set strategic intents (What + Why)
- AI executes tactical tasks (How)
- Intent-Engine records the entire process

### 2. Cross-session Context Recovery
- AI can resume complete context anytime via `task start --with-events`
- No need for humans to repeatedly explain background

### 3. Decision Traceability
- Every key decision is recorded (`event add --type decision`)
- Future review of "why we chose solution A over solution B"

### 4. Hierarchical Problem Decomposition
- Support unlimited levels of parent-child tasks
- Enforces completing all subtasks before parent task completion

---

## 🛠️ Technology Stack

- **Language**: Rust 2021
- **CLI**: clap 4.5
- **Database**: SQLite with sqlx 0.7
- **Async Runtime**: tokio 1.35
- **Full-text Search**: SQLite FTS5

---

## 🔧 Development Setup

### First-time Setup for Contributors (Required)

To avoid CI formatting check failures, please run immediately after cloning:

```bash
./scripts/setup-git-hooks.sh
```

This installs git pre-commit hooks that automatically run `cargo fmt` before each commit, ensuring code formatting compliance.

### Development Tool Commands

The project provides a Makefile to simplify common operations:

```bash
make help          # Show all available commands
make fmt           # Format code
make check         # Run format, clippy and tests
make test          # Run all tests
make setup-hooks   # Install git hooks (same as above script)
```

> 📖 **Detailed Documentation**: See [scripts/README.md](scripts/README.md) for complete development workflow and automation tools.

---

## 🧪 Testing

Intent-Engine includes a complete testing suite:

```bash
# Run all tests
cargo test

# Run performance tests
cargo test --test performance_tests -- --ignored

# View test coverage
cargo tarpaulin
```

**Test Statistics**: 505+ tests all passing ✅
- Unit tests, integration tests, CLI tests
- MCP integration tests
- Special character security tests
- Performance and benchmarking tests
- Windows encoding compatibility tests

---

## 📄 License

This project is dual-licensed under MIT or Apache-2.0.

- MIT License - See [LICENSE-MIT](LICENSE-MIT)
- Apache License 2.0 - See [LICENSE-APACHE](LICENSE-APACHE)

---

## 🤝 Contributing

Issues and Pull Requests are welcome!

- [Contributing Guide](docs/en/contributing/contributing.md)
- [Code of Conduct](CODE_OF_CONDUCT.md) (coming soon)

---

## 🔗 Related Links

- [GitHub Repository](https://github.com/wayfind/intent-engine)
- [Crates.io](https://crates.io/crates/intent-engine)
- [Documentation](https://docs.rs/intent-engine)
- [Issue Tracker](https://github.com/wayfind/intent-engine/issues)

---

**Next Steps**: Read [The Intent-Engine Way](docs/en/guide/the-intent-engine-way.md) for deep understanding of design philosophy, or check out [QUICKSTART.md](QUICKSTART.en.md) to start using it right away.
