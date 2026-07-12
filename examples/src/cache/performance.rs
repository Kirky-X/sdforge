// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Caching and Performance Optimization Example
//!
//! This example demonstrates advanced caching strategies:
//! - Multi-level caching (L1/L2)
//! - Cache-aside pattern
//! - Write-through caching
//! - Cache invalidation strategies
//! - TTL-based expiration
//! - Memory-efficient storage
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --features "http cache" --example cache/performance
//! ```

use sdforge::cache::{DashMapCache, SyncCache};
use sdforge::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

// =============================================================================
// 本地 Cacheable trait
// =============================================================================
// oxcache 0.3 移除了全局 Cacheable trait（序列化职责交由调用方负责）。
// 本示例直接使用 serde_json::to_vec / from_slice 进行序列化，
// 通过 trait bound `T: Serialize + for<'de> Deserialize<'de>` 约束可缓存类型。

/// 可缓存类型约定 — 实现 Serialize + Deserialize 即可
pub trait Cacheable: Serialize + for<'de> Deserialize<'de> {}
impl<T: Serialize + for<'de> Deserialize<'de>> Cacheable for T {}

// =============================================================================
// Data Models
// =============================================================================

/// Product data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: u64,
    pub name: String,
    pub price: f64,
    pub stock: u32,
    pub category: String,
}

/// Expensive computation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationResult {
    pub data: Vec<String>,
    pub computed_at: i64,
    pub ttl_seconds: u64,
}

// =============================================================================
// Cache Layers
// =============================================================================

/// Two-level cache system
pub struct TwoLevelCache {
    l1_cache: Arc<DashMapCache>, // Fast, small L1 cache
    l2_cache: Arc<DashMapCache>, // Larger, slower L2 cache
    l1_max_size: usize,
}

impl TwoLevelCache {
    /// Create a new two-level cache
    pub fn new(l1_max_size: usize) -> Self {
        Self {
            l1_cache: Arc::new(DashMapCache::new()),
            l2_cache: Arc::new(DashMapCache::new()),
            l1_max_size,
        }
    }

    /// Get value from cache (L1 first, then L2)
    pub async fn get<T: Cacheable>(&self, key: &str) -> Option<T> {
        // Try L1 first (fastest)
        if let Some(data) = self.l1_cache.get(key) {
            return serde_json::from_slice(&data).ok();
        }

        // Try L2
        if let Some(data) = self.l2_cache.get(key) {
            // Promote to L1
            self.l1_cache.set(key, data.clone());
            return serde_json::from_slice(&data).ok();
        }

        None
    }

    /// Set value in both cache levels
    pub async fn set<T: Cacheable>(&self, key: &str, value: &T) {
        if let Ok(serialized) = serde_json::to_vec(value) {
            // Always set in L2
            self.l2_cache.set(key, serialized.clone());

            // Set in L1 if under size limit
            if self.l1_cache.len() < self.l1_max_size {
                self.l1_cache.set(key, serialized);
            }
        }
    }

    /// Invalidate in both levels
    pub async fn invalidate(&self, key: &str) {
        self.l1_cache.delete(key);
        self.l2_cache.delete(key);
    }

    /// Get statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            l1_size: self.l1_cache.len(),
            l2_size: self.l2_cache.len(),
            l1_max_size: self.l1_max_size,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub l1_size: usize,
    pub l2_size: usize,
    pub l1_max_size: usize,
}

// =============================================================================
// Cache Patterns
// =============================================================================

/// Cache-aside pattern implementation
pub struct CacheAsidePattern {
    cache: Arc<DashMapCache>,
}

impl CacheAsidePattern {
    pub fn new(cache: Arc<DashMapCache>) -> Self {
        Self { cache }
    }

