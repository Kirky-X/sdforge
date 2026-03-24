<div align="center">

# 📖 SDForge API Reference

**Comprehensive guide to SDForge's public API, macros, types, and configuration**

</div>

---

## 📋 Table of Contents

- [🎨 Macros](#-macros)
  - [`service_api` Macro](#service_api-macro)
  - [`service_module` Macro](#service_module-macro)
  - [`test_macro` Macro](#test_macro-macro)
- [🔧 Core Types](#-core-types)
- [❌ Error Types](#-error-types)
- [🌐 HTTP API](#-http-api)
- [🤖 MCP API](#-mcp-api)
- [🛡️ Security API](️-security-api)
- [💾 Cache API](️-cache-api)
- [💬 WebSocket API](️-websocket-api)
- [🔌 gRPC API](️-grpc-api)
- [⚙️ Configuration API](️-configuration-api)
- [📡 Streaming API](️-streaming-api)

---

## 🎨 Macros

### `service_api` Macro

Defines a service endpoint that automatically generates protocol handlers.

#### 📝 Syntax

```rust
#[service_api(
    name = "api_name",
    version = "v1",
    path = "/path",
    method = "GET",
    tool_name = "tool_name",
    description = "Description",
    cache_ttl = 300,
    stream = false,
    ws_path = "/ws",
    grpc_method = "Call"
)]
async fn my_api(param1: Type1, param2: Type2) -> Result<ResponseType, ApiError> {
    // Implementation
}
```

#### 📊 Attributes

| Attribute      | Type    | Required | Default   | Description                                                                 |
|----------------|---------|----------|-----------|-----------------------------------------------------------------------------|
| `name`         | String  | Yes      | -         | API identifier name (must be valid Rust identifier)                          |
| `version`      | String  | Yes      | -         | API version (e.g., "v1", "1.0", "v1.2.3")                               |
| `path`         | String  | Yes*     | -         | URL path for HTTP (required when http feature enabled)                         |
| `method`       | String  | Yes*     | -         | HTTP method (GET, POST, PUT, DELETE, PATCH) - required when http enabled    |
| `tool_name`    | String  | Yes*     | -         | MCP tool name (required when mcp feature enabled)                             |
| `description`  | String  | No       | `""`      | Human-readable description of the API                                        |
| `cache_ttl`    | Integer | No       | `300`     | Cache time-to-live in seconds (0 to disable)                                |
| `stream`       | Boolean | No       | `false`   | Enable streaming response (requires streaming feature)                          |
| `ws_path`      | String  | No       | -         | WebSocket path (requires websocket feature)                                    |
| `grpc_method`  | String  | No       | -         | gRPC method name (requires grpc feature)                                      |

* Required only when the corresponding feature is enabled

#### ✅ Name Validation

API names must follow Rust identifier rules:
- ✅ Start with a letter or underscore
- ✅ Contain only alphanumeric characters and underscores
- ✅ Not be a reserved Rust keyword
- ✅ Not be empty

#### 🔢 Version Validation

Version strings must:
- ✅ Not be empty
- ✅ Contain only alphanumeric characters, dots, and hyphens
- ✅ May optionally start with 'v' (e.g., "v1", "1.0", "v1.2.3")

#### 💡 Examples

**Basic HTTP Endpoint**:

```rust
use sdforge::prelude::*;

#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    Ok(User { id, name: "John Doe".into() })
}
```

**With Caching**:

```rust
#[service_api(
    name = "get_config",
    version = "v1",
    path = "/config",
    method = "GET",
    cache_ttl = 600
)]
async fn get_config() -> Result<Config, ApiError> {
    Ok(Config::default())
}
```

**With Streaming**:

```rust
#[service_api(
    name = "stream_events",
    version = "v1",
    path = "/events",
    method = "GET",
    stream = true
)]
async fn stream_events() -> Result<StreamResponse, ApiError> {
    let (tx, rx) = create_stream_channel();
    Ok(StreamResponse::new(rx))
}
```

**With Authentication**:

```rust
#[service_api(
    name = "update_profile",
    version = "v1",
    path = "/profile",
    method = "POST"
)]
async fn update_profile(
    auth: BearerAuth,
    profile: Profile
) -> Result<User, ApiError> {
    // auth.token contains the JWT
    Ok(User::from(profile))
}
```

**MCP Tool**:

```rust
#[service_api(
    name = "calculate",
    version = "v1",
    tool_name = "calculate",
    description = "Perform a calculation"
)]
async fn calculate(expression: String) -> Result<f64, ApiError> {
    // This will be available as an MCP tool
    Ok(eval_expression(&expression)?)
}
```

**Multi-Protocol (HTTP + MCP)**:

```rust
#[service_api(
    name = "get_data",
    version = "v1",
    path = "/data/:id",
    method = "GET",
    tool_name = "get_data",
    description = "Get data by ID"
)]
async fn get_data(id: u64) -> Result<Data, ApiError> {
    // Available via both HTTP and MCP
    Ok(Data::get(id)?)
}
```

---

### `service_module` Macro

Groups related API endpoints under a common prefix.

#### 📝 Syntax

```rust
#[service_module(prefix = "/prefix")]
mod module_name {
    use super::*;

    #[service_api(...)]
    async fn api1() -> Result<T, ApiError> { ... }

    #[service_api(...)]
    async fn api2() -> Result<T, ApiError> { ... }
}
```

#### 📊 Attributes

| Attribute | Type   | Required | Description                                 |
|-----------|--------|----------|---------------------------------------------|
| `prefix`   | String | Yes      | URL path prefix for all endpoints in module   |

#### 💡 Examples

**Basic Module**:

```rust
#[service_module(prefix = "/auth")]
mod auth {
    use super::*;

    #[service_api(
        name = "login",
        version = "v1",
        path = "/login",
        method = "POST"
    )]
    async fn login(credentials: Credentials) -> Result<Token, ApiError> {
        Ok(Token::new())
    }

    #[service_api(
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

Creates endpoints:
- `/auth/api/v1/login`
- `/auth/api/v1/logout`

**Nested Modules**:

```rust
#[service_module(prefix = "/users")]
mod users {
    use super::*;

    #[service_module(prefix = "/admin")]
    mod admin {
        use super::*;

        #[service_api(
            name = "create_user",
            version = "v1",
            path = "/create",
            method = "POST"
        )]
        async fn create_user(user: NewUser) -> Result<User, ApiError> {
            Ok(User::create(user)?)
        }
    }
}
```

Creates endpoint:
- `/users/admin/api/v1/create`

---

### `test_macro` Macro

Helper macro for testing API endpoints.

#### 📝 Syntax

```rust
#[test_macro]
async fn test_my_api() {
    // Test implementation
}
```

#### 💡 Examples

```rust
#[test_macro]
async fn test_get_user() {
    let result = get_user(123).await;
    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.id, 123);
}
```

---

## 🔧 Core Types

### `ApiMetadata`

Metadata about an API endpoint.

```rust
pub struct ApiMetadata { /* fields are pub(crate), use constructor and accessors */ }
```

**Construction:**
```rust
let metadata = ApiMetadata::new(
    "api_name".to_string(),      // name
    "v1".to_string(),            // version
    "Description".to_string(),   // description
    Some(300),                   // cache_ttl: Option<u64>
    false,                       // is_streaming
);
```

| Accessor Method | Return Type | Description                              |
|-----------------|-------------|------------------------------------------|
| `name()`        | `&str`      | API identifier                           |
| `version()`     | `&str`      | API version                             |
| `description()` | `&str`      | API description                          |
| `cache_ttl()`   | `Option<u64>` | Cache TTL in seconds, None if disabled |
| `is_streaming()` | `bool`     | Whether streaming is enabled            |

### `ServiceResponse<T>`

Wrapper for successful API responses.

```rust
pub struct ServiceResponse<T> { /* fields are pub(crate), use methods */ }
```

**Construction:**
```rust
// Create a successful response
let response = ServiceResponse::success(data);

// Create an error response
let response = ServiceResponse::error(ServiceError::new("ERROR", "message", 500));
```

| Method          | Return Type           | Description                             |
|-----------------|----------------------|-----------------------------------------|
| `success(data)` | `Self`               | Create successful response (static)     |
| `error(err)`   | `Self`               | Create error response (static)          |
| `is_success()` | `bool`               | Check if response is successful        |
| `data()`        | `Option<&T>`         | Get reference to response data         |
| `error_ref()`   | `Option<&ServiceError>` | Get reference to error details       |

### `ApiError`

Framework error enum for API operations.

```rust
pub enum ApiError {
    NotFound { resource: String, resource_id: Option<String> },
    InvalidInput { message: String, field: Option<String>, value: Option<Value> },
    AuthenticationFailed { reason: String },
    AccessDenied { permission: String, user_id: Option<String> },
    RateLimitExceeded { limit: u32, window_seconds: u32 },
    Internal { message: String, error_id: String },
    ServiceUnavailable { service: String, retry_after: Option<u64> },
    ValidationError { field: String, constraint: String },
}
```

**Constructor:**
```rust
ApiError::validation_error("code", "message") // Returns InvalidInput variant
```

### `ServiceError`

Detailed error type with structured information.

```rust
pub struct ServiceError {
    // Fields are pub(crate), use accessor methods
}
```

**Construction:**
```rust
// Basic error
ServiceError::new("NOT_FOUND", "Resource not found", 404)

// Error with details
ServiceError::with_details(
    "VALIDATION_ERROR",
    "Invalid input",
    json!({ "field": "email" }),
    400
)
```

| Method          | Return Type              | Description                        |
|-----------------|-------------------------|------------------------------------|
| `code()`        | `&str`                  | Error code                         |
| `message()`     | `&str`                  | Human-readable error message       |
| `details()`     | `Option<&serde_json::Value>` | Additional error details      |
| `http_status()` | `u16`                   | HTTP status code                   |

---

## ❌ Error Types

### 📋 Predefined Error Codes

| Code              | Status | Description                        |
|-------------------|--------|------------------------------------|
| `BAD_REQUEST`     | 400    | Invalid request parameters           |
| `UNAUTHORIZED`    | 401    | Authentication required              |
| `FORBIDDEN`       | 403    | Access denied                      |
| `NOT_FOUND`       | 404    | Resource not found                |
| `METHOD_NOT_ALLOWED` | 405 | HTTP method not allowed          |
| `CONFLICT`        | 409    | Resource conflict                 |
| `VALIDATION_ERROR` | 400    | Input validation failed           |
| `INTERNAL_ERROR`  | 500    | Internal server error              |

### ✏️ Creating Custom Errors

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("User not found: {0}")]
    UserNotFound(u64),

    #[error("Invalid email: {0}")]
    InvalidEmail(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

impl From<MyError> for ServiceError {
    fn from(err: MyError) -> Self {
        match err {
            MyError::UserNotFound(id) => ServiceError::with_details(
                "USER_NOT_FOUND",
                format!("User with ID {} not found", id),
                json!({ "user_id": id }),
                404,
            ),
            MyError::InvalidEmail(email) => ServiceError::with_details(
                "INVALID_EMAIL",
                format!("Invalid email address: {}", email),
                json!({ "email": email }),
                400,
            ),
            MyError::DatabaseError(e) => ServiceError::new(
                "DATABASE_ERROR",
                "Database operation failed",
                500,
            ),
        }
    }
}
```

---

## 🌐 HTTP API

### 🏗️ Building HTTP Server

```rust
use sdforge::http;

#[tokio::main]
async fn main() {
    // Build default server
    let app = http::build();

    // Build with configuration
    let config = ServerConfig {
        host: "0.0.0.0".into(),
        port: 3000,
        ..Default::default()
    };
    let app = http::build_with_config(config);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### 📨 Request Types

#### `HttpRoute`

HTTP route metadata.

```rust
pub struct HttpRoute {
    pub path: String,
    pub method: String,
    pub metadata: ApiMetadata,
}
```

### 📤 Response Types

All types implementing `axum::response::IntoResponse` can be returned.

```rust
// Return custom type
#[derive(Serialize)]
struct User { id: u64, name: String }

async fn get_user() -> Result<User, ApiError> {
    Ok(User { id: 1, name: "John".into() })
}

// Return ServiceResponse
async fn get_user_wrapped() -> Result<ServiceResponse<User>, ApiError> {
    Ok(ServiceResponse::new(User { id: 1, name: "John".into() }))
}

// Return String
async fn get_status() -> Result<String, ApiError> {
    Ok("OK".to_string())
}
```

### 🛤️ Path Parameters

Path parameters are automatically extracted based on function parameter names.

```rust
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    // `id` comes from path `/users/:id`
    Ok(User::get(id)?)
}
```

**Supported Types**:
- `u8`, `u16`, `u32`, `u64`, `usize`
- `i8`, `i16`, `i32`, `i64`, `isize`
- `String`
- `bool`

### 🔍 Query Parameters

Use axum extractors for query parameters.

```rust
use axum::extract::Query;

#[derive(serde::Deserialize)]
struct UserQuery {
    page: Option<u32>,
    limit: Option<u32>,
    sort: Option<String>,
}

#[service_api(
    name = "list_users",
    version = "v1",
    path = "/users",
    method = "GET"
)]
async fn list_users(Query(query): Query<UserQuery>) -> Result<Vec<User>, ApiError> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);
    Ok(User::list(page, limit)?)
}
```

### 📦 Request Body

Request body is automatically deserialized.

```rust
#[derive(serde::Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[service_api(
    name = "create_user",
    version = "v1",
    path = "/users",
    method = "POST"
)]
async fn create_user(req: CreateUserRequest) -> Result<User, ApiError> {
    Ok(User::create(req)?)
}
```

---

## 🤖 MCP API

### 🔧 MCP Tool Registration

Tools are automatically registered with the MCP server.

```rust
#[service_api(
    name = "weather",
    version = "v1",
    tool_name = "get_weather",
    description = "Get weather forecast for a location"
)]
async fn get_weather(location: String) -> Result<Weather, ApiError> {
    Ok(Weather::fetch(location)?)
}
```

### 📦 `McpToolInstance`

Represents a registered MCP tool.

```rust
pub struct McpToolInstance {
    pub name: String,
    pub description: String,
    pub handler: HandlerFn,
}
```

---

## 🛡️ Security API

### 🔑 Authentication

#### BearerAuth

Extract JWT bearer token from Authorization header.

```rust
use sdforge::security::BearerAuth;

