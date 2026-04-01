// Copyright (c) 2026 Kirky.X
//! Phase 2 Configuration and Cache Performance Benchmarks
//!
//! These benchmarks measure the performance of:
//! - Configuration validation operations
//! - Cache invalidation with patterns
//! - LRU cache operations
//! - Builder pattern overhead

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

/// Benchmark configuration validation performance
#[cfg(all(feature = "validation", feature = "http"))]
fn benchmark_config_validation(c: &mut Criterion) {
    use sdforge::config::{AppConfig, AuthConfig, ServerConfig};

    let mut group = c.benchmark_group("config_validation");

    // Valid configuration
    let valid_config = AppConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
        },
        authentication: AuthConfig::ApiKey {
            header_name: "X-Auth".to_string(),
            prefix: "Bearer ".to_string(),
        },
        timeout: None,
    };

    group.bench_function("validate_valid_config", |b| {
        b.iter(|| valid_config.validate())
    });

    // Invalid configuration (empty prefix)
    let invalid_api_key_config = AppConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
        },
        authentication: AuthConfig::ApiKey {
            header_name: "X-Auth".to_string(),
            prefix: "".to_string(), // Invalid: empty prefix
        },
        timeout: None,
    };

    group.bench_function("validate_invalid_api_key_prefix", |b| {
        b.iter(|| invalid_api_key_config.validate())
    });

    group.finish();
}

/// Benchmark builder pattern with validation
#[cfg(all(feature = "validation", feature = "http"))]
fn benchmark_builder_with_validation(c: &mut Criterion) {
    use sdforge::config::{AppConfig, AuthConfig, ServerConfig};

    let mut group = c.benchmark_group("builder_validation");

    group.bench_function("build_valid_config", |b| {
        b.iter(|| {
            AppConfig::builder()
                .server(ServerConfig {
                    host: "0.0.0.0".to_string(),
                    port: 8080,
                    request_timeout_secs: 30,
                    cors: None,
                })
                .authentication(AuthConfig::ApiKey {
                    header_name: "X-Auth".to_string(),
                    prefix: "Bearer ".to_string(),
                })
                .build()
        })
    });

    group.bench_function("build_minimal_config", |b| {
        b.iter(|| AppConfig::builder().build())
    });

    group.finish();
}

/// Benchmark cache pattern matching performance
#[cfg(feature = "cache")]
fn benchmark_cache_pattern_matching(c: &mut Criterion) {
    use sdforge::cache::{DashMapCache, SyncCache};
    use std::sync::Arc;

    let mut group = c.benchmark_group("cache_pattern_matching");

    // Setup cache with test data
    let cache = Arc::new(DashMapCache::new());
    for i in 0..100 {
        cache.set(&format!("user:{}", i), format!("data_{}", i).into_bytes());
        cache.set(
            &format!("session:{}", i),
            format!("session_{}", i).into_bytes(),
        );
    }

    group.bench_function("invalidate_user_pattern", |b| {
        b.iter(|| cache.invalidate("user:*"))
    });

    group.bench_function("invalidate_session_pattern", |b| {
        b.iter(|| cache.invalidate("session:*"))
    });

    group.bench_function("invalidate_all_pattern", |b| {
        b.iter(|| cache.invalidate("*"))
    });

    group.bench_function("find_keys_user_pattern", |b| {
        b.iter(|| cache.find_keys_by_pattern("user:*"))
    });

    group.finish();
}

/// Benchmark cache operations with different data sizes
#[cfg(feature = "cache")]
fn benchmark_cache_data_sizes(c: &mut Criterion) {
    use sdforge::cache::{DashMapCache, SyncCache};
    use std::sync::Arc;

    let mut group = c.benchmark_group("cache_data_sizes");

    let cache = Arc::new(DashMapCache::new());

    // Small value (10 bytes)
    let small_value = vec![0u8; 10];
    cache.set("small", small_value.clone());

    group.bench_function("get_small_value", |b| b.iter(|| cache.get("small")));

    // Medium value (1KB)
    let medium_value = vec![0u8; 1024];
    cache.set("medium", medium_value.clone());

    group.bench_function("get_medium_value", |b| b.iter(|| cache.get("medium")));

    // Large value (10KB)
    let large_value = vec![0u8; 10240];
    cache.set("large", large_value.clone());

    group.bench_function("get_large_value", |b| b.iter(|| cache.get("large")));

    group.finish();
}

