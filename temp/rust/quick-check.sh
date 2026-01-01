#!/bin/bash

# 快速 CI 预检脚本（只检查关键项，跳过耗时的覆盖率等）
# 使用方法: ./quick-check.sh

set -e
set -o pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 图标
CHECK="✓"
CROSS="✗"
ARROW="→"

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  ⚡ 快速 CI 预检 (Rust)${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# 1. 格式检查
echo -e "${BLUE}[1/4]${NC} ${ARROW} 检查代码格式..."
echo -e "  ${YELLOW}运行命令: cargo fmt -- --check${NC}"
echo ""

if command -v cargo fmt &> /dev/null; then
    if cargo fmt -- --check > /dev/null 2>&1; then
        echo -e "${GREEN}  ${CHECK} 代码格式检查通过${NC}"
        echo ""
    else
        echo -e "${RED}  ✗ 代码格式检查失败${NC}"
        echo ""
        echo -e "${BLUE}💡 修复命令:${NC}"
        echo -e "  ${YELLOW}cargo fmt${NC}"
        echo ""
        exit 1
    fi
else
    echo -e "${YELLOW}  ⚠ cargo 未安装，跳过格式检查${NC}"
    echo ""
fi

# 2. Clippy
echo -e "${BLUE}[2/4]${NC} ${ARROW} 运行 Clippy 检查..."
echo -e "  ${YELLOW}运行命令: cargo clippy --all-targets --all-features --workspace -- -D warnings${NC}"
echo ""

if command -v cargo clippy &> /dev/null; then
    if cargo clippy --all-targets --all-features --workspace -- -D warnings > /dev/null 2>&1; then
        echo -e "${GREEN}  ${CHECK} Clippy 检查通过${NC}"
        echo ""
    else
        echo -e "${RED}  ✗ Clippy 发现问题${NC}"
        echo ""
        echo -e "${BLUE}💡 详细命令:${NC}"
        echo -e "  ${YELLOW}cargo clippy --all-targets --all-features --workspace${NC}"
        echo ""
        exit 1
    fi
else
    echo -e "${YELLOW}  ⚠ clippy 未安装，跳过 lint 检查${NC}"
    echo ""
    echo -e "${BLUE}💡 安装命令:${NC}"
    echo -e "  ${YELLOW}rustup component add clippy${NC}"
    echo ""
fi

# 3. 编译
echo -e "${BLUE}[3/4]${NC} ${ARROW} 检查项目编译..."
echo -e "  ${YELLOW}运行命令: cargo build --all-features --workspace${NC}"
echo ""

if command -v cargo &> /dev/null; then
    if cargo build --all-features --workspace > /dev/null 2>&1; then
        echo -e "${GREEN}  ${CHECK} 项目编译成功${NC}"
        echo ""
    else
        echo -e "${RED}  ✗ 项目编译失败${NC}"
        echo ""
        echo -e "${BLUE}💡 详细命令:${NC}"
        echo -e "  ${YELLOW}cargo build --all-features --workspace${NC}"
        echo ""
        exit 1
    fi
else
    echo -e "${YELLOW}  ⚠ cargo 未安装，跳过编译检查${NC}"
    echo ""
fi

# 4. 测试
echo -e "${BLUE}[4/4]${NC} ${ARROW} 运行测试..."
echo -e "  ${YELLOW}运行命令: cargo test --all-features --workspace${NC}"
echo ""

if command -v cargo test &> /dev/null; then
    if cargo test --all-features --workspace > /dev/null 2>&1; then
        echo -e "${GREEN}  ${CHECK} 所有测试通过${NC}"
        echo ""
    else
        echo -e "${RED}  ✗ 部分测试失败${NC}"
        echo ""
        echo -e "${BLUE}💡 详细命令:${NC}"
        echo -e "  ${YELLOW}cargo test --all-features --workspace${NC}"
        echo ""
        exit 1
    fi
else
    echo -e "${YELLOW}  ⚠ cargo 未安装，跳过测试${NC}"
    echo ""
fi

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}  ✨ 所有检查通过！${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}推荐的提交流程：${NC}"
echo -e "  1. ${YELLOW}git add .${NC}"
echo -e "  2. ${YELLOW}git commit -m \"your message\"${NC}"
echo -e "  3. ${YELLOW}git push${NC}"
echo ""
