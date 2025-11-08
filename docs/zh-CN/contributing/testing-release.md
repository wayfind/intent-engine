# 测试 crates.io 发布流程指南

## 方法 1: 本地 Dry-run 测试（推荐先做）

在实际发布前，先在本地测试打包是否正常：

```bash
# 1. 检查 Cargo.toml 配置是否正确
cargo package --list

# 2. 测试打包（不会真正发布）
cargo package --allow-dirty

# 3. 验证打包后的内容
tar -tzf target/package/intent-engine-0.1.3.crate

# 4. Dry-run 发布（不会真正发布到 crates.io）
cargo publish --dry-run --allow-dirty
```

**预期结果：**
- ✅ 所有文件正确包含
- ✅ 没有错误或警告
- ✅ crates.io 验证通过

---

## 方法 2: 验证 GitHub Secret 是否正确设置

```bash
# 查看当前仓库的 secrets（需要 gh CLI）
gh secret list

# 或者在 GitHub Web UI 查看
# https://github.com/wayfind/intent-engine/settings/secrets/actions
```

**预期看到：**
```
CARGO_REGISTRY_TOKEN  Updated YYYY-MM-DD
```

---

## 方法 3: 手动触发测试发布（不推荐首次使用）

⚠️ **注意：这会真正发布到 crates.io，且无法撤销！**

### 选项 A: 发布 Patch 版本（安全测试）

```bash
# 1. 创建一个测试 patch 版本
# 编辑 Cargo.toml，将版本改为 0.1.4-test 或 0.1.4

# 2. 提交更改
git add Cargo.toml
git commit -m "Bump version to 0.1.4 for testing"
git push

# 3. 创建 tag 触发 release
git tag v0.1.4
git push origin v0.1.4
```

### 选项 B: 使用 workflow_dispatch（如果启用）

如果你的 workflow 支持手动触发：

```bash
# 使用 gh CLI
gh workflow run release.yml
```

---

## 方法 4: 监控自动发布流程

当你推送 tag 后，可以实时查看 workflow 执行：

```bash
# 查看最近的 workflow runs
gh run list --workflow=release.yml

# 查看特定 run 的日志
gh run view <run-id> --log

# 或在 Web UI 查看
# https://github.com/wayfind/intent-engine/actions
```

**关键步骤检查：**
1. ✅ Build 所有平台
2. ✅ Create Release（创建 GitHub Release）
3. ✅ Publish to crates.io（发布到 crates.io）

---

## 方法 5: 验证发布成功

发布完成后，验证：

### 检查 crates.io
```bash
# 搜索你的包
cargo search intent-engine --limit 1

# 或访问 Web
# https://crates.io/crates/intent-engine
```

### 测试安装
```bash
# 从 crates.io 安装
cargo install intent-engine

# 验证版本
intent-engine --version
```

### 检查 GitHub Release
访问：https://github.com/wayfind/intent-engine/releases

---

## 推荐的完整测试流程

### 阶段 1: 本地验证（安全）
```bash
# 1. Dry-run 测试
cargo publish --dry-run --allow-dirty

# 2. 检查输出，确保没有错误
# 如果有问题，修复后重新测试
```

### 阶段 2: Secret 验证
```bash
# 检查 secret 是否设置
gh secret list | grep CARGO_REGISTRY_TOKEN
```

### 阶段 3: 小版本测试（实际发布）
```bash
# 1. 确保当前分支干净
git status

# 2. 更新版本号（如 0.1.3 -> 0.1.4）
# 编辑 Cargo.toml: version = "0.1.4"

# 3. 提交并打 tag
git add Cargo.toml
git commit -m "Bump version to 0.1.4"
git push
git tag v0.1.4
git push origin v0.1.4

# 4. 观察 GitHub Actions
# 访问 https://github.com/wayfind/intent-engine/actions

# 5. 等待完成，然后验证
cargo search intent-engine
```

---

## 常见问题排查

### 1. crates.io 登录失败
```
error: failed to parse registry response
```
**原因：** Token 无效或过期
**解决：** 重新生成 token 并更新 GitHub Secret

### 2. 发布权限错误
```
error: not allowed to upload
```
**原因：** Token 没有发布权限
**解决：** 确保 token 有 "Publish new crates" 权限

### 3. 版本冲突
```
error: crate version `0.1.3` is already uploaded
```
**原因：** 版本号已存在
**解决：** 使用新的版本号

### 4. Workflow 未触发
**检查：**
- Tag 格式是否正确（必须是 `v*`）
- Workflow 文件是否在 main 分支
- GitHub Actions 是否启用

---

## 调试命令

```bash
# 查看本地 tags
git tag -l

# 查看远程 tags
git ls-remote --tags origin

# 查看 workflow 状态
gh run list --workflow=release.yml --limit 5

# 查看 workflow 详细日志
gh run view --log

# 查看最近的 commit
git log --oneline -5

# 验证 Cargo.toml
cargo verify-project
```

---

## 手动回滚（如果需要）

⚠️ **注意：crates.io 的版本无法删除，只能 yank**

```bash
# Yank 一个有问题的版本（不推荐使用）
cargo yank --vers 0.1.4 intent-engine

# Unyank
cargo yank --vers 0.1.4 --undo intent-engine
```

---

## 成功标志

✅ **发布成功的标志：**
1. GitHub Actions 所有步骤都是绿色 ✓
2. GitHub Releases 页面有新的 release
3. `cargo search intent-engine` 能找到新版本
4. crates.io 页面显示新版本
5. `cargo install intent-engine` 能安装成功

---

## 下一步

发布成功后：
1. 更新 Homebrew formula（运行 `./scripts/update-homebrew-formula.sh 0.1.4`）
2. 在 README 中移除 Homebrew 的"即将支持"标记
3. 通知用户新版本发布
