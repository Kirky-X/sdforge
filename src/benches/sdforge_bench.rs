// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Performance benchmarks for SdForge
//!
//! These benchmarks measure the performance of core SdForge operations.
//! Coverage: core types, JSON, cache (incl. batch + pattern), security
//! (API key, JWT, LRU), HTTP router construction, MCP tool registration.

use criterion::{criterion_group, Criterion, Throughput};
use sdforge::prelude::{ApiError, ServiceError, ServiceResponse};
use std::collections::HashMap;
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    // ============================================================================
    // Error Creation Benchmarks
    // ============================================================================

    c.bench_function("api_error_not_found_creation", |b| {
        b.iter(|| ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("123".to_string()),
        })
    });

    c.bench_function("service_error_creation", |b| {
        b.iter(|| ServiceError::new("ERR", "Error message", 500))
    });

    // ============================================================================
    // Response Creation Benchmarks
    // ============================================================================

    c.bench_function("service_response_success_creation", |b| {
        b.iter(|| ServiceResponse::success("test data"))
    });

    c.bench_function("service_response_error_creation", |b| {
        b.iter(|| ServiceResponse::<()>::error(ServiceError::new("ERR", "Error", 500)))
    });

    // ============================================================================
    // JSON Serialization Benchmarks
    // ============================================================================

    let mut group = c.benchmark_group("json_serialization");

    let small_data = serde_json::json!({"key": "value"});
    let medium_data = serde_json::json!({
        "user_id": 123,
        "name": "Test User",
        "email": "test@example.com",
        "active": true,
        "roles": ["admin", "user"]
    });
    let large_data = serde_json::json!({
        "users": (0..100).map(|i| serde_json::json!({
            "id": i,
            "name": format!("User {}", i),
            "email": format!("user{}@example.com", i),
            "active": i % 2 == 0
        })).collect::<Vec<_>>()
    });

    group.bench_function("small_json", |b| {
        b.iter(|| serde_json::to_string(&small_data))
    });

    group.bench_function("medium_json", |b| {
        b.iter(|| serde_json::to_string(&medium_data))
    });

    group.bench_function("large_json", |b| {
        b.iter(|| serde_json::to_string(&large_data))
    });

    group.finish();

    // ============================================================================
    // JSON Deserialization Benchmarks
    // ============================================================================

    let mut group = c.benchmark_group("json_deserialization");

    let small_json = serde_json::json!({"key": "value"}).to_string();
    let medium_json = serde_json::json!({
        "user_id": 123,
        "name": "Test User",
        "email": "test@example.com"
    })
    .to_string();
    let large_json = serde_json::json!({
        "data": (0..100).map(|i| serde_json::json!({
            "id": i,
            "value": i * 2
        })).collect::<Vec<_>>()
    })
    .to_string();

    group.bench_function("small_json_parse", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(&small_json))
    });

    group.bench_function("medium_json_parse", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(&medium_json))
    });

    group.bench_function("large_json_parse", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(&large_json))
    });

    group.finish();

    // ============================================================================
    // Metadata Benchmarks
    // ============================================================================

    c.bench_function("metadata_creation", |b| {
        use sdforge::core::ApiMetadata;
        b.iter(|| {
            ApiMetadata::new(
                "test_api".to_string(),
                "v1".to_string(),
                "Test API".to_string(),
                Some(300),
                false,
            )
        })
    });

    c.bench_function("metadata_cloning", |b| {
        use sdforge::core::ApiMetadata;

        let metadata = ApiMetadata::new(
            "test_api".to_string(),
            "v1".to_string(),
            "Test API".to_string(),
            Some(300),
            false,
        );

        b.iter(|| metadata.clone())
    });

    // ============================================================================
    // String Operations Benchmarks
    // ============================================================================

    let mut group = c.benchmark_group("string_operations");

    group.bench_function("string_concatenation", |b| {
        b.iter(|| {
            let mut result = String::new();
            for i in 0..100 {
                result.push_str(&format!("item_{}", i));
            }
            result
        })
    });

    group.bench_function("string_formatting", |b| {
        b.iter(|| (0..100).map(|i| format!("item_{}", i)).collect::<String>())
    });

    group.bench_function("string_length_check", |b| {
        let test_string = "test_string_for_length_check";
        b.iter(|| test_string.len())
    });

    group.finish();

    // ============================================================================
    // HashMap Operations Benchmarks
    // ============================================================================

    let mut group = c.benchmark_group("hashmap_operations");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let mut map = HashMap::new();
                for i in 0..size {
                    map.insert(i.to_string(), format!("value_{}", i));
                }

                b.iter(|| {
                    let mut sum = 0;
                    for (key, value) in &map {
                        sum += key.len() + value.len();
                    }
                    sum
                })
            },
        );
    }

    group.finish();

    // ============================================================================
    // Response Generation Benchmarks
    // ============================================================================

    let mut group = c.benchmark_group("response_generation");

    group.bench_function("empty_response", |b| {
        b.iter(|| ServiceResponse::<String>::success("".to_string()))
    });

    group.bench_function("small_response", |b| {
        b.iter(|| ServiceResponse::success("small data".to_string()))
    });

    group.bench_function("medium_response", |b| {
        let data = "x".repeat(1000);
        b.iter(|| ServiceResponse::success(data.clone()))
    });

    group.bench_function("large_response", |b| {
        let data = "x".repeat(10000);
        b.iter(|| ServiceResponse::success(data.clone()))
    });

    group.finish();

    // ============================================================================
    // Concurrent Access Benchmarks
    // ============================================================================

    c.bench_function("concurrent_metadata_access", |b| {
        use sdforge::core::ApiMetadata;
        use std::sync::{Arc, Mutex};

        let metadata = Arc::new(Mutex::new(ApiMetadata::new(
            "test".to_string(),
            "v1".to_string(),
            "Test".to_string(),
            Some(300),
            false,
        )));

        b.iter(|| {
            let mut sum = 0;
            for _ in 0..100 {
                let m = metadata.lock().unwrap();
                sum += m.name().len();
            }
            sum
        })
    });
}

