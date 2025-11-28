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

- **macOS/Linux**: `~/.claude.json`
- **Windows**: `%APPDATA%\Claude\.claude.json`

> **⚠️ Version Note**: Claude Code v2.0.37+ uses `~/.claude.json` as the primary config file on Linux/macOS/WSL.
> Earlier versions may use different paths like `~/.claude/mcp_servers.json` or `~/.config/claude-code/mcp_servers.json`.
> If MCP tools don't appear after installation, verify your Claude Code version and config file location.

Add Intent-Engine server configuration:

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "/home/user/.cargo/bin/intent-engine",
      "args": ["mcp-server"],
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
- Project directory is automatically detected (based on `.git`, `Cargo.toml`, etc. markers)
- Use absolute paths for reliability

#### Step 3: Restart Claude Code

Restart Claude Code to load the new MCP server.

## Verification

### Manual Testing

```bash
# Test JSON-RPC interface (from project directory)
cd /path/to/your/project
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
  ie mcp-server

# Should return JSON response with 13 tools
# Project directory is automatically detected via .git, Cargo.toml, etc.
```

### Verify in Claude Code

After starting Claude Code, you should see **13 Intent-Engine MCP tools** available:

**Task Management**:
- `task_add` - Create strategic task
- `task_start` - Start task (atomic: set doing + set as current)
- `task_pick_next` - Intelligently recommend next task
- `task_spawn_subtask` - Create subtask and switch (atomic)
- `task_done` - Complete task (validates all subtasks done)
- `task_update` - Update task properties
- `task_list` - List tasks with filtering, sorting, and pagination support
  - Supports `status`, `parent` filters
  - Pagination via `limit` and `offset`
  - Sorting: `id`, `priority`, `time`, `focus_aware`
  - Returns `PaginatedTasks` with `has_more` flag
- `task_get` - Get detailed task information

**Event Tracking**:
- `event_add` - Record decisions/blockers/milestones (AI's external long-term memory)
- `event_list` - List event history for a task (supports filtering by type and time)

**Search**:
- `search` - Unified full-text search across tasks and events
  - FTS5 full-text search with snippet highlighting
  - Pagination support via `limit` and `offset`
  - Returns `PaginatedSearchResults` with separate task/event counts

**Workflow**:
- `current_task_get` - Get currently focused task
- `report_generate` - Generate work reports
- `plan` - Declarative batch task creation with dependencies

## Usage Examples

Once installed, Claude Code can use Intent-Engine automatically:

### Basic Task Creation and Management

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

### Pagination and Filtering

```
You: "Show me all my in-progress tasks, sorted by priority"

Claude: I'll list your doing tasks with priority sorting.
[Uses task_list with {status: 'doing', sort_by: 'priority', limit: 20, offset: 0}]

"You have 15 in-progress tasks. Here are the top 20 sorted by priority:
- Task #42: Refactor authentication (priority: high)
- Task #58: Optimize database queries (priority: high)
- Task #71: Update documentation (priority: medium)
..."

[Response includes pagination metadata: total_count: 15, has_more: false]
```

### Search with Pagination

```
You: "Find all tasks and discussions related to JWT authentication"

Claude: I'll search across both tasks and events.
[Uses search with {query: "JWT authentication", limit: 20, offset: 0}]

"Found 8 tasks and 12 events related to JWT authentication:

Tasks:
- Task #42: Implement JWT-based auth (match in spec)
- Task #45: Configure JWT secret rotation (match in name)

Events:
- Decision on Task #42: 'Chose HS256 for JWT signing...'
- Blocker on Task #45: 'JWT secret environment variable...'
..."

[Response includes: total_tasks: 8, total_events: 12, has_more: false]
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
ie mcp-server ──────> SQLite
  (Rust Native, unified binary) (.intent-engine/project.db)
```

## Troubleshooting

### MCP Server Not Showing in Claude Code

**Checklist**:
1. Verify MCP config file path:
   ```bash
   # Linux/macOS/WSL (Claude Code v2.0.37+)
   cat ~/.claude.json

   # Windows PowerShell (Claude Code v2.0.37+)
   Get-Content $env:APPDATA\Claude\.claude.json

   # Note: Earlier versions may use different paths:
   # - ~/.claude/mcp_servers.json
   # - ~/.config/claude-code/mcp_servers.json
   ```

2. Validate JSON syntax:
   ```bash
   # Using jq to validate JSON
   jq . ~/.claude.json
   ```

3. Check binary exists and is executable:
   ```bash
   which intent-engine
   # Should output: ~/.cargo/bin/intent-engine

   # Test run (requires project directory)
   cd /path/to/your/project
   echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
     ie mcp-server
   ```

4. Check Claude Code logs:
   ```bash
   # macOS/Linux
   tail -f ~/.claude/logs/mcp.log

   # Windows
   # Check %APPDATA%\Claude\logs\
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
      "args": ["mcp-server"]
    }
  }
}
```

### Test MCP Server Functionality

```bash
# Complete test command (from project directory)
cd /path/to/your/project
cat << 'EOF' | ie mcp-server
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
EOF

# Expected: JSON response with 13 tools
# Errors will be output to stderr
# Project directory is automatically detected (via .git, Cargo.toml, etc.)
```

## Uninstall

### Remove MCP Server Configuration

1. Edit `~/.claude.json` (or your version's config file)
2. Delete the `"intent-engine"` entry from `mcpServers` section
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

**Configuration approach**: Use a single configuration, and Intent-Engine will automatically discover the project based on Claude Code's current working directory:

```json
{
  "mcpServers": {
    "intent-engine": {
      "command": "/home/user/.cargo/bin/intent-engine",
      "args": ["mcp-server"]
    }
  }
}
```

**Automatic project discovery**:
- If in a project directory (detected by `.git`, `Cargo.toml`, etc. markers) → uses that project's database
- If not in a project → searches upward for nearest `.intent-engine/` directory
- Data isolation is completely automatic, no manual configuration needed

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
      "args": ["mcp-server"]
    }
  }
}
```

**Note**: Project directory is automatically detected based on Claude Desktop's working directory, no manual configuration needed.
