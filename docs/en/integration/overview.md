# Intent-Engine Integration Guide Overview

Intent-Engine can be integrated into various AI tools and workflows. This guide helps you choose the most suitable integration method.

---

## Integration Method Comparison

| Integration Method | Applicable Tools | Complexity | Feature Completeness | Recommended Scenario |
|-------------------|------------------|------------|---------------------|---------------------|
| [MCP Server](mcp-server.md) | Claude Code/Desktop | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Requires native tool calling, best Claude integration |
| [Claude Skill](.claude-code/intent-engine.skill.md) | Claude Code | ⭐ | ⭐⭐⭐ | Quick trial, lightweight integration |
| [Direct CLI Call](generic-llm.md) | Any AI Tool | ⭐ | ⭐⭐⭐⭐⭐ | Universal solution, adapt to any AI tool |
| [Gemini CLI](gemini-cli.md) | Google Gemini | ⭐⭐ | ⭐⭐⭐⭐ | Coming soon |
| [Cursor](cursor-integration.md) | Cursor Editor | ⭐⭐ | ⭐⭐⭐ | Coming soon |

---

## Recommended Paths

### Using Claude Code?

**Quick Trial (5 minutes):**
1. Install Intent-Engine: `cargo install intent-engine`
2. Claude Code will automatically recognize `.claude-code/intent-engine.skill.md`
3. Ask Claude to use Intent-Engine in conversation

**Production Integration (15 minutes):**
1. Follow [MCP Server Guide](mcp-server.md) to install
2. Restart Claude Code
3. Enjoy native tool calling experience

### Using Other AI Tools?

Refer to [Generic Integration Guide](generic-llm.md) to have AI call Intent-Engine via CLI.

**Core Approach:**
1. Add [AI Quick Guide](../guide/ai-quick-guide.md) to System Prompt
2. Have AI call `intent-engine` commands via `Bash` tool
3. AI parses JSON output and continues working

### Your Team Uses CI/CD?

Refer to [CI/CD Integration Guide](ci-cd.md) (coming soon) to use Intent-Engine in GitHub Actions/GitLab CI.

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
      │ MCP │       │Skill│   │Bash CLI│
      │Server│      │     │   │        │
      └──┬──┘       └──┬──┘   └───┬────┘
         └─────────────┴──────────┘
                     │
         ┌───────────▼───────────┐
         │  Intent-Engine CLI    │
         │  (JSON I/O)           │
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

| Feature | MCP Server | Claude Skill | Generic CLI |
|---------|-----------|-------------|-------------|
| Task Management | ✅ | ✅ | ✅ |
| Event Recording | ✅ | ✅ | ✅ |
| Work Reports | ✅ | ✅ | ✅ |
| Native Tool Calling | ✅ | ❌ | ❌ |
| Auto-completion | ✅ | ❌ | ❌ |
| Type Checking | ✅ | ❌ | ❌ |
| Setup Cost | High | Low | Low |
| Maintenance Cost | Low | Low | Medium |

---

## Quick Decision Tree

```
Start
  │
  ├─ Using Claude Code?
  │   ├─ Yes → Need best experience?
  │   │   ├─ Yes → MCP Server
  │   │   └─ No → Claude Skill
  │   └─ No ↓
  │
  ├─ Using Gemini CLI?
  │   └─ Yes → Gemini CLI integration (coming soon)
  │
  ├─ Using Cursor?
  │   └─ Yes → Cursor integration (coming soon)
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

# Or download pre-compiled binary
# https://github.com/wayfind/intent-engine/releases

# Verify installation
ie --version
```

For detailed installation instructions, see [Installation Guide](../guide/installation.md).

### 2. Choose Integration Method

Select the integration method that suits you based on the comparison table above, then refer to the corresponding detailed guide:

- **Claude Code (MCP)**: [mcp-server.md](mcp-server.md)
- **Claude Code (Skill)**: [.claude-code/intent-engine.skill.md](../../../.claude-code/intent-engine.skill.md)
- **Generic Integration**: [generic-llm.md](generic-llm.md)

### 3. Verify Integration

After completing integration, verify with:

```bash
# Create test task
echo "Test Intent-Engine integration" | \
  ie task add --name "Integration Test" --spec-stdin

# Ask AI tool to view tasks
# For example in Claude Code:
# "Help me view all current tasks"
```

---

## Common Integration Questions

### Q: MCP Server vs Claude Skill, which should I choose?

**A**:
- **Trial phase**: Claude Skill (5-minute setup)
- **Production use**: MCP Server (more features, better experience)

### Q: Can I use multiple integration methods simultaneously?

**A**: Yes. All integration methods operate on the same SQLite database (`.intent-engine/project.db`), no conflicts.

### Q: What if my AI tool is not in the supported list?

**A**: Use [Generic CLI Integration](generic-llm.md). As long as your AI tool can execute Bash commands, you can integrate Intent-Engine.

### Q: Will AI automatically use Intent-Engine after integration?

**A**: You need to guide AI to use it in conversation. Recommend adding to System Prompt:

```
When working on complex, multi-session tasks, use Intent-Engine to track
strategic intent and decision history. See docs/en/guide/ai-quick-guide.md
for usage patterns.
```

---

## Next Steps

1. 📖 Read [The Intent-Engine Way](../guide/the-intent-engine-way.md) to understand design philosophy
2. 🚀 Complete [Quick Start](../../../QUICKSTART.en.md) to experience core features
3. 🔧 Choose integration method and start configuring
4. 💡 Refer to [AI Quick Guide](../guide/ai-quick-guide.md) to optimize AI usage

---

**Need Help?**

- [GitHub Issues](https://github.com/wayfind/intent-engine/issues)
- [Contributing Guide](../contributing/contributing.md)