criterion_group!(benches, criterion_benchmark);

// =============================================================================
// Cache Benchmarks (feature = "cache")
// =============================================================================

/// Benchmark for cache operations: get/set/delete + batch + contains/clear
#[cfg(feature = "cache")]
fn benchmark_cache_operations(c: &mut Criterion) {
    use sdforge::cache::{DashMapCache, SyncCache};
    use std::sync::Arc;

    let cache = Arc::new(DashMapCache::new());

    c.bench_function("cache_set_simple", |b| {
        b.iter(|| cache.set("key", b"value".to_vec()))
    });

    c.bench_function("cache_get_hit", |b| {
        cache.set("existing_key", b"value".to_vec());
        b.iter(|| cache.get("existing_key"))
    });

    c.bench_function("cache_get_miss", |b| {
        b.iter(|| cache.get("nonexistent_key"))
    });

    c.bench_function("cache_delete", |b| {
        cache.set("to_delete", b"value".to_vec());
        b.iter(|| cache.delete("to_delete"))
    });

    // --- Batch operations (Task 3.2.3) ---

    c.bench_function("cache_contains", |b| {
        cache.set("contains_key", b"v".to_vec());
        b.iter(|| cache.contains("contains_key"))
    });

    c.bench_function("cache_clear", |b| {
        let local = DashMapCache::new();
        local.set("k1", b"v1".to_vec());
        local.set("k2", b"v2".to_vec());
        b.iter(|| local.clear())
    });

    c.bench_function("cache_set_many_100", |b| {
        let local = DashMapCache::new();
        let items: Vec<(String, Vec<u8>)> = (0..100)
            .map(|i| {
                (
                    format!("batch_key_{}", i),
                    format!("val_{}", i).into_bytes(),
                )
            })
            .collect();
        b.iter(|| local.set_many(&items))
    });

    c.bench_function("cache_get_many_100", |b| {
        let local = DashMapCache::new();
        let keys: Vec<String> = (0..100).map(|i| format!("g_key_{}", i)).collect();
        for k in &keys {
            local.set(k, b"v".to_vec());
        }
        let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        b.iter(|| local.get_many(&key_refs))
    });

    c.bench_function("cache_delete_many_100", |b| {
        let local = DashMapCache::new();
        let keys: Vec<String> = (0..100).map(|i| format!("d_key_{}", i)).collect();
        for k in &keys {
            local.set(k, b"v".to_vec());
        }
        let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        b.iter(|| local.delete_many(&key_refs))
    });
}

