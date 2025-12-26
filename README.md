# Intent-Engine

**[中文](Readme.zh.md) | English**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)

---

> **AI forgets. You shouldn't have to remind it.**

## The Problem

Every new session with an AI assistant:

```
Day 1: "Let's build authentication"
       AI works brilliantly, makes smart decisions...
       [session ends]

Day 2: "Continue authentication"
       AI: "What authentication? I have no memory of this."
```

AI has powerful reasoning. It just can't remember.

## The Solution

```bash
cargo install intent-engine
```

Now your AI remembers everything — across days, weeks, months.

```
Day 1: "Let's build authentication"
       AI creates task, works, records decisions → saved to disk

Day 2: "Continue authentication"
       AI reads memory → "Resuming... we chose JWT with HS256,
       finished token generation, next: OAuth integration"
```

**No cloud. No config. Just persistent memory.**

---

## How It Works

Intent-Engine gives AI a simple protocol:

```bash
ie status              # What am I working on?
ie plan                # Create or update tasks (JSON stdin)
ie log decision "..."  # Record why I made this choice
ie search "auth"       # Find relevant history
```

When AI starts a session, it runs `ie status`. Everything comes back:
- Current task and its context
- All ancestor tasks (the bigger picture)
- Decision history (the "why" behind every choice)

**One command. Full context restoration.**

---

## Integration

### Claude Code (One-Click)

```bash
/plugin marketplace add wayfind/intent-engine
/plugin install intent-engine@intent-engine
```

That's it. The plugin includes:
- **Hook**: Auto-install ie, init project, run `ie status` at session start
- **Skill**: Guide Claude to use `ie plan` instead of TodoWrite

### Manual Install

If you prefer manual setup:

```bash
# 1. Install binary
cargo install intent-engine
# or: brew install wayfind/tap/intent-engine
# or: npm install -g intent-engine

# 2. Add system prompt
claude --append-system-prompt "Use ie plan instead of TodoWrite. Commands: ie status, echo '{...}'|ie plan, ie log, ie search"
```

### Other AI Assistants

Any AI with CLI access can use `ie` commands directly.

---

## The Deeper Idea

Most tools track **what happened** (commits, logs, events).

Intent-Engine tracks **what you intended** and **why**.

```
Git:           "Changed auth.rs line 42"
Intent-Engine: "Chose JWT over sessions for stateless API scalability"
```

Code changes. Intent persists.

---

## Core Features

- **Hierarchical tasks** — break big goals into smaller ones
- **Decision history** — every "why" recorded with context
- **Cross-session memory** — pick up where you left off
- **Local storage** — everything in `~/.intent-engine/`, no cloud
- **Dashboard UI** — visualize progress at `localhost:11391`

---

## Quick Reference

```bash
# Install
cargo install intent-engine
# or: brew install wayfind/tap/intent-engine
# or: npm install -g intent-engine

# Core commands
ie status                    # Current context
ie search "todo doing"       # Find unfinished work
echo '{"tasks":[...]}' | ie plan   # Create/update tasks
ie log decision "chose X"    # Record decision
ie dashboard open            # Visual UI
```

---

## Documentation

- [Quick Start](QUICKSTART.en.md) — 5 minutes to get going
- [CLAUDE.md](CLAUDE.md) — For AI assistants
- [Command Reference](docs/en/guide/command-reference-full.md) — All commands

---

## License

MIT OR Apache-2.0, at your option.

---

**Give your AI the memory it deserves.**

```bash
cargo install intent-engine
```
