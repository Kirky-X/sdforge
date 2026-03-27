// Copyright (c) 2026 Kirky.X
//!
//! 缓存模块 — 同步缓存接口与 oxcache 透传
//!
//! 本模块提供：
//! - `SyncCache` trait：同步键值存储接口（用于 security 模块等需要同步操作的场景）
//! - `DashMapCache`：基于 DashMap 的同步内存缓存实现
//! - oxcache 异步缓存的透传（用于需要 TTL、分层缓存等高级功能的场景）
//!
//! # 架构
//!
//! 根据依赖注入架构设计：
//! - oxcache 属于底层组件层（Infrastructure Layer）
//! - sdforge 属于功能组件层（Feature Layer）
//! - 功能组件通过依赖注入使用底层组件
//!
//! # 同步缓存使用示例
//!
//! ```rust,ignore
//! use sdforge::cache::{SyncCache, DashMapCache};
//! use std::sync::Arc;
//!
//! let cache = Arc::new(DashMapCache::new());
//! cache.set("key", b"value".to_vec());
//! assert!(cache.get("key").is_some());
//! ```
//!
//! # 异步缓存使用示例（透传 oxcache）
//!
//! ```rust,ignore
//! use sdforge::cache::Cache;
//!
//! let cache: Cache<String, MyData> = Cache::builder().build().await?;
//! cache.set(&"key".to_string(), &data).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;

// =============================================================================
// 同步缓存 Trait（用于 security 模块等需要同步操作的场景）
// =============================================================================

/// 同步缓存 trait — 功能组件的标准存储接口
///
/// 设计原则：
/// - 所有方法同步，立即返回
/// - 值存储为 `Vec<u8>`，序列化由调用方负责
/// - 不管理 TTL，TTL 由调用方或上层组件处理
pub trait SyncCache: Send + Sync {
    /// 获取值
    ///
    /// # Returns
    /// `Some(bytes)` 如果存在，`None` 如果不存在
    fn get(&self, key: &str) -> Option<Vec<u8>>;

    /// 批量获取多个键的值
    ///
    /// # Performance
    /// 比单独调用 get() 更高效，减少锁竞争和系统调用
    ///
    /// # Arguments
    /// * `keys` - 要获取的键列表
    ///
    /// # Returns
    /// HashMap 包含所有找到的键值对
    fn get_many(&self, keys: &[&str]) -> HashMap<String, Vec<u8>> {
        keys.iter()
            .filter_map(|&key| self.get(key).map(|v| (key.to_string(), v)))
            .collect()
    }

    /// 设置值（无 TTL，由调用方管理生命周期）
    fn set(&self, key: &str, value: Vec<u8>);

    /// 批量设置多个键值对
    ///
    /// # Performance
    /// 比单独调用 set() 更高效，减少锁竞争和系统调用
    ///
    /// # Arguments
    /// * `items` - 键值对切片
    fn set_many(&self, items: &[(String, Vec<u8>)]) {
        for (key, value) in items {
            self.set(key, value.clone());
        }
    }

    /// 删除键
    ///
    /// # Returns
    /// `true` 如果键存在并被删除，`false` 如果不存在
    fn delete(&self, key: &str) -> bool;

    /// 批量删除多个键
    ///
    /// # Performance
    /// 比单独调用 delete() 更高效，减少锁竞争和系统调用
    ///
    /// # Arguments
    /// * `keys` - 要删除的键列表
    ///
    /// # Returns
    /// 被删除的键的数量
    fn delete_many(&self, keys: &[&str]) -> usize {
        keys.iter().filter(|&&key| self.delete(key)).count()
    }

    /// 检查键是否存在
    fn contains(&self, key: &str) -> bool;

    /// 清空所有键
    fn clear(&self);

    /// 获取键的数量
    fn len(&self) -> usize;

    /// 检查是否为空
    fn is_empty(&self) -> bool;
}

/// SyncCache 的 Arc 智能指针别名
pub type SharedCache = Arc<dyn SyncCache>;

// =============================================================================
// DashMap 同步缓存实现
// =============================================================================

mod dashmap;

pub use dashmap::DashMapCache;

// =============================================================================
// oxcache 异步缓存透传
// =============================================================================

// 直接透传 oxcache 库的缓存接口
pub use oxcache::backend::{DashMapMemoryBackend, MemoryBackend, MokaMemoryBackend};
pub use oxcache::cache::Cache;
pub use oxcache::traits::{CacheKey, Cacheable};

// 如果启用了 rate-limiting feature，也导出 oxcache 的限流组件
#[cfg(feature = "cache")]
pub use oxcache::rate_limiting::{ClientRateLimiter, RateLimitConfig, TokenBucket};
