# Parallel Agent Support

Intent-Engine supports multiple AI agents working in parallel on the same project via **session isolation**. Each agent maintains its own independent focus (current task) without interfering with others.

**You don't need this if** you're a single user working on a single project — the default session handles everything automatically.

**You need this when** multiple terminals, processes, or agents are hitting the same project database simultaneously and need independent focus state.

---

## The Contract: `IE_SESSION_ID`

The `IE_SESSION_ID` environment variable is the first-class interface for parallel agent support. It is the **caller's responsibility** to set this value.

```
Priority: --session <id>  >  IE_SESSION_ID env var  >  default "-1"
```

When `IE_SESSION_ID` is set, every `ie` command operates in an isolated session:
- `ie task start` sets focus only for this session
- `ie status` returns the focus of this session
- Multiple agents with different `IE_SESSION_ID` values are fully independent

```bash
# Agent A — works on task 42
IE_SESSION_ID=agent-backend ie task start 42
IE_SESSION_ID=agent-backend ie status

# Agent B — works on task 67, completely isolated
IE_SESSION_ID=agent-frontend ie task start 67
IE_SESSION_ID=agent-frontend ie status
```

Session data is persisted in SQLite and automatically cleaned up (max 1000 sessions retained by last-active time).

---

## Claude Code

### Hook-Based Setup (Recommended)

Claude Code fires a `SessionStart` hook at the beginning of every session. The hook receives the session ID via stdin JSON, and can write environment variables via `$CLAUDE_ENV_FILE` — a file Claude Code provides that gets sourced into every subsequent Bash tool invocation.

> **Note**: `CLAUDE_ENV_FILE` support depends on your Claude Code version. Verify it is set
> before the hook runs: `echo "CLAUDE_ENV_FILE=${CLAUDE_ENV_FILE:-not set}"`.

**Step 1: Create the hook script**

```bash
mkdir -p ~/.claude/hooks
cat > ~/.claude/hooks/ie-session-init.sh << 'EOF'
#!/usr/bin/env bash
# Do NOT use set -e here: hook failure is non-fatal and should not block the session.

# Read session_id from Claude Code's SessionStart JSON.
# Fall back to "" if jq is missing or the session_id field is absent.
session_id=$(jq -r '.session_id // empty' 2>/dev/null) || session_id=""

# Inject IE_SESSION_ID into Claude Code's environment file so that every
# subsequent Bash tool call in this session inherits the correct value.
# CLAUDE_ENV_FILE is provided by Claude Code and sourced before each Bash call.
if [ -n "${CLAUDE_ENV_FILE:-}" ] && [ -n "$session_id" ]; then
    echo "export IE_SESSION_ID=\"$session_id\"" >> "$CLAUDE_ENV_FILE"
fi

# Show current task context for this session (output is visible to Claude).
if command -v ie &>/dev/null && [ -n "$session_id" ]; then
    IE_SESSION_ID="$session_id" ie status 2>/dev/null || true
fi
EOF
chmod +x ~/.claude/hooks/ie-session-init.sh
```

**Step 2: Register the hook in `~/.claude/settings.json`**

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "$HOME/.claude/hooks/ie-session-init.sh"
          }
        ]
      }
    ]
  }
}
```

After this, every Claude Code session automatically gets a unique `IE_SESSION_ID`, and all `ie` commands within that session are isolated.

### Parallel Subagents

Claude Code subagents (launched via the `Agent` tool) do **not** trigger `SessionStart`, so they inherit no `IE_SESSION_ID` by default. The orchestrating agent must generate a unique session ID for each subagent, claim the relevant task under that ID, and then pass the ID explicitly in the subagent's prompt.

**Complete orchestrator pattern:**

```bash
# Step 1: Generate collision-free session IDs for each subagent.
# Use uuidgen for uniqueness; fall back to timestamp+PID if unavailable.
SID_BACKEND="backend-$(uuidgen 2>/dev/null || echo "$(date +%s)-$$")"
SID_FRONTEND="frontend-$(uuidgen 2>/dev/null || echo "$(date +%s)-$$")"

# Step 2: Claim (start) each task under the corresponding session ID.
# This records which task each subagent "owns" in the database.
IE_SESSION_ID="$SID_BACKEND"  ie task start 42
IE_SESSION_ID="$SID_FRONTEND" ie task start 67

# Step 3: Spawn subagents, passing the pre-generated session ID in their prompt.
# The subagent must prefix every ie command with IE_SESSION_ID=<value>.
echo "Session ID for your ie commands: $SID_BACKEND"
echo "Session ID for your ie commands: $SID_FRONTEND"
```

When composing the subagent prompt, include the session ID explicitly:

```
Your Intent-Engine session ID is: backend-a1b2c3d4-...
Prefix every ie command with: IE_SESSION_ID=backend-a1b2c3d4-...

Example:
  IE_SESSION_ID=backend-a1b2c3d4-... ie status
  IE_SESSION_ID=backend-a1b2c3d4-... ie log decision "chose X"