#[service_api(
    name = "protected",
    version = "v1",
    path = "/protected",
    method = "GET"
)]
async fn protected_endpoint(auth: BearerAuth) -> Result<String, ApiError> {
    // auth.token contains the JWT token
    Ok(format!("Hello, authenticated user!"))
}
```

#### ApiKeyAuth

Extract API key from headers.

```rust
use sdforge::security::ApiKeyAuth;

#[service_api(
    name = "api_endpoint",
    version = "v1",
    path = "/api",
    method = "GET"
)]
async fn api_endpoint(auth: ApiKeyAuth) -> Result<String, ApiError> {
    // auth.key contains the API key
    Ok("Authenticated with API key")
}
```

### ⏱️ Rate Limiting

#### RateLimiter

Rate limiting middleware.

```rust
use sdforge::security::{RateLimiter, RateLimitConfig};

#[tokio::main]
async fn main() {
    let config = RateLimitConfig {
        requests_per_minute: 60,
        burst_size: 10,
        ..Default::default()
    };

    let limiter = RateLimiter::new(config);
    let app = http::build().layer(limiter.middleware());

    // Start server...
}
```

### 📝 Audit Logging

#### AuditLogger

Log security events.

```rust
use sdforge::security::AuditLogger;

async fn login(credentials: Credentials) -> Result<Token, ApiError> {
    let result = authenticate(credentials)?;

    // Log successful login
    AuditLogger::log(AuditEvent {
        action: "login".into(),
        user_id: Some(result.user_id),
        ip: Some(client_ip),
        timestamp: Utc::now(),
        success: true,
    });

    Ok(result)
}
```

---

## 💾 Cache API

### 🔧 CacheMiddleware

Cache responses based on `cache_ttl` attribute.

```rust
use sdforge::cache::{CacheMiddleware, CacheConfig};