/// Benchmark for cache pattern-based invalidation (Task 3.2.4)
#[cfg(feature = "cache")]
fn benchmark_cache_pattern_invalidate(c: &mut Criterion) {
    use sdforge::cache::{DashMapCache, SyncCache};

    let mut group = c.benchmark_group("cache_pattern_invalidate");

    for size in [100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let cache = DashMapCache::new();
                for i in 0..size {
                    cache.set(&format!("user:{}", i), b"v".to_vec());
                }
                // Non-matching keys to add noise
                for i in 0..size {
                    cache.set(&format!("session:{}", i), b"s".to_vec());
                }

                b.iter(|| {
                    // Re-populate user keys after each invalidation
                    for i in 0..size {
                        cache.set(&format!("user:{}", i), b"v".to_vec());
                    }
                    cache.invalidate(black_box("user:*"))
                })
            },
        );
    }

    group.finish();

    // find_keys_by_pattern without deletion
    c.bench_function("cache_find_keys_by_pattern", |b| {
        use sdforge::cache::{DashMapCache, SyncCache};
        let cache = DashMapCache::new();
        for i in 0..500 {
            cache.set(&format!("user:{}", i), b"v".to_vec());
        }
        b.iter(|| cache.find_keys_by_pattern(black_box("user:*")))
    });
}

/// Benchmark for cache key normalization (Task 3.2.5)
#[cfg(feature = "cache")]
fn benchmark_cache_key_normalization(c: &mut Criterion) {
    use sdforge::cache::canonicalize_cache_key;

    c.bench_function("cache_key_canonicalize_simple", |b| {
        b.iter(|| canonicalize_cache_key(black_box("user:123")))
    });

    c.bench_function("cache_key_canonicalize_trim", |b| {
        b.iter(|| canonicalize_cache_key(black_box("  USER:123  ")))
    });

    c.bench_function("cache_key_canonicalize_mixed", |b| {
        b.iter(|| canonicalize_cache_key(black_box("\tMixed_Case_Key\n")))
    });
}

#[cfg(feature = "cache")]
criterion_group!(
    cache_benches,
    benchmark_cache_operations,
    benchmark_cache_pattern_invalidate,
    benchmark_cache_key_normalization,
);

// =============================================================================
// Security Benchmarks (feature = "security")
// =============================================================================

/// Benchmark for API key validation performance.
///
/// Merges the former `benchmark_rate_limiter` (first-time/existing key
/// validation) with the original `benchmark_api_key_validation` (valid/invalid
/// key validation) into a single bench function covering all four scenarios.
#[cfg(feature = "security")]
fn benchmark_api_key_validation(c: &mut Criterion) {
    use sdforge::security::AppApiKeyAuth;

    let auth = AppApiKeyAuth::new();
    let api_key = "testkey1234567890abcdef";

    // Add a test key
    auth.add_key(api_key.to_string(), vec!["read".to_string()]);

    // First-time validation (key not pre-populated in cache)
    c.bench_function("api_key_auth_validate_first", |b| {
        b.iter(|| auth.validate_key("test_key_1", "127.0.0.1"))
    });

    // Existing key validation (pre-populated)
    c.bench_function("api_key_auth_validate_existing", |b| {
        let _ = auth.validate_key("existing_key", "127.0.0.1");
        b.iter(|| auth.validate_key("existing_key", "127.0.0.1"))
    });

    // Valid key validation (explicitly added)
    c.bench_function("api_key_validate_valid", |b| {
        b.iter(|| auth.validate_key(api_key, "127.0.0.1"))
    });

    // Invalid key validation
    c.bench_function("api_key_validate_invalid", |b| {
        b.iter(|| auth.validate_key("invalid_key", "127.0.0.1"))
    });
}

/// Benchmark for JWT token operations
#[cfg(feature = "security")]
fn benchmark_jwt_operations(c: &mut Criterion) {
    use sdforge::security::generate_secure_jwt_secret;

    let _secret = generate_secure_jwt_secret();

    c.bench_function("jwt_secret_generation", |b| {
        b.iter(generate_secure_jwt_secret)
    });

    c.bench_function("jwt_secret_validation", |b| {
        b.iter(|| {
            let test_secret = generate_secure_jwt_secret();
            test_secret.len() >= 32
        })
    });
}

