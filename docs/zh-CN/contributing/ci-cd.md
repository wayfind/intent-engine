# CI/CD 系统文档

## 🎯 概述

Intent-Engine 使用现代化的 CI/CD 系统，旨在提供快速反馈和全面的质量检查。

### 设计原则

1. **快速反馈**: PR 检查在 5 分钟内完成
2. **全面覆盖**: 主分支上的完整平台测试
3. **自动化优先**: 最少的人工干预
4. **清晰分离**: 不同目的使用不同的工作流

---

## 🔄 本地 CI 流程

### Pre-commit Hook

项目配置了 pre-commit hook，在每次提交时自动运行以下检查：

1. **代码格式化** (`cargo fmt`)
2. **Clippy 检查** (`cargo clippy`)
3. **可选 UI 测试** (默认跳过)
4. **版本一致性检查**
5. **文档版本占位符替换**

### 可选 UI 测试

**默认行为**: UI/Dashboard 集成测试在提交时被跳过，以保持流程快速。

#### 启用 UI 测试

使用环境变量 `INTENT_RUN_UI_TESTS` 控制是否运行 UI 测试：

```bash
# 单次提交启用 UI 测试
INTENT_RUN_UI_TESTS=1 git commit -m "你的提交信息"

# 或者为整个会话设置
export INTENT_RUN_UI_TESTS=1
git commit -m "你的提交信息"

# 禁用
unset INTENT_RUN_UI_TESTS
```

#### 手动运行 UI 测试

```bash
# 运行所有 Dashboard 集成测试
cargo test --test dashboard_integration_tests --all-features

# 运行特定测试
cargo test --test dashboard_integration_tests test_name --all-features
```

#### 为什么默认跳过？

1. **速度**: Dashboard 集成测试需要启动服务器，比单元测试慢得多
2. **依赖**: 可能需要额外的系统依赖（如浏览器）
3. **频率**: 大多数代码更改不影响 UI，不需要每次都运行

#### 何时应该启用？

- 修改了 Dashboard 相关代码（`src/dashboard/`）
- 修改了 Web 前端模板（`static/`）
- 准备发布新版本
- 修复 UI 相关的 bug

---

## 🎯 贡献者最佳实践

### 提交前检查清单

1. **格式化和检查**:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features
   cargo test
   ```

2. **UI 测试（如需要）**:
   ```bash
   # 启用 UI 测试的提交
   INTENT_RUN_UI_TESTS=1 git commit -m "feat: add dashboard feature"
   ```

3. **提交消息**: 使用 conventional commits
   ```bash
   feat: 添加新功能
   fix: 修复问题
   docs: 更新文档
   ```

---

## 📊 GitHub Actions CI

### Pull Request 检查（快速 ~3-5 分钟）

创建 PR 时自动运行：

```yaml
✓ 格式检查       (cargo fmt)
✓ Clippy 检查    (cargo clippy)
✓ 快速测试       (Ubuntu/stable)
✓ 文档生成       (cargo doc)
✓ 依赖审查
✓ 自动标签
```

### Main 分支（完整测试 ~15-20 分钟）

合并到 main 后：

```yaml
✓ 跨平台测试
  ├── Linux (stable, beta)
  ├── macOS (stable)
  ├── Windows (stable)
  └── Linux nightly (experimental)

✓ 包验证
✓ 代码覆盖率上传
```

---

## 🔧 调试失败的 CI

### 格式失败

```bash
# 本地修复
cargo fmt --all

# 提交前检查
cargo fmt --all -- --check
```

### Clippy 失败

```bash
# 本地修复
cargo clippy --all-targets --all-features --fix

# 检查
cargo clippy --all-targets --all-features -- -D warnings
```

### 测试失败

```bash
# 本地运行所有测试
cargo test --verbose

# 运行特定测试
cargo test test_name

# 显示输出
cargo test -- --nocapture
```

### UI 测试失败

```bash
# 运行 Dashboard 集成测试
cargo test --test dashboard_integration_tests --all-features

# 带详细输出
cargo test --test dashboard_integration_tests --all-features -- --nocapture

# 运行特定 UI 测试
cargo test --test dashboard_integration_tests test_dashboard_home --all-features
```

---

## 📝 环境变量参考

### Pre-commit Hook

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `INTENT_RUN_UI_TESTS` | `0` (关闭) | 设置为 `1` 或 `true` 启用 UI 测试 |

### 示例使用

```bash
# Windows PowerShell
$env:INTENT_RUN_UI_TESTS=1
git commit -m "feat: dashboard update"

# Linux/macOS
export INTENT_RUN_UI_TESTS=1
git commit -m "feat: dashboard update"

# 一次性使用（Linux/macOS）
INTENT_RUN_UI_TESTS=1 git commit -m "feat: dashboard update"
```

---

## 🆘 故障排除

### Pre-commit Hook 未运行

```bash
# 检查 hook 是否存在
ls -la .git/hooks/pre-commit

# 确保可执行
chmod +x .git/hooks/pre-commit

# 手动运行
.git/hooks/pre-commit
```

### UI 测试超时

1. 检查是否有其他进程占用端口 3030-3099
2. 增加测试超时时间
3. 检查系统资源（内存、CPU）

### 数据库冲突

UI 测试使用隔离的临时目录，如果遇到数据库问题：

```bash
# 清理可能残留的测试数据
rm -rf /tmp/.tmp*/.intent-engine

# 重新运行测试
cargo test --test dashboard_integration_tests --all-features
```

---

**最后更新**: 2025-11-17
**系统版本**: 2.1 (添加可选 UI 测试)
