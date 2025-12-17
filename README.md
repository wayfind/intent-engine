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

## Show me how it works

### Installation (30 seconds)

```bash
# Install via Cargo (Rust package manager)
cargo install intent-engine

# Or download binary from releases
# https://github.com/wayfind/intent-engine/releases

# Verify installation
ie --version
```

### Example: Building Authentication (3 steps)

**Step 1: Tell AI what to build**

```bash
ie plan << 'JSON'
{
  "tasks": [{
    "name": "Build user authentication",
    "spec": "JWT tokens, OAuth2 support, 7-day session lifetime",
    "children": [
      {"name": "Implement JWT"},
      {"name": "Add OAuth2"}
    ]
  }]
}
JSON
```

**Step 2: AI works and records progress**

```bash
# AI updates task status as work progresses
echo '{"tasks":[{"name":"Build user authentication","status":"doing"}]}' | ie plan

# While working, AI records key decisions
ie log decision "Chose HS256 algorithm for JWT signing"
ie log decision "Store tokens in httpOnly cookies for security"

# Mark task complete when done
echo '{"tasks":[{"name":"Build user authentication","status":"done"}]}' | ie plan
```

**Step 3: Resume work anytime**

```bash
# Days later, check available tasks
ie list todo         # See pending tasks
ie list doing        # See in-progress tasks

# Get full task details including event history
ie get 1 --with-events

# Continue where you left off - AI now has full context
```

---

## Integration with AI Assistants

### Works with Claude Code (Zero Configuration!)

If you use **Claude Code** (Anthropic's official CLI), Intent-Engine works **out of the box**:

1. Install Intent-Engine (see above)
2. Chat with Claude in your project
3. Say: *"Let's use Intent-Engine to track our work"*
4. Claude automatically creates tasks, logs decisions, and resumes work

**No setup needed!** Claude Code detects Intent-Engine and uses it automatically.

📖 [Setup Guide for Claude Code](docs/en/integration/claude-code-system-prompt.md)

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

### 🎯 Declarative Workflow

AI manages tasks by **declaring state**, following a clear workflow:

```bash
# 1. Check available tasks
ie list todo

# 2. Get task details and history
ie get 5 --with-events

# 3. Start working (update status)
echo '{"tasks":[{"name":"My Task","status":"doing"}]}' | ie plan

# 4. Record progress and decisions
ie log decision "Chose REST over GraphQL for simplicity"
ie log note "Making good progress on API design"

# 5. Mark complete when done
echo '{"tasks":[{"name":"My Task","status":"done"}]}' | ie plan
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

---

**Ready to give AI long-term memory?**

```bash
cargo install intent-engine
ie --version
```

Start with our [5-minute Quick Start →](QUICKSTART.en.md)