/// Benchmark for BearerAuth token validation (Task 3.4.3)
#[cfg(feature = "security")]
fn benchmark_bearer_auth(c: &mut Criterion) {
    use sdforge::security::{AuthContext, AuthMetadata, BearerAuth};

    // Use a valid secret meeting complexity requirements (uppercase + special chars)
    let auth = BearerAuth::new("ValidBenchSecretKey123!@#WithUppercaseChars");
    let token = "bench_test_token_abc123";
    let context = AuthContext::new(
        Some("bench_user".to_string()),
        vec!["read".to_string(), "write".to_string()],
        AuthMetadata::default(),
    );
    auth.register_token(token.to_string(), context);

    c.bench_function("bearer_auth_validate_hit", |b| {
        b.iter(|| auth.validate_token(black_box(token)))
    });

    c.bench_function("bearer_auth_validate_miss", |b| {
        b.iter(|| auth.validate_token(black_box("nonexistent_token_xyz")))
    });

    c.bench_function("bearer_auth_register_token", |b| {
        let auth2 = BearerAuth::new("AnotherValidBenchSecretKey123!@#WithUppercaseChars");
        b.iter(|| {
            auth2.register_token(
                "new_token".to_string(),
                AuthContext::new(Some("u".to_string()), vec![], AuthMetadata::default()),
            )
        })
    });
}

/// Benchmark for LRU cache manager (Task 3.3)
#[cfg(feature = "security")]
fn benchmark_lru_cache(c: &mut Criterion) {
    use sdforge::cache::{DashMapCache, SharedCache};
    use sdforge::security::{LruCacheManager, LruConfig};
    use std::sync::Arc;
    use std::time::Duration;

    // LruCacheManager operations are synchronous (std::sync::Mutex), so no
    // tokio runtime context is needed.
    let cache: SharedCache = Arc::new(DashMapCache::new());
    let config = LruConfig {
        max_entries: 100,
        ttl: Duration::from_secs(3600),
        eviction_threshold: 0.8,
    };
    let manager = LruCacheManager::new(cache.clone(), config);

    // Pre-populate for get benchmark
    for i in 0..50 {
        manager.set(&format!("lru_key_{}", i), b"value".to_vec());
    }

    c.bench_function("lru_cache_set", |b| {
        b.iter(|| manager.set(black_box("lru_set_key"), black_box(b"v".to_vec())))
    });

    c.bench_function("lru_cache_get_hit", |b| {
        b.iter(|| manager.get(black_box("lru_key_0")))
    });

    c.bench_function("lru_cache_get_miss", |b| {
        b.iter(|| manager.get(black_box("nonexistent_lru_key")))
    });

    c.bench_function("lru_cache_delete", |b| {
        manager.set("lru_del_key", b"v".to_vec());
        b.iter(|| manager.delete(black_box("lru_del_key")))
    });

    // Eviction: fill beyond capacity to trigger eviction overhead
    let mut group = c.benchmark_group("lru_cache_eviction");
    group.throughput(Throughput::Elements(150));

    group.bench_function("evict_at_capacity", |b| {
        let cache2: SharedCache = Arc::new(DashMapCache::new());
        let mgr = LruCacheManager::new(
            cache2,
            LruConfig {
                max_entries: 100,
                ttl: Duration::from_secs(3600),
                eviction_threshold: 0.8,
            },
        );
        // Pre-fill to capacity
        for i in 0..100 {
            mgr.set(&format!("pre_{}", i), b"v".to_vec());
        }
        b.iter(|| {
            // Insert beyond capacity → triggers eviction
            for i in 0..50 {
                mgr.set(&format!("new_{}", i), b"v".to_vec());
            }
        })
    });

    group.finish();
}

#[cfg(feature = "security")]
criterion_group!(
    security_benches,
    benchmark_api_key_validation,
    benchmark_jwt_operations,
    benchmark_bearer_auth,
    benchmark_lru_cache,
);

// =============================================================================
// Rate Limit Benchmarks (feature = "ratelimit")
// =============================================================================

