// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 缓存模块 — 同步缓存接口与 oxcache 透传
//!
//! 本模块提供：
//! - `SyncCache` trait：同步键值存储接口（用于 security 模块等需要同步操作的场景）
//! - `OxcacheSyncCache`：基于 oxcache `DashMapMemoryBackend` 的同步内存缓存实现
//! - `DashMapCache`：`OxcacheSyncCache` 的类型别名（保持调用方兼容）
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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;

mod cache_impl;
pub use cache_impl::canonicalize_cache_key;

// Re-export matches_pattern for test access.
#[cfg(test)]
pub(crate) use cache_impl::matches_pattern;

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
// OxcacheSyncCache — 基于 oxcache DashMapMemoryBackend 的同步缓存实现
// =============================================================================

/// 基于 oxcache `DashMapMemoryBackend` 的同步缓存实现。
///
/// 包装 oxcache 0.3.2 的 `DashMapMemoryBackend`（实现了 `SyncCacheBackend`），
/// 通过 `SyncCacheReader`/`SyncCacheWriter` trait 方法提供同步 get/set/delete 操作。
///
/// 由于 oxcache 0.3.2 的公开 sync API 不暴露键枚举，`find_keys_by_pattern`
/// 通过维护一个并行的键索引（`Mutex<HashSet<String>>`）实现。
///
/// # Example
///
/// ```rust,ignore
/// use sdforge::cache::{SyncCache, OxcacheSyncCache};
///
/// let cache = OxcacheSyncCache::new();
/// cache.set("key", b"value".to_vec());
/// assert_eq!(cache.get("key"), Some(b"value".to_vec()));
/// ```
pub struct OxcacheSyncCache {
    /// oxcache 后端（实现了 SyncCacheBackend）
    backend: oxcache::backend::DashMapMemoryBackend,
    /// 键索引：oxcache 0.3.2 sync API 不暴露键枚举，需并行维护以支持 find_keys_by_pattern
    key_index: Mutex<HashSet<String>>,
}

/// DashMapCache 类型别名 — 保持调用方兼容
///
/// 底层实现已切换为 `OxcacheSyncCache`（基于 oxcache `DashMapMemoryBackend`）。
/// 历史代码中的 `DashMapCache::new()` / `DashMapCache::with_capacity(n)` 调用
/// 自动指向新的 oxcache 实现，无需修改调用方。
pub type DashMapCache = OxcacheSyncCache;

// =============================================================================
// oxcache 异步缓存透传
// =============================================================================

