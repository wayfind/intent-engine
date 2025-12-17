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
- ⚡ **Declarative Task Planning** (v0.6): Batch create/update task structures with idempotent `plan` interface

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

### 3. Declarative Task Planning (v0.6)

Create complex task structures in one go using the `plan` interface:

```bash
# Create task structure from JSON
cat > project.json <<'JSON'
{
  "tasks": [
    {
      "name": "Implement user authentication",
      "spec": "JWT + OAuth2 support",
      "priority": "high",
      "children": [
        {
          "name": "JWT Implementation",
          "spec": "HS256 algorithm, 7-day validity"
        },
        {
          "name": "OAuth2 Integration",
          "spec": "Google and GitHub providers",
          "depends_on": ["JWT Implementation"]
        }
      ]
    }
  ]
}
JSON

# Execute the plan (creates all tasks + dependencies)
ie plan < project.json

# Start working on a specific task
ie task start 2  # JWT Implementation task
```

**Key Benefits:**
- ✅ Batch create entire task trees
- ✅ Idempotent: run multiple times → same result
- ✅ Name-based dependencies (no manual ID management)
- ✅ Automatic cycle detection

> 📖 **Full Plan Interface Guide**: See [docs/PLAN_INTERFACE_GUIDE.md](docs/PLAN_INTERFACE_GUIDE.md)

> 💡 **More Detailed Tutorial**: See [QUICKSTART.md](QUICKSTART.en.md)

---

## 🤖 Claude Code Integration: Zero-Configuration AI Collaboration

**New in v0.10.0**: Intent-Engine now integrates seamlessly with Claude Code through an embedded system prompt - **no configuration required**.

### Why This Approach?

**Previous MCP Approach (v0.9.x)** vs **System Prompt (v0.10.0)**:

| Aspect | MCP Server (Old) | System Prompt (New) |
|--------|------------------|---------------------|
| **Setup Complexity** | Manual JSON configuration | Zero configuration |
| **Installation** | Multi-step process | Single binary install |
| **Maintenance** | Restart required for updates | Automatic |
| **Dashboard Integration** | Separate process | Auto-start daemon |
| **Real-Time Sync** | Database only | Database + HTTP notifications |
| **Learning Curve** | High (MCP concepts) | Low (standard CLI) |

### Quick Start

```bash
# 1. Install Intent-Engine
cargo install intent-engine

# 2. That's it! Claude Code automatically understands Intent-Engine
# No configuration files, no MCP setup, no restart required
```

### How It Works

Intent-Engine v0.10.0 uses a **345-line embedded system prompt** that teaches Claude Code:
- ✅ All CLI commands and their usage
- ✅ Focus-driven workflow patterns
- ✅ Hierarchical task decomposition
- ✅ Event tracking best practices
- ✅ Common mistakes and anti-patterns

**Automatic Features**:
- **Dashboard Auto-Start**: Dashboard starts automatically when you use any CLI command
- **Real-Time Sync**: CLI operations instantly update Dashboard UI via HTTP notifications
- **Cross-Platform**: Works on Linux, macOS, and Windows (including WSL)

### Usage Example

After installation, just use Intent-Engine naturally in conversations:

```
You: "Help me implement a user authentication system"

Claude: "I'll use Intent-Engine to track this work..."
        [Executes: ie add "Implement user authentication"]
        [Executes: ie start 1 --with-events]

        "I've created and started task #1. Let me break this down:

        Based on the requirements, I'll create subtasks for:
        1. JWT token generation and validation
        2. User password hashing
        3. Refresh token mechanism

        Starting with JWT implementation..."

        [Executes: ie add "JWT Implementation" --parent 1]
        [Executes: ie start 2]
        [Implements the feature]
        [Executes: ie log decision "Chose HS256 algorithm because..."]
        [Executes: ie done]
```

**Dashboard Feedback**: All these operations appear in real-time in the Dashboard UI (auto-started in background).

### Built-in Help System

Intent-Engine includes comprehensive embedded guides:

```bash
# AI integration patterns and best practices
ie guide ai

# Migration from TodoWriter to Intent-Engine
ie guide todowriter

# Core workflow patterns (focus-driven, hierarchical, etc.)
ie guide workflow

# Real-world usage examples
ie guide patterns
```

These guides are optimized for AI consumption and cover:
- ✅ All CLI commands with examples
- ✅ Common workflows and patterns
- ✅ Typical mistakes and how to avoid them
- ✅ Integration with other tools

### Supported AI Assistants

The system prompt approach works with:
- ✅ **Claude Code** (native support)
- ✅ **Claude Desktop** (native support)
- ✅ **Cursor** (via shell command execution)
- ✅ **Aider** (via shell command execution)
- ✅ **Generic LLM CLI tools** (any tool that can execute shell commands)

### Migrating from v0.9.x

