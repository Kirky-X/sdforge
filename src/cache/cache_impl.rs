// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

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

/// 通配符模式匹配：`*` 匹配任意字符序列（含空），`?` 匹配单个字符。
///
/// 与将 `*`→`.*`、`?`→`.` 后用 `^pattern$` 正则匹配的行为等价，
/// 但无需编译 regex，且不引入 regex 依赖到 `cache` feature。
pub(crate) fn matches_pattern(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let mut pi = 0usize;
    let mut ti = 0usize;
    let mut star_pi: Option<usize> = None;
    let mut star_ti = 0usize;

    while ti < t.len() {
        if pi < p.len() && (p[pi] == t[ti] || p[pi] == '?') {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star_pi = Some(pi);
            star_ti = ti;
            pi += 1;
        } else if let Some(spi) = star_pi {
            pi = spi + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }

    pi == p.len()
}

impl OxcacheSyncCache {
    /// 创建默认容量的同步缓存
    pub fn new() -> Self {
        Self {
            backend: oxcache::backend::DashMapMemoryBackend::new(),
            key_index: Mutex::new(HashSet::new()),
        }
    }

    /// 创建指定容量的同步缓存
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            backend: oxcache::backend::DashMapMemoryBackend::builder()
                .capacity(capacity)
                .build(),
            key_index: Mutex::new(HashSet::with_capacity(capacity)),
        }
    }

    /// 获取内部 backend 引用（供高级用途）
    pub fn inner(&self) -> &oxcache::backend::DashMapMemoryBackend {
        &self.backend
    }
}

impl Default for OxcacheSyncCache {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for OxcacheSyncCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.key_index.lock().map(|idx| idx.len()).unwrap_or(0);
        f.debug_struct("OxcacheSyncCache")
            .field("backend", &"DashMapMemoryBackend")
            .field("key_count", &len)
            .finish()
    }
}

