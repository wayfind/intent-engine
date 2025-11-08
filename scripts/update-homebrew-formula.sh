#!/bin/bash
# Update Homebrew formula with SHA256 checksums from GitHub releases

set -e

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.1.3"
    exit 1
fi

FORMULA_FILE="homebrew/intent-engine.rb"

echo "Updating Homebrew formula for version ${VERSION}..."

# Download release files and calculate SHA256
declare -A CHECKSUMS

PLATFORMS=(
    "macos-aarch64:PLACEHOLDER_ARM64_SHA256"
    "macos-x86_64:PLACEHOLDER_X86_64_SHA256"
    "linux-aarch64:PLACEHOLDER_LINUX_ARM64_SHA256"
    "linux-x86_64:PLACEHOLDER_LINUX_X86_64_SHA256"
)

for platform_placeholder in "${PLATFORMS[@]}"; do
    IFS=':' read -r platform placeholder <<< "$platform_placeholder"

    url="https://github.com/wayfind/intent-engine/releases/download/v${VERSION}/intent-engine-${platform}.tar.gz"

    echo "Downloading ${platform}..."
    temp_file=$(mktemp)

    if curl -sL "$url" -o "$temp_file"; then
        sha256=$(sha256sum "$temp_file" | cut -d' ' -f1)
        echo "  SHA256: $sha256"

        # Update formula file
        if [[ "$OSTYPE" == "darwin"* ]]; then
            # macOS sed
            sed -i '' "s/$placeholder/$sha256/" "$FORMULA_FILE"
        else
            # Linux sed
            sed -i "s/$placeholder/$sha256/" "$FORMULA_FILE"
        fi
    else
        echo "  Warning: Failed to download $url"
    fi

    rm -f "$temp_file"
done

echo "Homebrew formula updated successfully!"
echo "Formula location: $FORMULA_FILE"
echo ""
echo "To publish to Homebrew tap, create a repository: wayfind/homebrew-tap"
echo "Then copy the formula to Formula/intent-engine.rb"
