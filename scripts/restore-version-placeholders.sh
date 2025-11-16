#!/bin/bash

# restore-version-placeholders.sh
# Restores version placeholders in documentation (reverse of replace-version-placeholders.sh)
# Used for maintaining source documentation files with placeholders

set -e

# Color output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "🔄 Restoring version placeholders in documentation..."

# Extract versions from source files
CARGO_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
INTERFACE_VERSION=$(head -10 docs/spec-03-interface-current.md | grep '^\*\*Version\*\*:' | sed 's/.*: //')

# Calculate previous minor version
IFS='.' read -r major minor patch <<< "$CARGO_VERSION"
VERSION_PREVIOUS_MINOR="$major.$minor.$((patch - 1))"

# Get current date
DATE=$(date +%Y-%m-%d)

echo -e "${BLUE}   Will replace concrete versions back to placeholders${NC}"
echo -e "${BLUE}   Release version $CARGO_VERSION → {{VERSION}}${NC}"
echo -e "${BLUE}   Previous minor $VERSION_PREVIOUS_MINOR → {{VERSION_PREVIOUS_MINOR}}${NC}"
echo -e "${BLUE}   Date $DATE → {{DATE}}${NC}"

# Find and restore placeholders in all markdown files
FILES_MODIFIED=0

while IFS= read -r -d '' file; do
  # Check if file contains concrete versions that should be placeholders
  if grep -q "$CARGO_VERSION\|$VERSION_PREVIOUS_MINOR\|$DATE" "$file"; then
    # Only process files in specific locations that use placeholders
    case "$file" in
      */technical/cjk-search.md)
        echo "   Processing: $file"

        # Create backup
        cp "$file" "$file.bak"

        # Restore placeholders (in reverse order to avoid partial replacements)
        sed -i \
          -e "s/$VERSION_PREVIOUS_MINOR/{{VERSION_PREVIOUS_MINOR}}/g" \
          -e "s/v$CARGO_VERSION+/v{{VERSION}}+/g" \
          -e "s/: $CARGO_VERSION$/: {{VERSION}}/g" \
          -e "s/：$CARGO_VERSION$/：{{VERSION}}/g" \
          -e "s/$DATE/{{DATE}}/g" \
          "$file"

        FILES_MODIFIED=$((FILES_MODIFIED + 1))
        ;;
    esac
  fi
done < <(find docs -name "*.md" -type f -print0)

# Clean up backup files
find docs -name "*.md.bak" -type f -delete

if [ $FILES_MODIFIED -eq 0 ]; then
  echo -e "${GREEN}✅ No concrete versions found - all documentation already uses placeholders${NC}"
else
  echo ""
  echo -e "${GREEN}✅ Successfully restored placeholders in $FILES_MODIFIED file(s)${NC}"
fi

exit 0
