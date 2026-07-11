# 贡献指南

感谢你对 SDForge 项目的兴趣！本文档描述了参与开发所需的环境、工作流和规范。

## 开发环境

- **Rust 1.85+**（edition 2024）
- **protoc**（gRPC 特性编译需要，`sudo apt-get install protobuf-compiler`）
- **pre-commit hooks**（安装：`./scripts/install-pre-commit.sh`）

```bash
# 克隆仓库
git clone https://github.com/Kirky-X/sdforge.git
cd sdforge

# 安装 pre-commit 钩子
./scripts/install-pre-commit.sh

# 验证环境
cargo build --all-features
cargo test --all-features --lib
```

## TDD 工作流

每个开发任务组必须遵循以下循环（Red → Green → Commit → Analyze → Next）：

1. **定接口** — 先定义 trait / API 签名（`trait Xxx { ... }`），不写实现
2. **写测试** — 基于接口编写单元测试（`#[cfg(test)] mod tests { ... }`），此时测试应失败（red）
3. **写代码** — 实现接口，使测试通过（green）
4. **跑测试** — `cargo test --features <对应特性> --lib`，确保所有测试通过
5. **commit** — 通过后执行 `git add . && git commit -m "feat(<模块>): <描述>"`
6. **gitnexus analyze** — 用 gitnexus 工具分析本任务对其他模块的影响，识别需联动修改的代码
7. **继续下一个** — 基于 analyze 结果调整后续任务，再开始下一轮循环

## Pre-commit Hooks

- **禁止使用 `--no-verify` 跳过** pre-commit hooks
- **禁止 `git push --force` 到 main/master 分支**
- pre-commit 包含 `no-commit-to-branch`（保护 main 分支），需创建 feature 分支提交

## 代码质量

项目使用以下工具进行代码质量保障：

- **diting** — 代码简化、架构优化、性能审查、过度工程检测
- **tiangang** — SAST 安全扫描（发布前强制执行，0 CRITICAL 才允许继续）
- **kueiku** — 硬性 bug 分析、根因分析、FMEA 分析

## Pull Request 流程

1. **创建 feature 分支** — `git checkout -b feat/<功能名>` 或 `chore/<任务名>`
2. **确保 pre-commit hooks 通过** — 所有检查必须绿色
3. **确保 CI 通过** — `cargo fmt --check`、`cargo clippy --all-features -- -D warnings`、`cargo test --all-features`
4. **提交 PR** — 包含清晰的变更说明，关联相关 issue
5. **代码审查** — 至少一名审查者批准后合并

## 代码风格

- 提交前使用 `cargo fmt` 格式化代码
- 运行 `cargo clippy --all-features -- -D warnings` 检查问题
- 遵循现有的代码风格和模式
- 为新功能添加测试
- 根据需要更新文档

### 代码规范

- `Arc<dyn Trait>` 用于依赖注入
- trait 继承 `Send + Sync`
- 使用 `&self` 而非 `&mut self`
- 返回 `Option` 或 `Result`
- 实现 `Default` trait
- **依赖必须通过 feature 门控** — 所有可选依赖使用 `optional = true` 并在 `[features]` 中门控

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

## 发布流程

任何 tag 发布或 release 前，必须完成以下审查流程：

1. **tiangang SAST 扫描** — 0 个 CRITICAL 漏洞才允许继续
2. **diting 代码审查** — 无 HIGH 级别问题才允许打 tag
3. 审查结果必须贴出输出证据，禁止默认通过
4. 审查失败时修复问题后重新走完整流程，不得跳过