    /// Get from cache or compute and cache
    pub async fn get_or_compute<F, Fut, T>(&self, key: &str, compute_fn: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
        T: Cacheable + Clone,
    {
        // Try cache first
        if let Some(data) = self.cache.get(key) {
            if let Ok(value) = serde_json::from_slice::<T>(&data) {
                return value;
            }
        }

        // Compute and cache
        let value = compute_fn().await;

        if let Ok(serialized) = serde_json::to_vec(&value) {
            self.cache.set(key, serialized);
        }

        value
    }
}

/// Write-through pattern implementation
pub struct WriteThroughPattern {
    cache: Arc<DashMapCache>,
}

impl WriteThroughPattern {
    pub fn new(cache: Arc<DashMapCache>) -> Self {
        Self { cache }
    }

    /// Write to cache (and would write to database in real app)
    pub async fn write<T: Cacheable>(&self, key: &str, value: &T) -> Result<(), String> {
        let serialized = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        self.cache.set(key, serialized);
        Ok(())
    }

    /// Delete from cache (and would delete from database in real app)
    pub async fn delete(&self, key: &str) {
        self.cache.delete(key);
    }
}

// =============================================================================
// API Endpoints
//
// NOTE: 下面的 handler 接受 `&TwoLevelCache` / `&CacheAsidePattern` 引用参数，
// 不是有效的 axum extractor，因此不使用 `#[forge]` 宏注册为 HTTP 端点。
// 它们作为业务逻辑示例，展示缓存模式的集成方式。
// =============================================================================

/// Get product with intelligent caching
///
/// Demonstrates:
/// - Two-level caching
/// - Cache promotion (L2 → L1)
/// - Serialization/deserialization
async fn get_product(id: u64, cache: &TwoLevelCache) -> Result<Product, ApiError> {
    let cache_key = format!("product:{}", id);

    // Try to get from two-level cache
    if let Some(product) = cache.get::<Product>(&cache_key).await {
        return Ok(product);
    }

    // Cache miss - fetch from "database"
    let product = fetch_product_from_database(id).await?;

    // Cache for future requests
    cache.set(&cache_key, &product).await;

    Ok(product)
}

/// Simulated database fetch (expensive operation)
async fn fetch_product_from_database(id: u64) -> Result<Product, ApiError> {
    // Simulate database delay
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Return mock product
    Ok(Product {
        id,
        name: format!("Product {}", id),
        price: 99.99 + (id as f64 * 0.01),
        stock: 100 + (id as u32 * 10),
        category: "Electronics".to_string(),
    })
}

/// Perform expensive computation with caching
///
/// Demonstrates:
/// - Cache-aside pattern
/// - TTL-based caching
/// - Expensive computation avoidance
async fn compute_analytics(
    request: AnalyticsRequest,
    cache_pattern: &CacheAsidePattern,
) -> Result<ComputationResult, ApiError> {
    let cache_key = format!(
        "analytics:{}:{}:{}",
        request.metric_type, request.start_date, request.end_date
    );

    // Use cache-aside pattern
    let result = cache_pattern
        .get_or_compute(&cache_key, || async {
            // This is the expensive computation that we want to cache
            perform_expensive_computation(&request).await
        })
        .await;

    Ok(result)
}

/// Analytics request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsRequest {
    pub metric_type: String,
    pub start_date: String,
    pub end_date: String,
    pub filters: Option<Vec<String>>,
}

/// Perform expensive computation (simulated)
async fn perform_expensive_computation(request: &AnalyticsRequest) -> ComputationResult {
    // Simulate expensive computation
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Generate mock results
    ComputationResult {
        data: vec![
            format!("Metric: {}", request.metric_type),
            format!("Period: {} to {}", request.start_date, request.end_date),
            "Result: Computed successfully".to_string(),
        ],
        computed_at: chrono::Utc::now().timestamp(),
        ttl_seconds: 3600, // Cache for 1 hour
    }
}

