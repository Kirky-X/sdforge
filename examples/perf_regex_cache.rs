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
use sdforge::i18n::t;
use std::time::{Duration, Instant};

/// `user:{i}` 与 `session:{i}` 各写入的键数量
const KEYS_PER_PREFIX: usize = 1000;

fn main() {
    println!("{}", t("example-perf-regex-title", &[]));
    println!();
    println!("{}", t("example-perf-regex-legacy-note", &[]));
    println!();

    // Setup: Create cache with test data
    let cache = DashMapCache::new();
    for i in 0..KEYS_PER_PREFIX {
        cache.set(&format!("user:{}", i), format!("data_{}", i).into_bytes());
        cache.set(
            &format!("session:{}", i),
            format!("session_{}", i).into_bytes(),
        );
    }
    println!(
        "{}",
        t(
            "example-perf-regex-setup",
            &[("count", cache.len().to_string())]
        )
    );
    println!();

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
    println!("{}", t("example-perf-regex-correctness-title", &[]));
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
            "  {} {}",
            if correct { "✅" } else { "❌" },
            t(
                "example-perf-regex-match-row",
                &[
                    ("pattern", format!("{pattern:<12}")),
                    ("first", format!("{first_elapsed:>10?}")),
                    ("repeat", format!("{repeat_elapsed:>10?}")),
                    ("keys", first.len().to_string()),
                    ("expected", expected.to_string()),
                ]
            )
        );
    }

    println!(
        "\n  {}",
        t(
            "example-perf-regex-first-vs-repeat",
            &[(
                "ratio",
                format!(
                    "{:.2}",
                    first_total.as_secs_f64() / repeat_total.as_secs_f64().max(0.000_001)
                ),
            )]
        )
    );

    // 必须 with_capacity(size)：默认容量 10_000 会 FIFO 驱逐旧键，扫描结果将少于写入键数。
    println!("\n{}", t("example-perf-regex-scale-title", &[]));
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
            "  {} {}",
            if correct { "✅" } else { "❌" },
            t(
                "example-perf-regex-scale-row",
                &[
                    ("size", format!("{size:>6}")),
                    ("elapsed", format!("{elapsed:>10?}")),
                    ("matches", matched.len().to_string()),
                ]
            )
        );
    }

    if all_correct {
        println!("\n{}", t("example-perf-regex-success", &[]));
        println!("{}", t("example-perf-regex-tip", &[]));
    } else {
        println!("\n{}", t("example-perf-regex-warning", &[]));
    }
}
