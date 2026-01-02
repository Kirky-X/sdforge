#!/bin/bash

# Axiom Framework 验证脚本
# 用于编译和测试所有示例程序

set -e

echo "========================================"
echo "Axiom Framework 验证脚本"
echo "========================================"
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 统计变量
TOTAL=10
PASSED=0
FAILED=0

# 检查函数
check_build() {
    local bin_name=$1
    local display_name=$2

    echo -n "编译 $display_name ... "

    if cargo build --release --bin "$bin_name" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ 通过${NC}"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}✗ 失败${NC}"
        ((FAILED++))
        return 1
    fi
}

# 1. HTTP 协议示例
check_build "01_hello_http" "HTTP 协议示例"

# 2. MCP 工具示例
check_build "02_mcp_tool" "MCP 工具示例"

# 3. WebSocket 聊天示例
check_build "03_websocket_chat" "WebSocket 聊天示例"

# 4. gRPC 服务示例
check_build "04_grpc_service" "gRPC 服务示例"

# 5. 缓存功能示例
check_build "05_cache_demo" "缓存功能示例"

# 6. 配置管理示例
check_build "06_config_management" "配置管理示例"

# 7. 安全认证示例
check_build "07_security_auth" "安全认证示例"

# 8. 流式响应示例
check_build "08_streaming_sse" "流式响应示例"

# 9. 双协议示例
check_build "09_dual_protocol" "双协议示例"

# 10. 完整功能示例
check_build "10_full_stack" "完整功能示例"

# 输出结果
echo ""
echo "========================================"
echo "验证结果"
echo "========================================"
echo -e "总计: $TOTAL"
echo -e "${GREEN}通过: $PASSED${NC}"
if [ $FAILED -gt 0 ]; then
    echo -e "${RED}失败: $FAILED${NC}"
    exit 1
else
    echo -e "${GREEN}失败: 0${NC}"
fi
echo ""

# 运行测试（可选）
read -p "是否运行测试套件？(y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo "运行测试套件..."
    echo "========================================"
    cargo test --all-features
fi

echo ""
echo "========================================"
echo -e "${GREEN}所有验证完成！${NC}"
echo "========================================"
echo ""
echo "💡 运行示例:"
echo "  cargo run --bin 01_hello_http"
echo "  cargo run --bin 10_full_stack"
echo ""
echo "📚 查看文档:"
echo "  cat README.md"
echo ""