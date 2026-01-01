# GitHub Workflows 和徽章配置指南

## 📋 工作流文件

将以下文件放置在你的仓库中：

```
.github/
├── workflows/
│   ├── ci.yml              # CI 健康检查
│   ├── release.yml         # 发布工作流
│   └── tag-deleted.yml     # Tag 删除监听
└── deny.toml (放在项目根目录)  # cargo-deny 配置
```

## 🔧 配置步骤

### 1. GitHub Secrets 配置

在你的 GitHub 仓库中设置以下 Secrets（Settings → Secrets and variables → Actions）：

#### 必需的 Secrets：

1. **CODECOV_TOKEN**（用于代码覆盖率）
   - 访问 https://codecov.io/
   - 登录并添加你的仓库
   - 复制 token 到 GitHub Secrets

2. **CARGO_REGISTRY_TOKEN**（用于发布到 crates.io）
   - 访问 https://crates.io/settings/tokens
   - 创建新 token（需要有发布权限）
   - 复制 token 到 GitHub Secrets

### 2. crates.io Trusted Publishing 配置（可选但推荐）

crates.io 的 Trusted Publishing 目前还在开发中，当前仍需使用 API token。
一旦 Trusted Publishing 正式发布，可以移除 CARGO_REGISTRY_TOKEN，
改用 GitHub OIDC（OpenID Connect）进行身份验证。

### 3. 启用必要的 GitHub 权限

在仓库设置中（Settings → Actions → General → Workflow permissions）：
- ✅ 启用 "Read and write permissions"
- ✅ 启用 "Allow GitHub Actions to create and approve pull requests"

## 🏷️ GitHub 徽章

将以下徽章代码添加到你的 `README.md` 中，**只需替换 `YOUR_USERNAME` 和 `YOUR_REPO`**：

### 基础徽章

```markdown
<!-- CI 状态 -->
[![CI](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml)

<!-- Crates.io 版本 -->
[![Crates.io](https://img.shields.io/crates/v/YOUR_REPO.svg)](https://crates.io/crates/YOUR_REPO)

<!-- 文档 -->
[![Documentation](https://docs.rs/YOUR_REPO/badge.svg)](https://docs.rs/YOUR_REPO)

<!-- 下载量 -->
[![Downloads](https://img.shields.io/crates/d/YOUR_REPO.svg)](https://crates.io/crates/YOUR_REPO)

<!-- 许可证 -->
[![License](https://img.shields.io/crates/l/YOUR_REPO.svg)](https://github.com/YOUR_USERNAME/YOUR_REPO/blob/main/LICENSE)

<!-- Rust 版本 -->
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
```

### 代码质量徽章

```markdown
<!-- 代码覆盖率 -->
[![codecov](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO/branch/main/graph/badge.svg)](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO)

<!-- 依赖状态 -->
[![Dependency Status](https://deps.rs/repo/github/YOUR_USERNAME/YOUR_REPO/status.svg)](https://deps.rs/repo/github/YOUR_USERNAME/YOUR_REPO)

<!-- 安全审计 -->
[![Security Audit](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml/badge.svg?label=security)](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml)
```

### 完整示例（一行显示）

```markdown
[![CI](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/YOUR_REPO.svg)](https://crates.io/crates/YOUR_REPO)
[![Documentation](https://docs.rs/YOUR_REPO/badge.svg)](https://docs.rs/YOUR_REPO)
[![codecov](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO/branch/main/graph/badge.svg)](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO)
[![License](https://img.shields.io/crates/l/YOUR_REPO.svg)](https://github.com/YOUR_USERNAME/YOUR_REPO/blob/main/LICENSE)
```

## 🚀 使用说明

### CI 工作流

触发条件：
- 推送到 `main` 或 `develop` 分支
- 创建 Pull Request 到 `main` 或 `develop` 分支

检查项目：
- ✅ 所有测试是否通过
- ✅ 代码格式是否符合 rustfmt 标准
- ✅ Clippy lint 检查（无警告）
- ✅ 安全漏洞扫描（cargo-deny）
- ✅ 代码覆盖率（tarpaulin + codecov）
- ✅ 多平台构建检查（Linux, macOS, Windows）

### Release 工作流

触发条件：
- 推送形如 `v*.*.*` 的 tag（例如 `v1.0.0`）

执行步骤：
1. 从 tag 提取版本号
2. 自动更新 `Cargo.toml` 中的版本号
3. 构建多平台二进制文件：
   - x86_64-unknown-linux-gnu
   - aarch64-unknown-linux-gnu
4. 创建 GitHub Release 并上传资源
5. 发布到 crates.io

### 发布新版本

```bash
# 1. 确保所有改动已提交
git add .
git commit -m "feat: your changes"

# 2. 创建并推送 tag
git tag v1.0.0
git push origin v1.0.0

# 3. GitHub Actions 会自动：
#    - 更新版本号
#    - 构建二进制文件
#    - 创建 GitHub Release
#    - 发布到 crates.io
```

### 撤回发布（删除 tag）

```bash
# 删除本地 tag
git tag -d v1.0.0

# 删除远程 tag
git push origin :refs/tags/v1.0.0

# GitHub Actions 会自动：
# - 删除对应的 GitHub Release
# ⚠️ 注意：crates.io 上的版本无法删除，只能 yank
```

### 手动 yank crates.io 版本

```bash
# Yank 版本（标记为不推荐）
cargo yank --version 1.0.0 YOUR_CRATE_NAME

# 取消 yank
cargo yank --undo --version 1.0.0 YOUR_CRATE_NAME
```

## 📊 指标说明

| 指标 | 说明 | 通过标准 |
|------|------|---------|
| Tests | 单元测试和集成测试 | 所有测试通过 |
| Format | 代码格式检查 | 符合 rustfmt 标准 |
| Clippy | 代码质量检查 | 无警告 |
| Security | 依赖漏洞扫描 | 无高危漏洞 |
| Coverage | 代码覆盖率 | 建议 ≥ 70% |

## ⚠️ 重要提醒

### 关于 crates.io

1. **无法删除已发布的版本**
   - crates.io 的设计理念是版本一旦发布就永久存在
   - 这是为了确保依赖链的稳定性和可复现性

2. **只能 yank 版本**
   - Yank 会将版本标记为"不推荐"
   - 新项目无法依赖被 yank 的版本
   - 已经依赖该版本的项目不受影响

3. **发布前请确认**
   - 仔细检查版本号
   - 确认代码质量
   - 运行完整的测试套件

### cargo-deny 配置

`deny.toml` 中的许可证配置需要根据你的项目需求调整：
- `allow`: 允许的许可证列表
- `deny`: 明确拒绝的许可证
- 根据你的项目许可证政策修改

## 🔍 故障排查

### codecov token 无效
- 重新生成 codecov token
- 确认 token 有正确的仓库权限

### crates.io 发布失败
- 检查 CARGO_REGISTRY_TOKEN 是否有效
- 确认 crate 名称未被占用
- 验证 Cargo.toml 中的元数据是否完整

### 二进制构建失败
- 检查 Cargo.toml 中的 `[[bin]]` 配置
- 确认所有平台依赖都正确配置

## 📚 参考文档

- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [cargo-deny 文档](https://embarkstudios.github.io/cargo-deny/)
- [cargo-tarpaulin 文档](https://github.com/xd009642/tarpaulin)
- [crates.io 发布指南](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Codecov 文档](https://docs.codecov.com/)
