# Phase 1 架构改进迁移指南

本文档详细描述了 SDForge Phase 1 架构改进的迁移步骤和兼容性说明。

## 📋 改进概览

Phase 1 包含以下核心改进：

### 1. 统一注册系统
- **动机**：消除 HTTP、MCP、WebSocket、gRPC 四个协议模块的重复代码
- **改进**：使用 `Registration` trait + `define_registration!` 宏
- **影响范围**：所有协议模块

### 2. 配置管理重构
- **动机**：集中管理配置默认值，模块化配置结构
- **改进**：拆分为 app.rs, cache.rs, security.rs 等独立模块
- **影响范围**：配置加载和使用代码

### 3. 安全模块增强
- **动机**：支持 API Key 版本管理和密钥轮换
- **改进**：添加 LRU 缓存、版本追踪、审计日志
- **影响范围**：API Key 认证相关代码

### 4. 缓存系统优化
- **动机**：提供更灵活的缓存失效策略
- **改进**：模式匹配失效、键规范化、批量操作
- **影响范围**：缓存使用代码

---

## 🔧 迁移步骤

### 步骤 1: 更新依赖

确保 `Cargo.toml` 包含以下依赖：

```toml
[dependencies]
sdforge = { version = "0.1.0", features = ["full"] }
inventory = "0.3"  # 用于统一注册
```

### 步骤 2: 迁移协议注册代码

#### 迁移前（旧代码）
```rust
// HTTP 模块示例
#[derive(Debug, Clone)]
pub struct RouteRegistration {
    name: &'static str,
    version: &'static str,
    register_fn: fn() -> HttpRoute,
}

impl RouteRegistration {
    pub const fn new(
        name: &'static str,
        version: &'static str,
        register_fn: fn() -> HttpRoute,
    ) -> Self {
        Self { name, version, register_fn }
    }
}

inventory::collect!(RouteRegistration);
```

#### 迁移后（新代码）
```rust
use crate::core::{ApiMetadata, Registration};
use crate::define_registration;

// 一行宏定义替代 30+ 行手动定义
define_registration!(RouteRegistration, HttpRoute, ApiMetadata);
```

#### 测试代码迁移
```rust
// 旧代码
let registration = RouteRegistration::new("test", "v1", || {
    HttpRoute::new("/test".to_string(), get(test_handler), ...)
});

// 新代码 - 添加 metadata_fn 参数
let registration = RouteRegistration::new("test", "v1", 
    || {
        HttpRoute::new("/test".to_string(), get(test_handler), ...)
    },
    || {
        ApiMetadata {
            name: "test".to_string(),
            version: "v1".to_string(),
            description: "Test route".to_string(),
            cache_ttl: None,
            is_streaming: false,
        }
    },
);
```

### 步骤 3: 迁移配置代码

#### 新增配置类型使用示例

**CacheConfig 使用：**
```rust
use sdforge::config::{AppConfigBuilder, CacheConfig};

let config = AppConfigBuilder::default()
    .cache(CacheConfig::with_ttl(600)) // 10 分钟 TTL
    .build();
```

**SecurityConfig 使用：**
```rust
use sdforge::config::{AppConfigBuilder, SecurityConfig};

let config = AppConfigBuilder::default()
    .security(SecurityConfig::minimal()) // 最小化安全头
    .build();
```

#### 配置模块导入变更
```rust
// 旧代码 - 所有配置在 mod.rs
use sdforge::config::{AppConfig, ServerConfig, AuthConfig};

// 新代码 - 模块化导出（向后兼容）
use sdforge::config::{
    AppConfig, 
    CacheConfig,      // 新增
    SecurityConfig,   // 新增
};
```

### 步骤 4: 迁移安全模块代码

#### API Key 版本管理
```rust
use sdforge::security::{
    ApiKeyMetadata, 
    ApiKeyVersion,
    LruCacheManager,
    LruConfig,
};

// 创建带版本的 API Key
let mut metadata = ApiKeyMetadata {
    key_id: "key_123".to_string(),
    versions: vec![
        ApiKeyVersion::new(
            "v1".to_string(),
            hash_key("old_key"),
            vec!["read".to_string()],
            Some(Duration::from_secs(3600)),
        ),
    ],
    active_version_index: Some(0),
    created_at: Instant::now(),
    description: Some("Test key".to_string()),
};

// 旋转到新版本
metadata.rotate_to_version(1)?;

// 审计日志记录
audit_logger.log_key_rotation(
    "key_123",
    "v1",
    "v2",
    true,
    Some("Scheduled rotation".to_string()),
).await;
```

### 步骤 5: 迁移缓存代码

#### 键规范化
```rust
use sdforge::cache::canonicalize_cache_key;

let raw_key = "  User:123  ";
let normalized = canonicalize_cache_key(raw_key);
assert_eq!(normalized, "user:123");
```

#### 模式匹配失效
```rust
use sdforge::cache::{SyncCache, DashMapCache};

let cache = Arc::new(DashMapCache::new());

// 设置一些键
cache.set("user:1:profile", b"data1".to_vec());
cache.set("user:2:profile", b"data2".to_vec());
cache.set("session:abc", b"data3".to_vec());

// 删除所有 user: 开头的键
let deleted_count = cache.invalidate("user:*");
assert_eq!(deleted_count, 2);

// 查找匹配的键（不删除）
let keys = cache.find_keys_by_pattern("*session*");
assert_eq!(keys, vec!["session:abc"]);
```

