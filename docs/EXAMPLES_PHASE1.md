# SDForge Phase 1 新功能使用示例

本文档提供 Phase 1 新增功能的完整使用示例。

## 📋 目录

1. [统一注册系统](#1-统一注册系统)
2. [配置管理](#2-配置管理)
3. [安全模块](#3-安全模块)
4. [缓存系统](#4-缓存系统)

---

## 1. 统一注册系统

### 1.1 HTTP 路由注册

```rust
use sdforge::core::{ApiMetadata, Registration};
use sdforge::define_registration;
use sdforge::http::HttpRoute;
use axum::routing::get;

// 定义注册类型（一行代码替代 30+ 行）
define_registration!(MyRouteRegistration, HttpRoute, ApiMetadata);

// 在 main.rs 或 lib.rs 中注册
inventory::submit! {
    MyRouteRegistration::new(
        "my_api",           // API 名称
        "v1",               // 版本
        || {
            // 创建路由实例
            HttpRoute::new(
                "/api/users".to_string(),
                get(users_handler),
                vec!["GET".to_string()],
                None,
            )
        },
        || {
            // 元数据
            ApiMetadata {
                name: "users_api".to_string(),
                version: "v1".to_string(),
                description: "User management API".to_string(),
                cache_ttl: Some(300),
                is_streaming: false,
            }
        },
    )
}

// 运行时获取所有注册的路由
let routes = sdforge::http::build_routes().await?;
```

### 1.2 MCP Tool 注册

```rust
use sdforge::core::{ApiMetadata, Registration};
use sdforge::define_registration;
use sdforge::mcp::McpToolRegistration;
use mcp_sdk::tools::Tool;
use std::sync::Arc;

define_registration!(MyToolRegistration, Arc<dyn Tool>, ApiMetadata);

// 注册工具
inventory::submit! {
    MyToolRegistration::new(
        "calculator_tool",
        "v1",
        || {
            Arc::new(CalculatorTool) as Arc<dyn Tool>
        },
        || {
            ApiMetadata {
                name: "calculator".to_string(),
                version: "v1".to_string(),
                description: "Math calculations".to_string(),
                cache_ttl: None,
                is_streaming: false,
            }
        },
    )
}

// 构建 MCP 服务器
let server = sdforge::mcp::build().await;
```

### 1.3 WebSocket 路由注册

```rust
use sdforge::core::{ApiMetadata, Registration};
use sdforge::define_registration;
use sdforge::websocket::{WebSocketHandler, WebSocketRoute};
use std::sync::Arc;

define_registration!(MyWsRegistration, Arc<dyn WebSocketHandler>, ApiMetadata);

pub struct MyChatHandler;

impl WebSocketHandler for MyChatHandler {
    fn handle(&self, message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage> {
        // 处理消息逻辑
        Box::pin(async move {
            WebSocketMessage::Text("Hello".to_string())
        })
    }
}

inventory::submit! {
    MyWsRegistration::new(
        "/ws/chat",
        "v1",
        || Arc::new(MyChatHandler) as Arc<dyn WebSocketHandler>,
        || {
            ApiMetadata {
                name: "chat_websocket".to_string(),
                version: "v1".to_string(),
                description: "Real-time chat".to_string(),
                cache_ttl: None,
                is_streaming: true,
            }
        },
    )
}
```

### 1.4 gRPC 服务注册

```rust
use sdforge::core::{ApiMetadata, Registration};
use sdforge::define_registration;
use sdforge::grpc::{GrpcRoute, GrpcRouteRegistration};

define_registration!(MyGrpcRegistration, GrpcRoute, ApiMetadata);

inventory::submit! {
    MyGrpcRegistration::new(
        "UserService",
        "v1",
        || {
            GrpcRoute::new(
                "UserService".to_string(),
                ApiMetadata {
                    name: "user_service".to_string(),
                    version: "v1".to_string(),
                    description: "User management gRPC service".to_string(),
                    cache_ttl: None,
                    is_streaming: false,
                },
            )
        },
        || {
            ApiMetadata {
                name: "user_service".to_string(),
                version: "v1".to_string(),
                description: "User management gRPC service".to_string(),
                cache_ttl: None,
                is_streaming: false,
            }
        },
    )
}

// 构建 gRPC 服务器
sdforge::grpc::build_server("0.0.0.0:50051").await?;
```

---

## 2. 配置管理

### 2.1 基本配置使用

```rust
use sdforge::config::{AppConfigBuilder, ServerConfig, CacheConfig, SecurityConfig};

// 使用 Builder 模式构建配置
let config = AppConfigBuilder::default()
    .server(ServerConfig {
        host: "0.0.0.0".to_string(),
        port: 8080,
        request_timeout_secs: 30,
        cors: None,
    })
    .cache(CacheConfig {
        enabled: true,
        default_ttl_secs: 600,  // 10 分钟
        max_items: 10_000,
        track_stats: true,
    })
    .security(SecurityConfig {
        enable_headers: true,
        content_type_options: "nosniff".to_string(),
        frame_options: "DENY".to_string(),
        xss_protection: "1; mode=block".to_string(),
        cache_control: "no-store".to_string(),
        content_security_policy: "default-src 'self'".to_string(),
        strict_transport_security: "max-age=31536000".to_string(),
        referrer_policy: "strict-origin".to_string(),
        permissions_policy: "geolocation=()".to_string(),
    })
    .build();

println!("Server will listen on {}:{}", config.server.host, config.server.port);
```

### 2.2 从文件加载配置

```rust
use sdforge::config::{AppConfig, ConfigLoader};

// 从 YAML 文件加载
let config = ConfigLoader::new()
    .add_yaml_file("config.yaml")?
    .load::<AppConfig>()?;

// 从环境变量覆盖
let config = ConfigLoader::new()
    .add_yaml_file("config.yaml")?
    .add_environment_variable("APP_")?
    .load::<AppConfig>()?;
```

### 2.3 配置验证

```rust
#[cfg(feature = "validation")]
{
    use sdforge::config::AppConfig;
    
    let config = AppConfig::default();
    
    match config.validate() {
        Ok(_) => println!("✓ Configuration is valid"),
        Err(e) => eprintln!("✗ Configuration error: {}", e),
    }
}
```

---

## 3. 安全模块

### 3.1 API Key 版本管理

```rust
use sdforge::security::{
    ApiKeyMetadata, 
    ApiKeyVersion,
    LruCacheManager,
    LruConfig,
};
use std::time::{Duration, Instant};

// 创建带版本的 API Key
let mut metadata = ApiKeyMetadata {
    key_id: "key_prod_001".to_string(),
    versions: vec![
        ApiKeyVersion::new(
            "v1".to_string(),
            hash_key("old_secret_key"),
            vec!["read:users".to_string(), "write:posts".to_string()],
            Some(Duration::from_secs(3600)), // 1 小时过期
        ),
        ApiKeyVersion::new(
            "v2".to_string(),
            hash_key("new_secret_key"),
            vec!["read:users".to_string(), "write:posts".to_string(), "admin:reports".to_string()],
            Some(Duration::from_secs(7200)), // 2 小时过期
        ),
    ],
    active_version_index: Some(0),
    created_at: Instant::now(),
    description: Some("Production API Key".to_string()),
};

// 旋转到新版本
metadata.rotate_to_version(1)?;

// 检查是否过期
if let Some(active) = metadata.get_active_version() {
    if active.is_expired() {
        println!("⚠️  Active key is expired!");
    }
}

// 清理旧版本
metadata.cleanup_versions(2); // 保留最近 2 个版本
```

### 3.2 LRU 缓存管理器

```rust
use sdforge::security::{LruCacheManager, LruConfig};
use sdforge::cache::DashMapCache;
use std::sync::Arc;

// 创建 LRU 缓存管理器
let cache = Arc::new(DashMapCache::new());
let lru_config = LruConfig {
    max_entries: 10_000,
    ttl: Duration::from_secs(3600),
    eviction_threshold: 0.8, // 80% 容量时开始淘汰
};

let manager = LruCacheManager::new(cache, lru_config);

// 使用缓存
manager.set("user:123", b"profile_data".to_vec());
if let Some(data) = manager.get("user:123") {
    println!("Got user data: {:?}", data);
}

// 统计信息
println!("Cache size: {}", manager.cache.len());
```

### 3.3 密钥轮换审计日志

```rust
use sdforge::security::{AppAuditLogger, AppAuditLoggerBuilder};
use sdforge::cache::DashMapCache;
use std::sync::Arc;

// 创建审计日志器
let cache = Arc::new(DashMapCache::new());
let audit_logger = AppAuditLoggerBuilder::new()
    .cache(cache)
    .max_logs_per_user(1000)
    .max_concurrent_ops(100)
    .queue_size(1000)
    .build();

// 记录密钥轮换事件
audit_logger.log_key_rotation(
    "key_prod_001",      // 密钥 ID
    "v1",                // 旧版本
    "v2",                // 新版本
    true,                // 成功
    Some("Scheduled rotation".to_string()),
).await;

// 查询审计日志
let logs = audit_logger.get_logs("system");
for log in logs {
    println!(
        "[{}] {} - {} on {}: {:?}",
        log.timestamp,
        log.user_id.unwrap_or_default(),
        log.action,
        log.resource,
        log.result
    );
}
```

### 3.4 Bearer Token 认证

```rust
use sdforge::security::{BearerAuth, BearerAuthBuilder};

// 创建 JWT 验证器
let jwt_validator = BearerAuthBuilder::new()
    .secret("your-secret-key-at-least-32-bytes-long!")
    .expiration_secs(3600)
    .build()?;

// 生成 token
let token = jwt_validator.generate_token("user_123", vec!["admin"])?;

// 验证 token
match jwt_validator.validate(&token) {
    Ok(claims) => {
        println!("✓ Token valid for user: {}", claims.sub);
    }
    Err(e) => {
        eprintln!("✗ Token invalid: {}", e);
    }
}
```

---

## 4. 缓存系统

### 4.1 基本缓存操作

```rust
use sdforge::cache::{SyncCache, DashMapCache};
use std::sync::Arc;

let cache = Arc::new(DashMapCache::new());

// 设置值
cache.set("user:123", b"profile_data".to_vec());

// 获取值
if let Some(data) = cache.get("user:123") {
    println!("Got data: {:?}", data);
}

// 批量设置
cache.set_many(&[
    ("user:456".to_string(), b"data1".to_vec()),
    ("user:789".to_string(), b"data2".to_vec()),
]);

// 批量获取
let users = cache.get_many(&["user:456", "user:789"]);
println!("Found {} users", users.len());

// 删除
if cache.delete("user:123") {
    println!("✓ Deleted");
}

// 批量删除
let deleted = cache.delete_many(&["user:456", "user:789"]);
println!("Deleted {} keys", deleted);
```

### 4.2 键规范化

```rust
use sdforge::cache::canonicalize_cache_key;

// 规范化键名
let raw_keys = vec![
    "  User:123  ",
    "USER:456",
    "user:  789",
];

for raw in raw_keys {
    let normalized = canonicalize_cache_key(raw);
    println!("{} -> {}", raw, normalized);
}
// 输出:
// "  User:123  " -> "user:123"
// "USER:456" -> "user:456"
// "user:  789" -> "user:789"
```

### 4.3 模式匹配失效

```rust
use sdforge::cache::{SyncCache, DashMapCache};
use std::sync::Arc;

let cache = Arc::new(DashMapCache::new());

// 填充测试数据
cache.set("user:1:profile", b"data1".to_vec());
cache.set("user:2:profile", b"data2".to_vec());
cache.set("session:abc123", b"sess1".to_vec());
cache.set("session:def456", b"sess2".to_vec());
cache.set("config:app", b"cfg1".to_vec());

// 查找匹配模式
let user_keys = cache.find_keys_by_pattern("user:*");
println!("User keys: {:?}", user_keys);
// 输出：["user:1:profile", "user:2:profile"]

let session_keys = cache.find_keys_by_pattern("*session*");
println!("Session keys: {:?}", session_keys);
// 输出：["session:abc123", "session:def456"]

// 模式匹配失效
let deleted = cache.invalidate("user:*");
println!("Deleted {} user keys", deleted);
// 输出：Deleted 2 user keys

// 验证剩余键
let remaining = cache.len();
println!("Remaining keys: {}", remaining);
// 输出：Remaining keys: 3
```

### 4.4 缓存统计信息

```rust
use sdforge::cache::{SyncCache, DashMapCache};
use std::sync::Arc;

let cache = Arc::new(DashMapCache::new());

// 添加一些数据
for i in 0..100 {
    cache.set(&format!("key:{}", i), format!("value{}", i).into_bytes());
}

// 获取统计信息
let stats = cache.get_stats();
println!("Total keys: {:?}", stats.get("total_keys"));
println!("Capacity: {:?}", stats.get("capacity"));

// 自定义统计（如果实现）
// 注意：当前 DashMapCache 只返回基本统计
// 可以扩展实现更详细的统计（命中率、淘汰数等）
```

### 4.5 高级缓存使用

```rust
use sdforge::cache::{SyncCache, DashMapCache};
use serde::{Serialize, Deserialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
struct UserProfile {
    id: u64,
    name: String,
    email: String,
}

let cache = Arc::new(DashMapCache::new());

// 序列化存储
let profile = UserProfile {
    id: 123,
    name: "Alice".to_string(),
    email: "alice@example.com".to_string(),
};

let serialized = bincode::serialize(&profile).unwrap();
cache.set("user:123:profile", serialized);

// 反序列化读取
if let Some(data) = cache.get("user:123:profile") {
    let profile: UserProfile = bincode::deserialize(&data).unwrap();
    println!("Loaded profile: {:?}", profile);
}

// 使用规范化键
let key = sdforge::cache::canonicalize_cache_key("  User:123:Profile  ");
cache.set(&key, serialized);
```

---

## 🎯 完整示例：多协议应用

```rust
use sdforge::config::{AppConfigBuilder, CacheConfig, SecurityConfig};
use sdforge::security::{AppAuditLogger, BearerAuth};
use sdforge::cache::DashMapCache;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化配置
    let config = AppConfigBuilder::default()
        .cache(CacheConfig::with_ttl(600))
        .security(SecurityConfig::minimal())
        .build();

    // 2. 初始化缓存
    let cache = Arc::new(DashMapCache::new());

    // 3. 初始化审计日志
    let audit_logger = AppAuditLoggerBuilder::new()
        .cache(cache.clone())
        .build();

    // 4. 初始化 JWT 认证
    let jwt_auth = BearerAuthBuilder::new()
        .secret("your-secure-secret-key-here")
        .build()?;

    // 5. 启动 HTTP 服务器（自动加载注册的路由）
    let http_addr = format!("{}:{}", config.server.host, config.server.port);
    println!("🚀 Starting HTTP server on {}", http_addr);
    
    // 6. 启动其他协议服务器（根据需要）
    // let _mcp_server = sdforge::mcp::build().await;
    // let _ws_router = sdforge::websocket::build_router(cache.clone());
    // sdforge::grpc::build_server("0.0.0.0:50051").await?;

    Ok(())
}
```

---

## 📚 相关资源

- [迁移指南](./MIGRATION_GUIDE_PHASE1.md)
- [OpenSpec 变更提案](../openspec/changes/architecture-improvements-phase1/proposal.md)
- [API 文档](https://docs.rs/sdforge)

---

**最后更新**: 2024-01-XX  
**版本**: Phase 1 v0.1.0
