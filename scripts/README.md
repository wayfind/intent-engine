# CI Scripts

本目录包含本地运行 CI 检查的脚本。

## 可用脚本

### ci-quick.sh (推荐)

快速 CI 检查脚本，显示简洁的输出。

```bash
./scripts/ci-quick.sh
```

**包含检查:**
- ✓ 代码格式化 (`cargo fmt`)
- ✓ Clippy 静态分析
- ✓ Debug 构建
- ✓ Release 构建
- ✓ 完整测试套件
- ✓ Doctor 命令
- ✓ 文档构建
- ✓ 文档测试
- ✓ 包验证
- ✓ 安全审计 (如果已安装)
- ✓ Dependency 检查 (如果已安装)

**运行时间:** ~2-5 分钟

### ci-local.sh

详细的 CI 检查脚本，提供完整输出和日志。

```bash
./scripts/ci-local.sh
```

包含与 ci-quick.sh 相同的检查，但会显示每个命令的完整输出，便于调试。

**运行时间:** ~2-5 分钟

## 使用场景

### 提交前检查

在提交代码前运行快速检查：

```bash
./scripts/ci-quick.sh && git commit -m "your message"
```

### 推送前检查

在推送到远程前确保所有检查通过：

```bash
./scripts/ci-quick.sh && git push
```

### 调试失败

如果快速检查失败，使用详细脚本查看错误：

```bash
./scripts/ci-local.sh
```

### 手动运行单个检查

也可以手动运行单个检查：

```bash
# 格式化
cargo fmt --all -- --check

# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# 测试
cargo test --all-features

# 文档
cargo doc --no-deps --all-features

# 安全审计
cargo audit
cargo deny check
```

## 安装额外工具

脚本会自动跳过未安装的可选工具，但建议安装以获得完整检查：

```bash
# 安装 cargo-audit (安全漏洞扫描)
cargo install cargo-audit

# 安装 cargo-deny (依赖检查)
cargo install cargo-deny

# 安装 cargo-tarpaulin (代码覆盖率，仅 Linux)
cargo install cargo-tarpaulin

# 安装 cargo-outdated (过时依赖检查)
cargo install cargo-outdated
```

## 与 GitHub Actions CI 的对应关系

这些脚本运行的检查与 GitHub Actions CI (`.github/workflows/ci.yml`) 相同的核心检查：

| 本地脚本检查 | GitHub Actions Job |
|-------------|-------------------|
| Format | test (stable) |
| Clippy | test (stable/beta) |
| Build | test (all platforms) |
| Tests | test (all platforms) |
| Doctor | test (all platforms) |
| Docs | docs |
| Doc tests | docs |
| Package | check-package |
| Audit | security-audit |
| Deny | security-audit |

GitHub Actions 还包含额外的检查：
- 多平台测试 (Linux, macOS, Windows)
- 多版本测试 (stable, beta, nightly)
- 代码覆盖率报告
- 最小依赖版本测试
- 基准测试
- 依赖审查
- 安装脚本测试

## 故障排除

### 格式检查失败

```bash
# 自动修复格式问题
cargo fmt --all
```

### Clippy 失败

查看具体警告并修复，或者添加 `#[allow(...)]` 属性（谨慎使用）。

### 测试失败

```bash
# 运行特定测试
cargo test test_name -- --nocapture

# 只运行失败的测试
cargo test --all-features -- --nocapture
```

### 文档构建失败

检查文档注释中的语法错误和损坏的链接。

```bash
# 查看详细错误
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --document-private-items
```

### cargo-deny 失败

通常是网络问题或许可证/依赖配置问题。查看 `deny.toml` 配置。

## 性能优化

### 缓存

脚本会利用 Cargo 的缓存机制，第二次运行会快很多。

### 并行运行

如果只想快速检查格式和 clippy：

```bash
cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings
```

### 跳过耗时检查

如果只想快速验证代码能编译：

```bash
cargo check --all-targets --all-features
```

## 相关文档

- [CONTRIBUTING.md](../CONTRIBUTING.md) - 完整的贡献指南
- [.github/CI_OVERVIEW.md](../.github/CI_OVERVIEW.md) - CI 系统详细说明
- [GitHub Actions 工作流](../.github/workflows/) - CI 配置文件
