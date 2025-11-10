# 手动构建指南

## 🎯 概述

Intent-Engine 提供了灵活的手动构建系统，允许你在GitHub Actions中按需触发各种构建和测试任务。

## 🚀 快速开始

### 方法1：使用Manual Build工作流 (推荐)

这是最灵活的方式，提供多种构建选项。

**步骤：**

1. 访问GitHub仓库：https://github.com/wayfind/intent-engine
2. 点击顶部的 **"Actions"** 标签
3. 在左侧列表中选择 **"Manual Build"** workflow
4. 点击右上角的 **"Run workflow"** 按钮
5. 填写构建选项（见下方详细说明）
6. 点击绿色的 **"Run workflow"** 按钮
7. 等待构建完成

### 方法2：使用CI工作流

触发完整的CI流程，包括所有综合测试。

**步骤：**

1. 访问GitHub仓库
2. 点击 **"Actions"** → **"CI"**
3. 点击 **"Run workflow"**
4. 选择是否运行综合测试
5. 点击 **"Run workflow"**

## 📋 Manual Build 选项详解

### 构建类型 (Build Type)

| 选项 | 说明 | 用途 | 时间 |
|------|------|------|------|
| **quick-check** | 快速检查 | 格式化、clippy、编译检查 | ~2分钟 |
| **full-test** | 完整测试 | 运行所有测试套件（含ignored测试） | ~5-10分钟 |
| **cross-platform** | 跨平台构建 | 测试多个平台兼容性 | ~15-20分钟 |
| **release-build** | Release构建 | 生成优化的发布版二进制 | ~5分钟 |
| **bench** | 性能测试 | 运行所有性能基准测试 | ~10-15分钟 |
| **coverage** | 代码覆盖率 | 生成测试覆盖率报告 | ~5-10分钟 |
| **security-audit** | 安全审计 | 检查依赖的安全漏洞 | ~2分钟 |
| **all** | 全部运行 | 运行上述所有检查 | ~30-40分钟 |

### Rust版本

| 选项 | 说明 |
|------|------|
| **stable** | 稳定版 (推荐，默认) |
| **beta** | Beta版（测试即将发布的特性） |
| **nightly** | Nightly版（测试最新特性） |
| **1.75** | 最小支持版本（MSRV测试） |

### 目标平台 (Platform)

仅对 **cross-platform** 构建类型生效。

| 选项 | 说明 |
|------|------|
| **all** | 所有平台（默认） |
| **linux-x64** | Linux x86_64 |
| **linux-arm64** | Linux ARM64 |
| **macos-x64** | macOS Intel |
| **macos-arm64** | macOS Apple Silicon |
| **windows-x64** | Windows x86_64 |

### 其他选项

- **详细输出 (Verbose)**: 启用详细的构建日志（用于调试）
- **跳过缓存 (Skip Cache)**: 强制重新下载所有依赖（用于调试缓存问题）

## 💡 常见使用场景

### 场景1：快速验证代码格式和编译

```yaml
构建类型: quick-check
Rust版本: stable
详细输出: false
跳过缓存: false
```

**用途**: 在PR之前快速检查代码质量

### 场景2：测试特定平台

```yaml
构建类型: cross-platform
Rust版本: stable
平台: macos-arm64
详细输出: false
跳过缓存: false
```

**用途**: 验证macOS Apple Silicon的兼容性

### 场景3：调试构建问题

```yaml
构建类型: release-build
Rust版本: stable
详细输出: true ✅
跳过缓存: true ✅
```

**用途**: 排查构建失败原因，查看完整日志

### 场景4：发布前验证

```yaml
构建类型: all
Rust版本: stable
平台: all
详细输出: false
跳过缓存: false
```

**用途**: 发布新版本前的完整验证

### 场景5：测试Beta版Rust兼容性

```yaml
构建类型: full-test
Rust版本: beta ✅
详细输出: false
跳过缓存: false
```

**用途**: 提前测试即将发布的Rust版本

### 场景6：安全审计

```yaml
构建类型: security-audit
Rust版本: stable
详细输出: false
跳过缓存: false
```

**用途**: 检查依赖是否有已知安全漏洞

## 📊 查看构建结果

### 实时查看

1. 构建启动后，在Actions页面点击workflow运行
2. 查看各个job的实时日志
3. 点击job名称展开详细步骤

### 查看构建产物

对于 **release-build** 类型：

1. 构建完成后，进入workflow运行页面
2. 滚动到底部的 **"Artifacts"** 部分
3. 下载 `intent-engine-<version>-linux-x64.zip`
4. 解压后可以直接使用二进制文件

