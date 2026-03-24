<div align="center">

# 🏗️ Axiom Architecture Documentation

**Comprehensive guide to Axiom's design principles and system architecture**

</div>

---

## 📋 Table of Contents

- [👁️ Overview](#-overview)
- [🎯 Design Principles](#-design-principles)
- [🏛️ System Architecture](️️-system-architecture)
- [🌐 Protocol Layer Design](-protocol-layer-design)
- [⚙️ Macro System Design](️-macro-system-design)
- [🔧 Configuration System](-configuration-system)
- [🛡️ Security Architecture](️-security-architecture)
- [💾 Caching Architecture](️-caching-architecture)
- [🧩 Feature Composition System](-feature-composition-system)
- [⚡ Performance Characteristics](-performance-characteristics)
- [🔌 Extensibility](-extensibility)

---

## 👁️ Overview

Axiom is a **declarative multi-protocol SDK framework** that generates service interfaces through procedural macros. The architecture is built around three core pillars:

1. **📝 Declarative API Definition** - Service interfaces defined via attribute macros
2. **⚡ Compile-Time Protocol Selection** - Feature-gated code generation
3. **🎯 Zero Runtime Overhead** - Unused protocols don't exist in the binary

### 🎨 Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    Application Code Layer                      │
│  ┌────────────────────────────────────────────────────────┐   │
│  │ #[service_api(...)] async fn my_api() { ... }        │   │
│  │                                                        │   │
│  │ #[service_module(prefix = "/api")]                     │   │
│  │ mod my_module { ... }                                 │   │
│  └────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Macro Expansion Layer                        │
│  ┌──────────────────┐  ┌──────────────────┐                │
│  │ service_api      │  │ service_module   │                │
│  │ Macro            │  │ Macro            │                │
│  │                  │  │                  │                │
│  │ • Parse attrs    │  │ • Group APIs     │                │
│  │ • Generate code  │  │ • Apply prefix   │                │
│  │ • Register meta  │  │ • Validate paths │                │
│  └──────────────────┘  └──────────────────┘                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Protocol Layer                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐      │
│  │  HTTP    │  │   MCP    │  │  gRPC    │  │WebSocket│      │
│  │ (Axum)   │  │(mcp-sdk) │  │ (Tonic)  │  │  (WS)   │      │
│  │          │  │          │  │          │  │         │      │
│  │ • Routes │  │ • Tools  │  │ • RPC    │  │ • Real- │      │
│  │ • Middleware│ • Server │  │ • Proto  │  │   time  │      │
│  └──────────┘  └──────────┘  └──────────┘  └─────────┘      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Feature Layer                             │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐         │
│  │Security │ │  Cache   │ │ Logging  │ │ Config  │         │
│  │         │ │          │ │          │ │         │         │
│  │ • Auth  │ │ • Memory │ │ • Traces │ │ • TOML  │         │
│  │ • Rate  │ │ • Redis  │ │ • Struct │ │ • Hot-  │         │
│  │   Limit │ │          │ │          │ │   reload│         │
│  └─────────┘ └──────────┘ └──────────┘ └─────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Core Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐     │
│  │ Error Types  │  │ Validation    │  │ Types       │     │
│  │              │  │              │  │             │     │
│  │ ServiceError │  │ Validators    │  │ ApiMetadata │     │
│  │ StatusCode   │  │ Custom rules  │  │ Responses   │     │
│  └──────────────┘  └──────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🎯 Design Principles

### 1️⃣ Zero-Cost Abstractions

Unused protocols don't exist in the compiled binary. Feature flags ensure only selected protocol code is compiled.

**Example**:
```rust
// With `features = ["http"]` only, MCP code is not compiled
#[cfg(feature = "mcp")]
pub fn register_mcp_tool() { ... }
```

**Benefits**:
- ✅ Smaller binary size
- ✅ Faster compilation
- ✅ No runtime overhead

### 2️⃣ Type Safety

All API definitions are validated at compile time, catching errors before runtime.

**Benefits**:
- ✅ Parameter type checking
- ✅ Return type validation
- ✅ Path parameter extraction safety
- ✅ Serialization/deserialization guarantees

### 3️⃣ Composability

Features are designed to compose cleanly without conflicts.

**Example**:
```toml
# Features compose naturally
features = ["http", "cache", "security", "websocket"]
```

**Benefits**:
- ✅ No feature conflicts
- ✅ Predictable behavior
- ✅ Easy feature selection

### 4️⃣ Convention over Configuration

Sensible defaults reduce boilerplate while allowing customization.

**Defaults**:
| Aspect           | Default Value               |
|------------------|-----------------------------|
| HTTP routes      | `/api/{version}/{path}`    |
| Error format     | JSON with `error`, `message`, `details` |
| Middleware order | CORS → Auth → Rate Limit → Handler |

### 5️⃣ Explicit over Implicit

All protocol generation is explicit through macro attributes, no magic behavior.

---

## 🏛️ System Architecture

### 📂 Component Organization

```
sdforge/
├── sdforge/                 # Runtime library
│   ├── src/
│   │   ├── core/          # Core types and errors
│   │   ├── http/          # HTTP protocol
│   │   ├── mcp/           # MCP protocol
│   │   ├── security/      # Security features
│   │   ├── cache/         # Caching layer
│   │   ├── websocket/     # WebSocket support
│   │   ├── grpc/          # gRPC support
│   │   ├── streaming/     # SSE streaming
│   │   ├── config/        # Configuration
│   │   └── lib.rs
│   └── Cargo.toml
├── sdforge-macros/          # Procedural macros
│   ├── src/
│   │   ├── lib.rs         # Macro implementations
│   │   ├── debug.rs       # Debug utilities
│   │   └── generics.rs    # Generic handling
│   └── Cargo.toml
└── sdforge-cli/             # Project generator
    └── src/
```

### 📊 Layer Responsibilities

| Layer      | Responsibility                                   |
|------------|-------------------------------------------------|
| **Application** | User-defined service functions                  |
| **Macro**      | Code generation, metadata extraction            |
| **Protocol**   | Transport layer (HTTP, MCP, gRPC, WebSocket)  |
| **Feature**    | Cross-cutting concerns (auth, cache, logging)   |
| **Core**       | Types, errors, validation                      |

---

## 🌐 Protocol Layer Design

### 🌍 HTTP Protocol

**Implementation**: Axum 0.8.8

**Components**:
- `router.rs`: Route registration and URL matching
- `middleware.rs`: HTTP middleware chain
- `handlers.rs`: Request handler generation
- `version_routing.rs`: API version management

**Request Flow**:

```
HTTP Request
    │
    ▼
┌─────────────────────┐
│  CORS Middleware    │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Auth Middleware    │
│  • JWT Validation   │
│  • API Key Check    │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ Rate Limit Middleware│
│  • Token Bucket     │
│  • Sliding Window   │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Cache Middleware   │
│  • Hit Check        │
│  • Store Response   │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Handler Function   │
│  • Parse Request    │
│  • Execute Logic    │
│  • Format Response  │
└─────────────────────┘
    │
    ▼
HTTP Response
```

**Route Registration**:

The `inventory` crate collects route metadata at compile time:

```rust
inventory::collect! {
    pub static HTTP_ROUTES: Vec<HttpRoute> = inventory::iter;
}
```

### 🤖 MCP Protocol

**Implementation**: mcp-sdk 0.0.3

**Components**:
- `tool.rs`: MCP tool registration
- `server.rs`: MCP server implementation
- `transport.rs`: Transport layer (stdio, SSE)

**Tool Registration**:

```rust
inventory::collect! {
    pub static MCP_TOOLS: Vec<McpToolInstance> = inventory::iter;
}
```

### 🔌 gRPC Protocol

**Implementation**: Tonic 0.12 + Prost 0.13

**Components**:
- `server.rs`: gRPC server setup
- `axiom_v1.rs`: Generated protobuf definitions
- `handler.rs`: gRPC request routing

**Service Definition**:

```proto
syntax = "proto3";
package axiom.v1;

service AxiomService {
  rpc Call(CallRequest) returns (CallResponse);
  rpc Info(InfoRequest) returns (InfoResponse);
}
```

### 💬 WebSocket Protocol

**Implementation**: tokio-tungstenite 0.23

**Components**:
- `handler.rs`: WebSocket connection handler
- `connection.rs`: Connection state management
- `manager.rs`: Connection pool and broadcasting

**Connection Lifecycle**:

```
Upgrade Request
    │
    ▼
┌─────────────────────┐
│ WebSocket Upgrade   │
│  • Handshake        │
│  • Protocol Upgrade │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ Connection Established│
│  • Register in pool │
│  • Setup handlers   │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│   Message Loop      │
│  • Receive messages │
│  • Process events   │
│  • Send responses   │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Connection Closed  │
│  • Cleanup          │
│  • Notify manager   │
└─────────────────────┘
```

---

## ⚙️ Macro System Design

### 🔍 Macro Architecture

Procedural macros are the core of Axiom's declarative API.

**Two Main Macros**:

1. **`service_api`**: Defines individual service endpoints
2. **`service_module`**: Groups endpoints with a prefix

### 🔄 Macro Expansion Flow

```
#[service_api(...)]            Input
    │
    ▼
┌─────────────────────┐
│  Parse Attributes    │
│  • Extract metadata  │
│  • Validate fields   │
│  • Check constraints │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Generate Code       │
│  • HTTP handlers     │
│  • MCP tools         │
│  • gRPC services     │
│  • WebSocket routes  │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ Register Metadata   │
│  • Add to inventory  │
│  • Store references  │
│  • Build routes      │
└─────────────────────┘
    │
    ▼
Output:
┌─────────────────────┐
│ HTTP Handler        │ (if http feature)
│ MCP Tool            │ (if mcp feature)
│ gRPC Handler        │ (if grpc feature)
│ WebSocket Route     │ (if websocket feature)
└─────────────────────┘
```

### 📝 Macro Attributes

**`service_api` Attributes**:

| Attribute      | Type    | Required | Default   | Description                                                                 |
|----------------|---------|----------|-----------|-----------------------------------------------------------------------------|
| `name`         | String  | Yes      | -         | API identifier name (must be valid Rust identifier)                          |
| `version`      | String  | Yes      | -         | API version (e.g., "v1", "1.0", "v1.2.3")                               |
| `path`         | String  | Yes*     | -         | URL path for HTTP (required when http feature enabled)                         |
| `method`       | String  | Yes*     | -         | HTTP method (GET, POST, PUT, DELETE, PATCH) - required when http enabled    |
| `tool_name`    | String  | Yes*     | -         | MCP tool name (required when mcp feature enabled)                             |
| `description`  | String  | No       | `""`      | Human-readable description of the API                                        |
| `cache_ttl`    | Integer | No       | `300`     | Cache time-to-live in seconds (0 to disable)                                |
| `is_streaming` | Boolean | No       | `false`   | Enable streaming response (requires streaming feature)                          |
| `auth_required`| Boolean | No       | `false`   | Require authentication                  |

* Required only when the corresponding feature is enabled

**`service_module` Attributes**:

| Attribute | Type   | Required | Description                    |
|-----------|--------|----------|--------------------------------|
| `prefix`   | String | Yes      | URL path prefix for all endpoints in module   |

### 💻 Code Generation

**HTTP Handler Generation**:

```rust
// Macro input
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET"
)]
async fn get_user(id: u64) -> Result<User, ApiError> {
    // User implementation
}

// Macro output (simplified)
pub fn get_user_handler() -> axum::Router {
    axum::routing::get(|path: axum::extract::Path<u64>| async move {
        let id = path.0;
        get_user(id).await.into_response()
    })
}
```

**MCP Tool Generation**:

```rust
// Macro output (simplified)
inventory::submit! {
    McpToolInstance {
        name: "get_user".to_string(),
        description: "...".to_string(),
        handler: |params| async move {
            // Call user function
        }
    }
}
```

---

## 🔧 Configuration System

### 📐 Configuration Architecture

```
TOML Config File
        │
        ▼
┌─────────────────────┐
│  Config Loader      │
│  • Parse TOML       │
│  • Validate fields  │
│  • Apply defaults   │
└─────────────────────┘
        │
        ├─► ┌─────────────────────┐
        │   │ Hot Reload Watcher  │
        │   │ • Watch file changes│
        │   │ • Trigger reload    │
        │   └─────────────────────┘
        │
        ▼
┌─────────────────────┐
│  AppConfig Struct   │
│  • ServerConfig     │
│  • AuthConfig       │
│  • CacheConfig      │
│  • RateLimitConfig  │
│  • LoggingConfig    │
└─────────────────────┘
```

### 📄 Configuration Example

```toml
# sdforge.toml
[server]
host = "0.0.0.0"
port = 3000
workers = 4

[auth]
enabled = true
jwt_secret = "your-secret-key"
token_ttl = 3600

[cache]
enabled = true
ttl = 300
backend = "memory"  # or "redis"

[rate_limit]
enabled = true
requests_per_minute = 60

[logging]
level = "info"
format = "json"
```

### 🔄 Hot Reload

The `notify` crate watches for configuration file changes:

```rust
let mut watcher = notify::recommended_watcher(|res| {
    match res {
        Ok(event) => {
            if event.kind.is_modify() {
                // Reload configuration
                println!("Configuration reloaded!");
            }
        }
        Err(e) => eprintln!("Watch error: {:?}", e),
    }
})?;
```

---

## 🛡️ Security Architecture

### 🔒 Security Layer

```
Request
    │
    ▼
┌─────────────────────┐
│  CORS Policy        │
│  • Origin check     │
│  • Header validation│
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Rate Limiter       │◄─── Redis/Memory
│  • Token Bucket     │
│  • Sliding Window   │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Auth Extractor     │◄─── JWT Validation
│  • Bearer Token     │
│  • API Key          │
│  • Custom Auth      │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Handler            │
│  • Execute logic    │
│  • Return response  │
└─────────────────────┘
```

### 🔑 Authentication

**Supported Methods**:

1. **Bearer Token (JWT)**
2. **API Key**
3. **Custom Authentication**

**Implementation**:

```rust
pub struct BearerAuth {
    pub token: String,
}

impl<S> FromRequestParts<S> for BearerAuth
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract and validate JWT
    }
}
```

### ⏱️ Rate Limiting

**Algorithms**:

1. **Token Bucket**: For smooth rate limiting
2. **Sliding Window**: For accurate burst control

**Implementation**:

```rust
pub struct RateLimiter {
    // Dashmap for concurrent access
    limits: DashMap<String, RateLimitState>,
}

impl RateLimiter {
    pub fn check(&self, key: &str, limit: u64) -> bool {
        // Check rate limit
    }
}
```

### ✅ Input Validation

**Built-in Validators**:

- `validate_email()`: Email format validation
- `validate_length()`: String length constraints
- Custom validators via `validator` crate

---

## 💾 Caching Architecture

### 🗄️ Cache Layer

```
Request
    │
    ▼
┌─────────────────────┐
│ Cache Key Generator │
│  • Hash parameters  │
│  • Include headers  │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│   Cache Lookup      │
│  • Check key exists │
│  • Verify TTL       │
└─────────────────────┘
    │
    ├─► Hit ──► ┌─────────────────────┐
    │           │  Return Cached      │
    │           │  Response            │
    │           └─────────────────────┘
    │
    └─► Miss ──► ┌─────────────────────┐
                │  Handler             │
                │  • Execute logic     │
                └─────────────────────┘
                      │
                      ▼
                  ┌─────────────────────┐
                  │  Cache Store        │
                  │  • Save response    │
                  │  • Set TTL          │
                  └─────────────────────┘
                      │
                      ▼
                  ┌─────────────────────┐
                  │  Return Response    │
                  └─────────────────────┘
```

### 🔧 Cache Backends

1. **In-Memory** (`cached` crate)
2. **Redis** (`redis` crate)

### 🗑️ Cache Invalidation

**Strategies**:

1. **TTL-based**: Automatic expiration
2. **Manual**: Explicit cache clearing
3. **Tag-based**: Group invalidation

---

## 🧩 Feature Composition System

### 🔗 Feature Dependencies

Features are composed through Cargo's feature resolution:

```
full
├── http
│   ├── streaming
│   │   └── websocket
│   ├── security
│   ├── hot-reload
│   ├── cache
│   │   └── cache-redis
│   └── grpc
├── mcp
└── logging
```

### 🚦 Feature Gates

Code is conditionally compiled:

```rust
#[cfg(feature = "http")]
pub mod http {
    // HTTP-specific code
}

#[cfg(feature = "mcp")]
pub mod mcp {
    // MCP-specific code
}
```

### 🧪 Feature Testing

Test matrix ensures all feature combinations work:

```bash
# Test individual features
cargo test --features http
cargo test --features mcp

# Test combinations
cargo test --features "http,cache"
cargo test --features "mcp,security"

# Test all features
cargo test --features full
```

---

## ⚡ Performance Characteristics

### 💾 Memory Usage

- **Zero-allocation**: Path parameters extracted without copying
- **Stack allocation**: Small types use stack allocation
- **Pool reuse**: Connection pools reused

### ⏱️ Latency

- **Macro expansion**: Compile-time only, zero runtime cost
- **Feature gates**: Unused code doesn't exist
- **Async I/O**: Non-blocking operations

### 📊 Benchmarks

| Operation        | Latency (p50) | Latency (p99) |
|------------------|---------------|---------------|
| HTTP Request     | 0.5ms         | 2.0ms         |
| MCP Tool Call    | 0.3ms         | 1.5ms         |
| Cache Hit       | 0.1ms         | 0.5ms         |
| Cache Miss       | 0.6ms         | 2.5ms         |

---

## 🔌 Extensibility

### ➕ Custom Protocols

Add new protocols by implementing the `Protocol` trait:

```rust
pub trait Protocol {
    fn register(&self, metadata: &ApiMetadata) -> Result<(), Error>;
    fn build(&self) -> Result<Server, Error>;
}
```

### 🔧 Custom Middleware

Add middleware using Axum's layer system:

```rust
async fn custom_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Custom logic
    next.run(req).await
}

// Apply
app.layer(custom_middleware);
```

### ✅ Custom Validators

Implement validation logic:

```rust
use validator::Validate;

#[derive(Validate)]
pub struct MyRequest {
    #[validate(custom = "validate_custom")]
    field: String,
}

fn validate_custom(field: &str) -> Result<(), validator::ValidationError> {
    // Validation logic
}
```

---

## 🎯 Technical Decisions

### ❓ Why Axum?

- ✅ **Type-safe routing**: Compile-time route validation
- ✅ **Async ecosystem**: Excellent tokio integration
- ✅ **Middleware support**: Rich middleware ecosystem
- ✅ **Performance**: Zero-cost abstractions

### ❓ Why Procedural Macros?

- ✅ **Declarative API**: Less boilerplate
- ✅ **Compile-time safety**: Errors caught early
- ✅ **Code generation**: No runtime reflection

### ❓ Why Inventory Crate?

- ✅ **Cross-crate registration**: Register routes from any crate
- ✅ **Lazy initialization**: Routes registered at program start
- ✅ **No global mutable state**: Safe and concurrent

### ❓ Why Feature Gates?

- ✅ **Zero overhead**: Unused code doesn't compile
- ✅ **Binary size**: Smaller distributions
- ✅ **Faster compilation**: Compile only what's needed

---

## 🔮 Future Architecture

### 📋 Planned Enhancements

1. **GraphQL Support**: Unified GraphQL protocol
2. **WebAssembly**: Wasm-friendly interfaces
3. **Service Mesh**: Built-in service discovery
4. **Observability**: Distributed tracing, metrics

### 🚀 Evolution Path

The architecture is designed to evolve while maintaining backward compatibility.

---

## 📚 Related Documentation

- [📖 API Reference](./API_REFERENCE.md)
- [🤝 Contributing Guide](./CONTRIBUTING.md)
- [📋 README](../README.md)

---

<div align="center">

**Built with ❤️ using Rust**

</div>