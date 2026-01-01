# Intent-Engine

**[中文](Readme.zh.md) | English**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)

> **Persistent memory for AI coding assistants.**

---

## The Problem

AI assistants lose context constantly:

| Scenario | What Happens |
|----------|--------------|
| Session ends | All context lost |
| Tool crashes | Progress vanishes |
| Computer restarts | Start from zero |
| After a week | "What was I working on?" |

You waste time re-explaining. AI wastes tokens re-understanding.

## The Solution

```bash
# Claude Code users: just run this
/plugin marketplace add wayfind/origin-task
/plugin install intent-engine@wayfind/origin-task
```

Now your AI remembers everything — across sessions, crashes, restarts, weeks.

```
Week 1, Monday:    "Build authentication system"
                   AI works, records decisions → saved locally

Week 2, Wednesday: "Continue auth"
                   AI reads memory → "Resuming #42: JWT auth.
                   Done: token generation, validation middleware.
                   Next: refresh token rotation.
                   Decision log: chose HS256 for single-service simplicity."
```

**One command restores full context. Every time.**

---

## Why Intent-Engine

### Context-Friendly

| Aspect | Intent-Engine | Typical Solutions |
|--------|---------------|-------------------|
| Context usage | ~200 tokens | Thousands of tokens |
| Integration | System prompt / Hook / Skill | Heavy MCP servers |
| Footprint | Single binary, no daemon | Background processes |

AI gets what it needs. Nothing more.

### High Performance

| Component | Technology | Capability |
|-----------|------------|------------|
| Core | Rust | Memory-safe, zero-cost abstractions |
| Storage | SQLite | Battle-tested, zero-config |
| Search | FTS5 | GB-scale text, millisecond response |
| Privacy | Local-only | Your data never leaves your machine |

### Smart Task Model

- **Hierarchical** — Break complex goals into subtasks
- **Parallel** — Work on multiple tasks concurrently
- **Traceable** — Every decision recorded with context
- **Recoverable** — Resume from any interruption point

---

## Quick Start

**Claude Code users:** Plugin handles everything (binary + integration).

```
/plugin marketplace add wayfind/origin-task
/plugin install intent-engine@wayfind/origin-task
```

**Other users:** Two steps.

```bash
# Step 1: Install binary
brew install wayfind/tap/intent-engine
# or: npm install -g @origintask/intent-engine
# or: cargo install intent-engine

# Step 2: Add to your AI's system prompt
# "Use ie for task memory. Run ie status at session start."
```

---

## How It Works

```bash
ie status              # Restore context: current task, ancestors, decisions
ie plan                # Create/update tasks (JSON via stdin)
ie log decision "..."  # Record why you made a choice
ie search "keyword"    # Full-text search across all history
```

Typical AI workflow:

```
Session Start → ie status → Full context restored
                            ↓
Working       → ie plan    → Tasks created/updated
              → ie log     → Decisions recorded
                            ↓
Session End   → Data persisted locally
                            ↓
Next Session  → ie status  → Continue exactly where you left off
```

---

## Installation Details

### Binary Installation

| Method | Command | Notes |
|--------|---------|-------|
| Homebrew | `brew install wayfind/tap/intent-engine` | macOS/Linux |
| npm | `npm install -g @origintask/intent-engine` | Cross-platform |
| Cargo | `cargo install intent-engine` | Requires Rust |
| Direct | `curl -fsSL .../ie-manager.sh \| bash -s install` | No dependencies |

### AI Tool Integration

**Claude Code (Plugin)**
```
/plugin marketplace add wayfind/origin-task
/plugin install intent-engine@wayfind/origin-task
```

**Claude Code (Manual)**

Add to `~/.claude/CLAUDE.md`:
```markdown
Use `ie` for task management. Run `ie status` at session start.
```

**Other AI Tools**

Add to system prompt:
```
Use ie for persistent task memory. Commands: ie status, ie plan, ie log, ie search
```

---

## Command Reference

```bash
ie status                         # Current context
ie search "todo doing"            # Find unfinished work
echo '{"tasks":[...]}' | ie plan  # Create/update tasks
ie log decision "chose X"         # Record decision
ie dashboard open                 # Visual UI at localhost:11391
```

---

## Documentation

- [Quick Start](docs/en/guide/quickstart.md)
- [CLAUDE.md](CLAUDE.md) — AI assistant guide
- [Command Reference](docs/en/guide/command-reference-full.md)

---

## License

MIT OR Apache-2.0

---

**Give your AI the memory it deserves.**
