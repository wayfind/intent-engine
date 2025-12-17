#!/bin/bash
# Integration test for session-restore workflow
# Tests complete flow: workspace init -> task creation -> focus -> session-restore

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
echo "=== Test 1: Complete workflow with focus ==="
echo ""

# 1. Initialize workspace
ie workspace init
assert_success "workspace init"

# 2. Create task tree
PARENT_OUTPUT=$(ie task add --name "Implement authentication" --spec-stdin <<EOF
Complete auth system with JWT and sessions.
Use HS256 algorithm for simplicity.
Store tokens in httpOnly cookies.
EOF
)
assert_success "task add parent"

# Extract parent task ID (assuming JSON output)
PARENT_ID=$(echo "$PARENT_OUTPUT" | jq -r '.id')
echo "  Parent task ID: $PARENT_ID"

# 3. Start parent task
ie task start "$PARENT_ID" --with-events
assert_success "task start parent"

# 4. Create subtasks
SUBTASK1_OUTPUT=$(ie task spawn-subtask --name "JWT implementation" --spec-stdin <<EOF
Use jsonwebtoken crate, HS256 algorithm
EOF
)
assert_success "spawn subtask 1"
SUBTASK1_ID=$(echo "$SUBTASK1_OUTPUT" | jq -r '.id')

# Switch back to parent and create more subtasks
ie task switch "$PARENT_ID"
assert_success "switch to parent"

ie task spawn-subtask --name "Session management"
assert_success "spawn subtask 2"

ie task switch "$PARENT_ID"
ie task spawn-subtask --name "Password hashing"
assert_success "spawn subtask 3"

# 5. Start working on JWT task and record events
ie task switch "$SUBTASK1_ID"
assert_success "switch to JWT task"

ie event add --type decision --data-stdin <<EOF
Chose HS256 algorithm for simplicity over RS256
EOF
assert_success "add decision event"

ie event add --type blocker --data-stdin <<EOF
Need to decide on token storage location (localStorage vs httpOnly cookies)
EOF
assert_success "add blocker event"

ie event add --type note --data-stdin <<EOF
jsonwebtoken crate looks most mature, has 1M+ downloads
EOF
assert_success "add note event"

# 6. Complete password hashing task (to test siblings.done)
PASSWORD_TASK_ID=$(ie task list --parent "$PARENT_ID" | jq -r '.[] | select(.name == "Password hashing") | .id')
ie task switch "$PASSWORD_TASK_ID"
ie task done
assert_success "complete password hashing task"

# 7. Switch back to JWT task
ie task switch "$SUBTASK1_ID"
assert_success "switch back to JWT task"

# 8. Execute session-restore
RESTORE_OUTPUT=$(ie session-restore --json)
assert_success "session-restore execution"

echo ""
echo "=== Validating session-restore output ==="
echo ""

# 9. Validate output structure
echo "$RESTORE_OUTPUT" | jq -e '.status == "success"'
assert_success "status is success"

echo "$RESTORE_OUTPUT" | jq -e ".current_task.id == $SUBTASK1_ID"
assert_success "current task is JWT"

echo "$RESTORE_OUTPUT" | jq -e ".parent_task.id == $PARENT_ID"
assert_success "parent task is auth"

echo "$RESTORE_OUTPUT" | jq -e '.siblings.total == 3'
assert_success "3 siblings total"

echo "$RESTORE_OUTPUT" | jq -e '.siblings.done == 1'
assert_success "1 sibling done (password hashing)"

echo "$RESTORE_OUTPUT" | jq -e '.siblings.doing == 1'
assert_success "1 sibling doing (JWT)"

echo "$RESTORE_OUTPUT" | jq -e '.recent_events | length == 3'
assert_success "3 recent events"

# Validate event types
echo "$RESTORE_OUTPUT" | jq -e '.recent_events[] | select(.type == "decision")'
assert_success "has decision event"

echo "$RESTORE_OUTPUT" | jq -e '.recent_events[] | select(.type == "blocker")'
assert_success "has blocker event"

echo "$RESTORE_OUTPUT" | jq -e '.recent_events[] | select(.type == "note")'
assert_success "has note event"

# Validate suggested commands
echo "$RESTORE_OUTPUT" | jq -e '.suggested_commands | length > 0'
assert_success "has suggested commands"

echo ""
echo "✅ Test 1 passed"
echo ""

# === Test 2: No focus scenario ===

echo "=== Test 2: No focus scenario ==="
echo ""

# Complete current task to clear focus
ie task done
assert_success "complete current task"

# Execute session-restore
RESTORE_OUTPUT=$(ie session-restore --json)
assert_success "session-restore with no focus"

echo "$RESTORE_OUTPUT" | jq -e '.status == "no_focus"'
assert_success "status is no_focus"

echo "$RESTORE_OUTPUT" | jq -e '.current_task == null'
assert_success "no current task"

echo "$RESTORE_OUTPUT" | jq -e '.stats.total_tasks > 0'
assert_success "has total tasks stat"

echo "$RESTORE_OUTPUT" | jq -e '.stats.todo > 0'
assert_success "has todo count"

echo ""
echo "✅ Test 2 passed"
echo ""

# === Test 3: Error scenario (workspace not found) ===

echo "=== Test 3: Error scenario ==="
echo ""

cd /tmp
RESTORE_OUTPUT=$(ie session-restore --json 2>&1 || true)

echo "$RESTORE_OUTPUT" | jq -e '.status == "error"'
assert_success "status is error"

echo "$RESTORE_OUTPUT" | jq -e '.error_type == "workspace_not_found"'
assert_success "error type correct"

echo "$RESTORE_OUTPUT" | jq -e '.recovery_suggestion != null'
assert_success "has recovery suggestion"

echo "$RESTORE_OUTPUT" | jq -e '.suggested_commands | length > 0'
assert_success "has suggested commands"

echo ""
echo "✅ Test 3 passed"
echo ""

# Cleanup
rm -rf "$TEST_DIR"

echo ""
echo "✅✅✅ All integration tests passed! ✅✅✅"
