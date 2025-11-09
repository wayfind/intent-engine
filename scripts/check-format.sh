#!/bin/bash
# Format checker with helpful error messages
# This script is used by CI and can also be run locally

set -e

COLOR_RED='\033[0;31m'
COLOR_GREEN='\033[0;32m'
COLOR_YELLOW='\033[1;33m'
COLOR_NC='\033[0m' # No Color

echo "🔍 Checking code formatting..."
echo ""

# Run cargo fmt in check mode
if cargo fmt --all -- --check > /tmp/fmt-check.txt 2>&1; then
    echo -e "${COLOR_GREEN}✅ All code is properly formatted!${COLOR_NC}"
    exit 0
else
    echo -e "${COLOR_RED}❌ Formatting check failed!${COLOR_NC}"
    echo ""
    echo "The following files need formatting:"
    echo ""
    cat /tmp/fmt-check.txt
    echo ""
    echo -e "${COLOR_YELLOW}📝 How to fix:${COLOR_NC}"
    echo ""
    echo "  1. Run formatting locally:"
    echo "     cargo fmt --all"
    echo ""
    echo "  2. Commit the changes:"
    echo "     git add ."
    echo "     git commit --amend --no-edit"
    echo ""
    echo "  3. Push (force push if amending):"
    echo "     git push -f origin \$(git branch --show-current)"
    echo ""
    echo -e "${COLOR_YELLOW}💡 Prevent this in future:${COLOR_NC}"
    echo ""
    echo "  Install pre-commit hooks to auto-format:"
    echo "     make setup-hooks"
    echo ""
    echo "  Or run before each commit:"
    echo "     make fmt"
    echo ""
    echo -e "${COLOR_YELLOW}📖 More info:${COLOR_NC} .github/FORMATTING_GUIDE.md"
    echo ""

    exit 1
fi
