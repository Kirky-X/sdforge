# SDForge API Documentation Examples

This directory contains comprehensive examples demonstrating how to use SDForge's public APIs with proper documentation.

## Module Overview

### Core Types (`sdforge::core`)

Basic building blocks for API development:

- **ApiMetadata** - Service metadata (name, version, description)
- **ApiError** - Comprehensive error types with i18n support
- **ServiceResponse** - Standardized response wrapper
- **ServiceError** - Custom error definition utility

### HTTP Protocol (`sdforge::http`)

HTTP server implementation using Axum:

- **RouteRegistration** - HTTP route registration trait
- **HttpRoute** - HTTP route configuration
- **version_routing** - API versioning support

### Security (`sdforge::security`)

Authentication and authorization middleware:

- **ApiKeyAuth / AppApiKeyAuth** - API Key authentication
- **BearerAuth** - JWT Bearer token authentication  
- **RateLimiter** - Sliding window rate limiting
- **AuditLogger** - Tamper-proof audit logging

### Cache (`sdforge::cache`)

Caching infrastructure using oxcache:

- **Cache / SyncCache** - Cache trait interfaces
- **DashMapCache** - Thread-safe HashMap implementation
- **MemoryBackend** - In-memory cache backend

### Streaming (`sdforge::streaming`)

Server-Sent Events (SSE) support:

- **StreamResponse** - Streamable response wrapper
- **create_stream_channel** - Stream channel creation
- **stream_to_sse** - Convert stream to SSE events

### WebSocket (`sdforge::websocket`)

Real-time bidirectional communication:

- **WebSocketHandler** - WebSocket message handler trait
- **WebSocketConnection** - Connection management
- **ConnectionManager** - Active connection tracking

### gRPC (`sdforge::grpc`)

gRPC protocol support:

- **GrpcRoute** - gRPC route configuration
- **SdForgeServiceServer** - Generated gRPC service
- **build_server** - gRPC server builder

## Usage Examples

### Basic API Definition

```rust
use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

/// Get user by ID
/// 
/// # Arguments
/// * `id` - User ID from path parameter
/// 
/// # Returns
/// User data if found, NotFound error otherwise
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user",
    description = "Retrieve user information by ID"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    // Implementation here
    Ok(User {
        id,
        name: "John Doe".into(),
        email: "john@example.com".into(),
    })
}
```

### Error Handling with i18n

```rust
use sdforge::core::{ApiError, LocalizedError, Locale};

// Create error with context
let error = ApiError::NotFound {
    resource: "User".to_string(),
    resource_id: Some("123".to_string()),
};

// Get localized message
let msg_zh = error.localized_message(&"zh-CN".to_string());
// "资源未找到：User"

let msg_en = error.default_message();
// "Resource not found: User"
```

### Rate Limiting

```rust
use sdforge::security::{RateLimiter, RateLimiterConfig};

let config = RateLimiterConfig::new(100, 60); // 100 requests per minute
let limiter = RateLimiter::with_config(config);

// Check if request is allowed
if limiter.check("user_123") {
    // Process request
} else {
    // Return 429 Too Many Requests
}
```

### Audit Logging

```rust
use sdforge::security::{AuditLogger, AuditLog, AuditResult};

let logger = AuditLogger::default();

// Log an action
let mut log = AuditLog::new(
    "user.login".to_string(),
    Some("user_123".to_string()),
    Some("192.168.1.1".to_string()),
    None,
    None,
    AuditResult::Success,
);

// Generate signature for tamper-proof logging
let signature = log.generate_signature(secret_key);

// Verify integrity later
assert!(log.verify_signature(secret_key).is_ok());
```

### Caching

```rust
use sdforge::cache::{Cache, DashMapCache, SyncCache};
use std::sync::Arc;

let cache = Arc::new(DashMapCache::new());

// Set value
cache.set("user:123", vec![1, 2, 3]);

// Get value
if let Some(data) = cache.get("user:123") {
    // Use cached data
}

// Delete value
cache.delete("user:123");
```

### Input Validation

```rust
use sdforge::core::validation::{
    validate_email, validate_length, MAX_EMAIL_LENGTH, MIN_PASSWORD_LENGTH
};

// Validate email
assert!(validate_email("user@example.com"));
assert!(!validate_email("invalid-email"));

// Validate length
assert!(validate_length("password123", MIN_PASSWORD_LENGTH, 100));
assert!(validate_length(&short_password, MIN_PASSWORD_LENGTH, 100)); // false

// Check constants
assert_eq!(MAX_EMAIL_LENGTH, 320);
```

### Security Limits

The framework enforces these security limits by default:

```rust
use sdforge::core::validation::*;

// Request size limits
assert_eq!(MAX_REQUEST_BODY_SIZE, 10 * 1024 * 1024); // 10 MB
assert_eq!(MAX_HEADER_VALUE_LENGTH, 8 * 1024); // 8 KB
assert_eq!(MAX_URI_PATH_LENGTH, 2048);

// Authentication limits
assert_eq!(MAX_API_KEY_LENGTH, 512);
assert_eq!(MAX_JWT_TOKEN_LENGTH, 4096);

// User input limits
assert_eq!(MAX_USERNAME_LENGTH, 256);
assert_eq!(MAX_PASSWORD_LENGTH, 1024);
assert_eq!(MIN_PASSWORD_LENGTH, 8);

// JSON parsing limits
assert_eq!(MAX_JSON_ARRAY_LENGTH, 10_000);
assert_eq!(MAX_JSON_DEPTH, 100);
```

## Feature Flags

SDForge uses conditional compilation to minimize dependencies:

```toml
[dependencies]
sdforge = { version = "0.2", features = ["full"] }

# Or pick specific features:
sdforge = { version = "0.2", features = ["http", "security"] }
```

Available features:
- `default` - HTTP support only
- `http` - HTTP protocol (Axum)
- `mcp` - MCP protocol
- `security` - Authentication, rate limiting, audit
- `cache` - Caching (oxcache)
- `streaming` - SSE streaming
- `websocket` - WebSocket support
- `grpc` - gRPC support
- `full` - All features enabled

## Best Practices

1. **Always document public APIs** - Use `///` for functions, structs, and traits
2. **Use typed errors** - Define specific error types with context
3. **Validate inputs** - Check all user-provided data against limits
4. **Enable security features** - Use rate limiting and audit logging in production
5. **Cache strategically** - Cache expensive computations, not trivial data
6. **Handle errors gracefully** - Provide helpful error messages with context
7. **Test thoroughly** - Write unit tests for all public APIs

## Additional Resources

- [API Reference](../docs/API_REFERENCE.md)
- [Architecture Guide](../docs/ARCHITECTURE.md)
- [Testing Guide](../docs/TESTING_GUIDE.md)
- [Contributing Guide](../docs/CONTRIBUTING.md)