// =============================================================================
// Main Entry Point
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ SDForge Caching and Performance Example");
    println!("=========================================\n");

    // Initialize two-level cache
    let _two_level_cache = TwoLevelCache::new(1000); // L1 max 1000 items

    println!("✓ Two-Level Cache initialized:");
    println!("  L1 Cache: Max {} items (fast access)", 1000);
    println!("  L2 Cache: Unlimited (slower access)\n");

    // Initialize cache-aside pattern
    let _cache_aside = CacheAsidePattern::new(Arc::new(DashMapCache::new()));

    println!("✓ Cache-Aside Pattern configured\n");

    // Initialize write-through pattern
    let _write_through = WriteThroughPattern::new(Arc::new(DashMapCache::new()));

    println!("✓ Write-Through Pattern configured\n");

    // Demonstrate caching patterns
    println!("📊 Caching Strategies:\n");

    println!("1. Two-Level Cache (L1/L2):");
    println!("   - L1: Hot data, fastest access");
    println!("   - L2: Warm data, larger capacity");
    println!("   - Automatic promotion from L2 to L1\n");

    println!("2. Cache-Aside Pattern:");
    println!("   - Check cache first");
    println!("   - Compute on cache miss");
    println!("   - Store result for next time\n");

    println!("3. Write-Through Pattern:");
    println!("   - Write to cache and storage atomically");
    println!("   - Ensures cache consistency\n");

    // Print usage instructions
    println!("📖 Available Endpoints:");
    println!("  GET  /api/v1/products/:id          - Get product (cached)");
    println!("  POST /api/v1/analytics/compute     - Expensive computation (cached)\n");

    println!("⚡ Performance Benefits:");
    println!("  ✓ Reduced database queries");
    println!("  ✓ Faster response times");
    println!("  ✓ Lower computational overhead");
    println!("  ✓ Scalable architecture\n");

    println!("💡 Best Practices:");
    println!("  • Use appropriate TTL values");
    println!("  • Invalidate cache on updates");
    println!("  • Monitor cache hit rates");
    println!("  • Size L1 cache based on hot data\n");

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_two_level_cache() {
        let cache = TwoLevelCache::new(100);

        // Create test product
        let product = Product {
            id: 1,
            name: "Test Product".to_string(),
            price: 99.99,
            stock: 50,
            category: "Test".to_string(),
        };

        // Set in cache
        cache.set("product:1", &product).await;

        // Get from cache
        let retrieved: Option<Product> = cache.get("product:1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, 1);

        // Invalidate
        cache.invalidate("product:1").await;
        let retrieved: Option<Product> = cache.get("product:1").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_cache_aside_pattern() {
        let cache = CacheAsidePattern::new(Arc::new(DashMapCache::new()));

        let compute_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let compute_count_clone = compute_count.clone();

        let result = cache
            .get_or_compute("test_key", || async {
                compute_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                ComputationResult {
                    data: vec!["test".to_string()],
                    computed_at: 12345,
                    ttl_seconds: 60,
                }
            })
            .await;

        // First call should compute
        assert_eq!(result.data.len(), 1);
        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Second call should use cache
        let _result2 = cache
            .get_or_compute("test_key", || async {
                compute_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                ComputationResult {
                    data: vec!["test".to_string()],
                    computed_at: 12345,
                    ttl_seconds: 60,
                }
            })
            .await;

        // Should still be 1 (cached)
        assert_eq!(compute_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn test_serialization() {
        let product = Product {
            id: 1,
            name: "Test".to_string(),
            price: 99.99,
            stock: 50,
            category: "Test".to_string(),
        };

        let serialized = serde_json::to_vec(&product).unwrap();
        let deserialized: Product = serde_json::from_slice(&serialized).unwrap();

        assert_eq!(product.id, deserialized.id);
        assert_eq!(product.name, deserialized.name);
    }

    #[tokio::test]
    async fn test_write_through_pattern() {
        let cache = WriteThroughPattern::new(Arc::new(DashMapCache::new()));

        let product = Product {
            id: 1,
            name: "Test".to_string(),
            price: 99.99,
            stock: 50,
            category: "Test".to_string(),
        };

        // Write through cache
        cache.write("product:1", &product).await.unwrap();

        // Verify it's in cache
        let cached = cache.cache.get("product:1");
        assert!(cached.is_some());

        // Delete
        cache.delete("product:1").await;
        let cached = cache.cache.get("product:1");
        assert!(cached.is_none());
    }
}
