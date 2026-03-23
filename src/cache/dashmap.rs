// Copyright (c) 2026 Kirky.X
//!
//! 基于 DashMap 的同步缓存实现
//!
//! 提供 `SyncCache` trait 的内存实现。
//! 内部使用 `dashmap::DashMap`，提供 O(1) 的并发读写性能。

use crate::cache::traits::SyncCache;
use std::sync::Arc;

/// 基于 DashMap 的同步内存缓存
///
/// # 特点
///
/// - **同步**：所有操作立即返回，无需 async
/// - **并发安全**：`DashMap` 无锁并发读写
/// - **无 TTL**：TTL 由调用方管理（可通过定期清理实现）
///
/// # 与 oxcache 的关系
///
/// 此实现是 sdforge 功能组件的默认存储。
/// 如需 TTL 支持，未来可通过包装 oxcache::DashMapMemoryBackend 实现。
#[derive(Debug, Clone)]
pub struct DashMapCache {
    /// 内部存储
    inner: Arc<dashmap::DashMap<String, Vec<u8>>>,
}

impl DashMapCache {
    /// 创建默认容量（10000）的缓存
    pub fn new() -> Self {
        Self {
            inner: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// 创建指定容量的缓存
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Arc::new(dashmap::DashMap::with_capacity(capacity)),
        }
    }

    /// 获取底层 DashMap 的 Arc 引用（用于高级操作）
    pub fn inner(&self) -> &Arc<dashmap::DashMap<String, Vec<u8>>> {
        &self.inner
    }
}

impl Default for DashMapCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncCache for DashMapCache {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.inner.get(key).map(|v| v.clone())
    }

    fn set(&self, key: &str, value: Vec<u8>) {
        self.inner.insert(key.to_string(), value);
    }

    fn delete(&self, key: &str) -> bool {
        self.inner.remove(key).is_some()
    }

    fn contains(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    fn clear(&self) {
        self.inner.clear();
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let cache = DashMapCache::new();
        cache.set("key1", b"value1".to_vec());
        assert_eq!(cache.get("key1"), Some(b"value1".to_vec()));
    }

    #[test]
    fn test_get_nonexistent() {
        let cache = DashMapCache::new();
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_delete_existing() {
        let cache = DashMapCache::new();
        cache.set("key1", b"value1".to_vec());
        assert!(cache.delete("key1"));
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_delete_nonexistent() {
        let cache = DashMapCache::new();
        assert!(!cache.delete("nonexistent"));
    }

    #[test]
    fn test_contains() {
        let cache = DashMapCache::new();
        cache.set("key1", b"value1".to_vec());
        assert!(cache.contains("key1"));
        assert!(!cache.contains("key2"));
    }

    #[test]
    fn test_clear() {
        let cache = DashMapCache::new();
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_len() {
        let cache = DashMapCache::new();
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_is_empty() {
        let cache = DashMapCache::new();
        assert!(cache.is_empty());
        cache.set("k1", b"v1".to_vec());
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_default() {
        let cache = DashMapCache::default();
        cache.set("key", b"value".to_vec());
        assert_eq!(cache.get("key"), Some(b"value".to_vec()));
    }

    #[test]
    fn test_with_capacity() {
        let cache = DashMapCache::with_capacity(1000);
        cache.set("key", b"value".to_vec());
        assert_eq!(cache.get("key"), Some(b"value".to_vec()));
    }

    #[test]
    fn test_clone_is_independent() {
        let cache1 = DashMapCache::new();
        cache1.set("key", b"value".to_vec());
        let cache2 = cache1.clone();
        // Clone shares the same inner Arc, so they share data
        assert_eq!(cache2.get("key"), Some(b"value".to_vec()));
    }
}
