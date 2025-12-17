# Version Placeholder System

## Overview

The version placeholder system automatically manages version numbers in documentation files, ensuring they always stay in sync with `Cargo.toml` and `INTERFACE_SPEC.md`.

## Placeholders

Use these placeholders in markdown documentation:

| Placeholder | Example | Description |
|-------------|---------|-------------|
| `{{VERSION}}` | `0.3.3` | Current release version from Cargo.toml |
| `{{VERSION_INTERFACE}}` | `0.3` | Interface version from INTERFACE_SPEC.md |
| `{{VERSION_RELEASE}}` | `0.3.3` | Same as {{VERSION}}, for legacy compatibility |
| `{{VERSION_PREVIOUS_MINOR}}` | `0.3.2` | Previous patch version (current - 0.0.1) |
| `{{DATE}}` | `2025-11-14` | Current date in YYYY-MM-DD format |

## Scripts

### `replace-version-placeholders.sh`

**Purpose**: Replace placeholders with actual version numbers (for releases and commits)

**Usage**:
```bash
./scripts/replace-version-placeholders.sh
```

**When it runs**:
- Automatically in pre-commit hook
- Manually before releases
- In CI/CD pipelines

**Example**:
```markdown
# Before
**Version**: {{VERSION}}
**Last Updated**: {{DATE}}

# After
**Version**: 0.3.3
**Last Updated**: 2025-11-14
```

### `restore-version-placeholders.sh`

**Purpose**: Restore placeholders in source files (for development)

**Usage**:
```bash
./scripts/restore-version-placeholders.sh
```

**When to use**:
- After reviewing generated documentation
- When preparing source files for version control
- When switching between branches with different versions

**Example**:
```markdown
# Before (committed with concrete versions)
**Version**: 0.3.3
**Last Updated**: 2025-11-14

# After (back to placeholders for next version)
**Version**: {{VERSION}}
**Last Updated**: {{DATE}}
```

## Workflow

### Development Workflow

1. **Write documentation with placeholders**:
   ```markdown
   This feature was added in v{{VERSION}}+.

   **Before (v{{VERSION_PREVIOUS_MINOR}} and earlier)**:
   ...

   **After (v{{VERSION}}+)**:
   ...
   ```

2. **Commit your changes**:
   ```bash
   git add docs/en/technical/my-feature.md
   git commit -m "Add documentation for my feature"
   ```

3. **Pre-commit hook automatically**:
   - Runs `replace-version-placeholders.sh`
   - Replaces all placeholders with actual versions
   - Adds modified documentation to the commit

4. **Result**: Documentation is committed with concrete version numbers

### Maintaining Source Files

If you want to keep placeholders in your working directory for easier editing:

```bash
# After commit, restore placeholders in working directory
./scripts/restore-version-placeholders.sh

# Now you can continue editing with placeholders
# They'll be replaced again on next commit
```

## Pre-commit Hook Integration

The pre-commit hook (`.git/hooks/pre-commit`) automatically:

1. Formats Rust code with `cargo fmt`
2. Checks version consistency
3. **Replaces version placeholders** ← New!
4. Adds modified documentation files to commit

### Hook Workflow

```
Developer commits
       ↓
cargo fmt (format code)
       ↓
check-version-sync.sh (verify consistency)
       ↓
replace-version-placeholders.sh (update docs)
       ↓
git add docs/*.md (stage changes)
       ↓
Commit proceeds with updated versions
```

## Files Using Placeholders

Currently, the following files use version placeholders:

- `docs/en/technical/cjk-search.md`
- `docs/zh-CN/technical/cjk-search.md`

To add placeholders to more files, simply use the placeholder syntax in your documentation. The scripts will automatically detect and process them.

## Best Practices

### ✅ DO

- Use `{{VERSION}}` for current version references
- Use `{{VERSION_PREVIOUS_MINOR}}` when comparing with previous version
- Use `{{DATE}}` for "Last Updated" fields
- Keep placeholders in source documentation
- Let the pre-commit hook handle replacements

### ❌ DON'T

- Don't hardcode version numbers in new documentation
- Don't use placeholders in code examples that show specific version commands
- Don't edit concrete version numbers in committed files (edit placeholders instead)

### When NOT to Use Placeholders

Some version references should remain hardcoded:

1. **Tutorial examples**: `git tag v0.1.4` (teaching specific commands)
2. **Historical references**: "In v0.1.10, we introduced..." (past events)
3. **Migration guides**: Specific version upgrade paths
4. **Change logs**: Version-specific release notes

## Troubleshooting

### Placeholders not being replaced

**Check**:
```bash
# Verify scripts are executable
ls -l scripts/*.sh

# Test replacement manually
./scripts/replace-version-placeholders.sh
```

### Wrong version extracted

**Check**:
```bash
# Verify Cargo.toml version
grep '^version' Cargo.toml

# Verify INTERFACE_SPEC.md version
head -10 docs/INTERFACE_SPEC.md | grep 'Version'
```

### Placeholders in wrong files

The restore script only processes specific files (e.g., `*/technical/cjk-search.md`). To add more files, edit `restore-version-placeholders.sh` and add patterns to the `case` statement.

## Technical Details

### Placeholder Detection

The scripts use `grep` to find placeholders:
```bash
grep -q "{{VERSION}}\|{{VERSION_PREVIOUS_MINOR}}\|{{DATE}}" "$file"
```

### Replacement Logic

Uses `sed` for in-place replacement:
```bash
sed -i \
  -e "s/{{VERSION}}/$CARGO_VERSION/g" \
  -e "s/{{VERSION_PREVIOUS_MINOR}}/$VERSION_PREVIOUS_MINOR/g" \
  -e "s/{{DATE}}/$DATE/g" \
  "$file"
```

### Version Calculation

Previous minor version is calculated from current version:
```bash
IFS='.' read -r major minor patch <<< "$CARGO_VERSION"
VERSION_PREVIOUS_MINOR="$major.$minor.$((patch - 1))"
```

## Related Scripts

- `check-version-sync.sh` - Verifies version consistency across files
- `.git/hooks/pre-commit` - Orchestrates all pre-commit checks

---

**Last Updated**: 2025-11-14
**Status**: Active
