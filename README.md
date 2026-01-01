# Intent-Engine

**[中文](Readme.zh.md) | English**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)

**Persistent memory for AI coding assistants.**

<br>

## AI Forgets. Every Time.

<table>
<tr>
<td width="50%">

**Without Intent-Engine**

```
Day 1: "Build authentication"
       AI works brilliantly...
       [session ends]

Day 2: "Continue auth"
       AI: "What authentication?"
```

</td>
<td width="50%">

**With Intent-Engine**

```
Day 1: "Build authentication"
       AI works, saves progress...
       [session ends]

Day 2: "Continue auth"
       AI: "Resuming #42: JWT auth.
            Done: token generation.
            Next: refresh tokens."
```

</td>
</tr>
</table>

**One command restores everything:** `ie status`

<br>

## Not Just Memory — Infrastructure

Think about what actually happens:

| | Without | With |
|:--|:--|:--|
| Session ends | Lost | ✓ Persisted |
| Tool crashes | Gone | ✓ Recoverable |
| Week later | "What was I doing?" | ✓ Full history |
| Multiple agents | Chaos | ✓ Isolated |
| Complex project | Context explosion | ✓ Focus-driven |

<br>

## Why It Works

<table>
<tr>
<td width="50%">

### Minimal Footprint

- **~200 tokens** context overhead
- **System prompt / Hook** integration
- **Single binary**, no daemons

</td>
<td width="50%">

### Battle-Tested Stack

- **Rust** — memory-safe, fast
- **SQLite** — zero-config, reliable
- **FTS5** — GB-scale, milliseconds
- **Local-only** — your data stays yours

</td>
</tr>
</table>

<br>

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

| Challenge | Solution |
|:--|:--|
| Interruptions | Persistent memory |
| Multi-agent | Session isolation |
| Scheduling | Dependency graph (`depends_on`) |
| Context explosion | Focus-driven retrieval |

**Result:** Reliable multi-day, multi-agent workflows.

<br>

## Get Started

**Claude Code** — one command does everything:

```
/plugin marketplace add wayfind/origin-task
/plugin install intent-engine@wayfind/origin-task
```

**Manual setup:**

```bash
# Install (choose one)
brew install wayfind/tap/intent-engine
npm install -g @origintask/intent-engine
cargo install intent-engine

# Core commands
ie status                         # Restore context
echo '{"tasks":[...]}' | ie plan  # Create/update tasks
ie log decision "chose X"         # Record decisions
ie search "keyword"               # Search history
```

<br>

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

<br>

## Documentation

- **[Quick Start](docs/en/guide/quickstart.md)** — Get running in 5 minutes
- **[CLAUDE.md](CLAUDE.md)** — AI integration guide
- **[Commands](docs/en/guide/command-reference-full.md)** — Full reference

<br>

---

**MIT OR Apache-2.0** · [GitHub](https://github.com/wayfind/intent-engine)

*Give your AI the memory it deserves.*
