<div align="center" id="readme">

<img src="docs/asset/sdforge.png" alt="SDForge Logo" width="200" height="200">

[![Crates.io](https://img.shields.io/crates/v/sdforge)](https://crates.io/crates/sdforge) [![Documentation](https://img.shields.io/docsrs/sdforge)](https://docs.rs/sdforge) [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) [![Build Status](https://img.shields.io/github/actions/workflow/status/Kirky-X/sdforge/ci.yml?branch=main)](https://github.com/Kirky-X/sdforge/actions) [![Rust Version](https://img.shields.io/badge/rust-2021%2B-orange.svg)](https://www.rust-lang.org)

[中文](./README.md)

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
- [📜 OpenAPI Auto-Generation](#openapi-generation)
- [🔄 MCP 2026-07-28 Migration Guide](#mcp-migration)
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
- **💾 Caching** - In-memory caching (oxcache 0.3.2), no external database required
- **🔧 Configuration Management** - Self-contained TOML configuration (no external config center)
- **📊 Versioning** - Built-in API version management

</div>

### 🆕 Phase 1 Architecture Improvements (v0.1.0)

Recent architectural enhancements include:

- **🔄 Unified Registration System** - Eliminated 95+ lines of duplicate code across HTTP, MCP, WebSocket, and gRPC modules using trait-based abstraction and procedural macros
- **⚙️ Modular Configuration Management** - Refactored configuration into dedicated modules (app, cache, security) with centralized defaults and Builder pattern support
- **🔐 Enhanced Security Module** - API Key versioning, LRU caching, key rotation with audit logging, and comprehensive security headers configuration
- **💾 Advanced Caching** - Pattern-based cache invalidation, key normalization, batch operations, and statistics tracking

---

## <span id="installation">📦 Installation</span>

<div style="border-radius:8px; padding:16px; border:1px solid #E2E8F0;">

Add SDForge to your `Cargo.toml`:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["http"] }
```

</div>

---

## <span id="quick-start">🚀 Quick Start</span>

<div style="border-radius:8px; padding:16px; border:1px solid #E2E8F0;">

Define your API with a single macro:

```rust
use sdforge::prelude::*;

#[forge(
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
| `mcp`        | MCP protocol (rmcp 0.16, 2026-07-28 spec) | rmcp |
| `streaming`  | SSE streaming support                    | tokio-stream, futures-util                    |
| `timestamp`  | Auto-add timestamp to responses          | chrono                                        |
| `logging`    | Structured request logging                | chrono, tokio                                 |
| `security`   | Security features (auth, rate limiting)   | http, cache, uuid, hmac, sha2, chrono, tokio, secrets, zeroize, subtle, once_cell, argon2, password-hash, rand, regex, oxcache/memory, bincode, hex, base64 |
| `websocket`  | WebSocket support                        | tokio-tungstenite, axum-extra                |
| `grpc`       | gRPC support                             | tonic, prost                                 |
| `cache`      | Caching support                          | dep:http, oxcache/memory, async-trait         |
| `openapi`    | Automatic OpenAPI 3.1 spec generation    | utoipa, http                                  |
| `simd-json`  | SIMD-accelerated JSON serialization      | simd-json                                     |
| `hex`        | Hexadecimal encoding utility             | hex                                           |
| `full`       | All runtime features (excludes simd-json/hex tooling) | -                                |

### 🔗 Feature Dependencies

- `default`: [`http`]
- `mcp`: Independent protocol (depends on external http crate for stateless HTTP header parsing, not sdforge http feature)
- `streaming`: Requires `http`
- `timestamp`: No dependencies
- `logging`: No dependencies
- `security`: Requires `http`, `cache`
- `websocket`: Requires `http`, `streaming`
- `grpc`: Requires `http`
- `cache`: Independent (uses http crate types, not sdforge http feature)
- `openapi`: Requires `http`

---

## 💡 Usage Examples

### 🌐 HTTP Only

For traditional REST APIs:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["http"] }
```

### 🤖 MCP Only

For AI tool integration:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["mcp"] }
```

### 🔄 Both Protocols

Exposure via both HTTP and MCP from the same code:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["http", "mcp"] }
```

### 🎯 Full Features

All capabilities enabled:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["full"] }
```

### 🛰️ gRPC Dispatch

With the `grpc` feature enabled, `#[forge(grpc_method = "...")]` registers a
`GrpcHandlerRegistration` via inventory. `SdForgeGrpcService::call()` routes
`CallRequest` to the matching handler. Return types must satisfy
`serde::Serialize`; errors must be `ApiError`:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["grpc"] }
```

```rust
use sdforge::prelude::*;
use sdforge::forge;

