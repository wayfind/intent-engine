#!/bin/bash
# Setup git hooks for automatic formatting

set -e

HOOK_DIR=".git/hooks"
PRE_COMMIT="$HOOK_DIR/pre-commit"

echo "Setting up git hooks..."

# Create pre-commit hook
cat > "$PRE_COMMIT" << 'EOF'
#!/bin/sh
# Auto-format Rust code and check version sync before commit

echo "Running cargo fmt..."
cargo fmt --all

# Check if any files were modified
if ! git diff --quiet; then
    echo "✓ Code formatted. Adding changes to commit..."
    git diff --name-only | grep '\.rs$' | xargs -r git add
fi

# Check version consistency
echo ""
echo "Checking version sync..."
if ! ./scripts/check-version-sync.sh; then
    echo "❌ Version sync check failed. Commit aborted."
    exit 1
fi

echo "✓ Pre-commit hook completed"
EOF

chmod +x "$PRE_COMMIT"

echo "✓ Git hooks installed successfully!"
echo ""
echo "Pre-commit hook will now:"
echo "  1. Auto-format Rust code (cargo fmt)"
echo "  2. Check version consistency (Cargo.toml ↔ spec-03-interface-current.md)"
echo ""
echo "To bypass: git commit --no-verify"
