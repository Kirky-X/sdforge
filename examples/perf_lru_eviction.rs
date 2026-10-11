// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Capacity-based (FIFO) eviction — correctness and write-cost verification.
//!
//! `DashMapCache::with_capacity(n)` bounds the cache to `n` entries. The backend
//! evicts by **insertion order** (FIFO): when a write pushes the entry count past
//! the capacity, the oldest inserted keys leave first. A `get` never refreshes an
//! entry's position, so a read does not protect a key from eviction; a `set` does.
//! `DashMapCache::new()` is bounded too — at the backend default capacity 10_000.
//!
//! The example verifies:
//! 1. the capacity bound and insertion-order eviction;
//! 2. read vs write effect on eviction order (the LRU assumption trap);
//! 3. the write cost of a bounded cache compared with the default-capacity one.
//!
//! 历史说明：本示例原按「访问序（LRU）驱逐」验证，文件名保留为
//! `perf_lru_eviction`。`DashMapCache` 的后端（oxcache DashMap 后端）按插入序
//! 驱逐，不按访问新近度；需要访问序驱逐请改用 oxcache 提供 LRU/TinyLFU 的
//! Moka 后端（`MemoryBackendType::Moka`）。

use sdforge::cache::{DashMapCache, SyncCache};
use sdforge::i18n::t;
use std::time::Instant;

/// 自检标记：通过打印 ✅，失败打印 ❌
fn mark(ok: bool) -> &'static str {
    if ok { "✅" } else { "❌" }
}

fn main() {
    println!("{}", t("example-perf-lru-title", &[]));
    println!();
    println!("{}", t("example-perf-lru-legacy-note", &[]));
    println!();

    let mut all_correct = true;

    // Test 1: 容量上界 + 插入序驱逐
    println!("{}", t("example-perf-lru-capacity-title", &[]));
    const CAPACITY: usize = 100;
    let bounded = DashMapCache::with_capacity(CAPACITY);
    for i in 0..CAPACITY * 2 {
        bounded.set(&format!("key_{}", i), format!("value_{}", i).into_bytes());
    }

    let size_ok = bounded.len() == CAPACITY;
    let oldest_evicted = bounded.get("key_0").is_none();
    let boundary_evicted = bounded.get("key_99").is_none();
    let boundary_kept = bounded.get("key_100").is_some();
    let newest_kept = bounded.get("key_199").is_some();
    all_correct &= size_ok && oldest_evicted && boundary_evicted && boundary_kept && newest_kept;

    println!(
        "  {} {}",
        mark(size_ok),
        t(
            "example-perf-lru-final-size",
            &[
                ("size", bounded.len().to_string()),
                ("capacity", CAPACITY.to_string()),
            ]
        )
    );
    println!(
        "  {} {}",
        mark(oldest_evicted && boundary_evicted),
        t(
            "example-perf-lru-oldest-evicted",
            &[
                ("first", bounded.get("key_0").is_some().to_string()),
                ("last", bounded.get("key_99").is_some().to_string()),
            ]
        )
    );
    println!(
        "  {} {}",
        mark(boundary_kept && newest_kept),
        t(
            "example-perf-lru-newest-kept",
            &[
                ("first", bounded.get("key_100").is_some().to_string()),
                ("last", bounded.get("key_199").is_some().to_string()),
            ]
        )
    );

    // Test 2: 读取不续期，写入续期
    println!("\n{}", t("example-perf-lru-rw-title", &[]));
    const SMALL: usize = 50;
    // 两个分支都追加 SMALL/2 个新键：不足以绕回队尾，仅淘汰最旧的 25 个
    const EXTRA: usize = SMALL / 2;

    // 2a: 只读取——FIFO 位置不变，item_0 仍是最旧条目
    let read_case = DashMapCache::with_capacity(SMALL);
    for i in 0..SMALL {
        read_case.set(&format!("item_{}", i), b"value".to_vec());
    }
    let _ = read_case.get("item_0");
    for i in SMALL..SMALL + EXTRA {
        read_case.set(&format!("item_{}", i), b"value".to_vec());
    }
    let read_evicted = read_case.get("item_0").is_none();
    all_correct &= read_evicted;
    let read_outcome = if read_evicted {
        t("example-perf-lru-evicted-read", &[])
    } else {
        t("example-perf-lru-still-cached-unexpected", &[])
    };
    println!(
        "  {} {}",
        mark(read_evicted),
        t(
            "example-perf-lru-read-case",
            &[("inserts", EXTRA.to_string()), ("outcome", read_outcome),]
        )
    );

    // 2b: 重新写入——移到队尾，可抵抗后续驱逐
    let write_case = DashMapCache::with_capacity(SMALL);
    for i in 0..SMALL {
        write_case.set(&format!("item_{}", i), b"value".to_vec());
    }
    write_case.set("item_0", b"refreshed".to_vec());
    for i in SMALL..SMALL + EXTRA {
        write_case.set(&format!("item_{}", i), b"value".to_vec());
    }
    let write_refreshed = write_case.get("item_0").as_deref() == Some(b"refreshed".as_slice());
    all_correct &= write_refreshed;
    let write_outcome = if write_refreshed {
        t("example-perf-lru-still-cached-write", &[])
    } else {
        t("example-perf-lru-evicted-unexpected", &[])
    };
    println!(
        "  {} {}",
        mark(write_refreshed),
        t(
            "example-perf-lru-write-case",
            &[("inserts", EXTRA.to_string()), ("outcome", write_outcome),]
        )
    );

    // Test 3: 写入成本（信息性测量，不作为通过条件）
    println!("\n{}", t("example-perf-lru-cost-title", &[]));
    let default_cap = DashMapCache::new();
    let start = Instant::now();
    for i in 0..1000 {
        default_cap.set(&format!("key_{}", i), b"value".to_vec());
    }
    let default_time = start.elapsed();

    let low_cap = DashMapCache::with_capacity(100);
    let start = Instant::now();
    for i in 0..1000 {
        low_cap.set(&format!("key_{}", i), b"value".to_vec());
    }
    let low_cap_time = start.elapsed();

    println!(
        "  {}",
        t(
            "example-perf-lru-default-inserts",
            &[
                ("elapsed", format!("{default_time:?}")),
                ("entries", default_cap.len().to_string()),
            ]
        )
    );
    println!(
        "  {}",
        t(
            "example-perf-lru-lowcap-inserts",
            &[
                ("elapsed", format!("{low_cap_time:?}")),
                ("entries", low_cap.len().to_string()),
            ]
        )
    );
    println!(
        "  {}",
        t(
            "example-perf-lru-ratio",
            &[(
                "ratio",
                format!(
                    "{:.2}",
                    low_cap_time.as_secs_f64() / default_time.as_secs_f64().max(0.000_001)
                ),
            )]
        )
    );

    if all_correct {
        println!("\n{}", t("example-perf-lru-success", &[]));
        println!("{}", t("example-perf-lru-tip", &[]));
    } else {
        println!("\n{}", t("example-perf-lru-warning", &[]));
    }
}
