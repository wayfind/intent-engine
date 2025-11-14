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

# Set config file path based on OS
# NOTE: Claude Code v2.0.37+ uses ~/.claude.json as primary config on Linux/macOS
if [ "$MACHINE" = "Mac" ] || [ "$MACHINE" = "Linux" ]; then
    MCP_CONFIG="$HOME/.claude.json"
elif [ "$MACHINE" = "Windows" ]; then
    MCP_CONFIG="$APPDATA/Claude/.claude.json"
else
    echo "Unsupported OS: ${MACHINE}"
    exit 1
fi
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Find ie binary
if command -v ie &> /dev/null; then
    MCP_BINARY="$(which ie)"
    INSTALL_TYPE="System-wide (in PATH)"
elif [ -f "$HOME/.cargo/bin/ie" ]; then
    MCP_BINARY="$HOME/.cargo/bin/ie"
    INSTALL_TYPE="Cargo install"
elif [ -f "/usr/local/bin/ie" ]; then
    MCP_BINARY="/usr/local/bin/ie"
    INSTALL_TYPE="System-wide (/usr/local/bin)"
elif [ -f "$PROJECT_ROOT/target/release/ie" ]; then
    MCP_BINARY="$PROJECT_ROOT/target/release/ie"
    INSTALL_TYPE="Local build"
else
    echo "❌ Error: ie binary not found!"
    echo
    echo "Please build and install ie first:"
    echo "  cd $PROJECT_ROOT"
    echo "  cargo build --release"
    echo "  cargo install --path ."
    echo
    echo "Or download a pre-built binary from:"
    echo "  https://github.com/wayfind/intent-engine/releases"
    exit 1
fi

echo "Found ie: $MCP_BINARY"
echo "Install type: $INSTALL_TYPE"
echo "Config file: $MCP_CONFIG"
echo "Project root: $PROJECT_ROOT"
echo

# Create config directory if it doesn't exist
mkdir -p "$(dirname "$MCP_CONFIG")"

# Create or update MCP config
if [ -f "$MCP_CONFIG" ]; then
    echo "Found existing MCP config"
    echo "Creating backup..."
    cp "$MCP_CONFIG" "$MCP_CONFIG.backup.$(date +%Y%m%d_%H%M%S)"
    echo "✓ Backup created"
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
        # Use jq to update JSON safely
        TEMP_CONFIG=$(mktemp)
        jq --arg cmd "$MCP_BINARY" \
           --arg projdir "$PROJECT_ROOT" \
           '.mcpServers["intent-engine"] = {
               command: $cmd,
               args: ["mcp-server"],
               env: {
                   INTENT_ENGINE_PROJECT_DIR: $projdir
               },
               description: "Strategic intent and task workflow management for human-AI collaboration"
           }' "$MCP_CONFIG" > "$TEMP_CONFIG"
        mv "$TEMP_CONFIG" "$MCP_CONFIG"
        echo "✓ Configuration updated (using jq)"
    else
        # Fallback: warn and suggest jq
        echo "⚠ Warning: jq not found"
        echo "For safer JSON editing, install jq:"
        echo "  - macOS: brew install jq"
        echo "  - Linux: sudo apt-get install jq"
        echo
        echo "Manual configuration required:"
        echo "Edit $MCP_CONFIG and add:"
        echo '{'
        echo '  "mcpServers": {'
        echo '    "intent-engine": {'
        echo "      \"command\": \"$MCP_BINARY\","
        echo '      "args": ["mcp-server"],'
        echo '      "env": {'
        echo "        \"INTENT_ENGINE_PROJECT_DIR\": \"$PROJECT_ROOT\""
        echo '      },'
        echo '      "description": "Strategic intent and task workflow management for human-AI collaboration"'
        echo '    }'
        echo '  }'
        echo '}'
        exit 1
    fi
else
    echo "Creating new MCP configuration..."

    if command -v jq &> /dev/null; then
        echo '{}' | jq --arg cmd "$MCP_BINARY" \
           --arg projdir "$PROJECT_ROOT" \
           '.mcpServers["intent-engine"] = {
               command: $cmd,
               args: ["mcp-server"],
               env: {
                   INTENT_ENGINE_PROJECT_DIR: $projdir
               },
               description: "Strategic intent and task workflow management for human-AI collaboration"
           }' > "$MCP_CONFIG"
        echo "✓ Configuration created"
    else
        echo "❌ Error: jq is required for initial configuration"
        echo "Please install jq first:"
        echo "  - macOS: brew install jq"
        echo "  - Linux: sudo apt-get install jq"
        exit 1
    fi
fi

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✓ Installation complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
echo "Configuration:"
echo "  Command: $MCP_BINARY mcp-server"
echo "  Project: $PROJECT_ROOT"
echo "  Config:  $MCP_CONFIG"
echo
echo "⚠️  Config file location notes:"
echo "  - Claude Code v2.0.37+ uses ~/.claude.json (current default)"
echo "  - If MCP tools don't appear, ensure you're using Claude Code v2.0.37+"
echo "  - Config path behavior may vary across versions"
echo
echo "Next steps:"
echo "  1. Restart Claude Code to load the MCP server"
echo "  2. Verify Intent-Engine tools are available (13 tools)"
echo "  3. Test: Ask Claude to create a task for you"
echo
echo "To test manually:"
echo "  echo '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}' | \\"
echo "    INTENT_ENGINE_PROJECT_DIR=$PROJECT_ROOT \\"
echo "    $MCP_BINARY mcp-server"
echo
echo "Documentation:"
echo "  README.md - MCP Service section"
echo "  docs/zh-CN/integration/mcp-server.md"
echo "  CLAUDE.md - AI integration guide"
echo
echo "To uninstall:"
echo "  1. Remove 'intent-engine' entry from $MCP_CONFIG"
echo "  2. Restart Claude Code"
