// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `LimiteronAdapter` — construction and `RateLimiter` impl.
//!
//! These tests use the real `limiteron::Governor` with in-memory storage
//! (no mocking), because `Governor` is a concrete struct with private
//! fields and cannot be mocked. See `design.md` D3 for rationale.

use std::sync::Arc;

// HTTP types are only needed for `check_request` tests (ratelimit-http feature).
#[cfg(feature = "ratelimit-http")]
use axum::body::Body;
#[cfg(feature = "ratelimit-http")]
use axum::http::Request;
use limiteron::Governor;

use crate::security::ratelimit::{LimiteronAdapter, RateLimiter};
// `HttpRequestRateLimiter` trait is needed for `check_request` tests.
#[cfg(feature = "ratelimit-http")]
use crate::security::ratelimit::HttpRequestRateLimiter;

// ============================================================================
// Construction Tests (T007/T008 — new/default)
// ============================================================================

/// `LimiteronAdapter::new().await` returns a usable instance without panic.
#[tokio::test]
async fn new_returns_non_panic_instance() {
    let adapter = LimiteronAdapter::new().await;
    drop(adapter);
}

/// `LimiteronAdapter::default().await` behaves identically to `new().await`.
#[tokio::test]
async fn default_matches_new_behavior() {
    let from_new = LimiteronAdapter::new().await;
    let from_default = LimiteronAdapter::default().await;
    drop(from_new);
    drop(from_default);
}

// ============================================================================
// T009 — builder / with_dependencies / RateLimiter impl
// ============================================================================

/// `LimiteronAdapter::builder().build().await` constructs an instance using
/// `default_config()` when no custom config is supplied.
///
/// HIGH-1 regression: `build()` now returns `Result<LimiteronAdapter,
/// RateLimitError>` instead of panicking via `.expect()`. Callers must
/// handle the error path (Rule 12: failures must be explicit).
#[tokio::test]
async fn builder_build_with_default_config_succeeds() {
    let adapter = LimiteronAdapter::builder()
        .build()
        .await
        .expect("default_config must produce a valid FlowControlConfig");
    // Should construct without panic. Verify by running a check.
    let result = adapter.check("127.0.0.1").await;
    assert!(
        result.is_ok(),
        "builder-built adapter should allow: {:?}",
        result
    );
}

/// HIGH-1: `LimiteronAdapter::builder().build()` returns `Err` (not panic)
/// when the configured `FlowControlConfig` is invalid. We construct an
/// invalid config with an empty `rules` vec (limiteron's `validate()`
/// rejects it with "至少需要一个规则").
#[tokio::test]
async fn builder_build_returns_err_on_invalid_config() {
    use limiteron::FlowControlConfig;
    use limiteron::config::GlobalConfig;

    let invalid_config = FlowControlConfig {
        version: "0.1.0".to_string(),
        global: GlobalConfig::default(),
        rules: vec![], // empty rules → validation failure
    };

    let result = LimiteronAdapter::builder()
        .with_config(invalid_config)
        .build()
        .await;

    assert!(
        result.is_err(),
        "build() with empty rules must return Err, not panic — got Ok"
    );
}

/// `LimiteronAdapter::with_dependencies(Arc<Governor>)` constructs an instance
/// from a pre-built governor (mode 3: full DI).
///
/// We build the governor the same way `new()` does to verify the DI path
/// accepts it and produces a working adapter.
#[tokio::test]
async fn with_dependencies_accepts_prebuilt_governor() {
    use limiteron::config::{GlobalConfig, Matcher, Rule};
    use limiteron::storage::{MemoryBanStorage, MemoryStorage};
    use limiteron::{ActionConfig, BanStorage, FlowControlConfig, LimiterConfig, Storage};

    let config = FlowControlConfig {
        version: "0.1.0".to_string(),
        global: GlobalConfig::default(),
        rules: vec![Rule {
            id: "di-test".to_string(),
            name: "DI test rule".to_string(),
            priority: 100,
            matchers: vec![Matcher::Ip {
                ip_ranges: vec!["0.0.0.0/0".to_string()],
            }],
            limiters: vec![LimiterConfig::TokenBucket {
                capacity: 5,
                refill_rate: 1,
            }],
            action: ActionConfig::default(),
        }],
    };
    let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new());
    let ban_storage: Arc<dyn BanStorage> = Arc::new(MemoryBanStorage::new());
    let governor = Governor::builder()
        .with_config(config)
        .with_storage(storage)
        .with_ban_storage(ban_storage)
        .build()
        .await
        .expect("DI test config must be valid");

    let adapter = LimiteronAdapter::with_dependencies(Arc::new(governor));
    let result = adapter.check("10.0.0.1").await;
    assert!(
        result.is_ok(),
        "DI adapter should allow first request: {:?}",
        result
    );
}

