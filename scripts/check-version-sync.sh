#!/bin/bash

# check-version-sync.sh
# Verifies version consistency between Cargo.toml and INTERFACE_SPEC.md
# Used as a pre-commit Git hook to prevent version inconsistencies

set -e

# Color output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Extract versions from source files
CARGO_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
SPEC_VERSION=$(head -10 docs/INTERFACE_SPEC.md | grep '^\*\*Version\*\*:' | sed 's/.*: //')

# Extract major.minor (x.y) components
CARGO_MAJOR_MINOR=$(echo "$CARGO_VERSION" | sed 's/\([0-9]*\.[0-9]*\)\..*/\1/')
SPEC_MAJOR_MINOR="$SPEC_VERSION"

echo "🔍 Checking version consistency..."
echo "   Cargo.toml: $CARGO_VERSION (interface: $CARGO_MAJOR_MINOR)"
echo "   INTERFACE_SPEC.md: $SPEC_VERSION"

# Check if major.minor versions match
if [ "$CARGO_MAJOR_MINOR" != "$SPEC_MAJOR_MINOR" ]; then
  echo -e "${RED}❌ Version mismatch detected!${NC}"
  echo -e "${RED}   Cargo.toml interface version: $CARGO_MAJOR_MINOR${NC}"
  echo -e "${RED}   INTERFACE_SPEC version: $SPEC_MAJOR_MINOR${NC}"
  echo ""
  echo "💡 Resolution steps:"
  echo "   1. If INTERFACE_SPEC changed, update Cargo.toml to match"
  echo "   2. If Cargo.toml changed, verify INTERFACE_SPEC needs update"
  echo "   3. INTERFACE_SPEC version is the source of truth for interface contracts"
  exit 1
fi

echo -e "${GREEN}✅ Version consistency verified${NC}"

# Check for hardcoded version numbers in documentation (warning only)
echo ""
echo "🔍 Checking for hardcoded version numbers in docs..."

# Search for patterns like "version 0.1.17" or "v0.1.17" but exclude INTERFACE_SPEC.md
HARDCODED_VERSIONS=$(grep -r -n "version [0-9]\+\.[0-9]\+\.[0-9]\+\|v[0-9]\+\.[0-9]\+\.[0-9]\+" docs/ \
  --include="*.md" \
  --exclude="INTERFACE_SPEC.md" \
  2>/dev/null || true)

if [ -n "$HARDCODED_VERSIONS" ]; then
  echo -e "${YELLOW}⚠️  Found potential hardcoded version numbers:${NC}"
  echo "$HARDCODED_VERSIONS"
  echo ""
  echo "💡 Consider using placeholders:"
  echo "   {{VERSION_INTERFACE}} for interface version (x.y)"
  echo "   {{VERSION_RELEASE}} for release version (x.y.z)"
  echo ""
  echo -e "${YELLOW}This is a warning only - commit will proceed${NC}"
fi

echo ""
echo -e "${GREEN}✅ Version sync check passed${NC}"
exit 0
