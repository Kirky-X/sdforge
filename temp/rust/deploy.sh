#!/bin/bash

# GitHub Workflows 快速部署脚本
# 使用方法: ./deploy.sh YOUR_USERNAME YOUR_REPO

set -e

if [ $# -lt 2 ]; then
    echo "用法: $0 <GitHub用户名> <仓库名>"
    echo "示例: $0 Kirky-X my-rust-project"
    exit 1
fi

USERNAME="$1"
REPO="$2"

echo "🚀 开始部署 GitHub Workflows 配置..."
echo "   用户: $USERNAME"
echo "   仓库: $REPO"
echo ""

# 检查是否在 git 仓库中
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "❌ 错误: 当前目录不是 git 仓库"
    exit 1
fi

# 创建目录结构
echo "📁 创建目录结构..."
mkdir -p .github/workflows

# 复制配置文件
echo "📋 复制工作流文件..."
if [ -f "ci.yml" ]; then
    cp ci.yml .github/workflows/
    echo "   ✓ ci.yml"
fi

if [ -f "release.yml" ]; then
    cp release.yml .github/workflows/
    echo "   ✓ release.yml"
fi

if [ -f "tag-deleted.yml" ]; then
    cp tag-deleted.yml .github/workflows/
    echo "   ✓ tag-deleted.yml"
fi

if [ -f "deny.toml" ]; then
    cp deny.toml ./
    echo "   ✓ deny.toml"
fi

# 更新 README.md 中的徽章
if [ -f "README.md" ]; then
    echo "🏷️  添加徽章到 README.md..."
    
    # 创建徽章代码
    BADGES="
<!-- GitHub Actions Badges -->
[![CI](https://github.com/$USERNAME/$REPO/actions/workflows/ci.yml/badge.svg)](https://github.com/$USERNAME/$REPO/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/$REPO.svg)](https://crates.io/crates/$REPO)
[![Documentation](https://docs.rs/$REPO/badge.svg)](https://docs.rs/$REPO)
[![codecov](https://codecov.io/gh/$USERNAME/$REPO/branch/main/graph/badge.svg)](https://codecov.io/gh/$USERNAME/$REPO)
[![License](https://img.shields.io/crates/l/$REPO.svg)](https://github.com/$USERNAME/$REPO/blob/main/LICENSE)
"
    
    # 检查 README 是否已包含徽章
    if ! grep -q "github.com/$USERNAME/$REPO/actions" README.md; then
        # 在第一行之后添加徽章
        echo "$BADGES" | cat - README.md > temp && mv temp README.md
        echo "   ✓ 徽章已添加到 README.md"
    else
        echo "   ⚠️  README.md 中已存在徽章，跳过"
    fi
else
    echo "⚠️  未找到 README.md，跳过徽章添加"
fi

echo ""
echo "✅ 部署完成！"
echo ""
echo "📝 后续步骤:"
echo ""
echo "1️⃣  配置 GitHub Secrets (Settings → Secrets and variables → Actions):"
echo "   • CODECOV_TOKEN: 从 https://codecov.io/ 获取"
echo "   • CARGO_REGISTRY_TOKEN: 从 https://crates.io/settings/tokens 获取"
echo ""
echo "2️⃣  提交并推送更改:"
echo "   git add .github/ deny.toml README.md"
echo "   git commit -m \"ci: add GitHub Actions workflows\""
echo "   git push origin main"
echo ""
echo "3️⃣  创建第一个发布:"
echo "   git tag v0.1.0"
echo "   git push origin v0.1.0"
echo ""
echo "📚 查看完整文档请参考 README.md 或访问:"
echo "   https://github.com/$USERNAME/$REPO/actions"
echo ""
