<div align="center">

# 🤝 Contributing to Axiom

**Thank you for your interest in contributing to Axiom!**

This document provides guidelines and instructions for contributing to the project.

</div>

---

## 📋 Table of Contents

- [🚀 Getting Started](#-getting-started)
- [🔄 Development Workflow](#-development-workflow)
- [📝 Coding Standards](#-coding-standards)
- [🧪 Testing](#-testing)
- [💬 Commit Message Conventions](#-commit-message-conventions)
- [📤 Pull Request Process](#-pull-request-process)
- [👀 Code Review Guidelines](#-code-review-guidelines)
- [🪝 Pre-commit Hooks](#-pre-commit-hooks)

---

## 🚀 Getting Started

### 📦 Prerequisites

- **Rust**: Latest stable version (2021 edition)
- **Git**: Version 2.20 or higher
- **Pre-commit**: For code quality checks

### 🛠️ Setup Development Environment

```bash
# Clone the repository
git clone https://github.com/Kirky-X/sdforge.git
cd sdforge

# Install Rust toolchain
rustup install stable
rustup default stable

# Install development tools
cargo install cargo-watch cargo-edit cargo-expand

# Install pre-commit hooks
./scripts/install-pre-commit.sh
```

### 🏗️ Build the Project

```bash
# Build with default features
cargo build

# Build with all features
cargo build --features full

# Build release version
cargo build --release --features full
```

### 🧪 Run Tests

```bash
# Run all tests
cargo test --all-features

# Run tests with output
cargo test --all-features -- --nocapture

# Run specific test
cargo test test_get_user --features http
```

---

## 🔄 Development Workflow

### 🌿 Branch Strategy

- `main`: Stable, production-ready code
- `develop`: Development branch for next release
- Feature branches: `feature/<description>` (e.g., `feature/websocket-support`)
- Bugfix branches: `fix/<description>` (e.g., `fix/memory-leak`)
- Documentation: `docs/<description>` (e.g., `docs/api-reference`)

### 📝 Workflow Steps

1. **Create a feature branch** from `develop`
   ```bash
   git checkout develop
   git pull origin develop
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following coding standards

3. **Run tests and checks**
   ```bash
   # Format code
   cargo fmt

   # Check formatting
   cargo fmt --check

   # Run Clippy
   cargo clippy --all-features

   # Run tests
   cargo test --all-features
   ```

4. **Commit your changes** using the commit message conventions

5. **Push to origin**
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Create a Pull Request** following the PR process

### 🛠️ Development Tools

#### 👀 Watch Mode

Use `cargo-watch` for automatic rebuilds during development:

```bash
# Watch for changes and run tests
cargo watch -x test -x check

# Watch for changes and run clippy
cargo watch -x clippy
```

#### 🔍 Macro Expansion

To debug procedural macros:

```bash
# Install cargo-expand
cargo install cargo-expand

# Expand macros in a file
cargo expand --example my_example

# Expand with specific features
cargo expand --features "http,mcp" --example my_example
```

---

## 📝 Coding Standards

### 🦀 Rust Style Guide

Follow these Rust coding standards:

1. **Edition**: Use Rust 2021 edition
2. **Formatting**: Always run `cargo fmt` before committing
3. **Linting**: Address all `clippy` warnings
4. **Documentation**: Document public APIs with `///` or `//!`
5. **Error Handling**: Use `Result<T, E>` and `thiserror` for custom errors
6. **Async**: Use `tokio` for async operations

### ✨ Code Formatting

Axiom uses `rustfmt` with the configuration in `rustfmt.toml`:

```toml
edition = "2021"
max_width = 100
tab_spaces = 4
hard_tabs = false
newline_style = "Unix"
```

Always format your code before committing:

```bash
cargo fmt
```

### 📚 Documentation Standards

#### 📖 Public APIs

All public functions, structs, and traits must be documented:

```rust
/// Retrieves a user by their unique identifier.
///
/// # Arguments
///
/// * `id` - The unique identifier of the user
///
/// # Returns
///
/// Returns `Ok(User)` if the user is found, otherwise returns `Err(ApiError)`.
///
/// # Errors
///
/// Returns `ApiError::NotFound` if the user does not exist.
///
/// # Examples
///
/// ```
/// let user = get_user(123).await?;
/// println!("User: {}", user.name);
/// ```
pub async fn get_user(id: u64) -> Result<User, ApiError> {
    // Implementation
}
```

#### 📁 Module Documentation

Document the purpose of each module:

```rust
//! HTTP protocol implementation for Axiom framework.
//!
//! This module provides HTTP server functionality using Axum,
//! including route registration, middleware, and request handling.

pub mod routing;
pub mod middleware;
pub mod handlers;
```

### 🏷️ Naming Conventions

- **Types**: `PascalCase` (e.g., `UserService`, `ApiError`)
- **Functions**: `snake_case` (e.g., `get_user`, `create_handler`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_RETRIES`)
- **Modules**: `snake_case` (e.g., `http`, `mcp`, `security`)
- **Features**: `kebab-case` (e.g., `hot-reload`, `cache-redis`)

### ❌ Error Handling

Use `thiserror` for custom error types:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Validation failed: {field}")]
    ValidationError { field: String, reason: String },

    #[error("Internal error: {0}")]
    Internal(#[from] Box<dyn std::error::Error + Send + Sync>),
}
```

---

## 🧪 Testing

### 📂 Test Organization

```
axiom/
├── tests/              # Integration tests
│   ├── integration.rs
│   └── e2e.rs
└── src/
    └── core/
        └── mod.rs     # Unit tests
```

### 🔬 Unit Tests

Write unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_user_success() {
        let result = get_user(123).await;
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.id, 123);
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let result = get_user(999).await;
        assert!(result.is_err());
    }
}
```

### 🔗 Integration Tests

Place integration tests in the `tests/` directory:

```rust
// tests/integration.rs
use sdforge::prelude::*;

#[tokio::test]
async fn test_http_endpoint() {
    // Test HTTP endpoint
}

#[tokio::test]
async fn test_mcp_tool() {
    // Test MCP tool
}
```

### 📊 Test Coverage

- Aim for **80%+ code coverage**
- Test both success and error paths
- Test edge cases and boundary conditions
- Use property-based testing with `proptest` for complex logic

### 🚀 Running Tests

```bash
# Run all tests
cargo test --all-features

# Run tests with coverage
cargo install tarpaulin
cargo tarpaulin --out Html --features full

# Run specific test
cargo test test_get_user --features http

# Run tests in release mode
cargo test --release --features full

# Run tests with output
cargo test --all-features -- --nocapture
```

---

## 💬 Commit Message Conventions

We follow [Conventional Commits](https://www.conventionalcommits.org/) specification.

### 📝 Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### 🏷️ Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Test additions or changes
- `chore`: Maintenance tasks
- `ci`: CI/CD changes

### 💡 Examples

```
feat(http): add websocket support

Implement WebSocket connections with upgrade handling.

Closes #123
```

```
fix(mcp): resolve memory leak in tool registration

The inventory system was not properly releasing references to
registered tools, causing memory to accumulate over time.

Fixes #456
```

```
docs(api): update service_api macro documentation

Clarify path parameter extraction and add examples.
```

```
refactor(security): simplify auth middleware logic

Extract common authentication logic into separate module.
```

### ✅ Commit Checklist

Before committing, ensure:

- [ ] Code is formatted with `cargo fmt`
- [ ] No clippy warnings: `cargo clippy --all-features`
- [ ] Tests pass: `cargo test --all-features`
- [ ] Documentation is updated
- [ ] Commit message follows conventions

---

## 📤 Pull Request Process

### 📋 Before Creating a PR

1. **Update documentation** if your changes affect user-facing APIs
2. **Add tests** for new functionality
3. **Run all checks** and ensure they pass
4. **Rebase** your branch onto the latest `develop`:
   ```bash
   git fetch origin
   git rebase origin/develop
   ```

### 📝 PR Title and Description

**Title**: Follow commit message conventions
```
feat(http): add websocket support
```

**Description Template**:

```markdown
## Summary
Brief description of the changes

## Changes
- Bullet list of changes
- Include breaking changes if any

## Testing
- Description of tests added
- How to manually test

## Related Issues
Closes #123
Related to #456
```

### 👀 PR Review Process

1. **Automated checks** must pass:
   - CI/CD pipeline
   - Pre-commit hooks
   - Code coverage

2. **Code review** by maintainers:
   - One approval required
   - Address review comments
   - Update tests if needed

3. **Merge**:
   - Squash and merge to `develop`
   - Update changelog

---

## 👀 Code Review Guidelines

### 📋 For Reviewers

1. **Review checklist**:
   - [ ] Code follows coding standards
   - [ ] Tests are adequate
   - [ ] Documentation is updated
   - [ ] No security vulnerabilities
   - [ ] Performance is acceptable

2. **Provide constructive feedback**:
   - Be specific about issues
   - Suggest improvements
   - Explain reasoning

3. **Approvals**:
   - Maintain: One approval required
   - Feature changes: Two approvals required
   - Breaking changes: Consensus required

### ✍️ For Authors

1. **Respond to feedback**:
   - Address all review comments
   - Explain why if you disagree
   - Update tests and documentation

2. **Keep PR focused**:
   - Small, focused PRs review faster
   - Split large changes into multiple PRs

3. **Test your changes**:
   - Run tests before updating PR
   - Test on multiple platforms if applicable

---

## 🪝 Pre-commit Hooks

We use pre-commit hooks to ensure code quality. Hooks are automatically installed via `./scripts/install-pre-commit.sh`.

### 🔍 Hook Details

The `.pre-commit-config.yaml` includes:

- **Merge conflict detection**: Prevent commits with conflict markers
- **Large file detection**: Warn about files >1MB
- **Whitespace cleanup**: Remove trailing whitespace
- **YAML syntax check**: Validate YAML files
- **TOML format check**: Validate TOML files
- **Rust formatting**: Run `rustfmt`
- **Clippy linting**: Run `clippy` with all features
- **Compilation check**: Ensure code compiles
- **Build check**: Ensure project builds successfully

### 🚀 Running Hooks Manually

```bash
# Run all hooks
pre-commit run --all-files

# Run specific hook
pre-commit run rustfmt --all-files

# Skip hooks (not recommended)
git commit --no-verify -m "message"
```

### 🔧 Troubleshooting

#### ❌ Hook Fails

If a hook fails:

1. **Read the error message** carefully
2. **Fix the issue** (format, lint, compilation)
3. **Run the hook manually** to verify fix
4. **Commit again**

#### ❌ Install Issues

If hooks fail to install:

```bash
# Uninstall hooks
pre-commit uninstall

# Clean cache
pre-commit clean

# Reinstall
./scripts/install-pre-commit.sh
```

---

## 📚 Additional Resources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Effective Rust](https://doc.rust-lang.org/book/title-page.html)
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/)
- [Async Rust Book](https://rust-lang.github.io/async-book/)

---

## ❓ Questions?

- **💬 Discussions**: Use GitHub Discussions for questions
- **🐛 Issues**: Report bugs or request features
- **💬 Discord**: Join our community server (link in README)

---

<div align="center">

**Thank you for contributing to Axiom! 🎉**

</div>