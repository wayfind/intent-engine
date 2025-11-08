# Testing crates.io Release Process Guide

## Method 1: Local Dry-run Testing (Recommended First)

Before actual release, test if packaging works locally:

```bash
# 1. Check if Cargo.toml configuration is correct
cargo package --list

# 2. Test packaging (won't actually publish)
cargo package --allow-dirty

# 3. Verify packaged content
tar -tzf target/package/intent-engine-0.1.3.crate

# 4. Dry-run publish (won't actually publish to crates.io)
cargo publish --dry-run --allow-dirty
```

**Expected Results:**
- ✅ All files correctly included
- ✅ No errors or warnings
- ✅ crates.io validation passed

---

## Method 2: Verify GitHub Secret is Correctly Set

```bash
# View current repository's secrets (requires gh CLI)
gh secret list

# Or view in GitHub Web UI
# https://github.com/wayfind/intent-engine/settings/secrets/actions
```

**Expected to See:**
```
CARGO_REGISTRY_TOKEN  Updated YYYY-MM-DD
```

---

## Method 3: Manually Trigger Test Release (Not Recommended for First Time)

⚠️ **Warning: This will actually publish to crates.io and cannot be undone!**

### Option A: Publish Patch Version (Safe Test)

```bash
# 1. Create a test patch version
# Edit Cargo.toml, change version to 0.1.4-test or 0.1.4

# 2. Commit changes
git add Cargo.toml
git commit -m "Bump version to 0.1.4 for testing"
git push

# 3. Create tag to trigger release
git tag v0.1.4
git push origin v0.1.4
```

### Option B: Use workflow_dispatch (If Enabled)

If your workflow supports manual triggering:

```bash
# Use gh CLI
gh workflow run release.yml
```

---

## Method 4: Monitor Automatic Release Process

After pushing a tag, you can view workflow execution in real-time:

```bash
# View recent workflow runs
gh run list --workflow=release.yml

# View specific run logs
gh run view <run-id> --log

# Or view in Web UI
# https://github.com/wayfind/intent-engine/actions
```

**Key Steps to Check:**
1. ✅ Build all platforms
2. ✅ Create Release (create GitHub Release)
3. ✅ Publish to crates.io (publish to crates.io)

---

## Method 5: Verify Release Success

After release completes, verify:

### Check crates.io
```bash
# Search for your package
cargo search intent-engine --limit 1

# Or visit Web
# https://crates.io/crates/intent-engine
```

### Test Installation
```bash
# Install from crates.io
cargo install intent-engine

# Verify version
intent-engine --version
```

### Check GitHub Release
Visit: https://github.com/wayfind/intent-engine/releases

---

## Recommended Complete Testing Process

### Stage 1: Local Validation (Safe)
```bash
# 1. Dry-run test
cargo publish --dry-run --allow-dirty

# 2. Check output, ensure no errors
# If issues found, fix and retest
```

### Stage 2: Secret Validation
```bash
# Check if secret is set
gh secret list | grep CARGO_REGISTRY_TOKEN
```

### Stage 3: Small Version Test (Actual Release)
```bash
# 1. Ensure current branch is clean
git status

# 2. Update version number (e.g., 0.1.3 -> 0.1.4)
# Edit Cargo.toml: version = "0.1.4"

# 3. Commit and tag
git add Cargo.toml
git commit -m "Bump version to 0.1.4"
git push
git tag v0.1.4
git push origin v0.1.4

# 4. Observe GitHub Actions
# Visit https://github.com/wayfind/intent-engine/actions

# 5. Wait for completion, then verify
cargo search intent-engine
```

---

## Common Issue Troubleshooting

### 1. crates.io Login Failed
```
error: failed to parse registry response
```
**Cause:** Token invalid or expired
**Solution:** Regenerate token and update GitHub Secret

### 2. Publish Permission Error
```
error: not allowed to upload
```
**Cause:** Token doesn't have publish permission
**Solution:** Ensure token has "Publish new crates" permission

### 3. Version Conflict
```
error: crate version `0.1.3` is already uploaded
```
**Cause:** Version number already exists
**Solution:** Use new version number

### 4. Workflow Not Triggered
**Check:**
- Is tag format correct (must be `v*`)
- Is workflow file on main branch
- Is GitHub Actions enabled

---

## Debugging Commands

```bash
# View local tags
git tag -l

# View remote tags
git ls-remote --tags origin

# View workflow status
gh run list --workflow=release.yml --limit 5

# View workflow detailed logs
gh run view --log

# View recent commits
git log --oneline -5

# Verify Cargo.toml
cargo verify-project
```

---

## Manual Rollback (If Needed)

⚠️ **Note: Versions on crates.io cannot be deleted, only yanked**

```bash
# Yank a problematic version (not recommended for use)
cargo yank --vers 0.1.4 intent-engine

# Unyank
cargo yank --vers 0.1.4 --undo intent-engine
```

---

## Success Indicators

✅ **Signs of Successful Release:**
1. All GitHub Actions steps are green ✓
2. New release appears on GitHub Releases page
3. `cargo search intent-engine` finds new version
4. crates.io page shows new version
5. `cargo install intent-engine` installs successfully

---

## Next Steps

After successful release:
1. Update Homebrew formula (run `./scripts/update-homebrew-formula.sh 0.1.4`)
2. Remove "Coming soon" marker from Homebrew in README
3. Notify users of new version release
