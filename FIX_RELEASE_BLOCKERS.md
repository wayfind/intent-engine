# Fix Release Blockers for v0.10.0

This document provides step-by-step instructions to fix all critical issues identified in the pre-release test.

---

## Quick Fix Script

Run this script to fix all automatically fixable issues:

```bash
#!/bin/bash
# save as: fix-blockers.sh

set -e

echo "🔧 Fixing Intent-Engine v0.10.0 Release Blockers..."
echo

# Step 1: Fix formatting
echo "1️⃣ Fixing code formatting..."
cargo fmt
echo "✅ Formatting fixed"
echo

# Step 2: Run tests to identify specific failures
echo "2️⃣ Running tests to verify current state..."
cargo test --lib --test interface_spec_test 2>&1 | tee test-output.log
echo

echo "3️⃣ Manual fixes required:"
echo "   - Fix Clippy errors in src/cli_handlers/other.rs"
echo "   - Update interface_spec_test.rs"
echo "   - Fix test_cleanup_stale_pid in src/dashboard/pid.rs"
echo "   - Update MIGRATION_v0.10.0.md"
echo
echo "See FIX_RELEASE_BLOCKERS.md for detailed instructions"
```

---

## Critical Fixes (P0) - Must Complete Before Release

### Fix 1: Clippy Compilation Errors

**File**: `src/cli_handlers/other.rs`

**Problem**: Private types exposed in public interface

**Solution**: Make the enums public

```bash
# Apply this fix using sed or manual edit
```

**Manual Fix**:
1. Open `src/cli_handlers/other.rs`
2. Line 14: Change `enum CurrentAction {` to `pub enum CurrentAction {`
3. Line 20: Change `enum EventCommands {` to `pub enum EventCommands {`

**Verification**:
```bash
cargo clippy --all-targets -- -D warnings
```

---

### Fix 2: Redundant Pattern Matching

**File**: `src/dashboard/pid.rs:88`

**Problem**: `matches!` macro is redundant

**Solution**: Use `.is_ok()` instead

**Manual Fix**:
1. Open `src/dashboard/pid.rs`
2. Find line 88 (around `is_process_running` function)
3. Replace:
   ```rust
   matches!(
       kill(Pid::from_raw(pid as i32), None),
       Ok(_)
   )
   ```
   With:
   ```rust
   kill(Pid::from_raw(pid as i32), None).is_ok()
   ```

**Verification**:
```bash
cargo clippy --lib -- -D warnings
```

---

### Fix 3: Update Interface Spec Test

**File**: `tests/interface_spec_test.rs`

**Problem**: Test expects old CLI structure (`ie task add`)

**Solution Option A** (Recommended): Update test for new CLI

Replace the test function:

```rust
#[test]
fn test_cli_help_matches_spec() {
    let bin_path = get_binary_path();

    // Test 'plan' command help
    let output = Command::new(&bin_path)
        .args(["plan", "--help"])
        .output()
        .expect("Failed to run plan --help");

    let help_text = String::from_utf8_lossy(&output.stdout);

    // Verify plan command documents format parameter
    assert!(
        help_text.contains("--format") || help_text.contains("format"),
        "plan --help should document format parameter"
    );

    // Test 'log' command help
    let output = Command::new(&bin_path)
        .args(["log", "--help"])
        .output()
        .expect("Failed to run log --help");

    let help_text = String::from_utf8_lossy(&output.stdout);

    assert!(
        help_text.contains("event_type") || help_text.contains("decision") || help_text.contains("blocker"),
        "log --help should document event types"
    );
    assert!(
        help_text.contains("message"),
        "log --help should document message parameter"
    );
    assert!(
        help_text.contains("--task") || help_text.contains("task"),
        "log --help should document task parameter"
    );

    // Test 'search' command help
    let output = Command::new(&bin_path)
        .args(["search", "--help"])
        .output()
        .expect("Failed to run search --help");

    let help_text = String::from_utf8_lossy(&output.stdout);

    assert!(
        help_text.contains("query"),
        "search --help should document query parameter"
    );
    assert!(
        help_text.contains("--tasks") || help_text.contains("tasks"),
        "search --help should document tasks flag"
    );
    assert!(
        help_text.contains("--events") || help_text.contains("events"),
        "search --help should document events flag"
    );

    println!("✅ CLI help output contains documented parameters for new command structure");
}
```

**Solution Option B**: Remove the test temporarily

If you need to release urgently:
```bash
# Comment out or remove the test
# Not recommended - tests are documentation
```

**Verification**:
```bash
cargo test --test interface_spec_test
```

---

### Fix 4: Update Migration Guide

**File**: `MIGRATION_v0.10.0.md`

**Problem**: Contains 22+ references to removed commands

**Solution**: Complete rewrite of command examples

**Search and Replace Strategy**:

1. Find all old command patterns:
```bash
grep -n "ie add\|ie start\|ie done\|ie task" MIGRATION_v0.10.0.md
```

2. Replace with new CLI patterns:

| Old Command | New Command | Notes |
|-------------|-------------|-------|
| `ie add "Task"` | `ie plan` (with JSON/YAML) | More complex, needs full example |
| `ie start 1` | Via Claude Code system prompt | Not direct CLI |
| `ie done` | Via Claude Code system prompt | Not direct CLI |
| `ie task list` | `ie search "" --no-events` | Or use dashboard |
| `ie event add ...` | `ie log <type> "..."` | Simplified syntax |

**Example replacement**:

**OLD**:
```bash
# Test basic functionality
ie init
ie add "Test task"
ie start 1
ie done
```

