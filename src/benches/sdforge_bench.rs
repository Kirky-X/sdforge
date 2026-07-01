// Copyright (c) 2026 Kirky.X
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

        group.bench_with_input(criterion::BenchmarkId::from_parameter(size), size, |b, &size| {
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
        });
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
            .map(|i| (format!("batch_key_{}", i), format!("val_{}", i).into_bytes()))
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

/// Benchmark for API key auth rate limiting performance
#[cfg(feature = "security")]
fn benchmark_rate_limiter(c: &mut Criterion) {
    use sdforge::security::AppApiKeyAuth;

    let auth = AppApiKeyAuth::new();

    c.bench_function("api_key_auth_validate_first", |b| {
        b.iter(|| auth.validate_key("test_key_1", "127.0.0.1"))
    });

    c.bench_function("api_key_auth_validate_existing", |b| {
        // Pre-populate the key
        let _ = auth.validate_key("existing_key", "127.0.0.1");
        b.iter(|| auth.validate_key("existing_key", "127.0.0.1"))
    });
}

/// Benchmark for API key validation
#[cfg(feature = "security")]
fn benchmark_api_key_validation(c: &mut Criterion) {
    use sdforge::security::AppApiKeyAuth;

    let auth = AppApiKeyAuth::new();
    let api_key = "testkey_test_1234567890abcdef";

    // Add a test key
    auth.add_key(api_key.to_string(), vec!["read".to_string()]);

    c.bench_function("api_key_validate_valid", |b| {
        b.iter(|| auth.validate_key(api_key, "127.0.0.1"))
    });

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
                AuthContext::new(
                    Some("u".to_string()),
                    vec![],
                    AuthMetadata::default(),
                ),
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

    // LruCacheManager uses tokio::spawn internally, so we need a runtime context.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();

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
    benchmark_rate_limiter,
    benchmark_api_key_validation,
    benchmark_jwt_operations,
    benchmark_bearer_auth,
    benchmark_lru_cache,
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

    c.bench_function("http_router_build", |b| {
        b.iter(|| sdforge::http::build())
    });

    c.bench_function("http_router_build_with_redirect", |b| {
        b.iter(|| sdforge::http::build_with_redirect())
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
    c.bench_function("mcp_get_tools", |b| {
        b.iter(|| sdforge::get_mcp_tools())
    });

    c.bench_function("mcp_init_all_plugins", |b| {
        b.iter(|| sdforge::init_all_plugins())
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
        benchmark_rate_limiter(&mut criterion);
        benchmark_api_key_validation(&mut criterion);
        benchmark_jwt_operations(&mut criterion);
        benchmark_bearer_auth(&mut criterion);
        benchmark_lru_cache(&mut criterion);
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