// 直接透传 oxcache 库的异步缓存接口（用于需要 TTL、分层缓存等高级功能的场景）
pub use oxcache::cache::Cache;
pub use oxcache::traits::CacheKey;

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

    // ========================================================================
    // matches_pattern 单元测试
    // ========================================================================

    #[test]
    fn test_matches_pattern_exact() {
        assert!(matches_pattern("user:1", "user:1"));
        assert!(!matches_pattern("user:1", "user:2"));
    }

    #[test]
    fn test_matches_pattern_prefix_wildcard() {
        assert!(matches_pattern("user:*", "user:1"));
        assert!(matches_pattern("user:*", "user:2"));
        assert!(matches_pattern("user:*", "user:"));
        assert!(!matches_pattern("user:*", "session:1"));
    }

    #[test]
    fn test_matches_pattern_suffix_wildcard() {
        assert!(matches_pattern("*:1", "user:1"));
        assert!(matches_pattern("*:1", "admin:1"));
        assert!(!matches_pattern("*:1", "user:2"));
    }

    #[test]
    fn test_matches_pattern_middle_wildcard() {
        assert!(matches_pattern("*session*", "my_session_data"));
        assert!(matches_pattern("*session*", "session"));
        assert!(!matches_pattern("*session*", "user:1"));
    }

    #[test]
    fn test_matches_pattern_single_char_wildcard() {
        assert!(matches_pattern("user:?", "user:1"));
        assert!(matches_pattern("user:?", "user:a"));
        assert!(!matches_pattern("user:?", "user:12"));
    }

    #[test]
    fn test_matches_pattern_combined_wildcards() {
        assert!(matches_pattern("?ser:*", "user:1"));
        assert!(matches_pattern("?ser:*", "aser:xyz"));
        assert!(!matches_pattern("?ser:*", "xser1:"));
    }

    #[test]
    fn test_matches_pattern_empty_pattern_and_text() {
        assert!(matches_pattern("", ""));
        assert!(matches_pattern("*", ""));
        assert!(!matches_pattern("", "a"));
        assert!(matches_pattern("*", "anything"));
    }

    // ========================================================================
    // SyncCache trait 测试（通过 DashMapCache 别名指向 OxcacheSyncCache）
    // ========================================================================

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
    fn test_synccache_trait_find_keys_by_pattern_suffix() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        cache.set("user:1", b"v1".to_vec());
        cache.set("admin:1", b"a1".to_vec());
        cache.set("user:2", b"v2".to_vec());

        let keys = cache.find_keys_by_pattern("*:1");
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"user:1".to_string()));
        assert!(keys.contains(&"admin:1".to_string()));
    }

    #[test]
    fn test_synccache_trait_find_keys_by_pattern_no_match() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        cache.set("user:1", b"v1".to_vec());

        let keys = cache.find_keys_by_pattern("session:*");
        assert!(keys.is_empty());
    }

    #[test]
    fn test_synccache_trait_get_stats() {
        let cache: Box<dyn SyncCache> = Box::new(DashMapCache::new());
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());

        let stats = cache.get_stats();
        assert!(stats.contains_key("total_keys"));
        assert_eq!(stats.get("total_keys"), Some(&2));
        assert!(stats.contains_key("capacity"));
    }

    #[test]
    fn test_shared_cache_type_alias() {
        let cache: SharedCache = Arc::new(DashMapCache::new());
        cache.set("key", b"value".to_vec());
        assert!(cache.contains("key"));
    }

    // ============================================================================
    // OxcacheSyncCache 专属测试
    // ============================================================================

    #[test]
    fn test_oxcache_sync_cache_with_capacity() {
        let cache = OxcacheSyncCache::with_capacity(100);
        cache.set("k1", b"v1".to_vec());
        assert_eq!(cache.get("k1"), Some(b"v1".to_vec()));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_oxcache_sync_cache_default() {
        let cache = OxcacheSyncCache::default();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_oxcache_sync_cache_inner() {
        let cache = OxcacheSyncCache::new();
        let _backend: &oxcache::backend::DashMapMemoryBackend = cache.inner();
    }

    #[test]
    fn test_oxcache_sync_cache_debug() {
        let cache = OxcacheSyncCache::new();
        let debug_str = format!("{:?}", cache);
        assert!(debug_str.contains("OxcacheSyncCache"));
    }

    #[test]
    fn test_oxcache_sync_cache_delete_nonexistent_returns_false() {
        let cache = OxcacheSyncCache::new();
        assert!(!cache.delete("nonexistent"));
    }

    // ============================================================================
    // Default trait method coverage tests
    //
    // OxcacheSyncCache overrides get_many/set_many/delete_many/get_stats, so the
    // default trait method implementations in SyncCache are not exercised by
    // the tests above. The MinimalCache below deliberately only implements
    // the required methods, leaving the default implementations in place so
    // they get coverage.
    // ============================================================================

    /// Minimal SyncCache implementation that relies on the default trait
    /// method implementations for batch operations and stats.
    struct MinimalCache {
        data: std::sync::Mutex<HashMap<String, Vec<u8>>>,
    }

    impl MinimalCache {
        fn new() -> Self {
            Self {
                data: std::sync::Mutex::new(HashMap::new()),
            }
        }
    }

    impl SyncCache for MinimalCache {
        fn get(&self, key: &str) -> Option<Vec<u8>> {
            self.data.lock().unwrap().get(key).cloned()
        }

        fn set(&self, key: &str, value: Vec<u8>) {
            self.data.lock().unwrap().insert(key.to_string(), value);
        }

        fn delete(&self, key: &str) -> bool {
            self.data.lock().unwrap().remove(key).is_some()
        }

        fn contains(&self, key: &str) -> bool {
            self.data.lock().unwrap().contains_key(key)
        }

        fn clear(&self) {
            self.data.lock().unwrap().clear();
        }

        fn len(&self) -> usize {
            self.data.lock().unwrap().len()
        }

        fn is_empty(&self) -> bool {
            self.len() == 0
        }

        fn find_keys_by_pattern(&self, pattern: &str) -> Vec<String> {
            self.data
                .lock()
                .unwrap()
                .keys()
                .filter(|k| matches_pattern(pattern, k))
                .cloned()
                .collect()
        }
    }

    #[test]
    fn test_default_get_many_implementation() {
        let cache = MinimalCache::new();
        cache.set("key1", b"v1".to_vec());
        cache.set("key2", b"v2".to_vec());

        let results = cache.get_many(&["key1", "key2", "missing"]);
        assert_eq!(results.len(), 2);
        assert_eq!(results.get("key1"), Some(&b"v1".to_vec()));
        assert_eq!(results.get("key2"), Some(&b"v2".to_vec()));
        assert!(!results.contains_key("missing"));
    }

    #[test]
    fn test_default_get_many_empty() {
        let cache = MinimalCache::new();
        let results = cache.get_many(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_default_set_many_implementation() {
        let cache = MinimalCache::new();
        let items = vec![
            ("k1".to_string(), b"v1".to_vec()),
            ("k2".to_string(), b"v2".to_vec()),
            ("k3".to_string(), b"v3".to_vec()),
        ];
        cache.set_many(&items);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get("k1"), Some(b"v1".to_vec()));
        assert_eq!(cache.get("k2"), Some(b"v2".to_vec()));
        assert_eq!(cache.get("k3"), Some(b"v3".to_vec()));
    }

    #[test]
    fn test_default_set_many_empty() {
        let cache = MinimalCache::new();
        cache.set_many(&[]);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_default_delete_many_implementation() {
        let cache = MinimalCache::new();
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());
        cache.set("k3", b"v3".to_vec());

        let deleted = cache.delete_many(&["k1", "k3", "missing"]);
        assert_eq!(deleted, 2);
        assert_eq!(cache.len(), 1);
        assert!(cache.contains("k2"));
    }

    #[test]
    fn test_default_delete_many_empty() {
        let cache = MinimalCache::new();
        let deleted = cache.delete_many(&[]);
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_default_get_stats_implementation() {
        let cache = MinimalCache::new();
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());

        // Default get_stats returns an empty HashMap
        let stats = cache.get_stats();
        assert!(stats.is_empty());
    }

    #[test]
    fn test_default_invalidate_implementation() {
        let cache = MinimalCache::new();
        cache.set("user:1", b"v1".to_vec());
        cache.set("user:2", b"v2".to_vec());
        cache.set("session:1", b"s1".to_vec());

        // Default invalidate uses find_keys_by_pattern + delete_many
        let deleted = cache.invalidate("user:*");
        assert_eq!(deleted, 2);
        assert_eq!(cache.len(), 1);
        assert!(cache.contains("session:1"));
    }

    // ========================================================================
    // Mutex poison 分支测试
    //
    // OxcacheSyncCache 在 set/delete/clear/find_keys_by_pattern/set_many/
    // delete_many 中先获取 `key_index` 锁。若锁中毒（持有锁时 panic），
    // 各方法会降级为 backend-only 模式或返回安全默认值。这些测试通过
    // 故意 panic 让 Mutex 中毒，验证降级路径。
    // ========================================================================

    /// Helper: 让 cache 的 key_index Mutex 中毒。
    ///
    /// 持有锁时 panic 会导致 Mutex 中毒，后续 lock() 调用返回 Err。
    fn poison_key_index(cache: &OxcacheSyncCache) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = cache.key_index.lock().unwrap();
            panic!("intentional panic to poison key_index mutex");
        }));
        assert!(result.is_err(), "poisoning panic should be caught");
    }

    #[test]
    fn test_set_with_poisoned_mutex_falls_back_to_backend() {
        let cache = OxcacheSyncCache::new();
        poison_key_index(&cache);

        // Mutex 中毒后，set 应降级为 backend-only 写入（不更新 index）
        cache.set("poisoned_key", b"poisoned_value".to_vec());

        // backend 仍然能读到值（通过 contains/get 验证）
        assert!(cache.contains("poisoned_key"));
        assert_eq!(cache.get("poisoned_key"), Some(b"poisoned_value".to_vec()));
    }

    #[test]
    fn test_delete_with_poisoned_mutex_falls_back_to_backend() {
        let cache = OxcacheSyncCache::new();
        // 先写入一个值（此时 Mutex 未中毒）
        cache.set("to_delete", b"value".to_vec());
        assert!(cache.contains("to_delete"));

        // 中毒 Mutex
        poison_key_index(&cache);

        // delete 应降级为 backend-only 删除
        let deleted = cache.delete("to_delete");
        assert!(deleted, "delete should report the key existed");
        assert!(!cache.contains("to_delete"));
    }

    #[test]
    fn test_delete_with_poisoned_mutex_nonexistent_key() {
        let cache = OxcacheSyncCache::new();
        poison_key_index(&cache);

        // 删除不存在的键，backend.exists 返回 false，应返回 false
        let deleted = cache.delete("nonexistent_key");
        assert!(!deleted);
    }

    #[test]
    fn test_clear_with_poisoned_mutex_falls_back_to_backend() {
        let cache = OxcacheSyncCache::new();
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());
        assert_eq!(cache.len(), 2);

        poison_key_index(&cache);

        // clear 应降级为 backend-only 清空
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_find_keys_by_pattern_with_poisoned_mutex_returns_empty() {
        let cache = OxcacheSyncCache::new();
        cache.set("user:1", b"v1".to_vec());
        cache.set("user:2", b"v2".to_vec());

        poison_key_index(&cache);

        // Mutex 中毒时，find_keys_by_pattern 直接返回空 Vec
        let keys = cache.find_keys_by_pattern("user:*");
        assert!(keys.is_empty(), "poisoned mutex should yield empty result");
    }

    #[test]
    fn test_set_many_with_poisoned_mutex_falls_back_to_backend() {
        let cache = OxcacheSyncCache::new();
        poison_key_index(&cache);

        let items = vec![
            ("batch_k1".to_string(), b"v1".to_vec()),
            ("batch_k2".to_string(), b"v2".to_vec()),
        ];
        cache.set_many(&items);

        // backend 应能读到写入的值
        assert!(cache.contains("batch_k1"));
        assert!(cache.contains("batch_k2"));
        assert_eq!(cache.get("batch_k2"), Some(b"v2".to_vec()));
    }

    #[test]
    fn test_delete_many_with_poisoned_mutex_falls_back_to_backend() {
        let cache = OxcacheSyncCache::new();
        // 先写入值（Mutex 未中毒时）
        cache.set("del_k1", b"v1".to_vec());
        cache.set("del_k2", b"v2".to_vec());
        cache.set("del_k3", b"v3".to_vec());

        poison_key_index(&cache);

        // delete_many 应降级为 backend-only 删除
        let deleted = cache.delete_many(&["del_k1", "del_k3", "nonexistent"]);
        assert_eq!(deleted, 2, "should report 2 keys deleted");
        assert!(!cache.contains("del_k1"));
        assert!(cache.contains("del_k2"));
        assert!(!cache.contains("del_k3"));
    }

    #[test]
    fn test_delete_many_with_poisoned_mutex_all_nonexistent() {
        let cache = OxcacheSyncCache::new();
        poison_key_index(&cache);

        // 所有键都不存在时，deleted 应为 0
        let deleted = cache.delete_many(&["no_k1", "no_k2"]);
        assert_eq!(deleted, 0);
    }

    /// Helper: poison the key_index mutex while a key is already in the index.
    /// Used to verify that find_keys_by_pattern's stale-key cleanup path
    /// (lines 408-416) is not the only path exercised.
    #[test]
    fn test_get_with_poisoned_mutex_still_works() {
        // get() 不使用 key_index 锁，只读 backend，所以不受中毒影响
        let cache = OxcacheSyncCache::new();
        cache.set("k", b"v".to_vec());
        poison_key_index(&cache);

        // get 应仍然正常工作（不获取 key_index 锁）
        assert_eq!(cache.get("k"), Some(b"v".to_vec()));
        assert!(cache.contains("k"));
    }

    /// Verify that find_keys_by_pattern removes stale keys from the index
    /// when the backend has evicted them (lines 409-416).
    #[test]
    fn test_find_keys_by_pattern_removes_stale_keys() {
        let cache = OxcacheSyncCache::new();

        // Manually insert keys into the index without setting them in the backend
        {
            let mut idx = cache.key_index.lock().unwrap();
            idx.insert("stale_key_1".to_string());
            idx.insert("stale_key_2".to_string());
        }

        // Call find_keys_by_pattern with a glob pattern that matches
        let result = cache.find_keys_by_pattern("stale_*");

        // The stale keys should not be in the results (backend doesn't have them)
        assert!(result.is_empty(), "Stale keys should not appear in results");

        // The stale keys should have been removed from the index
        let idx = cache.key_index.lock().unwrap();
        assert!(
            !idx.contains("stale_key_1"),
            "Stale key 1 should be removed from index"
        );
        assert!(
            !idx.contains("stale_key_2"),
            "Stale key 2 should be removed from index"
        );
    }
}
