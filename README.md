# Intent-Engine

**[中文](Readme.zh.md) | English**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)

**Persistent memory for AI coding assistants.**

---

## AI Forgets. Every Time.

**Without Intent-Engine:**
```
Day 1: "Build authentication"
       AI works brilliantly...
       [session ends]

Day 2: "Continue auth"
       AI: "What authentication?"
```

**With Intent-Engine:**
```
Day 1: "Build authentication"
       AI works, saves progress...
       [session ends]

Day 2: "Continue auth"
       AI: "Resuming #42: JWT auth.
            Done: token generation.
            Next: refresh tokens."
```

**One command restores everything:** `ie status`

---

## Not Just Memory — Infrastructure

What actually happens when things go wrong:

- **Session ends** → ✓ Persisted
- **Tool crashes** → ✓ Recoverable
- **Week later** → ✓ Full history
- **Multiple agents** → ✓ Isolated
- **Complex project** → ✓ Focus-driven

---

## Why It Works

**Minimal Footprint** — ~200 tokens overhead, single binary, no daemons

**Battle-Tested Stack** — Rust + SQLite + FTS5, GB-scale in milliseconds, local-only

---

## The Bigger Picture

> **The unsolved problem in AI agents: tasks that span days or weeks.**

Intent-Engine provides the foundation:

```
Week-long refactoring:

├── Agent A (session: "api")    → focus: #12 REST endpoints
├── Agent B (session: "db")     → focus: #15 Schema migration
└── Agent C (session: "test")   → focus: #18 Integration tests
                                  depends_on: [#12, #15]
```

- **Interruptions** → Persistent memory
- **Multi-agent** → Session isolation
- **Scheduling** → Dependency graph (`depends_on`)
- **Context explosion** → Focus-driven retrieval

**Result:** Reliable multi-day, multi-agent workflows.

---

## Get Started

**Claude Code** — one command does everything:

```
/plugin marketplace add wayfind/origin-task
/plugin install intent-engine@wayfind/origin-task
```

**Manual setup:**

```bash
# Install (choose one)
curl -fsSL https://raw.githubusercontent.com/wayfind/intent-engine/main/scripts/install/ie-manager.sh | sh -s install
brew install wayfind/tap/intent-engine
npm install -g @origintask/intent-engine
cargo install intent-engine

# Core commands
ie status                         # Restore context
echo '{"tasks":[...]}' | ie plan  # Create/update tasks
ie log decision "chose X"         # Record decisions
ie search "keyword"               # Search history
```

---

## How It Works

```
Session Start  →  ie status  →  Full context restored
                                       ↓
Working        →  ie plan    →  Tasks tracked
               →  ie log     →  Decisions recorded
                                       ↓
Interruption   →  Auto-persisted
                                       ↓
Next Session   →  ie status  →  Continue where you left off
```

---

## Documentation

- **[Quick Start](docs/en/guide/quickstart.md)** — Get running in 5 minutes
- **[CLAUDE.md](CLAUDE.md)** — AI integration guide
- **[Commands](docs/en/guide/command-reference-full.md)** — Full reference

---

**MIT OR Apache-2.0** · [GitHub](https://github.com/wayfind/intent-engine)

*Give your AI the memory it deserves.*
