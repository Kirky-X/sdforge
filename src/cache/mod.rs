// Copyright (c) 2026 Kirky.X
//!
//! 同步缓存抽象 — 用于功能组件的同步存储需求
//!
//! 与 oxcache 的异步 CacheBackend 对应，提供同步接口。
//! 底层实现：
//! - `DashMapCache`：基于 DashMap 的内存缓存（sdforge 自实现）
//! - `oxcache::DashMapCache`：来自 oxcache 库的同步缓存实现
//!
//! # 架构
//!
//! 功能组件（AppRateLimiter、AppApiKeyAuth 等）依赖 `Arc<dyn SyncCache>`，
//! 不直接依赖 DashMap。允许注入不同实现：
//!
//! # Example
//!
//! ```rust
//! use sdforge::cache::{SyncCache, DashMapCache};
//! use std::sync::Arc;
//!
//! let cache = Arc::new(DashMapCache::default());
//! cache.set("key", b"value".to_vec());
//! assert!(cache.get("key").is_some());
//! ```
//!
//! # 与 oxcache 的桥接
//!
//! 当启用 `cache` feature 时，sdforge 的 DashMapCache 也实现了 `oxcache::sync::SyncCache`，
//! 可直接传递给需要 oxcache SyncCache 的组件。

pub mod dashmap;

pub use dashmap::DashMapCache;
pub use traits::{SharedCache, SyncCache};

pub mod traits;

// oxcache SyncCache 桥接
#[cfg(feature = "cache")]
mod oxcache_bridge;
