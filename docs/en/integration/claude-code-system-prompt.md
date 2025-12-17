# Using Intent-Engine with Claude Code (System Prompt)

**Version**: v0.10.0+
**Last Updated**: 2025-12-16

---

## Overview

Starting with v0.10.0, Intent-Engine uses a **system prompt** instead of MCP for Claude Code integration. This provides:

- ✅ **Zero context overhead** - No MCP tool definitions
- ✅ **Simple setup** - One command to configure
- ✅ **Full CLI access** - Direct shell command execution
- ✅ **Offline capable** - No background process needed

---

## Quick Start

### 1. Install Intent-Engine

```bash
cargo install intent-engine
# or
brew install intent-engine
```

### 2. Initialize Project

```bash
cd your-project
ie init
```

### 3. Configure Claude Code

```bash
# Option A: Append system prompt (if persistent)
claude --append-system-prompt "$(cat /path/to/intent-engine/system-prompt.txt)"

# Option B: Use hook script (if not persistent)
# See "Hook Script Setup" section below
```

### 4. Verify Setup

Ask Claude Code:
```
"What is intent-engine and how do I use it?"
```

Claude should respond with information about IE commands and usage patterns.

---

## System Prompt Persistence

### Testing Persistence

To check if `--append-system-prompt` persists across sessions:

1. Run the append command above
2. Restart Claude Code
3. Ask Claude: "Do you know about intent-engine?"

**If Claude remembers**: System prompt is persistent ✅
**If Claude doesn't remember**: Use hook script method ❌

---

## Hook Script Setup (If Not Persistent)

If system prompts don't persist, use a Claude Code hook:

### Method 1: Session Start Hook

Create `.claude/hooks/session-start.sh`:

```bash
#!/usr/bin/env bash
# Load Intent-Engine system prompt on session start

SYSTEM_PROMPT_FILE="${HOME}/.intent-engine/system-prompt.txt"

if [ ! -f "$SYSTEM_PROMPT_FILE" ]; then
    # Copy from installation
    IE_ROOT=$(dirname $(which ie))/../share/intent-engine
    if [ -f "$IE_ROOT/system-prompt.txt" ]; then
        mkdir -p "${HOME}/.intent-engine"
        cp "$IE_ROOT/system-prompt.txt" "$SYSTEM_PROMPT_FILE"
    fi
fi

if [ -f "$SYSTEM_PROMPT_FILE" ]; then
    export CLAUDE_SYSTEM_PROMPT="$(cat $SYSTEM_PROMPT_FILE)"
fi
```

Make it executable:
```bash
chmod +x .claude/hooks/session-start.sh
```

### Method 2: Environment Variable

Add to your shell profile (`~/.bashrc`, `~/.zshrc`):

```bash
# Intent-Engine system prompt
if [ -f "$HOME/.intent-engine/system-prompt.txt" ]; then
    export CLAUDE_SYSTEM_PROMPT="$(cat $HOME/.intent-engine/system-prompt.txt)"
fi
```

Reload your shell:
```bash
source ~/.bashrc  # or ~/.zshrc
```

---

## Usage Examples

### Plan-First Workflow (Primary - 90% of cases) ⭐

```bash
# Create task with status workflow
echo '{
  "tasks": [{
    "name": "Implement user authentication",
    "status": "doing",
    "priority": "high",
    "children": [
      {"name": "Design JWT schema", "status": "todo"},
      {"name": "Implement token generation", "status": "todo"}
    ]
  }]
}' | ie plan

# Plan automatically sets focus, begin work
ie log decision "Using JWT for stateless auth"

# Update status when done
echo '{"tasks": [{"name": "Design JWT schema", "status": "done"}]}' | ie plan
```

Why plan-first:
- ✅ Idempotent (safe to re-run)
- ✅ Status tracking built-in
- ✅ Hierarchy enforced
- ✅ Auto-focus on "doing" task

### Traditional Workflow (Advanced - 10% of cases)

```bash
# For single quick tasks or dynamic workflows
ie add "Quick fix"
ie start 1
ie done
```

### Context Recovery

```bash
# Resume work with plan
ie search "authentication"
echo '{"tasks": [{"name": "Implement user authentication", "status": "doing"}]}' | ie plan
ie event list --task-id 1  # Review decisions
```

---

## Troubleshooting

### Claude Doesn't Recognize IE Commands

**Symptoms**: Claude responds "I don't know about intent-engine"

**Solutions**:
1. Verify system prompt was loaded:
   ```bash
   # Check if file exists
   cat ~/.intent-engine/system-prompt.txt
   ```
2. Check Claude Code configuration
3. Try hook script method instead

### Commands Not Working

**Symptoms**: `ie` command not found

**Solutions**:
1. Verify installation:
   ```bash
   which ie
   ie --version
   ```
