#!/bin/bash
# Install git hooks for intent-engine development

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

# Color output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "🔧 Installing git hooks..."

# Check if we're in a git repository
if [ ! -d "$PROJECT_ROOT/.git" ]; then
    echo "❌ Error: Not a git repository"
    exit 1
fi

# Install pre-commit hook
if [ -f "$SCRIPT_DIR/git-hooks/pre-commit" ]; then
    echo -e "${BLUE}   Installing pre-commit hook...${NC}"
    cp "$SCRIPT_DIR/git-hooks/pre-commit" "$HOOKS_DIR/pre-commit"
    chmod +x "$HOOKS_DIR/pre-commit"
    echo -e "${GREEN}   ✓ pre-commit hook installed${NC}"
else
    echo "⚠️  Warning: pre-commit hook template not found"
fi

echo ""
echo -e "${GREEN}✅ Git hooks installation complete!${NC}"
echo ""
echo "The following hooks are now active:"
echo "  - pre-commit: Runs cargo fmt, cargo clippy, version checks, and doc updates"
echo ""
echo "To verify, try making a commit and you'll see the hooks in action."