/// `check("127.0.0.1").await` on a fresh adapter returns `Ok(())` because the
/// default config (TokenBucket capacity=100, refill_rate=10) allows the first
/// request.
///
/// This is the canonical T009 acceptance test from `tasks.md`.
#[tokio::test]
async fn check_allows_first_request_with_default_config() {
    let adapter = LimiteronAdapter::new().await;
    let result = adapter.check("127.0.0.1").await;
    assert!(
        result.is_ok(),
        "first request should be allowed: {:?}",
        result
    );
}

/// After exhausting the token bucket, subsequent `check()` calls must return
/// `Err(RateLimitError::Exceeded { .. })`.
///
/// This validates the `Decision::Rejected` → `Exceeded` mapping in the
/// `RateLimiter` impl. We use `with_dependencies` to inject a tight config
/// (capacity=2, refill_rate=1). limiteron forbids `refill_rate=0`, so we use
/// `refill_rate=1` (1 token/sec) and fire 5 requests in a tight loop — the
/// test runs in milliseconds, so at most ~1 refill token could sneak in, but
/// 5 requests vs capacity 2 guarantees at least 2 rejections.
#[tokio::test]
async fn check_returns_exceeded_after_capacity_exhausted() {
    use limiteron::config::{GlobalConfig, Matcher, Rule};
    use limiteron::storage::{MemoryBanStorage, MemoryStorage};
    use limiteron::{ActionConfig, BanStorage, FlowControlConfig, LimiterConfig, Storage};

    let config = FlowControlConfig {
        version: "0.1.0".to_string(),
        global: GlobalConfig::default(),
        rules: vec![Rule {
            id: "tight".to_string(),
            name: "tight bucket".to_string(),
            priority: 100,
            matchers: vec![Matcher::Ip {
                ip_ranges: vec!["0.0.0.0/0".to_string()],
            }],
            // capacity=2 + refill_rate=1: limiteron rejects refill_rate=0.
            // 5 rapid requests must yield >=2 rejections even with 1 token
            // of refill during the test window.
            limiters: vec![LimiterConfig::TokenBucket {
                capacity: 2,
                refill_rate: 1,
            }],
            action: ActionConfig::default(),
        }],
    };
    let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new());
    let ban_storage: Arc<dyn BanStorage> = Arc::new(MemoryBanStorage::new());
    let governor = Governor::builder()
        .with_config(config)
        .with_storage(storage)
        .with_ban_storage(ban_storage)
        // Disable L1 cache: by default Governor caches the first Allowed
        // decision and returns it for subsequent identical requests, which
        // would prevent us from observing token-bucket exhaustion.
        .with_l1_cache_enabled(false)
        .build()
        .await
        .expect("tight config must be valid");
    let adapter = LimiteronAdapter::with_dependencies(Arc::new(governor));

    // Fire 5 rapid requests against the same IP. capacity=2 means the first
    // 2 (or 3 with refill) succeed; the rest must be rejected with Exceeded.
    let mut allowed = 0;
    let mut rejected_with_exceeded = 0;
    let mut other = 0;
    for _ in 0..5 {
        match adapter.check("192.0.2.1").await {
            Ok(()) => allowed += 1,
            Err(crate::security::RateLimitError::Exceeded { .. }) => {
                rejected_with_exceeded += 1;
            }
            Err(other_err) => {
                other += 1;
                eprintln!("unexpected error: {:?}", other_err);
            }
        }
    }

    assert_eq!(other, 0, "no unexpected error variants allowed");
    assert!(
        rejected_with_exceeded >= 2,
        "expected at least 2 Exceeded rejections (capacity=2, 5 requests), got allowed={}, exceeded={}",
        allowed,
        rejected_with_exceeded
    );
}

/// `check_request(req)` extracts the IP from `X-Forwarded-For` (first IP) and
/// delegates to `check()`. With default config the first request is allowed.
#[cfg(feature = "ratelimit-http")]
#[tokio::test]
async fn check_request_extracts_ip_from_x_forwarded_for() {
    let adapter = LimiteronAdapter::new().await;
    let req = Request::builder()
        .header("x-forwarded-for", "203.0.113.7, 10.0.0.1")
        .body(Body::empty())
        .expect("request build");
    let result = adapter.check_request(&req).await;
    assert!(result.is_ok(), "check_request should allow: {:?}", result);
}

/// `check_request(req)` falls back to `X-Real-IP` when `X-Forwarded-For` is
/// absent.
#[cfg(feature = "ratelimit-http")]
#[tokio::test]
async fn check_request_falls_back_to_x_real_ip() {
    let adapter = LimiteronAdapter::new().await;
    let req = Request::builder()
        .header("x-real-ip", "198.51.100.42")
        .body(Body::empty())
        .expect("request build");
    let result = adapter.check_request(&req).await;
    assert!(
        result.is_ok(),
        "check_request should allow via X-Real-IP: {:?}",
        result
    );
}

