# 🔍 本地 CI 检查脚本 - 快速参考卡

## 📦 脚本总览

```
┌─────────────────────────────────────────────────────────────┐
│  本地 CI 检查脚本                                            │
├─────────────────────────────────────────────────────────────┤
│  quick-check.sh          ⚡ 快速检查 (1-2 分钟)             │
│  pre-commit-check.sh     🔍 完整检查 (5-10 分钟)           │
│  fix-issues.sh           🔧 自动修复                        │
│  install-git-hook.sh     🪝 安装 Git Hook                   │
└─────────────────────────────────────────────────────────────┘
```

## 🚀 快速命令

### 首次使用
```bash
# 1. 设置权限
chmod +x *.sh

# 2. 安装 Git hook（推荐）
./install-git-hook.sh
```

### 日常使用
```bash
# 快速检查（推荐日常使用）
./quick-check.sh

# 发现问题？自动修复
./fix-issues.sh

# 提交前完整检查
./pre-commit-check.sh
```

## 📊 对比表格

| 特性 | quick-check | pre-commit-check | fix-issues | git hook |
|------|-------------|------------------|------------|----------|
| 速度 | ⚡⚡⚡ | ⚡ | ⚡⚡ | ⚡⚡⚡ |
| 详细度 | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| 自动化 | ❌ | ❌ | ✅ | ✅ |
| 格式检查 | ✅ | ✅ | ✅ | ✅ |
| Clippy | ✅ | ✅ | ✅ | ✅ |
| 测试 | ✅ | ✅ | ❌ | ✅ |
| 安全审计 | ❌ | ✅ | ❌ | ❌ |
| 覆盖率 | ❌ | ✅ | ❌ | ❌ |

## 🎯 使用场景

### 场景 1: 日常开发
```bash
# 写代码 → 快速验证
./quick-check.sh
```

### 场景 2: 发现问题
```bash
# 检查失败 → 自动修复 → 重新检查
./fix-issues.sh
./quick-check.sh
```

### 场景 3: 准备提交
```bash
# 完整验证 → 提交
./pre-commit-check.sh
git add .
git commit -m "feat: new feature"
```

### 场景 4: 自动化
```bash
# 安装 hook → 自动检查
./install-git-hook.sh
git commit -m "message"  # 自动触发检查
```

## 🔧 检查项目详情

### quick-check.sh
```
[1/4] 格式检查 ✓
[2/4] Clippy 检查 ✓
[3/4] 编译检查 ✓
[4/4] 运行测试 ✓
```

### pre-commit-check.sh
```
[1/7] 代码格式 (rustfmt)
[2/7] Clippy lint
[3/7] 项目编译
[4/7] 运行测试
[5/7] 安全审计 (cargo-deny)
[6/7] 文档生成
[7/7] 代码覆盖率 (可选)
```

### fix-issues.sh
```
[1/3] 修复代码格式 (cargo fmt)
[2/3] 修复 Clippy 问题 (cargo clippy --fix)
[3/3] 检查依赖更新 (cargo-outdated)
```

## ⚙️ 依赖工具

### 必需（基础功能）
```bash
rustup component add rustfmt clippy
```

### 可选（增强功能）
```bash
# 安全审计
cargo install --locked cargo-deny

# 代码覆盖率（仅 Linux）
cargo install cargo-tarpaulin

# 依赖检查
cargo install cargo-outdated
```

## 💡 常用命令组合

### 快速修复并验证
```bash
./fix-issues.sh && ./quick-check.sh
```

### 完整流程
```bash
./fix-issues.sh && ./pre-commit-check.sh && git add . && git commit
```

### 跳过 Git hook
```bash
git commit --no-verify -m "message"
```

## 🐛 故障排查

### 权限错误
```bash
chmod +x *.sh
```

### 检查通过但 CI 失败
```bash
cargo clean
cargo build --all-features --workspace
./pre-commit-check.sh
```

### 脚本太慢
```bash
# 使用快速检查
./quick-check.sh

# 或跳过覆盖率（pre-commit-check.sh 中按 Ctrl+C）
```

## 📝 输出示例

### ✅ 成功
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ⚡ 快速 CI 预检
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[1/4] 格式检查... ✓
[2/4] Clippy 检查... ✓
[3/4] 编译检查... ✓
[4/4] 运行测试... ✓

✨ 所有检查通过！
```

### ❌ 失败
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ⚡ 快速 CI 预检
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[1/4] 格式检查... ✗
[2/4] Clippy 检查... ✓
[3/4] 编译检查... ✓
[4/4] 运行测试... ✗

⚠️  2 项检查失败

运行详细检查脚本查看问题：
  ./pre-commit-check.sh
```

## 🎓 最佳实践

1. **日常开发**: 使用 `quick-check.sh`
2. **提交前**: 运行 `pre-commit-check.sh`
3. **自动化**: 安装 Git hook
4. **快速修复**: 使用 `fix-issues.sh`
5. **CI 对齐**: 本地检查 = CI 检查

## 🔗 相关文档

- 完整指南: `LOCAL_CHECK_GUIDE.md`
- 快速开始: `QUICK_START.md`
- GitHub 工作流: `.github/workflows/`

---

**记住：本地检查通过 = CI 流水线通过** ✅
