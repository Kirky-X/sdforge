# 🤝 Sdforge 贡献指南

感谢你对 SDForge 项目的兴趣！本文档描述了参与开发所需的环境、工作流和规范。

## 📋 目录

<details open>

- [欢迎](#-欢迎)
- [环境准备](#-环境准备)
- [开发工作流（TDD）](#-开发工作流tdd)
- [代码规范](#-代码规范)
- [提交与 PR 流程](#-提交与-pr-流程)
- [行为准则](#-行为准则)

</details>

## 🙌 欢迎

SDForge 是一个基于 Rust 的声明式多协议 SDK 框架。无论是修复 Bug、新增特性还是改进文档，我们都欢迎你的贡献。开始之前，请先阅读本指南；如有疑问，欢迎提一个带 `question` 标签的 [Issue](https://github.com/Kirky-X/sdforge/issues)。

## 🧰 环境准备

### 前置条件

- **Rust 1.97.1+**（edition 2024，工具链见 `rust-toolchain.toml`）
- **protoc**（gRPC 特性编译需要，`sudo apt-get install protobuf-compiler`）
- **pre-commit hooks**（安装：`./scripts/install-pre-commit.sh`）
- 安全与质量工具：
  - `cargo-audit`（依赖漏洞扫描）：`cargo install cargo-audit`
  - `cargo-deny`（依赖许可/漏洞策略）：`cargo install cargo-deny`
  - `cargo-llvm-cov`（覆盖率）：`cargo install cargo-llvm-cov`

### 初始化

```bash
# 克隆仓库
git clone https://github.com/Kirky-X/sdforge.git
cd sdforge

# 安装 pre-commit 钩子（lefthook）
./scripts/install-pre-commit.sh

# 验证环境
cargo build --all-features
cargo test --all-features --lib

# 检查格式与 Lint
cargo fmt --check
cargo clippy --all-features --all-targets
```

## 🔁 开发工作流（TDD）

### 1. 创建分支

```bash
git checkout -b feat/<功能名>   # 或 chore/<任务名>
```

### 2. Red → Green → Commit → Analyze → Next

每个开发任务组必须遵循以下循环：

1. **定接口** — 先定义 trait / API 签名（`trait Xxx { ... }`），不写实现
2. **写测试** — 基于接口编写单元测试（`#[cfg(test)] mod tests { ... }`），此时测试应失败（red）
3. **写代码** — 实现接口，使测试通过（green）
4. **跑测试** — `cargo test --features <对应特性> --lib`，确保所有测试通过
5. **commit** — 通过后执行 `git add . && git commit -m "feat(<模块>): <描述>"`
6. **gitnexus analyze** — 用 gitnexus 工具分析本任务对其他模块的影响，识别需联动修改的代码
7. **继续下一个** — 基于 analyze 结果调整后续任务，再开始下一轮循环

### 3. 测试要求

- 为所有新功能编写测试
- 单元测试内嵌在源文件 `#[cfg(test)] mod tests` 中；集成测试放 `tests/integration/`
- 确保所有特性组合可编译：`cargo test --features "<feature>"`
- 覆盖率目标：核心逻辑 80%+，工具代码 70%+

### 4. 特性组合校验

新增特性时，验证以下组合仍可编译：

| 组合 | 用途 |
|------|------|
| `http` | 仅 HTTP |
| `mcp` | 仅 MCP |
| `http,mcp` | 双协议 |
| `http,security` | HTTP + 安全 |
| `http,cache` | HTTP + 缓存 |
| `http,websocket` | HTTP + WebSocket |
| `http,grpc` | HTTP + gRPC |
| `http,streaming` | HTTP + SSE |
| `full` | 全部特性 |

## 📐 代码规范

### 命名与组织

- 变量与函数使用 `snake_case`，类型与 trait 使用 `PascalCase`
- 遵循现有的模块组织方式（`mod.rs` 与内联样式混用）
- 提交前使用 `cargo fmt` 格式化代码
- 运行 `cargo clippy --all-features -- -D warnings` 检查问题
- 为新功能添加测试，根据需要更新文档

### 通用约定

- `Arc<dyn Trait>` 用于依赖注入
- trait 继承 `Send + Sync`
- 使用 `&self` 而非 `&mut self`
- 返回 `Option` 或 `Result`
- 实现 `Default` trait
- **依赖必须通过 feature 门控** — 所有可选依赖使用 `optional = true` 并在 `[features]` 中门控
- 新特性应尽量独立、在 `Cargo.toml` 中声明依赖、使用 `#[cfg(feature = "...")]` 条件编译，且不得破坏现有特性组合

### 构造模式

所有组件必须支持三种构造模式：

```rust
// 模式 1：开箱即用
let component = Component::new();

// 模式 2：Builder 模式
let component = Component::builder()
    .with_option(value)
    .build();

// 模式 3：完全依赖注入
let component = Component::with_dependencies(dep_a, dep_b);
```

## 📮 提交与 PR 流程

### 提交信息

遵循 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/)：

```
type(scope): subject

type: feat, fix, refactor, docs, test, ci, chore, perf, build, revert, style
scope: 可选的模块名
```

示例：

```
feat(security): add API key rotation support
fix(websocket): resolve connection leak on disconnect
docs(readme): update installation instructions
```

### Pre-commit 检查

项目使用 lefthook 执行本地检查：

```bash
# 安装钩子
./scripts/install-pre-commit.sh   # 或 lefthook install

# 手动运行检查
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check
cargo audit
```

### PR 流程

1. **创建 feature 分支** — `git checkout -b feat/<功能名>` 或 `chore/<任务名>`
2. **变更影响公开 API 时更新文档**，适用时更新 CHANGELOG
3. **确保 pre-commit hooks 通过** — 所有检查必须绿色
4. **确保 CI 通过** — `cargo fmt --check`、`cargo clippy --all-features -- -D warnings`、`cargo test --all-features`
5. **提交 PR** — 使用 [.github/PULL_REQUEST_TEMPLATE.md](../.github/PULL_REQUEST_TEMPLATE.md) 模板，包含清晰的变更说明，关联相关 issue
6. **代码审查** — 至少一名审查者批准后合并

### 发布流程

任何 tag 发布或 release 前，必须完成以下审查流程：

1. **tiangang SAST 扫描** — 0 个 CRITICAL 漏洞才允许继续
2. **diting 代码审查** — 无 HIGH 级别问题才允许打 tag
3. 审查结果必须贴出输出证据，禁止默认通过
4. 审查失败时修复问题后重新走完整流程，不得跳过

## 📜 行为准则

- **禁止使用 `--no-verify` 跳过** pre-commit hooks
- **禁止 `git push --force` 到 main/master 分支**
- pre-commit 包含 `no-commit-to-branch`（保护 main 分支），必须创建 feature 分支提交
- 代码质量工具是项目强制门禁：**diting**（简化/架构/性能审查）、**tiangang**（SAST 安全扫描，发布前强制）、**kueiku**（bug 分析/根因分析/FMEA）
- 提交即代表你同意你的贡献将基于 [MIT 许可证](../LICENSE) 授权
- 有任何疑问，欢迎提一个带 `question` 标签的 [Issue](https://github.com/Kirky-X/sdforge/issues)
