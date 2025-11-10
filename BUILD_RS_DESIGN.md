# build.rs Git Hooks Auto-Installation Design

## Overview

This document explains the design and implementation of the automatic git hooks installation system using `build.rs`.

## Goals

1. **Zero-config experience**: Developers get formatting hooks automatically
2. **Robustness**: Handle all edge cases gracefully
3. **Idempotence**: Safe to run multiple times without side effects
4. **Performance**: Minimal overhead, only run when necessary
5. **Transparency**: Clear messaging when actions are taken

## Architecture

### Primary Mechanism: build.rs

**File**: `build.rs` at project root

**Trigger**: Runs automatically during any cargo build command
- `cargo build`
- `cargo test`
- `cargo run`
- `cargo check` (in some cases)

**Flow**:
```
┌─────────────────────────────────────────┐
│ cargo build / test / run                │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│ build.rs: main()                        │
│ 1. Check rerun conditions               │
│    - build.rs changed?                  │
│    - auto-setup-hooks.sh changed?       │
│    - pre-commit hook deleted?           │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│ Should skip?                            │
│ - CI environment? (skip)                │
│ - Release build? (skip)                 │
│ - SKIP_GIT_HOOKS_SETUP set? (skip)      │
│ - Not a git repo? (skip)                │
└──────────────┬──────────────────────────┘
               │ No (continue)
               ▼
┌─────────────────────────────────────────┐
│ Hooks already installed?                │
│ Check: .git/hooks/pre-commit contains   │
│        "cargo fmt"                      │
└──────────────┬──────────────────────────┘
               │
         ┌─────┴─────┐
         │           │
       Yes          No
         │           │
         ▼           ▼
   ┌─────────┐  ┌────────────────┐
   │ Silent  │  │ Install hooks  │
   │  skip   │  │ Show messages  │
   └─────────┘  └────────────────┘
```

### Backup Mechanism: SessionStart

**File**: `.claude-code/SessionStart`

**Trigger**: Claude Code session initialization

**Purpose**: Backup for edge cases where repo is used without building

**Behavior**: Runs `auto-setup-hooks.sh` silently (output suppressed)

## Implementation Details

### Key Functions

#### `should_skip_hooks_setup()`
Determines if hooks installation should be skipped based on:
- CI environment detection (GitHub Actions, GitLab CI, CircleCI, Travis, etc.)
- Build profile (release vs dev)
- `SKIP_GIT_HOOKS_SETUP` environment variable

#### `get_marker_file()`
Returns path to `target/.git-hooks-installed` marker file.
- Creates `target/` directory if it doesn't exist
- Returns `None` if directory creation fails

#### `hooks_already_installed()`
Checks if `.git/hooks/pre-commit` exists and contains `"cargo fmt"`.

#### `create_marker_file(marker_path)`
Creates marker file with timestamp for tracking.

#### `install_git_hooks(marker_path)`
- Locates `scripts/auto-setup-hooks.sh`
- Finds bash executable (tries multiple locations)
- Executes setup script
- Creates marker file on success
- Shows warning messages on failure

#### `find_bash_command()`
Tries to find bash in common locations:
1. `bash` (in PATH)
2. `/bin/bash` (standard Unix)
3. `/usr/bin/bash` (alternative Unix)
4. `sh` (fallback)

### Marker File Strategy

**Purpose**: Track installation to avoid repeated messages

**Location**: `target/.git-hooks-installed`

**Content**: Timestamp for debugging

**Lifetime**: Deleted by `cargo clean`, triggering reinstallation on next build

### Cargo Rerun Conditions

```rust
println!("cargo:rerun-if-changed=build.rs");
println!("cargo:rerun-if-changed=scripts/auto-setup-hooks.sh");
println!("cargo:rerun-if-changed=.git/hooks/pre-commit");
```

**Effect**: build.rs only reruns when:
- build.rs itself is modified
- The setup script is modified
- The pre-commit hook is deleted/modified

This prevents unnecessary execution on every build.

## Edge Cases Handled

### 1. Missing target/ Directory
**Scenario**: First build after fresh clone
**Solution**: `get_marker_file()` creates directory before creating marker

### 2. Concurrent Builds
**Scenario**: Multiple cargo commands running simultaneously
**Solution**: Script handles existing hooks gracefully (idempotent)

### 3. Deleted Hooks
**Scenario**: User accidentally deletes `.git/hooks/pre-commit`
**Solution**: `rerun-if-changed` triggers rebuild, hooks reinstalled

### 4. CI Environments
**Scenario**: Building in CI where git hooks aren't needed
**Solution**: Detect CI environment and skip installation

### 5. Release Builds
**Scenario**: Building release artifacts
**Solution**: Skip installation in release profile

### 6. No Bash Available
**Scenario**: Windows systems without bash
**Solution**: Try multiple shell locations, graceful failure with helpful message

### 7. Permission Issues
**Scenario**: Cannot write to `.git/hooks/` or `target/`
**Solution**: Catch errors, log warnings, don't fail build

## Interaction with SessionStart

**Relationship**: Complementary, not conflicting

- **build.rs**: Primary mechanism (runs on first build)
- **SessionStart**: Backup mechanism (runs on Claude Code startup)

