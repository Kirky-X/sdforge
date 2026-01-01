#!/bin/bash

# 智能修复脚本 - 自动修复常见问题
# 使用方法: ./fix-issues.sh

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  🔧 自动修复常见问题${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

FIXED=0
FAILED=0

# 1. 修复代码格式
echo -e "${BLUE}[1/3]${NC} 修复代码格式..."
if cargo fmt; then
    echo -e "${GREEN}  ✓ 代码格式已修复${NC}"
    FIXED=$((FIXED + 1))
else
    echo -e "${RED}  ✗ 格式修复失败${NC}"
    FAILED=$((FAILED + 1))
fi
echo ""

# 2. 尝试自动修复 Clippy 问题
echo -e "${BLUE}[2/3]${NC} 尝试修复 Clippy 问题..."
if cargo clippy --all-targets --all-features --workspace --fix --allow-dirty --allow-staged; then
    echo -e "${GREEN}  ✓ Clippy 问题已修复（如有）${NC}"
    FIXED=$((FIXED + 1))
else
    echo -e "${YELLOW}  ⚠ 部分 Clippy 问题需要手动修复${NC}"
fi
echo ""

# 3. 更新依赖
echo -e "${BLUE}[3/3]${NC} 检查依赖更新..."
if command -v cargo-outdated &> /dev/null; then
    echo "  可用的依赖更新："
    cargo outdated 2>/dev/null || true
    echo ""
    echo -e "${YELLOW}  提示: 运行 'cargo update' 更新依赖${NC}"
else
    echo -e "${YELLOW}  ⚠ cargo-outdated 未安装${NC}"
    echo -e "  安装命令: ${BLUE}cargo install cargo-outdated${NC}"
fi
echo ""

# 总结
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✨ 修复完成！${NC}"
    echo ""
    echo -e "${BLUE}下一步：${NC}"
    echo "  1. 检查修改内容: git diff"
    echo "  2. 运行验证脚本: ./quick-check.sh"
    echo "  3. 提交更改: git add . && git commit -m \"fix: auto-fix issues\""
else
    echo -e "${YELLOW}⚠️  部分问题需要手动修复${NC}"
    echo ""
    echo -e "${BLUE}建议：${NC}"
    echo "  运行详细检查查看具体问题: ./pre-commit-check.sh"
fi
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
