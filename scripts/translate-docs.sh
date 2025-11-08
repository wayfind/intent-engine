#!/bin/bash
# translate-docs.sh
# 文档翻译脚本：将 zh-CN 文档翻译为英文

set -e  # 遇到错误立即退出

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 项目根目录
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${GREEN}Intent-Engine 文档翻译工具${NC}"
echo "========================================"
echo ""

# 列出所有需要翻译的文档
ZH_CN_DIR="$PROJECT_ROOT/docs/zh-CN"
EN_DIR="$PROJECT_ROOT/docs/en"

# 确保英文目录存在
mkdir -p "$EN_DIR"/{guide,integration,technical,contributing}

# 查找所有中文 markdown 文档
find_zh_docs() {
    find "$ZH_CN_DIR" -name "*.md" -type f | sort
}

# 获取对应的英文文档路径
get_en_path() {
    local zh_path="$1"
    echo "${zh_path/$ZH_CN_DIR/$EN_DIR}"
}

# 检查文档是否已翻译
is_translated() {
    local en_path="$1"
    [[ -f "$en_path" ]]
}

# 检查英文版本是否比中文版本新
is_up_to_date() {
    local zh_path="$1"
    local en_path="$2"

    if [[ ! -f "$en_path" ]]; then
        return 1
    fi

    # 比较文件修改时间
    [[ "$en_path" -nt "$zh_path" ]]
}

# 列出所有文档状态
list_status() {
    echo -e "${YELLOW}文档翻译状态：${NC}"
    echo ""

    local total=0
    local translated=0
    local outdated=0

    while IFS= read -r zh_file; do
        total=$((total + 1))
        local en_file
        en_file=$(get_en_path "$zh_file")
        local rel_path="${zh_file/$ZH_CN_DIR\//}"

        if is_translated "$en_file"; then
            if is_up_to_date "$zh_file" "$en_file"; then
                echo -e "  ${GREEN}✓${NC} $rel_path"
                translated=$((translated + 1))
            else
                echo -e "  ${YELLOW}⚠${NC} $rel_path (需要更新)"
                outdated=$((outdated + 1))
            fi
        else
            echo -e "  ${RED}✗${NC} $rel_path"
        fi
    done < <(find_zh_docs)

    echo ""
    echo "统计："
    echo "  总计: $total"
    echo "  已翻译: $translated"
    echo "  需更新: $outdated"
    echo "  未翻译: $((total - translated - outdated))"
}

# 翻译单个文件（需要手动使用 Claude Code）
translate_file() {
    local zh_file="$1"
    local en_file
    en_file=$(get_en_path "$zh_file")

    echo -e "${YELLOW}准备翻译：${NC}$zh_file"
    echo -e "${YELLOW}目标文件：${NC}$en_file"
    echo ""
    echo "请按照以下步骤操作："
    echo ""
    echo "1. 在 Claude Code 中打开源文件："
    echo "   $zh_file"
    echo ""
    echo "2. 复制以下提示词并发送给 Claude："
    echo ""
    echo -e "${GREEN}---[开始]---${NC}"
    cat << 'EOF'
请将以下中文文档翻译为英文，要求：

1. 保持 Markdown 格式不变
2. 保持代码块、命令示例、文件路径原样
3. 保持链接结构，但翻译链接文字
4. 翻译要专业、准确，符合技术文档规范
5. 保持表格对齐
6. 保持 YAML front matter（如果有）

请翻译以下文档：

[粘贴中文文档内容]
EOF
    echo -e "${GREEN}---[结束]---${NC}"
    echo ""
    echo "3. 将 Claude 的翻译结果保存到："
    echo "   $en_file"
    echo ""

    # 等待用户确认
    read -p "完成翻译后按 Enter 继续，或按 Ctrl+C 取消..."

    if [[ -f "$en_file" ]]; then
        echo -e "${GREEN}✓${NC} 翻译完成！"
    else
        echo -e "${RED}✗${NC} 未找到翻译文件，请检查路径"
        return 1
    fi
}