/// Benchmark `LimiteronAdapter::check` performance.
///
/// Measures two scenarios (design.md D8):
/// 1. `ratelimit_governor_allowed`: first check on a fresh identifier (allowed)
/// 2. `ratelimit_governor_denied`: check after quota exhausted (denied)
///
/// Uses a tokio runtime because `LimiteronAdapter::new()` and `check()` are
/// async. Each iteration uses a unique identifier to avoid cross-iteration
/// state pollution on the "allowed" path; the "denied" path pre-exhausts a
/// fixed identifier and then benches the denied check.
#[cfg(feature = "ratelimit")]
fn benchmark_limiteron_governor(c: &mut Criterion) {
    use sdforge::security::ratelimit::{LimiteronAdapter, RateLimiter};
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    let rt = Runtime::new().expect("tokio runtime for bench");

    // Pre-construct the adapter (Governor with default config + memory storage).
    let adapter = rt.block_on(LimiteronAdapter::new());
    let limiter: Arc<dyn RateLimiter> = Arc::new(adapter);

    // Allowed scenario: unique identifier per iteration → always first call.
    let mut counter: u64 = 0;
    c.bench_function("ratelimit_governor_allowed", |b| {
        b.iter(|| {
            let id = format!("allowed-{}", counter);
            counter = counter.wrapping_add(1);
            rt.block_on(limiter.check(&id)).expect("first call allowed")
        })
    });

    // Denied scenario: exhaust a fixed identifier's quota, then bench the
    // denied check. We loop until `check` returns Err to guarantee the
    // identifier is rate-limited before measuring.
    c.bench_function("ratelimit_governor_denied", |b| {
        // Pre-exhaust: call until denied. Default limiteron config may have a
        // generous quota, so we cap at 10_000 calls to avoid an infinite loop.
        for _ in 0..10_000 {
            if rt.block_on(limiter.check("denied-bench")).is_err() {
                break;
            }
        }
        b.iter(|| {
            // Each iteration should return Err (denied).
            let _ = rt.block_on(limiter.check("denied-bench"));
        })
    });
}

/// Benchmark `RateLimitMiddleware` end-to-end overhead.
///
/// Measures the full middleware path (limiter check + inner service call) with
/// a mock `RateLimiter` that always returns `Ok(())`, so the bench isolates
/// the middleware's own overhead (clone, async dispatch, response build).
///
/// Uses `tower::Layer` + `tower::Service::call` and a tokio runtime to drive
/// the async future.
#[cfg(feature = "ratelimit")]
fn benchmark_rate_limit_middleware(c: &mut Criterion) {
    use axum::body::Body;
    use axum::http::Request;
    use axum::response::Response;
    use sdforge::security::ratelimit::{
        RateLimitError, RateLimitLayer, RateLimitMiddleware, RateLimiter,
    };
    use std::convert::Infallible;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use tokio::runtime::Runtime;
    use tower::{Layer, Service};

    /// Mock `RateLimiter` that always approves (returns `Ok(())`).
    struct AlwaysAllowLimiter;

    impl RateLimiter for AlwaysAllowLimiter {
        fn check<'a>(
            &'a self,
            _identifier: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
            Box::pin(async { Ok(()) })
        }

        fn check_request<'a>(
            &'a self,
            _req: &'a Request<Body>,
        ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
            Box::pin(async { Ok(()) })
        }
    }

    /// Minimal `Clone` echo service (mirrors `middleware_tests.rs`).
    #[derive(Clone)]
    struct EchoService;

    impl Service<Request<Body>> for EchoService {
        type Response = Response;
        type Error = Infallible;
        type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: Request<Body>) -> Self::Future {
            Box::pin(async move {
                let resp = Response::new(Body::from("ok"));
                Ok(resp)
            })
        }
    }

    let rt = Runtime::new().expect("tokio runtime for bench");
    let limiter: Arc<dyn RateLimiter> = Arc::new(AlwaysAllowLimiter);
    let layer = RateLimitLayer::new(limiter);

    c.bench_function("ratelimit_middleware_passthrough", |b| {
        b.iter(|| {
            // Construct a fresh middleware per iteration because `Service::call`
            // takes `&mut self` and the future owns the request.
            let mut middleware: RateLimitMiddleware<EchoService> = layer.layer(EchoService);
            let req = Request::builder()
                .body(Body::empty())
                .expect("request build");
            rt.block_on(middleware.call(req))
                .expect("middleware must not error")
        })
    });
}

