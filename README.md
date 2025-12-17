# Intent-Engine

**[中文](Readme.zh.md) | English**

[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](https://github.com/wayfind/intent-engine/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wayfind/intent-engine/branch/main/graph/badge.svg)](https://codecov.io/gh/wayfind/intent-engine)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](https://crates.io/crates/intent-engine)

---

## What is this?

**Intent-Engine is AI's external long-term memory.**

Think of it as a **shared notebook** between you and AI assistants (like Claude), where:
- You write down **strategic goals** ("Build authentication system")
- AI writes down **decisions** ("Chose JWT because...")
- Both of you can **pick up where you left off** - days or weeks later

```
┌─────────────────────────────────────────────────────────┐
│  Without Intent-Engine                                  │
├─────────────────────────────────────────────────────────┤
│  You: "Let's build authentication"                      │
│  AI:  "Sure!" [builds something]                        │
│  --- Next day ---                                       │
│  You: "Continue authentication"                         │
│  AI:  "What authentication? Starting from scratch..."   │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│  With Intent-Engine                                     │
├─────────────────────────────────────────────────────────┤
│  You: "Let's build authentication"                      │
│  AI:  "Sure!" [creates task, builds, records decisions] │
│  --- Next day ---                                       │
│  You: "Continue authentication"                         │
│  AI:  [reads task history] "Resuming... we chose JWT,  │
│       implemented basic auth, next: OAuth2 integration" │
└─────────────────────────────────────────────────────────┘
```

---

## Why do I need this?

### 🤔 The Problem

AI agents (Claude, GPT, etc.) are **stateless** - they forget everything when the chat ends:
- ❌ You can't ask AI to "continue that feature from last week"
- ❌ You can't see **why** AI made certain decisions
- ❌ Complex projects become a mess of copy-pasting old conversations

### ✅ The Solution

Intent-Engine gives AI a **persistent memory** stored on your computer:
- ✅ AI remembers what you were building and why
- ✅ Every decision is tracked with context
- ✅ You can pause work, come back anytime, and AI picks up instantly

---

## How AI Works Automatically

### Installation (30 seconds)

```bash
# Install via Cargo (Rust package manager)
cargo install intent-engine

# Or download binary from releases
# https://github.com/wayfind/intent-engine/releases

# Verify installation
ie --version
```

### AI's Automatic Workflow

**Every time you enter Claude Code, AI automatically:**

```bash
# 1. Search for unfinished tasks
ie search "todo doing"
# → Returns all pending and in-progress tasks with summaries

# 2. AI analyzes and selects important/priority tasks
# → Based on priority, dependencies, context

# 3. Start task via ie plan (plan embeds start)
echo '{"tasks":[{"name":"Implement authentication","status":"doing"}]}' | ie plan
# → Updates task status to doing
# → Automatically fetches ancestor task memory (full context)

# 4. Record decisions during work
ie log decision "Chose JWT over Session for stateless API"
ie log note "Completed token generation logic"

# 5. Update status when complete
echo '{"tasks":[{"name":"Implement authentication","status":"done"}]}' | ie plan
```

**If no unfinished tasks exist:**
- AI waits silently for your input
- When you propose a **large or long-term goal**, AI automatically launches `ie plan` to create new tasks

### Example: Human Proposes New Requirement

```bash
# You say: "Help me build a user login system"
# AI determines it's a large goal and automatically executes:

echo '{
  "tasks": [{
    "name": "Build user login system",
    "status": "doing",
    "spec": "JWT tokens, OAuth2 support, 7-day session lifetime",
    "priority": "high",
    "children": [
      {"name": "Implement JWT generation/validation", "status": "todo"},
      {"name": "Add OAuth2 integration", "status": "todo"},
      {"name": "Implement session management", "status": "todo"}
    ]
  }]
}' | ie plan

# After plan executes, automatically:
# ✓ Creates task tree (parent + children)
# ✓ Sets parent to doing (starts work)
# ✓ Fetches complete memory (if ancestor tasks exist)
# ✓ AI immediately begins implementation and records decisions
```

---

## Integration with AI Assistants

### Works with Claude Code (Zero Configuration!)

If you use **Claude Code** (Anthropic's official CLI), Intent-Engine is **fully automated**:

**Ready to use after installation:**

1. Install Intent-Engine: `cargo install intent-engine`
2. Launch Claude Code in your project
3. **No configuration needed!** Claude automatically:
   - Executes `ie search "todo doing"` at session start
   - Proactively asks if you want to continue when unfinished tasks are found
   - Automatically starts tasks via `ie plan` and fetches long-term memory
   - Continuously records key decisions with `ie log` during work
   - Automatically updates task status when complete

**Workflow Examples:**

```
# Scenario 1: Unfinished tasks exist
You: [Open Claude Code]
Claude: [Auto-executes ie search "todo doing"]
        "I found 3 pending tasks:
         1. Implement user authentication (todo)
         2. Refactor database layer (doing)
         3. Fix login bug (todo, high priority)

         I suggest continuing #2, shall we proceed?"
You: "Yes"
Claude: [Executes ie plan to start task]
        [Fetches complete context and history]
        "OK, I see we previously chose Repository pattern...continuing"

# Scenario 2: No tasks
You: [Open Claude Code] "Help me implement a REST API"
Claude: [Determines it's a large goal]
        [Auto-executes ie plan to create task tree]
        "I've created task 'Implement REST API' with 4 subtasks, starting now..."
```

📖 [Claude Code Integration Details](docs/en/integration/claude-code-system-prompt.md)

### Works with Any AI Tool

Intent-Engine is just a CLI tool - any AI that can run commands can use it:
- Gemini CLI
- Custom GPT agents
- Cursor AI
- Any chatbot with bash access

📖 [Generic Integration Guide](docs/en/integration/generic-llm.md)

---

## Core Features

### 🌳 Hierarchical Task Trees

Break big problems into smaller ones, just like you think:

```
Build Authentication System
├── Implement JWT
│   ├── Generate tokens
│   └── Validate tokens
└── Add OAuth2
    ├── Google provider
    └── GitHub provider
```

### 📝 Decision History (Events)

Every "why" is recorded:

```bash
ie log decision "Chose PostgreSQL over MongoDB for ACID guarantees"
ie log blocker "Waiting for design approval from team"
ie log milestone "MVP complete, ready for testing"
```

### 🎯 Declarative Workflow: ie plan is Core

**ie plan is not just for creating tasks — it's also how you start them**

```bash
# plan's triple role:

# 1️⃣ Create new task
echo '{"tasks":[{"name":"New task","spec":"..."}]}' | ie plan

# 2️⃣ Update existing task (idempotent)
echo '{"tasks":[{"name":"New task","spec":"Updated content"}]}' | ie plan

# 3️⃣ Start task = set status="doing"
echo '{"tasks":[{"name":"New task","status":"doing"}]}' | ie plan
# ✓ Task status becomes doing
# ✓ Automatically fetches ancestor task info (long-term memory)
# ✓ AI gets complete context to start work
```

**Complete workflow:**

```bash
# Session start: AI auto-searches for tasks
ie search "todo doing"

# If tasks found: AI starts via plan
echo '{"tasks":[{"name":"Existing task","status":"doing"}]}' | ie plan

# If no tasks: Wait for new requirement, then plan creates and starts
echo '{
  "tasks": [{
    "name": "New large goal",
    "status": "doing",  # Created directly in doing state
    "children": [...]
  }]
}' | ie plan

# During work: Record decisions
ie log decision "Chose option A for better performance"

# When complete: Update status
echo '{"tasks":[{"name":"Task name","status":"done"}]}' | ie plan
```

### 📊 Progress Reports

See what was accomplished:

```bash
ie search "authentication"  # Find all auth-related work
```

---

## What makes Intent-Engine different?

### vs. Claude Code TodoWriter

| Feature | Intent-Engine | TodoWriter |
|---------|--------------|------------|
| **Persistence** | Saved to disk, never lost | Lost when chat ends |
| **Decision history** | Full event log with reasoning | No history |
| **AI resume work** | Yes - load full context | No - starts from scratch |
| **Cross-session** | Yes | No |
| **Best for** | Strategic, multi-day work | Current session notes |

### vs. Jira / Linear / Asana

| Feature | Intent-Engine | Project Management Tools |
|---------|--------------|-------------------------|
| **AI integration** | Native CLI, JSON output | Web UI only (manual) |
| **Decision tracking** | Structured event stream | Unstructured comments |
| **Automation** | AI can create/update tasks | Requires manual input |
| **Focus** | Strategic "why" + technical specs | Tactical "when" + assignments |
| **Best for** | Human-AI collaboration | Human team coordination |

---

## Real-World Use Cases

### ✅ Multi-Day Development Projects

**Problem**: You're building a complex feature with AI over multiple sessions
**Solution**: Intent-Engine remembers progress, decisions, and next steps

### ✅ Code Refactoring

**Problem**: AI suggests changes but you need to verify them later
**Solution**: Record all refactoring decisions with reasoning

### ✅ Learning from AI

**Problem**: AI makes technical choices but you forget why
**Solution**: Event log becomes a learning document of best practices

### ✅ Team Handoff

**Problem**: Different team members (or AI agents) work on same project
**Solution**: Complete context and decision history available to everyone

---

## Documentation

### Quick Links

- 📖 **[Quick Start Guide](QUICKSTART.en.md)** - 5-minute tutorial
- 🔧 **[Installation Guide](docs/en/guide/installation.md)** - All installation methods
- 🤖 **[Claude Code Integration](docs/en/integration/claude-code-system-prompt.md)** - Zero-config setup
- 📚 **[Complete Command Reference](docs/en/guide/command-reference-full.md)** - All commands explained
- 🧠 **[AI Quick Guide](docs/en/guide/ai-quick-guide.md)** - For AI assistants

### Architecture

- **[CLAUDE.md](CLAUDE.md)** - For AI assistants: how to use Intent-Engine
- **[AGENT.md](AGENT.md)** - Technical details: data models, atomic operations
- **[The Intent-Engine Way](docs/en/guide/the-intent-engine-way.md)** - Design philosophy

---

## Download & Install

### Pre-built Binaries

Download for your platform:
- **[Latest Release](https://github.com/wayfind/intent-engine/releases/latest)**
  - Linux (x86_64)
  - macOS (Intel & Apple Silicon)
  - Windows (x86_64)

### From Source

```bash
# Requires Rust toolchain (https://rustup.rs)
cargo install intent-engine
```

### Package Managers

```bash
# Homebrew (macOS/Linux)
brew install wayfind/tap/intent-engine

# Cargo (cross-platform)
cargo install intent-engine
```

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](docs/en/contributing/contributing.md)

**Areas we'd love help with:**
- 📝 Documentation improvements
- 🐛 Bug reports and fixes
- 🌐 Translations
- 💡 Feature suggestions
- 🧪 More test coverage

---

## License

Dual-licensed under either:
- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

---

## FAQ

**Q: How does AI automatically use Intent-Engine?**
A: The system prompt instructs AI to auto-execute `ie search "todo doing"` at session start. When unfinished tasks are found, AI proactively asks if you want to continue. If no tasks exist, AI waits for you to propose large goals, then automatically creates task trees and starts work.

**Q: What's the relationship between ie plan and ie start?**
A: In v0.10.0+, **there's no separate start command**. `ie plan` with `status: "doing"` is how you start tasks. Plan automatically fetches ancestor task memory, giving AI complete context.

**Q: Do I need to know Rust?**
A: No! Intent-Engine is a pre-built binary. Just install and use.

**Q: Does it send data to the cloud?**
A: No. Everything is stored locally in `~/.intent-engine/` on your computer.

**Q: Can I use it without AI?**
A: Yes! It's a powerful task tracker for humans too. But it really shines when AI uses it.

**Q: Is it free?**
A: Yes, completely open-source and free forever.

**Q: What AI assistants work with it?**
A: Anything with CLI access: Claude Code (best), custom GPT agents, Gemini CLI, Cursor, etc.

**Q: How is this different from git?**
A: Git tracks **code changes**. Intent-Engine tracks **strategic decisions and context**. They complement each other.

**Q: What does "long-term memory" mean for tasks?**
A: When you start a task with `ie plan`, the system automatically fetches all ancestor tasks (parent, grandparent, etc.) and their complete decision history. This gives AI context for the entire project, not just the current task.

---

**Ready to give AI long-term memory?**

```bash
cargo install intent-engine
ie --version
```

Start with our [5-minute Quick Start →](QUICKSTART.en.md)
