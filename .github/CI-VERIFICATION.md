# CI 优化验证指南

本文档帮助你验证三层CI策略是否正确实施和运行。

## 📊 三层CI策略概览

| 层级 | 触发条件 | 测试矩阵 | 耗时任务 | 预期时间 |
|------|---------|---------|---------|---------|
| 🚀 **快速检查** | `claude/**` 分支推送 | 1个配置 | ❌ 跳过 | ~5-10分钟 |
| ✅ **标准CI** | Pull Request | 5个配置 | ❌ 跳过 | ~15-20分钟 |
| 🔬 **完整CI** | main/master/定时/手动 | 8个配置 | ✅ 运行 | ~30-40分钟 |

## ✅ 验证检查清单

### 1. 验证快速检查（开发分支）

**触发方式**: 推送到 `claude/**` 分支

**预期行为**:
- ✅ **应该运行**:
  - Test (ubuntu-latest / stable) - 1个配置
  - Security Audit
  - Documentation
  - Check Package
  - Format check & Clippy（在Test job中）

- ❌ **应该跳过**:
  - Test (macos-latest / *) - 标准CI配置
  - Test (windows-latest / *) - 完整CI配置
  - Test (ubuntu-latest / nightly) - 完整CI配置
  - Code Coverage
  - Benchmarks
  - Test Minimal Versions
  - Install Scripts

**验证步骤**:
```bash
# 1. 查看你最近的CI运行
# 访问: https://github.com/wayfind/intent-engine/actions

# 2. 找到claude/**分支的推送触发的workflow运行

# 3. 检查运行的jobs数量
# 快速检查应该只运行约4-5个jobs，总时间5-10分钟
```

**预期结果**:
```
✅ Test (ubuntu-latest / stable)     - 通过
✅ Security Audit                     - 通过
✅ Documentation                      - 通过
✅ Check Package                      - 通过
✅ CI Success                         - 通过
⏭️  Code Coverage                     - 跳过
⏭️  Benchmarks                        - 跳过
⏭️  Test Minimal Versions             - 跳过
⏭️  Install Scripts                   - 跳过
```

---

### 2. 验证标准CI（Pull Request）

**触发方式**: 创建或更新Pull Request到 `main`/`master`

**预期行为**:
- ✅ **应该运行**:
  - Test (ubuntu-latest / stable)
  - Test (ubuntu-latest / beta)
  - Test (macos-latest / stable)
  - Test (macos-latest / beta)
  - Test (ubuntu-latest / stable) - tier: fast（总是运行）
  - Security Audit
  - Documentation
  - Check Package
  - Dependency Review

- ❌ **应该跳过**:
  - Test (windows-latest / *)
  - Test (ubuntu-latest / nightly)
  - Code Coverage
  - Benchmarks
  - Test Minimal Versions
  - Install Scripts

**验证步骤**:
```bash
# 1. 创建或查看PR
# 访问: https://github.com/wayfind/intent-engine/pulls

# 2. 检查CI运行
# 点击PR页面的"Checks"标签

# 3. 验证运行的测试配置数量
# 应该有5个Test jobs（ubuntu x2, macos x2, 加上tier:fast）
```

**预期结果**:
```
✅ Test (ubuntu-latest / stable)     - 通过 (tier: fast)
✅ Test (ubuntu-latest / beta)       - 通过 (tier: standard)
✅ Test (macos-latest / stable)      - 通过 (tier: standard)
✅ Test (macos-latest / beta)        - 通过 (tier: standard)
✅ Security Audit                     - 通过
✅ Documentation                      - 通过
✅ Check Package                      - 通过
✅ Dependency Review                  - 通过或跳过
✅ CI Success                         - 通过
⏭️  Code Coverage                     - 跳过
⏭️  Benchmarks                        - 跳过
⏭️  Test (windows-latest / *)        - 跳过
⏭️  Test (ubuntu-latest / nightly)   - 跳过
```

---

### 3. 验证完整CI（生产分支）

**触发方式**:
- 推送到 `main` 或 `master` 分支
- 每日定时任务（00:00 UTC）
- 手动触发（workflow_dispatch）

**预期行为**:
- ✅ **全部运行**:
  - Test - 8个配置（ubuntu/macos/windows × stable/beta + nightly）
  - Security Audit
  - Documentation
  - Check Package
  - Code Coverage
  - Test Minimal Versions
  - Benchmarks
  - Install Scripts (ubuntu + macos)

