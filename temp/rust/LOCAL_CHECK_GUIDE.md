# 本地 CI 检查脚本使用指南

## 📦 脚本概览

本目录包含 4 个本地 CI 检查脚本，在提交代码前验证是否能通过 GitHub Actions CI 流水线。

| 脚本 | 用途 | 耗时 | 推荐场景 |
|------|------|------|---------|
| `quick-check.sh` | 快速检查关键项 | 1-2 分钟 | 日常快速验证 |
| `pre-commit-check.sh` | 完整 CI 检查（含覆盖率） | 5-10 分钟 | 提交前全面检查 |
| `fix-issues.sh` | 自动修复常见问题 | 1 分钟 | 快速修复格式等问题 |
| `install-git-hook.sh` | 安装 Git pre-commit hook | 立即 | 自动化检查 |

## 🚀 快速开始

### 1. 设置脚本权限

```bash
chmod +x quick-check.sh pre-commit-check.sh fix-issues.sh install-git-hook.sh
```

### 2. 推荐工作流

```bash
# 日常开发：快速检查
./quick-check.sh

# 发现问题：自动修复
./fix-issues.sh

# 提交前：完整检查
./pre-commit-check.sh

# 提交代码
git add .
git commit -m "your message"
git push
```

## 📋 脚本详解

### 1. quick-check.sh - 快速检查

**检查项目：**
- ✅ 代码格式 (rustfmt)
- ✅ Clippy lint
- ✅ 编译检查
- ✅ 运行测试

**特点：**
- 输出简洁
- 速度最快
- 适合日常使用

**使用示例：**
```bash
./quick-check.sh

# 输出示例：
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#   ⚡ 快速 CI 预检
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#
# [1/4] 格式检查... ✓
# [2/4] Clippy 检查... ✓
# [3/4] 编译检查... ✓
# [4/4] 运行测试... ✓
#
# ✨ 所有检查通过！
```

---

### 2. pre-commit-check.sh - 完整 CI 检查

**检查项目：**
1. ✅ 代码格式 (rustfmt)
2. ✅ Clippy lint
3. ✅ 编译检查
4. ✅ 运行所有测试
5. ✅ 安全审计 (cargo-deny)
6. ✅ 文档生成
7. ✅ 代码覆盖率 (cargo-tarpaulin，可选)

**特点：**
- 输出详细
- 提供修复建议
- 完全模拟 CI 环境
- 可设置超时

**使用示例：**
```bash
./pre-commit-check.sh

# 输出示例：
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#   🚀 Rust 项目本地 CI 预检
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#
# 环境检查通过 ✓
#
# ────────────────────────────────────────────────────────
#
# [1/7] → 检查代码格式 (rustfmt)...
#   ✓ 代码格式检查通过
#
# [2/7] → 运行 Clippy lint 检查...
#   (这可能需要一些时间...)
#   ✓ Clippy 检查通过，无警告
#
# ... (更多详细输出)
#
# ────────────────────────────────────────────────────────
#
# 📊 检查结果总结
#
#   总检查数: 7
#   通过: 7
#   失败: 0
#   耗时: 234 秒
#
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#   ✨ 所有检查通过！可以安全提交代码
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**检查失败时的输出：**
```bash
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#   ⚠️  发现 2 个问题，请修复后再提交
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#
# 修复建议：
#
#   1. 修复格式问题：
#      cargo fmt
#
#   2. 修复 Clippy 警告：
#      cargo clippy --all-targets --all-features --workspace --fix
#
#   修复完成后，重新运行此脚本验证
```

---

### 3. fix-issues.sh - 自动修复

**修复项目：**
- 🔧 自动格式化代码
- 🔧 自动修复 Clippy 可修复的问题
- 📦 检查依赖更新

**特点：**
- 自动化修复
- 安全（不会破坏代码）
- 显示修复结果

**使用示例：**
```bash
./fix-issues.sh

# 输出示例：
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#   🔧 自动修复常见问题
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#
# [1/3] 修复代码格式...
#   ✓ 代码格式已修复
#
# [2/3] 尝试修复 Clippy 问题...
#   ✓ Clippy 问题已修复（如有）
#
# [3/3] 检查依赖更新...
#   可用的依赖更新：
#   serde 1.0.100 → 1.0.150
#   
#   提示: 运行 'cargo update' 更新依赖
#
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# ✨ 修复完成！
#
# 下一步：
#   1. 检查修改内容: git diff
#   2. 运行验证脚本: ./quick-check.sh
#   3. 提交更改: git add . && git commit -m "fix: auto-fix issues"
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

### 4. install-git-hook.sh - 自动化检查

**功能：**
- 安装 Git pre-commit hook
- 每次 commit 前自动运行检查
- 检查失败时阻止提交

**特点：**
- 完全自动化
- 可以跳过（--no-verify）
- 备份现有 hook

**使用示例：**
```bash
# 安装 hook
./install-git-hook.sh

# 输出示例：
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#   🔧 安装 Git Pre-commit Hook
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#
# ✓ pre-commit hook 已安装到 .git/hooks/pre-commit
#
# 说明：
#   • 每次 git commit 时会自动运行检查
#   • 检查包括：格式、Clippy、编译、测试
#   • 如需跳过检查，使用: git commit --no-verify
#
# 测试 hook：
#   git commit --allow-empty -m "test hook"
#
# ✨ 安装完成！

# 之后每次提交时会自动检查
git commit -m "feat: add new feature"

# 输出示例：
# 🔍 运行 pre-commit 检查...
#
#   [1/4] 格式检查... ✓
#   [2/4] Clippy... ✓
#   [3/4] 编译... ✓
#   [4/4] 测试... ✓
#
# ✨ 所有检查通过，提交继续

# 如需跳过检查（不推荐）
git commit --no-verify -m "feat: add new feature"
```

