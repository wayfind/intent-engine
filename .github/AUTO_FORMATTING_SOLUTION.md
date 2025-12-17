# 自动格式化完整解决方案

## 🎯 目标

**从根本上杜绝格式化问题**，确保所有代码在提交前自动格式化，无需手动干预。

## 🔧 多层防护方案

### 第1层：Claude Code 会话自动安装 (Session Hook)

**文件**: `.claude-code/SessionStart`

每次Claude Code启动新会话时自动运行，确保hooks总是被安装。

```bash
#!/bin/bash
# 自动在每个Claude Code session开始时安装hooks
./scripts/auto-setup-hooks.sh
```

**优点**:
- ✅ 对Claude Code用户完全自动化
- ✅ 无需记住手动运行安装命令
- ✅ 每次session都会检查并安装

### 第2层：Git Pre-Commit Hook (本地强制)

**文件**: `.git/hooks/pre-commit` (由脚本生成)

每次 `git commit` 时自动运行 `cargo fmt --all`。

```bash
#!/bin/sh
echo "Running cargo fmt..."
cargo fmt --all

if ! git diff --quiet; then
    echo "✓ Code formatted. Adding changes to commit..."
    git diff --name-only | grep '\.rs$' | xargs -r git add
fi
```

**优点**:
- ✅ 提交时自动格式化
- ✅ 格式化后的更改自动添加到提交
- ✅ 无法提交未格式化的代码

### 第3层：CI 格式检查 (云端验证)

**文件**: `.github/workflows/ci.yml`

```yaml
- name: Check formatting
  run: cargo fmt --all -- --check
```

**优点**:
- ✅ 最后一道防线
- ✅ 检测所有平台的格式问题
- ✅ 防止绕过本地hooks的提交

### 第4层：格式化配置 (统一标准)

**文件**: `rustfmt.toml`

```toml
edition = "2021"
max_width = 100
match_block_trailing_comma = true
use_try_shorthand = true
use_field_init_shorthand = true
force_explicit_abi = true
```

**优点**:
- ✅ 所有开发者使用相同格式规则
- ✅ 只使用stable rustfmt特性
- ✅ CI和本地格式化100%一致

## 📁 文件结构

```
intent-engine/
├── .claude-code/
│   └── SessionStart           # Claude Code会话启动时自动运行
├── .github/
│   ├── workflows/
│   │   └── ci.yml             # CI格式检查
│   ├── FORMATTING_GUIDE.md    # 格式化指南
│   └── AUTO_FORMATTING_SOLUTION.md  # 本文档
├── scripts/
│   ├── setup-git-hooks.sh     # 手动安装hooks
│   ├── auto-setup-hooks.sh    # 自动安装hooks (检查是否已安装)
│   └── check-format.sh        # 格式检查脚本(带友好错误信息)
├── rustfmt.toml               # 格式化配置
└── Makefile                   # 便捷命令
```

## 🚀 工作流程

### Claude Code 用户 (推荐)

```bash
# 1. 克隆仓库
git clone <repo>
cd intent-engine

# 2. 启动Claude Code
# SessionStart hook会自动安装格式化hooks

# 3. 开发
vim src/some_file.rs

# 4. 提交 (自动格式化)
git commit -m "feat: Add feature"

# 5. 推送
git push
```

**完全自动！无需任何手动格式化命令。**

### 其他用户 (手动设置一次)

```bash
# 1. 克隆仓库
git clone <repo>
cd intent-engine

# 2. 安装hooks (仅需一次)
make setup-hooks

# 3-5. 同上
```

## 🔍 验证安装

### 检查hooks是否安装

```bash
# 方法1: 检查文件存在
ls -la .git/hooks/pre-commit

# 方法2: 检查内容
cat .git/hooks/pre-commit | grep "cargo fmt"

# 方法3: 测试格式化
echo "fn test(){}" >> src/test_format.rs
git add src/test_format.rs
git commit -m "test"  # 应该看到 "Running cargo fmt..."
git reset HEAD~1  # 撤销测试提交
rm src/test_format.rs
```

