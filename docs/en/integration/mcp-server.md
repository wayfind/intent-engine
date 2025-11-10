# Intent-Engine MCP Server Setup

This guide explains how to add Intent-Engine as an MCP (Model Context Protocol) server to Claude Code or Claude Desktop.

## Prerequisites

1. **Rust toolchain**: For building the MCP server binary
2. **Claude Code/Claude Desktop**: AI assistant application with MCP support

> **Note**: Intent-Engine uses a **Rust-native MCP server** with zero Python dependencies, offering superior performance and faster startup times.

## Installation Methods

### Method 1: Quick Install (Recommended)

```bash
# Clone or download Intent-Engine
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine

# Build and install (unified binary with CLI and MCP server)
cargo install --path .

# Run auto-configuration script
./scripts/install/install-mcp-server.sh
```

The auto-configuration script will:
- ✅ Detect your operating system and locate the correct config directory
- ✅ Automatically locate the MCP server binary
- ✅ Backup existing configuration (if any)
- ✅ Create or update `mcp_servers.json` configuration

### Method 2: Manual Setup

#### Step 1: Build MCP Server

```bash
# Build from source (unified binary with CLI and MCP server)
cargo build --release

# Install to user path (recommended)
cargo install --path .
# Installs to: ~/.cargo/bin/intent-engine

# Or copy to system path
sudo cp target/release/intent-engine /usr/local/bin/
```

#### Step 2: Configure Claude Code

Edit Claude Code's MCP settings file:

- **macOS/Linux**: `~/.config/claude-code/mcp_servers.json`
- **Windows**: `%APPDATA%\claude-code\mcp_servers.json`

Add Intent-Engine server configuration:

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "/home/user/.cargo/bin/intent-engine",
      "args": ["mcp-server"],
      "env": {
        "INTENT_ENGINE_PROJECT_DIR": "/path/to/your/project"
      },
      "description": "Strategic intent and task workflow management for human-AI collaboration"
    }
  }
}
```

**Configuration notes**:
- `command`: Absolute path to Intent-Engine binary
  - Using `cargo install`: `~/.cargo/bin/intent-engine`
  - Copied to system path: `/usr/local/bin/intent-engine`
- `args`: Must include `["mcp-server"]` to start MCP server mode
- `env`: Environment variables
  - `INTENT_ENGINE_PROJECT_DIR`: Absolute path to project root
  - Replace `/path/to/your/project` with your actual project path
- Use absolute paths for reliability

#### Step 3: Restart Claude Code

Restart Claude Code to load the new MCP server.

## Verification

### Manual Testing

```bash
# Test JSON-RPC interface (from project directory)
cd /path/to/your/project
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
  intent-engine mcp-server

# Or using environment variable
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
  INTENT_ENGINE_PROJECT_DIR=/path/to/your/project intent-engine mcp-server

# Should return JSON response with 13 tools
```

### Verify in Claude Code

After starting Claude Code, you should see **13 Intent-Engine MCP tools** available:

**Task Management**:
- `task_add` - Create strategic task
- `task_start` - Start task (atomic: set doing + set as current)
- `task_pick_next` - Intelligently recommend next task
- `task_spawn_subtask` - Create subtask and switch (atomic)
- `task_switch` - Switch tasks (atomic: pause current + start new)
- `task_done` - Complete task (validates all subtasks done)
- `task_update` - Update task properties
- `task_find` - Find tasks by status/parent
- `task_get` - Get detailed task information

**Event Tracking**:
- `event_add` - Record decisions/blockers/milestones (AI's external long-term memory)
- `event_list` - List event history for a task

**Workflow**:
- `current_task_get` - Get currently focused task
- `report_generate` - Generate work reports

## Usage Example

Once installed, Claude Code can use Intent-Engine automatically:

```
You: "Help me refactor the authentication system"

Claude: I'll create a task to track this work.
[Uses task_add tool]
[Uses task_start tool with events history]

"I've started task #42. Based on the code analysis, I see three main areas:
1. Password hashing (currently MD5, should upgrade to bcrypt)
2. Session management (no expiration mechanism)
3. OAuth integration (missing)

Let me create subtasks for each area..."
[Uses task_spawn_subtask for each area]
```

## Technical Advantages

### Why Rust Native Implementation?

| Feature | Rust Native MCP Server | Python Wrapper (Legacy) |
|---------|------------------------|------------------------|
| **Startup Time** | < 10ms | 300-500ms |
| **Memory Usage** | ~5MB | ~30-50MB |
| **Dependencies** | Zero | Requires Python 3.7+ |
| **Performance** | Native | IPC overhead |
| **Maintenance** | Single codebase | Dual maintenance |

### Architecture

```
Claude Code (Client)
      │
      ├─ JSON-RPC 2.0 over stdio ─┐
      │                           │
      ▼                           ▼
