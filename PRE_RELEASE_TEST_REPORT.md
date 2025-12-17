# Pre-Release Test Report for v0.10.0

**Date**: 2025-12-16
**Version**: 0.10.0
**Status**: 🔴 **NOT READY FOR RELEASE** - Critical issues found

---

## Executive Summary

A comprehensive test was conducted on Intent-Engine v0.10.0. The assessment identified **critical blockers** that must be addressed before release:

- ❌ **2 Test Failures** (1 integration, 1 unit)
- ❌ **Code Format Issues** (rustfmt failures)
- ❌ **Clippy Errors** (compilation errors with -D warnings)
- ❌ **Documentation Inconsistencies** (migration guide doesn't match actual CLI)

---

## 1. Test Results Summary

### 1.1 Unit Tests (Library)
```
Result: ❌ FAILED
- Passed: 384 tests
- Failed: 1 test
- Ignored: 2 tests
```

**Failed Test**:
- `dashboard::pid::tests::test_cleanup_stale_pid`
  - **Location**: `src/dashboard/pid.rs:257`
  - **Error**: `assertion failed: cleaned`
  - **Severity**: Medium (PID cleanup functionality may not work correctly)

### 1.2 Integration Tests
```
Result: ⚠️ MOSTLY PASSING (15/16 test files passed)
- cascade_tests: ✅ 0 passed, 8 ignored
- cjk_search_tests: ✅ 10 passed
- cli_special_chars_tests: ✅ 10 passed
- cli_tests: ✅ 25 passed
- dashboard_integration_test: ✅ 2 passed
- dashboard_integration_tests: ✅ 7 passed, 3 ignored
- dependency_tests: ✅ 9 passed
- doctor_cli_tests: ✅ 13 passed
- doctor_command_tests: ✅ 2 passed
- error_handling_tests: ✅ 7 passed, 8 ignored
- event_filtering_tests: ✅ 12 passed
- focus_switching_tests: ✅ 8 passed
- init_cli_tests: ✅ 6 passed
- init_command_tests: ✅ 7 passed
- integration_tests: ✅ 14 passed
- interface_spec_test: ❌ 5 passed, 1 FAILED
```

**Failed Test**:
- `interface_spec_test::test_cli_help_matches_spec`
  - **Location**: `tests/interface_spec_test.rs:88`
  - **Error**: "task add --help should document name parameter"
  - **Root Cause**: Test expects `ie task add` command, but `task` subcommand has been removed in v0.10.0
  - **Severity**: Critical (indicates test/docs not updated for new CLI structure)

---

## 2. Code Quality Issues

### 2.1 Format Check (rustfmt)
```
Result: ❌ FAILED
```

**Issues Found**: 21 formatting violations across multiple files:
- `src/cli_handlers/dashboard.rs` (3 violations)
- `src/cli_handlers/mod.rs` (1 violation)
- `src/dashboard/cli_notifier.rs` (2 violations)
- `src/dashboard/pid.rs` (3 violations)
- `src/tasks.rs` (1 violation)
- `tests/doctor_cli_tests.rs` (2 violations)
- `tests/doctor_command_tests.rs` (2 violations)
- `tests/main_coverage_tests.rs` (1 violation)
- `tests/owner_mechanism_tests.rs` (6 violations)

**Fix**: Run `cargo fmt` to auto-fix all formatting issues.

### 2.2 Clippy Check
```
Result: ❌ FAILED (compilation errors)
```

**Critical Errors**:

1. **Private Type in Public Interface** (2 occurrences)
   - `src/cli_handlers/other.rs:34` - `CurrentAction` enum is private but used in public function `handle_current_command`
   - `src/cli_handlers/other.rs:105` - `EventCommands` enum is private but used in public function `handle_event_command`
   - **Fix**: Either make enums `pub` or make functions non-public

2. **Redundant Pattern Matching**
   - `src/dashboard/pid.rs:88` - `matches!` can be simplified to `.is_ok()`
   - **Fix**: Replace `matches!(kill(...), Ok(_))` with `kill(...).is_ok()`

**Impact**: Code will not compile with `-D warnings` flag (used in CI).

---

## 3. Architecture Changes (v0.10.0)

### 3.1 CLI Structure Simplification

**Old CLI (v0.9.x)**:
```bash
ie task add "..."
ie task start <id>
ie task done
ie task list
ie event add ...
ie current
ie search ...
```

**New CLI (v0.10.0)**:
```bash
ie plan          # Declarative task creation/update
ie log           # Quick event logging
ie search        # Unified search
ie init          # Project initialization
ie dashboard     # Dashboard management
ie doctor        # Health check
```

**Key Changes**:
- ❌ **Removed**: Entire `task` subcommand and all its operations
- ❌ **Removed**: Standalone `event`, `current`, `report` commands
- ✅ **Added**: `plan` command for declarative task management
- ✅ **Added**: `log` command for quick event recording
- ✅ **Unified**: `search` now covers both tasks and events

**Design Rationale**: Simplify CLI to 3 core verbs (`plan`, `log`, `search`) plus utilities.

### 3.2 MCP Server Removal

**Removed Components**:
- ❌ `src/mcp/mod.rs` (deleted)
- ❌ `src/mcp/server.rs` (deleted)
- ❌ `src/mcp/server_tests.rs` (deleted)
- ❌ `src/mcp/ws_client.rs` (deleted)
- ❌ `mcp-server.json` (deleted)

**Impact**: Moved from MCP-based to system prompt-based AI integration.

---

## 4. Documentation Consistency Issues

### 4.1 Migration Guide (CRITICAL)

**File**: `MIGRATION_v0.10.0.md`

**Issues**:
The migration guide contains **22+ references** to commands that no longer exist:
- ❌ `ie add "Test task"` (line 85, 129, 146, etc.)
- ❌ `ie start 1` (line 86, 147, 321, etc.)
- ❌ `ie done` (line 87, 198, 327, etc.)
- ❌ `ie add "Task A" --priority high` (line 344)
- ❌ `ie add "Subtask A" --parent 1` (line 346)

**Impact**: Users following the migration guide will encounter "unrecognized subcommand" errors.

**Required Action**: Complete rewrite of migration guide to reflect actual CLI commands.

### 4.2 Other Documentation

**Files requiring review**:
1. `CLAUDE.md` - May reference old commands
2. `docs/spec-03-interface-current.md` - Interface specification
3. `RELEASE_NOTES_v0.10.0.md` - Release notes
4. `README.md` - Quick start examples
5. All language-specific docs in `docs/en/` and `docs/zh-CN/`

---

## 5. Blockers for Release

### 5.1 CRITICAL Blockers (Must Fix)

1. **Fix Clippy Errors** (prevents compilation in CI)
   - Private interface errors in `src/cli_handlers/other.rs`
   - Priority: P0

2. **Update Migration Guide** (misleads users)
   - Rewrite all command examples to use new CLI
   - Priority: P0

3. **Fix/Update interface_spec_test**
   - Test expects old CLI structure
   - Either fix test or remove it
   - Priority: P0

### 5.2 HIGH Priority (Should Fix)

4. **Fix rustfmt violations**
   - Run `cargo fmt` to fix 21 formatting issues
   - Priority: P1

5. **Fix test_cleanup_stale_pid**
   - Investigate PID cleanup failure
   - Priority: P1

6. **Update all documentation**
   - CLAUDE.md
   - spec-03-interface-current.md
   - README.md
   - Language-specific docs
   - Priority: P1

---

## 6. Test Coverage Statistics

### Overall Test Coverage
```
Total Tests: 387
- Unit Tests: 384 + 2 ignored
- Integration Tests: 161 tests across 16 files + 19 ignored
```

### Test Distribution by Module
- CJK Search: 10 tests ✅
- CLI Operations: 25 tests ✅
- Dashboard: 9 tests ✅
- Dependencies: 9 tests ✅
- Doctor Command: 15 tests ✅
- Error Handling: 15 tests (7 pass, 8 ignored)
- Event Filtering: 12 tests ✅
- Focus Switching: 8 tests ✅
- Initialization: 13 tests ✅
- Interface Spec: 6 tests (5 pass, 1 fail) ❌
- Integration: 14 tests ✅

---

## 7. Recommended Actions

### Immediate Actions (Before Release)

1. **Fix Compilation Issues**
   ```bash
   # Make enums public or functions private
   # Fix redundant pattern matching in pid.rs
   ```

2. **Fix Formatting**
   ```bash
   cargo fmt
   ```

3. **Fix/Update Failing Test**
   ```bash
   # Option A: Update test to check new CLI structure
   # Option B: Remove outdated test
   # Recommended: Option A
   ```

4. **Rewrite Migration Guide**
   - Document actual v0.10.0 CLI commands
   - Provide clear migration path from v0.9.x
   - Include examples using `ie plan`, `ie log`, `ie search`

5. **Audit All Documentation**
   ```bash
   # Search for references to old commands
   grep -r "ie add\|ie start\|ie done\|ie task" docs/ README.md CLAUDE.md
   ```

### Post-Fix Verification

6. **Re-run Full Test Suite**
   ```bash
   cargo test --all
   ```

7. **Verify CI Passes**
   ```bash
   cargo fmt --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all
   cargo build --release
   ```

8. **Manual CLI Testing**
   ```bash
   # Test new commands
   ie init
   ie plan
   ie log decision "Test"
   ie search "Test"
   ie dashboard status
   ie doctor
   ```

---

## 8. Breaking Changes Summary

### For Users Upgrading from v0.9.x

**Command Mapping**:
```
OLD                          NEW
---                          ---
ie task add "X"           → ie plan (with task spec)
ie task start <id>        → (via system prompt in Claude Code)
ie task done              → (via system prompt in Claude Code)
ie task list              → ie search "" --no-events
ie event add ...          → ie log <type> "..."
ie current                → (removed, use search)
ie report                 → (removed, use dashboard)
```

**Configuration**:
- ❌ Remove: `mcp-server.json` configuration
- ✅ New: System prompt integration (automatic)

**AI Integration**:
- Old: MCP tools via JSON-RPC
- New: System prompt + direct CLI invocation

---

## 9. Conclusion

**Release Readiness**: 🔴 **NOT READY**

The codebase has undergone significant architectural improvements (CLI simplification, MCP removal), but **critical issues prevent immediate release**:

1. Compilation errors (Clippy)
2. Test failures
3. Documentation severely out of sync

**Estimated Time to Fix**: 2-4 hours for critical issues

**Recommendation**:
1. Fix critical blockers (P0)
2. Run full verification suite
3. Consider extending to v0.10.1 if major docs update needed

---

## Appendix A: File Changes Summary

**Deleted**:
- `src/mcp/` (entire directory)
- `src/cli_handlers/task.rs`
- `tests/mcp_*.rs` (3 files)
- `.serena/` (entire directory)

**Backup**:
- `src/cli_handlers/task_backup.rs` (exists)

**New**:
- `src/cli_handlers/guide.rs`
- `src/dashboard/cli_notifier.rs`
- `src/dashboard/pid.rs`
- `docs/OWNER_MECHANISM.md`
- `MIGRATION_v0.10.0.md`
- `RELEASE_NOTES_v0.10.0.md`

**Modified** (100+ files):
- Extensive changes across CLI, dashboard, and tests

---

*Report generated by automated testing suite*
*For questions, contact: development team*
