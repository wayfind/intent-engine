# 🚀 快速触发Codecov - 30秒指南

## 最简单的方法（3步）

### 步骤1：访问Actions页面

```
https://github.com/wayfind/intent-engine/actions/workflows/codecov.yml
```

或者：
1. 打开仓库：https://github.com/wayfind/intent-engine
2. 点击顶部 **"Actions"** 标签
3. 在左侧找到 **"Code Coverage (Codecov)"**

### 步骤2：运行Workflow

点击右上角的 **"Run workflow"** 按钮

### 步骤3：确认并启动

保持默认设置（上传到Codecov: ✅），点击绿色的 **"Run workflow"** 按钮

**完成！** 🎉

---

## ⏱️ 预计时间

- **运行时间**: 3-5分钟
- **结果**:
  - ✅ 代码覆盖率报告生成
  - ✅ 自动上传到Codecov
  - ✅ 下载HTML报告（30天保留）

---

## 📊 查看结果

### 在GitHub Actions中查看

1. 等待workflow完成（绿色勾号）
2. 点击运行记录
3. 查看 **"Generate Coverage Report"** 步骤的输出
4. 在 **"Artifacts"** 部分下载HTML报告

### 在Codecov网站查看

访问：
```
https://codecov.io/gh/wayfind/intent-engine
```

---

## 🎛️ 高级选项

### 不上传到Codecov（仅生成本地报告）

```yaml
上传到Codecov: ❌ (取消勾选)
详细输出: ❌
```

### 调试模式（详细日志）

```yaml
上传到Codecov: ✅
详细输出: ✅ (勾选)
```

---

## 📥 下载HTML报告

1. Workflow完成后，滚动到页面底部
2. 在 **"Artifacts"** 部分找到:
   - `coverage-report` - LCOV文件
   - `coverage-html-report` - 完整HTML报告
3. 点击下载
4. 解压后打开 `index.html`

---

## 💡 提示

- **自动运行**: PR和push到main时会自动生成覆盖率
- **手动触发**: 随时可以手动运行获取最新数据
- **无需token**: Codecov token已配置在仓库secrets中

---

## 🔧 工具说明

此workflow使用 `cargo-llvm-cov`（现代Rust覆盖率工具）：

**优点**:
- ✅ 比tarpaulin更快
- ✅ 更准确的覆盖率数据
- ✅ 原生支持LLVM
- ✅ 生成多种格式（LCOV, HTML, JSON）

---

## ❓ 常见问题

### Q: 需要配置token吗？

A: 不需要！已经在仓库secrets中配置好了。

### Q: 可以看到具体哪些代码没覆盖吗？

A: 可以！下载HTML报告，打开后可以看到每行代码的覆盖情况。

### Q: 覆盖率太低怎么办？

A:
1. 下载HTML报告查看未覆盖的代码
2. 添加相应的测试
3. 重新运行workflow验证

### Q: 运行失败了？

A: 检查错误日志，常见原因：
- 测试失败（修复测试）
- 编译错误（修复代码）
- Token过期（联系maintainer）

---

## 🎯 下一步

查看完整的覆盖率报告后：

1. 识别未覆盖的关键代码
2. 编写测试增加覆盖率
3. 提交PR时会自动生成新的覆盖率报告
4. 在PR中查看覆盖率变化

---

**需要更多信息？** 查看 [Manual Build Guide](.github/MANUAL_BUILD_GUIDE.md)