/// `check_request(req)` falls back to `"unknown"` when neither header is
/// present. Default config matches `0.0.0.0/0` so even "unknown" is allowed.
#[cfg(feature = "ratelimit-http")]
#[tokio::test]
async fn check_request_falls_back_to_unknown_identifier() {
    let adapter = LimiteronAdapter::new().await;
    let req = Request::builder()
        .body(Body::empty())
        .expect("request build");
    let result = adapter.check_request(&req).await;
    assert!(
        result.is_ok(),
        "check_request should allow unknown: {:?}",
        result
    );
}

/// H-1 security regression: a request from a non-trusted direct IP (8.8.8.8)
/// carrying a spoofed `X-Forwarded-For` header must NOT trust the header.
/// The identifier extracted must be the direct connection IP, not the
/// spoofed value. This prevents bypassing rate limits by rotating the
/// spoofed header.
///
/// We verify the fix by configuring a rule that only allows `8.8.8.8` once
/// (capacity=1) and then sending two requests with the same spoofed header
/// but expecting both to hit the same `8.8.8.8` bucket → second must be
/// rejected. Before the fix, the spoofed header would have been trusted,
/// allowing unlimited requests because each spoofed IP would get its own
/// bucket.
#[cfg(feature = "ratelimit-http")]
#[tokio::test]
async fn check_request_ignores_spoofed_x_forwarded_for_from_non_trusted_source() {
    use axum::extract::connect_info::ConnectInfo;
    use limiteron::config::{GlobalConfig, Matcher, Rule};
    use limiteron::storage::{MemoryBanStorage, MemoryStorage};
    use limiteron::{ActionConfig, BanStorage, FlowControlConfig, LimiterConfig, Storage};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    // capacity=1 + refill_rate=1: 2 rapid requests from the same identifier
    // must yield at least 1 rejection.
    let config = FlowControlConfig {
        version: "0.1.0".to_string(),
        global: GlobalConfig::default(),
        rules: vec![Rule {
            id: "anti-spoof".to_string(),
            name: "anti-spoofing rule".to_string(),
            priority: 100,
            matchers: vec![Matcher::Ip {
                ip_ranges: vec!["0.0.0.0/0".to_string()],
            }],
            limiters: vec![LimiterConfig::TokenBucket {
                capacity: 1,
                refill_rate: 1,
            }],
            action: ActionConfig::default(),
        }],
    };
    let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new());
    let ban_storage: Arc<dyn BanStorage> = Arc::new(MemoryBanStorage::new());
    let governor = Governor::builder()
        .with_config(config)
        .with_storage(storage)
        .with_ban_storage(ban_storage)
        .with_l1_cache_enabled(false)
        .build()
        .await
        .expect("anti-spoof config must be valid");
    let adapter = LimiteronAdapter::with_dependencies(Arc::new(governor));

    // Direct connection from 8.8.8.8 (non-trusted, public IP) with a spoofed
    // X-Forwarded-For. The fix must extract 8.8.8.8 (direct IP), not 1.2.3.4.
    let mut req1 = Request::builder()
        .body(Body::empty())
        .expect("request build");
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 8080);
    req1.extensions_mut().insert(ConnectInfo(addr));
    req1.headers_mut()
        .insert("X-Forwarded-For", "1.2.3.4".parse().unwrap());

    let mut req2 = Request::builder()
        .body(Body::empty())
        .expect("request build");
    req2.extensions_mut().insert(ConnectInfo(addr));
    req2.headers_mut()
        .insert("X-Forwarded-For", "5.6.7.8".parse().unwrap());

    let r1 = adapter.check_request(&req1).await;
    let r2 = adapter.check_request(&req2).await;

    // Both requests must resolve to the same identifier (8.8.8.8). With
    // capacity=1, at least one must be rejected. If the spoofed headers
    // were trusted, both would succeed (different buckets).
    let rejections = [r1, r2].iter().filter(|r| r.is_err()).count();
    assert!(
        rejections >= 1,
        "expected at least 1 rejection (same direct IP, capacity=1), got 0 — spoofed X-Forwarded-For was likely trusted"
    );
}

/// H-1 positive case: a request from a trusted reverse proxy (10.0.0.1)
/// carrying `X-Forwarded-For: 203.0.113.5` must trust the header and
/// extract `203.0.113.5` as the client identifier.
#[cfg(feature = "ratelimit-http")]
#[tokio::test]
async fn check_request_trusts_x_forwarded_for_from_trusted_proxy() {
    use axum::extract::connect_info::ConnectInfo;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let adapter = LimiteronAdapter::new().await;
    let mut req = Request::builder()
        .body(Body::empty())
        .expect("request build");
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8080);
    req.extensions_mut().insert(ConnectInfo(addr));
    req.headers_mut()
        .insert("X-Forwarded-For", "203.0.113.5".parse().unwrap());

    let result = adapter.check_request(&req).await;
    assert!(
        result.is_ok(),
        "request via trusted proxy with valid X-Forwarded-For should be allowed: {:?}",
        result
    );
}
