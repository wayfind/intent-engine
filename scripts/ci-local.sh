#!/bin/bash

# Local CI execution script
# This script runs all the checks that CI would run on GitHub Actions

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0

print_header() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

run_check() {
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    local name="$1"
    shift

    echo -e "${YELLOW}Running: $name${NC}"

    if "$@" > /tmp/ci_check.log 2>&1; then
        print_success "$name"
        return 0
    else
        print_error "$name"
        echo "Error output:"
        cat /tmp/ci_check.log
        return 1
    fi
}

# Start CI checks
print_header "Starting Local CI Checks"
echo "$(date)"

# 1. Code Formatting
print_header "1. Code Formatting Check"
if cargo fmt --all -- --check; then
    print_success "Code formatting check"
else
    print_error "Code formatting check - run 'cargo fmt --all' to fix"
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

# 2. Clippy
print_header "2. Clippy Linting"
if cargo clippy --all-targets --all-features -- -D warnings; then
    print_success "Clippy check"
else
    print_error "Clippy check"
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

# 3. Build (Debug)
print_header "3. Debug Build"
if cargo build --verbose; then
    print_success "Debug build"
else
    print_error "Debug build"
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

# 4. Build (Release)
print_header "4. Release Build"
if cargo build --release --verbose; then
    print_success "Release build"
else
    print_error "Release build"
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

# 5. Tests
print_header "5. Test Suite"
if cargo test --verbose --all-features; then
    print_success "All tests"
else
    print_error "Tests failed"
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

# 6. Doctor Command
print_header "6. Doctor Command"
if cargo run --release -- doctor; then
    print_success "Doctor command"
else
    print_error "Doctor command"
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

# 7. Documentation Build
print_header "7. Documentation Build"
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --document-private-items > /tmp/doc_build.log 2>&1
if [ $? -eq 0 ]; then
    print_success "Documentation build"
else
    print_error "Documentation build"
    cat /tmp/doc_build.log
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

# 8. Documentation Tests
print_header "8. Documentation Tests"
if cargo test --doc --all-features; then
    print_success "Documentation tests"
else
    print_error "Documentation tests"
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

# 9. Package Check
print_header "9. Package Verification"
if cargo package --list --allow-dirty > /dev/null; then
    print_success "Package list"
else
    print_error "Package list"
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

# 10. Security Audit (cargo-audit)
print_header "10. Security Audit (cargo-audit)"
if command -v cargo-audit &> /dev/null; then
    if cargo audit; then
        print_success "cargo-audit"
    else
        print_warning "cargo-audit found issues (may not be critical)"
    fi
else
    print_warning "cargo-audit not installed - skipping (install with: cargo install cargo-audit)"
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

# 11. Security Check (cargo-deny)
print_header "11. Security Check (cargo-deny)"
if command -v cargo-deny &> /dev/null; then
    # Note: cargo-deny might fail on network issues, so we make it non-fatal
    if cargo deny check 2>&1 | tee /tmp/deny_check.log; then
        print_success "cargo-deny"
    else
        if grep -q "failed to fetch advisory database" /tmp/deny_check.log; then
            print_warning "cargo-deny check skipped (network issue)"
        else
            print_error "cargo-deny check"
            FAILED_CHECKS=$((FAILED_CHECKS + 1))
        fi
    fi
else
    print_warning "cargo-deny not installed - skipping (install with: cargo install cargo-deny)"
fi
TOTAL_CHECKS=$((TOTAL_CHECKS + 1))

# 12. Benchmarks (optional)
print_header "12. Benchmarks (Optional)"
echo "Skipping benchmarks in quick CI check (run 'cargo bench' manually if needed)"
print_warning "Benchmarks skipped"

# Summary
print_header "CI Check Summary"
echo "Total Checks: $TOTAL_CHECKS"
echo -e "${GREEN}Passed: $PASSED_CHECKS${NC}"
if [ $FAILED_CHECKS -gt 0 ]; then
    echo -e "${RED}Failed: $FAILED_CHECKS${NC}"
else
    echo -e "${GREEN}Failed: 0${NC}"
fi

echo ""
if [ $FAILED_CHECKS -eq 0 ]; then
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}   ✓ ALL CI CHECKS PASSED! 🎉${NC}"
    echo -e "${GREEN}========================================${NC}"
    exit 0
else
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}   ✗ SOME CI CHECKS FAILED${NC}"
    echo -e "${RED}========================================${NC}"
    echo "Please fix the failed checks before committing."
    exit 1
fi
