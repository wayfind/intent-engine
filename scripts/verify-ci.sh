#!/bin/bash

# CI配置验证脚本
# 用于验证三层CI策略的正确实施

set -e

YELLOW='\033[1;33m'
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}    CI优化配置验证脚本${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# 检查ci.yml文件存在
if [ ! -f ".github/workflows/ci.yml" ]; then
    echo -e "${RED}❌ 错误: 找不到 .github/workflows/ci.yml${NC}"
    exit 1
fi

echo -e "${GREEN}✅ 找到CI配置文件${NC}"
echo ""

# 1. 验证测试Job配置
echo -e "${YELLOW}📊 检查测试Job配置...${NC}"

# 检查test-fast job
if grep -q "test-fast:" .github/workflows/ci.yml; then
    echo -e "${GREEN}  ✅ test-fast job存在（快速检查）${NC}"
    fast_job=1
else
    echo -e "${RED}  ❌ 缺少test-fast job${NC}"
    fast_job=0
fi

# 检查test-standard job
if grep -q "test-standard:" .github/workflows/ci.yml; then
    echo -e "${GREEN}  ✅ test-standard job存在（标准CI）${NC}"
    standard_job=1
else
    echo -e "${RED}  ❌ 缺少test-standard job${NC}"
    standard_job=0
fi

# 检查test-full job
if grep -q "test-full:" .github/workflows/ci.yml; then
    echo -e "${GREEN}  ✅ test-full job存在（完整CI）${NC}"
    full_job=1
else
    echo -e "${RED}  ❌ 缺少test-full job${NC}"
    full_job=0
fi

# 检查test-standard的条件
if grep -A 6 "test-standard:" .github/workflows/ci.yml | grep -q "pull_request"; then
    echo -e "${GREEN}  ✅ test-standard在PR时运行${NC}"
else
    echo -e "${YELLOW}  ⚠️  test-standard的PR条件可能缺失${NC}"
fi

# 检查test-full的条件
if grep -A 6 "test-full:" .github/workflows/ci.yml | grep -q "refs/heads/main"; then
    echo -e "${GREEN}  ✅ test-full仅在main/master分支运行${NC}"
else
    echo -e "${YELLOW}  ⚠️  test-full的分支条件可能缺失${NC}"
fi

echo ""

# 2. 验证条件执行配置
echo -e "${YELLOW}🎯 检查条件执行配置...${NC}"

# 检查coverage job的条件
if grep -A 5 "coverage:" .github/workflows/ci.yml | grep -q "if:.*main.*master.*schedule.*workflow_dispatch"; then
    echo -e "${GREEN}  ✅ Code Coverage 条件配置正确（仅完整CI）${NC}"
else
    echo -e "${RED}  ❌ Code Coverage 条件配置可能有问题${NC}"
fi

# 检查benchmarks job的条件
if grep -A 5 "bench:" .github/workflows/ci.yml | grep -q "if:.*main.*master.*schedule.*workflow_dispatch"; then
    echo -e "${GREEN}  ✅ Benchmarks 条件配置正确（仅完整CI）${NC}"
else
    echo -e "${RED}  ❌ Benchmarks 条件配置可能有问题${NC}"
fi

# 检查test-minimal-versions job的条件
if grep -A 5 "test-minimal-versions:" .github/workflows/ci.yml | grep -q "if:.*main.*master.*schedule.*workflow_dispatch"; then
    echo -e "${GREEN}  ✅ Test Minimal Versions 条件配置正确（仅完整CI）${NC}"
else
    echo -e "${RED}  ❌ Test Minimal Versions 条件配置可能有问题${NC}"
fi

# 检查install-scripts job的条件
if grep -A 5 "install-scripts:" .github/workflows/ci.yml | grep -q "if:.*main.*master.*schedule.*workflow_dispatch"; then
    echo -e "${GREEN}  ✅ Install Scripts 条件配置正确（仅完整CI）${NC}"
else
    echo -e "${RED}  ❌ Install Scripts 条件配置可能有问题${NC}"
fi

echo ""

# 3. 验证workflow触发条件
echo -e "${YELLOW}🚀 检查workflow触发条件...${NC}"

if grep -q "workflow_dispatch:" .github/workflows/ci.yml; then
    echo -e "${GREEN}  ✅ 支持手动触发${NC}"
else
    echo -e "${RED}  ❌ 缺少手动触发配置${NC}"
fi

if grep -q "schedule:" .github/workflows/ci.yml; then
    echo -e "${GREEN}  ✅ 配置了定时任务${NC}"
else
    echo -e "${YELLOW}  ⚠️  未配置定时任务${NC}"
fi

if grep -q "claude/\*\*" .github/workflows/ci.yml; then
    echo -e "${GREEN}  ✅ 支持claude/**开发分支${NC}"
else
    echo -e "${RED}  ❌ 缺少claude/**分支配置${NC}"
fi

echo ""

# 4. 验证安全审计配置
echo -e "${YELLOW}🛡️  检查安全配置...${NC}"

# 检查audit.toml
if [ -f ".cargo/audit.toml" ]; then
    echo -e "${GREEN}  ✅ 找到cargo-audit配置文件${NC}"

    # 检查是否配置了忽略的advisory
    if grep -q "RUSTSEC-2023-0071" .cargo/audit.toml; then
        echo -e "${GREEN}  ✅ 已配置RSA漏洞忽略${NC}"
    fi
else
    echo -e "${RED}  ❌ 缺少.cargo/audit.toml配置文件${NC}"
fi

# 检查deny.toml
if [ -f "deny.toml" ]; then
    echo -e "${GREEN}  ✅ 找到cargo-deny配置文件${NC}"

    # 检查许可证配置
    license_count=$(grep -A 20 "\[licenses\]" deny.toml | grep -c '"' || true)
    if [ "$license_count" -ge 10 ]; then
        echo -e "${GREEN}  ✅ 配置了足够的许可证白名单（$license_count 个许可证）${NC}"
    else
        echo -e "${YELLOW}  ⚠️  许可证白名单可能不足（$license_count 个许可证）${NC}"
    fi
else
    echo -e "${RED}  ❌ 缺少deny.toml配置文件${NC}"
fi

echo ""

# 5. 验证release配置
echo -e "${YELLOW}📦 检查Release配置...${NC}"

if [ -f ".github/workflows/release.yml" ]; then
    echo -e "${GREEN}  ✅ 找到release workflow${NC}"

    # 检查平台支持
    platforms=$(grep -c "target:" .github/workflows/release.yml || true)
    echo -e "${GREEN}  ✅ 支持 $platforms 个构建平台${NC}"

    # 检查ARM64配置
    if grep -q "aarch64-unknown-linux-gnu" .github/workflows/release.yml; then
        echo -e "${GREEN}  ✅ 包含ARM64支持${NC}"

        # 检查ARM64链接器配置
        if grep -q "linker.*aarch64-linux-gnu-gcc" .github/workflows/release.yml; then
            echo -e "${GREEN}  ✅ ARM64链接器配置正确${NC}"
        else
            echo -e "${YELLOW}  ⚠️  ARM64链接器配置可能缺失${NC}"
        fi
    fi
else
    echo -e "${YELLOW}  ⚠️  未找到release workflow${NC}"
fi

echo ""

# 6. 总结
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}    验证总结${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# 计算总体配置正确性
total_checks=15
passed_checks=0

# 重新检查关键配置
[ "$fast_job" -eq 1 ] && ((passed_checks++))
[ "$standard_job" -eq 1 ] && ((passed_checks++))
[ "$full_job" -eq 1 ] && ((passed_checks++))
grep -A 6 "test-standard:" .github/workflows/ci.yml | grep -q "pull_request" && ((passed_checks++))

grep -A 5 "coverage:" .github/workflows/ci.yml | grep -q "if:.*main.*master" && ((passed_checks++))
grep -A 5 "bench:" .github/workflows/ci.yml | grep -q "if:.*main.*master" && ((passed_checks++))
grep -A 5 "test-minimal-versions:" .github/workflows/ci.yml | grep -q "if:.*main.*master" && ((passed_checks++))
grep -A 5 "install-scripts:" .github/workflows/ci.yml | grep -q "if:.*main.*master" && ((passed_checks++))

grep -q "workflow_dispatch:" .github/workflows/ci.yml && ((passed_checks++))
grep -q "schedule:" .github/workflows/ci.yml && ((passed_checks++))
grep -q "claude/\*\*" .github/workflows/ci.yml && ((passed_checks++))

[ -f ".cargo/audit.toml" ] && ((passed_checks++))
[ -f "deny.toml" ] && ((passed_checks++))
[ -f ".github/workflows/release.yml" ] && ((passed_checks++))

grep -q "linker.*aarch64-linux-gnu-gcc" .github/workflows/release.yml 2>/dev/null && ((passed_checks++))

percentage=$((passed_checks * 100 / total_checks))

echo -e "通过检查: ${GREEN}$passed_checks${NC} / $total_checks"
echo -e "完成度: ${GREEN}${percentage}%${NC}"
echo ""

if [ "$percentage" -ge 90 ]; then
    echo -e "${GREEN}✅ CI配置优秀！三层策略已正确实施。${NC}"
    exit 0
elif [ "$percentage" -ge 70 ]; then
    echo -e "${YELLOW}⚠️  CI配置基本正确，但仍有改进空间。${NC}"
    exit 0
else
    echo -e "${RED}❌ CI配置存在问题，请检查上述错误。${NC}"
    exit 1
fi
