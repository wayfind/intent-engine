#!/bin/bash

# Intent Engine Installation Script for Unix/Linux/macOS
# This script installs intent-engine either from crates.io or builds from source

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running on supported platform
print_info "Checking system compatibility..."
OS="$(uname -s)"
case "${OS}" in
    Linux*)     PLATFORM=Linux;;
    Darwin*)    PLATFORM=macOS;;
    *)          print_error "Unsupported operating system: ${OS}"; exit 1;;
esac
print_info "Platform: ${PLATFORM}"

# Check if Rust and Cargo are installed
print_info "Checking for Rust and Cargo..."
if ! command -v cargo &> /dev/null; then
    print_error "Cargo is not installed!"
    print_info "Please install Rust and Cargo from https://rustup.rs/"
    print_info "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

RUST_VERSION=$(rustc --version)
CARGO_VERSION=$(cargo --version)
print_info "Found: ${RUST_VERSION}"
print_info "Found: ${CARGO_VERSION}"

# Determine installation method
if [ -f "Cargo.toml" ] && grep -q "name = \"intent-engine\"" Cargo.toml 2>/dev/null; then
    print_info "Detected intent-engine source repository"
    print_info "Installing from source..."

    # Build and install from source
    cargo install --path . --force

    if [ $? -eq 0 ]; then
        print_info "Successfully installed intent-engine from source!"
    else
        print_error "Installation from source failed!"
        exit 1
    fi
else
    print_info "Installing from crates.io..."

    # Install from crates.io
    cargo install intent-engine --force

    if [ $? -eq 0 ]; then
        print_info "Successfully installed intent-engine from crates.io!"
    else
        print_error "Installation from crates.io failed!"
        print_info "This might mean the package hasn't been published yet."
        print_info "Please clone the repository and run this script from within it."
        exit 1
    fi
fi

# Verify installation
print_info "Verifying installation..."
if command -v intent-engine &> /dev/null; then
    VERSION=$(intent-engine --version 2>&1 || echo "unknown")
    print_info "intent-engine is installed: ${VERSION}"

    # Run doctor command to check system health
    print_info "Running system health check..."
    intent-engine doctor

    if [ $? -eq 0 ]; then
        echo ""
        print_info "Installation complete! 🎉"
        print_info "You can now use 'intent-engine' command"
        print_info "Try: intent-engine --help"
    else
        print_warning "Installation succeeded but health check failed"
        print_info "You may need to troubleshoot your environment"
    fi
else
    print_error "Installation verification failed!"
    print_info "The binary may not be in your PATH"
    print_info "Please add ~/.cargo/bin to your PATH"
    print_info "Add this to your ~/.bashrc or ~/.zshrc:"
    print_info "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
    exit 1
fi
