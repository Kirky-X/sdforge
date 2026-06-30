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
// 键规范化函数
// =============================================================================

/// 规范化缓存键，确保一致的格式
///
/// # Arguments
/// * `key` - 原始键
///
/// # Returns
/// 规范化后的键字符串
///
/// # Examples
/// ```
/// use sdforge::cache::canonicalize_cache_key;
///
/// let normalized = canonicalize_cache_key("  user:123  ");
/// assert_eq!(normalized, "user:123");
/// ```
pub fn canonicalize_cache_key(key: &str) -> String {
    key.trim().to_lowercase()
}

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

    /// 根据模式删除匹配的键
    ///
    /// # Arguments
    /// * `pattern` - 匹配模式（支持通配符 * 和前缀匹配）
    ///
    /// # Returns
    /// 被删除的键的数量
    ///
    /// # Examples
    /// ```ignore
    /// cache.invalidate("user:*"); // 删除所有 user: 开头的键
    /// cache.invalidate("*session*"); // 删除包含 session 的键
    /// ```
    fn invalidate(&self, pattern: &str) -> usize {
        let keys = self.find_keys_by_pattern(pattern);
        self.delete_many(&keys.iter().map(|s| s.as_str()).collect::<Vec<_>>())
    }

    /// 根据模式查找匹配的键（不删除）
    ///
    /// # Arguments
    /// * `pattern` - 匹配模式
    ///
    /// # Returns
    /// 匹配的键列表
    fn find_keys_by_pattern(&self, pattern: &str) -> Vec<String>;

    /// 获取缓存统计信息
    ///
    /// # Returns
    /// 包含命中数、未命中数、命中率等的 HashMap
    fn get_stats(&self) -> HashMap<String, u64> {
        HashMap::new()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::DashMapCache;

    #[test]
    fn test_canonicalize_cache_key_trims_whitespace() {
        assert_eq!(canonicalize_cache_key("  user:123  "), "user:123");
        assert_eq!(canonicalize_cache_key("key"), "key");
        assert_eq!(canonicalize_cache_key("\tkey\n"), "key");
    }

    #[test]
    fn test_canonicalize_cache_key_lowercase() {
        assert_eq!(canonicalize_cache_key("USER:123"), "user:123");
        assert_eq!(canonicalize_cache_key("MixedCase"), "mixedcase");
        assert_eq!(canonicalize_cache_key("USER:ABC"), "user:abc");
    }

    #[test]
    fn test_canonicalize_cache_key_combined() {
        assert_eq!(canonicalize_cache_key("  USER:123  "), "user:123");
        assert_eq!(canonicalize_cache_key("\tMIXED_CASE\n"), "mixed_case");
    }

    #[test]
    fn test_synccache_trait_get_set() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        cache.set("test_key", b"test_value".to_vec());
        assert_eq!(cache.get("test_key"), Some(b"test_value".to_vec()));
        assert!(cache.contains("test_key"));
    }

    #[test]
    fn test_synccache_trait_delete() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        cache.set("key1", b"value1".to_vec());
        assert!(cache.delete("key1"));
        assert!(!cache.contains("key1"));
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_synccache_trait_clear() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        cache.set("key1", b"v1".to_vec());
        cache.set("key2", b"v2".to_vec());
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_synccache_trait_len_and_is_empty() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        cache.set("key1", b"v1".to_vec());
        cache.set("key2", b"v2".to_vec());
        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_synccache_trait_get_many() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        cache.set("key1", b"v1".to_vec());
        cache.set("key2", b"v2".to_vec());
        cache.set("key3", b"v3".to_vec());

        let results = cache.get_many(&["key1", "key2", "nonexistent"]);
        assert_eq!(results.len(), 2);
        assert_eq!(results.get("key1"), Some(&b"v1".to_vec()));
    }

    #[test]
    fn test_synccache_trait_set_many() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        let items = vec![
            ("k1".to_string(), b"v1".to_vec()),
            ("k2".to_string(), b"v2".to_vec()),
        ];
        cache.set_many(&items);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get("k1"), Some(b"v1".to_vec()));
        assert_eq!(cache.get("k2"), Some(b"v2".to_vec()));
    }

    #[test]
    fn test_synccache_trait_delete_many() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());
        cache.set("k3", b"v3".to_vec());

        let deleted = cache.delete_many(&["k1", "k3", "nonexistent"]);
        assert_eq!(deleted, 2);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_synccache_trait_invalidate() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        cache.set("user:1", b"v1".to_vec());
        cache.set("user:2", b"v2".to_vec());
        cache.set("session:1", b"s1".to_vec());

        let deleted = cache.invalidate("user:*");
        assert_eq!(deleted, 2);
        assert_eq!(cache.len(), 1);
        assert!(cache.contains("session:1"));
    }

    #[test]
    fn test_synccache_trait_find_keys_by_pattern() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        cache.set("user:1", b"v1".to_vec());
        cache.set("user:2", b"v2".to_vec());
        cache.set("admin:1", b"a1".to_vec());

        let keys = cache.find_keys_by_pattern("user:*");
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"user:1".to_string()));
        assert!(keys.contains(&"user:2".to_string()));
    }

    #[test]
    fn test_synccache_trait_get_stats() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());

        let stats = cache.get_stats();
        assert!(stats.contains_key("total_keys"));
    }

    #[test]
    fn test_shared_cache_type_alias() {
        let cache: SharedCache = Arc::new(DashMapCache::new());
        cache.set("key", b"value".to_vec());
        assert!(cache.contains("key"));
    }
}