**产物包含**:
- `intent-engine` - 统一二进制（包含CLI和MCP服务器）
  - CLI模式: `intent-engine task add ...`
  - MCP服务器模式: `intent-engine mcp-server`

**保留期**: 7天

## 🔍 调试失败的构建

### 步骤1：启用详细输出

```yaml
详细输出: true ✅
```

### 步骤2：检查具体失败的步骤

点击失败的job → 展开失败的步骤 → 查看错误信息

### 步骤3：如果是缓存问题

```yaml
跳过缓存: true ✅
```

### 步骤4：在本地复现

根据workflow中的命令，在本地运行相同的命令：

```bash
# 例如，如果clippy失败
cargo clippy --all-targets --all-features -- -D warnings

# 如果特定平台测试失败
cargo build --release --target aarch64-unknown-linux-gnu
```

## ⚡ 性能优化建议

### 快速迭代

使用 **quick-check** 进行快速验证：

```yaml
构建类型: quick-check
```

只需2分钟即可获得反馈。

### 并行构建

多个构建可以同时运行，例如：

1. 启动 `quick-check` 用于快速验证
2. 同时启动 `cross-platform` 测试特定平台
3. 最后启动 `all` 进行完整验证

### 使用缓存

保持 **跳过缓存** 为 `false`（默认）可以：
- 大幅减少构建时间（5-10x速度提升）
- 节省GitHub Actions分钟数

仅在调试缓存相关问题时才启用 `skip_cache: true`。

## 📈 构建时间参考

基于实际测量（Ubuntu Latest, stable Rust）：

| 构建类型 | 首次（无缓存） | 有缓存 |
|---------|--------------|--------|
| quick-check | ~5分钟 | ~2分钟 |
| full-test | ~10分钟 | ~5分钟 |
| cross-platform (all) | ~40分钟 | ~20分钟 |
| release-build | ~10分钟 | ~5分钟 |
| bench | ~20分钟 | ~15分钟 |
| coverage | ~15分钟 | ~10分钟 |
| security-audit | ~3分钟 | ~2分钟 |
| all | ~60分钟 | ~35分钟 |

## 🔐 权限要求

### 读取权限

所有贡献者都可以：
- ✅ 查看workflow运行状态
- ✅ 查看构建日志
- ✅ 下载构建产物

### 写入权限

需要write权限才能：
- ❌ 触发手动构建（需要push权限）
- ❌ 重新运行失败的job

如果你需要触发构建但没有权限：
1. Fork仓库到你的账号
2. 在你的fork中运行workflow
3. 或者请求maintainer运行构建

## 📞 获取帮助

### 构建失败了怎么办？

1. 检查错误日志中的具体错误信息
2. 尝试启用 **详细输出** 重新运行
3. 在本地复现问题
4. 查看 [CI调试指南](./CI_DEBUGGING.md)（如果存在）
5. 提交Issue: https://github.com/wayfind/intent-engine/issues

### 想添加新的构建选项？

编辑 `.github/workflows/manual-build.yml` 并提交PR。

## 🎓 最佳实践

### ✅ 推荐做法

```
1. 开发时：使用 quick-check 快速验证
2. PR前：运行 full-test 确保所有测试通过
3. 发布前：运行 all 进行完整验证
4. 调试时：启用 verbose 和 skip_cache
5. 定期：运行 security-audit 检查安全问题
```

### ❌ 避免做法

```
1. 不要频繁运行 all（浪费资源和时间）
2. 不要总是启用 skip_cache（除非调试）
3. 不要忽略 quick-check 的失败
4. 不要在PR中运行 bench（耗时且不稳定）
```

## 🔄 与自动CI的关系

### 自动CI (推送时触发)

```yaml
触发: 每次push到main/claude/**分支
运行: fast-check (2分钟)
```

快速反馈，捕获95%的问题。

### 手动构建 (按需触发)

```yaml
触发: 手动
运行: 任何你选择的配置
```

灵活测试特定场景。

### 定时CI (每天UTC 10:00)

```yaml
触发: 定时（如果有新提交）
运行: 所有综合测试 (30-40分钟)
```

深度验证，创建Issue如果失败。

**三者互补，确保代码质量！**

## 📚 相关文档

- [CI配置文件](.github/workflows/ci.yml)
- [Manual Build配置](.github/workflows/manual-build.yml)
- [格式化指南](./FORMATTING_GUIDE.md)
- [贡献指南](../docs/zh-CN/contributing/contributing.md)

---

**Pro Tip**: 将此页面加入书签，方便随时查阅构建选项！
