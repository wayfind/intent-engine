#!/bin/bash
# Automatic hooks setup - runs after git clone/checkout
# This ensures hooks are always installed when working with the repository

set -e

HOOK_DIR=".git/hooks"
PRE_COMMIT="$HOOK_DIR/pre-commit"

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    echo "Not in a git repository, skipping hook setup"
    exit 0
fi

# Check if hook is already installed
if [ -f "$PRE_COMMIT" ] && grep -q "cargo fmt" "$PRE_COMMIT" 2>/dev/null; then
    # Hook already installed
    exit 0
fi

echo ""
echo "🔧 Auto-installing git pre-commit hooks..."
echo ""

# Install the hook
./scripts/setup-git-hooks.sh

echo ""
echo "✅ Hooks installed! Your commits will be auto-formatted."
echo ""
