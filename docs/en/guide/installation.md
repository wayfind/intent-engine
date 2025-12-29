# Intent-Engine Installation Guide

This document details various installation methods for Intent-Engine, as well as how to set up the release process for project contributors.

## 🚀 User Installation Methods

### 1. Cargo Install (Recommended)

**For:** Users who have Rust installed

This is the simplest and most standard installation method. Cargo automatically downloads and compiles the latest version from [crates.io](https://crates.io/crates/intent-engine).

```bash
cargo install intent-engine
```

**First time installing Rust:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Pros:**
- ✅ Always get the latest version
- ✅ Automatically adapts to your platform
- ✅ Integrates with Rust ecosystem

**Cons:**
- ❌ Requires compilation (may take a few minutes)
- ❌ Requires Rust toolchain

---

### 2. Homebrew (macOS/Linux)

**For:** macOS and Linux users

```bash
# Add wayfind tap (first time)
brew tap wayfind/tap

# Install intent-engine
brew install intent-engine

# Update
brew upgrade intent-engine
```

**Pros:**
- ✅ No Rust required
- ✅ Pre-compiled binary, fast installation
- ✅ Convenient version management

**Cons:**
- ❌ Requires maintaining Homebrew tap
- ❌ May not be the latest version

---

### 3. cargo-binstall (Quick Install)

**For:** Rust users who want quick installation of pre-compiled binaries

```bash
# Install cargo-binstall (first time)
cargo install cargo-binstall

# Use binstall to install intent-engine
cargo binstall intent-engine
```

**Pros:**
- ✅ Much faster than cargo install (no compilation needed)
- ✅ Automatically downloads from GitHub Releases
- ✅ Automatically selects correct platform

**Cons:**
- ❌ Need to install cargo-binstall first

---

### 4. Download Pre-compiled Binaries

**For:** Users who don't want to install any toolchain

Download from [GitHub Releases](https://github.com/wayfind/intent-engine/releases):

#### Linux
```bash
# x86_64
curl -LO https://github.com/wayfind/intent-engine/releases/latest/download/intent-engine-linux-x86_64.tar.gz
tar xzf intent-engine-linux-x86_64.tar.gz
sudo mv intent-engine /usr/local/bin/

# ARM64
curl -LO https://github.com/wayfind/intent-engine/releases/latest/download/intent-engine-linux-aarch64.tar.gz
tar xzf intent-engine-linux-aarch64.tar.gz
sudo mv intent-engine /usr/local/bin/
```

#### macOS
```bash
# Intel Mac
curl -LO https://github.com/wayfind/intent-engine/releases/latest/download/intent-engine-macos-x86_64.tar.gz
tar xzf intent-engine-macos-x86_64.tar.gz
sudo mv intent-engine /usr/local/bin/

# Apple Silicon (M1/M2/M3)
curl -LO https://github.com/wayfind/intent-engine/releases/latest/download/intent-engine-macos-aarch64.tar.gz
tar xzf intent-engine-macos-aarch64.tar.gz
sudo mv intent-engine /usr/local/bin/
```

#### Windows
```powershell
# Download Windows version
Invoke-WebRequest -Uri "https://github.com/wayfind/intent-engine/releases/latest/download/intent-engine-windows-x86_64.zip" -OutFile "intent-engine.zip"

# Extract
Expand-Archive -Path intent-engine.zip -DestinationPath .

# Manually move intent-engine.exe to a directory in PATH
```

**Pros:**
- ✅ No dependencies required
- ✅ Full control over installation location

**Cons:**
- ❌ Manual updates required
- ❌ Need to manually select correct platform

---

### 5. Build from Source

**For:** Developers and contributors

```bash
# Clone repository
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine

# Install (recommended)
cargo install --path .

# Or build manually
cargo build --release
sudo cp target/release/intent-engine /usr/local/bin/
```

**Pros:**
- ✅ Get latest development version
- ✅ Can modify and test locally

**Cons:**
- ❌ Requires Rust toolchain
- ❌ Compilation time required

---

## 🔧 Verify Installation

After installation, verify success:

```bash
# Check version
ie --version

# Run health check
ie doctor

# View help
ie --help
```

---

## 🎯 Choosing Installation Method

| Scenario | Recommended Method |
|----------|-------------------|
| I have Rust, want latest version | `cargo install` |
| I use macOS/Linux, want quick install | Homebrew |
| I have Rust, want quick install | `cargo binstall` |
| I don't want to install any tools | Download pre-compiled binary |
| I'm a developer/contributor | Build from source |

---

## 🔄 Updating Intent-Engine

### Cargo
```bash
cargo install intent-engine --force
```

### Homebrew
```bash
brew upgrade intent-engine
```

### cargo-binstall
```bash
cargo binstall intent-engine --force
```

### Manual
Re-download the latest pre-compiled binary

---

## 🐛 Troubleshooting

### Command Not Found

If you get `command not found: ie` after installation, add the binary directory to PATH:

```bash
# Default location for Cargo install
export PATH="$HOME/.cargo/bin:$PATH"

# Add to shell config file (permanent)
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc  # or ~/.zshrc
```

### Cargo Compilation Failed

Ensure Rust version is 1.70+:
```bash
rustc --version
rustup update
```

### macOS Security Warning

If macOS shows "unable to verify developer", run:
```bash
xattr -d com.apple.quarantine /usr/local/bin/intent-engine
```

---

## 📦 Maintainers: Release Process

### Prerequisites

1. Add `CARGO_REGISTRY_TOKEN` secret in GitHub repository settings
   - Get API token from [crates.io](https://crates.io/me)
   - Add to GitHub: Settings → Secrets → Actions → New repository secret

2. Ensure permission to publish to crates.io
   ```bash
   cargo login
   ```

### Publishing a New Version

1. **Update version number**
   ```bash
   # Edit Cargo.toml
   version = "0.1.4"  # New version
   ```

2. **Create Git tag**
   ```bash
   git tag v0.1.4
   git push origin v0.1.4
   ```

3. **Automatically trigger Release workflow**
   - Build binaries for all platforms
   - Create GitHub Release
   - Publish to crates.io

4. **Update Homebrew formula** (optional)
   ```bash
   ./scripts/update-homebrew-formula.sh 0.1.4
   ```

### Manual Publishing to crates.io

If automatic publishing fails:
```bash
cargo publish
```

---

## 🌟 Other Package Managers (Planned)

We plan to support more package managers:

- **Scoop** (Windows)
- **Chocolatey** (Windows)
- **AUR** (Arch Linux)
- **nixpkgs** (Nix)

Contributions welcome!