**验证步骤**:
```bash
# 方式1: 检查main分支推送
# 访问: https://github.com/wayfind/intent-engine/actions?query=branch%3Amain

# 方式2: 手动触发完整CI
# 1. 访问: https://github.com/wayfind/intent-engine/actions/workflows/ci.yml
# 2. 点击 "Run workflow" 按钮
# 3. 选择分支，点击 "Run workflow"

# 方式3: 检查定时任务
# 查看每日00:00 UTC的自动运行
```

**预期结果**:
```
✅ Test (ubuntu-latest / stable)     - 通过
✅ Test (ubuntu-latest / beta)       - 通过
✅ Test (macos-latest / stable)      - 通过
✅ Test (macos-latest / beta)        - 通过
✅ Test (windows-latest / stable)    - 通过
✅ Test (windows-latest / beta)      - 通过
🟡 Test (ubuntu-latest / nightly)    - 通过（允许失败）
✅ Security Audit                     - 通过
✅ Documentation                      - 通过
✅ Check Package                      - 通过
✅ Code Coverage                      - 通过
✅ Test Minimal Versions              - 通过
✅ Benchmarks                         - 通过（允许失败）
✅ Install Scripts (ubuntu-latest)   - 通过
✅ Install Scripts (macos-latest)    - 通过
✅ CI Success                         - 通过
```

---

## 🔍 详细验证方法

### 方法1: 通过GitHub Actions UI

1. 访问仓库的Actions页面:
   ```
   https://github.com/wayfind/intent-engine/actions
   ```

2. 选择一个workflow运行，检查:
   - **运行时长**: 快速检查 < 10分钟，标准CI < 20分钟，完整CI < 40分钟
   - **Jobs数量**: 快速检查 ≈ 5个，标准CI ≈ 10个，完整CI ≈ 15个
   - **跳过的jobs**: 查看哪些jobs被标记为"skipped"

### 方法2: 通过GitHub CLI

```bash
# 列出最近的workflow运行
gh run list --workflow=ci.yml --limit 5

# 查看特定运行的详情
gh run view <run-id>

# 查看运行的jobs
gh run view <run-id> --log
```

### 方法3: 检查workflow文件

验证 `.github/workflows/ci.yml` 中的关键配置:

```bash
# 检查测试矩阵配置
grep -A 30 "matrix:" .github/workflows/ci.yml

# 检查条件执行配置
grep -A 2 "if: " .github/workflows/ci.yml

# 验证tier字段
grep "tier:" .github/workflows/ci.yml
```

---

## 📈 性能对比

### 预期性能提升

| 场景 | 之前 | 现在 | 提升 |
|------|------|------|------|
| 开发分支推送 | ~30-40分钟 | ~5-10分钟 | **70-80% ↓** |
| Pull Request | ~30-40分钟 | ~15-20分钟 | **40-50% ↓** |
| Main分支 | ~30-40分钟 | ~30-40分钟 | 无变化（完整测试） |

### 测算示例

**每天10次推送到开发分支**:
- 之前: 10 × 40分钟 = 400分钟（6.7小时）
- 现在: 10 × 8分钟 = 80分钟（1.3小时）
- **节省**: 320分钟（5.3小时）

---

## 🐛 常见问题排查

### 问题1: 所有jobs都在运行（没有跳过）

**原因**: 条件判断可能有问题

**解决**:
```bash
# 检查tier字段是否正确
grep -B 2 "tier:" .github/workflows/ci.yml

# 确认if条件
grep -A 10 "# Conditional execution" .github/workflows/ci.yml
```

### 问题2: 快速检查运行太慢

**检查**:
- 是否有耗时任务（coverage, benchmarks）在运行？
- 缓存是否正常工作？
- 是否有网络问题导致依赖下载缓慢？

### 问题3: CI Success job失败

**原因**: 可能跳过的jobs被错误判断为失败

**解决**: 检查 `ci-success` job中的条件判断逻辑

---

## ✅ 验证成功标准

CI优化正确实施的标志:

- [ ] 开发分支推送在10分钟内完成
- [ ] PR运行约5个test配置
- [ ] Main分支运行完整测试（8个配置）
- [ ] 耗时任务仅在完整CI中运行
- [ ] 所有必要的安全检查始终运行
- [ ] CI Success job正确处理跳过的jobs

---

## 📞 需要帮助？

如果遇到问题:

1. 检查最近的workflow运行日志
2. 查看 `.github/workflows/ci.yml` 配置
3. 对比本文档中的预期行为
4. 在Issues中报告问题并附上workflow运行链接

---

**最后更新**: 2025-11-08
**适用版本**: CI workflow commit `ef50f02` 及之后
