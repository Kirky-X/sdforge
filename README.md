# Axiom - Multi-Protocol SDK Framework

**Axiom** is a Rust-based declarative SDK framework that uses procedural macros to automatically generate multi-protocol service interfaces (HTTP + MCP) from unified function annotations. The key innovation is compile-time protocol selection via Cargo features—unused protocols produce zero compiled code.

## Features

- **Unified Interface Definition**: Single macro configuration for both HTTP and MCP
- **Compile-Time Protocol Selection**: Feature-gated code generation
- **Zero Runtime Overhead**: Unused protocols don't exist in the binary
- **Type Safety**: Compile-time validation of interface definitions
- **Easy Integration**: Works as a library in any Rust project

## Quick Start

Add Axiom to your `Cargo.toml`:

```toml
[dependencies]
axiom = { version = "0.1", features = ["http"] }
```

Define your API with a single macro:

```rust
use axiom::prelude::*;

#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user",
    description = "Get a user by ID"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    // Your implementation
    Ok(User { id, name: "Test".into() })
}

#[tokio::main]
async fn main() {
    let app = axiom::http::build();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Feature System

| Feature    | Description                          | Dependencies          |
|------------|--------------------------------------|-----------------------|
| `http`     | HTTP server (Axum 0.8.8)            | axum, tower, tower-http |
| `mcp`      | MCP protocol (mcp-sdk 0.0.3)        | mcp-sdk               |
| `streaming`| SSE streaming support                | tokio-stream, futures |
| `timestamp`| Auto-add timestamp to responses      | chrono                |
| `logging`  | Structured request logging           | tracing, tracing-subscriber |
| `full`     | All features enabled                 | -                     |

## Usage Examples

### HTTP Only

```toml
axiom = { version = "0.1", features = ["http"] }
```

### MCP Only (for AI tools)

```toml
axiom = { version = "0.1", features = ["mcp"] }
```

### Both Protocols

```toml
axiom = { version = "0.1", features = ["http", "mcp"] }
```

### Full Features

```toml
axiom = { version = "0.1", features = ["full"] }
```

## Architecture

```
User Code                    Axiom Framework
     │                              │
     ├─ #[service_api] ─────────────┤
     │     │                        │
     │     └─► Parse & Validate ────┤
     │                              │
     │                    ┌─────────┴─────────┐
     │                    │                   │
     │              Generate HTTP        Generate MCP
     │              Handler (if          Handler (if
     │              feature=             feature=
     │              "http")              "mcp")
     │                    │                   │
     │                    └─────────┬─────────┘
     │                              │
     │                    ┌─────────┴─────────┐
     │                    │                   │
     │              HTTP Server          MCP Server
     │              (Axum)               (mcp-sdk)
```

## Building

```bash
# Build with HTTP only
cargo build --features http

# Build with MCP only
cargo build --features mcp

# Build with all features
cargo build --features full

# Run tests
cargo test --features http
cargo test --features mcp
cargo test --features "http,mcp"
```

## Documentation

- [API Documentation](https://docs.rs/axiom)
- [Examples](./axiom/examples/)
- [Tests](./axiom/tests/)
- [Best Practices](./docs/best_practices.md)

## Crates

- `axiom`: Runtime library with HTTP/MCP server builders
- `axiom-macros`: Procedural macros for API definition

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT License

at your option.

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.
