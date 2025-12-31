# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Axiom** is a Rust-based declarative SDK framework that uses procedural macros to automatically generate multi-protocol service interfaces (HTTP + MCP) from unified function annotations. The key innovation is compile-time protocol selection via Cargo features—unused protocols produce zero compiled code.

## Build Commands

```bash
# Build with specific features
cargo build --features http              # HTTP only
cargo build --features mcp               # MCP only
cargo build --features http,mcp          # Both protocols
cargo build --features full              # All features (http, mcp, streaming, timestamp, logging)

# Run tests with feature combinations
cargo test --features http
cargo test --features mcp
cargo test --features "http,mcp"

# Run a single test
cargo test --features http test_name_here

# Code coverage
cargo tarpaulin --features http --out Html

# Documentation
cargo doc --no-deps --features http
```

## Architecture Summary

### Dual-Crate Structure

- **`axiom-macros/`** (proc-macro crate) - Handles AST parsing, validation, and code generation
- **`axiom/`** (runtime crate) - Provides runtime types, error handling, and service builders

### Code Generation Pipeline

1. **Parse**: `#[service_api]` attributes → `ApiConfig` struct (via `darling`)
2. **Validate**: Check required params per enabled feature (HTTP needs `path`+`method`, MCP needs `tool_name`)
3. **Generate**:
   - Input/Output structs with Serde derives
   - Protocol-specific adapters (HTTP handler or MCP tool)
   - `inventory::submit!` registration
4. **Build**: `inventory::iter::<HttpRoute>()` or `inventory::iter::<McpToolRegistration>()`

### Feature System

| Feature | Purpose | Dependencies |
|---------|---------|--------------|
| `http` | HTTP server (Axum 0.8.8) | axum, tower, tower-http |
| `mcp` | MCP protocol (mcp-sdk 0.0.3) | mcp-sdk |
| `streaming` | SSE streaming support | tokio-stream, futures |
| `timestamp` | Auto-add timestamp to responses | chrono |
| `logging` | Structured request logging | tracing, tracing-subscriber |
| `full` | Convenience: all features | - |

**Critical**: At least one protocol feature (`http` or `mcp`) must be enabled. The `streaming` feature requires `http`.

### Key Runtime Types (axiom/)

- `ApiMetadata` - Protocol-agnostic API metadata (name, version, description)
- `ServiceResponse<T>` - Unified response wrapper with optional timestamp
- `ApiError` / `ServiceError` - Error enum with HTTP status mapping
- `HttpRoute` / `McpToolRegistration` - Inventory-collectable registration types

### Macro Attributes

```rust
// Function-level: generates protocol adapters + inventory registration
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",      // HTTP only
    method = "GET",           // HTTP only
    tool_name = "get_user",   // MCP only
    description = "Get user", // MCP only
)]

// Module-level: injects const __AXIOM_MODULE_PREFIX for path prefixing
#[service_module(prefix = "/auth")]
mod auth { /* APIs here inherit /auth/api/{version}/ prefix */ }
```

### Path Composition

- Base: `/api/{version}{path}`
- With module: `{module_prefix}/api/{version}{path}`
- Example: `#[service_module(prefix = "/auth")]` + `path = "/login"` → `/auth/api/v1/login`

## Testing Strategy

See `docs/test.md` for detailed test cases:
- **Unit tests**: Config parsing, code generation, validation logic
- **Integration tests**: HTTP/MCP E2E, feature combinations, module prefixes
- **Performance tests**: QPS benchmarks, binary size verification
- **Feature matrix**: Validates unused features produce no code

## Common Development Patterns

1. **Adding a new protocol** (future): Implement `#[cfg(feature = "protocol_name")]` code generation block, add protocol-specific validation
2. **Adding a new feature**: Add to Cargo.toml features, wrap output fields with `#[cfg(feature = "...")]`
3. **Modifying validation**: Update `ApiConfig::validate()` in macros crate

## Key Dependencies

- `syn` + `quote` + `darling` - AST parsing and code generation
- `inventory` - Static registration without lazy_static
- `axum` - HTTP framework
- `mcp-sdk` - MCP protocol SDK
- `thiserror` - Error enum derive

## Status Indicators

Project documents use these markers:
- `⏳ 待开发` / `⏳ 待实现` - Not yet implemented
- `🔴 P0` - Blocking/priority highest
- `🟠 P1` - High priority
- `🟡 P2` - Medium priority
- `🟢 P3` - Low priority
