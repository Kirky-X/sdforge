# SDForge Examples

This directory contains examples demonstrating how to use the SDForge framework.

## Structure

- `src/basic/` - Core API definitions and error handling
- `src/http/` - HTTP protocol examples (routing, parameters, middleware, versioning)
- `src/mcp/` - MCP protocol examples (tools, registration)
- `src/security/` - Authentication, authorization, rate limiting, audit logging
- `src/cache/` - Memory and Redis caching examples
- `src/config/` - Configuration management and hot reload
- `src/streaming/` - SSE streaming examples
- `src/websocket/` - WebSocket examples
- `src/grpc/` - gRPC examples
- `src/logging/` - Structured logging examples
- `src/combined/` - Full examples combining multiple features

## Running Examples

### Default Features

```bash
cargo run
```

### With Specific Features

```bash
# Run with HTTP support
cargo run --features http

# Run with MCP support
cargo run --features mcp

# Run with all features
cargo run --features full
```

## Dependencies

Some examples require external services:

- **Redis**: Required for `cache/redis.rs` examples (use Docker Compose or run Redis locally)
- **PostgreSQL**: Some advanced examples may require a database connection

## Feature Matrix

| Module | Default | HTTP | MCP | Streaming | Security | Cache | WebSocket | gRPC | Logging |
|--------|---------|------|-----|-----------|----------|-------|-----------|------|---------|
| basic | ✓ | - | - | - | - | - | - | - | - |
| http | - | ✓ | - | - | - | - | - | - | - |
| mcp | - | - | ✓ | - | - | - | - | - | - |
| security | - | - | - | - | ✓ | - | - | - | - |
| cache | - | - | - | - | - | ✓ | - | - | - |
| streaming | - | - | - | ✓ | - | - | - | - | - |
| websocket | - | - | - | - | - | - | ✓ | - | - |
| grpc | - | - | - | - | - | - | - | ✓ | - |
| logging | - | - | - | - | - | - | - | - | ✓ |

## Next Steps

1. Start with `basic/simple_api.rs` to understand the core macro
2. Explore `http/routing.rs` for HTTP routing patterns
3. Check `mcp/tools.rs` for MCP protocol usage
4. Look at `combined/multi_protocol.rs` to see HTTP + MCP together
