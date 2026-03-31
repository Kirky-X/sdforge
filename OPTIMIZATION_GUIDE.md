# SDForge 代码优化实施指南

本指南提供具体的代码修改方案，帮助实施 CODE_REVIEW_REPORT.md 中发现的问题。

---

## 🔴 Critical 优先级修复

### 1. 修复硬编码路径依赖

**文件**: `Cargo.toml`

#### 方案 A: 使用相对路径（推荐）

```toml
# 修改前 (行 55,60)
confers = { path = "/home/dev/projects/confers", features = ["watch"] }
oxcache = { version = "0.2", path = "/home/dev/projects/oxcache", features = ["rate-limiting", "dashmap"] }

# 修改后
confers = { path = "../confers", features = ["watch"] }
oxcache = { version = "0.2", path = "../oxcache", features = ["rate-limiting", "dashmap"] }
```

#### 方案 B: 使用 Git 仓库

```toml
# 修改后
confers = { git = "https://github.com/Kirky-X/confers", branch = "main", features = ["watch"] }
oxcache = { git = "https://github.com/Kirky-X/oxcache", branch = "main", features = ["rate-limiting", "dashmap"] }
```

#### 方案 C: 使用 crates.io（最终目标）

```toml
# 修改后
confers = { version = "0.2", features = ["watch"] }
oxcache = { version = "0.2", features = ["rate-limiting", "dashmap"] }
```

**实施步骤**:
1. 确认 confers 和 oxcache 的相对路径或仓库地址
2. 修改 Cargo.toml
3. 运行 `cargo check` 验证编译
4. 运行测试确保功能正常

---

### 2. JWT Secret 安全加固

**文件**: `src/security/bearer.rs`

#### 步骤 1: 添加密钥强度验证

在 `BearerAuth::try_new` 方法中添加验证：

```rust
// src/security/bearer.rs - 在 try_new 方法中

pub fn try_new(secret: &str) -> Result<Self, AuthError> {
    // 验证密钥长度（至少 32 字节）
    if secret.len() < 32 {
        return Err(AuthError::InvalidCredentials(
            "JWT secret must be at least 32 bytes long".to_string()
        ));
    }
    
    // 验证密钥熵值（可选，需要引入 entropy crate）
    // use entropy::shannon_entropy;
    // if shannon_entropy(secret) < 4.0 {
    //     return Err(AuthError::InvalidCredentials(
    //         "JWT secret has insufficient entropy".to_string()
    //     ));
    // }
    
    let key = EncodingKey::from_secret(secret.as_bytes());
    
    // ... 现有初始化逻辑
}
```

#### 步骤 2: 添加工具函数生成安全密钥

在 `src/security/bearer.rs` 模块末尾添加：

```rust
/// Generate a cryptographically secure JWT secret
/// 
/// # Example
/// ```
/// use sdforge::security::generate_secure_jwt_secret;
/// 
/// let secret = generate_secure_jwt_secret();
/// println!("Generated secret: {}", secret);
/// ```
pub fn generate_secure_jwt_secret() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::encode(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_secure_jwt_secret() {
        let secret1 = generate_secure_jwt_secret();
        let secret2 = generate_secure_jwt_secret();
        
        // 验证长度（32 字节的 base64 编码为 44 字符）
        assert_eq!(secret1.len(), 44);
        assert_eq!(secret2.len(), 44);
        
        // 验证唯一性
        assert_ne!(secret1, secret2);
        
        // 验证 try_new 接受生成的密钥
        assert!(BearerAuth::try_new(&secret1).is_ok());
    }
    
    #[test]
    fn test_try_new_rejects_weak_secrets() {
        // 太短的密钥
        assert!(BearerAuth::try_new("short").is_err());
        assert!(BearerAuth::try_new("1234567890123456789012345678901").is_err()); // 31 字节
        
        // 刚好 32 字节
        assert!(BearerAuth::try_new("12345678901234567890123456789012").is_ok());
    }
}
```

#### 步骤 3: 更新文档示例

在 `docs/ARCHITECTURE.md` 或其他文档中更新配置示例：

```toml
# 修改前
[auth]
jwt_secret = "your-secret-key"  # ❌ 弱密钥示例

# 修改后
[auth]
# 使用环境变量（推荐）
jwt_secret = "${JWT_SECRET}"  # 从环境变量读取

# 或使用工具生成安全密钥
# 运行：cargo run -- generate-jwt-secret
jwt_secret = "base64-encoded-32-byte-random-string"
```

#### 步骤 4: 添加 CLI 命令（可选）

如果项目有 CLI，添加生成密钥的命令：

```rust
// src/cli/commands/generate.rs

