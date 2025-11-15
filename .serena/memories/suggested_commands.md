# Suggested Development Commands

## Building

```bash
# Build the project
cargo build

# Build with release optimizations
cargo build --release

# Build documentation
cargo doc --no-deps --all-features
```

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test file
cargo test --test cli_tests

# Run unit and CLI tests only
cargo test --lib --bins

# Run doc tests
cargo test --doc --all-features

# Run benchmarks
cargo bench
```

## Code Quality

```bash
# Format code (auto-fix)
cargo fmt --all

# Check formatting (CI mode)
cargo fmt --all -- --check

# Run clippy lints
cargo clippy --all-targets --all-features -- -D warnings

# Full check (format + clippy + test)
make check
```

## Running the Binary

```bash
# Run directly with cargo
cargo run --bin ie -- <args>

# Example: run doctor command
cargo run --bin ie -- doctor

# Example: add a task
cargo run --bin ie -- task add --name "My Task"

# After building, run the binary directly
./target/debug/ie <args>
./target/release/ie <args>
```

## Development Workflow

```bash
# First time setup: install git hooks
./scripts/setup-git-hooks.sh
# or
make setup-hooks

# During development: format code
make fmt

# Before committing: run all checks
make check

# Run tests
make test
```

## Git Hooks

The project uses pre-commit hooks to auto-format code:
- Installed via `./scripts/setup-git-hooks.sh`
- Runs `cargo fmt --all` before each commit
- Skip with `git commit --no-verify` (not recommended)

## MCP Server

```bash
# Run MCP server (for Claude Code integration)
cargo run --bin ie -- mcp-server
```

## Continuous Integration

Local CI check:
```bash
# Quick CI check
./scripts/ci-quick.sh

# Full CI simulation
./scripts/ci-local.sh
```
