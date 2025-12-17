# Formatting Guide - 从根本上解决格式问题

## 🎯 目标

确保所有代码在提交前都已正确格式化，避免CI格式检查失败。

## 📋 解决方案

### 1. 自动格式化 (推荐)

#### 首次设置（仅需一次）

```bash
# 方法1: 使用Makefile
make setup-hooks

# 方法2: 直接运行脚本
./scripts/setup-git-hooks.sh
```

这将安装pre-commit hook，**每次提交时自动运行** `cargo fmt`。

#### 工作流程

```bash
# 正常开发
vim src/some_file.rs

# 提交时自动格式化
git add .
git commit -m "message"  # <- 自动运行 cargo fmt

# 如果格式化后有变更，会自动添加到当前提交
```

### 2. 手动格式化

如果你跳过了hooks安装，可以手动格式化：

```bash
# 格式化所有代码
cargo fmt --all

# 或使用Makefile
make fmt
```

### 3. 提交前检查

推荐使用完整检查（格式 + clippy + 测试）：

```bash
make check
```

## 🔧 配置文件

项目使用 `rustfmt.toml` 确保格式一致：

```toml
edition = "2021"
max_width = 100
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
# ... 更多配置
```

所有贡献者和CI使用相同的配置。

## 🚫 常见问题

### 问题：CI格式检查失败

```
Error: Process completed with exit code 1.
Diff in /home/runner/work/.../src/some_file.rs:123:
```

**原因**：提交的代码未经格式化

**解决方案**：

```bash
# 1. 安装hooks（如果还没安装）
make setup-hooks

# 2. 格式化现有代码
cargo fmt --all

# 3. 修正提交
git add .
git commit --amend --no-edit  # 或创建新提交

# 4. 推送
git push -f origin <branch>  # 如果是amend，需要force push
```

### 问题：我忘记运行格式化了

如果你已经提交但忘记格式化：

```bash
# 格式化代码
cargo fmt --all

# 检查是否有变更
git status

# 如果有变更，修正最后一次提交
git add .
git commit --amend --no-edit
git push -f origin <branch>

# 或者创建新的格式化提交
git add .
git commit -m "chore: Format code with rustfmt"
git push origin <branch>
```

### 问题：想临时跳过格式检查

```bash
# 使用 --no-verify 跳过hooks（不推荐）
git commit --no-verify -m "message"

# 但你仍然需要在推送前格式化，否则CI会失败！
```

## 🔄 CI流程

CI会执行以下检查：

```yaml
- name: Check formatting
  run: cargo fmt --all --check

- name: Run clippy
  run: cargo clippy -- -D warnings

- name: Run tests
  run: cargo test
```

**所有检查必须通过才能合并PR。**

## 📚 最佳实践

### ✅ 推荐工作流

```bash
# 1. 首次克隆后立即设置
git clone <repo>
cd intent-engine
make setup-hooks

# 2. 开发
vim src/file.rs

# 3. 提交（自动格式化）
git add .
git commit -m "feat: Add new feature"

# 4. 推送前再次检查（可选但推荐）
make check

# 5. 推送
git push
```

### ❌ 避免的陷阱

```bash
# ❌ 不要跳过hooks
git commit --no-verify

# ❌ 不要手动编辑格式
# rustfmt会自动处理，不要试图"优化"它的输出

# ❌ 不要在未格式化的情况下直接推送
git push  # 没有运行 cargo fmt --all
```

## 🛠️ 开发者工具

### VSCode配置

在 `.vscode/settings.json` 添加：

```json
{
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true
  },
  "rust-analyzer.rustfmt.extraArgs": ["--config-path", "rustfmt.toml"]
}
```

### IntelliJ IDEA/CLion

1. Settings → Languages & Frameworks → Rust → Rustfmt
2. 勾选 "Run rustfmt on Save"
3. 设置 "Use rustfmt instead of built-in formatter"

### Vim/Neovim

使用 `rust.vim` 或 `rust-tools.nvim`：

```vim
let g:rustfmt_autosave = 1
```

## 📞 获取帮助

如果遇到格式化问题：

1. 检查是否安装了hooks: `ls -la .git/hooks/pre-commit`
2. 检查rustfmt版本: `rustfmt --version`
3. 重新安装hooks: `make setup-hooks`
4. 提交Issue: https://github.com/wayfind/intent-engine/issues

## 🎓 总结

**记住这一条规则**：

> 首次克隆项目后，立即运行 `make setup-hooks`，然后忘掉格式化 — hooks会自动处理一切！

hooks安装后，你无需手动运行 `cargo fmt`，也无需担心CI格式检查失败。
