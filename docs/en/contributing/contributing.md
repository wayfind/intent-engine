# Contributing to Intent Engine

Thank you for your interest in the Intent Engine project! We welcome contributions of all kinds.

## Development Environment Setup

### Prerequisites

- Rust 1.70+ (recommended to use rustup)
- Git

### Clone Repository

```bash
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine
```

### Build Project

```bash
cargo build
```

### Run Tests

```bash
cargo test
```

### Run Doctor Command

```bash
cargo run -- doctor
```

## Pre-Commit Checklist

Before submitting a Pull Request, ensure all the following checks pass:

### 1. Code Formatting

```bash
cargo fmt --all
```

### 2. Clippy Check

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### 3. Run All Tests

```bash
cargo test --all-features
```

### 4. Documentation Tests

```bash
cargo test --doc --all-features
```

### 5. Build Documentation

```bash
cargo doc --no-deps --all-features
```

### 6. Run Benchmarks (Optional)

```bash
cargo bench
```

## CI/CD Process

Our CI system automatically runs multiple checks:

### Main CI Tasks

1. **Multi-platform Testing** - Test on Linux, macOS, and Windows
2. **Multi-version Testing** - Test with stable, beta, and nightly Rust
3. **Code Format Check** - Ensure consistent code style
4. **Clippy Check** - Catch common errors and code quality issues
5. **Code Coverage** - Generate coverage report using cargo-tarpaulin
6. **Security Audit** - Check dependencies with cargo-audit and cargo-deny
7. **Documentation Build** - Verify documentation can be built correctly
8. **Package Validation** - Ensure publishable to crates.io
9. **Minimal Dependency Version Test** - Ensure compatibility with minimal versions
10. **Installation Script Test** - Verify installation process

### CI Triggers

- Push to `main`, `master`, or `claude/**` branches
- Create Pull Request to `main` or `master`
- Daily automatic run at UTC 00:00 (check for dependency issues)

### Run Complete CI Checks Locally

```bash
# Format
cargo fmt --all

# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --all-features

# Documentation
cargo doc --no-deps --all-features

# Install cargo-audit (first time)
cargo install cargo-audit

# Run security audit
cargo audit

# Install cargo-deny (first time)
cargo install cargo-deny

# Run cargo-deny checks
cargo deny check

# Package check
cargo package --allow-dirty

# Doctor command
cargo run --release -- doctor
```

## Pull Request Process

1. **Fork the repository** to your GitHub account
2. **Create feature branch** (`git checkout -b feature/amazing-feature`)
3. **Commit your changes** (`git commit -m 'Add some amazing feature'`)
4. **Push to branch** (`git push origin feature/amazing-feature`)
5. **Create Pull Request**

### Pull Request Best Practices

- **Clear title and description** - Explain your changes and reasons
- **Small, focused PRs** - Each PR should focus on one feature or fix
- **Test coverage** - Add tests for new features
- **Update documentation** - If API changes, update documentation
- **Follow code style** - Run `cargo fmt`
- **Pass all CI checks** - Ensure all automated tests pass

## Commit Message Convention

We use Conventional Commits format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation update
- `style`: Code formatting (doesn't affect code execution)
- `refactor`: Code refactoring
- `perf`: Performance optimization
- `test`: Add tests
- `chore`: Changes to build process or auxiliary tools
- `ci`: Changes to CI configuration files and scripts
- `deps`: Dependency updates

**Example:**

```
feat(tasks): add priority and complexity fields

Add support for task priority (1-10) and complexity (1-10) to help
with task scheduling and workload estimation.

Closes #123
```

## Code Review

All submissions require code review. Reviewers will check:

- Code quality and readability
- Test coverage
- Performance impact
- Security considerations
- Backward compatibility
- Documentation completeness

## Reporting Bugs

Use GitHub Issues to report bugs, please include:

1. **Clear title**
2. **Reproduction steps**
3. **Expected behavior**
4. **Actual behavior**
5. **Environment information** (OS, Rust version, etc.)
6. **Relevant logs or error messages**

## Feature Requests

We welcome feature suggestions! Please create a GitHub Issue including:

1. **Use case description** - Why this feature is needed
2. **Proposed solution** - How you'd like to implement it
3. **Alternatives** - Other possible implementation approaches
4. **Additional context** - Any other relevant information

## Security Issues

If you discover a security vulnerability, please **do not** create a public Issue. Instead, email the project maintainers.

## License

By submitting code, you agree that your contributions will use the MIT OR Apache-2.0 dual license.

## Code of Conduct

- **Respect others** - Maintain friendly and professional behavior
- **Constructive feedback** - Provide helpful suggestions
- **Inclusivity** - Welcome contributors from all backgrounds
- **Patience** - Remember everyone is learning

## Getting Help

- 📖 Read the [documentation](./README.en.md)
- 💬 Ask questions in GitHub Discussions
- 🐛 Report [Issues](https://github.com/wayfind/intent-engine/issues)

---

Thank you again for your contribution! 🎉
