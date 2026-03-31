# SDForge Examples

This directory contains examples demonstrating how to use the SDForge framework.

## Structure

- `src/basic/` - Core API definitions and error handling
- `src/http/` - HTTP protocol examples (routing, parameters, middleware, versioning)
- `src/mcp/` - MCP protocol examples (tools, registration)
- `src/security/` - Authentication, authorization, rate limiting, audit logging
  - `api_key.rs` - API Key authentication setup and usage
  - `rate_limiting.rs` - Sliding window rate limiter implementation
  - `comprehensive.rs` - **NEW** Complete security stack example (API Key + JWT + Rate Limiting + Audit + Cache)
- `src/cache/` - Memory and Redis caching examples
  - `performance.rs` - **NEW** Advanced caching patterns (Two-Level, Cache-Aside, Write-Through)
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
5. **NEW**: Study `security/comprehensive.rs` for complete security implementation
6. **NEW**: Review `cache/performance.rs` for advanced caching patterns

## Example Applications

### Security Examples

#### Comprehensive Security Stack (`security/comprehensive.rs`)

This example demonstrates a complete production-ready API with:

- **Authentication**: API Key + JWT Bearer Token
- **Authorization**: Role-based access control (Admin/User/Guest)
- **Rate Limiting**: Sliding window algorithm (100 req/min)
- **Audit Logging**: Tamper-proof with HMAC-SHA256 signatures
- **Caching**: LRU cache with automatic invalidation
- **Input Validation**: Email, password strength, length limits

**Key Features:**
```rust
// Two-level authentication
- API Key for service-to-service calls
- JWT tokens for user authentication

// Rate limiting per client
if !state.rate_limiter.check(&client_key) {
    return Err(ApiError::RateLimitExceeded { ... });
}

// Tamper-proof audit logging
let mut audit_log = AuditLog::new(...);
audit_log.generate_signature(secret_key);
assert!(audit_log.verify_signature(secret_key).unwrap());
```

### Cache Examples

#### Performance Optimization (`cache/performance.rs`)

This example shows advanced caching strategies:

- **Two-Level Cache**: L1 (hot data) + L2 (warm data)
- **Cache-Aside Pattern**: Lazy loading with computation avoidance
- **Write-Through Pattern**: Atomic cache and storage updates
- **TTL Management**: Time-based expiration
- **Serialization**: Efficient binary storage

**Key Patterns:**
```rust
// Two-level cache for optimal performance
struct TwoLevelCache {
    l1_cache: Arc<DashMapCache>,  // Fast, small
    l2_cache: Arc<DashMapCache>,  // Larger, slower
}

// Cache-aside pattern
let result = cache.get_or_compute(key, || async {
    expensive_computation().await
}).await;

// Automatic cache promotion (L2 → L1)
if let Some(data) = l2_cache.get(key) {
    l1_cache.set(key, data.clone()); // Promote!
}
```

## Best Practices Demonstrated

### Security
✓ Never trust client input - validate everything  
✓ Use strong secrets (32+ characters for JWT)  
✓ Sign audit logs to prevent tampering  
✓ Rate limit all endpoints to prevent abuse  
✓ Cache sensitive data with appropriate TTL  

### Performance
✓ Cache expensive computations  
✓ Use two-level caching for hot/warm data separation  
✓ Invalidate cache on writes (write-through or explicit)  
✓ Serialize/deserialize efficiently (binary formats)  
✓ Monitor cache hit rates and adjust TTLs  

### Code Quality
✓ Comprehensive error handling with context  
✓ Type-safe request/response models  
✓ Input validation at multiple layers  
✓ Clear separation of concerns  
✓ Extensive test coverage  

## Running Specific Examples

```bash
# Security comprehensive example
cargo run --features "http security cache" --example security/comprehensive

# Cache performance example
cargo run --features "http cache" --example cache/performance

# All features combined
cargo run --features full --example security/comprehensive
```
