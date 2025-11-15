# 本地构建和验证 ie 二进制文件

## 快速验证脚本

```bash
# 1. 清理之前的构建
cargo clean

# 2. 构建（检查是否有警告）
cargo build --release 2>&1 | tee build.log

# 3. 检查构建日志中是否有 "multiple build targets" 警告
grep "multiple build targets" build.log && echo "❌ 仍有警告" || echo "✅ 无警告"

# 4. 验证只有 ie binary 存在
ls -lh target/release/ie && echo "✅ ie 存在" || echo "❌ ie 不存在"
ls target/release/intent-engine 2>/dev/null && echo "❌ intent-engine 仍存在" || echo "✅ intent-engine 已移除"

# 5. 测试 ie 命令
./target/release/ie --version
./target/release/ie doctor

# 6. 运行测试套件
cargo test --lib
cargo test --test cli_tests
cargo test --test integration_tests

# 7. 验证文档示例（抽样检查）
echo "检查 README.md 中的命令示例..."
grep "intent-engine task\|intent-engine event" README.md && echo "❌ 仍有旧命令" || echo "✅ 文档已更新"

grep "ie task\|ie event" README.md && echo "✅ 新命令存在" || echo "⚠️  未找到新命令"
```

## 详细验证步骤

### 步骤 1: 清理构建

```bash
# 确保从干净状态开始
cargo clean
rm -f build.log
```

### 步骤 2: 构建并检查警告

```bash
# 构建 release 版本
cargo build --release 2>&1 | tee build.log

# 检查是否有 "multiple build targets" 警告
echo "=== 检查构建警告 ==="
if grep -q "multiple build targets" build.log; then
    echo "❌ 失败：仍然存在 multiple build targets 警告"
    grep "multiple build targets" build.log
    exit 1
else
    echo "✅ 通过：无 multiple build targets 警告"
fi
```

**预期结果**：
```
✅ 通过：无 multiple build targets 警告
Finished `release` profile [optimized] target(s) in XX.XXs
```

### 步骤 3: 验证 binary 文件

```bash
echo "=== 验证 binary 文件 ==="

# 检查 ie 是否存在
if [ -f target/release/ie ]; then
    echo "✅ ie binary 存在"
    ls -lh target/release/ie
else
    echo "❌ 失败：ie binary 不存在"
    exit 1
fi

# 检查 intent-engine 是否已移除
if [ -f target/release/intent-engine ]; then
    echo "❌ 失败：intent-engine binary 仍然存在"
    exit 1
else
    echo "✅ intent-engine binary 已移除"
fi

# 检查它们是否是同一个文件的硬链接（不应该是）
if [ -f target/release/ie ] && [ -f target/release/intent-engine ]; then
    if [ target/release/ie -ef target/release/intent-engine ]; then
        echo "⚠️  警告：ie 和 intent-engine 是同一个文件的硬链接"
    fi
fi
```

**预期结果**：
```
✅ ie binary 存在
-rwxrwxrwx 1 user user 7.0M Nov 14 17:37 target/release/ie
✅ intent-engine binary 已移除
```

### 步骤 4: 测试基本功能

```bash
echo "=== 测试基本功能 ==="

# 测试 --version
echo "1. 测试 --version:"
./target/release/ie --version
if [ $? -eq 0 ]; then
    echo "✅ --version 正常"
else
    echo "❌ --version 失败"
    exit 1
fi

# 测试 --help
echo -e "\n2. 测试 --help:"
./target/release/ie --help | head -10
if [ $? -eq 0 ]; then
    echo "✅ --help 正常"
else
    echo "❌ --help 失败"
    exit 1
fi

# 测试 doctor 命令
echo -e "\n3. 测试 doctor 命令:"
./target/release/ie doctor
if [ $? -eq 0 ]; then
    echo "✅ doctor 正常"
else
    echo "❌ doctor 失败"
    exit 1
fi
```

**预期结果**：
```
1. 测试 --version:
intent-engine 0.3.3
✅ --version 正常

2. 测试 --help:
A command-line database service for tracking strategic intent
✅ --help 正常

3. 测试 doctor 命令:
{
  "checks": [...]
  "overall_status": "healthy"
}
✅ doctor 正常
```

### 步骤 5: 运行测试套件

```bash
echo "=== 运行测试套件 ==="

# 库测试
echo "1. 运行库测试..."
cargo test --lib 2>&1 | tail -5
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✅ 库测试通过"
else
    echo "❌ 库测试失败"
    exit 1
fi

# CLI 测试
echo -e "\n2. 运行 CLI 测试..."
cargo test --test cli_tests 2>&1 | tail -5
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✅ CLI 测试通过"
else
    echo "❌ CLI 测试失败"
    exit 1
fi

# 集成测试
echo -e "\n3. 运行集成测试..."
cargo test --test integration_tests 2>&1 | tail -5
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✅ 集成测试通过"
else
    echo "❌ 集成测试失败"
    exit 1
fi
```

