// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Cache key pattern matching — correctness and scan-cost verification.
//!
//! `SyncCache::find_keys_by_pattern` matches a whole key against `*` (any run of
//! characters, including none) and `?` (exactly one character). Matching runs a
//! wildcard algorithm and compiles no regex, so there is no compiled-pattern
//! cache to warm up: a repeated call costs the same as the first call.
//!
//! The example verifies, against a 2000-key dataset:
//! 1. match correctness for prefix / wildcard / match-all patterns;
//! 2. the cost of a single call and how it grows with the number of keys —
//!    every call scans the full key index, so point lookups on hot paths
//!    should use `get` / `contains` instead.
//!
//! 历史说明：本示例原用于验证正则编译缓存，文件名保留为 `perf_regex_cache`。
//! `cache` 的模式匹配现为无正则的通配算法（见 `matches_pattern`），首/次调用
//! 已无编译差异，验证目标同步改为通配匹配本身。

use sdforge::cache::{DashMapCache, SyncCache};
use std::time::{Duration, Instant};

/// `user:{i}` 与 `session:{i}` 各写入的键数量
const KEYS_PER_PREFIX: usize = 1000;

fn main() {
    println!("🧪 Cache Key Pattern Matching - Performance Verification\n");
    println!("ℹ️  Legacy name: perf_regex_cache. find_keys_by_pattern() matches");
    println!("    keys with `*` / `?` wildcards and compiles no regex, so a");
    println!("    repeated call costs the same as the first call.\n");

    // Setup: Create cache with test data
    let cache = DashMapCache::new();
    for i in 0..KEYS_PER_PREFIX {
        cache.set(&format!("user:{}", i), format!("data_{}", i).into_bytes());
        cache.set(
            &format!("session:{}", i),
            format!("session_{}", i).into_bytes(),
        );
    }
    println!("✅ Setup: Created cache with {} keys\n", cache.len());

    // 期望命中数：user:1* 命中 user:1 / user:10..19 / user:100..199 = 111；
    // user:? 只命中单字符后缀 user:0..user:9。
    let cases: [(&str, usize, &str); 5] = [
        ("user:*", KEYS_PER_PREFIX, "user:"),
        ("session:*", KEYS_PER_PREFIX, "session:"),
        ("user:1*", 111, "user:1"),
        ("user:?", 10, "user:"),
        ("*", KEYS_PER_PREFIX * 2, ""),
    ];

    // Test 1: 匹配正确性 + 首次调用与重复调用的耗时对比
    println!("📊 Match correctness (first call / repeat call):");
    let mut all_correct = true;
    let (mut first_total, mut repeat_total) = (Duration::ZERO, Duration::ZERO);

    for (pattern, expected, expected_prefix) in cases {
        let start = Instant::now();
        let first = cache.find_keys_by_pattern(pattern);
        let first_elapsed = start.elapsed();

        let start = Instant::now();
        let repeat = cache.find_keys_by_pattern(pattern);
        let repeat_elapsed = start.elapsed();

        first_total += first_elapsed;
        repeat_total += repeat_elapsed;

        let correct = first.len() == expected
            && repeat.len() == expected
            && first.iter().all(|k| k.starts_with(expected_prefix));
        all_correct &= correct;

        println!(
            "  {} {:<12} first {:>10?} / repeat {:>10?} - {} keys (expected {})",
            if correct { "✅" } else { "❌" },
            pattern,
            first_elapsed,
            repeat_elapsed,
            first.len(),
            expected
        );
    }

    println!(
        "\n  📈 First vs repeat: {:.2}x — expected ≈1.0x: without regex compilation",
        first_total.as_secs_f64() / repeat_total.as_secs_f64().max(0.000_001)
    );
    println!("     there is nothing to amortize, the repeat call re-scans every key.");

    // Test 2: 单次调用成本随键数增长。
    // 必须用 with_capacity(size)：DashMapCache::new() 默认容量 10_000，超出后
    // 后端会 FIFO 驱逐旧键，扫描结果将少于写入键数。
    println!("\n📊 Single-call cost vs dataset size (pattern \"key:*\"):");
    for size in [1_000usize, 5_000, 20_000] {
        let sized = DashMapCache::with_capacity(size);
        for i in 0..size {
            sized.set(&format!("key:{}", i), b"value".to_vec());
        }
        // 预热一次，排除首次分配的影响
        let _ = sized.find_keys_by_pattern("key:*");

        let start = Instant::now();
        let matched = sized.find_keys_by_pattern("key:*");
        let elapsed = start.elapsed();

        let correct = matched.len() == size;
        all_correct &= correct;

        println!(
            "  {} {:>6} keys: {:>10?} - {} matches",
            if correct { "✅" } else { "❌" },
            size,
            elapsed,
            matched.len()
        );
    }

    if all_correct {
        println!("\n✅ SUCCESS: wildcard matching verified (correct keys, full-scan cost).");
        println!("💡 find_keys_by_pattern() / invalidate() scan every key: O(total keys) per");
        println!("   call. Use get() / contains() for point lookups on hot paths.");
    } else {
        println!("\n❌ WARNING: pattern matching returned unexpected results.");
    }
}
