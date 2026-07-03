#!/bin/bash
# Copyright (c) 2026 Kirky.X
# SPDX-License-Identifier: MIT
# Feature 组合测试脚本
# 测试所有主要 feature 组合的编译和测试

set -e

echo "=========================================="
echo "SdForge Feature 组合测试"
echo "=========================================="

# 定义要测试的 feature 组合
FEATURE_COMBINATIONS=(
    "http"
    "mcp"
    "http,mcp"
    "http,mcp,security"
    "http,mcp,security,cache"
    "http,streaming"
    "http,mcp,streaming,timestamp,logging"
    "full"
)

# 存储测试结果
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo ""
echo "开始测试..."
echo ""

for FEATURES in "${FEATURE_COMBINATIONS[@]}"; do
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo -e "${YELLOW}==========================================${NC}"
    echo -e "${YELLOW}测试 Feature 组合: $FEATURES${NC}"
    echo -e "${YELLOW}==========================================${NC}"

    # 清理并构建
    echo "1. 清理构建..."
    cargo clean -p sdforge 2>/dev/null || true

    # 构建测试
    echo "2. 构建..."
    if cargo build --features "$FEATURES" --package sdforge 2>&1; then
        echo -e "${GREEN}✓ 构建成功${NC}"
    else
        echo -e "${RED}✗ 构建失败${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        continue
    fi

    # 运行测试
    echo "3. 运行测试..."
    if cargo test --features "$FEATURES" --package sdforge 2>&1; then
        echo -e "${GREEN}✓ 测试通过${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}✗ 测试失败${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi

    # 获取 binary 大小
    echo "4. Binary 大小:"
    SIZE=$(ls -l target/debug/libsdforge.rlib 2>/dev/null | awk '{print $5}')
    if [ -n "$SIZE" ]; then
        echo "   libsdforge.rlib: $SIZE bytes"
    fi

    echo ""
done

echo "=========================================="
echo "测试结果汇总"
echo "=========================================="
echo "总测试数: $TOTAL_TESTS"
echo -e "通过: ${GREEN}$PASSED_TESTS${NC}"
echo -e "失败: ${RED}$FAILED_TESTS${NC}"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}所有测试通过!${NC}"
    exit 0
else
    echo -e "${RED}有测试失败${NC}"
    exit 1
fi
