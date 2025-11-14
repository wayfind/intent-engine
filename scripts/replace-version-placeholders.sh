#!/bin/bash

# replace-version-placeholders.sh
# Replaces version placeholders in documentation with actual version numbers
# Used in CI/CD pipelines, release workflows, and pre-commit hooks

set -e

# Color output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "🔄 Replacing version placeholders in documentation..."

# Extract versions from source files
CARGO_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
INTERFACE_VERSION=$(head -10 docs/INTERFACE_SPEC.md | grep '^\*\*Version\*\*:' | sed 's/.*: //')

# Calculate previous minor version (e.g., 0.3.3 -> 0.3.2)
IFS='.' read -r major minor patch <<< "$CARGO_VERSION"
VERSION_PREVIOUS_MINOR="$major.$minor.$((patch - 1))"

# Get current date
DATE=$(date +%Y-%m-%d)

echo -e "${BLUE}   Interface version: $INTERFACE_VERSION${NC}"
echo -e "${BLUE}   Release version: $CARGO_VERSION${NC}"
echo -e "${BLUE}   Previous minor: $VERSION_PREVIOUS_MINOR${NC}"
echo -e "${BLUE}   Date: $DATE${NC}"

# Find and replace placeholders in all markdown files
FILES_MODIFIED=0

while IFS= read -r -d '' file; do
  # Check if file contains any placeholders
  if grep -q "{{VERSION_INTERFACE}}\|{{VERSION_RELEASE}}\|{{VERSION}}\|{{VERSION_PREVIOUS_MINOR}}\|{{DATE}}" "$file"; then
    echo "   Processing: $file"

    # Create backup
    cp "$file" "$file.bak"

    # Replace placeholders
    sed -i \
      -e "s/{{VERSION_INTERFACE}}/$INTERFACE_VERSION/g" \
      -e "s/{{VERSION_RELEASE}}/$CARGO_VERSION/g" \
      -e "s/{{VERSION_PREVIOUS_MINOR}}/$VERSION_PREVIOUS_MINOR/g" \
      -e "s/{{VERSION}}/$CARGO_VERSION/g" \
      -e "s/{{DATE}}/$DATE/g" \
      "$file"

    FILES_MODIFIED=$((FILES_MODIFIED + 1))
  fi
done < <(find docs -name "*.md" -type f -print0)

# Clean up backup files
find docs -name "*.md.bak" -type f -delete

if [ $FILES_MODIFIED -eq 0 ]; then
  echo -e "${GREEN}✅ No placeholders found - all documentation already concrete${NC}"
else
  echo ""
  echo -e "${GREEN}✅ Successfully replaced placeholders in $FILES_MODIFIED file(s)${NC}"
fi

exit 0