**Coordination**:
1. Both call the same script: `scripts/auto-setup-hooks.sh`
2. Script is idempotent (checks if hooks exist before installing)
3. SessionStart now runs silently to avoid noise

**Why keep both?**:
- build.rs: Covers all developers who run `cargo build`
- SessionStart: Covers edge case where someone uses repo without building

## User Experience

### Fresh Clone Scenario

```bash
$ git clone https://github.com/user/intent-engine
$ cd intent-engine
$ cargo build

   Compiling intent-engine v0.1.13
warning: 🔧 Setting up git pre-commit hooks for auto-formatting...
warning: ✅ Git hooks configured! Commits will be auto-formatted.
    Finished dev [unoptimized + debuginfo] target(s) in 4.2s
```

### Subsequent Builds

```bash
$ cargo build
   Compiling intent-engine v0.1.13
    Finished dev [unoptimized + debuginfo] target(s) in 3.5s
```
**Note**: No hook-related messages (silent)

### After `cargo clean`

```bash
$ cargo clean
$ cargo build

   Compiling intent-engine v0.1.13
warning: 🔧 Setting up git pre-commit hooks for auto-formatting...
warning: ✅ Git hooks configured! Commits will be auto-formatted.
    Finished dev [unoptimized + debuginfo] target(s) in 4.1s
```
**Note**: Reinstalls because marker file was deleted

### Committing Code

```bash
$ git commit -m "feat: add feature"
Running cargo fmt...
✓ Code formatted. Adding changes to commit...
✓ Pre-commit hook completed
[main abc123] feat: add feature
 1 file changed, 10 insertions(+)
```

## Disabling Automatic Installation

### Temporary (current session)
```bash
export SKIP_GIT_HOOKS_SETUP=1
cargo build
```

### Permanent (for a user)
Add to shell profile (`~/.bashrc`, `~/.zshrc`, etc.):
```bash
export SKIP_GIT_HOOKS_SETUP=1
```

### Remove Installed Hooks
```bash
rm .git/hooks/pre-commit
```

## Testing

### Manual Tests

```bash
# Test 1: Fresh installation
rm -f .git/hooks/pre-commit target/.git-hooks-installed
cargo build  # Should show installation messages

# Test 2: Idempotence
cargo build  # Should be silent (no messages)

# Test 3: Reinstallation after deletion
rm .git/hooks/pre-commit
touch build.rs  # Trigger rerun
cargo build  # Should reinstall

# Test 4: Bypass
export SKIP_GIT_HOOKS_SETUP=1
rm .git/hooks/pre-commit target/.git-hooks-installed
cargo build  # Should not install
unset SKIP_GIT_HOOKS_SETUP

# Test 5: CI simulation
export CI=true
rm .git/hooks/pre-commit target/.git-hooks-installed
cargo build  # Should skip installation
unset CI
```

### Automated Tests

Tests are integrated into the existing test suite and run in CI to verify:
- Installation works on all platforms
- Hooks function correctly
- Edge cases are handled

## Security Considerations

### Trust Model
- Scripts are part of the repository (version controlled)
- Users clone and build, implicitly trusting repo content
- No external downloads or dependencies

### Integrity
- Git ensures script integrity (SHA-verified commits)
- Code review process for changes to `build.rs` and setup scripts
- Hooks can be reviewed before building

### Opt-out Available
Users can disable if they don't trust the automation:
```bash
export SKIP_GIT_HOOKS_SETUP=1
```

## Future Enhancements

### Potential Improvements

1. **Cross-platform shell script**: Replace bash with Rust for portability
2. **Hook versioning**: Detect and update outdated hooks
3. **Multiple hooks**: Support additional git hooks (pre-push, commit-msg, etc.)
4. **Configuration file**: Allow users to customize hook behavior
5. **Telemetry**: Optional anonymous metrics on usage patterns

### Not Planned

- **Automatic push hooks**: Too intrusive, should be explicit
- **Network dependencies**: Keep offline-first approach
- **Binary hooks**: Stick with shell scripts for transparency

## Comparison with Alternatives

### vs. Manual Installation
**Manual**: Requires developers to remember and run setup script
**build.rs**: Automatic, zero-config, impossible to forget

### vs. Git Template
**Template**: Requires one-time git config setup
**build.rs**: No configuration, works out of the box

### vs. Husky (npm ecosystem)
**Husky**: Requires npm/package.json
**build.rs**: Native to Rust/Cargo, no external dependencies

### vs. Pre-commit Framework
**pre-commit**: Separate tool, requires Python
**build.rs**: Built-in, no additional installations

## Maintenance

### When to Update build.rs

- Adding new CI platforms to detect
- Supporting new shell types
- Changing hook content or behavior
- Fixing bugs in edge case handling

### When to Update Documentation

- Changing user-visible behavior
- Adding/removing environment variables
- Modifying skip conditions
- Updating error messages

## Conclusion

The `build.rs` approach provides:
- ✅ **Automatic**: No manual steps required
- ✅ **Robust**: Handles edge cases gracefully
- ✅ **Transparent**: Clear messaging when active
- ✅ **Performant**: Minimal overhead
- ✅ **Maintainable**: Simple, well-documented code

This design ensures that all developers, regardless of their editor or workflow, get consistent code formatting enforcement without manual setup.