2. Add to PATH if needed:
   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   ```

### System Prompt Too Long

**Symptoms**: Claude Code rejects system prompt

**Solutions**:
1. Use condensed version (current: ~373 lines)
2. Remove "Advanced Features" section if needed
3. Keep only "Core Concepts" + "Essential Commands"

### Plan Status Constraint Error

**Symptoms**: Error "Only one task can have status='doing'"

**Cause**: Multiple tasks with `status: "doing"` in same plan request

**Solution**:
```bash
# ❌ Wrong: Two doing tasks
echo '{"tasks": [
  {"name": "A", "status": "doing"},
  {"name": "B", "status": "doing"}
]}' | ie plan

# ✅ Correct: One doing task
echo '{"tasks": [
  {"name": "A", "status": "doing"},
  {"name": "B", "status": "todo"}
]}' | ie plan
```

Why: Enforces single-focus workflow (one task at a time per batch)

### Hierarchical Doing (Parent + Child)

**Symptoms**: Need parent and child both in 'doing' status

**Understanding**:
- **Database level**: Supports multiple 'doing' tasks (parent + focused child)
- **Plan API level**: Enforces single 'doing' per request batch

**Solution** - Use separate plan calls:
```bash
# Step 1: Set parent to doing
echo '{"tasks": [{"name": "Implement authentication", "status": "doing"}]}' | ie plan
# Output: Task ID 42

# Step 2: Set child to doing (separate request)
echo '{"tasks": [{"name": "Design JWT schema", "status": "doing"}]}' | ie plan
# Output: Task ID 43
# Database now has TWO doing: parent (42) + child (43, focused)
```

**Why separate requests**: Plan API enforces single-focus constraint per batch, while database supports hierarchical workflows. Each plan call can only mark one task as 'doing'.

---

## Best Practices

### 1. Update System Prompt After IE Upgrades

```bash
ie doctor  # Check for updates
# If updated, refresh system prompt
claude --append-system-prompt "$(cat /path/to/system-prompt.txt)"
```

### 2. Project-Specific Initialization

Always run `ie init` in each project:
```bash
cd new-project
ie init
```

### 3. Use Dashboard for Visualization

```bash
ie dashboard start
# Open http://localhost:11391
```

### 4. Regular Context Reviews

```bash
ie report --since 7d  # Weekly review
```

---

## Migration from MCP (v0.9.0 → v0.10.0)

### What Changed

- ❌ **Removed**: MCP server and tools
- ✅ **Added**: System prompt approach
- ✅ **Same**: All CLI commands remain unchanged
- ✅ **Simplified**: Single-direction communication (CLI → Dashboard)

### Architecture Changes

**Old (v0.9.0)**:
```
Claude Code → MCP Server → Dashboard ←→ Frontend
              (persistent connections, heartbeat)
```

**New (v0.10.0+)**:
```
Claude Code (ie CLI) → Local SQLite DB
                            ↓ (single notify)
                       Global Dashboard
                            ↓ (direct access)
                       All Project DBs ← Frontend
```

**Key Improvements**:
- No persistent connections needed
- No "online/offline" project states
- Dashboard can directly create/modify tasks in any project
- CLI operations are fully offline-capable

### Migration Steps

1. **Uninstall old MCP configuration**:
   ```bash
   # Remove from Claude Code MCP settings
   # Delete mcp-server.json references
   ```

2. **Install new version**:
   ```bash
   cargo install intent-engine
   ie --version  # Should be v0.10.0+
   ```

3. **Configure system prompt**:
   ```bash
   claude --append-system-prompt "$(cat system-prompt.txt)"
   ```

4. **Optional - Start global dashboard**:
   ```bash
   ie dashboard start
   # Dashboard now monitors all projects (no per-project servers)
   ```

5. **Verify**:
   Ask Claude: "How do I use intent-engine?"

### Breaking Changes

- MCP tools no longer available
- Use CLI commands instead (all functionality preserved)
- Dashboard communication is now single-direction (CLI → Dashboard)
- Projects no longer have "online/offline" states

---

## FAQ

**Q: Can I still use MCP?**
A: No, MCP support was removed in v0.10.0. Use system prompt + CLI.

**Q: Do I need Dashboard running?**
A: No, CLI works independently. Dashboard is optional for visualization.

**Q: How much context does this use?**
A: ~345 lines (~3-4K tokens), much less than MCP tools (~15K tokens).

**Q: Can I customize the system prompt?**
A: Yes, edit `system-prompt.txt` and reload. Keep core sections intact.

**Q: Does this work offline?**
A: Yes, no network connection needed for CLI operations.

---

## See Also

- [Quick Start Guide](../guide/quickstart.md)
- [Command Reference](../guide/command-reference-full.md)
- [Migration Guide](../../../MIGRATION_v0.10.0.md)

---

**Need Help?**
- GitHub Issues: https://github.com/user/intent-engine/issues
- Documentation: https://intent-engine.dev