### 检查格式化配置

```bash
# 验证rustfmt.toml被使用
cargo fmt -- --print-config current

# 手动格式化测试
cargo fmt --all
git status  # 不应该有变更
```

## ❓ 常见问题

### 问: SessionStart hook不执行？

**可能原因**:
1. 没有执行权限
2. 脚本路径错误
3. Claude Code未启用hooks

**解决方案**:
```bash
# 1. 检查权限
ls -la .claude-code/SessionStart

# 2. 添加执行权限
chmod +x .claude-code/SessionStart

# 3. 手动运行测试
./.claude-code/SessionStart
```

### 问: Pre-commit hook被跳过？

**可能原因**:
使用了 `git commit --no-verify`

**解决方案**:
不要使用 `--no-verify`，如果必须使用，记得在推送前运行 `cargo fmt --all`

### 问: CI格式检查失败但本地通过？

**可能原因**:
1. 使用了不同的rustfmt版本
2. rustfmt.toml有不稳定特性

**解决方案**:
```bash
# 1. 检查rustfmt版本
rustfmt --version

# 2. 确保使用stable版本
rustup default stable
cargo fmt --all

# 3. 重新提交
git add .
git commit --amend --no-edit
git push -f
```

### 问: 如何临时禁用自动格式化？

**不推荐，但如果真的需要**:
```bash
# 重命名hook临时禁用
mv .git/hooks/pre-commit .git/hooks/pre-commit.disabled

# 提交后恢复
mv .git/hooks/pre-commit.disabled .git/hooks/pre-commit
```

## 📊 方案对比

| 方案 | 自动化程度 | 适用场景 | 防护级别 |
|------|-----------|---------|---------|
| **SessionStart Hook** | ⭐⭐⭐⭐⭐ 完全自动 | Claude Code用户 | 高 |
| **Pre-commit Hook** | ⭐⭐⭐⭐ 半自动(需安装一次) | 所有Git用户 | 高 |
| **CI检查** | ⭐⭐⭐ 被动检查 | 所有贡献者 | 最高 |
| **手动运行** | ⭐ 手动 | 紧急情况 | 低 |

## 🎓 最佳实践

### ✅ 推荐

```bash
# Claude Code用户
1. 克隆项目 -> SessionStart自动安装hooks -> 开始开发

# 其他用户
1. 克隆项目 -> make setup-hooks -> 开始开发

# 提交前(可选但推荐)
make check  # 运行完整检查
```

### ❌ 避免

```bash
# 不要跳过hooks
git commit --no-verify

# 不要手动编辑格式
# rustfmt会处理一切

# 不要使用nightly-only特性
# rustfmt.toml只包含stable特性
```

## 🔄 维护

### 更新格式化规则

编辑 `rustfmt.toml`，只使用stable特性：

```bash
# 检查哪些特性是stable
rustfmt --help=config

# 测试新配置
cargo fmt --all
git status  # 查看变更
```

### 更新hooks

```bash
# 编辑 scripts/setup-git-hooks.sh
vim scripts/setup-git-hooks.sh

# 重新安装
make setup-hooks
```

## 📞 支持

如果遇到问题：

1. 查看 [FORMATTING_GUIDE.md](.github/FORMATTING_GUIDE.md)
2. 运行 `./scripts/check-format.sh` 查看详细错误
3. 提交Issue: https://github.com/wayfind/intent-engine/issues

## 🎯 总结

这个多层防护方案确保：

1. **Claude Code用户**: 完全自动，无需任何手动操作
2. **其他用户**: 一次性设置后自动化
3. **CI**: 最后防线，捕获所有遗漏

**核心理念**: 让格式化成为无感知的自动化流程，而不是需要记住的手动任务。