**预期结果**：
```
test result: ok. 258 passed; 0 failed; 0 ignored
✅ 库测试通过
test result: ok. XX passed; 0 failed; 0 ignored
✅ CLI 测试通过
test result: ok. XX passed; 0 failed; 0 ignored
✅ 集成测试通过
```

### 步骤 6: 验证文档更新

```bash
echo "=== 验证文档中的命令引用 ==="

# 检查是否还有旧的命令引用
echo "1. 检查是否有遗留的 'intent-engine <command>' 引用..."
OLD_REFS=$(grep -r "intent-engine task\|intent-engine event\|intent-engine report" \
    --include="*.md" --include="*.sh" --include="*.yml" \
    . 2>/dev/null | grep -v ".git/" | wc -l)

if [ "$OLD_REFS" -eq 0 ]; then
    echo "✅ 无遗留的旧命令引用"
else
    echo "❌ 发现 $OLD_REFS 处旧命令引用："
    grep -r "intent-engine task\|intent-engine event\|intent-engine report" \
        --include="*.md" --include="*.sh" --include="*.yml" \
        . 2>/dev/null | grep -v ".git/" | head -10
    exit 1
fi

# 检查新命令是否存在
echo -e "\n2. 检查文档中是否使用了 'ie' 命令..."
NEW_REFS=$(grep -r "ie task\|ie event\|ie report" \
    --include="*.md" \
    . 2>/dev/null | grep -v ".git/" | wc -l)

if [ "$NEW_REFS" -gt 0 ]; then
    echo "✅ 找到 $NEW_REFS 处新命令引用"
else
    echo "⚠️  警告：未找到新命令引用"
fi

# 抽样检查几个关键文件
echo -e "\n3. 抽样检查关键文件..."
for file in README.md README.en.md CLAUDE.md; do
    if [ -f "$file" ]; then
        IE_COUNT=$(grep -c "^ie \|[^\`]ie " "$file" 2>/dev/null || echo "0")
        echo "  $file: 找到 $IE_COUNT 处 'ie' 命令"
    fi
done
```

**预期结果**：
```
1. 检查是否有遗留的 'intent-engine <command>' 引用...
✅ 无遗留的旧命令引用

2. 检查文档中是否使用了 'ie' 命令...
✅ 找到 XXX 处新命令引用

3. 抽样检查关键文件...
  README.md: 找到 XX 处 'ie' 命令
  README.en.md: 找到 XX 处 'ie' 命令
  CLAUDE.md: 找到 XX 处 'ie' 命令
```

### 步骤 7: 验证 Cargo.toml 配置

```bash
echo "=== 验证 Cargo.toml 配置 ==="

# 检查 binary targets 数量
BIN_COUNT=$(grep -c "^\[\[bin\]\]" Cargo.toml)
echo "Binary targets 数量: $BIN_COUNT"

if [ "$BIN_COUNT" -eq 1 ]; then
    echo "✅ 只有一个 binary target"
else
    echo "❌ 失败：有 $BIN_COUNT 个 binary targets"
    exit 1
fi

# 检查 default-run
DEFAULT_RUN=$(grep "^default-run" Cargo.toml | cut -d'"' -f2)
echo "default-run = \"$DEFAULT_RUN\""

if [ "$DEFAULT_RUN" = "ie" ]; then
    echo "✅ default-run 设置正确"
else
    echo "❌ 失败：default-run 应该是 'ie'，实际是 '$DEFAULT_RUN'"
    exit 1
fi

# 显示 binary 配置
echo -e "\nBinary 配置："
sed -n '/^\[\[bin\]\]/,/^$/p' Cargo.toml
```

**预期结果**：
```
Binary targets 数量: 1
✅ 只有一个 binary target
default-run = "ie"
✅ default-run 设置正确

Binary 配置：
[[bin]]
name = "ie"
path = "src/main.rs"
```

### 步骤 8: 安装并测试

```bash
echo "=== 安装并测试 ==="

# 安装到本地
echo "1. 安装到本地 cargo bin..."
cargo install --path . --force

# 验证安装
echo -e "\n2. 验证安装的命令..."
which ie
if [ $? -eq 0 ]; then
    echo "✅ ie 已安装到 PATH"
else
    echo "❌ ie 未找到在 PATH 中"
    exit 1
fi

# 检查是否还有 intent-engine
which intent-engine 2>/dev/null
if [ $? -eq 0 ]; then
    echo "⚠️  警告：intent-engine 仍在 PATH 中（可能是旧版本）"
    echo "建议运行: cargo uninstall intent-engine"
else
    echo "✅ intent-engine 不在 PATH 中"
fi

# 测试已安装的版本
echo -e "\n3. 测试已安装的版本..."
ie --version
ie doctor
```

