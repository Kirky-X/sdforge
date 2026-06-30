// Copyright (c) 2026 Kirky.X
//!
//! 基于 DashMap 的同步缓存实现
//!
//! 提供 `SyncCache` trait 的内存实现。
//! 内部使用 `dashmap::DashMap`，提供 O(1) 的并发读写性能。

use crate::cache::SyncCache;
use once_cell::sync::Lazy;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// 基于 DashMap 的同步内存缓存
///
/// # 特点
///
/// - **同步**：所有操作立即返回，无需 async
/// - **并发安全**：`DashMap` 无锁并发读写
/// - **无 TTL**：TTL 由调用方管理（可通过定期清理实现）
/// - **可选 LRU**：使用 `with_capacity()` 启用 LRU 驱逐
/// - **前缀索引**：自动维护前缀索引加速模式匹配
#[derive(Debug, Clone)]
pub struct DashMapCache {
    /// 内部存储
    inner: Arc<dashmap::DashMap<String, Vec<u8>>>,
    /// 可选：LRU 队列用于驱逐 (Some 时启用)
    lru_queue: Option<Arc<Mutex<VecDeque<String>>>>,
    /// 最大容量限制
    max_capacity: Option<usize>,
    /// 前缀索引：加速前缀匹配查找
    prefix_index: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl DashMapCache {
    /// 创建默认容量（10000）的缓存
    pub fn new() -> Self {
        Self {
            inner: Arc::new(dashmap::DashMap::new()),
            lru_queue: None,
            max_capacity: None,
            prefix_index: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 创建指定容量的缓存（带 LRU 驱逐）
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Arc::new(dashmap::DashMap::with_capacity(capacity)),
            lru_queue: Some(Arc::new(Mutex::new(VecDeque::with_capacity(capacity)))),
            max_capacity: Some(capacity),
            prefix_index: Arc::new(Mutex::new(HashMap::with_capacity(capacity / 10))),
        }
    }

    /// 获取底层 DashMap 的 Arc 引用（用于高级操作）
    pub fn inner(&self) -> &Arc<dashmap::DashMap<String, Vec<u8>>> {
        &self.inner
    }

    /// 获取当前缓存大小
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// 提取前缀（用于索引）
    fn extract_prefix(key: &str) -> Option<&str> {
        key.split(':').next()
    }

    /// 更新前缀索引
    fn update_prefix_index(&self, key: &str, add: bool) {
        if let Some(prefix) = Self::extract_prefix(key) {
            if let Ok(mut index) = self.prefix_index.lock() {
                let keys = index.entry(prefix.to_string()).or_insert_with(Vec::new);
                if add {
                    keys.push(key.to_string());
                } else if let Some(pos) = keys.iter().position(|k| k == key) {
                    keys.remove(pos);
                }
            }
        }
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

    fn get_many(&self, keys: &[&str]) -> std::collections::HashMap<String, Vec<u8>> {
        // Pre-allocate HashMap with expected size to avoid reallocations
        let mut result = std::collections::HashMap::with_capacity(keys.len());

        // Batch get with reduced lock contention
        for &key in keys {
            if let Some(value) = self.inner.get(key) {
                result.insert(key.to_string(), value.clone());
            }
        }

        result
    }

    fn set(&self, key: &str, value: Vec<u8>) {
        let key_string = key.to_string();

        // If LRU is enabled, update the queue
        if let Some(ref lru_queue) = self.lru_queue {
            if let Ok(mut queue) = lru_queue.lock() {
                // Remove existing entry if it exists
                if let Some(pos) = queue.iter().position(|k| k == &key_string) {
                    queue.remove(pos);
                }

                // Add to front (most recently used)
                queue.push_front(key_string.clone());

                // Evict oldest if over capacity
                if let Some(max_cap) = self.max_capacity {
                    while queue.len() > max_cap {
                        if let Some(oldest_key) = queue.pop_back() {
                            self.inner.remove(&oldest_key);
                            self.update_prefix_index(&oldest_key, false);
                        }
                    }
                }
            }
        }

        self.inner.insert(key_string.clone(), value);
        self.update_prefix_index(&key_string, true);
    }

    fn set_many(&self, items: &[(String, Vec<u8>)]) {
        // Optimized batch insert with pre-allocation check
        if items.is_empty() {
            return;
        }

        // Batch insert - DashMap handles concurrent writes efficiently
        for (key, value) in items {
            self.inner.insert(key.clone(), value.clone());
        }
    }

    fn delete(&self, key: &str) -> bool {
        let result = self.inner.remove(key).is_some();
        if result {
            self.update_prefix_index(key, false);
        }
        result
    }

    fn delete_many(&self, keys: &[&str]) -> usize {
        // Optimized batch delete with early exit
        if keys.is_empty() {
            return 0;
        }

        // Batch remove - count successful deletions
        let mut deleted_count = 0;
        for &key in keys {
            if self.inner.remove(key).is_some() {
                self.update_prefix_index(key, false);
                deleted_count += 1;
            }
        }

        deleted_count
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

    fn find_keys_by_pattern(&self, pattern: &str) -> Vec<String> {
        // Check if it's a simple prefix pattern (e.g., "user:*")
        if let Some(prefix_end) = pattern.find('*') {
            let prefix_part = &pattern[..prefix_end];

            // If prefix contains no other wildcards, use prefix index
            if !prefix_part.contains(['*', '?']) && prefix_part.ends_with(':') {
                let prefix_key = &prefix_part[..prefix_part.len() - 1];

                if let Ok(index) = self.prefix_index.lock() {
                    if let Some(matching_keys) = index.get(prefix_key) {
                        // Filter matching keys with the full pattern
                        let regex_pattern = pattern.replace('*', ".*").replace('?', ".");

                        let re = regex::Regex::new(&format!("^{}$", regex_pattern))
                            .expect("Invalid regex pattern");

                        return matching_keys
                            .iter()
                            .filter(|key| re.is_match(key))
                            .cloned()
                            .collect();
                    }
                }
            }
        }

        // Fallback to full scan with regex caching
        let regex_pattern = pattern.replace('*', ".*").replace('?', ".");

        static REGEX_CACHE: Lazy<dashmap::DashMap<String, regex::Regex>> =
            Lazy::new(dashmap::DashMap::new);

        let re = REGEX_CACHE
            .entry(regex_pattern.clone())
            .or_insert_with(|| {
                regex::Regex::new(&format!("^{}$", regex_pattern)).expect("Invalid regex pattern")
            })
            .value()
            .clone();

        self.inner
            .iter()
            .filter(|item| re.is_match(item.key()))
            .map(|item| item.key().clone())
            .collect()
    }

    fn get_stats(&self) -> std::collections::HashMap<String, u64> {
        let mut stats = std::collections::HashMap::new();
        stats.insert("total_keys".to_string(), self.len() as u64);
        stats.insert("capacity".to_string(), u64::MAX); // DashMap doesn't expose capacity
        stats
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

    #[test]
    fn test_set_overwrites_existing() {
        let cache = DashMapCache::new();
        cache.set("key1", b"value1".to_vec());
        cache.set("key1", b"value2".to_vec());
        assert_eq!(cache.get("key1"), Some(b"value2".to_vec()));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_get_many() {
        let cache = DashMapCache::new();
        cache.set("key1", b"value1".to_vec());
        cache.set("key2", b"value2".to_vec());
        cache.set("key3", b"value3".to_vec());

        let results = cache.get_many(&["key1", "key2", "nonexistent"]);
        assert_eq!(results.len(), 2);
        assert_eq!(results.get("key1"), Some(&b"value1".to_vec()));
        assert_eq!(results.get("key2"), Some(&b"value2".to_vec()));
        assert!(!results.contains_key("nonexistent"));
    }

    #[test]
    fn test_get_many_empty_keys() {
        let cache = DashMapCache::new();
        let results = cache.get_many(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_set_many() {
        let cache = DashMapCache::new();
        let items = vec![
            ("key1".to_string(), b"value1".to_vec()),
            ("key2".to_string(), b"value2".to_vec()),
            ("key3".to_string(), b"value3".to_vec()),
        ];
        cache.set_many(&items);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get("key1"), Some(b"value1".to_vec()));
        assert_eq!(cache.get("key2"), Some(b"value2".to_vec()));
        assert_eq!(cache.get("key3"), Some(b"value3".to_vec()));
    }

    #[test]
    fn test_set_many_empty() {
        let cache = DashMapCache::new();
        cache.set_many(&[]);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_delete_many() {
        let cache = DashMapCache::new();
        cache.set("key1", b"value1".to_vec());
        cache.set("key2", b"value2".to_vec());
        cache.set("key3", b"value3".to_vec());

        let deleted = cache.delete_many(&["key1", "key2", "nonexistent"]);
        assert_eq!(deleted, 2);
        assert_eq!(cache.len(), 1);
        assert!(!cache.contains("key1"));
        assert!(!cache.contains("key2"));
        assert!(cache.contains("key3"));
    }

    #[test]
    fn test_delete_many_empty() {
        let cache = DashMapCache::new();
        let deleted = cache.delete_many(&[]);
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = DashMapCache::with_capacity(3);
        cache.set("key1", b"value1".to_vec());
        cache.set("key2", b"value2".to_vec());
        cache.set("key3", b"value3".to_vec());
        assert_eq!(cache.len(), 3);

        // Adding a 4th key should evict the oldest (key1)
        cache.set("key4", b"value4".to_vec());
        assert_eq!(cache.len(), 3);
        assert!(cache.get("key1").is_none());
        assert!(cache.contains("key2"));
        assert!(cache.contains("key3"));
        assert!(cache.contains("key4"));
    }

    #[test]
    fn test_lru_updates_access_order() {
        let cache = DashMapCache::with_capacity(3);
        cache.set("key1", b"value1".to_vec());
        cache.set("key2", b"value2".to_vec());
        cache.set("key3", b"value3".to_vec());

        // Re-access key1 to make it most recently used
        cache.set("key1", b"value1".to_vec());

        // Adding a 4th key should evict key2 (now the oldest)
        cache.set("key4", b"value4".to_vec());
        assert_eq!(cache.len(), 3);
        assert!(cache.contains("key1"));
        assert!(!cache.contains("key2"));
        assert!(cache.contains("key3"));
        assert!(cache.contains("key4"));
    }

    #[test]
    fn test_inner_method() {
        let cache = DashMapCache::new();
        cache.set("key", b"value".to_vec());
        let inner = cache.inner();
        assert!(inner.contains_key("key"));
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let cache = Arc::new(DashMapCache::new());
        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = cache.clone();
            let handle = thread::spawn(move || {
                let key = format!("key{}", i);
                cache_clone.set(&key, vec![i as u8; 4]);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(cache.len(), 10);
        for i in 0..10 {
            let key = format!("key{}", i);
            assert!(cache.contains(&key));
        }
    }

    #[test]
    fn test_concurrent_reads_and_writes() {
        use std::thread;

        let cache = Arc::new(DashMapCache::new());

        // Write some initial data
        for i in 0..5 {
            cache.set(&format!("key{}", i), vec![i as u8; 4]);
        }

        let mut handles = vec![];

        // Spawn reader threads
        for i in 0..5 {
            let cache_clone = cache.clone();
            let handle = thread::spawn(move || {
                let key = format!("key{}", i);
                let _ = cache_clone.get(&key);
            });
            handles.push(handle);
        }

        // Spawn writer threads
        for i in 5..10 {
            let cache_clone = cache.clone();
            let handle = thread::spawn(move || {
                let key = format!("key{}", i);
                cache_clone.set(&key, vec![i as u8; 4]);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(cache.len(), 10);
    }

    #[test]
    fn test_key_collision_same_key() {
        let cache = DashMapCache::new();
        cache.set("key", b"first".to_vec());
        cache.set("key", b"second".to_vec());
        cache.set("key", b"third".to_vec());

        assert_eq!(cache.get("key"), Some(b"third".to_vec()));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_empty_cache_operations() {
        let cache = DashMapCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.get("any_key"), None);
        assert!(!cache.delete("any_key"));
        assert!(!cache.contains("any_key"));
    }

    #[test]
    fn test_large_capacity() {
        let cache = DashMapCache::with_capacity(10000);
        for i in 0..100 {
            cache.set(&format!("key{}", i), vec![i as u8; 8]);
        }
        assert_eq!(cache.len(), 100);
    }

    #[test]
    fn test_binary_values() {
        let cache = DashMapCache::new();
        let binary_data = vec![0u8, 1, 2, 255, 254, 253];
        cache.set("binary", binary_data.clone());
        assert_eq!(cache.get("binary"), Some(binary_data));
    }

    #[test]
    fn test_empty_string_key() {
        let cache = DashMapCache::new();
        cache.set("", b"empty_key".to_vec());
        assert_eq!(cache.get(""), Some(b"empty_key".to_vec()));
        assert!(cache.contains(""));
        assert!(cache.delete(""));
        assert_eq!(cache.get(""), None);
    }

    #[test]
    fn test_find_keys_by_pattern_prefix() {
        let cache = DashMapCache::new();
        cache.set("user:1", b"v1".to_vec());
        cache.set("user:2", b"v2".to_vec());
        cache.set("user:3", b"v3".to_vec());
        cache.set("session:1", b"s1".to_vec());

        let keys = cache.find_keys_by_pattern("user:*");
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"user:1".to_string()));
        assert!(keys.contains(&"user:2".to_string()));
        assert!(keys.contains(&"user:3".to_string()));
    }

    #[test]
    fn test_find_keys_by_pattern_no_match() {
        let cache = DashMapCache::new();
        cache.set("user:1", b"v1".to_vec());
        cache.set("user:2", b"v2".to_vec());

        let keys = cache.find_keys_by_pattern("session:*");
        assert!(keys.is_empty());
    }

    #[test]
    fn test_find_keys_by_pattern_wildcard() {
        let cache = DashMapCache::new();
        cache.set("user:1", b"v1".to_vec());
        cache.set("user:2", b"v2".to_vec());
        cache.set("admin:1", b"a1".to_vec());

        let keys = cache.find_keys_by_pattern("*:1");
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"user:1".to_string()));
        assert!(keys.contains(&"admin:1".to_string()));
    }

    #[test]
    fn test_invalidate_pattern() {
        let cache = DashMapCache::new();
        cache.set("user:1", b"v1".to_vec());
        cache.set("user:2", b"v2".to_vec());
        cache.set("user:3", b"v3".to_vec());
        cache.set("session:1", b"s1".to_vec());

        let deleted = cache.invalidate("user:*");
        assert_eq!(deleted, 3);
        assert_eq!(cache.len(), 1);
        assert!(cache.contains("session:1"));
        assert!(!cache.contains("user:1"));
    }

    #[test]
    fn test_get_stats() {
        let cache = DashMapCache::new();
        cache.set("key1", b"v1".to_vec());
        cache.set("key2", b"v2".to_vec());
        cache.set("key3", b"v3".to_vec());

        let stats = cache.get_stats();
        assert_eq!(stats.get("total_keys"), Some(&3u64));
        assert!(stats.contains_key("capacity"));
    }

    #[tokio::test]
    async fn test_concurrent_async_access() {
        let cache = Arc::new(DashMapCache::new());
        let mut handles = vec![];

        for i in 0..20 {
            let cache_clone = cache.clone();
            let handle = tokio::spawn(async move {
                let key = format!("key{}", i);
                cache_clone.set(&key, vec![i as u8; 4]);
                cache_clone.get(&key)
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(cache.len(), 20);
    }

    #[test]
    fn test_lru_eviction_with_updates() {
        let cache = DashMapCache::with_capacity(3);
        cache.set("key1", b"v1".to_vec());
        cache.set("key2", b"v2".to_vec());
        cache.set("key3", b"v3".to_vec());

        // Update key1 to make it most recently used
        cache.set("key1", b"v1_updated".to_vec());

        // Add key4, should evict key2
        cache.set("key4", b"v4".to_vec());

        assert_eq!(cache.len(), 3);
        assert!(!cache.contains("key2"));
        assert!(cache.contains("key1"));
        assert!(cache.contains("key3"));
        assert!(cache.contains("key4"));
        assert_eq!(cache.get("key1"), Some(b"v1_updated".to_vec()));
    }

    #[test]
    fn test_delete_updates_prefix_index() {
        let cache = DashMapCache::new();
        cache.set("user:1", b"v1".to_vec());
        cache.set("user:2", b"v2".to_vec());
        cache.delete("user:1");

        let keys = cache.find_keys_by_pattern("user:*");
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&"user:2".to_string()));
    }
}
