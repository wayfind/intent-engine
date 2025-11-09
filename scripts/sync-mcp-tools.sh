#!/usr/bin/env bash
# Sync mcp-server.json version with Cargo.toml
# This script ensures the MCP server tool schema version stays in sync with the package version

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔄 Syncing MCP Tools Schema..."

# Extract version from Cargo.toml
CARGO_VERSION=$(grep '^version = ' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')
if [ -z "$CARGO_VERSION" ]; then
    echo -e "${RED}❌ Failed to extract version from Cargo.toml${NC}"
    exit 1
fi
echo "📦 Cargo version: $CARGO_VERSION"

# Extract version from mcp-server.json
MCP_VERSION=$(jq -r '.version' "$PROJECT_ROOT/mcp-server.json")
if [ -z "$MCP_VERSION" ]; then
    echo -e "${RED}❌ Failed to extract version from mcp-server.json${NC}"
    exit 1
fi
echo "🔧 MCP version: $MCP_VERSION"

# Compare versions
if [ "$CARGO_VERSION" = "$MCP_VERSION" ]; then
    echo -e "${GREEN}✅ Versions are in sync!${NC}"
    exit 0
fi

# Update version if different
echo -e "${YELLOW}⚠️  Versions are out of sync, updating...${NC}"

# Use jq to update version
jq --arg version "$CARGO_VERSION" '.version = $version' "$PROJECT_ROOT/mcp-server.json" > "$PROJECT_ROOT/mcp-server.json.tmp"
mv "$PROJECT_ROOT/mcp-server.json.tmp" "$PROJECT_ROOT/mcp-server.json"

echo -e "${GREEN}✅ Updated mcp-server.json version to $CARGO_VERSION${NC}"
echo "📝 Don't forget to commit this change!"
