# 发布到 crates.io 的解决方案

## 🔍 问题分析

**当前状态：**
- ✅ 本地有 v0.1.4 tag
- ❌ 远程**没有** v0.1.4 tag（推送被 403 阻止）
- ❌ GitHub Actions release workflow 未触发
- ❌ 因此没有发布到 crates.io

## 🎯 推荐方案：先合并 PR，再从 main 创建 tag

### 步骤 1: 创建并合并 PR

1. 访问：https://github.com/wayfind/intent-engine/compare/main...claude/improve-installation-experience-011CUv3p6NQmi6Xd5EKqJE1r
2. 创建 PR（使用 PR_DESCRIPTION.md 中的内容）
3. 等待 CI 通过
4. 合并 PR

### 步骤 2: 从 main 分支创建 tag

PR 合并后，从 main 分支创建 tag（权限可能不同）：

```bash
# 1. 切换到 main 并拉取最新代码
git checkout main
git pull origin main

# 2. 确认版本是 0.1.4
grep "^version" Cargo.toml

# 3. 创建 tag
git tag v0.1.4

# 4. 推送 tag
git push origin v0.1.4
```

如果从 main 推送 tag 仍然遇到 403，继续看方案 2。

---

## 🎯 方案 2: 通过 GitHub Web UI 创建 Release（推荐）

这个方法可以绕过 git push 的权限问题：

### 步骤：

1. **确保 PR 已合并到 main**

2. **访问 GitHub Releases 页面**：
   https://github.com/wayfind/intent-engine/releases/new

3. **填写表单**：
   - **Choose a tag**: 输入 `v0.1.4` 并选择 "Create new tag: v0.1.4 on publish"
   - **Target**: 选择 `main` 分支
   - **Release title**: `v0.1.4`
   - **Description**: 可以使用自动生成，或手动填写：

   ```markdown
   ## 🚀 v0.1.4 - 改进安装体验

   此版本大幅改进了安装体验，支持多种包管理器和安装方式。

   ### ✨ 新增功能

   - ✅ **cargo install** 支持 - 现在可以直接从 crates.io 安装
   - ✅ **Homebrew** 支持 - 提供 formula 和自动更新脚本
   - ✅ **cargo-binstall** 支持 - 快速安装预编译二进制
   - ✅ 完整的安装文档和测试指南

   ### 📦 安装方式

   ```bash
   # 从 crates.io 安装（推荐）
   cargo install intent-engine

   # 使用 cargo-binstall
   cargo binstall intent-engine

   # 或下载预编译二进制
   # 见下方 Assets
   ```

   ### 📚 文档

   - 完整安装指南：[INSTALLATION.md](https://github.com/wayfind/intent-engine/blob/main/INSTALLATION.md)
   - 发布测试指南：[docs/HOW_TO_TEST_RELEASE.md](https://github.com/wayfind/intent-engine/blob/main/docs/HOW_TO_TEST_RELEASE.md)
   ```

4. **发布**：
   - 点击 "Publish release"
   - 这会自动触发 release workflow

### 这会自动完成：
- ✅ 创建 v0.1.4 tag
- ✅ 构建所有平台的二进制
- ✅ 上传二进制到 Release
- ✅ 发布到 crates.io

---

## 🎯 方案 3: 手动发布到 crates.io（临时方案）

如果需要立即发布到 crates.io，可以手动操作：

```bash
# 1. 确保在正确的 commit（版本 0.1.4）
git log --oneline -1
# 应该看到：83371e3 Bump version to 0.1.4

# 2. 登录 crates.io
cargo login
# 输入你的 crates.io API token

# 3. 发布
cargo publish

# 4. 验证
cargo search intent-engine
```

**优点：** 立即发布
**缺点：** GitHub Release 和二进制需要单独创建

---

## 🎯 方案 4: 使用 workflow_dispatch 手动触发

如果 release workflow 支持手动触发（需要添加配置）：

```yaml
# 在 .github/workflows/release.yml 中添加
on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:  # 添加这个
    inputs:
      version:
        description: 'Version to release (e.g., 0.1.4)'
        required: true
```

然后可以在 GitHub Actions 页面手动触发。

---

## 📋 推荐执行顺序

### 最佳实践流程：

1. ✅ **创建并合并 PR**
   - 让代码通过 CI 验证
   - 确保代码在 main 分支上

2. ✅ **通过 Web UI 创建 Release**（方案 2）
   - 最可靠的方式
   - 自动触发所有流程
   - 无需处理权限问题

3. ✅ **验证发布**：
   ```bash
   # 等待几分钟后验证
   cargo search intent-engine
   cargo install intent-engine
   ie --version
   ```

---

## 🚨 故障排除

### 如果 crates.io 发布失败：

1. **检查 GitHub Actions 日志**：
   https://github.com/wayfind/intent-engine/actions

2. **查看 publish-crates-io job 的输出**：
   ```
   可能的错误：
   - "error: failed to authenticate" → Token 无效
   - "error: crate version already exists" → 版本号冲突
   - "error: not allowed to upload" → 权限问题
   ```

3. **验证 Secret 设置**：
   https://github.com/wayfind/intent-engine/settings/secrets/actions
   - 确认 `CARGO_REGISTRY_TOKEN` 存在
   - 如果需要，重新生成 token

### 如果需要重新发布：

```bash
# 1. 删除本地 tag
git tag -d v0.1.4

# 2. 删除远程 tag（如果存在）
git push origin :refs/tags/v0.1.4

# 3. 重新创建 Release（通过 Web UI）
```

---

## 📊 检查清单

在发布前确认：

- [ ] PR 已合并到 main
- [ ] Cargo.toml 版本为 0.1.4
- [ ] `CARGO_REGISTRY_TOKEN` secret 已设置
- [ ] 选择了发布方式（推荐方案 2）
- [ ] 准备好 Release 描述

---

## 🎯 立即行动

**现在就做：**
1. 创建 PR：https://github.com/wayfind/intent-engine/compare/main...claude/improve-installation-experience-011CUv3p6NQmi6Xd5EKqJE1r
2. 等待合并
3. 通过 Web UI 创建 Release：https://github.com/wayfind/intent-engine/releases/new
