#!/bin/bash
# Test script for project discovery mechanism

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$SCRIPT_DIR/target/release/intent-engine"
TEST_CMD='{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"current_task_get","arguments":{}}}'

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║          Intent-Engine 项目发现机制健壮性测试                    ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo

# 确保二进制文件存在
if [ ! -f "$BINARY" ]; then
    echo "❌ 二进制文件不存在，正在编译..."
    cargo build --release
fi

echo "测试二进制: $BINARY"
echo "项目根目录: $SCRIPT_DIR"
echo

# 测试计数
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

run_test() {
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    local test_name="$1"
    local test_cmd="$2"
    local expect_success="$3"  # "success" or "failure"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "测试 $TOTAL_TESTS: $test_name"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if eval "$test_cmd" > /dev/null 2>&1; then
        if [ "$expect_success" = "success" ]; then
            echo "✅ 通过 - 成功找到项目"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            echo "❌ 失败 - 预期失败但成功了"
            FAILED_TESTS=$((FAILED_TESTS + 1))
        fi
    else
        if [ "$expect_success" = "failure" ]; then
            echo "✅ 通过 - 正确返回错误"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            echo "❌ 失败 - 预期成功但失败了"
            FAILED_TESTS=$((FAILED_TESTS + 1))
        fi
    fi
    echo
}

# 测试 1: 从项目根目录运行
run_test "从项目根目录运行" \
    "cd '$SCRIPT_DIR' && echo '$TEST_CMD' | '$BINARY' mcp-server" \
    "success"

# 测试 2: 从子目录运行
run_test "从子目录运行 (src/)" \
    "cd '$SCRIPT_DIR/src' && echo '$TEST_CMD' | '$BINARY' mcp-server" \
    "success"

# 测试 3: 从深层子目录运行
run_test "从深层子目录运行 (src/mcp/)" \
    "cd '$SCRIPT_DIR/src/mcp' && echo '$TEST_CMD' | '$BINARY' mcp-server" \
    "success"

# 测试 4: 使用环境变量
run_test "使用环境变量 INTENT_ENGINE_PROJECT_DIR" \
    "cd /tmp && INTENT_ENGINE_PROJECT_DIR='$SCRIPT_DIR' echo '$TEST_CMD' | '$BINARY' mcp-server" \
    "success"

# 测试 5: 从完全不相关的目录运行（应该失败）
run_test "从不相关目录运行（无环境变量）" \
    "cd /tmp && echo '$TEST_CMD' | '$BINARY' mcp-server" \
    "failure"

# 测试 6: 环境变量指向错误路径（应该失败）
run_test "环境变量指向不存在的项目" \
    "INTENT_ENGINE_PROJECT_DIR='/nonexistent/path' echo '$TEST_CMD' | '$BINARY' mcp-server" \
    "failure"

# 测试 7: CLI 模式（不依赖项目目录）
run_test "CLI 模式: --help" \
    "'$BINARY' --help" \
    "success"

# 测试 8: CLI 模式: 从项目目录运行任务命令
run_test "CLI 模式: task find (从项目目录)" \
    "cd '$SCRIPT_DIR' && '$BINARY' task find --status todo" \
    "success"

# 测试 9: MCP 模式: 检查工具列表
run_test "MCP 模式: tools/list" \
    "cd '$SCRIPT_DIR' && echo '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}' | '$BINARY' mcp-server" \
    "success"

# 测试 10: MCP 模式: 实际调用工具
run_test "MCP 模式: task_find" \
    "cd '$SCRIPT_DIR' && echo '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"task_find\",\"arguments\":{}}}' | '$BINARY' mcp-server" \
    "success"

# 总结
echo "╔════════════════════════════════════════════════════════════════╗"
echo "║                        测试总结                                 ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo
echo "总测试数: $TOTAL_TESTS"
echo "✅ 通过: $PASSED_TESTS"
echo "❌ 失败: $FAILED_TESTS"
echo

if [ $FAILED_TESTS -eq 0 ]; then
    echo "🎉 所有测试通过！"
    exit 0
else
    echo "⚠️  有测试失败，请检查上述输出"
    exit 1
fi
