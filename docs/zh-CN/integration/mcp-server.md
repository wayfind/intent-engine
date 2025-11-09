# Intent-Engine MCP Server Setup

This guide explains how to add Intent-Engine as an MCP (Model Context Protocol) server to Claude Code.

## Prerequisites

1. **Intent-Engine installed**: Make sure `intent-engine` is in your PATH
2. **Python 3.7+**: Required for the MCP server wrapper
3. **Claude Code**: Desktop app with MCP support

## Installation Methods

### Method 1: Quick Install (Recommended)

```bash
# Clone or download Intent-Engine
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine

# Build and install
cargo build --release
sudo cp target/release/intent-engine /usr/local/bin/

# Install MCP server
./scripts/install/install-mcp-server.sh
```

### Method 2: Manual Setup

#### Step 1: Install Intent-Engine Binary

```bash
# Build from source
cargo build --release

# Or download pre-built binary from releases
# https://github.com/wayfind/intent-engine/releases

# Make sure it's in PATH
sudo cp target/release/intent-engine /usr/local/bin/
# Or add to PATH: export PATH=$PATH:/path/to/intent-engine
```

#### Step 2: Configure MCP Server in Claude Code

Edit Claude Code's MCP settings file:

**macOS/Linux**: `~/.config/claude-code/mcp_servers.json`
**Windows**: `%APPDATA%\claude-code\mcp_servers.json`

Add Intent-Engine server:

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "python3",
      "args": ["/path/to/intent-engine/mcp-server.py"],
      "description": "Strategic intent and task workflow management for human-AI collaboration"
    }
  }
}
```

Replace `/path/to/intent-engine/` with your actual path.

#### Step 3: Restart Claude Code

Restart Claude Code to load the new MCP server.

## Verification

In Claude Code, you should now see Intent-Engine tools available:

- `task_add` - Create strategic task
- `task_start` - Begin working on task
- `task_pick_next` - Select optimal tasks
- `task_spawn_subtask` - Create and switch to subtask
- `task_switch` - Switch between tasks
- `task_done` - Complete task
- `task_update` - Update task properties
- `event_add` - Record decisions/blockers/milestones
- `report_generate` - Generate work reports

## Usage Example

Once installed, Claude Code can use Intent-Engine automatically:

```
You: "Help me refactor the authentication system"

Claude: I'll create a task to track this work.
[Uses task_add tool]
[Uses task_start tool with --with-events]

"I've started task #42. Based on the code, I see three main areas:
1. Password hashing (currently MD5, should upgrade)
2. Session management (no expiration)
3. OAuth integration (missing)

Let me create subtasks for each..."
[Uses task_spawn_subtask for each area]
```

## Troubleshooting

### "Command not found: intent-engine"

Make sure `intent-engine` is in your PATH:
```bash
which intent-engine
# Should print: /usr/local/bin/intent-engine or similar
```

### "Permission denied"

Make MCP server executable:
```bash
chmod +x /path/to/intent-engine/mcp-server.py
```

### "Python not found"

Install Python 3:
```bash
# macOS
brew install python3

# Ubuntu/Debian
sudo apt-get install python3

# Windows
# Download from python.org
```

### MCP server not showing in Claude Code

1. Check MCP settings file path is correct
2. Verify JSON syntax is valid
3. Check Claude Code logs: `~/.config/claude-code/logs/`
4. Restart Claude Code

## Uninstall

To remove Intent-Engine MCP server:

1. Remove from `mcp_servers.json`
2. Restart Claude Code
3. Optionally remove binary: `sudo rm /usr/local/bin/intent-engine`

## See Also

- [The Intent-Engine Way](../guide/the-intent-engine-way.md) - Collaboration philosophy
- [README.md](../../../README.md) - Full command reference
- [Task Workflow Analysis](../technical/task-workflow-analysis.md) - Technical details