#[cfg(feature = "ratelimit")]
criterion_group!(
    ratelimit_benches,
    benchmark_limiteron_governor,
    benchmark_rate_limit_middleware,
);

// =============================================================================
// HTTP Benchmarks (feature = "http")
// =============================================================================

/// Benchmark for regex caching (existing)
#[cfg(feature = "http")]
fn benchmark_regex_caching(c: &mut Criterion) {
    use sdforge::core::regex_cache::get_regex;

    let pattern = r"^\d{3}-\d{3}-\d{4}$";

    c.bench_function("regex_first_compile", |b| b.iter(|| get_regex(pattern)));

    c.bench_function("regex_cached_lookup", |b| {
        // First call to cache it
        let _ = get_regex(pattern);
        b.iter(|| get_regex(pattern))
    });

    c.bench_function("regex_is_match", |b| {
        let regex = get_regex(pattern).unwrap();
        b.iter(|| regex.is_match("123-456-7890"))
    });
}

/// Benchmark for HTTP router construction (Task 3.6.1)
#[cfg(feature = "http")]
fn benchmark_http_router_construction(c: &mut Criterion) {
    use sdforge::config::{AppConfig, AuthConfig, ServerConfig};

    c.bench_function("http_router_build", |b| b.iter(sdforge::http::build));

    c.bench_function("http_router_build_with_redirect", |b| {
        b.iter(sdforge::http::build_with_redirect)
    });

    let config = AppConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: None,
        },
        authentication: AuthConfig::None,
        timeout: None,
    };

    c.bench_function("http_router_build_with_config_none_auth", |b| {
        b.iter(|| sdforge::http::build_with_config(black_box(&config)))
    });

    let config_cors = AppConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: Some(sdforge::config::CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
                allowed_methods: vec!["GET".to_string(), "POST".to_string()],
                allowed_headers: vec!["Content-Type".to_string()],
            }),
        },
        authentication: AuthConfig::None,
        timeout: None,
    };

    c.bench_function("http_router_build_with_config_cors", |b| {
        b.iter(|| sdforge::http::build_with_config(black_box(&config_cors)))
    });
}

#[cfg(feature = "http")]
criterion_group!(
    http_benches,
    benchmark_regex_caching,
    benchmark_http_router_construction,
);

// =============================================================================
// MCP Benchmarks (feature = "mcp")
// =============================================================================

/// Benchmark for MCP tool registration/collection (Task 3.6.2)
#[cfg(feature = "mcp")]
fn benchmark_mcp_tool_registration(c: &mut Criterion) {
    c.bench_function("mcp_get_tools", |b| b.iter(sdforge::get_mcp_tools));

    c.bench_function("mcp_init_all_plugins", |b| {
        b.iter(sdforge::init_all_plugins)
    });
}

#[cfg(feature = "mcp")]
criterion_group!(mcp_benches, benchmark_mcp_tool_registration);

// =============================================================================
// Main entry point — manual main for robust feature gating
// =============================================================================

fn main() {
    let mut criterion = Criterion::default().configure_from_args();

    // Core benchmarks (always run)
    criterion_benchmark(&mut criterion);

    // Cache benchmarks
    #[cfg(feature = "cache")]
    {
        benchmark_cache_operations(&mut criterion);
        benchmark_cache_pattern_invalidate(&mut criterion);
        benchmark_cache_key_normalization(&mut criterion);
    }

    // Security benchmarks
    #[cfg(feature = "security")]
    {
        benchmark_api_key_validation(&mut criterion);
        benchmark_jwt_operations(&mut criterion);
        benchmark_bearer_auth(&mut criterion);
        benchmark_lru_cache(&mut criterion);
    }

    // Rate limit benchmarks (limiteron governor + middleware)
    #[cfg(feature = "ratelimit")]
    {
        benchmark_limiteron_governor(&mut criterion);
        benchmark_rate_limit_middleware(&mut criterion);
    }

    // HTTP benchmarks
    #[cfg(feature = "http")]
    {
        benchmark_regex_caching(&mut criterion);
        benchmark_http_router_construction(&mut criterion);
    }

    // MCP benchmarks
    #[cfg(feature = "mcp")]
    {
        benchmark_mcp_tool_registration(&mut criterion);
    }

    criterion.final_summary();
}