```

### Project-Level Hook (Per-Project Isolation)

For project-specific setup, place the hook in `.claude/settings.json` within the project:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/ie-session-init.sh"
          }
        ]
      }
    ]
  }
}
```

---

## Codex (OpenAI Codex CLI)

Codex inherits environment variables from the shell it is launched in. Set `IE_SESSION_ID` before invoking Codex.

**Per-invocation (recommended for parallel runs):**

```bash
# Launch Codex agent for backend work
IE_SESSION_ID=codex-backend codex "implement the database schema for task #42"

# Launch Codex agent for frontend work (in parallel, different terminal)
IE_SESSION_ID=codex-frontend codex "implement the UI components for task #67"
```

**Shell profile setup (single agent, persistent session across shell restarts):**

```bash
# ~/.bashrc or ~/.zshrc
# Stable, unique ID: combines date and PID so it survives within a shell session
# but differs between independent shell invocations.
export IE_SESSION_ID="codex-$(date +%Y%m%d-%H%M%S)-$$"
```

---

## Generic Agent Tools

For any agent tool that can execute shell commands, the pattern is the same: ensure `IE_SESSION_ID` is set before `ie` commands run.

### Pattern 1: Environment variable (simplest)

```bash
# Set once; all ie commands in this process inherit it.
export IE_SESSION_ID="my-agent-$(uuidgen)"
ie status
ie task start 42
ie log decision "chose approach X"
```

### Pattern 2: Inline prefix (explicit, no side effects)

```bash
# Prefix every ie command — useful when the environment cannot be set globally.
IE_SESSION_ID=worker-1 ie status
IE_SESSION_ID=worker-1 ie task start 42
IE_SESSION_ID=worker-1 ie log decision "chose approach X"
```

### Pattern 3: CLI flag (highest priority, overrides env)

```bash
# Use --session flag — available on all ie commands.
ie status --session worker-1
ie task start 42 --session worker-1
ie log decision "chose approach X" --session worker-1
```

### Hook Integration Template

For tools that support lifecycle hooks:

```bash
#!/usr/bin/env bash
# agent-session-init.sh — run this at agent startup.
# Do NOT use set -e: partial failure here should not abort the agent.

# Reuse an existing IE_SESSION_ID if already set (e.g., for restarts).
# Otherwise generate a unique ID using uuidgen or a timestamp+PID fallback.
if [ -z "${IE_SESSION_ID:-}" ]; then
    IE_SESSION_ID="agent-$(uuidgen 2>/dev/null || echo "$(date +%s)-$$")"
    export IE_SESSION_ID
fi

echo "Intent-Engine session: $IE_SESSION_ID"
ie status
```

---

## Task Coordination

Session isolation handles *focus* independence. For task *work* coordination (preventing two agents from picking up the same task), use `depends_on` to sequence work:

```json
{
  "tasks": [
    {
      "name": "Backend API",
      "status": "doing",
      "spec": "## Goal\nImplement REST endpoints\n## Owner\nAgent: backend-agent"
    },
    {
      "name": "Frontend UI",
      "status": "todo",
      "depends_on": ["Backend API"],
      "spec": "## Goal\nBuild UI components\n## Owner\nAgent: frontend-agent"
    }
  ]
}
```

`ie task next` automatically skips tasks blocked by incomplete dependencies — each agent gets naturally routed to available work without conflicts.

**Explicit assignment via `ie task start`:**

```bash
# Agent A explicitly claims task 42
IE_SESSION_ID=agent-a ie task start 42

# Agent B explicitly claims task 67
IE_SESSION_ID=agent-b ie task start 67

# Both agents work independently, each sees their own current task
IE_SESSION_ID=agent-a ie status  # shows task 42
IE_SESSION_ID=agent-b ie status  # shows task 67
```

---

## Session Lifecycle

```
Agent Starts
     │
     ▼
Set IE_SESSION_ID (via hook / env / --session flag)
     │
     ▼
ie commands use isolated session
     │
     ├── ie task start <id>   → sets focus for THIS session only
     ├── ie status            → shows focus for THIS session only
     ├── ie log ...           → recorded globally (all agents share event log)
     └── ie search ...        → global search (no session filter)
     │
     ▼
Session retained until evicted
(max 1000 sessions; oldest by last-active time are removed first)
```

**Note**: Events (`ie log`) and search are global — all agents share the same task graph and event history. Session isolation applies only to the *current focus* (which task the agent is actively working on).

---

## Reference

| Interface | Priority | Notes |
|-----------|----------|-------|
| `--session <id>` CLI flag | Highest | Overrides all other sources |
| `IE_SESSION_ID` env var | Medium | Set by hooks or shell profile |
| Default `"-1"` | Lowest | All agents share this if neither is set |

**Valid session ID format**: Any non-empty string. Use meaningful names like `agent-backend-001` or UUIDs for easier debugging.

**Persistence**: Session state (current focus) persists across restarts as long as the same `IE_SESSION_ID` is used.

---

## See Also

- [Integration Overview](overview.md)
- [Claude Code Setup](claude-code-system-prompt.md)
- [Generic LLM Integration](generic-llm.md)
