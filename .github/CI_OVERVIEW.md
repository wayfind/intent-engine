# CI/CD 系统概览

本项目采用了全面的 CI/CD 流程，确保代码质量、安全性和可靠性。

## 📋 CI 流程总览

### 主要 CI 检查 (`.github/workflows/ci.yml`)

#### 1. **多平台多版本测试**
- **平台**: Linux, macOS, Windows
- **Rust 版本**: stable, beta, nightly (允许失败)
- **检查项**:
  - 代码格式化 (`cargo fmt`)
  - Clippy 静态分析 (`-D warnings`)
  - 完整测试套件
  - Release 构建验证
  - Doctor 命令健康检查

#### 2. **最小依赖版本测试**
- 使用 `-Z minimal-versions` 确保与最小依赖版本兼容
- 避免意外依赖更新版本的功能

#### 3. **代码覆盖率**
- 工具: `cargo-tarpaulin`
- 集成: Codecov
- 目标: 80% 覆盖率
- 自动 PR 评论展示覆盖率变化

#### 4. **安全审计**
- **cargo-audit**: 检查已知安全漏洞
- **cargo-deny**:
  - 许可证合规检查
  - 重复依赖检测
  - 源码验证
  - 废弃包警告

#### 5. **文档验证**
- 文档构建检查 (`cargo doc`)
- 文档测试 (`cargo test --doc`)
- 确保所有 API 文档示例可运行

#### 6. **包发布验证**
- 列出将要发布的文件
- 实际构建和测试打包内容
- 确保发布到 crates.io 时不会失败

#### 7. **依赖审查** (仅 PR)
- 使用 GitHub Dependency Review Action
- 检测新引入的漏洞依赖
- 阻止有问题的依赖合并

#### 8. **过时依赖检查** (每日)
- 使用 `cargo-outdated`
- 每天 UTC 00:00 运行
- 及时发现可更新的依赖

#### 9. **基准测试**
- 运行性能基准测试
- 归档结果供分析
- 允许失败（不阻塞 CI）

#### 10. **安装脚本测试**
- 在 Linux 和 macOS 上测试 `scripts/install/install.sh`
- 验证一键安装流程
- 确保安装后 `doctor` 命令可用

#### 11. **最终状态检查**
- 聚合所有关键 job 的状态
- 提供统一的 CI 通过/失败标志
- 便于设置分支保护规则

## 🔄 自动化工作流

### Release PR 工作流 (`.github/workflows/release-pr.yml`)

当 PR 标记为 `release` 时自动运行：

1. **版本检查**
   - 验证版本号未被使用
   - 防止重复发布

2. **变更日志验证**
   - 检查 CHANGELOG.md 是否包含新版本
   - 确保发布有文档记录

3. **发布预检**
   - 构建和测试打包内容
   - 干运行发布流程 (`cargo publish --dry-run`)

4. **自动评论**
   - 在 PR 上添加发布就绪评论
   - 提供合并后的下一步操作指引

### Dependabot (`.github/dependabot.yml`)

自动依赖更新：

- **Cargo 依赖**: 每周一检查
- **GitHub Actions**: 每周一检查
- **自动分组**: minor 和 patch 更新合并到一个 PR
- **自动标签**: 依赖类型标签
- **自动审查者**: 指定维护者团队

## 🔒 安全配置

### cargo-deny (`deny.toml`)

全面的依赖管理策略：

1. **安全咨询**
   - 漏洞: 拒绝 (deny)
   - 废弃包: 警告 (warn)
   - 被撤回: 警告 (warn)

2. **许可证策略**
   - 允许: MIT, Apache-2.0, BSD 等
   - 拒绝: GPL-2.0, GPL-3.0, AGPL-3.0
   - 未授权: 拒绝

3. **重复检测**
   - 多版本依赖: 警告
   - 帮助优化依赖树

4. **源码验证**
   - 只允许来自 crates.io
   - 警告未知 Git 源

### Codecov (`codecov.yml`)

代码覆盖率配置：

- **项目目标**: 80% 覆盖率
- **补丁目标**: 80% 覆盖率
- **阈值**: 允许 1% 波动
- **忽略**: tests/, benches/, docs/, examples/
- **PR 评论**: 自动展示覆盖率差异

## 📊 CI 触发条件

### 自动触发

1. **Push 到分支**:
   - `main`
   - `master`
   - `claude/**`

2. **Pull Request**:
   - 目标分支: `main`, `master`

3. **定时任务**:
   - 每天 UTC 00:00
   - 检查依赖问题

4. **手动触发**:
   - workflow_dispatch (部分工作流)

## 🎯 CI 状态徽章

项目 README 包含以下徽章：

```markdown
[![CI](https://github.com/wayfind/intent-engine/workflows/CI/badge.svg)](...)
[![codecov](https://codecov.io/gh/wayfind/intent-engine/branch/main/graph/badge.svg)](...)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](...)
[![Crates.io](https://img.shields.io/crates/v/intent-engine.svg)](...)
[![Documentation](https://docs.rs/intent-engine/badge.svg)](...)
```

## 🚀 本地运行 CI 检查

详见 [CONTRIBUTING.md](../CONTRIBUTING.md#本地运行完整-ci-检查)

快速检查清单：

```bash
# 格式化
cargo fmt --all

# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# 测试
cargo test --all-features

# 文档
cargo doc --no-deps --all-features

# 安全审计
cargo audit
cargo deny check

# 包验证
cargo package --allow-dirty
```

## 📈 CI 性能优化

1. **智能缓存**
   - Cargo registry
   - Cargo git
   - 构建产物 (target/)

2. **并行执行**
   - 多平台并行测试
   - 独立 job 并行运行

3. **fail-fast: false**
   - 一个平台失败不影响其他
   - 获得完整的测试结果

4. **continue-on-error**
   - Nightly 版本允许失败
   - 基准测试允许失败
   - 不阻塞主要流程

## 🔧 维护建议

1. **定期更新**
   - 审查 Dependabot PR
   - 更新 Rust 工具链
   - 更新 GitHub Actions

2. **监控覆盖率**
   - 保持 80% 以上
   - 为新功能添加测试

3. **安全响应**
   - 及时处理 security-audit 警告
   - 更新有漏洞的依赖

4. **文档同步**
   - API 变更更新文档
   - 保持示例代码可运行

## 📚 相关文档

- [CONTRIBUTING.md](../CONTRIBUTING.md) - 贡献指南
- [README.md](../README.md) - 项目说明
- [cargo-deny 文档](https://embarkstudios.github.io/cargo-deny/)
- [Codecov 文档](https://docs.codecov.com/)

---

**CI 系统版本**: v2.0
**最后更新**: 2025-11-07
**维护者**: Intent Engine Team