use sdforge::security::generate_secure_jwt_secret;

pub fn generate_jwt_secret() {
    let secret = generate_secure_jwt_secret();
    println!("Generated JWT Secret:");
    println!("{}", secret);
    println!("\nAdd this to your configuration or .env file:");
    println!("JWT_SECRET={}", secret);
}
```

---

## 🟡 High 优先级修复

### 3. 全局状态重构为上下文对象

**文件**: `src/lib.rs`, `src/http/mod.rs`

#### 步骤 1: 创建 SdForgeContext 结构

创建新文件 `src/context.rs`:

```rust
// src/context.rs

//! SDForge application context for managing plugin registrations

use std::sync::{Arc, RwLock};

#[cfg(feature = "http")]
use crate::http::RouteRegistration;
#[cfg(feature = "mcp")]
use crate::mcp::McpToolRegistration;
#[cfg(feature = "websocket")]
use crate::websocket::WebSocketRoute;
#[cfg(feature = "grpc")]
use crate::grpc::GrpcRouteRegistration;

/// Application context that holds all registered plugins and shared state
/// 
/// This replaces the global inventory-based registration system with
/// an explicit context object that can be injected into handlers.
/// 
/// # Example
/// ```ignore
/// use sdforge::SdForgeContext;
/// 
/// let ctx = SdForgeContext::new();
/// ctx.register_http(my_route);
/// 
/// // Share across threads
/// let ctx = Arc::new(ctx);
/// ```
pub struct SdForgeContext {
    #[cfg(feature = "http")]
    http_routes: RwLock<Vec<Box<dyn RouteRegistration>>>,
    
    #[cfg(feature = "mcp")]
    mcp_tools: RwLock<Vec<Box<dyn McpToolRegistration>>>,
    
    #[cfg(feature = "websocket")]
    ws_routes: RwLock<Vec<Box<dyn WebSocketRoute>>>,
    
    #[cfg(feature = "grpc")]
    grpc_routes: RwLock<Vec<Box<dyn GrpcRouteRegistration>>>,
}

impl SdForgeContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "http")]
            http_routes: RwLock::new(Vec::new()),
            
            #[cfg(feature = "mcp")]
            mcp_tools: RwLock::new(Vec::new()),
            
            #[cfg(feature = "websocket")]
            ws_routes: RwLock::new(Vec::new()),
            
            #[cfg(feature = "grpc")]
            grpc_routes: RwLock::new(Vec::new()),
        }
    }
    
    /// Register an HTTP route
    #[cfg(feature = "http")]
    pub fn register_http(&self, route: Box<dyn RouteRegistration>) {
        let mut routes = self.http_routes.write().unwrap();
        routes.push(route);
    }
    
    /// Get all registered HTTP routes
    #[cfg(feature = "http")]
    pub fn get_http_routes(&self) -> Vec<Box<dyn RouteRegistration>> {
        let routes = self.http_routes.read().unwrap();
        routes.clone()
    }
    
    /// Register an MCP tool
    #[cfg(feature = "mcp")]
    pub fn register_mcp_tool(&self, tool: Box<dyn McpToolRegistration>) {
        let mut tools = self.mcp_tools.write().unwrap();
        tools.push(tool);
    }
    
    /// Get all registered MCP tools
    #[cfg(feature = "mcp")]
    pub fn get_mcp_tools(&self) -> Vec<Box<dyn McpToolRegistration>> {
        let tools = self.mcp_tools.read().unwrap();
        tools.clone()
    }
    
    /// Register a WebSocket route
    #[cfg(feature = "websocket")]
    pub fn register_websocket(&self, route: Box<dyn WebSocketRoute>) {
        let mut routes = self.ws_routes.write().unwrap();
        routes.push(route);
    }
    
    /// Register a gRPC route
    #[cfg(feature = "grpc")]
    pub fn register_grpc(&self, route: Box<dyn GrpcRouteRegistration>) {
        let mut routes = self.grpc_routes.write().unwrap();
        routes.push(route);
    }
    
    /// Get counts of registered items
    pub fn counts(&self) -> PluginCounts {
        PluginCounts {
            #[cfg(feature = "http")]
            routes: self.http_routes.read().unwrap().len(),
            
            #[cfg(feature = "mcp")]
            mcp_tools: self.mcp_tools.read().unwrap().len(),
            
            #[cfg(feature = "websocket")]
            ws_routes: self.ws_routes.read().unwrap().len(),
            
            #[cfg(feature = "grpc")]
            grpc_routes: self.grpc_routes.read().unwrap().len(),
        }
    }
}