intent-engine mcp-server ──────> SQLite
  (Rust Native, unified binary) (.intent-engine/project.db)
```

## Troubleshooting

### MCP Server Not Showing in Claude Code

**Checklist**:
1. Verify MCP config file path:
   ```bash
   # Linux/macOS
   cat ~/.config/claude-code/mcp_servers.json

   # Windows PowerShell
   Get-Content $env:APPDATA\claude-code\mcp_servers.json
   ```

2. Validate JSON syntax:
   ```bash
   # Using jq to validate JSON
   jq . ~/.config/claude-code/mcp_servers.json
   ```

3. Check binary exists and is executable:
   ```bash
   which intent-engine
   # Should output: ~/.cargo/bin/intent-engine

   # Test run (requires project directory)
   cd /path/to/your/project
   echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
     intent-engine mcp-server
   ```

4. Check Claude Code logs:
   ```bash
   # macOS/Linux
   tail -f ~/.config/claude-code/logs/mcp.log

   # Windows
   # Check %APPDATA%\claude-code\logs\
   ```

5. **Restart Claude Code** (Required!)

### Permission Issues

```bash
# Ensure binary is executable
chmod +x ~/.cargo/bin/intent-engine

# Or
chmod +x /usr/local/bin/intent-engine
```

### Config Path Issues

If relative paths or `~` symbols don't work, use **absolute paths**:

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "/home/username/.cargo/bin/intent-engine",
      "args": ["mcp-server"],
      "env": {
        "INTENT_ENGINE_PROJECT_DIR": "/home/username/your-project"
      }
    }
  }
}
```

### Test MCP Server Functionality

```bash
# Complete test command (from project directory)
cd /path/to/your/project
cat << 'EOF' | intent-engine mcp-server
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
EOF

# Or using environment variable
cat << 'EOF' | INTENT_ENGINE_PROJECT_DIR=/path/to/your/project intent-engine mcp-server
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
EOF

# Expected: JSON response with 13 tools
# Errors will be output to stderr
```

## Uninstall

### Remove MCP Server Configuration

1. Edit `~/.config/claude-code/mcp_servers.json`
2. Delete the `"intent-engine"` entry
3. Restart Claude Code

### Uninstall Binary

```bash
# If installed via cargo install
cargo uninstall intent-engine

# If manually copied to system path
sudo rm /usr/local/bin/intent-engine
```

## Related Resources

- [CLAUDE.md](../../../CLAUDE.md) - Complete Claude integration guide
- [INTERFACE_SPEC.md](../../INTERFACE_SPEC.md) - Interface specification (authoritative)
- [MCP Tools Sync System](../technical/mcp-tools-sync.md) - Maintenance and testing
- [README.md](../../../README.md) - Project homepage

## Advanced Configuration

### Using Different Intent-Engine Databases for Different Projects

Intent-Engine supports multi-project isolation, with each project having its own database:

```
/home/user/project-a/.intent-engine/project.db  # Project A tasks
/home/user/project-b/.intent-engine/project.db  # Project B tasks
```

**Configuration approach**: Configure separate MCP server instances for each project using different `INTENT_ENGINE_PROJECT_DIR`:

```json
{
  "mcpServers": {
    "intent-engine-project-a": {
      "command": "/home/user/.cargo/bin/intent-engine",
      "args": ["mcp-server"],
      "env": {
        "INTENT_ENGINE_PROJECT_DIR": "/home/user/project-a"
      }
    },
    "intent-engine-project-b": {
      "command": "/home/user/.cargo/bin/intent-engine",
      "args": ["mcp-server"],
      "env": {
        "INTENT_ENGINE_PROJECT_DIR": "/home/user/project-b"
      }
    }
  }
}
```

Alternatively, use a single configuration and let Intent-Engine automatically discover the project based on Claude Code's current working directory (searches upward for `.intent-engine/` directory).

### Using with Claude Desktop

The Intent-Engine MCP server also works with Claude Desktop. Configuration file paths:

- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

Configuration format is the same:

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "/home/user/.cargo/bin/intent-engine",
      "args": ["mcp-server"],
      "env": {
        "INTENT_ENGINE_PROJECT_DIR": "/path/to/your/project"
      }
    }
  }
}
```
