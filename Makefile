.PHONY: fmt check test commit setup-hooks help

help:
	@echo "Intent-Engine Development Commands"
	@echo "===================================="
	@echo "make fmt          - Format code with rustfmt"
	@echo "make check        - Run format, clippy and tests"
	@echo "make test         - Run all tests"
	@echo "make commit       - Format and commit (interactive)"
	@echo "make setup-hooks  - Install git pre-commit hooks"
	@echo ""
	@echo "Git hooks will auto-format code before each commit."

fmt:
	@echo "Formatting code..."
	@cargo fmt --all
	@echo "✓ Code formatted"

check: fmt
	@echo "Running clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings
	@echo "Running tests..."
	@cargo test
	@echo "✓ All checks passed"

test:
	@cargo test

commit: fmt
	@echo "Code formatted. Ready to commit."
	@git status
	@echo ""
	@echo "Run: git add <files> && git commit -m 'message'"

setup-hooks:
	@./scripts/setup-git-hooks.sh
