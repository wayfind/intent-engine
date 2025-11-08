# Homebrew Formula for Intent-Engine

This directory contains the Homebrew formula for Intent-Engine.

## For Users

To install Intent-Engine via Homebrew:

```bash
# Add the wayfind tap
brew tap wayfind/tap

# Install intent-engine
brew install intent-engine
```

## For Maintainers

### Setting up the Homebrew Tap

1. Create a new repository: `wayfind/homebrew-tap`

2. Copy the formula to the tap repository:
   ```bash
   mkdir -p Formula
   cp intent-engine.rb Formula/
   git add Formula/intent-engine.rb
   git commit -m "Add intent-engine formula"
   git push
   ```

### Updating the Formula

When releasing a new version:

1. Update the version in the formula:
   ```ruby
   version "0.1.4"
   ```

2. Run the update script to calculate SHA256 checksums:
   ```bash
   cd /path/to/intent-engine
   ./scripts/update-homebrew-formula.sh 0.1.4
   ```

3. Copy the updated formula to the tap repository:
   ```bash
   cp homebrew/intent-engine.rb /path/to/homebrew-tap/Formula/
   cd /path/to/homebrew-tap
   git add Formula/intent-engine.rb
   git commit -m "Update intent-engine to 0.1.4"
   git push
   ```

### Manual SHA256 Update

If the automated script doesn't work, you can manually calculate checksums:

```bash
# Download the release file
curl -LO https://github.com/wayfind/intent-engine/releases/download/v0.1.4/intent-engine-macos-x86_64.tar.gz

# Calculate SHA256
sha256sum intent-engine-macos-x86_64.tar.gz

# Update the formula with the checksum
```

### Testing the Formula

Test the formula locally before publishing:

```bash
# Audit the formula
brew audit --strict intent-engine

# Test installation
brew install --build-from-source intent-engine

# Test the binary
intent-engine --version
intent-engine doctor
```

## Formula Template

The formula uses platform-specific URLs and checksums:

- **macOS ARM64** (M1/M2/M3): `intent-engine-macos-aarch64.tar.gz`
- **macOS x86_64** (Intel): `intent-engine-macos-x86_64.tar.gz`
- **Linux ARM64**: `intent-engine-linux-aarch64.tar.gz`
- **Linux x86_64**: `intent-engine-linux-x86_64.tar.gz`

All checksums are automatically updated during release.

## Resources

- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Creating and Maintaining a Tap](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)
