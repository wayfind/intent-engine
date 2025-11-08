#!/bin/bash
# Setup git hooks for automatic formatting

set -e

HOOK_DIR=".git/hooks"
PRE_COMMIT="$HOOK_DIR/pre-commit"

echo "Setting up git hooks..."

# Create pre-commit hook
cat > "$PRE_COMMIT" << 'EOF'
#!/bin/sh
# Auto-format Rust code before commit

echo "Running cargo fmt..."
cargo fmt --all

# Check if any files were modified
if ! git diff --quiet; then
    echo "✓ Code formatted. Adding changes to commit..."
    git diff --name-only | grep '\.rs$' | xargs -r git add
fi

echo "✓ Pre-commit hook completed"
EOF

chmod +x "$PRE_COMMIT"

echo "✓ Git hooks installed successfully!"
echo ""
echo "Now 'cargo fmt' will run automatically before each commit."
echo "To bypass: git commit --no-verify"
