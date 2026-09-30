// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 分布式限流与 L2 缓存的真实 Redis 集成测试。
//!
//! **门控跳过**：未设置 `SDFORGE_TEST_REDIS_URL`（如
//! `redis://127.0.0.1:6379/0`）时全部用例打印提示后直接通过——沙箱/CI
//! 无 Redis 是环境限制而非代码缺陷。设置变量后走真实 Redis 全链路。
//!
//! Feature requirement: `cargo test --features "ratelimit-dist,cache-l2" --test dist_backends_tests`
//! （真实后端用例另需 `SDFORGE_TEST_REDIS_URL`）。

#![cfg(all(feature = "ratelimit-dist", feature = "cache-l2"))]

use sdforge::SyncCache;
use sdforge::security::ratelimit::RateLimiter;
use std::sync::Arc;
use std::time::Duration;

fn redis_url() -> Option<String> {
    std::env::var("SDFORGE_TEST_REDIS_URL")
        .ok()
        .filter(|u| !u.is_empty())
}

/// 真实 Redis 上的多副本配额一致性：两个适配器共享 RedisDistributedLimiter
/// 计数后端，配额全局一致。
#[tokio::test]
async fn redis_distributed_limiter_global_quota() {
    let Some(url) = redis_url() else {
        eprintln!("skip: SDFORGE_TEST_REDIS_URL not set (no Redis in sandbox/CI)");
        return;
    };

    let backend = oxcache::RedisBackend::builder()
        .connection_string(&url)
        .connection_timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("redis backend build");
    let cache = oxcache::Cache::<String, String>::builder()
        .backend_arc(Arc::new(backend))
        .build_sync()
        .expect("cache build");
    let counter = Arc::new(limiteron::limiters::RedisDistributedLimiter::new(
        cache, 3, 60_000,
    ));

    let config = sdforge::security::ratelimit::dist::DistributedRateLimitConfig::new(
        3,
        Duration::from_secs(60),
    )
    .with_key_prefix(format!("sdforge:test:{}:", std::process::id()));
    let replica_a = sdforge::security::ratelimit::dist::DistributedRateLimiter::new(
        Arc::clone(&counter),
        config.clone(),
    );
    let replica_b = sdforge::security::ratelimit::dist::DistributedRateLimiter::new(
        Arc::clone(&counter),
        config,
    );

    let key = format!(
        "quota-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    );
    for _ in 0..3 {
        assert!(replica_a.check(&key).await.is_ok(), "first three allowed");
    }
    assert!(
        replica_b.check(&key).await.is_err(),
        "fourth request across replicas must be throttled"
    );
}

/// 真实 Redis 上的 L2 缓存读写/前缀/删除全链路。
#[tokio::test]
async fn redis_l2_cache_roundtrip() {
    let Some(url) = redis_url() else {
        eprintln!("skip: SDFORGE_TEST_REDIS_URL not set (no Redis in sandbox/CI)");
        return;
    };

    let config = sdforge::cache::l2::RedisL2CacheConfig::new(url)
        .with_key_prefix(format!("sdforge:test:{}:l2:", std::process::id()))
        .with_default_ttl(Duration::from_secs(60));
    let l2 = sdforge::cache::l2::RedisL2Cache::connect(&config)
        .await
        .expect("redis L2 connect");

    let payload = b"cached-response-body".to_vec();
    assert!(!l2.contains("roundtrip"), "fresh key must miss");
    l2.set("roundtrip", payload.clone());
    assert_eq!(
        l2.get("roundtrip"),
        Some(payload),
        "set then get must round-trip"
    );
    assert!(l2.contains("roundtrip"));
    assert!(l2.delete("roundtrip"), "delete must succeed");
    assert!(!l2.contains("roundtrip"), "deleted key must miss");
}

/// 真实 Redis 上的 L2 **异步消费路径**全链路（tower/axum 异步中间件热路径
/// 专用面）：异步读写/存在性/删除与同步面语义一致，且不依赖
/// `block_in_place`（current-thread runtime 可用——用 current_thread
/// runtime 驱动即为回归证明：同步面在该 runtime 上恒 miss）。
#[tokio::test(flavor = "current_thread")]
async fn redis_l2_cache_async_roundtrip_on_current_thread_runtime() {
    let Some(url) = redis_url() else {
        eprintln!("skip: SDFORGE_TEST_REDIS_URL not set (no Redis in sandbox/CI)");
        return;
    };

    let config = sdforge::cache::l2::RedisL2CacheConfig::new(url)
        .with_key_prefix(format!("sdforge:test:{}:l2a:", std::process::id()))
        .with_default_ttl(Duration::from_secs(60));
    let l2 = sdforge::cache::l2::RedisL2Cache::connect(&config)
        .await
        .expect("redis L2 connect");

    let payload = b"async-cached-body".to_vec();
    assert!(
        !l2.contains_async("aroundtrip").await,
        "fresh key must miss on the async face"
    );
    l2.set_async("aroundtrip", payload.clone()).await;
    assert_eq!(
        l2.get_async("aroundtrip").await,
        Some(payload),
        "async set then get must round-trip"
    );
    assert!(l2.contains_async("aroundtrip").await);
    assert!(l2.delete_async("aroundtrip").await, "async delete succeeds");
    assert!(!l2.contains_async("aroundtrip").await);
}
