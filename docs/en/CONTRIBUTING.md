# 🤝 Sdforge Contributing Guide

Thank you for your interest in the SDForge project! This document describes the environment, workflow, and standards for contributing to development.

## 📋 Table of Contents

<details open>
<summary>📑 Table of Contents</summary>

- [Welcome](#-welcome)
- [Environment Setup](#-environment-setup)
- [Development Workflow (TDD)](#-development-workflow-tdd)
- [Code Standards](#-code-standards)
- [Commit and PR Process](#-commit-and-pr-process)
- [Code of Conduct](#-code-of-conduct)

</details>

## 🙌 Welcome

SDForge is a Rust-based declarative multi-protocol SDK framework. Whether it's fixing bugs, adding features, or improving documentation, your contributions are welcome. Before you start, please read this guide; if you have questions, feel free to open an [Issue](https://github.com/Kirky-X/sdforge/issues) with the `question` label.

## 🧰 Environment Setup

### Prerequisites

- **Rust 1.97.1+** (edition 2024; toolchain in `rust-toolchain.toml`)
- **protoc** (needed to compile the gRPC feature; `sudo apt-get install protobuf-compiler`)
- **Git hooks** (install with `lefthook install`, or `pip install pre-commit && pre-commit install`)
- Security and quality tools:
  - `cargo-audit` (dependency vulnerability scanning): `cargo install cargo-audit`
  - `cargo-deny` (dependency license/vulnerability policy): `cargo install cargo-deny`
  - `cargo-llvm-cov` (coverage): `cargo install cargo-llvm-cov`

### Initialization

```bash
# Clone the repository
git clone https://github.com/Kirky-X/sdforge.git
cd sdforge

# Install Git hooks (choose one)
lefthook install
# or: pip install pre-commit && pre-commit install

# Verify the environment
cargo build --all-features
cargo test --all-features --lib

# Check formatting and lint
cargo fmt --check
cargo clippy --all-features --all-targets
```

## 🔁 Development Workflow (TDD)

### 1. Create a Branch

```bash
git checkout -b feat/<feature-name>   # or chore/<task-name>
```

### 2. Red → Green → Commit → Analyze → Next

Every development task group must follow this loop:

1. **Define the interface** — define the trait / API signatures first (`trait Xxx { ... }`) without implementing
2. **Write tests** — write unit tests against the interface (`#[cfg(test)] mod tests { ... }`); they should fail at this point (red)
3. **Write code** — implement the interface until the tests pass (green)
4. **Run tests** — `cargo test --features <relevant-features> --lib`, ensure all tests pass
5. **commit** — once green, stage only this change's files and commit: `git add <changed files> && git commit -m "feat(<module>): <description>"`
6. **Impact analysis** — analyze the change's impact on other modules and identify code and docs that need coordinated updates
7. **Continue to the next** — adjust the follow-up tasks based on the analysis, then start the next loop iteration

### 3. Test Requirements

- Write tests for all new features
- Unit tests live inline in source files under `#[cfg(test)] mod tests`; integration tests go in `tests/integration/`
- Ensure all feature combinations compile: `cargo test --features "<feature>"`
- Coverage targets: 90%+ for core logic (aligned with the CI gate threshold), 70%+ for tooling code
- **Coverage scope (the single mandatory item)**: the CI coverage job runs
  `cargo llvm-cov --features full --lib --fail-under-lines 90`; the `lefthook.yml`
  pre-push only reuses the same 90% threshold, with the command at default features
  and all targets: `cargo llvm-cov --fail-under-lines 90` (a different measurement
  scope from CI, not the same metric). What Codecov receives is also that one
  `--lib` lcov;
  `codecov/patch` is an **advisory** status (`fail_ci_if_error: false`, does not affect
  CI results), and its target follows Codecov's default policy (auto ≈ the project's
  current coverage) — when a large PR's patch coverage falls below the project average it
  shows red, which is a signal, not a gate. To control that status explicitly,
  add a `codecov.yml` declaring project/patch targets or mark it informational.
- The test pyramid baseline and E2E scenario definitions are in [test scenarios](TEST_SCENARIOS.md)

### 4. Feature Combination Checks

When adding a feature, verify the following combinations still compile:

| Combination | Purpose |
|------|------|
| `http` | HTTP only |
| `mcp` | MCP only |
| `http,mcp` | Dual protocol |
| `http,security` | HTTP + security |
| `http,cache` | HTTP + cache |
| `http,websocket` | HTTP + WebSocket |
| `http,grpc` | HTTP + gRPC |
| `http,streaming` | HTTP + SSE |
| `full` | All features |

## 📐 Code Standards

### Naming and Organization

- Use `snake_case` for variables and functions, `PascalCase` for types and traits
- Follow the existing module organization (a mix of `mod.rs` and inline styles)
- Run `cargo fmt` before committing
- Run `cargo clippy --all-features -- -D warnings` to check for issues
- Add tests for new features and update documentation as needed

### General Conventions

- `Arc<dyn Trait>` for dependency injection
- Traits inherit `Send + Sync`
- Prefer `&self` over `&mut self`
- Return `Option` or `Result`
- Implement the `Default` trait
- **Dependencies must be feature-gated** — all optional dependencies use `optional = true` and are gated in `[features]`
- New features should be as independent as possible, declare their dependencies in `Cargo.toml`, use `#[cfg(feature = "...")]` conditional compilation, and must not break existing feature combinations

### Construction Patterns

All components must support three construction modes:

```rust
// Mode 1: works out of the box
let component = Component::new();

// Mode 2: Builder pattern
let component = Component::builder()
    .with_option(value)
    .build();

// Mode 3: full dependency injection
let component = Component::with_dependencies(dep_a, dep_b);
```

## 📮 Commit and PR Process

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```text
type(scope): subject

type: feat, fix, refactor, docs, test, ci, chore, perf, build, revert, style
scope: optional module name
```

Examples:

```text
feat(security): add API key rotation support
fix(websocket): resolve connection leak on disconnect
docs(readme): update installation instructions
```

### Pre-commit Checks

The project runs local checks through pre-commit or lefthook (`.pre-commit-config.yaml` and `lefthook.yml` are functionally equivalent; pick one):

```bash
# Install hooks (choose one)
lefthook install             # lefthook
# or pip install pre-commit && pre-commit install   # Python pre-commit framework

# Run checks manually
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check
cargo audit
```

### PR Process

1. **Create a feature branch** — `git checkout -b feat/<feature-name>` or `chore/<task-name>`
2. **Update documentation when the change affects public APIs**, and update the CHANGELOG where applicable
3. **Ensure pre-commit hooks pass** — all checks must be green
4. **Ensure CI passes** — `cargo fmt --check`, `cargo clippy --all-features -- -D warnings`, `cargo test --all-features`
5. **Submit the PR** — use the [.github/PULL_REQUEST_TEMPLATE.md](../../.github/PULL_REQUEST_TEMPLATE.md) template, include a clear change description, and link related issues
6. **Code review** — merge after at least one reviewer approves

### Release Process

Before any tag release or release, the following review process must be completed:

1. **SAST security scan** — 0 CRITICAL vulnerabilities required to proceed
2. **Code quality review** — no HIGH-severity issues before tagging
3. Review results must be posted with output evidence; silent pass-through is forbidden
4. If a review fails, fix the issues and rerun the full process; skipping steps is not allowed

## 📜 Code of Conduct

- **Using `--no-verify` to skip pre-commit hooks is forbidden**
- **`git push --force` to main/master branches is forbidden**
- pre-commit does **not** configure a `no-commit-to-branch` hook: this repo's workflow commits directly to main, and that hook only protects `master` — keeping it would have no effect and would only create the misimpression of a guardrail (see the comments in `.pre-commit-config.yaml`)
- Code quality tools are mandatory project gates: **code quality review** (simplification/architecture/performance review), **SAST security scan** (mandatory pre-release), **kueiku** (bug analysis/root-cause analysis/FMEA)
- Submitting a contribution means you agree to license it under the [MIT](../../LICENSE) license
- If you have any questions, feel free to open an [Issue](https://github.com/Kirky-X/sdforge/issues) with the `question` label
