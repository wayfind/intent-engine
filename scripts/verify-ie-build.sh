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
echo "  - 测试 --version"
./target/release/ie --version
echo "  - 测试 doctor"
./target/release/ie doctor > /dev/null && echo "    ✓ doctor 正常" || { echo "    ✗ doctor 失败"; exit 1; }
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
OLD_REFS=$(grep -r "intent-engine task\|intent-engine event\|intent-engine report" \
    --include="*.md" --include="*.sh" --include="*.yml" \
    . 2>/dev/null | grep -v ".git/" | grep -v "verify-ie-build" | grep -v "VERIFY_IE_BINARY" | wc -l)
if [ "$OLD_REFS" -eq 0 ]; then
    echo "✅ 无遗留的旧命令引用"
else
    echo "❌ 发现 $OLD_REFS 处旧命令引用："
    grep -r "intent-engine task\|intent-engine event\|intent-engine report" \
        --include="*.md" --include="*.sh" --include="*.yml" \
        . 2>/dev/null | grep -v ".git/" | grep -v "verify-ie-build" | grep -v "VERIFY_IE_BINARY" | head -5
    exit 1
fi

# 检查新命令
NEW_REFS=$(grep -r " ie task\| ie event\| ie report" --include="*.md" . 2>/dev/null | grep -v ".git/" | wc -l)
echo "✅ 找到 $NEW_REFS 处新命令 'ie' 引用"
echo ""

# 8. 验证 Cargo.toml
echo "步骤 8/8: 验证 Cargo.toml..."
BIN_COUNT=$(grep -c "^\[\[bin\]\]" Cargo.toml)
[ "$BIN_COUNT" -eq 1 ] && echo "✅ 只有一个 binary target" || { echo "❌ 有 $BIN_COUNT 个 binary targets"; exit 1; }

DEFAULT_RUN=$(grep "^default-run" Cargo.toml | cut -d'"' -f2)
[ "$DEFAULT_RUN" = "ie" ] && echo "✅ default-run = ie" || { echo "❌ default-run = $DEFAULT_RUN"; exit 1; }

echo ""
echo "Cargo.toml binary 配置："
sed -n '/^\[\[bin\]\]/,/^$/p' Cargo.toml | sed 's/^/  /'
echo ""

echo "======================================================================"
echo "  ✅✅✅ 所有验证通过！ ✅✅✅"
echo "======================================================================"
echo ""
echo "下一步："
echo "  1. 本地安装: cargo install --path . --force"
echo "  2. 测试命令: ie --version && ie doctor"
echo "  3. 如果有旧的 intent-engine: cargo uninstall intent-engine"
echo ""
