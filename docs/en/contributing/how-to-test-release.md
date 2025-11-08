# How to Test crates.io Automatic Publishing

## 📋 Test Results Summary

✅ **Local Validation Completed:**
- ✅ Git working tree clean
- ✅ Packaging successful (58 files)
- ✅ Dry-run publish test passed
- ✅ Package ready to publish to crates.io

## 🎯 Three Testing Methods Available

---

### Method 1: Use Test Script (Recommended)

Run the automated test script:

```bash
./scripts/test-release.sh
```

**This script will check:**
1. Git working tree status
2. Current version number
3. Local packaging
4. Dry-run publish
5. GitHub Secret (if gh CLI is installed)
6. Provide next step guidance

---

### Method 2: Verify GitHub Secret

#### Option A: Use gh CLI (Recommended)

```bash
# Install gh CLI (if not already installed)
# macOS: brew install gh
# Linux: see https://github.com/cli/cli#installation

# Login to GitHub
gh auth login

# View secrets
gh secret list

# Should see:
# CARGO_REGISTRY_TOKEN  Updated YYYY-MM-DD
```

#### Option B: Manual Verification in Web UI

1. Visit: https://github.com/wayfind/intent-engine/settings/secrets/actions
2. Check if `CARGO_REGISTRY_TOKEN` exists
3. Confirm the set time is correct

#### Option C: Verify Token Validity (Optional)

If you want to ensure the token is valid:

```bash
# Test login locally (requires real token)
cargo login

# Or test query permissions
cargo owner --list intent-engine
```

---

### Method 3: Simulate Complete Release Process (Not Recommended for First Time)

⚠️ **Note: This will create a real GitHub Release, but won't publish to crates.io (because version already exists)**

```bash
# 1. View current branch and tags
git branch
git tag -l

# 2. Create a test tag (using existing version)
git tag v0.1.3-test

# 3. Push tag (this will trigger workflow)
git push origin v0.1.3-test

# 4. View Actions immediately
# Method A: Use gh CLI
gh run list --workflow=release.yml --limit 5

# Method B: Visit Web UI
# https://github.com/wayfind/intent-engine/actions

# 5. View live logs (using gh CLI)
gh run watch

# 6. Delete test tag after testing
git tag -d v0.1.3-test
git push origin :refs/tags/v0.1.3-test
```

---

## 🚀 Real Release Process (Production)

When you're ready to release a new version:

### Step 1: Update Version Number

```bash
# Edit Cargo.toml
vim Cargo.toml
# Modify: version = "0.1.4"

# Commit changes
git add Cargo.toml
git commit -m "Bump version to 0.1.4"
git push
```

### Step 2: Create and Push Tag

```bash
# Create tag
git tag v0.1.4

# Push tag (this will trigger automatic release)
git push origin v0.1.4
```

### Step 3: Monitor Release Process

```bash
# View in real-time using gh CLI
gh run watch

# Or visit Web UI
# https://github.com/wayfind/intent-engine/actions
```

**Expected Steps:**

1. ✅ **Build** - Build binaries for all platforms
   - Linux x86_64, ARM64
   - macOS x86_64, ARM64
   - Windows x86_64

2. ✅ **Create Release** - Create GitHub Release
   - Upload all binary files
   - Generate release notes

3. ✅ **Publish to crates.io** - Publish to crates.io
   - Login using CARGO_REGISTRY_TOKEN
   - Execute `cargo publish`

### Step 4: Verify Release Success

```bash
# 1. Check crates.io
cargo search intent-engine --limit 1

# Should see new version:
# intent-engine = "0.1.4"    # A command-line database service...

# 2. Test installation
cargo install intent-engine --force

# 3. Verify version
intent-engine --version
# Should output: intent-engine 0.1.4

# 4. Check GitHub Release
# https://github.com/wayfind/intent-engine/releases
```

### Step 5: Follow-up Actions

```bash
# 1. Update Homebrew formula
./scripts/update-homebrew-formula.sh 0.1.4

# 2. Test cargo-binstall
cargo binstall intent-engine --force

# 3. Publish announcement (optional)
# - Publish in GitHub Discussions
# - Share on social media
# - Update documentation
```

---

## 🔍 Monitoring and Debugging

### View Workflow Run History

```bash
# List recent runs
gh run list --workflow=release.yml --limit 10

# View specific run details
gh run view <run-id>

# View complete logs
gh run view <run-id> --log

# Download logs
gh run download <run-id>
```

### Common Issue Troubleshooting

#### 1. Workflow Not Triggered

**Check:**
```bash
# Confirm tag format is correct (must start with v)
git tag -l

# Confirm tag is pushed to remote
git ls-remote --tags origin

# Confirm workflow file is on correct branch
git show origin/main:.github/workflows/release.yml
```

#### 2. crates.io Publish Failed

**Check logs for errors:**
```bash
gh run view --log | grep -A 10 "Publish to crates.io"
```

**Possible Causes:**
- ❌ Token invalid or expired → Regenerate and update Secret
- ❌ Version number already exists → Use new version number
- ❌ No publish permission → Check token permissions
- ❌ Package name already taken → Change package name (unlikely)

#### 3. Build Failed

**View build logs:**
```bash
gh run view --log | grep -A 20 "error:"
```

---

## 📊 Release Checklist

Before official release, ensure:

- [ ] All tests pass (`cargo test`)
- [ ] Dry-run successful (`cargo publish --dry-run`)
- [ ] Git working tree clean
- [ ] Version number updated and follows semantic versioning
- [ ] CHANGELOG.md updated (if exists)
- [ ] GitHub Secret `CARGO_REGISTRY_TOKEN` set
- [ ] Documentation updated
- [ ] CI passes on main branch

---

## 🎉 Success Indicators

After successful release, you should see:

✅ **GitHub Actions:**
- All steps are green ✓
- No errors or warnings

✅ **GitHub Releases:**
- New release appears at https://github.com/wayfind/intent-engine/releases
- All platform binaries uploaded

✅ **crates.io:**
- New version appears at https://crates.io/crates/intent-engine
- Can be found via `cargo search`
- Can be installed via `cargo install`

✅ **cargo-binstall:**
- Can be installed via `cargo binstall`

---

## 📚 Related Documentation

- [TESTING_RELEASE.md](TESTING_RELEASE.md) - Detailed testing guide
- [INSTALLATION.md](../guide/installation.md) - Complete installation guide
- [README.md](../../../README.en.md) - Main project documentation

---

## 💡 Tips

1. **For first release**, recommend publishing a small patch version first to test the entire process
2. **Use semantic versioning**: major.minor.patch
   - patch (0.1.3 → 0.1.4): Bug fixes
   - minor (0.1.4 → 0.2.0): New features, backward compatible
   - major (0.2.0 → 1.0.0): Breaking changes
3. **Before release** run `./scripts/test-release.sh` to ensure everything is ready
4. **Monitor Actions** to catch issues early
5. **Verify installation** to ensure users can use normally

Happy releasing! 🚀
