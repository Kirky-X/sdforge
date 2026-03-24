// Copyright (c) 2026 Kirky.X
//!
//! sdforge::DashMapCache 到 oxcache::sync::SyncCache 的桥接实现
//!
//! 当启用 `cache` feature 时，sdforge 的 DashMapCache 同时实现两个 trait：
//! - `sdforge::cache::SyncCache`（本地 trait）
//! - `oxcache::sync::SyncCache`（来自 oxcache 库的 trait）
//!
//! 两个 trait 的定义完全相同，可以互相替换使用。

use crate::cache::{DashMapCache, SyncCache};
use oxcache::sync::SyncCache as OxSyncCache;

/// 为 sdforge::DashMapCache 实现 oxcache::sync::SyncCache
///
/// 这样功能组件可以接受 `Arc<dyn oxcache::sync::SyncCache>` 类型的存储，
/// 同时仍然可以使用 sdforge::DashMapCache 作为实现。
impl OxSyncCache for DashMapCache {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        SyncCache::get(self, key)
    }

    fn set(&self, key: &str, value: Vec<u8>) {
        SyncCache::set(self, key, value)
    }

    fn delete(&self, key: &str) -> bool {
        SyncCache::delete(self, key)
    }

    fn contains(&self, key: &str) -> bool {
        SyncCache::contains(self, key)
    }

    fn clear(&self) {
        SyncCache::clear(self)
    }

    fn len(&self) -> usize {
        SyncCache::len(self)
    }

    fn is_empty(&self) -> bool {
        SyncCache::is_empty(self)
    }
}
