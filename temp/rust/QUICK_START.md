# 🚀 Rust 项目 GitHub Actions 完整配置包

这是一个为 Rust 项目定制的 GitHub Actions 工作流配置包，包含 CI/CD、发布管理和代码质量检查。

## 📦 包含内容

```
github-workflows/
├── .github/
│   └── workflows/
│       ├── ci.yml              # CI 健康检查工作流
│       ├── release.yml         # 自动发布工作流
│       └── tag-deleted.yml     # Tag 删除清理工作流
├── deny.toml                   # cargo-deny 安全配置
├── deploy.sh                   # 快速部署脚本（可执行）
├── README.md                   # 完整使用文档
└── BADGES_REFERENCE.md         # 徽章快速参考

```

## ✨ 核心功能

### 1. CI 健康检查 (ci.yml)
- ✅ **测试**: 运行所有单元测试和集成测试
- ✅ **格式**: 检查代码是否符合 rustfmt 标准
- ✅ **Clippy**: Rust 官方 lint 工具，确保代码质量
- ✅ **安全审计**: 使用 cargo-deny 扫描高危漏洞
- ✅ **代码覆盖率**: 使用 tarpaulin 生成覆盖率报告并上传到 codecov
- ✅ **多平台构建**: 在 Linux、macOS、Windows 上验证编译

### 2. 自动发布 (release.yml)
- 🏷️ **版本管理**: 自动从 git tag 提取版本号并更新 Cargo.toml
- 📦 **多平台打包**: 构建 x86_64 和 aarch64 Linux 二进制文件
- 🚀 **GitHub Release**: 自动创建 release 并上传所有资源
- 📤 **crates.io 发布**: 使用 Trusted Publishing 模式自动发布
- 📝 **自动生成 Changelog**: 基于 git commit 历史

### 3. Tag 删除清理 (tag-deleted.yml)
- 🗑️ **自动删除 Release**: 当 tag 被删除时自动清理对应的 GitHub Release
- ⚠️ **crates.io 提醒**: 提示用户 crates.io 无法删除版本，只能 yank

## 🚀 快速开始

### 方法 1: 使用自动部署脚本（推荐）

```bash
# 1. 下载并解压配置包到你的项目根目录
cd your-rust-project/

# 2. 运行部署脚本（自动替换用户名和仓库名）
chmod +x deploy.sh
./deploy.sh YOUR_GITHUB_USERNAME YOUR_REPO_NAME

# 例如：
./deploy.sh Kirky-X awesome-tool

# 3. 按照脚本提示配置 GitHub Secrets 并提交
```

### 方法 2: 手动配置

```bash
# 1. 复制工作流文件
cp -r .github/ your-rust-project/
cp deny.toml your-rust-project/

# 2. 手动替换 README.md 中的占位符
# YOUR_USERNAME → 你的 GitHub 用户名
# YOUR_REPO → 你的仓库名

# 3. 提交更改
cd your-rust-project/
git add .github/ deny.toml
git commit -m "ci: add GitHub Actions workflows"
git push
```

## 🔧 必需配置

### 1. GitHub Secrets

在 `Settings → Secrets and variables → Actions` 中添加：

| Secret 名称 | 用途 | 获取方式 |
|------------|------|---------|
| `CODECOV_TOKEN` | 代码覆盖率上传 | https://codecov.io/ |
| `CARGO_REGISTRY_TOKEN` | 发布到 crates.io | https://crates.io/settings/tokens |

### 2. GitHub 权限

在 `Settings → Actions → General → Workflow permissions` 中：
- ✅ 选择 "Read and write permissions"
- ✅ 勾选 "Allow GitHub Actions to create and approve pull requests"

## 🏷️ 徽章配置

### 快速复制（只需替换大写部分）

```markdown
[![CI](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/YOUR_REPO.svg)](https://crates.io/crates/YOUR_REPO)
[![Documentation](https://docs.rs/YOUR_REPO/badge.svg)](https://docs.rs/YOUR_REPO)
[![codecov](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO/branch/main/graph/badge.svg)](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO)
[![License](https://img.shields.io/crates/l/YOUR_REPO.svg)](LICENSE)
```

完整徽章参考请查看 `BADGES_REFERENCE.md`

