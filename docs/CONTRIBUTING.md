# Contributing to SDForge

Thank you for your interest in contributing to SDForge! This document outlines the process for contributing to the project.

## Development Setup

### Prerequisites

- Rust stable toolchain (see `rust-toolchain.toml`)
- `cargo-audit` for security scanning: `cargo install cargo-audit`
- `cargo-deny` for dependency policy: `cargo install cargo-deny`
- `cargo-llvm-cov` for coverage: `cargo install cargo-llvm-cov`

### Getting Started

```bash
# Clone the repository
git clone https://github.com/Kirky-X/sdforge.git
cd sdforge

# Verify the build
cargo build --features full

# Run tests
cargo test --features full

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --all-features --all-targets
```

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feat/your-feature-name
```

### 2. Make Changes

Follow the coding conventions:

- Use `snake_case` for variables and functions
- Use `PascalCase` for types and traits
- Follow the existing module organization (mixed `mod.rs` and inline styles)
- Implement `Default` trait for configurable types
- Use `Arc<dyn Trait>` for dependency injection
- All traits must be `Send + Sync`
- Use `&self` instead of `&mut self`
- Return `Option` or `Result` for fallible operations

### 3. Three Construction Patterns

All components must support three construction patterns:

```rust
// Pattern 1: Out-of-the-box
let component = Component::new();

// Pattern 2: Builder pattern
let component = Component::builder()
    .with_option(value)
    .build();

// Pattern 3: Full dependency injection
let component = Component::with_dependencies(dep_a, dep_b);
```

### 4. Feature Gating

SDForge uses Cargo features for compile-time protocol selection. New features should:

- Be independent where possible
- Document dependencies in `Cargo.toml`
- Use `#[cfg(feature = "...")]` for conditional compilation
- Not break existing feature combinations

### 5. Testing

- Write tests for all new functionality
- Tests are embedded in source files under `#[cfg(test)] mod tests`
- Integration tests go in `tests/integration/`
- Ensure all feature combinations compile: `cargo test --features "<feature>"`
- Coverage threshold: 80%+ for core logic, 70%+ for utilities

### 6. Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): subject

type: feat, fix, refactor, docs, test, ci, chore, perf, build, revert, style
scope: optional module name
```

Examples:
```
feat(security): add API key rotation support
fix(websocket): resolve connection leak on disconnect
docs(readme): update installation instructions
```

### 7. Pre-commit Checks

The project uses lefthook (or pre-commit) for local checks:

```bash
# Install hooks
lefthook install  # or pre-commit install

# Run checks manually
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check
cargo audit
```

## Pull Request Process

1. **Update documentation** if your changes affect the public API
2. **Add tests** for new functionality
3. **Ensure all checks pass**:
   ```bash
   cargo fmt --check
   cargo clippy --all-features --all-targets
   cargo test --features full
   ```
4. **Update CHANGELOG** if applicable
5. **Link related issues** in your PR description

### PR Template

See `.github/PULL_REQUEST_TEMPLATE.md` for the PR template structure.

## Feature Combinations

When adding features, verify these combinations still compile:

| Combination | Purpose |
|-------------|---------|
| `http` | HTTP-only |
| `mcp` | MCP-only |
| `http,mcp` | Both protocols |
| `http,security` | HTTP with security |
| `http,cache` | HTTP with caching |
| `http,websocket` | HTTP with WebSocket |
| `http,grpc` | HTTP with gRPC |
| `http,streaming` | HTTP with SSE |
| `full` | All features |

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

## Questions?

Feel free to open an issue with the `question` label if you have any questions.
