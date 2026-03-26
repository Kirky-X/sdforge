// Copyright (c) 2026 Kirky.X
//!
//! sdforge::DashMapCache 到 oxcache 的桥接实现
//!
//! 当启用 `cache` feature 时，sdforge 的 DashMapCache 实现 `sdforge::cache::SyncCache` trait，
//! 可以与 oxcache 的 CacheBackend 接口兼容使用。
//!
//! DashMapCache 已经在 traits.rs 中实现了 SyncCache trait，
//! 此文件用于未来可能的 oxcache 集成扩展。