impl SyncCache for OxcacheSyncCache {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        use oxcache::backend::SyncCacheReader;
        self.backend.get(key).ok().flatten()
    }

    fn set(&self, key: &str, value: Vec<u8>) {
        use oxcache::backend::SyncCacheWriter;
        // 持有 index 锁直到 backend 操作完成，保证 backend 与 index 一致性
        // (HIGH-001: 避免并发下 backend 有键但 index 缺失的竞态)
        let mut idx = match self.key_index.lock() {
            Ok(idx) => idx,
            Err(_) => {
                log::warn!(
                    "cache key_index poisoned; set falling back to backend-only for key={:?}",
                    key
                );
                if let Err(e) = self.backend.set(Arc::from(key), Arc::new(value), None) {
                    log::warn!("cache backend set failed for key={:?}: {}", key, e);
                }
                return;
            }
        };
        // HIGH-002: 不静默吞掉 backend 错误；失败时不更新 index 以保持一致
        if let Err(e) = self.backend.set(Arc::from(key), Arc::new(value), None) {
            log::warn!("cache backend set failed for key={:?}: {}", key, e);
            return;
        }
        idx.insert(key.to_string());
    }

    fn delete(&self, key: &str) -> bool {
        use oxcache::backend::{SyncCacheReader, SyncCacheWriter};
        let mut idx = match self.key_index.lock() {
            Ok(idx) => idx,
            Err(_) => {
                log::warn!(
                    "cache key_index poisoned; delete falling back to backend-only for key={:?}",
                    key
                );
                let existed = self.backend.exists(key).unwrap_or(false);
                if existed && let Err(e) = self.backend.delete(key) {
                    log::warn!("cache backend delete failed for key={:?}: {}", key, e);
                }
                return existed;
            }
        };
        let existed = self.backend.exists(key).unwrap_or(false);
        if existed {
            // HIGH-002: backend 失败时不更新 index，保持一致
            if let Err(e) = self.backend.delete(key) {
                log::warn!("cache backend delete failed for key={:?}: {}", key, e);
                return existed;
            }
            idx.remove(key);
        }
        existed
    }

    fn contains(&self, key: &str) -> bool {
        use oxcache::backend::SyncCacheReader;
        self.backend.exists(key).unwrap_or(false)
    }

    fn clear(&self) {
        use oxcache::backend::SyncCacheWriter;
        let mut idx = match self.key_index.lock() {
            Ok(idx) => idx,
            Err(_) => {
                log::warn!("cache key_index poisoned; clear falling back to backend-only");
                if let Err(e) = self.backend.clear() {
                    log::warn!("cache backend clear failed: {}", e);
                }
                return;
            }
        };
        if let Err(e) = self.backend.clear() {
            log::warn!("cache backend clear failed: {}", e);
            return;
        }
        idx.clear();
    }

    fn len(&self) -> usize {
        use oxcache::backend::SyncCacheReader;
        self.backend.len().unwrap_or(0) as usize
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn find_keys_by_pattern(&self, pattern: &str) -> Vec<String> {
        use oxcache::backend::SyncCacheReader;
        let mut idx = match self.key_index.lock() {
            Ok(idx) => idx,
            Err(_) => return Vec::new(),
        };
        // BUG-4 修复: oxcache backend 达到容量时会内部驱逐键，但 `key_index` 不会同步感知，
        // 导致 index 逐渐成为 backend 的超集，`find_keys_by_pattern` 返回已不存在的键。
        //
        // 修复策略：遍历 index 时通过 `backend.exists()` 过滤，并惰性清理已被驱逐的键，
        // 既保证返回结果与 backend 一致，又避免 index 无限增长（内存泄漏）。
        let mut result = Vec::new();
        let mut stale_keys = Vec::new();
        for k in idx.iter() {
            if !matches_pattern(pattern, k) {
                continue;
            }
            if self.backend.exists(k).unwrap_or(false) {
                result.push(k.clone());
            } else {
                // backend 已驱逐此键，标记为 stale 以便从 index 移除
                stale_keys.push(k.clone());
            }
        }
        // 惰性清理：移除已被 backend 驱逐的键，防止 index 内存泄漏
        for k in stale_keys {
            idx.remove(&k);
        }
        result
    }

    fn get_many(&self, keys: &[&str]) -> HashMap<String, Vec<u8>> {
        use oxcache::backend::SyncCacheReader;
        let mut results = HashMap::with_capacity(keys.len());
        for &key in keys {
            if let Ok(Some(value)) = self.backend.get(key) {
                results.insert(key.to_string(), value);
            }
        }
        results
    }

    fn set_many(&self, items: &[(String, Vec<u8>)]) {
        use oxcache::backend::SyncCacheWriter;
        let mut idx = match self.key_index.lock() {
            Ok(idx) => idx,
            Err(_) => {
                log::warn!(
                    "cache key_index poisoned; set_many falling back to backend-only for {} items",
                    items.len()
                );
                for (key, value) in items {
                    if let Err(e) =
                        self.backend
                            .set(Arc::from(key.as_str()), Arc::new(value.clone()), None)
                    {
                        log::warn!("cache backend set failed for key={:?}: {}", key, e);
                    }
                }
                return;
            }
        };
        // 先执行所有 backend 写入，成功后才更新 index，避免部分失败导致 index 与 backend 不一致
        let mut succeeded: Vec<&String> = Vec::with_capacity(items.len());
        for (key, value) in items {
            if let Err(e) = self
                .backend
                .set(Arc::from(key.as_str()), Arc::new(value.clone()), None)
            {
                log::warn!("cache backend set failed for key={:?}: {}", key, e);
            } else {
                succeeded.push(key);
            }
        }
        for key in succeeded {
            idx.insert(key.clone());
        }
    }

    fn delete_many(&self, keys: &[&str]) -> usize {
        use oxcache::backend::{SyncCacheReader, SyncCacheWriter};
        let mut idx = match self.key_index.lock() {
            Ok(idx) => idx,
            Err(_) => {
                log::warn!(
                    "cache key_index poisoned; delete_many falling back to backend-only for {} keys",
                    keys.len()
                );
                let mut deleted = 0usize;
                for &key in keys {
                    let existed = self.backend.exists(key).unwrap_or(false);
                    if existed {
                        if let Err(e) = self.backend.delete(key) {
                            log::warn!("cache backend delete failed for key={:?}: {}", key, e);
                        } else {
                            deleted += 1;
                        }
                    }
                }
                return deleted;
            }
        };
        let mut deleted = 0usize;
        for &key in keys {
            let existed = self.backend.exists(key).unwrap_or(false);
            if existed {
                if let Err(e) = self.backend.delete(key) {
                    log::warn!("cache backend delete failed for key={:?}: {}", key, e);
                } else {
                    idx.remove(key);
                    deleted += 1;
                }
            }
        }
        deleted
    }

    fn get_stats(&self) -> HashMap<String, u64> {
        use oxcache::backend::SyncCacheReader;
        let mut stats = HashMap::new();
        let len = self.backend.len().unwrap_or(0);
        stats.insert("total_keys".to_string(), len);
        stats.insert(
            "capacity".to_string(),
            SyncCacheReader::capacity(&self.backend).unwrap_or(0),
        );
        // 透传 backend stats（命中数、未命中数、命中率等）
        //
        // BUG-5 修复: 原代码仅尝试 `v.parse::<u64>()`，对 float 类型统计
        // （如 hit_rate="0.85"）静默丢弃，违反 Rule 12（失败必须显性化）。
        //
        // 修复策略：
        // 1. 先尝试 u64 解析（适用于 hits、misses 等整数统计）
        // 2. 失败则尝试 f64 解析：
        //    - 若键名含 "rate"/"ratio"/"pct"，视为 0.0-1.0 的比率，×100 后四舍五入为百分比 u64
        //    - 否则直接四舍五入为 u64
        // 3. 两者均失败则 log::warn! 显性化（不再静默丢弃）
        if let Ok(backend_stats) = self.backend.stats() {
            for (k, v) in backend_stats {
                if let Ok(n) = v.parse::<u64>() {
                    stats.entry(k).or_insert(n);
                } else if let Ok(f) = v.parse::<f64>() {
                    let lower = k.to_lowercase();
                    let converted = if lower.contains("rate")
                        || lower.contains("ratio")
                        || lower.contains("pct")
                    {
                        (f * 100.0).round() as u64
                    } else {
                        f.round() as u64
                    };
                    log::debug!(
                        "cache stat {:?} parsed as f64={} converted to u64={}",
                        k,
                        f,
                        converted
                    );
                    stats.entry(k).or_insert(converted);
                } else {
                    log::warn!(
                        "cache stat {:?} value {:?} could not be parsed as u64 or f64; dropped",
                        k,
                        v
                    );
                }
            }
        }
        stats
    }
}
