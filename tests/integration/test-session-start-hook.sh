#!/bin/bash
# Integration test for SessionStart hook
# Tests that the hook script correctly parses session-restore output and formats it

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
echo "=== Test: SessionStart Hook Integration ==="
echo ""

# 1. Setup environment
ie workspace init
assert_success "workspace init"

ie setup --target claude-code
assert_success "setup for claude-code"

# Verify hook file exists
if [ ! -f ".claude/hooks/session-start.sh" ]; then
    echo "❌ Hook file not created"
    exit 1
fi
echo "✓ Hook file exists"

# Verify hook is executable (on Unix)
if [[ "$OSTYPE" != "msys" && "$OSTYPE" != "win32" ]]; then
    if [ ! -x ".claude/hooks/session-start.sh" ]; then
        echo "❌ Hook file is not executable"
        exit 1
    fi
    echo "✓ Hook file is executable"
fi

# 2. Create task with long spec to test preview truncation
LONG_SPEC="This is a test specification with enough text to test preview truncation functionality. "
LONG_SPEC="${LONG_SPEC}${LONG_SPEC}${LONG_SPEC}${LONG_SPEC}"  # Make it longer

TASK_OUTPUT=$(ie task add --name "Test authentication task" --spec-stdin <<EOF
$LONG_SPEC
EOF
)
TASK_ID=$(echo "$TASK_OUTPUT" | jq -r '.id')
assert_success "create task with long spec"

ie task start "$TASK_ID"
assert_success "start task"

# 3. Add events of different types
ie event add --type decision --data-stdin <<EOF
Made a decision to use JWT tokens
EOF
assert_success "add decision event"

ie event add --type blocker --data-stdin <<EOF
Blocked on choosing token storage method
EOF
assert_success "add blocker event"

ie event add --type note --data-stdin <<EOF
Reviewing security best practices
EOF
assert_success "add note event"

# 4. Simulate SessionStart hook trigger
export CLAUDE_WORKSPACE_ROOT="$TEST_DIR"
HOOK_OUTPUT=$(.claude/hooks/session-start.sh 2>&1)
assert_success "hook execution"

echo ""
echo "=== Validating hook output ==="
echo ""

# 5. Validate output format
echo "$HOOK_OUTPUT" | grep -q "Intent-Engine: Session Restored"
assert_success "has header"

echo "$HOOK_OUTPUT" | grep -q "Focus: #$TASK_ID 'Test authentication task'"
assert_success "has focus line with task ID and name"

echo "$HOOK_OUTPUT" | grep -q "Spec:"
assert_success "has spec section"

# Verify spec preview is truncated (should not contain full spec)
if echo "$HOOK_OUTPUT" | grep -q "$LONG_SPEC"; then
    echo "❌ Spec should be truncated but full spec found"
    exit 1
fi
echo "✓ Spec is truncated"

echo "$HOOK_OUTPUT" | grep -q "Recent decisions:"
assert_success "has decisions section"

echo "$HOOK_OUTPUT" | grep -q "JWT tokens"
assert_success "contains decision content"

echo "$HOOK_OUTPUT" | grep -q "⚠️  Blockers:"
assert_success "has blockers section with warning emoji"

echo "$HOOK_OUTPUT" | grep -q "token storage"
assert_success "contains blocker content"

echo "$HOOK_OUTPUT" | grep -q "Commands:"
assert_success "has commands hint"

# 6. Validate system-reminder tags
echo "$HOOK_OUTPUT" | grep -q "<system-reminder"
assert_success "has opening system-reminder tag"

echo "$HOOK_OUTPUT" | grep -q "priority=\"high\""
assert_success "has priority attribute"

echo "$HOOK_OUTPUT" | grep -q "</system-reminder>"
assert_success "has closing system-reminder tag"

# 7. Verify minimal style (should not be too verbose)
LINE_COUNT=$(echo "$HOOK_OUTPUT" | wc -l)
if [ "$LINE_COUNT" -gt 30 ]; then
    echo "⚠️  Warning: Output has $LINE_COUNT lines, might be too verbose"
else
    echo "✓ Output is concise ($LINE_COUNT lines)"
fi

echo ""
echo "=== Test: No focus scenario ==="
echo ""

# Complete task to clear focus
ie task done
assert_success "complete task"

# Run hook again
HOOK_OUTPUT=$(.claude/hooks/session-start.sh 2>&1)
assert_success "hook execution with no focus"

echo "$HOOK_OUTPUT" | grep -q "Intent-Engine: No active focus"
assert_success "has no focus header"

echo "$HOOK_OUTPUT" | grep -q "Tasks:"
assert_success "has tasks count"

echo "$HOOK_OUTPUT" | grep -q "ie pick-next"
assert_success "suggests pick-next command"

# Should NOT have task-specific sections
if echo "$HOOK_OUTPUT" | grep -q "Focus: #"; then
    echo "❌ Should not have focus line in no-focus scenario"
    exit 1
fi
echo "✓ No focus-specific content"

echo ""
echo "=== Test: Error scenario (Intent-Engine not available) ==="
echo ""

# Move to directory without workspace
cd /tmp
HOOK_OUTPUT=$(.claude/hooks/session-start.sh 2>&1)

# Hook should not crash
if [ $? -ne 0 ]; then
    echo "❌ Hook crashed on error"
    exit 1
fi
echo "✓ Hook handles errors gracefully"

echo "$HOOK_OUTPUT" | grep -q "Intent-Engine"
assert_success "has error message"

echo "$HOOK_OUTPUT" | grep -q "<system-reminder>"
assert_success "still uses system-reminder format"

echo ""
echo "✅ Hook integration test passed"
echo ""

# Cleanup
rm -rf "$TEST_DIR"

echo ""
echo "✅✅✅ All SessionStart hook tests passed! ✅✅✅"
