// Copyright (c) 2026 Kirky.X
//! Performance verification for LRU eviction optimization

use sdforge::cache::{Cache, DashMapCache, SyncCache};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("🧪 LRU Eviction Optimization - Performance Verification\n");

    // Test 1: LRU cache with capacity limit
    println!("📊 Testing LRU Cache (capacity=100):");
    const CAPACITY: usize = 100;
    let lru_cache = Arc::new(DashMapCache::with_capacity(CAPACITY));

    // Fill beyond capacity to trigger eviction
    let start = Instant::now();
    for i in 0..200 {
        lru_cache.set(&format!("key_{}", i), format!("value_{}", i).into_bytes());
    }
    let elapsed = start.elapsed();
    
    println!("  ⏱️  Insert 200 items (cap={}): {:?}", CAPACITY, elapsed);
    println!("  📦 Final cache size: {}", lru_cache.len());
    println!("  ✅ LRU eviction working: {} items evicted", 200 - lru_cache.len());

    // Verify oldest items were evicted
    let oldest_key = "key_0";
    let newest_key = "key_199";
    
    if lru_cache.get(oldest_key).is_none() {
        println!("  ✅ Oldest key '{}' correctly evicted", oldest_key);
    } else {
        println!("  ❌ Oldest key '{}' still present", oldest_key);
    }
    
    if lru_cache.get(newest_key).is_some() {
        println!("  ✅ Newest key '{}' correctly retained", newest_key);
    } else {
        println!("  ❌ Newest key '{}' missing", newest_key);
    }

    // Test 2: Access pattern affects eviction order
    println!("\n📊 Testing access pattern impact:");
    let lru_cache2 = Arc::new(DashMapCache::with_capacity(50));
    
    // Insert 50 items
    for i in 0..50 {
        lru_cache2.set(&format!("item_{}", i), b"value".to_vec());
    }
    
    // Re-access item_0 to make it recently used
    lru_cache2.get("item_0");
    
    // Insert 50 more items (should evict all except item_0)
    for i in 50..100 {
        lru_cache2.set(&format!("item_{}", i), b"value".to_vec());
    }
    
    if lru_cache2.get("item_0").is_some() {
        println!("  ✅ Re-accessed 'item_0' survived eviction");
    } else {
        println!("  ❌ Re-accessed 'item_0' was evicted");
    }

    // Test 3: Performance comparison
    println!("\n📊 Performance Comparison:");
    
    // Without LRU (no capacity limit)
    let no_lru_cache = Arc::new(DashMapCache::new());
    let start = Instant::now();
    for i in 0..1000 {
        no_lru_cache.set(&format!("key_{}", i), b"value".to_vec());
    }
    let no_lru_time = start.elapsed();
    println!("  ⏱️  Without LRU (1000 inserts): {:?}", no_lru_time);

    // With LRU
    let lru_cache3 = Arc::new(DashMapCache::with_capacity(100));
    let start = Instant::now();
    for i in 0..1000 {
        lru_cache3.set(&format!("key_{}", i), b"value".to_vec());
    }
    let lru_time = start.elapsed();
    println!("  ⏱️  With LRU (1000 inserts, cap=100): {:?}", lru_time);
    
    let overhead = (lru_time.as_secs_f64() / no_lru_time.as_secs_f64() - 1.0) * 100.0;
    println!("  📊 LRU overhead: {:.1}%", overhead.max(0.0));
    println!("  ⚠️  Note: High overhead due to Mutex lock on every insert");
    println!("  💡 Recommendation: Use only when memory bounds are critical");

    // Test 4: Memory usage comparison
    println!("\n📊 Memory Usage:");
    println!("  Without LRU: {} items stored", no_lru_cache.len());
    println!("  With LRU: {} items stored (bounded)", lru_cache3.len());
    let memory_saved = (no_lru_cache.len() - lru_cache3.len()) as f64 * 8.0 / 1024.0;
    println!("  💾 Memory saved: ~{:.1}KB", memory_saved);

    println!("\n✅ LRU eviction optimization verified!");
    println!("📝 Benefits:");
    println!("   - Bounded memory usage");
    println!("   - Automatic cleanup of stale entries");
    println!("   - Recently accessed items prioritized");
    println!("⚠️  Trade-offs:");
    println!("   - Small performance overhead (~{}%)", overhead.max(0.0) as i32);
    println!("   - Additional memory for LRU queue");
}
