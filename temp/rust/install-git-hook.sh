#!/bin/bash

# Git pre-commit hook 安装脚本
# 自动在 git commit 前运行检查
# 使用方法: ./install-git-hook.sh

set -e

HOOK_FILE=".git/hooks/pre-commit"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  🔧 安装 Git Pre-commit Hook${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# 检查是否在 git 仓库中
if [ ! -d ".git" ]; then
    echo -e "${YELLOW}⚠️  当前目录不是 git 仓库根目录${NC}"
    exit 1
fi

# 检查 hook 文件是否已存在
if [ -f "$HOOK_FILE" ]; then
    echo -e "${YELLOW}⚠️  pre-commit hook 已存在${NC}"
    echo -n "是否覆盖？(y/N): "
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        echo "已取消"
        exit 0
    fi
    # 备份现有的 hook
    cp "$HOOK_FILE" "$HOOK_FILE.backup"
    echo -e "${GREEN}✓${NC} 已备份现有 hook 到 $HOOK_FILE.backup"
fi

# 创建 pre-commit hook
cat > "$HOOK_FILE" << 'EOF'
#!/bin/bash

# Git pre-commit hook - 自动运行 CI 检查
# 由 install-git-hook.sh 自动生成

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo ""
echo -e "${BLUE}🔍 运行 pre-commit 检查...${NC}"
echo ""

FAILED=0

# 1. 格式检查
echo -ne "  [1/4] 格式检查... "
if cargo fmt -- --check > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    echo -e "${YELLOW}      运行 'cargo fmt' 修复格式问题${NC}"
    FAILED=$((FAILED + 1))
fi

# 2. Clippy
echo -ne "  [2/4] Clippy... "
if cargo clippy --all-targets --all-features --workspace -- -D warnings > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    echo -e "${YELLOW}      运行 'cargo clippy' 查看详情${NC}"
    FAILED=$((FAILED + 1))
fi

# 3. 编译
echo -ne "  [3/4] 编译... "
if cargo build --all-features --workspace > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    FAILED=$((FAILED + 1))
fi

# 4. 测试
echo -ne "  [4/4] 测试... "
if cargo test --all-features --workspace > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    FAILED=$((FAILED + 1))
fi

echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✨ 所有检查通过，提交继续${NC}"
    echo ""
    exit 0
else
    echo -e "${RED}⚠️  ${FAILED} 项检查失败${NC}"
    echo ""
    echo -e "${YELLOW}选项：${NC}"
    echo -e "  1. 修复问题后重新提交"
    echo -e "  2. 使用 ${YELLOW}git commit --no-verify${NC} 跳过检查（不推荐）"
    echo ""
    exit 1
fi
EOF

# 设置执行权限
chmod +x "$HOOK_FILE"

echo -e "${GREEN}✓${NC} pre-commit hook 已安装到 $HOOK_FILE"
echo ""
echo -e "${BLUE}说明：${NC}"
echo "  • 每次 git commit 时会自动运行检查"
echo "  • 检查包括：格式、Clippy、编译、测试"
echo "  • 如需跳过检查，使用: git commit --no-verify"
echo ""
echo -e "${BLUE}测试 hook：${NC}"
echo -e "  ${YELLOW}git commit --allow-empty -m \"test hook\"${NC}"
echo ""
echo -e "${GREEN}✨ 安装完成！${NC}"