#[tokio::main]
async fn main() {
    let config = CacheConfig {
        enabled: true,
        ttl: 300,
        backend: CacheBackend::Memory,
    };

    let cache = CacheMiddleware::new(config);
    let app = http::build().layer(cache);

    // Start server...
}
```

### 🔴 Redis Caching

```rust
use sdforge::cache::{CacheConfig, CacheBackend};
use redis::Client;

#[tokio::main]
async fn main() {
    let client = Client::open("redis://localhost")?;
    let config = CacheConfig {
        enabled: true,
        ttl: 300,
        backend: CacheBackend::Redis(client),
    };

    let cache = CacheMiddleware::new(config);
    let app = http::build().layer(cache);

    // Start server...
}
```

---

## 💬 WebSocket API

### 🔌 WebSocket Connection

```rust
use sdforge::websocket::{WebSocketHandler, WebSocketMessage};

#[service_api(
    name = "websocket",
    version = "v1",
    ws_path = "/ws"
)]
async fn websocket_handler(
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move {
        let mut handler = WebSocketHandler::new(socket);

        while let Some(msg) = handler.next().await {
            match msg {
                WebSocketMessage::Text(text) => {
                    handler.send_text(format!("Echo: {}", text)).await;
                }
                WebSocketMessage::Close => break,
                _ => {}
            }
        }
    })
}
```

### 🎛️ WebSocketManager

Manage multiple WebSocket connections.

```rust
use sdforge::websocket::{WebSocketManager, broadcast};

