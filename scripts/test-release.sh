#!/bin/bash
# 测试发布流程脚本

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Intent-Engine 发布测试脚本 ===${NC}\n"

# 1. 检查 git 状态
echo -e "${YELLOW}[1/6] 检查 git 状态...${NC}"
if [[ -n $(git status -s) ]]; then
    echo -e "${RED}✗ Git 工作区不干净，有未提交的更改${NC}"
    git status -s
    echo ""
    echo "请先提交或暂存所有更改"
    exit 1
else
    echo -e "${GREEN}✓ Git 工作区干净${NC}\n"
fi

# 2. 检查当前版本
echo -e "${YELLOW}[2/6] 检查当前版本...${NC}"
CURRENT_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
echo -e "当前版本: ${GREEN}$CURRENT_VERSION${NC}\n"

# 3. 测试本地打包
echo -e "${YELLOW}[3/6] 测试本地打包...${NC}"
if cargo package --list > /dev/null 2>&1; then
    FILE_COUNT=$(cargo package --list 2>/dev/null | wc -l)
    echo -e "${GREEN}✓ 打包成功，包含 $FILE_COUNT 个文件${NC}\n"
else
    echo -e "${RED}✗ 打包失败${NC}"
    exit 1
fi

# 4. Dry-run 发布测试
echo -e "${YELLOW}[4/6] Dry-run 发布测试...${NC}"
echo "这将验证包能否成功发布到 crates.io（不会真正发布）"
if cargo publish --dry-run > /tmp/publish-test.log 2>&1; then
    echo -e "${GREEN}✓ Dry-run 成功！包已准备好发布${NC}\n"
else
    echo -e "${RED}✗ Dry-run 失败${NC}"
    echo "错误日志："
    tail -20 /tmp/publish-test.log
    exit 1
fi

# 5. 检查 GitHub Secret（如果有 gh CLI）
echo -e "${YELLOW}[5/6] 检查 GitHub Secret...${NC}"
if command -v gh &> /dev/null; then
    if gh secret list 2>/dev/null | grep -q "CARGO_REGISTRY_TOKEN"; then
        echo -e "${GREEN}✓ CARGO_REGISTRY_TOKEN 已设置${NC}\n"
    else
        echo -e "${RED}✗ CARGO_REGISTRY_TOKEN 未设置${NC}"
        echo "请在 GitHub 仓库设置中添加 CARGO_REGISTRY_TOKEN"
        echo "位置: Settings → Secrets → Actions → New repository secret"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ gh CLI 未安装，跳过 Secret 检查${NC}"
    echo "手动验证: https://github.com/wayfind/intent-engine/settings/secrets/actions"
    echo ""
fi

# 6. 显示下一步操作
echo -e "${YELLOW}[6/6] 下一步操作${NC}\n"
echo -e "${GREEN}✓ 所有预检查通过！${NC}\n"
echo "要发布新版本，请执行以下步骤："
echo ""
echo -e "${YELLOW}1. 更新版本号（如果需要）:${NC}"
echo "   编辑 Cargo.toml，修改 version = \"$CURRENT_VERSION\" 为新版本"
echo ""
echo -e "${YELLOW}2. 提交并创建 tag:${NC}"
echo "   git add Cargo.toml"
echo "   git commit -m \"Bump version to X.Y.Z\""
echo "   git push"
echo "   git tag vX.Y.Z"
echo "   git push origin vX.Y.Z"
echo ""
echo -e "${YELLOW}3. 监控 GitHub Actions:${NC}"
echo "   https://github.com/wayfind/intent-engine/actions"
echo ""
echo -e "${YELLOW}4. 验证发布:${NC}"
echo "   cargo search intent-engine"
echo "   cargo install intent-engine"
echo ""
echo -e "${GREEN}祝发布顺利！${NC}"
