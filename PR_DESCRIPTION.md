# Pull Request: 改进安装体验：添加多种包管理器支持 (v0.1.4)

## 📦 改进安装体验

此 PR 为 Intent-Engine 添加了全面的安装体验改进，支持多种包管理器和安装方式。

## ✨ 新增功能

### 1. Cargo Install（推荐安装方式）
- ✅ 在 README 中突出显示 `cargo install` 作为推荐方式
- ✅ 支持从 crates.io 直接安装
- ✅ 添加自动发布到 crates.io 的 GitHub Actions workflow

### 2. Homebrew 支持
- ✅ 创建 Homebrew formula (`homebrew/intent-engine.rb`)
- ✅ 支持 macOS 和 Linux 多架构（x86_64, ARM64）
- ✅ 添加自动更新 formula SHA256 的脚本
- ✅ 提供 Homebrew tap 维护文档

### 3. cargo-binstall 支持
- ✅ 在 Cargo.toml 添加 binstall 元数据配置
- ✅ 支持从 GitHub Releases 快速安装预编译二进制
- ✅ 自动选择正确的平台和架构

### 4. 完整文档
- ✅ 新增 `INSTALLATION.md` 全面安装指南
- ✅ 新增 `docs/HOW_TO_TEST_RELEASE.md` 发布测试指南
- ✅ 新增 `docs/TESTING_RELEASE.md` 详细测试参考
- ✅ 新增 `scripts/test-release.sh` 自动化测试脚本

## 📝 更改的文件

### 核心配置
- `.github/workflows/release.yml`: 添加 crates.io 自动发布 job
- `Cargo.toml`: 添加 cargo-binstall 配置，版本更新至 0.1.4
- `README.md`: 重组安装部分，突出推荐方式

### 新增文件
- `INSTALLATION.md`: 完整安装指南（7 种安装方式）
- `homebrew/intent-engine.rb`: Homebrew formula
- `homebrew/README.md`: Homebrew 维护指南
- `scripts/update-homebrew-formula.sh`: SHA256 自动更新脚本
- `scripts/test-release.sh`: 发布预检脚本
- `docs/HOW_TO_TEST_RELEASE.md`: 完整发布测试指南
- `docs/TESTING_RELEASE.md`: 详细测试参考

## 🎯 现在支持的安装方式

1. ✅ **cargo install** (推荐) - 从 crates.io
2. ✅ **Homebrew** (即将支持) - macOS/Linux
3. ✅ **cargo-binstall** - 快速安装预编译二进制
4. ✅ **下载预编译二进制** - 手动安装
5. ✅ **从源码构建** - 开发者
6. ✅ **MCP Server 集成** - Claude Code
7. ✅ **Claude Code Skill** - 轻量级集成

## 🔧 发布配置

### GitHub Actions
添加了新的 `publish-crates-io` job：
- 在创建 GitHub Release 后自动触发
- 使用 `CARGO_REGISTRY_TOKEN` secret 登录 crates.io
- 执行 `cargo publish` 自动发布

### cargo-binstall 配置
为所有平台配置了正确的下载 URL：
- Linux x86_64/ARM64
- macOS x86_64/ARM64 (Apple Silicon)
- Windows x86_64

## 🧪 测试

- ✅ 本地 dry-run 测试通过
- ✅ 打包验证通过（58 个文件）
- ✅ 编译验证通过
- ✅ CI 将自动运行

## 📋 合并后的操作

合并此 PR 后，需要：

1. **创建 v0.1.4 tag**：
   ```bash
   git checkout main
   git pull
   git tag v0.1.4
   git push origin v0.1.4
   ```

2. **这会自动触发**：
   - 构建所有平台的二进制
   - 创建 GitHub Release
   - 发布到 crates.io

3. **后续可选操作**：
   - 创建 Homebrew tap 仓库（wayfind/homebrew-tap）
   - 更新 Homebrew formula
   - 公告新版本发布

## 📚 相关文档

- [INSTALLATION.md](INSTALLATION.md) - 完整安装指南
- [docs/HOW_TO_TEST_RELEASE.md](docs/HOW_TO_TEST_RELEASE.md) - 发布测试指南
- [homebrew/README.md](homebrew/README.md) - Homebrew 维护指南

## ⚠️ 注意事项

**需要确保 GitHub Secret 已设置：**
- `CARGO_REGISTRY_TOKEN`: crates.io API token

没有此 token，自动发布到 crates.io 会失败（但不影响 GitHub Release）。

---

## 🎉 影响

此次改进显著降低了用户的安装门槛：

| 用户类型 | 之前 | 现在 |
|---------|------|------|
| Rust 开发者 | 从源码构建 | `cargo install intent-engine` |
| macOS/Linux 用户 | 下载二进制 | `brew install intent-engine` |
| 快速安装 | 从源码编译 | `cargo binstall intent-engine` |

## 📊 提交历史

此 PR 包含以下提交：
- e82a8d4: 改进安装体验：添加多种包管理器支持
- 0521bee: 添加 crates.io 发布测试指南
- c6ba89c: 添加发布测试脚本
- 23c74d2: 添加完整的发布测试指南
- 83371e3: Bump version to 0.1.4
