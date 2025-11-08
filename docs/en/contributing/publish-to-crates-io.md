# Publishing to crates.io Solution

## 🔍 Problem Analysis

**Current Status:**
- ✅ v0.1.4 tag exists locally
- ❌ v0.1.4 tag does **not** exist remotely (push blocked by 403)
- ❌ GitHub Actions release workflow not triggered
- ❌ Therefore not published to crates.io

## 🎯 Recommended Solution: Merge PR First, Then Create Tag from main

### Step 1: Create and Merge PR

1. Visit: https://github.com/wayfind/intent-engine/compare/main...claude/improve-installation-experience-011CUv3p6NQmi6Xd5EKqJE1r
2. Create PR (using content from PR_DESCRIPTION.md)
3. Wait for CI to pass
4. Merge PR

### Step 2: Create Tag from main Branch

After PR is merged, create tag from main branch (permissions may differ):

```bash
# 1. Switch to main and pull latest code
git checkout main
git pull origin main

# 2. Confirm version is 0.1.4
grep "^version" Cargo.toml

# 3. Create tag
git tag v0.1.4

# 4. Push tag
git push origin v0.1.4
```

If pushing tag from main still encounters 403, continue to Solution 2.

---

## 🎯 Solution 2: Create Release via GitHub Web UI (Recommended)

This method can bypass git push permission issues:

### Steps:

1. **Ensure PR is merged to main**

2. **Visit GitHub Releases page**:
   https://github.com/wayfind/intent-engine/releases/new

3. **Fill in form**:
   - **Choose a tag**: Enter `v0.1.4` and select "Create new tag: v0.1.4 on publish"
   - **Target**: Select `main` branch
   - **Release title**: `v0.1.4`
   - **Description**: Can use auto-generate, or manually fill in:

   ```markdown
   ## 🚀 v0.1.4 - Improved Installation Experience

   This version significantly improves installation experience, supporting multiple package managers and installation methods.

   ### ✨ New Features

   - ✅ **cargo install** support - Can now install directly from crates.io
   - ✅ **Homebrew** support - Provides formula and auto-update script
   - ✅ **cargo-binstall** support - Quick install of pre-compiled binaries
   - ✅ Complete installation documentation and testing guides

   ### 📦 Installation Methods

   ```bash
   # Install from crates.io (recommended)
   cargo install intent-engine

   # Use cargo-binstall
   cargo binstall intent-engine

   # Or download pre-compiled binary
   # See Assets below
   ```

   ### 📚 Documentation

   - Complete Installation Guide: [INSTALLATION.md](https://github.com/wayfind/intent-engine/blob/main/INSTALLATION.md)
   - Release Testing Guide: [docs/HOW_TO_TEST_RELEASE.md](https://github.com/wayfind/intent-engine/blob/main/docs/HOW_TO_TEST_RELEASE.md)
   ```

4. **Publish**:
   - Click "Publish release"
   - This will automatically trigger release workflow

### This Will Automatically:
- ✅ Create v0.1.4 tag
- ✅ Build binaries for all platforms
- ✅ Upload binaries to Release
- ✅ Publish to crates.io

---

## 🎯 Solution 3: Manually Publish to crates.io (Temporary Solution)

If immediate publication to crates.io is needed, you can do it manually:

```bash
# 1. Ensure on correct commit (version 0.1.4)
git log --oneline -1
# Should see: 83371e3 Bump version to 0.1.4

# 2. Login to crates.io
cargo login
# Enter your crates.io API token

# 3. Publish
cargo publish

# 4. Verify
cargo search intent-engine
```

**Pros:** Immediate publication
**Cons:** GitHub Release and binaries need to be created separately

---

## 🎯 Solution 4: Use workflow_dispatch for Manual Trigger

If release workflow supports manual triggering (requires adding configuration):

```yaml
# Add in .github/workflows/release.yml
on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:  # Add this
    inputs:
      version:
        description: 'Version to release (e.g., 0.1.4)'
        required: true
```

Then you can manually trigger on GitHub Actions page.

---

## 📋 Recommended Execution Order

### Best Practice Workflow:

1. ✅ **Create and Merge PR**
   - Let code pass CI validation
   - Ensure code is on main branch

2. ✅ **Create Release via Web UI** (Solution 2)
   - Most reliable method
   - Automatically triggers all processes
   - No need to handle permission issues

3. ✅ **Verify Release**:
   ```bash
   # Verify after a few minutes
   cargo search intent-engine
   cargo install intent-engine
   intent-engine --version
   ```

---

## 🚨 Troubleshooting

### If crates.io Publish Fails:

1. **Check GitHub Actions logs**:
   https://github.com/wayfind/intent-engine/actions

2. **View publish-crates-io job output**:
   ```
   Possible errors:
   - "error: failed to authenticate" → Token invalid
   - "error: crate version already exists" → Version conflict
   - "error: not allowed to upload" → Permission issue
   ```

3. **Verify Secret settings**:
   https://github.com/wayfind/intent-engine/settings/secrets/actions
   - Confirm `CARGO_REGISTRY_TOKEN` exists
   - Regenerate token if needed

### If Re-release is Needed:

```bash
# 1. Delete local tag
git tag -d v0.1.4

# 2. Delete remote tag (if exists)
git push origin :refs/tags/v0.1.4

# 3. Recreate Release (via Web UI)
```

---

## 📊 Checklist

Before release, confirm:

- [ ] PR merged to main
- [ ] Cargo.toml version is 0.1.4
- [ ] `CARGO_REGISTRY_TOKEN` secret is set
- [ ] Selected release method (recommend Solution 2)
- [ ] Release description prepared

---

## 🎯 Take Action Now

**Do now:**
1. Create PR: https://github.com/wayfind/intent-engine/compare/main...claude/improve-installation-experience-011CUv3p6NQmi6Xd5EKqJE1r
2. Wait for merge
3. Create Release via Web UI: https://github.com/wayfind/intent-engine/releases/new
