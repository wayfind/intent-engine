#!/bin/bash
# Integration test for setup-claude-code command
# Tests hook installation, directory creation, permissions, and error handling

set -euo pipefail

# Helper function for assertions
assert_success() {
    if [ $? -ne 0 ]; then
        echo "❌ FAILED: $1"
        exit 1
    fi
    echo "✓ $1"
}

# Setup test environment
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
echo "Test directory: $TEST_DIR"

# Build intent-engine if needed
if ! command -v ie &> /dev/null; then
    echo "Building intent-engine..."
    cd "$OLDPWD"
    cargo build --release
    export PATH="$OLDPWD/target/release:$PATH"
    cd "$TEST_DIR"
fi

echo ""
echo "=== Test 1: Fresh directory setup ==="
echo ""

# Initialize workspace first
ie workspace init
assert_success "workspace init"

# Run setup
ie setup-claude-code
assert_success "setup-claude-code"

# Verify directory structure
if [ ! -d ".claude" ]; then
    echo "❌ .claude directory not created"
    exit 1
fi
echo "✓ .claude directory exists"

if [ ! -d ".claude/hooks" ]; then
    echo "❌ .claude/hooks directory not created"
    exit 1
fi
echo "✓ .claude/hooks directory exists"

if [ ! -f ".claude/hooks/session-start.sh" ]; then
    echo "❌ session-start.sh not created"
    exit 1
fi
echo "✓ session-start.sh exists"

# Verify file permissions on Unix systems
if [[ "$OSTYPE" != "msys" && "$OSTYPE" != "win32" ]]; then
    PERMS=$(stat -c "%a" ".claude/hooks/session-start.sh")
    if [[ ! "$PERMS" =~ [57][0-7][0-7] ]]; then
        echo "❌ Hook file is not executable (permissions: $PERMS)"
        exit 1
    fi
    echo "✓ Hook file is executable (permissions: $PERMS)"
fi

# Verify file content
HOOK_CONTENT=$(cat .claude/hooks/session-start.sh)
echo "$HOOK_CONTENT" | grep -q "#!/bin/bash"
assert_success "has bash shebang"

echo "$HOOK_CONTENT" | grep -q "ie session-restore"
assert_success "calls session-restore command"

echo "$HOOK_CONTENT" | grep -q "system-reminder"
assert_success "generates system-reminder output"

echo ""
echo "✅ Test 1 passed"
echo ""

# === Test 2: Existing .claude directory ===

echo "=== Test 2: Existing .claude directory ==="
echo ""

# Create new test directory
TEST_DIR2=$(mktemp -d)
cd "$TEST_DIR2"

ie workspace init

# Pre-create .claude directory
mkdir -p .claude

# Run setup - should still work
ie setup-claude-code
assert_success "setup with existing .claude dir"

if [ ! -f ".claude/hooks/session-start.sh" ]; then
    echo "❌ Hook not created with existing .claude dir"
    exit 1
fi
echo "✓ Hook created even with existing .claude dir"

rm -rf "$TEST_DIR2"

echo ""
echo "✅ Test 2 passed"
echo ""

# === Test 3: Hook already exists (should fail without --force) ===

echo "=== Test 3: Hook already exists (no --force) ==="
echo ""

cd "$TEST_DIR"

# Try to run setup again
if ie setup-claude-code 2>&1; then
    echo "❌ Should fail when hook already exists"
    exit 1
fi
echo "✓ Correctly fails when hook exists"

echo ""
echo "✅ Test 3 passed"
echo ""

# === Test 4: --force flag ===

echo "=== Test 4: Force overwrite ==="
echo ""

# Modify hook file
echo "# OLD CONTENT" > .claude/hooks/session-start.sh

# Run setup with --force
ie setup-claude-code --force
assert_success "setup with --force"

# Verify new content
HOOK_CONTENT=$(cat .claude/hooks/session-start.sh)
if echo "$HOOK_CONTENT" | grep -q "OLD CONTENT"; then
    echo "❌ Hook was not overwritten"
    exit 1
fi
echo "✓ Hook was overwritten"

echo "$HOOK_CONTENT" | grep -q "ie session-restore"
assert_success "new hook has correct content"

echo ""
echo "✅ Test 4 passed"
echo ""

# === Test 5: --dry-run flag ===

echo "=== Test 5: Dry-run mode ==="
echo ""

TEST_DIR3=$(mktemp -d)
cd "$TEST_DIR3"

ie workspace init

# Run with --dry-run
OUTPUT=$(ie setup-claude-code --dry-run 2>&1)
assert_success "dry-run execution"

echo "$OUTPUT" | grep -q "Would create"
assert_success "shows what would be done"

# Verify nothing was actually created
if [ -d ".claude/hooks" ]; then
    echo "❌ Dry-run should not create files"
    exit 1
fi
echo "✓ No files created in dry-run mode"

rm -rf "$TEST_DIR3"

echo ""
echo "✅ Test 5 passed"
echo ""

# === Test 6: Custom --claude-dir ===

echo "=== Test 6: Custom claude-dir ==="
echo ""

TEST_DIR4=$(mktemp -d)
cd "$TEST_DIR4"

ie workspace init

# Run with custom directory
ie setup-claude-code --claude-dir "./custom-claude"
assert_success "setup with custom directory"

if [ ! -f "./custom-claude/hooks/session-start.sh" ]; then
    echo "❌ Hook not created in custom directory"
    exit 1
fi
echo "✓ Hook created in custom directory"

rm -rf "$TEST_DIR4"

echo ""
echo "✅ Test 6 passed"
echo ""

# === Test 7: Hook actually works ===

echo "=== Test 7: Hook functionality ==="
echo ""

cd "$TEST_DIR"

# Create a task to test with
TASK_OUTPUT=$(ie task add --name "Test task" --spec-stdin <<EOF
This is a test specification
EOF
)
TASK_ID=$(echo "$TASK_OUTPUT" | jq -r '.id')

ie task start "$TASK_ID"
ie event add --type decision --data-stdin <<EOF
Test decision
EOF

# Execute the hook
export CLAUDE_WORKSPACE_ROOT="$TEST_DIR"
HOOK_OUTPUT=$(.claude/hooks/session-start.sh 2>&1)

if [ $? -ne 0 ]; then
    echo "❌ Hook execution failed"
    exit 1
fi
echo "✓ Hook executes successfully"

echo "$HOOK_OUTPUT" | grep -q "Focus: #$TASK_ID"
assert_success "hook output contains task info"

echo ""
echo "✅ Test 7 passed"
echo ""

# Cleanup
rm -rf "$TEST_DIR"

echo ""
echo "✅✅✅ All setup-claude-code tests passed! ✅✅✅"
