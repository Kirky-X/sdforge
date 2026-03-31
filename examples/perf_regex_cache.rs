// Copyright (c) 2026 Kirky.X
//! Simple performance verification for regex cache optimization

use sdforge::cache::{Cache, DashMapCache, SyncCache};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("🧪 Regex Cache Optimization - Performance Verification\n");

    // Setup: Create cache with test data
    let cache = Arc::new(DashMapCache::new());
    for i in 0..1000 {
        cache.set(&format!("user:{}", i), format!("data_{}", i).into_bytes());
        cache.set(&format!("session:{}", i), format!("session_{}", i).into_bytes());
    }
    println!("✅ Setup: Created cache with 2000 keys\n");

    // Test 1: First call (regex compilation)
    let start = Instant::now();
    let keys = cache.find_keys_by_pattern("user:*");
    let first_call = start.elapsed();
    println!("⏱️  First call (user:*):      {:?} - Found {} keys", first_call, keys.len());

    // Test 2: Second call (should use cached regex)
    let start = Instant::now();
    let keys = cache.find_keys_by_pattern("user:*");
    let second_call = start.elapsed();
    println!("⏱️  Second call (user:*):     {:?} - Found {} keys", second_call, keys.len());

    // Test 3: Different pattern (first time for this pattern)
    let start = Instant::now();
    let keys = cache.find_keys_by_pattern("session:*");
    let session_first = start.elapsed();
    println!("⏱️  First call (session:*):   {:?} - Found {} keys", session_first, keys.len());

    // Test 4: Repeat session pattern
    let start = Instant::now();
    let keys = cache.find_keys_by_pattern("session:*");
    let session_second = start.elapsed();
    println!("⏱️  Second call (session:*):  {:?} - Found {} keys", session_second, keys.len());

    // Test 5: Complex pattern
    let start = Instant::now();
    let keys = cache.find_keys_by_pattern("user:1*");
    let complex_first = start.elapsed();
    println!("⏱️  First call (user:1*):     {:?} - Found {} keys", complex_first, keys.len());

    // Test 6: Repeat complex pattern
    let start = Instant::now();
    let keys = cache.find_keys_by_pattern("user:1*");
    let complex_second = start.elapsed();
    println!("⏱️  Second call (user:1*):    {:?} - Found {} keys", complex_second, keys.len());

    // Calculate speedup
    println!("\n📊 Performance Summary:");
    println!("  user:* pattern:        {:.2}x faster (cached vs first)", 
             first_call.as_secs_f64() / second_call.as_secs_f64().max(0.0001));
    println!("  session:* pattern:     {:.2}x faster (cached vs first)", 
             session_first.as_secs_f64() / session_second.as_secs_f64().max(0.0001));
    println!("  user:1* pattern:       {:.2}x faster (cached vs first)", 
             complex_first.as_secs_f64() / complex_second.as_secs_f64().max(0.0001));

    let avg_speedup = (first_call.as_secs_f64() / second_call.as_secs_f64().max(0.0001)
                     + session_first.as_secs_f64() / session_second.as_secs_f64().max(0.0001)
                     + complex_first.as_secs_f64() / complex_second.as_secs_f64().max(0.0001)) / 3.0;
    
    println!("\n🎯 Average Speedup:       {:.2}x", avg_speedup);
    
    if avg_speedup >= 10.0 {
        println!("\n✅ SUCCESS: Optimization achieved expected performance improvement!");
    } else if avg_speedup >= 2.0 {
        println!("\n⚠️  GOOD: Optimization shows improvement, but below expected target.");
    } else {
        println!("\n❌ WARNING: Optimization may not be working as expected.");
    }
}
