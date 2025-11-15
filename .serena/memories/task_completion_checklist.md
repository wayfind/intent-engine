# Task Completion Checklist

When completing a coding task, ensure the following steps are done:

## 1. Code Formatting ✅
```bash
cargo fmt --all
```
**Must pass**: Code must be formatted according to project standards.

## 2. Linting with Clippy ✅
```bash
cargo clippy --all-targets --all-features -- -D warnings
```
**Must pass**: No clippy warnings allowed (all warnings treated as errors).

## 3. Run Tests ✅
```bash
# All tests
cargo test

# Or use make
make test
```
**Must pass**: All existing tests must continue to pass.

## 4. Add Tests for New Features ✅
- Unit tests for new functions
- Integration tests for new commands/workflows
- Special character/edge case tests if applicable

## 5. Documentation ✅
- Add doc comments (`///`) for public APIs
- Update relevant documentation files if needed
- Ensure `cargo doc` builds without errors

## 6. Interface Changes ✅
If you modified CLI commands, MCP tools, or data models:
- Update `docs/INTERFACE_SPEC.md` if interface changed
- Update version number if breaking changes
- Update `CLAUDE.md` if AI integration affected
- Run `./scripts/sync-mcp-tools.sh` if MCP tools changed

## 7. Commit Standards ✅
- Use clear, descriptive commit messages
- Follow conventional commits if possible (feat:, fix:, docs:, etc.)
- Git hooks will auto-format code

## Quick Command for Full Check
```bash
make check
```
This runs: format + clippy + tests

## CI Expectations

The CI pipeline runs:
1. **Format Check**: `cargo fmt --all -- --check`
2. **Clippy**: `cargo clippy --all-targets --all-features -- -D warnings`
3. **Tests**: Full test suite on Linux/macOS/Windows
4. **Doc Build**: `cargo doc --no-deps --all-features`
5. **Security Audit**: `cargo audit` (scheduled daily)

All must pass before merging.
