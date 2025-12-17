# Phase 1 Focus Restoration - Integration Tests

This directory contains integration tests for Phase 1 of the Speckit Guardian Integration Protocol.

## Test Files

### 1. `test-session-restore-workflow.sh`
**Purpose**: Tests the complete session-restore workflow including task creation, focus management, and event recording.

**Test Scenarios**:
- ✅ Complete workflow with focus (parent task, subtasks, siblings, events)
- ✅ No focus scenario (returns stats and guidance)
- ✅ Error scenario (workspace not found)

**Key Validations**:
- Correct JSON structure
- Parent/sibling/child relationships
- Event history preservation
- Suggested commands generation

### 2. `test-session-start-hook.sh`
**Purpose**: Tests the SessionStart hook integration and output formatting.

**Test Scenarios**:
- ✅ Hook execution with focused task
- ✅ Spec preview truncation
- ✅ Event types (decision, blocker, note) display
- ✅ No focus scenario output
- ✅ Error handling (graceful degradation)

**Key Validations**:
- `<system-reminder>` formatting
- Minimal style output (concise, high information density)
- Correct emoji usage (⚠️ for blockers)
- Executable permissions (Unix)

### 3. `test-setup-claude-code.sh`
**Purpose**: Tests the automated hook installation command.

**Test Scenarios**:
- ✅ Fresh directory setup
- ✅ Existing .claude directory handling
- ✅ Hook already exists (should fail without --force)
- ✅ --force flag (overwrite existing)
- ✅ --dry-run mode (preview only)
- ✅ --claude-dir custom directory
- ✅ Hook functionality verification

**Key Validations**:
- Directory structure creation
- File permissions (executable on Unix)
- Hook content correctness
- Error handling

## Running the Tests

### Run All Tests
```bash
./run-all-tests.sh
```

### Run Individual Tests
```bash
./test-session-restore-workflow.sh
./test-session-start-hook.sh
./test-setup-claude-code.sh
```

## Prerequisites

- **jq**: JSON parsing in bash scripts
  ```bash
  # Ubuntu/Debian
  sudo apt-get install jq

  # macOS
  brew install jq
  ```

- **Intent-Engine**: Built and available in PATH
  ```bash
  cargo build --release
  export PATH="$PWD/target/release:$PATH"
  ```

## Test Environment

Each test creates a temporary directory and cleans up after completion. Tests are isolated and do not interfere with each other.

## Test Output Format

Tests use a clear assertion-based format:
```
✓ workspace init
✓ task add parent
✓ session-restore execution
✓ status is success
...
✅ Test 1 passed
```

Failed assertions show:
```
❌ FAILED: status is success
```

## Success Criteria

All tests must pass with:
- Exit code 0
- All assertions passing
- No errors in stderr (except intentional error tests)

## Coverage

These integration tests cover:
- ✅ CLI command execution
- ✅ JSON output parsing
- ✅ Hook script execution
- ✅ File system operations
- ✅ Error scenarios
- ✅ Cross-platform compatibility (Unix/Windows considerations)

## Related Documentation

- [Phase 1 Implementation Spec](../../docs/phase1-focus-restoration-spec.md)
- [Phase 1 Testing Spec](../../docs/phase1-testing-spec.md)
- [Speckit Guardian v2.0](../../docs/speckit-guardian.md)
