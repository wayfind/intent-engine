# 如何测试 crates.io 自动发布

## 📋 测试结果总结

✅ **本地验证已完成：**
- ✅ Git 工作区干净
- ✅ 打包成功（58 个文件）
- ✅ Dry-run 发布测试通过
- ✅ 包已准备好发布到 crates.io

## 🎯 现在你有三种测试方式

---

### 方式 1: 使用测试脚本（推荐）

运行自动化测试脚本：

```bash
./scripts/test-release.sh
```

**这个脚本会检查：**
1. Git 工作区状态
2. 当前版本号
3. 本地打包
4. Dry-run 发布
5. GitHub Secret（如果安装了 gh CLI）
6. 提供下一步操作指引

---

### 方式 2: 验证 GitHub Secret

#### 选项 A: 使用 gh CLI（推荐）

```bash
# 安装 gh CLI（如果还没有）
# macOS: brew install gh
# Linux: 见 https://github.com/cli/cli#installation

# 登录 GitHub
gh auth login

# 查看 secrets
gh secret list

# 应该看到：
# CARGO_REGISTRY_TOKEN  Updated YYYY-MM-DD
```

#### 选项 B: 手动在 Web UI 验证

1. 访问：https://github.com/wayfind/intent-engine/settings/secrets/actions
2. 检查是否有 `CARGO_REGISTRY_TOKEN`
3. 确认设置时间是否正确

#### 选项 C: 验证 Token 有效性（可选）

如果想确保 token 是有效的：

```bash
# 在本地测试登录（需要真实的 token）
cargo login

# 或者测试查询权限
cargo owner --list intent-engine
```

---

### 方式 3: 模拟完整发布流程（不推荐首次）

⚠️ **注意：这会创建真实的 GitHub Release，但不会发布到 crates.io（因为版本已存在）**

```bash
# 1. 查看当前分支和 tags
git branch
git tag -l

# 2. 创建一个测试 tag（使用已存在的版本）
git tag v0.1.3-test

# 3. 推送 tag（这会触发 workflow）
git push origin v0.1.3-test

# 4. 立即查看 Actions
# 方式 A: 使用 gh CLI
gh run list --workflow=release.yml --limit 5

# 方式 B: 访问 Web UI
# https://github.com/wayfind/intent-engine/actions

# 5. 查看实时日志（使用 gh CLI）
gh run watch

# 6. 测试完成后删除 test tag
git tag -d v0.1.3-test
git push origin :refs/tags/v0.1.3-test
```

---

## 🚀 真实发布流程（生产环境）

当你准备好发布新版本时：

### 步骤 1: 更新版本号

```bash
# 编辑 Cargo.toml
vim Cargo.toml
# 修改: version = "0.1.4"

# 提交更改
git add Cargo.toml
git commit -m "Bump version to 0.1.4"
git push
```

### 步骤 2: 创建并推送 tag

```bash
# 创建 tag
git tag v0.1.4

# 推送 tag（这会触发自动发布）
git push origin v0.1.4
```

### 步骤 3: 监控发布流程

```bash
# 使用 gh CLI 实时查看
gh run watch

# 或者访问 Web UI
# https://github.com/wayfind/intent-engine/actions
```

**期望看到的步骤：**

1. ✅ **Build** - 为所有平台构建二进制
   - Linux x86_64, ARM64
   - macOS x86_64, ARM64
   - Windows x86_64

2. ✅ **Create Release** - 创建 GitHub Release
   - 上传所有二进制文件
   - 生成 release notes

3. ✅ **Publish to crates.io** - 发布到 crates.io
   - 使用 CARGO_REGISTRY_TOKEN 登录
   - 执行 `cargo publish`

### 步骤 4: 验证发布成功

```bash
# 1. 检查 crates.io
cargo search ie --limit 1

# 应该看到新版本：
# intent-engine = "0.1.4"    # A command-line database service...

# 2. 测试安装
cargo install ie --force

# 3. 验证版本
ie --version
# 应该输出: intent-engine 0.1.4

# 4. 检查 GitHub Release
# https://github.com/wayfind/intent-engine/releases
```

### 步骤 5: 后续操作

```bash
# 1. 更新 Homebrew formula
./scripts/update-homebrew-formula.sh 0.1.4

# 2. 测试 cargo-binstall
cargo binstall ie --force

# 3. 发布公告（可选）
# - 在 GitHub Discussions 发布
# - 在社交媒体分享
# - 更新文档
```

---

## 🔍 监控和调试

### 查看 Workflow 运行历史

```bash
# 列出最近的 runs
gh run list --workflow=release.yml --limit 10

# 查看特定 run 的详情
gh run view <run-id>

# 查看完整日志
gh run view <run-id> --log

# 下载日志
gh run download <run-id>
```

### 常见问题排查

#### 1. Workflow 没有触发

**检查：**
```bash
# 确认 tag 格式正确（必须以 v 开头）
git tag -l

# 确认 tag 已推送到远程
git ls-remote --tags origin

# 确认 workflow 文件在正确的分支
git show origin/main:.github/workflows/release.yml
```

#### 2. crates.io 发布失败

**检查日志中的错误：**
```bash
gh run view --log | grep -A 10 "Publish to crates.io"
```

**可能的原因：**
- ❌ Token 无效或过期 → 重新生成并更新 Secret
- ❌ 版本号已存在 → 使用新的版本号
- ❌ 没有发布权限 → 检查 token 权限
- ❌ 包名已被占用 → 更改包名（不太可能）

#### 3. 构建失败

**查看构建日志：**
```bash
gh run view --log | grep -A 20 "error:"
```

---

## 📊 发布检查清单

在正式发布前，确保：

- [ ] 所有测试通过 (`cargo test`)
- [ ] Dry-run 成功 (`cargo publish --dry-run`)
- [ ] Git 工作区干净
- [ ] 版本号已更新且符合语义化版本规范
- [ ] CHANGELOG.md 已更新（如果有）
- [ ] GitHub Secret `CARGO_REGISTRY_TOKEN` 已设置
- [ ] 文档已更新
- [ ] CI 在 main 分支通过

---

## 🎉 成功标志

发布成功后，你应该看到：

✅ **GitHub Actions:**
- 所有步骤都是绿色 ✓
- 没有错误或警告

✅ **GitHub Releases:**
- 新的 release 出现在 https://github.com/wayfind/intent-engine/releases
- 所有平台的二进制文件都已上传

✅ **crates.io:**
- 新版本出现在 https://crates.io/crates/intent-engine
- 可以通过 `cargo search` 找到
- 可以通过 `cargo install` 安装

✅ **cargo-binstall:**
- 可以通过 `cargo binstall` 安装

---

## 📚 相关文档

- [TESTING_RELEASE.md](TESTING_RELEASE.md) - 详细的测试指南
- [INSTALLATION.md](../INSTALLATION.md) - 完整的安装指南
- [README.md](../README.md) - 项目主文档

---

## 💡 提示

1. **首次发布时**，建议先发布一个小的 patch 版本测试整个流程
2. **使用语义化版本**：major.minor.patch
   - patch (0.1.3 → 0.1.4): 修复 bug
   - minor (0.1.4 → 0.2.0): 新功能，向后兼容
   - major (0.2.0 → 1.0.0): 破坏性更改
3. **发布前**运行 `./scripts/test-release.sh` 确保一切就绪
4. **监控 Actions** 以便及时发现问题
5. **验证安装** 确保用户能够正常使用

祝发布顺利！🚀
