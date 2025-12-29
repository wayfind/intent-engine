#!/bin/bash
# Master test runner for Phase 1 integration tests
# Runs all integration tests in sequence

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo ""
echo "======================================================================"
echo "  Phase 1 Focus Restoration - Integration Test Suite"
echo "======================================================================"
echo ""

FAILED_TESTS=()
PASSED_TESTS=()

run_test() {
    local test_name=$1
    local test_script=$2

    echo ""
    echo "----------------------------------------------------------------------"
    echo "Running: $test_name"
    echo "----------------------------------------------------------------------"

    if bash "$test_script"; then
        PASSED_TESTS+=("$test_name")
        echo ""
        echo "✅ $test_name: PASSED"
    else
        FAILED_TESTS+=("$test_name")
        echo ""
        echo "❌ $test_name: FAILED"
    fi
}

# Run all integration tests
run_test "Session Restore Workflow" "./test-session-restore-workflow.sh"

# Print summary
echo ""
echo "======================================================================"
echo "  Test Summary"
echo "======================================================================"
echo ""
echo "Passed: ${#PASSED_TESTS[@]}"
echo "Failed: ${#FAILED_TESTS[@]}"
echo ""

if [ ${#PASSED_TESTS[@]} -gt 0 ]; then
    echo "✅ Passed tests:"
    for test in "${PASSED_TESTS[@]}"; do
        echo "  - $test"
    done
    echo ""
fi

if [ ${#FAILED_TESTS[@]} -gt 0 ]; then
    echo "❌ Failed tests:"
    for test in "${FAILED_TESTS[@]}"; do
        echo "  - $test"
    done
    echo ""
    exit 1
fi

echo ""
echo "======================================================================"
echo "  ✅✅✅ ALL INTEGRATION TESTS PASSED! ✅✅✅"
echo "======================================================================"
echo ""
