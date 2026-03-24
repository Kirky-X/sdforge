<div align="center" id="readme">

<img src="resource/sdforge.png" alt="SDForge Logo" width="200" height="200">

[![Crates.io](https://img.shields.io/crates/v/sdforge)](https://crates.io/crates/sdforge) [![Documentation](https://img.shields.io/docsrs/sdforge)](https://docs.rs/sdforge) [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) [![Build Status](https://img.shields.io/github/actions/workflow/status/Kirky-X/sdforge/ci.yml?branch=main)](https://github.com/Kirky-X/sdforge/actions) [![Rust Version](https://img.shields.io/badge/rust-2021%2B-orange.svg)](https://www.rust-lang.org)

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
- [🔒 Security Configuration](#security-configuration)
- [⚡ Performance Optimization](#performance-optimization)
- [🚀 Production Deployment](#deployment-guide)
- [🐛 Troubleshooting](#troubleshooting)
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
git clone https://github.com/Kirky-X/sdforge.git
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

Licensed under the MIT License:

- [MIT License](LICENSE) or http://opensource.org/licenses/MIT

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

- **🏠 Repository**: https://github.com/Kirky-X/sdforge
- **📖 Documentation**: https://docs.rs/sdforge
- **🐛 Issues**: https://github.com/Kirky-X/sdforge/issues
- **💬 Discussions**: https://github.com/Kirky-X/sdforge/discussions

---

## <span id="security-configuration">🔒 Security Configuration</span>

SDForge provides comprehensive security features out of the box. Here's how to configure them:

### 🛡️ Authentication Setup

```rust
use sdforge::prelude::*;

#[service_api(
    name = "secure_endpoint",
    version = "v1",
    path = "/secure",
    method = "GET",
    auth_required = true
)]
async fn secure_endpoint(
    auth_context: AuthContext
) -> Result<String, ApiError> {
    // Only authenticated users can access this
    Ok(format!("Hello, {}!", auth_context.user_id().unwrap_or("Anonymous")))
}
```

### ⚡ Rate Limiting

```toml
# config.toml
[rate_limit]
enabled = true
requests_per_minute = 60
burst_size = 10
```

### 🔐 API Key Authentication

```rust
use sdforge::security::{ApiKeyAuth, auth_middleware};

let app = Router::new()
    .route("/api/*path", get(handler))
    .layer(auth_middleware(ApiKeyAuth::new("your-secret-key")));
```

---

## <span id="performance-optimization">⚡ Performance Optimization</span>

### 🚀 Caching Configuration

```toml
# config.toml
[cache]
enabled = true
ttl_seconds = 300
max_size_mb = 100
max_entries = 10000

[cache.redis]
url = "redis://localhost:6379"
enabled = false
```

### 📊 Memory Management

```rust
use sdforge::cache::CacheConfig;

let cache_config = CacheConfig {
    ttl_seconds: 600,
    max_size_bytes: 50 * 1024 * 1024, // 50MB
    max_entries: 5000,
    cacheable_methods: vec!["GET".to_string(), "HEAD".to_string()],
    cacheable_status_codes: vec![200, 203, 204, 206, 300, 301, 404, 410],
};
```

### ⚙️ Connection Pooling

```rust
use sdforge::http::build_with_config;

let config = HttpConfig {
    max_connections: 1000,
    connection_timeout: Duration::from_secs(30),
    keep_alive: Duration::from_secs(60),
    ..Default::default()
};

let app = build_with_config(config);
```

---

## <span id="deployment-guide">🚀 Production Deployment</span>

### 🐳 Docker Deployment

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features full

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/sdforge /usr/local/bin/
EXPOSE 3000
CMD ["sdforge", "serve", "--port", "3000"]
```

### ☸️ Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sdforge-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: sdforge-api
  template:
    metadata:
      labels:
        app: sdforge-api
    spec:
      containers:
      - name: sdforge
        image: sdforge:latest
        ports:
        - containerPort: 3000
        env:
        - name: FEATURES
          value: "full"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

### 🔧 Environment Configuration

```bash
# Production environment variables
export RUST_LOG=info
export SD_FORGE_PORT=3000
export SD_FORGE_HOST=0.0.0.0
export SD_FORGE_CONFIG_PATH=/etc/sdforge/config.toml
export SD_FORGE_FEATURES=full
```

---

## <span id="troubleshooting">🐛 Troubleshooting</span>

### 🔍 Common Issues

#### **Compilation Errors**
```bash
# Error: Feature not found
# Solution: Check available features
cargo check --help | grep features

# Enable specific features
cargo build --features "http,security,cache"
```

#### **Runtime Issues**
```bash
# Check logs with tracing
RUST_LOG=debug cargo run --features logging

# Common port conflicts
# Solution: Change port or kill existing process
lsof -i :3000
kill -9 <PID>
```

#### **Performance Issues**
```bash
# Profile with cargo-flamegraph
cargo install flamegraph
cargo flamegraph --bin sdforge --features full

# Memory usage analysis
valgrind --tool=massif target/release/sdforge
```

### 📋 Health Check Endpoint

```rust
#[service_api(
    name = "health_check",
    version = "v1",
    path = "/health",
    method = "GET"
)]
async fn health_check() -> Result<HealthStatus, ApiError> {
    Ok(HealthStatus {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: get_uptime(),
    })
}
```

### 🆘 Getting Help

- 📖 [Documentation](https://docs.rs/sdforge)
- 🐛 [Issue Tracker](https://github.com/Kirky-X/sdforge/issues)
- 💬 [Discussions](https://github.com/Kirky-X/sdforge/discussions)
- 📧 [Support Email](mailto:support@sdforge.dev)

---

<div align="center">

**Built with ❤️ using Rust**

</div>

---

<div align="center">

[🔝 Back To Top](#readme) | [🇨🇳 中文版](./README_zh.md)

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
