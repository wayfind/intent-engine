# CI/CD System Documentation

## 🎯 Overview

Intent-Engine uses a modern, efficient CI/CD system designed for fast feedback and comprehensive quality checks.

### Design Principles

1. **Fast Feedback**: PR checks complete in <5 minutes
2. **Comprehensive Coverage**: Full platform testing on main branch
3. **Automation First**: Minimal manual intervention required
4. **Clear Separation**: Different workflows for different purposes

---

## 📁 Workflow Structure

```
.github/workflows/
├── _build-release.yml      # [Reusable] Cross-platform release builds
├── _setup-rust.yml         # [Reusable] Rust setup and caching
├── _test-suite.yml         # [Reusable] Test execution
│
├── ci.yml                  # Main CI pipeline
├── release.yml             # Release automation
├── version-bump.yml        # Version management
├── codecov.yml             # Code coverage
├── manual-build.yml        # Manual debugging builds
├── labeler.yml             # Auto PR labeling
├── stale.yml               # Stale issue management
├── changelog.yml           # CHANGELOG generation
├── validate-translations.yml  # Translation validation
└── release-pr.yml          # Release PR checks
```

---

## 🔄 CI Pipeline

### Pull Request Checks (Fast ~3-5 min)

When you create a PR, these checks run automatically:

```yaml
✓ Format Check       (cargo fmt)
✓ Clippy Lints       (cargo clippy)
✓ Quick Tests        (Ubuntu/stable)
✓ Documentation      (cargo doc)
✓ Dependency Review
✓ Auto Labeling
```

**Branch Protection**: All PRs must pass these checks before merging.

### Main Branch (Full Suite ~15-20 min)

After merging to main:

```yaml
✓ Cross-Platform Tests
  ├── Linux (stable, beta)
  ├── macOS (stable)
  ├── Windows (stable)
  └── Linux nightly (experimental)

✓ Package Verification
✓ Code Coverage Upload
```

### Daily Scheduled (Security)

Every day at 10:00 UTC:

```yaml
✓ Security Audit      (cargo audit, cargo deny)
✓ Outdated Dependencies (cargo outdated)
```

If any check fails, an issue is automatically created.

---

## 🚀 Release Process

### Option 1: Automated (Recommended)

1. **Trigger Version Bump Workflow**
   - Go to Actions → Version Bump
   - Select bump type (patch/minor/major)
   - Choose whether to create tag immediately

2. **Review and Merge** (if PR created)
   - Review the version bump PR
   - Merge when ready

3. **Create Tag** (if not auto-created)
   ```bash
   git tag -a v0.1.10 -m "Release v0.1.10"
   git push origin v0.1.10
   ```

4. **Automatic Release**
   - Builds binaries for 5 platforms
   - Creates GitHub release
   - Publishes to crates.io

### Option 2: Local Script

```bash
# Install cargo-edit if needed
cargo install cargo-edit

# Run release script
./scripts/release.sh patch           # 0.1.9 → 0.1.10
./scripts/release.sh minor           # 0.1.9 → 0.2.0
./scripts/release.sh major           # 0.1.9 → 1.0.0

# Auto-commit and push
./scripts/release.sh patch --auto
```

### What Gets Updated

- ✅ `Cargo.toml` version
- ✅ `Cargo.lock`
- ✅ `CLAUDE.md` version
- ✅ `docs/INTERFACE_SPEC.md` version

---

## 🛠️ Manual Workflows

### Manual Build & Test

For debugging or testing specific configurations:

1. Go to Actions → Manual Build & Test
2. Select:
   - Rust version (stable/beta/nightly)
   - Run tests (yes/no)
   - Run benchmarks (yes/no)
   - Build release binary (yes/no)

### Manual Coverage Run

1. Go to Actions → Code Coverage
2. Click "Run workflow"
3. Report uploads to Codecov and creates artifact

---

## 📊 Code Coverage

### Automatic

- **PR**: Generates coverage + comments on PR
- **Push to main**: Uploads to Codecov
- **Manual**: Can trigger anytime

### Viewing Reports

- **Codecov**: https://codecov.io/gh/wayfind/intent-engine
- **PR Comments**: Automatic summary with percentage
- **Artifacts**: Download from workflow run

---

## 🏷️ Auto-Labeling

PRs are automatically labeled based on changed files:

