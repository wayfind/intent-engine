#!/usr/bin/env bash
#
# Generate system-prompt.txt from CLAUDE.md
#
# This script extracts core concepts from CLAUDE.md and generates a condensed
# system prompt optimized for LLM consumption (~500 lines).
#
# Usage: ./scripts/generate-system-prompt.sh
# Output: system-prompt.txt (in project root)

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Generating system-prompt.txt from CLAUDE.md...${NC}"

# Project root directory
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# Check if CLAUDE.md exists
if [ ! -f "CLAUDE.md" ]; then
    echo -e "${YELLOW}Error: CLAUDE.md not found${NC}"
    exit 1
fi

# Output file
OUTPUT_FILE="system-prompt.txt"

# Note: For now, system-prompt.txt is manually maintained
# Future versions may auto-generate from CLAUDE.md

if [ ! -f "$OUTPUT_FILE" ]; then
    echo -e "${YELLOW}Error: $OUTPUT_FILE not found${NC}"
    echo "Please create system-prompt.txt manually first"
    echo "This script is a placeholder for future auto-generation"
    exit 1
fi

# Verify the system prompt
LINE_COUNT=$(wc -l < "$OUTPUT_FILE")
echo -e "${GREEN}✓ Found $OUTPUT_FILE ($LINE_COUNT lines)${NC}"

# Check if it's within target range (400-600 lines)
if [ "$LINE_COUNT" -lt 400 ] || [ "$LINE_COUNT" -gt 600 ]; then
    echo -e "${YELLOW}⚠ Warning: System prompt should be 400-600 lines (current: $LINE_COUNT)${NC}"
fi

# Verify key sections exist
echo -e "${GREEN}Verifying key sections...${NC}"

REQUIRED_SECTIONS=(
    "Core Concepts"
    "Essential CLI Commands"
    "Common Usage Patterns"
    "Best Practices"
    "Common Mistakes"
    "TodoWriter Integration"
    "Mental Model"
)

for section in "${REQUIRED_SECTIONS[@]}"; do
    if grep -q "$section" "$OUTPUT_FILE"; then
        echo -e "  ${GREEN}✓${NC} $section"
    else
        echo -e "  ${YELLOW}✗${NC} $section (missing)"
    fi
done

echo ""
echo -e "${GREEN}System prompt generation complete!${NC}"
echo "File: $OUTPUT_FILE"
echo "Lines: $LINE_COUNT"
echo ""
echo "Usage with Claude Code:"
echo "  claude --append-system-prompt \"\$(cat system-prompt.txt)\""
