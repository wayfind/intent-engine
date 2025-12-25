# Homebrew Formula for Intent-Engine

This directory contains the Homebrew formula for Intent-Engine.

## For Users

To install Intent-Engine via Homebrew:

```bash
# Add the wayfind tap
brew tap wayfind/intent-engine

# Install intent-engine (binary name: ie)
brew install intent-engine

# Verify installation
ie --version
ie doctor
```

## For Maintainers

### Setting up the Homebrew Tap

1. Create a new repository: `wayfind/homebrew-intent-engine`

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
   version "0.10.1"
   ```

2. Run the update script to calculate SHA256 checksums:
   ```bash
   cd /path/to/intent-engine
   ./scripts/update-homebrew-formula.sh 0.10.1
   ```

3. Copy the updated formula to the tap repository:
   ```bash
   cp homebrew/intent-engine.rb /path/to/homebrew-intent-engine/Formula/
   cd /path/to/homebrew-intent-engine
   git add Formula/intent-engine.rb
   git commit -m "Update intent-engine to 0.10.1"
   git push
   ```

### Automated Release (Recommended)

The release workflow automatically updates the Homebrew tap when a new version is released.
See `.github/workflows/release.yml` for details.

### Manual SHA256 Update

If the automated script doesn't work, you can manually calculate checksums:

```bash
# Download the release file
curl -LO https://github.com/wayfind/intent-engine/releases/download/v0.10.0/intent-engine-macos-aarch64.tar.gz

# Calculate SHA256
sha256sum intent-engine-macos-aarch64.tar.gz

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
ie --version
ie doctor
```

## Formula Platforms

- **macOS ARM64** (M1/M2/M3): `intent-engine-macos-aarch64.tar.gz`
- **macOS x86_64** (Intel): `intent-engine-macos-x86_64.tar.gz`
- **Linux ARM64**: `intent-engine-linux-aarch64.tar.gz`
- **Linux x86_64**: `intent-engine-linux-x86_64.tar.gz`

## Resources

- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Creating and Maintaining a Tap](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)
