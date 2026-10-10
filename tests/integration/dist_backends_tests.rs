// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 分布式限流与 L2 缓存的真实 Redis 集成测试。
//!
//! **后端解析顺序**：
//! 1. `SDFORGE_TEST_REDIS_URL`（如 `redis://127.0.0.1:6379/0`）——CI/容器
//!    环境显式指定；
//! 2. 未设置且 PATH 上有 `redis-server` 时，拉起一次性本地实例（随机回环
//!    端口、关闭持久化），随用例结束终止——沙箱无外部 Redis 时把「零断言
//!    skip」变成真实覆盖；
//! 3. 两者都没有时才打印提示跳过（环境限制而非代码缺陷）。
//!
//! Feature requirement: `cargo test --features "ratelimit-dist,cache-l2" --test dist_backends_tests`

#![cfg(all(feature = "ratelimit-dist", feature = "cache-l2"))]

use sdforge::SyncCache;
use sdforge::security::ratelimit::RateLimiter;
use std::sync::Arc;
use std::time::Duration;

/// 一次测试运行使用的 Redis 目标。
///
/// `child`/`dir` 仅为本地拉起的一次性实例所有：Drop 时终止子进程并清理
/// 临时目录。外部 URL（环境变量）不携带子进程。
struct RedisTarget {
    url: String,
    child: Option<std::process::Child>,
    dir: Option<std::path::PathBuf>,
}

impl Drop for RedisTarget {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(dir) = self.dir.take() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// Resolve a real Redis backend (env var → local `redis-server` → `None`).
async fn redis_target() -> Option<RedisTarget> {
    if let Ok(url) = std::env::var("SDFORGE_TEST_REDIS_URL")
        && !url.is_empty()
    {
        return Some(RedisTarget {
            url,
            child: None,
            dir: None,
        });
    }
    spawn_local_redis().await
}

/// 拉起一次性 `redis-server`：随机回环端口、`--save ""` 关闭持久化。
/// 就绪探测失败（binary 不可用/参数错误）返回 `None` 交由用例跳过。
async fn spawn_local_redis() -> Option<RedisTarget> {
    let port = {
        // 绑定 :0 取空闲端口后立即释放；测试环境可接受极小的重绑定竞态。
        let listener = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
        listener.local_addr().ok()?.port()
    };
    let dir = std::env::temp_dir().join(format!("sdforge-redis-{}-{}", std::process::id(), port));
    std::fs::create_dir_all(&dir).ok()?;

    let child = std::process::Command::new("redis-server")
        .arg("--port")
        .arg(port.to_string())
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--save")
        .arg("")
        .arg("--appendonly")
        .arg("no")
        .arg("--daemonize")
        .arg("no")
        .arg("--dir")
        .arg(&dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    let child = match child {
        Ok(child) => child,
        Err(_) => {
            // 无 redis-server 可执行文件：清掉已建临时目录后再走 skip 路径。
            let _ = std::fs::remove_dir_all(&dir);
            return None;
        }
    };

    let mut target = RedisTarget {
        url: format!("redis://127.0.0.1:{port}/0"),
        child: Some(child),
        dir: Some(dir),
    };
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            // 本地回环一次性实例无 TLS，而 oxcache 默认拒绝明文 redis://。
            // 仅对「本测试自行拉起的 loopback 实例」放行；外部 URL 不代设，
            // 操作者须自行声明 OXCACHE_ALLOW_INSECURE_REDIS。
            unsafe {
                std::env::set_var("OXCACHE_ALLOW_INSECURE_REDIS", "I_UNDERSTAND_THE_RISKS");
            }
            return Some(target);
        }
        if let Some(child) = target.child.as_mut()
            && matches!(child.try_wait(), Ok(Some(_)))
        {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    None
}

/// 真实 Redis 上的多副本配额一致性：两个适配器共享 RedisDistributedLimiter
/// 计数后端，配额全局一致。
#[tokio::test]
async fn redis_distributed_limiter_global_quota() {
    let Some(redis) = redis_target().await else {
        eprintln!("skip: no SDFORGE_TEST_REDIS_URL and no local redis-server available");
        return;
    };
    let url = redis.url.clone();

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
///
/// 同步面（`SyncCache`）经 `block_in_place` + `block_on` 桥接，按模块文档
/// 约束只能在**多线程 runtime** 上使用（`worker_threads >= 2`：单 worker
/// 上 `block_in_place` 会占满唯一 worker，I/O 桥接可能挂起）——`#[tokio::test]`
/// 默认 current_thread，必须显式 multi_thread，否则同步面按 fail-open 恒 miss。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redis_l2_cache_roundtrip() {
    let Some(redis) = redis_target().await else {
        eprintln!("skip: no SDFORGE_TEST_REDIS_URL and no local redis-server available");
        return;
    };
    let url = redis.url.clone();

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
    let Some(redis) = redis_target().await else {
        eprintln!("skip: no SDFORGE_TEST_REDIS_URL and no local redis-server available");
        return;
    };
    let url = redis.url.clone();

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