**NEW**:
```bash
# Test basic functionality
ie init

# Create tasks (using plan)
echo '{
  "tasks": [
    {"name": "Test task", "priority": "medium"}
  ]
}' | ie plan

# Task operations now done via Claude Code system prompt
# or Dashboard UI, not direct CLI commands
```

**Quick fix script**:
```bash
# Create backup
cp MIGRATION_v0.10.0.md MIGRATION_v0.10.0.md.backup

# This is complex - recommend manual editing
# Focus on sections:
# - "Step 3: Verify Setup" (lines 77-95)
# - "New Features in v0.10.0" (lines 123-149)
# - All code examples throughout
```

**Verification**:
```bash
# Check for remaining old commands
grep -c "ie add\|ie start\|ie done" MIGRATION_v0.10.0.md
# Should be 0
```

---

## High Priority Fixes (P1) - Should Complete

### Fix 5: Format All Code

**Automatic Fix**:
```bash
cargo fmt
```

**Verification**:
```bash
cargo fmt --check
# Should output nothing (success)
```

---

### Fix 6: Fix PID Cleanup Test

**File**: `src/dashboard/pid.rs:257`

**Problem**: `test_cleanup_stale_pid` assertion fails

**Investigation needed**:
1. Check if test is flaky (timing issue)
2. Check if cleanup logic has a bug
3. Review recent changes to PID management

**Debug the test**:
```bash
# Run test with output
cargo test test_cleanup_stale_pid -- --nocapture --test-threads=1

# Run with backtrace
RUST_BACKTRACE=1 cargo test test_cleanup_stale_pid
```

**Temporary workaround** (not recommended):
```rust
// In src/dashboard/pid.rs
#[tokio::test]
#[ignore] // TODO: Fix flaky test
async fn test_cleanup_stale_pid() {
    // ...
}
```

**Proper fix**: Depends on investigation results. Likely issues:
- Race condition in test setup
- Platform-specific behavior (Unix vs Windows)
- Mock process not behaving as expected

---

### Fix 7: Update All Documentation

**Files to audit and update**:

1. **CLAUDE.md**
   ```bash
   grep -n "ie add\|ie start\|ie done\|ie task" CLAUDE.md
   # Update any references
   ```

2. **README.md**
   ```bash
   grep -n "ie add\|ie start\|ie done\|ie task" README.md
   # Update quick start examples
   ```

3. **docs/spec-03-interface-current.md**
   ```bash
   # Verify CLI command specification matches new structure
   grep -A 5 "^## CLI Commands" docs/spec-03-interface-current.md
   ```

4. **Language-specific docs**
   ```bash
   find docs/ -type f -name "*.md" -exec grep -l "ie add\|ie start\|ie done" {} \;
   # Update each file
   ```

5. **RELEASE_NOTES_v0.10.0.md**
   ```bash
   # Verify release notes mention CLI changes
   grep -i "breaking\|CLI\|command" RELEASE_NOTES_v0.10.0.md
   ```

---

## Verification Checklist

After applying all fixes, verify with this checklist:

```bash
# 1. Format check
echo "1. Checking format..."
cargo fmt --check || { echo "❌ Format check failed"; exit 1; }
echo "✅ Format OK"

# 2. Clippy check
echo "2. Running Clippy..."
cargo clippy --all-targets --all-features -- -D warnings || { echo "❌ Clippy failed"; exit 1; }
echo "✅ Clippy OK"

# 3. Unit tests
echo "3. Running unit tests..."
cargo test --lib || { echo "❌ Unit tests failed"; exit 1; }
echo "✅ Unit tests OK"

# 4. Integration tests
echo "4. Running integration tests..."
cargo test --test '*' || { echo "❌ Integration tests failed"; exit 1; }
echo "✅ Integration tests OK"

# 5. Build release
echo "5. Building release..."
cargo build --release || { echo "❌ Release build failed"; exit 1; }
echo "✅ Release build OK"

# 6. Manual CLI test
echo "6. Testing CLI commands..."
cargo run -- --version
cargo run -- --help
cargo run -- plan --help
cargo run -- log --help
cargo run -- search --help
echo "✅ CLI commands OK"

echo
echo "🎉 All checks passed! Ready for release."
```

---

## Post-Fix Actions

1. **Commit fixes**
   ```bash
   git add -A
   git commit -m "fix: resolve v0.10.0 release blockers

   - Fix Clippy errors (private interfaces)
   - Update interface_spec_test for new CLI
   - Fix formatting issues
   - Update migration guide
   - Update all documentation"
   ```

2. **Tag release**
   ```bash
   git tag -a v0.10.0 -m "Release v0.10.0"
   git push origin main --tags
   ```

3. **Verify CI**
   - Check GitHub Actions pass
   - Review test coverage report

4. **Manual smoke test**
   - Install from release
   - Run through quick start guide
   - Verify dashboard works

---

## Timeline Estimate

| Task | Estimated Time | Priority |
|------|----------------|----------|
| Fix Clippy errors | 5 minutes | P0 |
| Run cargo fmt | 1 minute | P0 |
| Update interface_spec_test | 15 minutes | P0 |
| Update MIGRATION_v0.10.0.md | 60 minutes | P0 |
| Fix PID cleanup test | 30 minutes | P1 |
| Update all docs | 60 minutes | P1 |
| **Total** | **2.5-3 hours** | |

---

## Contact

For questions or assistance:
- Review full test report: `PRE_RELEASE_TEST_REPORT.md`
- Check CI logs: GitHub Actions
- Run local tests: `cargo test --all`

---

*Last updated: 2025-12-16*