**预期结果**：
```
1. 安装到本地 cargo bin...
  Installing ie v0.3.3

2. 验证安装的命令...
/home/user/.cargo/bin/ie
✅ ie 已安装到 PATH
✅ intent-engine 不在 PATH 中

3. 测试已安装的版本...
intent-engine 0.3.3
{
  "overall_status": "healthy"
}
```

## 完整的一键验证脚本

将以下内容保存为 `verify-ie-build.sh`：

```bash
#!/bin/bash
set -e

echo "======================================================================"
echo "  验证 ie binary 构建和文档更新"
echo "======================================================================"
echo ""

# 1. 清理
echo "步骤 1/8: 清理之前的构建..."
cargo clean
rm -f build.log
echo "✅ 完成"
echo ""

# 2. 构建
echo "步骤 2/8: 构建 release 版本..."
cargo build --release 2>&1 | tee build.log | tail -5
if grep -q "multiple build targets" build.log; then
    echo "❌ 失败：仍有 multiple build targets 警告"
    exit 1
fi
echo "✅ 完成"
echo ""

# 3. 验证 binary
echo "步骤 3/8: 验证 binary 文件..."
[ -f target/release/ie ] && echo "✅ ie 存在" || { echo "❌ ie 不存在"; exit 1; }
[ ! -f target/release/intent-engine ] && echo "✅ intent-engine 已移除" || echo "⚠️  intent-engine 仍存在"
echo ""

# 4. 测试功能
echo "步骤 4/8: 测试基本功能..."
./target/release/ie --version
./target/release/ie doctor > /dev/null
echo "✅ 完成"
echo ""

# 5. 库测试
echo "步骤 5/8: 运行库测试..."
cargo test --lib --quiet
echo "✅ 完成"
echo ""

# 6. CLI 测试
echo "步骤 6/8: 运行 CLI 测试..."
cargo test --test cli_tests --quiet
echo "✅ 完成"
echo ""

# 7. 验证文档
echo "步骤 7/8: 验证文档更新..."
OLD_REFS=$(grep -r "intent-engine task\|intent-engine event" --include="*.md" . 2>/dev/null | grep -v ".git/" | wc -l)
if [ "$OLD_REFS" -eq 0 ]; then
    echo "✅ 无遗留的旧命令引用"
else
    echo "❌ 发现 $OLD_REFS 处旧命令引用"
    exit 1
fi
echo ""

# 8. 验证 Cargo.toml
echo "步骤 8/8: 验证 Cargo.toml..."
BIN_COUNT=$(grep -c "^\[\[bin\]\]" Cargo.toml)
[ "$BIN_COUNT" -eq 1 ] && echo "✅ 只有一个 binary target" || { echo "❌ 有 $BIN_COUNT 个 binary targets"; exit 1; }

DEFAULT_RUN=$(grep "^default-run" Cargo.toml | cut -d'"' -f2)
[ "$DEFAULT_RUN" = "ie" ] && echo "✅ default-run = ie" || { echo "❌ default-run = $DEFAULT_RUN"; exit 1; }
echo ""

echo "======================================================================"
echo "  ✅✅✅ 所有验证通过！ ✅✅✅"
echo "======================================================================"
echo ""
echo "下一步："
echo "  1. 本地安装: cargo install --path . --force"
echo "  2. 测试命令: ie --version && ie doctor"
echo "  3. 如果有旧的 intent-engine: cargo uninstall intent-engine"
```

## 使用方法

```bash
# 给脚本添加执行权限
chmod +x verify-ie-build.sh

# 运行验证
./verify-ie-build.sh
```

## 预期的完整输出

```
======================================================================
  验证 ie binary 构建和文档更新
======================================================================

步骤 1/8: 清理之前的构建...
✅ 完成

步骤 2/8: 构建 release 版本...
    Finished `release` profile [optimized] target(s) in 39.46s
✅ 完成

步骤 3/8: 验证 binary 文件...
✅ ie 存在
✅ intent-engine 已移除

步骤 4/8: 测试基本功能...
intent-engine 0.3.3
✅ 完成

步骤 5/8: 运行库测试...
✅ 完成

步骤 6/8: 运行 CLI 测试...
✅ 完成

步骤 7/8: 验证文档更新...
✅ 无遗留的旧命令引用

步骤 8/8: 验证 Cargo.toml...
✅ 只有一个 binary target
✅ default-run = ie

======================================================================
  ✅✅✅ 所有验证通过！ ✅✅✅
======================================================================

下一步：
  1. 本地安装: cargo install --path . --force
  2. 测试命令: ie --version && ie doctor
  3. 如果有旧的 intent-engine: cargo uninstall intent-engine
```