# 批量翻译（交互式）
translate_all() {
    echo -e "${YELLOW}开始批量翻译...${NC}"
    echo ""

    local count=0

    while IFS= read -r zh_file; do
        local en_file
        en_file=$(get_en_path "$zh_file")

        # 跳过已翻译且最新的文档
        if is_up_to_date "$zh_file" "$en_file"; then
            echo -e "${GREEN}跳过${NC} $zh_file (已是最新)"
            continue
        fi

        count=$((count + 1))
        translate_file "$zh_file"
        echo ""
    done < <(find_zh_docs)

    echo -e "${GREEN}完成！共翻译 $count 个文档。${NC}"
}

# 翻译根目录的文档
translate_root_docs() {
    echo -e "${YELLOW}翻译根目录文档...${NC}"
    echo ""

    # README.md
    if [[ -f "$PROJECT_ROOT/README.md" ]]; then
        echo "翻译 README.md -> README.en.md"
        echo ""
        echo "请在 Claude Code 中翻译 README.md 并保存为 README.en.md"
        read -p "完成后按 Enter..."
    fi

    # QUICKSTART.md
    if [[ -f "$PROJECT_ROOT/QUICKSTART.md" ]]; then
        echo "翻译 QUICKSTART.md -> QUICKSTART.en.md"
        echo ""
        echo "请在 Claude Code 中翻译 QUICKSTART.md 并保存为 QUICKSTART.en.md"
        read -p "完成后按 Enter..."
    fi
}

# 验证翻译完整性
validate_translations() {
    echo -e "${YELLOW}验证翻译完整性...${NC}"
    echo ""

    local missing=0

    while IFS= read -r zh_file; do
        local en_file
        en_file=$(get_en_path "$zh_file")

        if [[ ! -f "$en_file" ]]; then
            echo -e "  ${RED}缺失${NC} $en_file"
            missing=$((missing + 1))
        fi
    done < <(find_zh_docs)

    if [[ $missing -eq 0 ]]; then
        echo -e "${GREEN}✓ 所有文档已翻译！${NC}"
        return 0
    else
        echo -e "${RED}✗ 有 $missing 个文档未翻译${NC}"
        return 1
    fi
}

# 主菜单
show_menu() {
    echo ""
    echo "请选择操作："
    echo "  1) 查看翻译状态"
    echo "  2) 翻译单个文件"
    echo "  3) 批量翻译所有文档"
    echo "  4) 翻译根目录文档（README, QUICKSTART）"
    echo "  5) 验证翻译完整性"
    echo "  6) 退出"
    echo ""
    read -p "请输入选项 [1-6]: " choice

    case $choice in
        1) list_status ;;
        2)
            read -p "请输入要翻译的文档路径（相对于 docs/zh-CN/）: " rel_path
            translate_file "$ZH_CN_DIR/$rel_path"
            ;;
        3) translate_all ;;
        4) translate_root_docs ;;
        5) validate_translations ;;
        6) echo "退出"; exit 0 ;;
        *) echo "无效选项" ;;
    esac

    show_menu
}

# 命令行参数处理
if [[ $# -eq 0 ]]; then
    # 交互模式
    list_status
    show_menu
elif [[ "$1" == "--status" ]]; then
    list_status
elif [[ "$1" == "--validate" ]]; then
    validate_translations
elif [[ "$1" == "--all" ]]; then
    translate_all
elif [[ "$1" == "--root" ]]; then
    translate_root_docs
elif [[ -f "$1" ]]; then
    translate_file "$1"
else
    echo "用法:"
    echo "  $0                    # 交互式模式"
    echo "  $0 --status           # 查看翻译状态"
    echo "  $0 --validate         # 验证翻译完整性"
    echo "  $0 --all              # 批量翻译"
    echo "  $0 --root             # 翻译根目录文档"
    echo "  $0 <file>             # 翻译单个文件"
    exit 1
fi