impl Default for SdForgeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Counts of registered plugins
pub struct PluginCounts {
    #[cfg(feature = "http")]
    pub routes: usize,
    
    #[cfg(feature = "mcp")]
    pub mcp_tools: usize,
    
    #[cfg(feature = "websocket")]
    pub ws_routes: usize,
    
    #[cfg(feature = "grpc")]
    pub grpc_routes: usize,
}

impl std::fmt::Debug for SdForgeContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdForgeContext")
            .field("counts", &self.counts())
            .finish()
    }
}
```

#### 步骤 2: 更新 lib.rs 导出

```rust
// src/lib.rs - 添加导出

/// Application context for managing plugin registrations
pub mod context;

pub use context::{SdForgeContext, PluginCounts};

// 保留旧的 init_all_plugins 用于向后兼容（标记为 deprecated）
#[deprecated(
    since = "0.2.0",
    note = "Use SdForgeContext::new() instead for better testability and isolation"
)]
pub fn init_all_plugins() -> PluginCounts {
    // 调用新的上下文实现或保持旧实现
    let ctx = SdForgeContext::new();
    ctx.counts()
}
```

#### 步骤 3: 更新 HTTP 模块使用上下文

```rust
// src/http/mod.rs - 修改 build_router 函数

use crate::context::SdForgeContext;
use std::sync::Arc;

pub fn build_router(ctx: Arc<SdForgeContext>) -> Router {
    let mut app = Router::new();
    
    // 从上下文获取路由并注册
    #[cfg(feature = "http")]
    {
        let routes = ctx.get_http_routes();
        for route in routes {
            app = route.register_http(app);
        }
    }
    
    // ... 其他中间件配置
    
    // 将上下文作为状态注入
    app.with_state(ctx)
}
```

#### 步骤 4: 更新示例代码

更新 examples 中的使用方式：

```rust
// examples/basic_usage.rs

use sdforge::{SdForgeContext, service_api};
use std::sync::Arc;

#[service_api(
    name = "hello",
    version = "v1",
    path = "/hello",
    method = "GET"
)]
async fn hello() -> Result<String, sdforge::ApiError> {
    Ok("Hello, World!".to_string())
}

