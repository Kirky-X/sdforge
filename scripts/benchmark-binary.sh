#!/bin/bash
# Binary Size 基准测试脚本
# 测试不同 feature 组合的 Release binary 大小

set -e

echo "=========================================="
echo "Axiom Binary Size 基准测试"
echo "=========================================="

# 定义要测试的 feature 组合
FEATURE_COMBINATIONS=(
    "http"
    "mcp"
    "http,mcp"
    "http,mcp,security"
    "http,mcp,security,cache"
    "http,mcp,security,cache,logging,timestamp,streaming"
    "full"
)

# 存储结果
declare -A SIZES

echo ""
echo "开始构建和测试..."
echo ""

for FEATURES in "${FEATURE_COMBINATIONS[@]}"; do
    echo -e "${YELLOW}构建 Feature: $FEATURES${NC}"

    # 清理并构建
    cargo clean -p axiom 2>/dev/null || true
    cargo build --release --features "$FEATURES" --package axiom 2>&1 | grep -E "(Finished|error)" || true

    # 获取 binary 大小
    SIZE=$(ls -l target/release/libaxiom.rlib 2>/dev/null | awk '{print $5}')
    if [ -n "$SIZE" ]; then
        SIZES["$FEATURES"]=$SIZE
        echo "  Size: $SIZE bytes"
    else
        SIZES["$FEATURES"]="0"
        echo -e "  ${RED}构建失败${NC}"
    fi

    echo ""
done

# 输出汇总表格
echo "=========================================="
echo "Binary Size 汇总"
echo "=========================================="
echo ""
printf "%-40s %15s %15s\n" "Feature 组合" "大小 (bytes)" "大小 (KB)"
printf "%-40s %15s %15s\n" "--------------------" "---------------" "---------------"

for FEATURES in "${FEATURE_COMBINATIONS[@]}"; do
    SIZE=${SIZES[$FEATURES]}
    SIZE_KB=$(echo "scale=2; $SIZE / 1024" | bc 2>/dev/null || echo "N/A")
    printf "%-40s %15s %15s\n" "$FEATURES" "$SIZE" "$SIZE_KB"
done

echo ""
echo "=========================================="
echo "Binary Size 差异分析"
echo "=========================================="

# 找到最大和最小
MIN_SIZE=999999999
MAX_SIZE=0
MIN_FEATURES=""
MAX_FEATURES=""

for FEATURES in "${FEATURE_COMBINATIONS[@]}"; do
    SIZE=${SIZES[$FEATURES]}
    if [ "$SIZE" -gt 0 ] 2>/dev/null; then
        if [ $SIZE -lt $MIN_SIZE ]; then
            MIN_SIZE=$SIZE
            MIN_FEATURES=$FEATURES
        fi
        if [ $SIZE -gt $MAX_SIZE ]; then
            MAX_SIZE=$SIZE
            MAX_FEATURES=$FEATURES
        fi
    fi
done

if [ $MAX_SIZE -gt 0 ]; then
    DIFF=$(echo "scale=2; $MAX_SIZE / $MIN_SIZE" | bc 2>/dev/null || echo "N/A")
    echo "最小: $MIN_FEATURES ($MIN_SIZE bytes)"
    echo "最大: $MAX_FEATURES ($MAX_SIZE bytes)"
    echo "差异倍数: ${DIFF}x"
    echo ""
    echo "结论:"
    if (( $(echo "$DIFF > 2" | bc -l) )); then
        echo "Feature 组合对 binary 大小有显著影响"
        echo "建议只启用必要的 features 以减小 binary 大小"
    else
        echo "Feature 组合对 binary 大小影响适中"
    fi
fi

echo ""
echo "=========================================="
echo "优化建议"
echo "=========================================="
echo ""
echo "1. 只启用需要的 protocol (http 或 mcp)"
echo "2. security, cache 等功能会增加 binary 大小"
echo "3. 使用 LTO 优化可以进一步减小 binary 大小"
echo "4. 考虑使用 panic='abort' 减少 runtime 大小"
echo ""