## 🔧 依赖工具

### 必需工具

这些工具是脚本运行的基础：

```bash
# Rust 工具链（必需）
rustup component add rustfmt clippy

# Cargo 基础命令
cargo --version
```

### 可选工具

这些工具可以增强检查功能：

```bash
# cargo-deny - 安全审计和许可证检查
cargo install --locked cargo-deny

# cargo-tarpaulin - 代码覆盖率（仅 Linux）
cargo install cargo-tarpaulin

# cargo-outdated - 检查过期依赖
cargo install cargo-outdated

# cargo-edit - 便捷的依赖管理
cargo install cargo-edit
```

### 工具安装检查

脚本会自动检测工具是否安装，并在缺失时给出安装提示：

```bash
⚠ cargo-deny 未安装，跳过安全审计
ℹ 安装命令: cargo install --locked cargo-deny
```

## 💡 使用技巧

### 1. 别名设置

在 `~/.bashrc` 或 `~/.zshrc` 中添加别名：

```bash
# 快速检查
alias rcheck='./quick-check.sh'

# 完整检查
alias rfull='./pre-commit-check.sh'

# 自动修复
alias rfix='./fix-issues.sh'
```

### 2. 集成到编辑器

#### VS Code

在 `.vscode/tasks.json` 中添加：

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Quick Check",
      "type": "shell",
      "command": "./quick-check.sh",
      "problemMatcher": [],
      "group": {
        "kind": "test",
        "isDefault": true
      }
    }
  ]
}
```

#### IntelliJ IDEA / CLion

1. 打开 Run/Debug Configurations
2. 添加 Shell Script
3. 设置脚本路径为 `quick-check.sh`

### 3. CI/CD 对比

脚本完全模拟 GitHub Actions CI 环境：

| 检查项 | 本地脚本 | GitHub Actions |
|--------|----------|----------------|
| 格式检查 | ✅ | ✅ |
| Clippy | ✅ | ✅ |
| 测试 | ✅ | ✅ |
| 安全审计 | ✅ | ✅ |
| 覆盖率 | ✅ | ✅ |
| 多平台构建 | ❌ | ✅ |

### 4. 性能优化

**启用增量编译：**
```bash
export CARGO_INCREMENTAL=1
```

**使用 sccache 加速编译：**
```bash
cargo install sccache
export RUSTC_WRAPPER=sccache
```

**并行测试：**
```bash
cargo test -- --test-threads=4
```

### 5. 跳过耗时检查

如果觉得覆盖率检查太慢，可以按 `Ctrl+C` 跳过，或修改脚本注释掉覆盖率部分。

## 🐛 常见问题

### Q1: 脚本运行失败，提示权限不足

```bash
chmod +x *.sh
```

### Q2: cargo-tarpaulin 安装失败

cargo-tarpaulin 仅支持 Linux。macOS 和 Windows 用户可以跳过覆盖率检查，或使用其他工具如 llvm-cov。

### Q3: 检查通过但 CI 失败

可能原因：
- 平台差异（如 Windows 路径问题）
- 依赖版本不同
- 环境变量差异

建议：
```bash
# 清理并重新构建
cargo clean
cargo build --all-features
```

### Q4: 脚本太慢

使用快速检查脚本：
```bash
./quick-check.sh
```

或只运行特定检查：
```bash
cargo fmt -- --check && cargo clippy
```

### Q5: Git hook 干扰工作流程

临时跳过：
```bash
git commit --no-verify -m "message"
```

永久禁用：
```bash
rm .git/hooks/pre-commit
```

## 📊 检查矩阵对比

| 功能 | quick-check | pre-commit-check | GitHub CI |
|------|-------------|------------------|-----------|
| 格式检查 | ✅ | ✅ | ✅ |
| Clippy | ✅ | ✅ | ✅ |
| 编译 | ✅ | ✅ | ✅ |
| 测试 | ✅ | ✅ | ✅ |
| 安全审计 | ❌ | ✅ | ✅ |
| 文档生成 | ❌ | ✅ | ✅ |
| 代码覆盖率 | ❌ | ✅ | ✅ |
| 多平台 | ❌ | ❌ | ✅ |
| 耗时 | 1-2 min | 5-10 min | 5-15 min |

## 🚀 推荐工作流

### 日常开发
```bash
# 1. 开发功能
# 2. 快速检查
./quick-check.sh

# 3. 发现问题就修复
./fix-issues.sh

# 4. 重新检查
./quick-check.sh
```

### 提交前
```bash
# 1. 完整检查
./pre-commit-check.sh

# 2. 通过后提交
git add .
git commit -m "feat: new feature"
git push
```

### 首次设置
```bash
# 1. 安装 Git hook（推荐）
./install-git-hook.sh

# 2. 之后每次提交会自动检查
git commit -m "message"
```

## 📚 参考资料

- [Rust 代码格式化](https://github.com/rust-lang/rustfmt)
- [Clippy 文档](https://github.com/rust-lang/rust-clippy)
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin)

---

**享受愉快的 Rust 开发！** 🦀
