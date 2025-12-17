# Git Hooks for Intent-Engine

This directory contains git hooks templates that ensure code quality and consistency across the project.

## Installation

To install the git hooks, run:

```bash
./scripts/install-git-hooks.sh
```

This will copy the hooks to your `.git/hooks/` directory and make them executable.

## Available Hooks

### pre-commit

The pre-commit hook runs automatically before each commit and performs the following checks:

1. **Code Formatting** (`cargo fmt --all`)
   - Automatically formats all Rust code
   - Stages formatted files for commit

2. **Clippy Linting** (`cargo clippy -- -D warnings`)
   - Runs Clippy with all warnings treated as errors
   - Ensures code follows Rust best practices
   - **Aborts commit if warnings are found**

3. **Version Consistency Check** (`./scripts/check-version-sync.sh`)
   - Verifies Cargo.toml and INTERFACE_SPEC.md versions match
   - **Aborts commit if versions are inconsistent**

4. **Version Placeholder Replacement** (`./scripts/replace-version-placeholders.sh`)
   - Replaces placeholders like {{VERSION}} with actual version numbers
   - Automatically stages updated documentation

## What Gets Checked

### Code Quality
- ✅ All code is properly formatted (rustfmt)
- ✅ No clippy warnings (including pedantic checks)
- ✅ All tests pass (run manually with `cargo test`)

### Version Management
- ✅ Cargo.toml version matches INTERFACE_SPEC.md
- ✅ Documentation version placeholders are replaced
- ✅ Date placeholders are updated

### Documentation
- ✅ Version numbers are consistent across all docs
- ✅ Documentation is automatically staged when modified

## Hook Execution Flow

```
Developer runs: git commit
         ↓
┌────────────────────────────────┐
│ 1. cargo fmt --all             │ ← Format code
└────────────────────────────────┘
         ↓
┌────────────────────────────────┐
│ 2. git add *.rs                │ ← Stage formatted files
└────────────────────────────────┘
         ↓
┌────────────────────────────────┐
│ 3. cargo clippy -D warnings    │ ← Lint code
│    ❌ ABORT if warnings found  │
└────────────────────────────────┘
         ↓
┌────────────────────────────────┐
│ 4. check-version-sync.sh       │ ← Verify versions
│    ❌ ABORT if mismatch         │
└────────────────────────────────┘
         ↓
┌────────────────────────────────┐
│ 5. replace-version-placeholders│ ← Update docs
└────────────────────────────────┘
         ↓
┌────────────────────────────────┐
│ 6. git add docs/*.md           │ ← Stage updated docs
└────────────────────────────────┘
         ↓
    Commit proceeds ✅
```

## Bypassing Hooks (Not Recommended)

In rare cases, you may need to bypass the hooks:

```bash
git commit --no-verify -m "Emergency fix"
```

**⚠️ Warning**: This skips all quality checks. Only use in emergencies.

## Troubleshooting

### Hook doesn't run

**Check if installed**:
```bash
ls -la .git/hooks/pre-commit
```

**Reinstall**:
```bash
./scripts/install-git-hooks.sh
```

### Clippy warnings fail commit

**View warnings**:
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

**Fix warnings and try again**:
```bash
# Fix the issues, then
git add .
git commit -m "Your message"
```

### Version sync fails

**Check versions**:
```bash
./scripts/check-version-sync.sh
```

**Fix version mismatch**:
- Update `Cargo.toml` version
- Update `INTERFACE_SPEC.md` version
- Ensure they match

### Hook takes too long

The clippy check can take a few seconds. This is normal and ensures code quality. If it's too slow:

1. Ensure you have a release build of clippy: `rustup component add clippy`
2. Consider using `cargo clippy --cached` (if available)
3. Use `git commit --no-verify` only for emergency commits

## Updating Hooks

If the hook templates are updated in the repository:

1. Pull the latest changes
2. Reinstall hooks: `./scripts/install-git-hooks.sh`

## Contributing

When modifying hooks:

1. Edit `scripts/git-hooks/pre-commit` (the template)
2. Test thoroughly
3. Document changes in this README
4. Commit the template to version control
5. Notify team to reinstall hooks

## Related Documentation

- [Version Placeholders](../VERSION_PLACEHOLDERS.md) - How version placeholders work
- [check-version-sync.sh](../check-version-sync.sh) - Version consistency checker
- [replace-version-placeholders.sh](../replace-version-placeholders.sh) - Version updater

---

**Last Updated**: 2025-11-14
**Status**: Active
