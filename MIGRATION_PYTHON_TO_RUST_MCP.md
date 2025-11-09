# Migration from Python to Rust MCP Server

## Summary

The Intent-Engine MCP (Model Context Protocol) server has been **migrated from Python to Rust** for improved performance, reduced dependencies, and better integration with the core library.

## Key Changes

### Before (Python Implementation)
- **File**: `mcp-server.py` (now deprecated as `mcp-server.py.deprecated`)
- **Implementation**: Wrapper script that calls `intent-engine` CLI via subprocess
- **Dependencies**: Python 3, subprocess overhead
- **Performance**: Lower (subprocess creation + JSON parsing overhead)

### After (Rust Implementation)
- **File**: `src/bin/mcp-server.rs`
- **Binary**: `intent-engine-mcp-server`
- **Implementation**: Native Rust binary using library functions directly
- **Dependencies**: None (statically linked)
- **Performance**: Higher (direct function calls, no subprocess overhead)

## Benefits

1. **Better Performance**: Direct library calls instead of subprocess + CLI parsing
2. **No Python Dependency**: Users no longer need Python 3 installed
3. **Single Binary**: Deployed alongside `intent-engine` main binary
4. **Type Safety**: Compile-time guarantees for protocol correctness
5. **Consistent Error Handling**: Uses the same error types as the core library

## Installation

### For New Users

```bash
# Build both binaries
cargo build --release

# Install both binaries
cargo install --path .

# Configure MCP server (automatically detects Rust version)
./install-mcp-server.sh
```

### For Existing Users (Migrating from Python)

The installation script (`install-mcp-server.sh`) has been updated to:

1. **Prefer Rust binary** if available:
   - `~/.cargo/bin/intent-engine-mcp-server` (installed)
   - `./target/release/intent-engine-mcp-server` (local build)

2. **Fallback to Python** if Rust binary not found:
   - `./mcp-server.py` (deprecated, shows warning)

3. **Auto-configure** the correct command in MCP config

To migrate, simply rebuild and reinstall:

```bash
cargo build --release --bin intent-engine-mcp-server
cargo install --path . --bin intent-engine-mcp-server
./install-mcp-server.sh  # Will automatically use Rust version
```

## Testing

The Rust MCP server has been tested with:

- ✅ `tools/list` - Returns all available tools
- ✅ `task_add` - Creates new tasks
- ✅ `task_start` - Starts tasks with events
- ✅ `event_add` - Adds events to tasks
- ✅ All other MCP tools defined in `mcp-server.json`

## Deprecation Timeline

- **Current**: Python version kept as fallback (`mcp-server.py.deprecated`)
- **Recommended**: Use Rust version for all new installations
- **Future**: Python version may be removed in a future release

## Technical Details

### Architecture

```
┌─────────────────────────────────────┐
│  MCP Client (Claude Code/Desktop)   │
└────────────┬────────────────────────┘
             │ JSON-RPC 2.0 (stdio)
             ▼
┌─────────────────────────────────────┐
│  intent-engine-mcp-server (Rust)    │
│  - JSON-RPC protocol handler        │
│  - Tool dispatcher                  │
└────────────┬────────────────────────┘
             │ Direct function calls
             ▼
┌─────────────────────────────────────┐
│  intent-engine library (Rust)       │
│  - TaskManager                      │
│  - EventManager                     │
│  - ReportManager                    │
│  - ProjectContext                   │
└─────────────────────────────────────┘
```

### Protocol Compatibility

The Rust implementation is **100% compatible** with the MCP protocol definition in `mcp-server.json`. All tool schemas, parameter types, and response formats remain identical.

## Troubleshooting

### "No MCP server found" Error

If you see this error, build the Rust MCP server:

```bash
cargo build --release --bin intent-engine-mcp-server
cargo install --path . --bin intent-engine-mcp-server
```

### Still Using Python Version

Check your MCP configuration:

```bash
cat ~/.config/claude-code/mcp_servers.json
```

If it shows `"command": "python3"`, re-run the installer:

```bash
./install-mcp-server.sh
```

### Performance Issues

The Python version has higher latency due to:
- Subprocess creation overhead (~50-100ms per call)
- Additional JSON parsing (CLI output → MCP response)

Switch to the Rust version for instant response times.

## Developer Notes

### Building Only MCP Server

```bash
cargo build --release --bin intent-engine-mcp-server
```

### Running MCP Server Standalone

```bash
# Start server (reads JSON-RPC from stdin, writes to stdout)
./target/release/intent-engine-mcp-server

# Test with echo
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | \
  ./target/release/intent-engine-mcp-server
```

### Adding New Tools

1. Update `mcp-server.json` with tool schema
2. Add handler function in `src/bin/mcp-server.rs`
3. Register handler in `handle_tool_call()` match statement
4. Test with JSON-RPC request

## Questions?

- **Issue Tracker**: https://github.com/wayfind/intent-engine/issues
- **Documentation**: https://docs.rs/intent-engine
- **MCP Protocol**: See `mcp-server.json` for tool schemas
