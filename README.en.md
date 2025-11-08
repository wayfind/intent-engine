# Intent-Engine

**[中文](README.md) | English**

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
- ✅ **Atomic Operations**: Compound commands like `start`, `pick-next`, `spawn-subtask`, `switch` save 60-70% tokens

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
intent-engine --version
```

> 📖 **Detailed Installation Guide**: See [INSTALLATION.md](docs/en/guide/installation.md) for all installation methods, troubleshooting, and integration options.

### 2. Experience Core Features in 5 Minutes

```bash
# 1. Add a strategic task
echo "Implement JWT authentication with token refresh, 7-day validity" | \
  intent-engine task add --name "Implement user authentication" --spec-stdin

# 2. Start task and view context
intent-engine task start 1 --with-events

# 3. Discover sub-problem during work? Create subtask and auto-switch
intent-engine task spawn-subtask --name "Configure JWT secret key"

# 4. Record key decisions
echo "Chose HS256 algorithm, store secret in environment variables" | \
  intent-engine event add --task-id 2 --type decision --data-stdin

# 5. Complete subtask
intent-engine task done

# 6. Switch back to parent task and complete
intent-engine task switch 1
intent-engine task done

# 7. Generate work report
intent-engine report --since 1d --summary-only
```

> 💡 **More Detailed Tutorial**: See [QUICKSTART.md](QUICKSTART.en.md)

---

## ✨ Core Features

- **Project Awareness**: Automatically searches upward for `.intent-engine` directory, senses project root
- **Lazy Initialization**: Write commands auto-initialize project, no manual init needed
- **Task Tree Management**: Support unlimited levels of parent-child task relationships
- **Decision History**: Complete event stream recording (decision, blocker, milestone, etc.)
- **Smart Selection**: `pick-next` automatically selects tasks based on priority and complexity
- **Atomic Operations**: Compound commands like `start`, `switch`, `spawn-subtask` save 60-70% tokens
- **Full-text Search**: Efficient search based on SQLite FTS5
- **JSON Output**: All commands output structured JSON, perfect for AI tool integration

---

## 📚 Documentation Navigation

### 🚀 Getting Started
- [**QUICKSTART.md**](QUICKSTART.en.md) - 5-minute quick start
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

**Test Statistics**: 116 tests all passing ✅
- 47 unit tests
- 22 CLI integration tests
- 10 special character security tests
- 37 performance tests

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