async fn broadcast_message(message: String) {
    WebSocketManager::broadcast(message).await;
}
```

---

## 🔌 gRPC API

### 🖥️ gRPC Server

```rust
use sdforge::grpc::{build_server, GrpcServerConfig};
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = GrpcServerConfig {
        addr: "[::1]:50051".parse()?,
        ..Default::default()
    };

    let server = build_server(config)?;
    Server::builder()
        .add_service(server)
        .serve(config.addr)
        .await?;

    Ok(())
}
```

### 🔧 Generated Services

```rust
use sdforge::grpc::axiom_v1::axiom_service_server::AxiomServiceServer;
use sdforge::grpc::axiom_v1::{CallRequest, CallResponse};

#[tonic::async_trait]
impl AxiomService for MyService {
    async fn call(
        &self,
        request: Request<CallRequest>,
    ) -> Result<Response<CallResponse>, Status> {
        // Handle gRPC call
    }
}
```

---

## ⚙️ Configuration API

### 🖥️ ServerConfig

HTTP server configuration.

```rust
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub timeout: Duration,
}
```

### 🔑 AuthConfig

Authentication configuration.

```rust
pub struct AuthConfig {
    pub enabled: bool,
    pub jwt_secret: String,
    pub token_ttl: u64,
}
```

### 💾 CacheConfig

Cache configuration.

```rust
pub enum CacheBackend {
    Memory,
    Redis(Client),
}