#[forge(
    name = "grpc_echo",
    version = "v1",
    grpc_method = "comprehensive.echo",
    description = "gRPC echo handler"
)]
async fn echo(msg: String) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({ "echo": msg }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    sdforge::init_all_plugins();
    let server = sdforge::grpc::SdForgeGrpcServer::default();
    server.serve("0.0.0.0:50051").await?;
    Ok(())
}
```

### 🖥️ CLI Dispatch

With the `cli` feature enabled, `#[forge(cli = true)]` emits paired
`CliCommandRegistration` + `CliHandlerRegistration` inventory submissions.
`CliBuilder::execute()` is a one-shot runner that handles build / parse /
dispatch / output / exit. Returning `Value::String` prints the raw string
(no quotes); other types are JSON-serialized:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["cli"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

```rust
use sdforge::cli::CliBuilder;
use sdforge::core::ApiError;
use sdforge::forge;

#[forge(name = "echo", version = "1.0", description = "Echo a greeting", cli = true)]
async fn echo(name: String) -> Result<String, ApiError> {
    Ok(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() {
    sdforge::init_all_plugins();
    // execute() returns `!`: it calls std::process::exit(0/1) internally,
    // so callers don't need to match on the result.
    CliBuilder::new().execute().await;
}
```

```sh
# Run: cargo run --example basic_cli --features cli -- echo --name world
# Output: Hello, world!   (no quotes — smart Value::String extraction)
```

---

## <span id="module-prefixes">📁 Module Prefixes</span>

Group related APIs with module prefixes for better organization:

```rust
#[service_module(prefix = "/auth")]
mod auth_api {
    use super::*;

    #[forge(
        name = "login",
        version = "v1",
        path = "/login",
        method = "POST"
    )]
    async fn login(credentials: Credentials) -> Result<Token, ApiError> {
        // Implementation
        Ok(Token::new())
    }

    #[forge(
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
#[forge(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user_v1"
)]
async fn get_user_v1(id: u64) -> Result<UserV1, ApiError> {
    Ok(UserV1 { id, name: "John Doe".into() })
}

#[forge(
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
#[forge(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    // `id` is automatically extracted from `/users/:id`
    Ok(User { id, name: "John".into() })
}

#[forge(
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
#[forge(
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
- [💡 Examples](./examples/)

---

## <span id="contributing">🤝 Contributing</span>

We welcome contributions! Please submit pull requests with clear descriptions of your changes.

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
│   ├── streaming/    # SSE streaming support
│   ├── config/       # Configuration management
│   └── lib.rs        # Library entry point
├── macros/            # Procedural macros crate
│   ├── src/
│   └── Cargo.toml
├── docs/              # Documentation
├── .github/           # GitHub workflows
└── scripts/           # Build and utility scripts
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

#[forge(
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

### ⚠️ Security Defaults (v0.3.0+)

> **Note**: v0.3.0 tightened security defaults. Please check during migration:
> - **JWT secret minimum length**: `MIN_SECRET_LENGTH=32`. Secrets shorter than 32 characters are rejected with an error
> - **ServerConfig default host**: Changed from `"0.0.0.0"` (fail-open) to `"127.0.0.1"` (fail-safe loopback). Production deployments must explicitly configure host
> - **CORS validation tightened**: `"http://"` (scheme only, no host) is now rejected

---

## <span id="performance-optimization">⚡ Performance Optimization</span>

### 🚀 Caching Configuration

```toml
# config.toml
[cache]
enabled = true
default_ttl_secs = 600
max_items = 5000
track_stats = true
```

### 📊 Memory Management

```rust
use sdforge::config::CacheConfig;

let cache_config = CacheConfig {
    enabled: true,
    default_ttl_secs: 600,
    max_items: 5000,
    track_stats: true,
};
```

### ⚙️ Connection Pooling

```rust
use sdforge::config::AppConfig;
use sdforge::http::build_with_config;

let config = AppConfig::default();
let app = build_with_config(&config)?;
```

---

## <span id="openapi-generation">📜 OpenAPI Auto-Generation</span>

SDForge v0.2.0 introduces automatic OpenAPI 3.1 specification generation based on [utoipa 5.5](https://crates.io/crates/utoipa). When the `openapi` feature is enabled, each `#[forge]` macro registers an `OpenApiRouteInfo` at compile time via `inventory`. At runtime, calling `generate_openapi_spec()` collects all routes and generates a complete specification.

### 🔧 Enabling

```toml
[dependencies]
sdforge = { version = "0.5", features = ["http", "openapi"] }
```

### 🚀 Basic Usage

```rust
use sdforge::openapi::generate_openapi_spec;

// Collect all routes registered via #[forge] and generate the OpenAPI specification
let spec = generate_openapi_spec();

// Serialize to JSON to write to a file or return to the client
let json = serde_json::to_string_pretty(&spec).unwrap();
println!("{json}");
```

### 🎨 Custom Metadata

Use the `OpenApiBuilder` chainable calls to customize the `info` section (title, version, description). Routes are always collected from the global `inventory` registry:

```rust
use sdforge::openapi::OpenApiBuilder;

let spec = OpenApiBuilder::new()
    .title("My Service")
    .version("2.0.0")
    .description("User-facing API for the billing domain")
    .build();
```

### 🔗 Macro Integration

When the `openapi` feature is enabled, `#[forge]` automatically generates registration code—no manual maintenance required:

```rust
#[forge(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    description = "Get a user by ID"
)]
async fn get_user(id: u64) -> Result<User, ApiError> { /* ... */ }
```

The above code automatically submits an `OpenApiRouteInfo { path: "/users/{id}", method: "GET", ... }` to the global registry at compile time. `generate_openapi_spec()` will include it in the generated specification.

> **Note**: When the `openapi` feature is not enabled, the macro does not generate any utoipa-related code—zero runtime overhead.

---

## <span id="mcp-migration">🔄 MCP 2026-07-28 Migration Guide</span>

v0.2.0 fully migrates the MCP implementation from `mcp-sdk 0.0.3` to [`rmcp 0.16`](https://crates.io/crates/rmcp), adapting to the MCP 2026-07-28 specification. This migration is a **BREAKING** change.

### ⚠️ BREAKING Changes

| Old Version (v0.1.x)                  | New Version (v0.2.0)                          |
|----------------------------------------|-----------------------------------------------|
| `mcp-sdk = "0.0"` dependency          | `rmcp = "0.16"` dependency                    |
| `initialize` handshake flow             | Removed, replaced with `server/discover` endpoint |
| Stateful sessions (`StatefulServerHandler`) | Stateless adapter layer (`StatelessServerHandler`) |
| `register_mcp(&mut Server)` signature   | `register_mcp(&mut dyn McpToolRegistry)`      |

### 🛠️ Stateless Adapter Layer

`StatelessServerHandler` implements the `rmcp::ServerHandler` trait. None of its methods depend on session state, adapting to the stateless protocol model of the 2026-07-28 specification:

```rust
use sdforge::mcp::stateless::StatelessServerHandler;

let handler = StatelessServerHandler::new();
// Mount to HTTP routes via rmcp's axum integration
```

### 📨 HTTP Header Protocol

The stateless protocol passes methods and tool names through HTTP headers, parsed by `parse_mcp_headers`:

```rust
use sdforge::mcp::headers::parse_mcp_headers;

// Client requests must carry:
//   Mcp-Method: tools/call
//   Mcp-Name: get_user
let info = parse_mcp_headers(&headers)?;
```

Missing headers return `400 Bad Request`, consistent with the 2026-07-28 specification.

### 🔁 Multi Round-Trip Requests (MRTR)

New MRTR support is added. Tools can suspend execution via `InputRequiredResult` and wait for the client to provide additional input. It is automatically canceled after a 300-second timeout:

```rust
use sdforge::mcp::mrtr::MrtrSessionManager;

let manager = MrtrSessionManager::new();
let result = manager.create_session("session-1", "get_user")?;
// The client later resumes execution via session_id
```

### 💾 Cache Semantics

The `cache_semantics` module handles the `ttlMs` and `cacheScope` fields, supporting both `global` and `request` cache scopes. It integrates with oxcache to implement tool result caching.

### 📚 Migration Steps

1. Replace the `mcp-sdk` dependency in `Cargo.toml` with `rmcp` (`features = ["server"]`)
2. Change the `register_mcp(&mut Server)` call to `register_mcp(&mut dyn McpToolRegistry)`
3. Remove the `initialize` handshake-related code and use the `server/discover` endpoint instead
4. If you need MRTR or cache semantics, import the corresponding modules

> For the complete migration example, see `examples/src/mcp/migration_2026.rs`.

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
#[forge(
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

[🔝 Back To Top](#readme) | [🇨🇳 中文](./README.md)

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
