#!/bin/bash
# 本地安装 ie 到 ~/.cargo/bin

set -e

echo "🚀 安装 ie 到 ~/.cargo/bin"
echo ""

# 检查是否在项目目录
if [ ! -f "Cargo.toml" ] || ! grep -q "name = \"intent-engine\"" Cargo.toml; then
    echo "❌ 错误：请在 intent-engine 项目根目录运行此脚本"
    exit 1
fi

# 显示当前状态
echo "📍 当前状态："
which ie 2>/dev/null && echo "  ie: $(which ie)" || echo "  ie: 未安装"
which intent-engine 2>/dev/null && echo "  intent-engine: $(which intent-engine)" || echo "  intent-engine: 未安装"
echo ""

# 安装
echo "📦 开始安装..."
cargo install --path . --force

echo ""
echo "✅ 安装完成！"
echo ""

# 验证
echo "🔍 验证安装："
echo "  位置: $(which ie)"
echo "  版本: $(ie --version)"
echo ""

# 运行健康检查
echo "🏥 运行健康检查..."
ie doctor | jq -r '.summary' 2>/dev/null || ie doctor | head -10

echo ""
echo "✨ ie 已成功安装到 $(which ie)"
echo ""
echo "下一步："
echo "  - 运行帮助: ie --help"
echo "  - 测试命令: ie task add --name 'Test task'"
echo "  - 查看文档: cat README.md"
