# Intent-Engine Integration Guide Overview

Intent-Engine can be integrated into various AI tools and workflows. This guide helps you choose the most suitable integration method.

---

## Integration Method Comparison

| Integration Method | Applicable Tools | Complexity | Feature Completeness | Recommended Scenario |
|-------------------|------------------|------------|---------------------|---------------------|
| [Claude Code Plugin](claude-code-system-prompt.md) | Claude Code | Minimal | Full | **Recommended** - One-click install |
| [System Prompt](claude-code-system-prompt.md) | Claude Code | Low | Full | Manual setup alternative |
| [Direct CLI Call](generic-llm.md) | Any AI Tool | Low | Full | Universal solution, adapt to any AI tool |

---

## Recommended Paths

### Using Claude Code?

**One-Click Install (Recommended):**
```bash
claude plugin marketplace add wayfind/origin-task
claude plugin install intent-engine
```

The plugin automatically:
- Runs `ie status` at every session start
- Auto-installs `ie` CLI via npm if not found
- Guides Claude to use `ie plan` instead of TodoWrite

**Manual Setup:**
1. Install Intent-Engine: `cargo install intent-engine`
2. Follow [System Prompt Guide](claude-code-system-prompt.md) to setup
3. Add system prompt to Claude Code configuration

### Using Other AI Tools?

Refer to [Generic Integration Guide](generic-llm.md) to have AI call Intent-Engine via CLI.

**Core Approach:**
1. Add [AI Quick Guide](../guide/ai-quick-guide.md) to System Prompt
2. Have AI call `ie` commands via `Bash` tool
3. AI parses output and continues working

---

## Integration Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      AI Tool Layer                       │
├─────────────────────────────────────────────────────────┤
│  Claude Code │ Gemini CLI │ Cursor │ Other Tools...     │
└────────┬──────────────┬──────────┬──────────────────────┘
         │              │          │
      ┌──▼──┐       ┌──▼──┐   ┌───▼────┐
      │Plugin│      │System│  │Bash CLI│
      │     │       │Prompt│  │        │
      └──┬──┘       └──┬──┘   └───┬────┘
         └─────────────┴──────────┘
                     │
         ┌───────────▼───────────┐
         │  Intent-Engine CLI    │
         │  (ie status/plan/log) │
         └───────────┬───────────┘
                     │
         ┌───────────▼───────────┐
         │     SQLite Database   │
         │  .intent-engine/      │
         │    project.db         │
         └───────────────────────┘
```

---

## Integration Feature Matrix

| Feature | Plugin | System Prompt | Generic CLI |
|---------|--------|---------------|-------------|
| Task Management | ✅ | ✅ | ✅ |
| Event Recording | ✅ | ✅ | ✅ |
| Search History | ✅ | ✅ | ✅ |
| Auto Session Start | ✅ | ❌ | ❌ |
| Zero Configuration | ✅ | ❌ | ❌ |
| Cross-Platform | ✅ | ✅ | ✅ |
| Setup Cost | Minimal | Low | Low |
| Maintenance Cost | None | Low | Medium |

---

## Quick Decision Tree

```
Start
  │
  ├─ Using Claude Code?
  │   ├─ Yes → Plugin install (Recommended)
  │   │        claude plugin install intent-engine
  │   └─ No ↓
  │
  ├─ AI tool has CLI/Bash access?
  │   ├─ Yes → Generic CLI integration
  │   └─ No → Not supported
  │
  └─ Other tools → Generic CLI integration
```

---

## Getting Started with Integration

### 1. Install Intent-Engine

All integration methods require Intent-Engine to be installed first:

```bash
# Recommended method
cargo install intent-engine

# Or using Homebrew
brew install wayfind/tap/intent-engine

# Or using npm
npm install -g @origintask/intent-engine

# Verify installation
ie --version
```

For detailed installation instructions, see [Installation Guide](../guide/installation.md).

### 2. Choose Integration Method

Select the integration method that suits you based on the comparison table above, then refer to the corresponding detailed guide:

- **Claude Code (Plugin)**: `claude plugin install intent-engine`
- **Claude Code (System Prompt)**: [claude-code-system-prompt.md](claude-code-system-prompt.md)
- **Generic Integration**: [generic-llm.md](generic-llm.md)

### 3. Verify Integration

After completing integration, verify with:

```bash
# Create test task via CLI
echo '{"tasks":[{
  "name": "Integration Test",
  "status": "doing",
  "spec": "Test Intent-Engine integration"
}]}' | ie plan

# Check status
ie status

# Ask AI tool to view tasks
# For example in Claude Code:
# "Help me view all current tasks"
```

---

## Common Integration Questions

### Q: Plugin vs System Prompt, which should I choose?

**A**:
- **Plugin**: One-click install, auto session start, recommended for most users
- **System Prompt**: Manual setup, more control, for advanced users

### Q: Can I use multiple integration methods simultaneously?

**A**: Yes. All integration methods operate on the same SQLite database (`.intent-engine/project.db`), no conflicts.

### Q: What if my AI tool is not in the supported list?

**A**: Use [Generic CLI Integration](generic-llm.md). As long as your AI tool can execute Bash commands, you can integrate Intent-Engine.

### Q: Will AI automatically use Intent-Engine after integration?

**A**: With the plugin, yes - it runs `ie status` automatically at session start. With system prompt, you need to guide AI to use it in conversation.

---

## Next Steps

1. Read [The Intent-Engine Way](../guide/the-intent-engine-way.md) to understand design philosophy
2. Complete [Quick Start](../guide/quickstart.md) to experience core features
3. Choose integration method and start configuring
4. Refer to [AI Quick Guide](../guide/ai-quick-guide.md) to optimize AI usage

---

**Need Help?**

- [GitHub Issues](https://github.com/wayfind/intent-engine/issues)
- [Contributing Guide](../contributing/contributing.md)