pub struct CacheConfig {
    pub enabled: bool,
    pub ttl: u64,
    pub backend: CacheBackend,
}
```

### 📥 Load Configuration

```rust
use sdforge::config::ConfigLoader;

#[tokio::main]
async fn main() {
    let config = ConfigLoader::load("axiom.toml").unwrap();
    let app = http::build_with_config(config.server);

    // Start server...
}
```

### 🔄 Hot Reload

```rust
use sdforge::config::hot_reload::ConfigWatcher;

#[tokio::main]
async fn main() {
    let mut watcher = ConfigWatcher::new("axiom.toml").await.unwrap();

    tokio::spawn(async move {
        while let Ok(event) = watcher.next().await {
            match event {
                ConfigEvent::Updated => println!("Config updated"),
                ConfigEvent::Error(e) => eprintln!("Config error: {}", e),
            }
        }
    });

    // Start server...
}
```

---

## 📡 Streaming API

### 🔄 SSE Streaming

```rust
use sdforge::streaming::{StreamResponse, StreamEvent};

#[service_api(
    name = "stream_events",
    version = "v1",
    path = "/events",
    method = "GET",
    stream = true
)]
async fn stream_events() -> Result<StreamResponse, ApiError> {
    let (tx, rx) = create_stream_channel();

    tokio::spawn(async move {
        for i in 0..10 {
            let event = StreamEvent::data(format!("Event {}", i));
            tx.send(event).await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    Ok(StreamResponse::new(rx))
}
```

### 🎨 Custom Stream

```rust
use tokio_stream::StreamExt;
use futures_util::stream;

async fn custom_stream() -> impl Stream<Item = String> {
    stream::iter(vec!["Item 1", "Item 2", "Item 3"])
        .map(|s| s.to_string())
}
```

---

## 💡 Best Practices

### ❌ Error Handling

```rust
// Always use Result<T, ApiError>
async fn my_api() -> Result<Response, ApiError> {
    // Handle errors explicitly
    match some_operation() {
        Ok(data) => Ok(data),
        Err(e) => Err(ApiError::from(e)),
    }
}
```

### 🛤️ Path Parameter Naming

Use descriptive names for path parameters:

```rust
// ✅ Good
path = "/users/:user_id/posts/:post_id"
async fn get_post(user_id: u64, post_id: u64) { ... }

// ❌ Avoid
path = "/users/:id1/posts/:id2"
async fn get_post(id1: u64, id2: u64) { ... }
```

### 🔢 Version Management

Use semantic versioning:

```rust
// Stable version
version = "v1"

// Pre-release
version = "v2.0.0-beta"

// Major version
version = "v3"
```

### 📝 Documentation

Always add descriptions for MCP tools:

```rust
#[service_api(
    name = "calculate",
    version = "v1",
    tool_name = "calculate",
    description = "Performs mathematical calculations. Supports basic arithmetic: +, -, *, /"
)]
async fn calculate(expr: String) -> Result<f64, ApiError> { ... }
```

---

## 🏷️ Type Aliases

Axiom provides convenient type aliases in `prelude`:

```rust
use sdforge::prelude::*;

type Result<T> = std::result::Result<T, ApiError>;
```

---

## 🚦 Feature Flags

API availability depends on enabled features:

| Feature      | Additional APIs                              |
|--------------|---------------------------------------------|
| `http`       | HTTP routes, middleware                     |
| `mcp`        | MCP tools                                   |
| `security`    | Auth, rate limiting, audit logging           |
| `cache`       | Cache middleware                            |
| `websocket`   | WebSocket handler, manager                   |
| `grpc`        | gRPC server, generated services              |
| `streaming`   | SSE streaming                              |

---

## 📚 Related Documentation

- [📋 README](../README.md)
- [🏗️ ARCHITECTURE.md](./ARCHITECTURE.md)
- [🤝 CONTRIBUTING.md](./CONTRIBUTING.md)

---

<div align="center">

**Built with ❤️ using Rust**

</div>