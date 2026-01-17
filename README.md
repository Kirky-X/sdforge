<div align="center" id="readme">

<img src="resource/sdforge.png" alt="SDForge Logo" width="200" height="200">

[![Crates.io](https://img.shields.io/crates/v/sdforge)](https://crates.io/crates/sdforge) [![Documentation](https://img.shields.io/docsrs/sdforge)](https://docs.rs/sdforge) [![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](LICENSE) [![Build Status](https://img.shields.io/github/actions/workflow/status/sdforge-rs/sdforge/ci.yml?branch=main)](https://github.com/sdforge-rs/sdforge/actions) [![Rust Version](https://img.shields.io/badge/rust-2021%2B-orange.svg)](https://www.rust-lang.org)

**SDForge** is a Rust-based declarative SDK framework that uses procedural macros to automatically generate multi-protocol service interfaces (HTTP + MCP) from unified function annotations. The key innovation is compile-time protocol selection via Cargo features—unused protocols produce zero compiled code.

</div>

## 📋 Table of Contents

<style>
.back-to-top {
  position: fixed;
  bottom: 20px;
  right: 20px;
  padding: 10px 20px;
  background-color: #007bff;
  color: white;
  text-decoration: none;
  border-radius: 5px;
  font-size: 14px;
  box-shadow: 0 2px 5px rgba(0,0,0,0.2);
  transition: background-color 0.3s;
  z-index: 1000;
}

.back-to-top:hover {
  background-color: #0056b3;
}

.doc-nav {
  padding: 10px 0;
  margin-top: 20px;
  border-top: 1px solid #e1e4e8;
}
</style>

- [✨ Features](#features)
- [📦 Installation](#installation)
- [🚀 Quick Start](#quick-start)
- [⚙️ Feature System](#feature-system)
- [💡 Usage Examples](#usage-examples)
- [📁 Module Prefixes](#module-prefixes)
- [🔢 Version Management](#version-management)
- [⚠️ Error Handling](#error-handling)
- [🛤️ Path Parameters](#path-parameters)
- [🔨 Building and Testing](#building-and-testing)
- [📚 Documentation](#documentation)
- [🤝 Contributing](#contributing)
- [📜 License](#license)
- [📂 Project Structure](#-project-structure)
- [🔗 Links](#-links)

---

## <span id="features">✨ Features</span>

<div style="border-radius:8px; padding:16px; border:1px solid #E2E8F0;">

- **🎯 Unified Interface Definition** - Single macro configuration for both HTTP and MCP protocols
- **⚡ Compile-Time Protocol Selection** - Feature-gated code generation with zero runtime overhead for unused protocols
- **🔒 Type Safety** - Compile-time validation of interface definitions
- **🌐 Multi-Protocol Support** - HTTP (Axum), MCP, gRPC, WebSocket, SSE streaming
- **🧩 Modular Design** - Feature-based architecture allows selecting only needed functionality
- **🛡️ Security Features** - Built-in authentication, rate limiting, and request validation
- **💾 Caching** - In-memory and Redis caching support
- **🔧 Configuration Management** - Hot-reloadable TOML-based configuration
- **📊 Versioning** - Built-in API version management

</div>

---

## <span id="installation">📦 Installation</span>

<div style="border-radius:8px; padding:16px; border:1px solid #E2E8F0;">

Add SDForge to your `Cargo.toml`:

```toml
[dependencies]
sdforge = { version = "0.2", features = ["http"] }
```

**CLI Tool**: To use the CLI, enable the `cli` feature:
```toml
sdforge = { version = "0.2", features = ["cli"] }
```

Then run:
```bash
cargo run --features cli -- --help
```

</div>

---

## <span id="quick-start">🚀 Quick Start</span>

<div style="border-radius:8px; padding:16px; border:1px solid #E2E8F0;">

Define your API with a single macro:

```rust
use sdforge::prelude::*;

#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user",
    description = "Get a user by ID"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    Ok(User { id, name: "Test".into() })
}

#[tokio::main]
async fn main() {
    let app = sdforge::http::build();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

</div>

---

## <span id="feature-system">⚙️ Feature System</span>

SDForge uses Cargo features for compile-time protocol selection and feature composition.

### 🔧 Core Features

| Feature      | Description                              | Dependencies                                   |
|--------------|------------------------------------------|-----------------------------------------------|
| `http`       | HTTP server (Axum 0.8.8)                 | axum, tower, tower-http                        |
| `mcp`        | MCP protocol (mcp-sdk 0.0.3)             | mcp-sdk                                       |
| `streaming`  | SSE streaming support                    | tokio-stream, futures-util                    |
| `timestamp`  | Auto-add timestamp to responses          | chrono                                        |
| `logging`    | Structured request logging                | tracing, tracing-subscriber, tracing-appender |
| `security`   | Security features (auth, rate limiting)   | dashmap, uuid, hmac, sha2, secrecy           |
| `hot-reload` | Config hot reload                        | notify                                        |
| `websocket`  | WebSocket support                        | tokio-tungstenite, axum-extra                |
| `grpc`       | gRPC support                             | tonic, prost                                 |
| `cache`      | Caching support                          | cached, cached_proc_macro                     |
| `cache-redis`| Redis caching                           | redis                                         |
| `full`       | All features enabled                     | -                                             |

### 🔗 Feature Dependencies

- `default`: [`http`]
- `mcp`: No dependencies
- `streaming`: Requires `http`
- `timestamp`: No dependencies
- `logging`: No dependencies
- `security`: Requires `http`
- `hot-reload`: Requires `http`
- `websocket`: Requires `http`, `streaming`
- `grpc`: Requires `http`
- `cache`: Requires `http`
- `cache-redis`: Requires `cache`

---

## 💡 Usage Examples

### 🌐 HTTP Only

For traditional REST APIs:

```toml
[dependencies]
sdforge = { version = "0.1", features = ["http"] }
```

### 🤖 MCP Only

For AI tool integration:

```toml
[dependencies]
sdforge = { version = "0.1", features = ["mcp"] }
```

### 🔄 Both Protocols

Exposure via both HTTP and MCP from the same code:

```toml
[dependencies]
sdforge = { version = "0.1", features = ["http", "mcp"] }
```

### 🎯 Full Features

All capabilities enabled:

```toml
[dependencies]
sdforge = { version = "0.1", features = ["full"] }
```

---

## <span id="module-prefixes">📁 Module Prefixes</span>

Group related APIs with module prefixes for better organization:

```rust
#[service_module(prefix = "/auth")]
mod auth_api {
    use super::*;

    #[service_api(
        name = "login",
        version = "v1",
        path = "/login",
        method = "POST"
    )]
    async fn login(credentials: Credentials) -> Result<Token, ApiError> {
        // Implementation
        Ok(Token::new())
    }

    #[service_api(
        name = "logout",
        version = "v1",
        path = "/logout",
        method = "POST"
    )]
    async fn logout() -> Result<(), ApiError> {
        // Implementation
        Ok(())
    }
}
```

This creates the endpoints:
- `/auth/api/v1/login`
- `/auth/api/v1/logout`

---

## <span id="version-management">🔢 Version Management</span>

Support multiple API versions simultaneously:

```rust
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user_v1"
)]
async fn get_user_v1(id: u64) -> Result<UserV1, ApiError> {
    Ok(UserV1 { id, name: "John Doe".into() })
}

#[service_api(
    name = "get_user",
    version = "v2",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user_v2"
)]
async fn get_user_v2(id: u64) -> Result<UserV2, ApiError> {
    Ok(UserV2 { id, first_name: "John".into(), last_name: "Doe".into() })
}
```

This creates versioned endpoints:
- `/api/v1/users/:id` → `get_user_v1`
- `/api/v2/users/:id` → `get_user_v2`

---

## <span id="error-handling">⚠️ Error Handling</span>

Define custom error types and convert them to `ServiceError`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Validation failed: {field}")]
    ValidationError { field: String },

    #[error("Unauthorized access")]
    Unauthorized,
}

impl From<MyError> for ServiceError {
    fn from(err: MyError) -> Self {
        match err {
            MyError::NotFound { resource } => ServiceError::with_details(
                "NOT_FOUND",
                format!("Resource not found: {}", resource),
                serde_json::json!({ "resource": resource }),
                404,
            ),
            MyError::ValidationError { field } => ServiceError::with_details(
                "VALIDATION_ERROR",
                format!("Validation failed for field: {}", field),
                serde_json::json!({ "field": field }),
                400,
            ),
            MyError::Unauthorized => ServiceError::new(
                "UNAUTHORIZED",
                "Authentication required",
                401,
            ),
        }
    }
}
```

---

## <span id="path-parameters">🛤️ Path Parameters</span>

Extract path parameters using Rust naming conventions. The macro automatically maps path segments to function parameters:

```rust
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    // `id` is automatically extracted from `/users/:id`
    Ok(User { id, name: "John".into() })
}

#[service_api(
    name = "get_comment",
    version = "v1",
    path = "/posts/:post_id/comments/:comment_id",
    method = "GET"
)]
async fn get_comment(
    post_id: u64,
    comment_id: u64
) -> Result<Comment, ApiError> {
    // Both parameters are extracted from the path
    Ok(Comment { post_id, comment_id, text: "Test".into() })
}
```

### 🔹 Multiple Path Parameters

For nested resources:

```rust
#[service_api(
    name = "get_nested_resource",
    version = "v1",
    path = "/orgs/:org_id/projects/:project_id/tasks/:task_id",
    method = "GET"
)]
async fn get_task(
    org_id: u64,
    project_id: u64,
    task_id: u64
) -> Result<Task, ApiError> {
    Ok(Task { org_id, project_id, task_id, title: "Task".into() })
}
```

---

## <span id="building-and-testing">🔨 Building and Testing</span>

### 🔧 Build Commands

```bash
# Build with HTTP only
cargo build --features http

# Build with MCP only
cargo build --features mcp

# Build with all features
cargo build --features full

# Build with custom feature set
cargo build --features "http,cache,security"

# Release build
cargo build --release --features http
```

### 🧪 Test Commands

```bash
# Run tests with HTTP
cargo test --features http

# Run tests with MCP
cargo test --features mcp

# Run tests with both protocols
cargo test --features "http,mcp"

# Run tests with all features
cargo test --features full

# Run specific test
cargo test test_get_user --features http

# Run tests with output
cargo test --features http -- --nocapture

# Run tests in release mode
cargo test --release --features http
```

### ✨ Formatting and Linting

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run Clippy
cargo clippy --all-features

# Run Clippy with all targets
cargo clippy --all-features --all-targets
```

---

## <span id="documentation">📚 Documentation</span>

- [📖 API Documentation](https://docs.rs/sdforge)
- [📋 API Reference](./docs/API_REFERENCE.md)
- [🏗️ Architecture Documentation](./docs/ARCHITECTURE.md)
- [🤝 Contributing Guide](./docs/CONTRIBUTING.md)
- [💡 Examples](./examples/)

---

## <span id="contributing">🤝 Contributing</span>

We welcome contributions! Please read our [contributing guidelines](./docs/CONTRIBUTING.md) before submitting pull requests.

### 🛠️ Development Setup

```bash
# Clone the repository
git clone https://github.com/sdforge-rs/sdforge.git
cd sdforge

# Install pre-commit hooks
./scripts/install-pre-commit.sh

# Install development tools
cargo install cargo-watch cargo-edit

# Run tests
cargo test --all-features
```

### 📝 Code Style

- Format code with `cargo fmt` before committing
- Run `cargo clippy --all-features` to check for issues
- Follow the existing code style and patterns
- Add tests for new features
- Update documentation as needed

---

## <span id="license">📜 License</span>

Licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0
- [MIT License](LICENSE-MIT) or http://opensource.org/licenses/MIT

at your option.

---

## <span id="project-structure">📂 Project Structure</span>

```
sdforge/
├── src/                # Main framework crate
│   ├── core/         # Core types and error handling
│   ├── http/         # HTTP protocol implementation
│   ├── mcp/          # MCP protocol implementation
│   ├── security/     # Security features
│   ├── cache/        # Caching implementation
│   ├── websocket/    # WebSocket support
│   ├── grpc/         # gRPC support
│   ├── config/       # Configuration management
│   ├── cli/          # CLI tool (optional, requires `cli` feature)
│   ├── lib.rs        # Library entry point
│   └── main.rs       # CLI binary entry point
├── macros/            # Procedural macros crate
│   ├── src/
│   └── Cargo.toml
├── docs/              # Documentation
├── .github/           # GitHub workflows
└── scripts/           # Build and utility scripts
```

**Note**: The CLI binary is only compiled when the `cli` feature is enabled:
```toml
sdforge = { version = "0.2", features = ["cli"] }
```

---

## <span id="links">🔗 Links</span>

- **🏠 Repository**: https://github.com/sdforge-rs/sdforge
- **📖 Documentation**: https://docs.rs/sdforge
- **🐛 Issues**: https://github.com/sdforge-rs/sdforge/issues
- **💬 Discussions**: https://github.com/sdforge-rs/sdforge/discussions

---

<div align="center">

**Built with ❤️ using Rust**

</div>

---

<div align="center">

[🔝 back to top](#readme) | [🇨🇳 中文版](./README_zh.md)

</div>

<style>
.back-to-top {
  position: fixed;
  bottom: 20px;
  right: 20px;
  padding: 10px 20px;
  background-color: #007bff;
  color: white;
  text-decoration: none;
  border-radius: 5px;
  font-size: 14px;
  box-shadow: 0 2px 5px rgba(0,0,0,0.2);
  transition: background-color 0.3s;
  z-index: 1000;
}

.back-to-top:hover {
  background-color: #0056b3;
}
</style>
