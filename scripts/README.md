# Development Scripts

这个目录包含开发辅助脚本。

## install/ie-manager.sh / ie-manager.ps1

从 GitHub Releases 安装、升级、卸载 `ie` CLI 的管理脚本。**无需 Rust 环境**。

### 功能

- **install** - 安装指定版本（默认最新）
- **upgrade** - 升级到指定版本（默认最新）
- **uninstall** - 卸载并可选删除数据

### 用法

```bash
# Linux/macOS
./scripts/install/ie-manager.sh install
./scripts/install/ie-manager.sh upgrade
./scripts/install/ie-manager.sh uninstall

# Windows PowerShell
.\scripts\install\ie-manager.ps1 install
.\scripts\install\ie-manager.ps1 upgrade
.\scripts\install\ie-manager.ps1 uninstall -Force
```

### 特性

- ✅ 自动检测平台 (Linux/macOS/Windows) 和架构 (x86_64/aarch64)
- ✅ SHA256 校验和验证
- ✅ 网络重试机制
- ✅ 语义版本比较（避免降级）
- ✅ 支持 `GITHUB_TOKEN` 环境变量（避免 API 限流）
- ✅ `--force/-y` 选项用于自动化

### 与 install.sh 的区别

| 脚本 | 安装方式 | 依赖 |
|------|----------|------|
| `install.sh` | 从源码/crates.io 编译 | 需要 Rust |
| `ie-manager.sh` | 下载预编译二进制 | 无需 Rust |

## setup-git-hooks.sh

自动安装 git pre-commit hooks，在每次提交前自动运行 `cargo fmt`。

### 安装

```bash
./scripts/setup-git-hooks.sh
```

### 效果

之后每次执行 `git commit` 时会自动：
1. 运行 `cargo fmt --all` 格式化所有代码
2. 如果有文件被修改，自动添加到当前提交
3. 完成提交

### 跳过 Hook

如果需要跳过格式化（不推荐）：
```bash
git commit --no-verify -m "message"
```

## verify-ie-binary.md

详细的 `ie` 二进制验证文档（497 行），包含：
- 二进制查找逻辑和测试用例
- Windows/Linux/macOS 平台验证
- CI 环境测试策略
- 故障排查指南

如需了解 `ie` 二进制的发现和验证机制，请参阅此文档。

## 使用 Makefile

项目根目录的 `Makefile` 提供了更多便捷命令：

```bash
make help          # 显示所有可用命令
make fmt           # 格式化代码
make check         # 运行格式化、clippy 和测试
make setup-hooks   # 安装 git hooks
```

## 推荐工作流

1. **首次克隆项目后**：
   ```bash
   ./scripts/setup-git-hooks.sh
   ```

2. **开发过程中**：
   - 编辑器会自动格式化（如果配置了）
   - 不需要手动运行 `cargo fmt`

3. **提交代码时**：
   ```bash
   git add <files>
   git commit -m "message"
   # hook 会自动运行 cargo fmt
   ```

4. **提交前检查**：
   ```bash
   make check  # 运行所有检查
   ```

## 为什么需要这个？

CI 中的 `cargo fmt --check` 会检查代码格式，如果不符合规范会失败。使用这些工具可以：
- ✅ 避免 CI 格式检查失败
- ✅ 保持代码风格一致
- ✅ 减少手动格式化的负担
- ✅ 提高开发效率