#[tokio::main]
async fn main() {
    // 创建上下文
    let ctx = SdForgeContext::new();
    
    // 注册路由（自动通过 inventory 完成）
    // 或者手动注册
    // ctx.register_http(Box::new(hello_route));
    
    println!("Registered {} routes", ctx.counts().routes);
    
    // 构建路由器
    let router = sdforge::http::build_router(Arc::new(ctx));
    
    // 启动服务器
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
```

---

### 4. 速率限制算法改进（滑动窗口）

**文件**: `src/security/rate_limiter.rs`

#### 实现滑动窗口计数器

```rust
// src/security/rate_limiter.rs - 修改 WindowState

/// Window state for sliding window rate limiting
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindowState {
    /// Current count in this window
    pub count: u64,
    /// Window start time in seconds
    pub window_start_secs: u64,
    /// Previous window's count (for sliding window calculation)
    pub previous_count: u64,
}

// 修改 check 方法
pub fn check(&self, key: &str) -> Result<u32, RateLimitError> {
    let window_secs = self.config.window.as_secs();
    let store_key = CacheNamespace::RateLimit.key(key);
    
    let current_time_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    
    let mut state = self
        .requests
        .get(&store_key)
        .and_then(|d| deserialize_window_state(&d))
        .unwrap_or_default();
    
    let current_window = current_time_secs / window_secs;
    let stored_window = if state.window_start_secs > 0 {
        state.window_start_secs / window_secs
    } else {
        0
    };
    
    if current_window != stored_window || state.window_start_secs == 0 {
        // 新窗口
        if current_window == stored_window + 1 {
            // 相邻窗口，保存当前计数为上一个窗口
            state.previous_count = state.count;
        } else {
            // 跳跃了多个窗口，重置
            state.previous_count = 0;
        }
        
        state.window_start_secs = current_time_secs;
        state.count = 1;
    } else {
        // 同一窗口，增加计数
        state.count += 1;
    }
    
    // 计算加权计数（滑动窗口）
    let elapsed_in_window = (current_time_secs % window_secs) as f64;
    let weight = 1.0 - (elapsed_in_window / window_secs as f64);
    let weighted_count = (state.previous_count as f64 * weight) + state.count as f64;
    
    // 保存到缓存
    let serialized = serialize_window_state(&state);
    let ttl = Duration::from_secs(window_secs * 2);
    self.requests.set(&store_key, serialized, Some(ttl));
    
    // 检查是否超过限制
    if weighted_count as u32 >= self.config.max_requests {
        let retry_after = window_secs - (current_time_secs % window_secs);
        return Err(RateLimitError {
            limit: self.config.max_requests,
            remaining: 0,
            retry_after,
        });
    }
    
    Ok((self.config.max_requests as f64 - weighted_count) as u32)
}
```

---

### 5. 统一错误处理

**文件**: `src/core/error/mod.rs`

#### 创建统一错误类型

```rust
// src/core/error/mod.rs - 添加新的统一错误类型

use thiserror::Error;

/// Unified error type for all SDForge operations
#[derive(Debug, Error)]
pub enum SdForgeError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Cache error: {0}")]
    Cache(String),
    
    #[error("Rate limit error: {0}")]
    RateLimit(#[from] RateLimitError),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Unified result type
pub type Result<T> = std::result::Result<T, SdForgeError>;

// 实现 From 转换
impl From<serde_json::Error> for SdForgeError {
    fn from(err: serde_json::Error) -> Self {
        SdForgeError::Internal(format!("JSON serialization error: {}", err))
    }
}

impl From<bincode::Error> for SdForgeError {
    fn from(err: bincode::Error) -> Self {
        SdForgeError::Cache(format!("Bincode serialization error: {}", err))
    }
}
```

#### 在各模块中使用统一错误

```rust
// src/security/api_key.rs - 修改返回类型

use crate::core::error::{SdForgeError, Result};

impl AppApiKeyAuth {
    pub fn validate_key(&self, key: &str, client_ip: &str) -> Result<Vec<String>> {
        // ... 实现
        Ok(permissions)
    }
}
```

---

## 🟢 Medium 优先级优化

### 6. 消除 Builder 模式重复代码

**文件**: 新建 `src/macros.rs` 或在现有工具模块中添加

#### 创建 Builder 宏

```rust
// src/macros.rs

/// Macro to implement Builder pattern for a struct
/// 
/// # Example
/// ```ignore
/// builder_pattern! {
///     pub struct MyStructBuilder {
///         field1: String,
///         field2: Option<u32>,
///         field3: Vec<String>,
///     }
///     target: MyStruct,
/// }
/// ```
#[macro_export]
macro_rules! builder_pattern {
    (
        $(#[$meta:meta])*
        $vis:vis struct $builder_name:ident {
            $($field_vis:vis $field_name:ident: $field_type:ty),* $(,)?
        }
        target: $target_name:ident,
        $(defaults: {
            $($default_field:ident: $default_value:expr),* $(,)?
        })?
    ) => {
        $(#[$meta])*
        $vis struct $builder_name {
            $(pub $field_name: $field_type),*
        }
        
        impl $builder_name {
            /// Create a new builder with default values
            pub fn new() -> Self {
                Self {
                    $($field_name: <$field_type as ::std::default::Default>::default()),*
                    $(
                        $($default_field: $default_value),*
                    )?
                }
            }
            
            /// Set field value and return self for method chaining
            $(
                pub fn $field_name(mut self, value: $field_type) -> Self {
                    self.$field_name = value;
                    self
                }
            )*
        }
        
        impl ::std::default::Default for $builder_name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

// 使用示例
#[cfg(test)]
mod tests {
    use super::*;
    
    builder_pattern! {
        pub struct TestBuilder {
            name: Option<String>,
            age: Option<u32>,
        }
        target: TestStruct,
    }
    
    #[test]
    fn test_builder_pattern() {
        let builder = TestBuilder::new()
            .name(Some("Alice".to_string()))
            .age(Some(30));
        
        assert_eq!(builder.name, Some("Alice".to_string()));
        assert_eq!(builder.age, Some(30));
    }
}
```

---

### 7. 性能优化 - Regex 缓存改进

**文件**: `src/core/validation.rs`

```rust
// 修改前
static REGEX_CACHE: Lazy<DashMap<String, regex::Regex>> = Lazy::new(DashMap::new);

// 修改后 - 使用 Arc 避免克隆
static REGEX_CACHE: Lazy<DashMap<String, Arc<regex::Regex>>> = Lazy::new(|| {
    DashMap::with_capacity(32)
});

pub fn validate_regex(value: &str, pattern: &str) -> Result<(), ValidationError> {
    // 尝试直接从缓存获取引用
    if let Some(cached) = REGEX_CACHE.get(pattern) {
        if cached.is_match(value) {
            return Ok(());
        } else {
            return Err(ValidationError::new("regex"));
        }
    }
    
    // 缓存未命中，创建新实例
    let new_regex = regex::Regex::new(pattern)
        .map_err(|_| ValidationError::new("regex"))?;
    
    let new_regex = Arc::new(new_regex);
    
    // 插入缓存（可能有竞争，但可接受）
    REGEX_CACHE.insert(pattern.to_string(), Arc::clone(&new_regex));
    
    if new_regex.is_match(value) {
        Ok(())
    } else {
        Err(ValidationError::new("regex"))
    }
}
```

---

### 8. 字符串常量枚举化

#### HTTP 方法枚举

创建新文件 `src/core/types/http_method.rs`:

```rust
use serde::{Deserialize, Serialize};

/// HTTP methods as a type-safe enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Connect,
    Trace,
}

impl HttpMethod {
    /// Parse HTTP method from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "get" => Some(Self::Get),
            "post" => Some(Self::Post),
            "put" => Some(Self::Put),
            "delete" => Some(Self::Delete),
            "patch" => Some(Self::Patch),
            "head" => Some(Self::Head),
            "options" => Some(Self::Options),
            "connect" => Some(Self::Connect),
            "trace" => Some(Self::Trace),
            _ => None,
        }
    }
    
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Connect => "CONNECT",
            HttpMethod::Trace => "TRACE",
        }
    }
    
    /// Check if this method typically has a request body
    pub fn has_body(&self) -> bool {
        matches!(self, Self::Post | Self::Put | Self::Patch)
    }
    
    /// Check if this method is idempotent
    pub fn is_idempotent(&self) -> bool {
        matches!(self, Self::Get | Self::Put | Self::Delete | Self::Head | Self::Options)
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for HttpMethod {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        HttpMethod::from_str(s)
            .ok_or_else(|| format!("Invalid HTTP method: {}", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_http_method_from_str() {
        assert_eq!(HttpMethod::from_str("GET"), Some(HttpMethod::Get));
        assert_eq!(HttpMethod::from_str("get"), Some(HttpMethod::Get));
        assert_eq!(HttpMethod::from_str("Post"), Some(HttpMethod::Post));
        assert_eq!(HttpMethod::from_str("INVALID"), None);
    }
    
    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
    }
    
    #[test]
    fn test_has_body() {
        assert!(HttpMethod::Post.has_body());
        assert!(!HttpMethod::Get.has_body());
    }
    
    #[test]
    fn test_is_idempotent() {
        assert!(HttpMethod::Get.is_idempotent());
        assert!(!HttpMethod::Post.is_idempotent());
    }
}
```

然后在 macros 中使用枚举：

```rust
// macros/src/lib.rs - 修改 HTTP 方法匹配

use crate::core::types::HttpMethod;

// 在生成代码时使用枚举
let http_method = HttpMethod::from_str(&method_lower)
    .unwrap_or(HttpMethod::Get);

quote! {
    match #http_method {
        HttpMethod::Get => router = router.get(#handler_closure),
        HttpMethod::Post => router = router.post(#handler_closure),
        // ...
    }
}
```

---

## 测试策略

### 单元测试

为每个修改添加单元测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_feature_x() {
        // 测试新功能
    }
    
    #[test]
    fn test_backward_compatibility() {
        // 确保向后兼容
    }
}
```

### 集成测试

添加集成测试确保修改不影响整体功能：

```rust
// tests/integration/optimization_tests.rs

#[test]
fn test_context_based_registration() {
    let ctx = SdForgeContext::new();
    // 测试上下文功能
}

#[test]
fn test_sliding_window_rate_limiting() {
    // 测试滑动窗口限流
}
```

---

## 迁移检查清单

- [ ] 修复硬编码路径
- [ ] 添加 JWT 密钥验证
- [ ] 创建 SdForgeContext
- [ ] 更新 HTTP 模块使用上下文
- [ ] 实现滑动窗口限流
- [ ] 统一错误类型
- [ ] 消除重复代码
- [ ] 性能优化
- [ ] 字符串枚举化
- [ ] 更新文档
- [ ] 运行所有测试
- [ ] 更新示例代码
- [ ] 发布新版本

---

## 回滚计划

如果某个修改导致问题，按以下步骤回滚：

1. **Git 回滚**: `git revert <commit-hash>`
2. **特性禁用**: 在 Cargo.toml 中禁用相关特性
3. **条件编译**: 使用 `#[cfg(feature = "old-behavior")]` 保留旧实现

---

**文档结束**

*本指南应与设计文档和测试计划配合使用。*