## 📖 使用示例

### 发布新版本

```bash
# 1. 确保代码已提交
git add .
git commit -m "feat: add new feature"
git push

# 2. 创建版本 tag
git tag v1.0.0

# 3. 推送 tag（触发自动发布）
git push origin v1.0.0

# ✨ GitHub Actions 会自动：
#    - 更新 Cargo.toml 版本号
#    - 运行所有测试
#    - 构建二进制文件（x86_64 + aarch64）
#    - 创建 GitHub Release
#    - 发布到 crates.io
```

### 撤回错误的发布

```bash
# 1. 删除本地 tag
git tag -d v1.0.0

# 2. 删除远程 tag
git push origin :refs/tags/v1.0.0

# ✨ GitHub Actions 会自动：
#    - 删除对应的 GitHub Release

# ⚠️ 注意：crates.io 无法删除版本
# 需要手动 yank：
cargo yank --version 1.0.0 your-crate-name
```

### 日常开发流程

```bash
# 推送代码到 main 或 develop 分支会自动触发 CI
git push origin main

# CI 会自动运行：
# ✓ 测试
# ✓ 格式检查
# ✓ Clippy lint
# ✓ 安全审计
# ✓ 代码覆盖率
```

## 🎯 支持的项目类型

这套配置支持以下 Rust 项目结构：

- ✅ **纯库项目** (lib only)
- ✅ **纯二进制项目** (bin only)
- ✅ **混合项目** (lib + bin)
- ✅ **多二进制项目** (lib + multiple bins)
- ✅ **Workspace 项目** (多个 crate)

工作流会自动：
- 检测 Cargo.toml 中定义的所有 binary targets
- 为每个二进制文件生成独立的发布包
- 正确处理库和二进制的混合发布

## ⚠️ 重要提醒

### 关于 crates.io

1. **版本无法删除**
   - 一旦发布到 crates.io，版本将永久存在
   - 这是设计理念，确保依赖链稳定

2. **只能 yank**
   - Yank 会标记版本为"不推荐"
   - 新项目无法依赖被 yank 的版本
   - 已有项目不受影响

3. **发布前请确认**
   - 仔细检查版本号
   - 运行完整测试
   - 检查文档和示例

### cargo-deny 配置

`deny.toml` 中的许可证策略需要根据项目调整：

```toml
# 允许的许可证
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
]

# 拒绝的许可证
deny = [
    "GPL-3.0",    # 如果你的项目不兼容 GPL
    "AGPL-3.0",
]
```

## 🔍 故障排查

### CI 失败

| 问题 | 原因 | 解决方案 |
|------|------|---------|
| 测试失败 | 代码有 bug | 修复测试 |
| 格式检查失败 | 代码格式不符合标准 | 运行 `cargo fmt` |
| Clippy 警告 | 代码质量问题 | 运行 `cargo clippy --fix` |
| 安全审计失败 | 依赖有漏洞 | 更新依赖或在 deny.toml 中忽略 |
| 覆盖率上传失败 | CODECOV_TOKEN 无效 | 重新生成 token |

### 发布失败

| 问题 | 原因 | 解决方案 |
|------|------|---------|
| crates.io 发布失败 | Token 无效或权限不足 | 检查 CARGO_REGISTRY_TOKEN |
| 版本已存在 | 重复发布相同版本 | 更改版本号 |
| Crate 名称冲突 | 名称已被占用 | 更改 crate 名称 |
| 二进制构建失败 | 交叉编译配置问题 | 检查依赖是否支持目标平台 |

## 📚 相关文档

- **完整使用指南**: 查看 `README.md`
- **徽章参考**: 查看 `BADGES_REFERENCE.md`
- [GitHub Actions 官方文档](https://docs.github.com/en/actions)
- [cargo-deny 文档](https://embarkstudios.github.io/cargo-deny/)
- [crates.io 发布指南](https://doc.rust-lang.org/cargo/reference/publishing.html)

## 🤝 贡献与反馈

如有问题或建议，欢迎：
- 提交 Issue
- 创建 Pull Request
- 参与讨论

## 📄 许可证

这套配置文件可以自由使用，无需署名。

---

**祝你的 Rust 项目发布顺利！** 🎉
