# 🏷️ GitHub 徽章快速参考

## 复制粘贴模板（替换 `YOUR_USERNAME` 和 `YOUR_REPO`）

### 一行式（推荐）
```markdown
[![CI](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml) [![Crates.io](https://img.shields.io/crates/v/YOUR_REPO.svg)](https://crates.io/crates/YOUR_REPO) [![Documentation](https://docs.rs/YOUR_REPO/badge.svg)](https://docs.rs/YOUR_REPO) [![codecov](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO/branch/main/graph/badge.svg)](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO) [![License](https://img.shields.io/crates/l/YOUR_REPO.svg)](LICENSE)
```

### 多行式（更清晰）
```markdown
[![CI](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/YOUR_REPO.svg)](https://crates.io/crates/YOUR_REPO)
[![Documentation](https://docs.rs/YOUR_REPO/badge.svg)](https://docs.rs/YOUR_REPO)
[![Downloads](https://img.shields.io/crates/d/YOUR_REPO.svg)](https://crates.io/crates/YOUR_REPO)
[![codecov](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO/branch/main/graph/badge.svg)](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO)
[![License](https://img.shields.io/crates/l/YOUR_REPO.svg)](LICENSE)
[![Dependency Status](https://deps.rs/repo/github/YOUR_USERNAME/YOUR_REPO/status.svg)](https://deps.rs/repo/github/YOUR_USERNAME/YOUR_REPO)
```

### 按类别分组
```markdown
## Build & Test
[![CI](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO/branch/main/graph/badge.svg)](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO)

## Package
[![Crates.io](https://img.shields.io/crates/v/YOUR_REPO.svg)](https://crates.io/crates/YOUR_REPO)
[![Documentation](https://docs.rs/YOUR_REPO/badge.svg)](https://docs.rs/YOUR_REPO)
[![Downloads](https://img.shields.io/crates/d/YOUR_REPO.svg)](https://crates.io/crates/YOUR_REPO)

## Quality
[![Dependency Status](https://deps.rs/repo/github/YOUR_USERNAME/YOUR_REPO/status.svg)](https://deps.rs/repo/github/YOUR_USERNAME/YOUR_REPO)
[![License](https://img.shields.io/crates/l/YOUR_REPO.svg)](LICENSE)
```

## 🎨 自定义徽章

### 自定义颜色
```markdown
[![Custom](https://img.shields.io/badge/custom-badge-blue.svg)](https://your-link.com)
```

颜色选项: `brightgreen`, `green`, `yellow`, `orange`, `red`, `blue`, `lightgrey`, `success`, `important`, `critical`, `informational`, `inactive`

### 自定义图标
```markdown
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg?logo=rust)](https://www.rust-lang.org)
[![GitHub](https://img.shields.io/badge/GitHub-YOUR__USERNAME-181717.svg?logo=github)](https://github.com/YOUR_USERNAME)
```

### 动态版本徽章
```markdown
<!-- 从 Cargo.toml 自动读取 -->
[![Version](https://img.shields.io/crates/v/YOUR_REPO.svg?label=version)](https://crates.io/crates/YOUR_REPO)

<!-- MSRV (Minimum Supported Rust Version) -->
[![MSRV](https://img.shields.io/badge/MSRV-1.70+-blue.svg)](https://www.rust-lang.org)
```

## 📊 更多徽章服务

### Shields.io（推荐）
- 网站: https://shields.io/
- 支持数千种徽章类型
- 高度可自定义

### Deps.rs
```markdown
[![Dependency Status](https://deps.rs/repo/github/YOUR_USERNAME/YOUR_REPO/status.svg)](https://deps.rs/repo/github/YOUR_USERNAME/YOUR_REPO)
```

### Codecov
```markdown
<!-- 基础徽章 -->
[![codecov](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO/branch/main/graph/badge.svg)](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO)

<!-- 带 token 的徽章（私有仓库） -->
[![codecov](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO/branch/main/graph/badge.svg?token=YOUR_CODECOV_TOKEN)](https://codecov.io/gh/YOUR_USERNAME/YOUR_REPO)
```

### Docs.rs
```markdown
<!-- 标准文档徽章 -->
[![Documentation](https://docs.rs/YOUR_REPO/badge.svg)](https://docs.rs/YOUR_REPO)

<!-- 指定版本 -->
[![Documentation](https://docs.rs/YOUR_REPO/badge.svg?version=1.0.0)](https://docs.rs/YOUR_REPO/1.0.0)
```

## 💡 实际例子（供参考）

假设你的项目是：
- GitHub: `Kirky-X/awesome-rust-tool`
- Crate: `awesome-rust-tool`

那么徽章应该是：

```markdown
[![CI](https://github.com/Kirky-X/awesome-rust-tool/actions/workflows/ci.yml/badge.svg)](https://github.com/Kirky-X/awesome-rust-tool/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/awesome-rust-tool.svg)](https://crates.io/crates/awesome-rust-tool)
[![Documentation](https://docs.rs/awesome-rust-tool/badge.svg)](https://docs.rs/awesome-rust-tool)
[![codecov](https://codecov.io/gh/Kirky-X/awesome-rust-tool/branch/main/graph/badge.svg)](https://codecov.io/gh/Kirky-X/awesome-rust-tool)
[![License](https://img.shields.io/crates/l/awesome-rust-tool.svg)](LICENSE)
```

## 🔧 快速替换工具

使用以下命令快速替换占位符：

```bash
# 在 Linux/macOS 上
sed -i 's/YOUR_USERNAME/Kirky-X/g' README.md
sed -i 's/YOUR_REPO/your-crate-name/g' README.md

# 在 macOS 上（BSD sed）
sed -i '' 's/YOUR_USERNAME/Kirky-X/g' README.md
sed -i '' 's/YOUR_REPO/your-crate-name/g' README.md
```

或者使用提供的 `deploy.sh` 脚本自动完成！