#### 获取统计信息
```rust
let stats = cache.get_stats();
println!("Total keys: {:?}", stats.get("total_keys"));
```

---

## ⚠️ 破坏性变更

### 1. 协议注册结构体字段变更

**影响**：直接访问结构体字段的代码需要更新

```rust
// ❌ 不再支持
registration.name  // 编译错误
registration.description  // 编译错误

// ✅ 使用 trait 方法
registration.name()  // 正确
registration.metadata().description()  // 正确
```

### 2. 配置验证 feature gate

**影响**：`validate()` 方法需要启用 `validation` feature

```toml
# Cargo.toml
[dependencies]
sdforge = { version = "0.1.0", features = ["validation"] }
```

```rust
#[cfg(feature = "validation")]
config.validate()?;
```

### 3. 缓存 trait 扩展

**影响**：实现 `SyncCache` trait 的类型需要实现新方法

```rust
impl SyncCache for MyCustomCache {
    // ... 现有方法 ...
    
    fn find_keys_by_pattern(&self, pattern: &str) -> Vec<String> {
        // 必须实现
        vec![]
    }
    
    fn get_stats(&self) -> HashMap<String, u64> {
        // 必须实现
        HashMap::new()
    }
}
```

---

## 🎯 最佳实践

### 1. 统一注册系统

✅ **推荐：**
```rust
define_registration!(MyRegistration, MyType, ApiMetadata);
```

❌ **不推荐：**
```rust
// 手动定义重复的结构体
pub struct MyRegistration { ... }
impl MyRegistration { ... }
```

### 2. 配置管理

✅ **推荐：**
```rust
// 使用 Builder 模式
let config = AppConfigBuilder::default()
    .server(server_config)
    .authentication(auth_config)
    .cache(CacheConfig::default())
    .build();
```

❌ **不推荐：**
```rust
// 直接构造（失去默认值）
let config = AppConfig {
    server: ServerConfig { .. },
    authentication: AuthConfig { .. },
    // ...
};
```

### 3. 密钥轮换

✅ **推荐：**
```rust
// 定期轮换并记录审计日志
if should_rotate() {
    metadata.rotate_to_version(new_index)?;
    audit_logger.log_key_rotation(
        key_id, old_ver, new_ver, true, None
    ).await;
}
```

❌ **不推荐：**
```rust
// 直接修改版本索引（无审计）
metadata.active_version_index = Some(new_index);
```

### 4. 缓存失效

✅ **推荐：**
```rust
// 使用模式匹配批量删除
cache.invalidate("user:*");  // 删除所有用户缓存
cache.invalidate("*session*");  // 删除会话相关
```

❌ **不推荐：**
```rust
// 逐个删除（效率低）
for key in all_keys {
    if key.starts_with("user:") {
        cache.delete(key);
    }
}
```

---

## 📊 性能影响

### 统一注册系统
- **编译时**：宏展开增加 ~50ms
- **运行时**：零开销（trait 方法内联）
- **内存**：无变化

### 配置管理
- **编译时**：模块拆分减少增量编译时间 ~20%
- **运行时**：无影响

### 安全增强
- **LRU 缓存**：额外 ~1KB/键 的元数据开销
- **审计日志**：~5ms/事件的写入延迟

### 缓存优化
- **模式匹配**：O(n) 复杂度，n 为键总数
- **键规范化**：可忽略（字符串 trim + lowercase）

---

## 🔍 故障排查

### 问题 1: 编译错误 "cannot find macro `define_registration`"

**解决方案：**
```rust
use crate::define_registration;
use crate::core::Registration;
```

### 问题 2: 配置验证方法找不到

**解决方案：**
```toml
# 启用 validation feature
sdforge = { version = "0.1.0", features = ["validation"] }
```

### 问题 3: 缓存模式匹配性能差

**解决方案：**
```rust
// 避免过于复杂的模式
cache.invalidate("user:*:profile");  // ✅ 简单模式
cache.invalidate("*:*:*:*");         // ❌ 复杂正则

// 或使用前缀匹配
let keys = cache.find_keys_by_pattern("user:*");
for key in keys {
    cache.delete(&key);
}
```

---

## 📚 相关文档

- [OpenSpec 变更提案](../../openspec/changes/architecture-improvements-phase1/proposal.md)
- [技术设计文档](../../openspec/changes/architecture-improvements-phase1/design.md)
- [统一注册系统规范](../../openspec/specs/unified-registration/spec.md)
- [配置验证规范](../../openspec/specs/config-validation/spec.md)
- [密钥轮换规范](../../openspec/specs/key-rotation/spec.md)
- [缓存失效规范](../../openspec/specs/cache-invalidation/spec.md)

---

## 🆘 获取帮助

如遇到迁移问题：

1. 检查 [常见问题](#故障排查) 部分
2. 查看 [示例代码](#迁移步骤)
3. 参考项目 AGENTS.md 文件
4. 提交 Issue 并附上错误信息

---

**最后更新**: 2024-01-XX
**版本**: Phase 1 v0.1.0