/// Benchmark key normalization
#[cfg(feature = "cache")]
fn benchmark_key_normalization(c: &mut Criterion) {
    use sdforge::cache::canonicalize_cache_key;

    let mut group = c.benchmark_group("key_normalization");

    group.bench_function("normalize_simple_key", |b| {
        b.iter(|| canonicalize_cache_key("simple_key"))
    });

    group.bench_function("normalize_key_with_spaces", |b| {
        b.iter(|| canonicalize_cache_key("  key with spaces  "))
    });

    group.bench_function("normalize_mixed_case_key", |b| {
        b.iter(|| canonicalize_cache_key("MiXeD_CaSe_Key"))
    });

    group.finish();
}

/// Benchmark concurrent cache access
#[cfg(feature = "cache")]
fn benchmark_concurrent_cache_access(c: &mut Criterion) {
    use sdforge::cache::{DashMapCache, SyncCache};
    use std::sync::{Arc, Barrier};
    use std::thread;

    let mut group = c.benchmark_group("concurrent_cache");

    for num_threads in [1, 2, 4, 8].iter() {
        let cache: Arc<DashMapCache> = Arc::new(DashMapCache::new());

        // Pre-populate cache
        for i in 0..100 {
            cache.set(&format!("key_{}", i), format!("value_{}", i).into_bytes());
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            num_threads,
            |b, &num_threads| {
                let barrier = Arc::new(Barrier::new(num_threads));

                b.iter(|| {
                    let cache_clone = Arc::clone(&cache);
                    let barrier_clone = Arc::clone(&barrier);

                    let handles: Vec<_> = (0..num_threads)
                        .map(|i| {
                            let cache = Arc::clone(&cache_clone);
                            let barrier = Arc::clone(&barrier_clone);

                            thread::spawn(move || {
                                barrier.wait();

                                // Each thread performs multiple operations
                                for j in 0..10 {
                                    let key = format!("key_{}", (i * 10 + j) % 100);
                                    let _ = cache.get(&key);
                                }
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cache statistics
#[cfg(feature = "cache")]
fn benchmark_cache_statistics(c: &mut Criterion) {
    use sdforge::cache::{DashMapCache, SyncCache};
    use std::sync::Arc;

    let mut group = c.benchmark_group("cache_statistics");

    let cache: Arc<DashMapCache> = Arc::new(DashMapCache::new());

    // Populate cache
    for i in 0..1000 {
        cache.set(&format!("key_{}", i), format!("value_{}", i).into_bytes());
    }

    group.bench_function("get_stats_populated_cache", |b| {
        b.iter(|| cache.get_stats())
    });

    group.bench_function("len_operation", |b| b.iter(|| cache.len()));

    group.finish();
}

// Criterion group registration
#[cfg(all(feature = "validation", feature = "http"))]
criterion_group!(
    config_benches,
    benchmark_config_validation,
    benchmark_builder_with_validation,
);

#[cfg(feature = "cache")]
criterion_group!(
    cache_pattern_benches,
    benchmark_cache_pattern_matching,
    benchmark_cache_data_sizes,
    benchmark_key_normalization,
);

#[cfg(feature = "cache")]
criterion_group!(
    cache_concurrent_benches,
    benchmark_concurrent_cache_access,
    benchmark_cache_statistics,
);

// Main benchmark groups - include all available benches based on features
#[cfg(all(feature = "validation", feature = "http", feature = "cache"))]
criterion_main!(
    config_benches,
    cache_pattern_benches,
    cache_concurrent_benches,
);

#[cfg(all(feature = "validation", feature = "http", not(feature = "cache")))]
criterion_main!(config_benches, cache_pattern_benches,);

#[cfg(all(not(feature = "validation"), not(feature = "http"), feature = "cache"))]
criterion_main!(cache_pattern_benches, cache_concurrent_benches,);

#[cfg(all(
    not(feature = "validation"),
    not(feature = "http"),
    not(feature = "cache")
))]
fn main() {} // Empty main when no features enabled