```yaml
documentation  → docs/, *.md
rust          → src/, Cargo.toml
tests         → tests/, benches/
ci            → .github/workflows/
dependencies  → Cargo.toml, Cargo.lock
mcp           → mcp-server related
cli           → CLI related
```

---

## 🤖 Dependency Management

### Dependabot

Automatically creates PRs weekly for:

- Cargo dependencies (grouped by type)
- GitHub Actions updates

Configuration: `.github/dependabot.yml`

### Security Audits

Daily checks for:

- Security vulnerabilities (`cargo audit`)
- License compliance (`cargo deny`)
- Known advisories

---

## 📝 CHANGELOG Generation

Automatic CHANGELOG generation using conventional commits:

```bash
# Commit message format
feat: add new feature        → Features
fix: resolve bug             → Bug Fixes
docs: update documentation   → Documentation
perf: improve performance    → Performance
refactor: restructure code   → Refactor
test: add tests              → Testing
chore: maintenance           → Miscellaneous
```

When a release tag is pushed, `git-cliff` generates a CHANGELOG and creates a PR.

Configuration: `cliff.toml`

---

## 🔧 Debugging Failed CI

### Format Failures

```bash
# Fix locally
cargo fmt --all

# Check before commit
cargo fmt --all -- --check
```

### Clippy Failures

```bash
# Fix locally
cargo clippy --all-targets --all-features --fix

# Check
cargo clippy --all-targets --all-features -- -D warnings
```

### Test Failures

```bash
# Run tests locally
cargo test --verbose

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Cross-Platform Issues

Use Manual Build workflow to test specific platforms:

1. Actions → Manual Build & Test
2. Select target OS and Rust version
3. Review logs

---

## 🎯 Best Practices

### For Contributors

1. **Before PR**:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features
   cargo test
   ```

2. **Optional UI Tests** (pre-commit hook):

   By default, UI/Dashboard integration tests are skipped during commit to keep the process fast.

   To enable UI tests in pre-commit hook:
   ```bash
   # One-time commit with UI tests
   INTENT_RUN_UI_TESTS=1 git commit -m "your message"

   # Or export for entire session
   export INTENT_RUN_UI_TESTS=1
   git commit -m "your message"

   # To disable again
   unset INTENT_RUN_UI_TESTS
   ```

   Run UI tests manually:
   ```bash
   cargo test --test dashboard_integration_tests --all-features
   ```

3. **Commit Messages**: Use conventional commits
   ```bash
   feat: add new feature
   fix: resolve issue
   docs: update readme
   ```

4. **PR Description**: Clear description of changes

### For Maintainers

1. **Releasing**:
   - Use Version Bump workflow
   - Review generated PR
   - Verify release notes

2. **Security**:
   - Review Dependabot PRs weekly
   - Address security issues immediately
   - Update dependencies regularly

3. **Monitoring**:
   - Check daily security audit results
   - Review coverage trends
   - Monitor CI performance

---

## 📈 Performance

### Current Benchmarks

- **PR CI**: ~3-5 minutes
- **Main CI**: ~15-20 minutes
- **Release Build**: ~20-30 minutes (5 platforms)

### Optimization Features

- ✅ Swatinem/rust-cache for dependency caching
- ✅ Parallel job execution
- ✅ Conditional job execution
- ✅ Artifact retention limits (7-30 days)

---

## 🔄 Migration from Old CI

### What Changed

**Before** (Old System):
- 600+ line ci.yml with complex conditions
- Duplicate code across 4+ workflows
- Manual version management
- Inconsistent caching

**After** (New System):
- Clean separation of concerns
- Reusable workflows
- Automated version management
- Consistent caching strategy

### Breaking Changes

- None for contributors
- Maintainers: Use new Version Bump workflow

---

## 📚 References

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)
- [git-cliff](https://git-cliff.org/)
- [cargo-edit](https://github.com/killercup/cargo-edit)

---

## 🆘 Troubleshooting

### CI Stuck or Slow

1. Check GitHub Actions status page
2. Review cache usage
3. Manually trigger workflow with fresh cache

### Version Mismatch

```bash
# Verify versions match
grep 'version = ' Cargo.toml
grep 'Version:' CLAUDE.md
```

### Failed Release

1. Check workflow logs
2. Verify tag format (`v` prefix required)
3. Ensure CARGO_REGISTRY_TOKEN secret is set
4. Re-run failed jobs

---

**Last Updated**: 2024-11-10
**System Version**: 2.0 (Post-refactor)
