# Intent-Engine

**[中文](Readme.zh.md) | English**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)

> **Persistent memory for AI coding assistants.**

---

## AI Forgets. Every Time.

```
Day 1: "Let's build authentication"
       AI works brilliantly, makes smart decisions...
       [session ends]

Day 2: "Continue authentication"
       AI: "What authentication?"
```

You've been there. We all have.

---

## One Command Changes Everything

```bash
ie status
```

Now your AI remembers:

```
Day 2: "Continue authentication"
       AI: "Resuming task #42: JWT auth.
            Done: token generation, validation.
            Next: refresh token rotation.
            Decision: chose HS256 for single-service simplicity."
```

**Full context. Instantly restored.**

---

## But It's Not Just About "Remembering"

Think about what actually happens during development:

| Scenario | Without Intent-Engine | With Intent-Engine |
|----------|----------------------|-------------------|
| Session ends | Context lost | ✓ Persisted |
| Tool crashes | Progress gone | ✓ Recoverable |
| Computer restarts | Start over | ✓ Resume instantly |
| After a week | "What was I doing?" | ✓ Full history |
| Multiple agents | Chaos | ✓ Isolated sessions |
| Complex project | Context explosion | ✓ Focus-driven |

**It's not memory. It's infrastructure for reliable AI workflows.**

---

## Why Intent-Engine Works

### Minimal Footprint

| Aspect | Intent-Engine | Typical Solutions |
|--------|---------------|-------------------|
| Context overhead | ~200 tokens | Thousands |
| Integration | System prompt / Hook | Heavy MCP servers |
| Runtime | Single binary | Background daemons |

AI gets exactly what it needs. Nothing more.

### Battle-Tested Stack

| Component | Choice | Why |
|-----------|--------|-----|
| Language | Rust | Memory-safe, fast |
| Storage | SQLite | Zero-config, reliable |
| Search | FTS5 | GB-scale, milliseconds |
| Location | Local-only | Your data stays yours |

---

## The Bigger Picture: Long-Running Tasks

Here's the unsolved problem in AI agents: **tasks that span days or weeks**.

Single-session AI can't handle this. Intent-Engine can.

```
Week-long refactoring project:

├── Agent A (session: "api")    → focus: #12 REST endpoints
├── Agent B (session: "db")     → focus: #15 Schema migration
└── Agent C (session: "test")   → focus: #18 Integration tests
                                  depends_on: [#12, #15]
```

**Four capabilities working together:**

| Challenge | Solution |
|-----------|----------|
| Interruptions | Persistent memory |
| Multi-agent | Session isolation |
| Scheduling | Dependency graph |
| Context explosion | Focus-driven retrieval |

Each agent maintains isolated focus. Orchestrators read `depends_on` for parallel scheduling. State persists across crashes, restarts, days.

**Result: Reliable multi-day, multi-agent workflows.**

---

## Get Started

**Claude Code (one command):**

```
/plugin marketplace add wayfind/origin-task
/plugin install intent-engine@wayfind/origin-task
```

Done. Plugin handles binary installation and integration.

**Other setups:**

```bash
# Install
brew install wayfind/tap/intent-engine  # or npm, cargo

# Use
ie status                         # Restore context
echo '{"tasks":[...]}' | ie plan  # Create tasks
ie log decision "chose X"         # Record decisions
ie search "keyword"               # Search history
```

---

## How It Works

```
Session Start → ie status → Context restored
                            ↓
Working       → ie plan   → Tasks updated
              → ie log    → Decisions recorded
                            ↓
Interruption  → State persisted automatically
                            ↓
Next Session  → ie status → Continue exactly where you left off
```

---

## Documentation

- [Quick Start](docs/en/guide/quickstart.md)
- [CLAUDE.md](CLAUDE.md) — AI integration guide
- [Command Reference](docs/en/guide/command-reference-full.md)

---

## License

MIT OR Apache-2.0

---

**Give your AI the memory it deserves.**
