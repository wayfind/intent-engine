#!/bin/bash
set -e

echo "Intent-Engine MCP Server Installer"
echo "==================================="
echo

# Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)     MACHINE=Linux;;
    Darwin*)    MACHINE=Mac;;
    MINGW*|MSYS*|CYGWIN*)     MACHINE=Windows;;
    *)          MACHINE="UNKNOWN:${OS}"
esac

echo "Detected OS: ${MACHINE}"
echo

# Set config directory based on OS
if [ "$MACHINE" = "Mac" ] || [ "$MACHINE" = "Linux" ]; then
    CONFIG_DIR="$HOME/.config/claude-code"
elif [ "$MACHINE" = "Windows" ]; then
    CONFIG_DIR="$APPDATA/claude-code"
else
    echo "Unsupported OS: ${MACHINE}"
    exit 1
fi

MCP_CONFIG="$CONFIG_DIR/mcp_servers.json"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Determine MCP server location - only Rust binary supported
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [ -f "$HOME/.cargo/bin/intent-engine-mcp-server" ]; then
    MCP_SERVER="$HOME/.cargo/bin/intent-engine-mcp-server"
    SERVER_TYPE="Rust native (installed via cargo install)"
elif [ -f "/usr/local/bin/intent-engine-mcp-server" ]; then
    MCP_SERVER="/usr/local/bin/intent-engine-mcp-server"
    SERVER_TYPE="Rust native (system-wide)"
elif [ -f "$PROJECT_ROOT/target/release/intent-engine-mcp-server" ]; then
    MCP_SERVER="$PROJECT_ROOT/target/release/intent-engine-mcp-server"
    SERVER_TYPE="Rust native (local build)"
else
    echo "Error: Rust MCP server binary not found!"
    echo
    echo "Please build and install the MCP server first:"
    echo "  cd $PROJECT_ROOT"
    echo "  cargo build --release --bin intent-engine-mcp-server"
    echo "  cargo install --path . --bin intent-engine-mcp-server"
    echo
    echo "Or download a pre-built binary from:"
    echo "  https://github.com/wayfind/intent-engine/releases"
    exit 1
fi

echo "Configuration will be written to: $MCP_CONFIG"
echo "MCP server location: $MCP_SERVER"
echo "MCP server type: $SERVER_TYPE"
echo

# Optional: Check if intent-engine CLI is also installed
if ! command -v intent-engine &> /dev/null; then
    echo "Note: 'intent-engine' CLI command not found in PATH"
    echo "The MCP server will still work, but you may also want to install the CLI:"
    echo "  cd $PROJECT_ROOT"
    echo "  cargo install --path ."
    echo
fi

# Create config directory if it doesn't exist
mkdir -p "$CONFIG_DIR"

# Create or update MCP config
if [ -f "$MCP_CONFIG" ]; then
    echo "Found existing MCP config: $MCP_CONFIG"
    echo "Creating backup..."
    cp "$MCP_CONFIG" "$MCP_CONFIG.backup.$(date +%Y%m%d_%H%M%S)"
    echo "Backup created"
    echo

    # Check if intent-engine already configured
    if grep -q '"intent-engine"' "$MCP_CONFIG"; then
        echo "Intent-Engine MCP server is already configured"
        read -p "Overwrite existing configuration? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Installation cancelled"
            exit 0
        fi
    fi

    # Update existing config
    echo "Updating MCP configuration..."

    # Check if jq is available
    if command -v jq &> /dev/null; then
        # Use jq to update JSON
        TEMP_CONFIG=$(mktemp)
        jq --arg cmd "$MCP_SERVER" \
           '.mcpServers["intent-engine"] = {
               command: $cmd,
               args: [],
               description: "Strategic intent and task workflow management for human-AI collaboration"
           }' "$MCP_CONFIG" > "$TEMP_CONFIG"
        mv "$TEMP_CONFIG" "$MCP_CONFIG"
        echo "Configuration updated successfully (using jq)"
    else
        # Fallback: manual JSON manipulation (warning about potential issues)
        echo "Warning: jq not found, using basic text replacement"
        echo "For safer JSON editing, install jq: sudo apt-get install jq (or brew install jq on macOS)"

        # Check if intent-engine entry exists
        if grep -q '"intent-engine"' "$MCP_CONFIG"; then
            # Remove old intent-engine entry
            TEMP_CONFIG=$(mktemp)
            sed '/"intent-engine"/,/}/d' "$MCP_CONFIG" > "$TEMP_CONFIG"
            mv "$TEMP_CONFIG" "$MCP_CONFIG"
        fi

        # Add new intent-engine entry
        # This is a simplified approach - assumes mcpServers object exists
        TEMP_CONFIG=$(mktemp)
        sed '/"mcpServers": {/a\
    "intent-engine": {\
      "command": "'"$MCP_SERVER"'",\
      "args": [],\
      "description": "Strategic intent and task workflow management for human-AI collaboration"\
    },' "$MCP_CONFIG" > "$TEMP_CONFIG"
        mv "$TEMP_CONFIG" "$MCP_CONFIG"
        echo "Configuration updated (basic mode)"
    fi
else
    echo "Creating new MCP configuration..."
    cat > "$MCP_CONFIG" << EOF
{
  "mcpServers": {
    "intent-engine": {
      "command": "$MCP_SERVER",
      "args": [],
      "description": "Strategic intent and task workflow management for human-AI collaboration"
    }
  }
}
EOF
    echo "Configuration created successfully"
fi

echo
echo "✓ Installation complete!"
echo
echo "MCP Server Type: $SERVER_TYPE"
echo "Configuration: $MCP_CONFIG"
echo
echo "Next steps:"
echo "1. Restart Claude Code to load the MCP server"
echo "2. Verify Intent-Engine tools are available (13 tools should appear)"
echo "3. Test in Claude Code: Ask Claude to create a task for you"
echo
echo "To verify installation:"
echo "  # Check config file"
echo "  cat $MCP_CONFIG"
echo
echo "  # Test MCP server manually"
echo "  echo '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}' | $MCP_SERVER"
echo
echo "Documentation:"
echo "  README.md - MCP Service section"
echo "  docs/zh-CN/integration/mcp-server.md - Complete guide"
echo "  CLAUDE.md - AI integration guide"
echo
echo "To uninstall:"
echo "  Remove 'intent-engine' entry from $MCP_CONFIG"
echo "  Restart Claude Code"
