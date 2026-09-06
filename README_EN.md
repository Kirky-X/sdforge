<div align="center">

<img src="docs/assets/sdforge.png" alt="SDForge Logo" width="200">

[![CI Status](https://github.com/Kirky-X/sdforge/actions/workflows/ci.yml/badge.svg)](https://github.com/Kirky-X/sdforge/actions/workflows/ci.yml) [![Version](https://img.shields.io/crates/v/sdforge.svg)](https://crates.io/crates/sdforge) [![Docs.rs](https://docs.rs/sdforge/badge.svg)](https://docs.rs/sdforge) [![Downloads](https://img.shields.io/crates/d/sdforge.svg)](https://crates.io/crates/sdforge) [![License](https://img.shields.io/crates/l/sdforge.svg)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.97.1%2B-orange.svg)](https://www.rust-lang.org/) [![Coverage](https://codecov.io/gh/Kirky-X/sdforge/branch/main/graph/badge.svg)](https://codecov.io/gh/Kirky-X/sdforge)

**[中文](README.md)** | English

**SDForge** is a Rust-based declarative SDK framework that uses procedural macros to automatically generate multi-protocol service interfaces (HTTP + MCP + gRPC + WebSocket + CLI) from unified function annotations. The key innovation is compile-time protocol selection via Cargo features — unused protocols produce zero compiled code.

[✨ Features](#-features) • [🚀 Quick Start](#-quick-start) • [📚 Documentation](#-documentation) • [💻 Examples](#-examples) • [🤝 Contributing](#-contributing)

</div>

---

## 📋 Table of Contents

<details open>

- [✨ Features](#-features)
- [🚀 Quick Start](#-quick-start)
  - [📦 Installation](#-installation)
  - [💡 Basic Usage](#-basic-usage)
  - [📁 Module Prefixes](#-module-prefixes)
  - [🔢 Version Management](#-version-management)
  - [🛤️ Path Parameters](#️-path-parameters)
  - [⚠️ Error Handling](#️-error-handling)
  - [🔧 `#[forge]` Macro Parameters](#-forge-macro-parameters)
  - [🌐 Protocol Combinations](#-protocol-combinations)
  - [🛰️ gRPC Dispatch](#️-grpc-dispatch)
  - [🖥️ CLI Dispatch](#️-cli-dispatch)
- [🎨 Feature Flags](#-feature-flags)
- [📚 Documentation](#-documentation)
- [💻 Examples](#-examples)
- [🏗️ Architecture](#️-architecture)
- [📜 OpenAPI Auto-Generation](#-openapi-auto-generation)
- [🔄 MCP 2026-07-28 Migration Guide](#-mcp-2026-07-28-migration-guide)
- [🚀 Production Deployment](#-production-deployment)
- [🐛 Troubleshooting](#-troubleshooting)
- [🧪 Testing](#-testing)
- [📊 Performance](#-performance)
- [🔒 Security](#-security)
- [🗺️ Roadmap](#️-roadmap)
- [🤝 Contributing](#-contributing)
- [📋 Changelog](#-changelog)
- [📄 License](#-license)
- [🙏 Acknowledgments](#-acknowledgments)
- [📞 Contact & Support](#-contact--support)
- [⭐ Star History](#-star-history)

</details>

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| **🎯 Unified Interface Definition** | Single macro configuration for HTTP, MCP, gRPC, WebSocket, and CLI |
| **⚡ Compile-Time Protocol Selection** | Feature-gated code generation with zero runtime overhead for unused protocols |
| **🔒 Type Safety** | Compile-time validation of interface definitions |
| **🌐 Multi-Protocol Support** | HTTP (Axum), MCP (rmcp 2.1), gRPC (tonic), WebSocket, SSE streaming, CLI (clap) |
| **🧩 Modular Design** | Feature-based architecture allows selecting only needed functionality |
| **🛡️ Security Features** | Built-in authentication (Bearer/API Key), rate limiting (limiteron), audit logging |
| **💾 Caching** | In-memory caching (oxcache), no external database required |
| **🔧 Configuration Management** | Self-contained TOML configuration (no external config center) |
| **📊 Versioning** | Built-in API version management |
| **📜 OpenAPI Auto-Generation** | OpenAPI 3.1 spec generation based on utoipa 5.5 |
| **🌐 Internationalization** | ICU4X 2.x-based localization support (`i18n` feature) |

### 🔌 Optional Capabilities

All of the following are enabled on demand via Cargo features — see [Feature Flags](#-feature-flags):

| Optional Capability | Feature | Description |
|---------------------|---------|-------------|
| HTTP server | `http` | Axum 0.8 routing, middleware, version routing |
| MCP protocol | `mcp` | Official rmcp SDK, 2026-07-28 spec (stateless HTTP headers, MRTR, cache semantics) |
| SSE streaming | `streaming` | SSE event streams and streaming response building |
| WebSocket | `websocket` | Connection management, broadcast, message parsing |
| gRPC | `grpc` | tonic service, unified handler dispatch |
| CLI | `cli` | clap integration, one-shot `CliBuilder::execute()` |
| OpenAPI 3.1 | `openapi` | Compile-time route registration, runtime spec generation |
| Unified docs output | `docs` | Swagger UI + CLI/MCP Markdown |
| Auth & audit | `security` | API Key / JWT Bearer, audit logging, security headers |
| Rate limiting | `ratelimit` / `ratelimit-http` | Unified limiteron rate limiting (core / Tower middleware) |
| Caching | `cache` | oxcache in-memory cache (LRU, pattern invalidation, stats) |
| Response timestamps | `timestamp` | Auto-add timestamps to responses |
| Structured logging | `logging` | Structured request logging |
| inklog integration | `inklog` | Bridge to the inklog LoggerManager structured logging pipeline |
| Internationalization | `i18n` | ICU4X locale-aware formatting and Accept-Language parsing |
| SIMD JSON | `simd-json` | SIMD-accelerated JSON serialization |

#### 🆕 Phase 1 Architecture Improvements

Recent architectural enhancements include:

- **🔄 Unified Registration System** — Eliminated 95+ lines of duplicate code across HTTP, MCP, WebSocket, and gRPC modules using trait-based abstraction and procedural macros
- **⚙️ Modular Configuration Management** — Refactored configuration into dedicated modules (app, cache, security) with centralized defaults and Builder pattern support
- **🔐 Enhanced Security Module** — API Key versioning, LRU caching, key rotation with audit logging, and comprehensive security headers configuration
- **💾 Advanced Caching** — Pattern-based cache invalidation, key normalization, batch operations, and statistics tracking

---

## 🚀 Quick Start

### 📦 Installation

```bash
cargo add sdforge
```

Or add it to your `Cargo.toml` manually:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["http"] }
```

> Note: `sdforge` enables no features by default (`default = []`); enable protocol features explicitly as needed.

### 💡 Basic Usage

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

### 📁 Module Prefixes

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
        Ok(Token::new())
    }

    #[forge(
        name = "logout",
        version = "v1",
        path = "/logout",
        method = "POST"
    )]
    async fn logout() -> Result<(), ApiError> {
        Ok(())
    }
}
```

This creates the endpoints:

- `/auth/api/v1/login`
- `/auth/api/v1/logout`

### 🔢 Version Management

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

### 🛤️ Path Parameters

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
```

#### 🔹 Multiple Path Parameters

For nested resources:

```rust
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

#[forge(
    name = "get_task",
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

### ⚠️ Error Handling

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

### 🔧 `#[forge]` Macro Parameters

| Parameter      | Description                                                        | Required | Default |
|----------------|--------------------------------------------------------------------|----------|---------|
| `name`         | Endpoint name                                                      | Yes      | -       |
| `version`      | API version                                                        | Yes      | -       |
| `path`         | HTTP path (e.g., `/users/:id`)                                     | No       | -       |
| `method`       | HTTP method (GET/POST/PUT/DELETE, etc.)                            | No       | GET     |
| `status`       | Explicit success status code (e.g., 201 for POST create)          | No       | 200     |
| `description`  | Endpoint description                                               | No       | -       |
| `tool_name`    | MCP tool name                                                      | No       | -       |
| `grpc_method`  | gRPC method name (effective when the `grpc` feature is enabled)   | No       | -       |
| `cli`          | Register as CLI command (effective when the `cli` feature is enabled) | No    | false   |

### 🌐 Protocol Combinations

**HTTP only** — for traditional REST APIs:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["http"] }
```

**MCP only** — for AI tool integration:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["mcp"] }
```

**Both protocols** — expose the same code via HTTP and MCP:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["http", "mcp"] }
```

**Full features** — all capabilities enabled:

```toml
[dependencies]
sdforge = { version = "0.5", features = ["full"] }
```

### 🛰️ gRPC Dispatch

With the `grpc` feature enabled, `#[forge(grpc_method = "...")]` registers into
`SdForgeGrpcService` via inventory; its `call()` method routes to the matching
handler. Return types must satisfy `serde::Serialize`; errors must be `ApiError`:

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
    let server = sdforge::grpc::SdForgeGrpcService::default();
    server.serve("0.0.0.0:50051").await?;
    Ok(())
}
```

### 🖥️ CLI Dispatch

With the `cli` feature enabled, `#[forge(cli = true)]` registers paired
`CliCommandRegistration` + `CliHandlerRegistration` entries.
`CliBuilder::execute()` is a one-shot runner that handles build / parse /
dispatch / output / exit. Returning `Value::String` prints the raw string
(no quotes); other types are serialized as JSON:

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

## 🎨 Feature Flags

SDForge uses Cargo features for compile-time protocol selection and feature composition.

| Feature          | Description                                          | Default |
|------------------|------------------------------------------------------|---------|
| `http`           | HTTP server (Axum 0.8)                               | ❌      |
| `mcp`            | MCP protocol (rmcp 2.1, 2026-07-28 spec)             | ❌      |
| `streaming`      | SSE streaming support                                | ❌      |
| `timestamp`      | Auto-add timestamp to responses                      | ❌      |
| `logging`        | Structured request logging                           | ❌      |
| `security`       | Security features (auth, rate limiting, audit)       | ❌      |
| `ratelimit`      | Rate limiting core (limiteron-based, no http dep)    | ❌      |
| `ratelimit-http` | HTTP rate limiting middleware (Tower middleware)     | ❌      |
| `websocket`      | WebSocket support                                    | ❌      |
| `grpc`           | gRPC support (tonic)                                 | ❌      |
| `cache`          | Caching support (oxcache)                            | ❌      |
| `openapi`        | Automatic OpenAPI 3.1 spec generation                | ❌      |
| `cli`            | CLI integration (clap)                               | ❌      |
| `docs`           | Unified docs output (Swagger UI + Markdown)          | ❌      |
| `inklog`         | inklog structured logging integration                | ❌      |
| `i18n`           | ICU4X internationalization (locale-aware formatting) | ❌      |
| `simd-json`      | SIMD-accelerated JSON serialization                  | ❌      |
| `full`           | All runtime features                                 | ❌      |

### 🔗 Feature Dependencies

- `default`: empty (no pre-enabled features; enable explicitly as needed)
- `mcp`/`grpc`/`openapi`/`cli`/`streaming`/`cache`: independent of `http`
- `security`: enables `http`, `ratelimit-http` (which includes `ratelimit`) and `cache`, plus hmac/sha2/uuid security dependencies
- `ratelimit-http`: requires `http` + `ratelimit`
- `websocket`: requires `http` + `streaming`
- `docs`: requires `openapi` + `cli` (the Swagger UI submodule additionally requires `http`)
- `kit`: trait-kit AsyncKit integration; requires `limiteron-integration` + `trait-kit`
- `full`: all runtime features (excludes the `simd-json` and `hex` tooling features)

### 🔨 Building and Testing

```bash
# Default (no features)
cargo build

# HTTP protocol
cargo build --features http

# MCP protocol
cargo build --features mcp

# Full features
cargo build --features full

# Custom feature set
cargo build --features "http,cache,security"

# Tests
cargo test --features http
cargo test --features full

# Formatting and linting
cargo fmt
cargo clippy --all-features -- -D warnings
```

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [📖 User Guide](docs/USER_GUIDE.md) | Complete tutorial from installation to advanced usage |
| [📘 API Reference](docs/API_REFERENCE.md) | Detailed reference for all public APIs |
| [🏗️ Architecture](docs/ARCHITECTURE.md) | Design philosophy and internal implementation |
| [🔒 Security](docs/SECURITY.md) | Security design and best practices |
| [⚡ Benchmarks](docs/benchmarks/vs-server-less.md) | Compile-time/binary size comparison: feature gating vs full build |
| [📋 Changelog](docs/CHANGELOG.md) | Change log for every release |
| [🤝 Contributing](docs/CONTRIBUTING.md) | How to participate in development |
| [📦 Online API Docs](https://docs.rs/sdforge) | Latest documentation generated by docs.rs |

---

## 💻 Examples

The repository contains two kinds of examples.

### Runnable examples (root package `cargo run --example`)

| Example | Required features | Description |
|---------|-------------------|-------------|
| `basic_cli` | `cli` | `#[forge(cli = true)]` + one-shot `CliBuilder` CLI entry point |
| `swagger_demo` | `docs` | Swagger UI route + axum serve |
| `perf_regex_cache` | `cache` | Regex cache performance verification |
| `perf_lru_eviction` | `cache` | LRU eviction performance verification |
| `perf_prefix_index` | `cache` | Prefix index performance verification |
| `perf_batch_ops` | `cache` | Batch operation performance verification |

```bash
# CLI example
cargo run --example basic_cli --features cli -- echo --name world

# Swagger UI example
cargo run --example swagger_demo --features "docs http"

# Cache performance example
cargo run --example perf_regex_cache --features cache
```

### Comprehensive example library (workspace member `sdforge-examples`, `examples/src/`)

| Module | Contents |
|--------|----------|
| `basics/` | Simple API, response building, types and error handling |
| `http/` | Routing (path params, query params), middleware (CORS) |
| `mcp/` | Tool definition & registration, MCP 2026-07-28 migration (`migration_2026.rs`), MRTR sessions (`mrtr_example.rs`) |
| `security/` | API Key auth, auth failure scenarios, full security stack (`comprehensive.rs`) |
| `cache/` | Advanced caching patterns (two-level, Cache-Aside, Write-Through) |
| `config/` | Configuration management (`app_config.rs`) |
| `streaming/` | SSE streaming responses |
| `websocket/` | Basic usage and chat room examples |
| `grpc/` | gRPC server |
| `logging/` | Structured logging |
| `openapi/` | OpenAPI spec generation (`OpenApiBuilder`, `generate_openapi_spec`) |
| `combined/` | Complete examples combining multiple features (`full_example.rs`) |

```bash
# Run all module tests of the comprehensive example library
cargo test --manifest-path examples/Cargo.toml --lib
```

Sample configuration files live in `examples/config/` (`default.toml`, `minimal.toml`, `production.toml`, `api-key-auth.toml`).

---

## 🏗️ Architecture

SDForge follows an architecture of "unified macro annotations → compile-time protocol gating → inventory runtime registration". See the [Architecture document](docs/ARCHITECTURE.md) for the full design description.

```
sdforge/
├── src/                # Main framework crate
│   ├── core/         # Core types, error handling, validation
│   ├── error/        # Framework error types (ApiError, SdForgeError)
│   ├── http/         # HTTP protocol implementation (Axum)
│   ├── mcp/          # MCP protocol implementation (rmcp)
│   ├── security/     # Security features (auth, rate limiting, audit)
│   ├── cache/        # Cache integration (oxcache)
│   ├── websocket/    # WebSocket support
│   ├── grpc/         # gRPC support (tonic)
│   ├── streaming/    # SSE streaming support
│   ├── cli/          # CLI integration (clap)
│   ├── docs/         # Documentation generation (Swagger UI + Markdown)
│   ├── openapi/      # OpenAPI 3.1 spec generation
│   ├── domain/       # Domain abstractions
│   ├── config/       # Configuration management
│   ├── i18n/         # Internationalization (ICU4X)
│   ├── integrations/ # trait-kit AsyncKit integration
│   └── lib.rs        # Library entry point
├── macros/            # Procedural macros crate (#[forge])
├── examples/          # Comprehensive example library (workspace member)
├── docs/              # Documentation
├── benches/           # Benchmarks
├── proto/             # Protobuf definitions (gRPC)
├── .github/           # GitHub workflows
└── scripts/           # Build and utility scripts
```

### Design Principles

- **Compile-time protocol selection**: unused protocols produce no compiled code at all
- **Inventory registration pattern**: `inventory::submit!()` for compile-time registration, `init_all_plugins()` to prevent linker optimization
- **Three construction modes**: every component supports `new()` (out of the box), `builder()` (builder pattern), and `with_dependencies()` (dependency injection)
- **No database**: all data interaction goes through oxcache (in-memory cache)

---

## 📜 OpenAPI Auto-Generation

SDForge generates OpenAPI 3.1 specifications automatically based on [utoipa 5.5](https://crates.io/crates/utoipa). When the `openapi` feature is enabled, each `#[forge]` macro registers an `OpenApiRouteInfo` at compile time via `inventory`. At runtime, calling `generate_openapi_spec()` collects all routes and generates a complete specification.

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

Use `OpenApiBuilder` chainable calls to customize the `info` section (title, version, description). Routes are always collected from the global `inventory` registry:

```rust
use sdforge::openapi::OpenApiBuilder;

let spec = OpenApiBuilder::new()
    .title("My Service")
    .version("2.0.0")
    .description("User-facing API for the billing domain")
    .build();
```

### 🔗 Macro Integration

When the `openapi` feature is enabled, `#[forge]` automatically generates registration code — no manual maintenance required:

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

The code above automatically submits `OpenApiRouteInfo { path: "/users/{id}", method: "GET", ... }` to the global registry at compile time; `generate_openapi_spec()` will include it in the generated specification.

> **Note**: when the `openapi` feature is not enabled, the macro generates no utoipa-related code at all — zero runtime overhead.

---

## 🔄 MCP 2026-07-28 Migration Guide

v0.2.0 fully migrated the MCP implementation from `mcp-sdk 0.0.3` to the official [`rmcp`](https://crates.io/crates/rmcp) SDK (currently rmcp 2.1), adapting to the MCP 2026-07-28 specification. This migration is a **BREAKING** change.

### ⚠️ BREAKING Changes

| Old Version (v0.1.x)                   | New Version (v0.2.0+)                         |
|-----------------------------------------|-----------------------------------------------|
| `mcp-sdk = "0.0"` dependency            | `rmcp` dependency                             |
| `initialize` handshake flow             | Removed, replaced with the `server/discover` endpoint |
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

MRTR support lets tools suspend execution via `InputRequiredResult` and wait for the client to provide additional input. Sessions are automatically canceled after a 300-second timeout:

```rust
use sdforge::mcp::mrtr::MrtrSessionManager;

let manager = MrtrSessionManager::new();
let result = manager.create_session("session-1", "get_user")?;
// The client later resumes execution via session_id
```

### 💾 Cache Semantics

The `cache_semantics` module handles the `ttlMs` and `cacheScope` fields, supporting both `global` and `request` cache scopes. It integrates with oxcache to implement tool result caching.

### 📚 Migration Steps

1. Replace the `mcp-sdk` dependency in `Cargo.toml` with `rmcp`
2. Change the `register_mcp(&mut Server)` call to `register_mcp(&mut dyn McpToolRegistry)`
3. Remove the `initialize` handshake-related code and use the `server/discover` endpoint instead
4. If you need MRTR or cache semantics, import the corresponding modules

> For the complete migration example, see `examples/src/mcp/migration_2026.rs`.

---

## 🚀 Production Deployment

### 🐳 Docker Deployment

```dockerfile
FROM rust:1.85 as builder
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

## 🐛 Troubleshooting

### 🔍 Common Issues

#### **Compilation Errors**

```bash
# Error: feature not found
# Solution: check available features
cargo check --help | grep features

# Enable specific features
cargo build --features "http,security,cache"
```

#### **Runtime Issues**

```bash
# Check logs with tracing
RUST_LOG=debug cargo run --features logging

# Common port conflicts
# Solution: change the port or kill the existing process
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

---

## 🧪 Testing

### Test Categories

| Category | Location | Description |
|----------|----------|-------------|
| Unit tests | Embedded `#[cfg(test)]` in `src/`, `tests/unit/` | Module-level unit tests |
| Integration tests | `tests/integration/` | cache / config / error_handling / feature_combinations / grpc / http / mcp / openapi / security / status_code / streaming / uat / websocket / cli / docs |
| Macro tests | `tests/macros/` | trybuild compile-failure cases and macro expansion verification |
| Advanced E2E tests | `tests/e2e_advanced.rs` | Uncovered scenarios across 12 modules (178 tests) |
| Examples comprehensive tests | `examples/tests/comprehensive_features.rs` | Re-export accessibility for all features, cross-protocol dispatch (77 tests) |
| Benchmarks | `src/benches/` | criterion benchmarks (require the `http` feature) |

### Running Commands

```bash
# Run tests per feature set
cargo test --features http
cargo test --features mcp
cargo test --features "http,mcp"
cargo test --features full

# Lib tests only (the CI coverage target)
cargo test --features full --lib

# Run a specific test
cargo test test_get_user --features http

# Run with output
cargo test --features http -- --nocapture

# Release-mode tests
cargo test --release --features http
```

CI generates coverage with `cargo llvm-cov --features full --lib` and uploads it to Codecov.

---

## 📊 Performance

SDForge's compile-time feature gating yields significant compile-time and artifact-size advantages (vs the full `--features full` build):

| Metric | http only | full | Savings |
|--------|-----------|------|---------|
| Compile time (debug) | 28.88s | 54.52s | **47.0%** |
| Compile time (release) | 13.72s | 25.48s | **46.2%** |
| Framework rlib size (debug) | 33.9 MB | 100.2 MB | **66.2%** |
| Framework rlib size (release) | 3.57 MB | 9.07 MB | **60.6%** |
| Unique dependency crates | 396 | 478 | 17.2% |

> Data source: [docs/benchmarks/vs-server-less.md](docs/benchmarks/vs-server-less.md) (measured 2026-07-03 on AMD Ryzen 9 9950X / rustc 1.93.1 / WSL2). See that document for the full methodology, binary size analysis, and reproduction commands.

---

## 🔒 Security

SDForge ships a full security suite (the `security` feature): API Key / JWT Bearer authentication, rate limiting (limiteron), audit logging, security headers (CORS/CSP), and input validation. See the [Security document](docs/SECURITY.md) for the design overview and best practices.

### 🛡️ API Key Authentication

```rust
use sdforge::security::{ApiKeyAuth, auth_middleware};

let app = Router::new()
    .route("/api/*path", get(handler))
    .layer(auth_middleware(ApiKeyAuth::new("your-secret-key")));
```

### ⚡ Rate Limiting

```toml
# config.toml
[rate_limit]
enabled = true
requests_per_minute = 60
burst_size = 10
```

### ⚠️ Security Defaults (v0.3.0+)

> **Note**: v0.3.0 tightened security defaults. Please check during migration:
> - **JWT secret minimum length**: `MIN_SECRET_LENGTH=32`. Secrets shorter than 32 characters are rejected
> - **ServerConfig default host**: changed from `"0.0.0.0"` (fail-open) to `"127.0.0.1"` (fail-safe loopback). Production deployments must explicitly configure the host
> - **CORS validation tightened**: `"http://"` (scheme only, no host) is now rejected
>
> Also: since v0.4.4, `extract_client_ip_core` no longer trusts the `X-Forwarded-For` / `X-Real-IP` headers when no `ConnectInfo` is available. Production deployments **must** configure `ConnectInfo` to enable unspoofable TCP peer IP extraction.

---

## 🗺️ Roadmap

The following plans are compiled from the unreleased entries in [CHANGELOG.md](docs/CHANGELOG.md) and the workspace acceptance plan (ACCEPTANCE_PLAN.md):

- **v0.5.0 release (in progress)** — currently at `0.5.0-rc.2`; completing coordinated releases of the dependency chain (trait-kit/oxcache/inklog/limiteron) and final verification per the workspace acceptance plan
- **Custom success status codes (merged, pending release)** — `#[forge(status = <code>)]` static declaration + `ServiceResponse::success_with_status` dynamic control (see CHANGELOG [Unreleased])
- **Unified error-code behavior contract** — evaluate unifying the status code divergence for the same validation error between HTTP (400) and gRPC (422) (SIMPL-001 in the acceptance plan; currently a recorded behavior contract)
- **Dependency hygiene** — mid-term evaluation of migrating `bincode` (RUSTSEC-2025-0141 unmaintained) to `postcard` / `bitcode` / `rkyv`
- **MSRV declaration alignment** — unified to 1.97.1 per workspace CONFIG_BASELINE (2026-09-06), covering the effective 1.94 requirement under `--all-features`

---

## 🤝 Contributing

We welcome contributions! Please read the [Contributing Guide](docs/CONTRIBUTING.md) for the development environment, TDD workflow, and PR process.

```bash
# Clone the repository
git clone https://github.com/Kirky-X/sdforge.git
cd sdforge

# Install pre-commit hooks
./scripts/install-pre-commit.sh

# Verify the environment
cargo build --all-features
cargo test --all-features --lib
```

---

## 📋 Changelog

See [CHANGELOG.md](docs/CHANGELOG.md). Highlights of recent releases:

- **[Unreleased]** — `#[forge(status = <code>)]` custom success status code (static declaration + `ServiceResponse::success_with_status` dynamic control, aligned across HTTP/gRPC and OpenAPI)
- **[0.4.7]** — Removed tilde constraints from dependency versions; published the `bincode` RUSTSEC-2025-0141 ignore decision
- **[0.4.6]** — Fixed CI Clippy failures; restored the `serde` dev-dependency for examples
- **[0.4.5]** — Added `tests/e2e_advanced.rs` (178 tests)

---

## 📄 License

This project is licensed under the MIT + Commons Clause License. Commercial use requires separate authorization. See [LICENSE](LICENSE).

Copyright (c) 2026 Kirky.X

---

## 🙏 Acknowledgments

SDForge stands on the shoulders of an excellent open-source ecosystem. Thanks to:

- [Axum](https://github.com/tokio-rs/axum) / [Tower](https://github.com/tower-rs/tower) — HTTP service and middleware
- [rmcp](https://crates.io/crates/rmcp) — the official MCP Rust SDK
- [Tonic](https://github.com/hyperium/tonic) / [Prost](https://github.com/tokio-rs/prost) — gRPC and protobuf
- [utoipa](https://crates.io/crates/utoipa) — OpenAPI spec generation
- [clap](https://github.com/clap-rs/clap) — command-line parsing
- [inventory](https://crates.io/crates/inventory) — compile-time registration
- [ICU4X](https://github.com/unicode-org/icu4x) — internationalization
- Base workspace sibling projects [oxcache](https://github.com/Kirky-X/oxcache), [limiteron](https://github.com/Kirky-X/limiteron), [trait-kit](https://github.com/Kirky-X/trait-kit), [inklog](https://github.com/Kirky-X/inklog)

---

## 📞 Contact & Support

- **🐛 Issues**: [github.com/Kirky-X/sdforge/issues](https://github.com/Kirky-X/sdforge/issues)
- **💬 Discussions**: [github.com/Kirky-X/sdforge/discussions](https://github.com/Kirky-X/sdforge/discussions)
- **🏠 Repository**: <https://github.com/Kirky-X/sdforge>
- **📖 Documentation**: <https://docs.rs/sdforge>
- **👤 Maintainer**: Kirky.X

---

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Kirky-X/sdforge&type=Date)](https://star-history.com/#Kirky-X/sdforge&Date)

### 💝 Support This Project

If you find this project useful, please consider giving it a ⭐️!

---

<div align="center">

**Built with ❤️ using Rust**

</div>