If you're upgrading from v0.9.x with MCP configuration:

```bash
# 1. Upgrade the binary
cargo install --force intent-engine

# 2. Remove old MCP configuration
# Edit Claude's config file and delete the "intent-engine" MCP entry
# - Linux/macOS: ~/.claude.json
# - Windows: %APPDATA%\Claude\.claude.json

# 3. Restart Claude Code

# 4. Verify
ie guide ai
```

**Your existing database is fully compatible** - no migration needed.

> 📖 **Complete Migration Guide**: See [MIGRATION_v0.10.0.md](MIGRATION_v0.10.0.md)

### Technical Architecture

**Embedded System Prompt**:
- Compiled into the binary at build time
- 345 lines of condensed AI guidance
- Covers all commands, patterns, and anti-patterns
- Zero external dependencies

**Dashboard Auto-Start**:
- Cross-platform daemon mode (Unix fork, Windows detached process)
- PID file management with automatic stale cleanup
- Health check with 3-second timeout
- Graceful degradation if Dashboard fails

**Real-Time Sync**:
- Fire-and-forget HTTP notifications (500ms timeout)
- Non-blocking CLI operations
- Dual notification pattern (CLI → Dashboard HTTP, Dashboard → UI WebSocket)
- Prevents circular dependencies

### Advantages Over MCP

1. **Simpler Mental Model**: Just use CLI commands naturally
2. **No Configuration**: Works out of the box
3. **Better Error Messages**: Standard CLI error handling
4. **Faster Iteration**: No restart required for updates
5. **More Portable**: No external configuration files
6. **Real-Time Feedback**: Dashboard auto-starts and stays in sync

### Detailed Documentation

- 📘 [CLAUDE.md](CLAUDE.md) - Complete AI assistant integration guide
- 📖 [AGENT.md](AGENT.md) - Technical details and data models
- 📝 [MIGRATION_v0.10.0.md](MIGRATION_v0.10.0.md) - Migration from v0.9.x

---

## 🌐 Web Dashboard: Visual Task Management

**New in v0.5**: Intent-Engine now includes a built-in web dashboard for visual task management and monitoring.

### Key Features

- ✅ **Modern Web UI**: Beautiful interface powered by TailwindCSS and HTMX
- ✅ **Markdown Rendering**: Rich text display with code syntax highlighting
- ✅ **Real-Time Search**: Instant full-text search across tasks and events
- ✅ **Task Workflows**: Visual buttons for start, complete, switch, and spawn operations
- ✅ **Event Tracking**: Timeline view of decisions, blockers, milestones, and notes
- ✅ **Multi-Project Support**: Run dashboards for multiple projects simultaneously

### Quick Start

```bash
# Start dashboard (uses fixed port 11391)
cd /path/to/your/project
ie dashboard start

# Open in browser automatically
ie dashboard open

# Or manually access the URL shown in the output
# http://127.0.0.1:11391

# Check running dashboards
ie dashboard list

# Stop dashboard
ie dashboard stop
```

### Why Use the Dashboard?

**Perfect for:**
- 👀 **Visualizing Progress**: See task hierarchy and status at a glance
- 📊 **Browsing History**: Review event timelines with rich Markdown rendering
- 🎨 **Presenting to Teams**: Share project status via browser
- 🔍 **Exploring Tasks**: Search and filter large task sets interactively

**Integration:**
- All changes sync instantly with CLI and Dashboard (shares same database + HTTP notifications)
- RESTful API available for custom integrations

### Documentation

- 📖 [Dashboard User Guide](docs/dashboard-user-guide.md) - Complete user manual
- 🔧 [API Reference](docs/dashboard-api-reference.md) - REST API documentation
- 🏗️ [Architecture Spec](docs/web-dashboard-spec.md) - Technical design

---

## ✨ Core Features

### New in v0.4 (2025-11)
- **🔍 Unified Search Engine**: `search` provides full-text search across both tasks and events, retrieving complete context in a single query

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
- [**CLAUDE.md**](CLAUDE.md) - Complete Claude Code/Desktop integration guide
- [**AGENT.md**](AGENT.md) - Technical details and data models

### 📖 Deep Dive
- [**Command Reference**](docs/en/guide/command-reference.md) - Complete command reference
- [**Task Workflow Analysis**](docs/en/technical/task-workflow-analysis.md) - Token optimization strategy explained
- [**Performance Report**](docs/en/technical/performance.md) - Performance benchmarks
- [**Security Testing**](docs/en/technical/security.md) - Security test reports
- [**Migration Guide (v0.10.0)**](MIGRATION_v0.10.0.md) - Upgrade from v0.9.x (MCP) to v0.10.0 (System Prompt)

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

**Test Statistics**: 500+ tests all passing ✅
- Unit tests, integration tests, CLI tests
- Dashboard integration tests
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
