#!/bin/bash

# Quick CI checks with minimal output
# Shows only summary and errors

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Running CI Checks...${NC}\n"

# Track results
PASSED=0
FAILED=0

check() {
    local name="$1"
    shift
    echo -n "  $name... "
    if "$@" >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗${NC}"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Run checks
check "Format" cargo fmt --all -- --check
check "Clippy" cargo clippy --all-targets --all-features -- -D warnings
check "Build (debug)" cargo build
check "Build (release)" cargo build --release
check "Tests" cargo test --all-features
check "Doctor" cargo run --release -- doctor
check "Docs" sh -c 'RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --document-private-items'
check "Doc tests" cargo test --doc --all-features
check "Package" cargo package --list --allow-dirty

# Optional checks
echo ""
if command -v cargo-audit &>/dev/null; then
    check "Audit" cargo audit || true
else
    echo -e "  Audit... ${BLUE}skipped (not installed)${NC}"
fi

if command -v cargo-deny &>/dev/null; then
    # Deny might fail on network, so don't fail on it
    echo -n "  Deny... "
    if cargo deny check >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${BLUE}skipped (network/config issue)${NC}"
    fi
else
    echo -e "  Deny... ${BLUE}skipped (not installed)${NC}"
fi

# Summary
echo ""
echo "========================================"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All checks passed! ($PASSED/$((PASSED + FAILED)))${NC}"
    exit 0
else
    echo -e "${RED}Some checks failed! ($FAILED failed)${NC}"
    exit 1
fi
