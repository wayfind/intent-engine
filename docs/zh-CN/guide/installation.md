# Intent-Engine 安装指南

本文档详细介绍了 Intent-Engine 的各种安装方式，以及如何为项目贡献者设置发布流程。

## 🚀 用户安装方式

### 1. Cargo Install（推荐）

**适用于：** 已安装 Rust 的用户

这是最简单、最标准的安装方式。Cargo 会自动从 [crates.io](https://crates.io/crates/intent-engine) 下载并编译最新版本。

```bash
cargo install intent-engine
```

**首次安装 Rust：**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**优点：**
- ✅ 始终获得最新版本
- ✅ 自动适配你的平台
- ✅ 与 Rust 生态系统集成

**缺点：**
- ❌ 需要编译（可能需要几分钟）
- ❌ 需要安装 Rust 工具链

---

### 2. Homebrew（macOS/Linux）

**适用于：** macOS 和 Linux 用户

```bash
# 添加 wayfind tap（首次）
brew tap wayfind/tap

# 安装 intent-engine
brew install intent-engine

# 更新
brew upgrade intent-engine
```

**优点：**
- ✅ 无需 Rust
- ✅ 预编译二进制，安装快速
- ✅ 方便的版本管理

**缺点：**
- ❌ 需要维护 Homebrew tap
- ❌ 可能不是最新版本

---

### 3. cargo-binstall（快速安装）

**适用于：** 想要快速安装预编译二进制的 Rust 用户

```bash
# 安装 cargo-binstall（首次）
cargo install cargo-binstall

# 使用 binstall 安装 intent-engine
cargo binstall intent-engine
```

**优点：**
- ✅ 比 cargo install 快得多（无需编译）
- ✅ 自动从 GitHub Releases 下载
- ✅ 自动选择正确的平台

**缺点：**
- ❌ 需要先安装 cargo-binstall

---

### 4. 下载预编译二进制

**适用于：** 不想安装任何工具链的用户

从 [GitHub Releases](https://github.com/wayfind/intent-engine/releases) 下载：

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
# 下载 Windows 版本
Invoke-WebRequest -Uri "https://github.com/wayfind/intent-engine/releases/latest/download/intent-engine-windows-x86_64.zip" -OutFile "intent-engine.zip"

# 解压
Expand-Archive -Path intent-engine.zip -DestinationPath .

# 手动将 intent-engine.exe 移动到 PATH 中的目录
```

**优点：**
- ✅ 无需任何依赖
- ✅ 完全控制安装位置

**缺点：**
- ❌ 需要手动更新
- ❌ 需要手动选择正确的平台

---

### 5. 从源码构建

**适用于：** 开发者和贡献者

```bash
# 克隆仓库
git clone https://github.com/wayfind/intent-engine.git
cd intent-engine

# 安装（推荐）
cargo install --path .

# 或者手动构建
cargo build --release
sudo cp target/release/intent-engine /usr/local/bin/
```

**优点：**
- ✅ 获得最新的开发版本
- ✅ 可以本地修改和测试

**缺点：**
- ❌ 需要 Rust 工具链
- ❌ 需要编译时间

---

## 🔧 验证安装

安装完成后，验证是否成功：

```bash
# 检查版本
intent-engine --version

# 运行健康检查
intent-engine doctor

# 查看帮助
intent-engine --help
```

---

## 🎯 选择安装方式

| 场景 | 推荐方式 |
|------|---------|
| 我有 Rust，想要最新版 | `cargo install` |
| 我用 macOS/Linux，想要快速安装 | Homebrew |
| 我有 Rust，想要快速安装 | `cargo binstall` |
| 我不想安装任何工具 | 下载预编译二进制 |
| 我是开发者/贡献者 | 从源码构建 |

---

## 🔄 更新 Intent-Engine

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

### 手动
重新下载最新的预编译二进制

---

## 🐛 故障排除

### 命令未找到

如果安装后提示 `command not found: intent-engine`，需要将二进制所在目录添加到 PATH：

```bash
# Cargo 安装的默认位置
export PATH="$HOME/.cargo/bin:$PATH"

# 添加到 shell 配置文件（永久生效）
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc  # 或 ~/.zshrc
```

### Cargo 编译失败

确保 Rust 版本为 1.70+：
```bash
rustc --version
rustup update
```

### macOS 安全警告

如果 macOS 提示"无法验证开发者"，运行：
```bash
xattr -d com.apple.quarantine /usr/local/bin/intent-engine
```

---

## 📦 维护者：发布流程

### 前提条件

1. 在 GitHub 仓库设置中添加 `CARGO_REGISTRY_TOKEN` secret
   - 从 [crates.io](https://crates.io/me) 获取 API token
   - 添加到 GitHub: Settings → Secrets → Actions → New repository secret

2. 确保有权限发布到 crates.io
   ```bash
   cargo login
   ```

### 发布新版本

1. **更新版本号**
   ```bash
   # 编辑 Cargo.toml
   version = "0.1.4"  # 新版本
   ```

2. **创建 Git tag**
   ```bash
   git tag v0.1.4
   git push origin v0.1.4
   ```

3. **自动触发 Release workflow**
   - 构建所有平台的二进制
   - 创建 GitHub Release
   - 发布到 crates.io

4. **更新 Homebrew formula**（可选）
   ```bash
   ./scripts/update-homebrew-formula.sh 0.1.4
   ```

### 手动发布到 crates.io

如果自动发布失败：
```bash
cargo publish
```

---

## 🌟 其他包管理器（计划中）

我们计划支持更多包管理器：

- **Scoop** (Windows)
- **Chocolatey** (Windows)
- **AUR** (Arch Linux)
- **nixpkgs** (Nix)

欢迎贡献！
