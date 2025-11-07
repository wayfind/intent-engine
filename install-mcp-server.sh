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
MCP_SERVER="$SCRIPT_DIR/mcp-server.py"

echo "Configuration will be written to: $MCP_CONFIG"
echo "MCP server script location: $MCP_SERVER"
echo

# Check if intent-engine is installed
if ! command -v intent-engine &> /dev/null; then
    echo "Warning: 'intent-engine' command not found in PATH"
    echo "Please build and install intent-engine first:"
    echo "  cargo build --release"
    echo "  sudo cp target/release/intent-engine /usr/local/bin/"
    echo
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check if Python 3 is installed
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 is required but not found"
    echo "Please install Python 3 and try again"
    exit 1
fi

echo "Python 3 version: $(python3 --version)"
echo

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
    python3 -c "
import json
import sys

config_file = '$MCP_CONFIG'
with open(config_file, 'r') as f:
    config = json.load(f)

if 'mcpServers' not in config:
    config['mcpServers'] = {}

config['mcpServers']['intent-engine'] = {
    'command': 'python3',
    'args': ['$MCP_SERVER'],
    'description': 'Strategic intent and task workflow management for human-AI collaboration'
}

with open(config_file, 'w') as f:
    json.dump(config, f, indent=2)

print('Configuration updated successfully')
"
else
    echo "Creating new MCP configuration..."
    cat > "$MCP_CONFIG" << EOF
{
  "mcpServers": {
    "intent-engine": {
      "command": "python3",
      "args": ["$MCP_SERVER"],
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
echo "Next steps:"
echo "1. Restart Claude Code to load the MCP server"
echo "2. Verify Intent-Engine tools are available in Claude Code"
echo "3. Read THE_INTENT_ENGINE_WAY.md for usage philosophy"
echo
echo "To verify installation:"
echo "  cat $MCP_CONFIG"
echo
echo "To uninstall:"
echo "  Remove 'intent-engine' entry from $MCP_CONFIG"
echo "  Restart Claude Code"
