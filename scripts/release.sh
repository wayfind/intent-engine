#!/usr/bin/env bash
# Release automation script
# Usage: ./scripts/release.sh [patch|minor|major|<version>]

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
info() {
    echo -e "${GREEN}ℹ${NC} $1"
}

warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

error() {
    echo -e "${RED}✗${NC} $1"
    exit 1
}

success() {
    echo -e "${GREEN}✓${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    error "This script must be run from the project root directory"
fi

# Check if git is clean
if [ -n "$(git status --porcelain)" ]; then
    warn "You have uncommitted changes. Please commit or stash them first."
    git status --short
    exit 1
fi

# Get bump type or version
BUMP_TYPE=${1:-patch}

if [ -z "$BUMP_TYPE" ]; then
    error "Usage: $0 [patch|minor|major|<version>]"
fi

info "Release type: $BUMP_TYPE"

# Check if cargo-edit is installed
if ! command -v cargo-set-version &> /dev/null; then
    info "Installing cargo-edit..."
    cargo install cargo-edit
fi

# Get current version
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -1 | cut -d'"' -f2)
info "Current version: $CURRENT_VERSION"

# Bump version
case $BUMP_TYPE in
    patch|minor|major)
        info "Bumping $BUMP_TYPE version..."
        cargo set-version --bump $BUMP_TYPE
        ;;
    *)
        info "Setting version to $BUMP_TYPE..."
        cargo set-version $BUMP_TYPE
        ;;
esac

# Get new version
NEW_VERSION=$(grep '^version = ' Cargo.toml | head -1 | cut -d'"' -f2)
success "New version: $NEW_VERSION"

# Update CLAUDE.md
if [ -f "CLAUDE.md" ]; then
    info "Updating CLAUDE.md..."
    sed -i.bak "s/^\*\*Version\*\*: .*/\*\*Version\*\*: $NEW_VERSION/" CLAUDE.md
    sed -i.bak "s/^\*\*Spec Version\*\*: .*/\*\*Spec Version\*\*: $NEW_VERSION/" CLAUDE.md
    rm CLAUDE.md.bak
    success "Updated CLAUDE.md"
fi

# Update INTERFACE_SPEC.md
if [ -f "docs/INTERFACE_SPEC.md" ]; then
    info "Updating INTERFACE_SPEC.md..."
    sed -i.bak "s/^Version: .*/Version: $NEW_VERSION/" docs/INTERFACE_SPEC.md
    rm docs/INTERFACE_SPEC.md.bak
    success "Updated INTERFACE_SPEC.md"
fi

# Update Cargo.lock
info "Updating Cargo.lock..."
cargo update -p intent-engine
success "Updated Cargo.lock"

# Run tests
info "Running tests..."
if cargo test --quiet; then
    success "All tests passed"
else
    error "Tests failed. Please fix before releasing."
fi

# Show changes
info "Changes to be committed:"
git diff --stat

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}Ready to release v$NEW_VERSION${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Next steps:"
echo "  1. Review the changes above"
echo "  2. Commit and push:"
echo "     git add Cargo.toml Cargo.lock CLAUDE.md docs/INTERFACE_SPEC.md"
echo "     git commit -m \"chore: bump version to $NEW_VERSION\""
echo "     git push origin main"
echo ""
echo "  3. Create and push tag:"
echo "     git tag -a \"v$NEW_VERSION\" -m \"Release v$NEW_VERSION\""
echo "     git push origin \"v$NEW_VERSION\""
echo ""
echo "  Or run this script with --auto to do it automatically"
echo ""

# Auto-commit if requested
if [ "$2" == "--auto" ] || [ "$2" == "-y" ]; then
    info "Auto-committing changes..."

    git add Cargo.toml Cargo.lock CLAUDE.md docs/INTERFACE_SPEC.md
    git commit -m "chore: bump version to $NEW_VERSION"

    # Delete existing tag if it exists
    TAG="v$NEW_VERSION"
    if git rev-parse "$TAG" >/dev/null 2>&1; then
        warn "Tag $TAG already exists locally, deleting..."
        git tag -d "$TAG"
    fi

    # Try to delete remote tag (ignore error if it doesn't exist)
    git push origin ":refs/tags/$TAG" 2>/dev/null && info "Deleted remote tag $TAG" || true

    # Create new tag
    git tag -a "$TAG" -m "Release $TAG"

    info "Pushing to remote..."
    git push origin main
    git push origin "$TAG"

    success "Release v$NEW_VERSION created and pushed!"
    echo ""
    echo "🚀 GitHub Actions will now build and publish the release"
fi
