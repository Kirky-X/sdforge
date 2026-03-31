// Copyright (c) 2026 Kirky.X
//! Performance benchmarks for SdForge
//!
//! These benchmarks measure the performance of core SdForge operations.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sdforge::prelude::{ApiError, ServiceError, ServiceResponse};
use std::collections::HashMap;

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

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
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
criterion_main!(benches);

// =============================================================================
// Additional Performance Benchmarks for Security and Core Features
// =============================================================================

/// Benchmark for rate limiter performance
#[cfg(feature = "security")]
fn benchmark_rate_limiter(c: &mut Criterion) {
    use sdforge::security::{RateLimiter, RateLimiterConfig};
    
    let config = RateLimiterConfig::new(100, 60); // 100 requests per minute
    let limiter = RateLimiter::with_config(config);
    
    c.bench_function("rate_limiter_check_first_request", |b| {
        b.iter(|| limiter.check("test_key_1"))
    });
    
    c.bench_function("rate_limiter_check_existing_key", |b| {
        // Pre-populate the key
        let _ = limiter.check("existing_key");
        b.iter(|| limiter.check("existing_key"))
    });
}

/// Benchmark for API key validation
#[cfg(feature = "security")]
fn benchmark_api_key_validation(c: &mut Criterion) {
    use sdforge::security::{ApiKeyManager, ApiKeyMetadata};
    
    let manager = ApiKeyManager::new();
    let api_key = "testkey_test_1234567890abcdef";
    
    // Add a test key
    let metadata = ApiKeyMetadata::new(
        "test_key".to_string(),
        Some("Test API Key".to_string()),
    );
    
    c.bench_function("api_key_validate_valid", |b| {
        b.iter(|| {
            manager.add_key(api_key.to_string(), metadata.clone());
            manager.validate(api_key)
        })
    });
    
    c.bench_function("api_key_validate_invalid", |b| {
        b.iter(|| manager.validate("invalid_key"))
    });
}

/// Benchmark for JWT token operations
#[cfg(feature = "security")]
fn benchmark_jwt_operations(c: &mut Criterion) {
    use sdforge::security::{BearerAuth, generate_secure_jwt_secret};
    
    let secret = generate_secure_jwt_secret();
    
    c.bench_function("jwt_secret_generation", |b| {
        b.iter(|| generate_secure_jwt_secret())
    });
    
    c.bench_function("jwt_secret_validation", |b| {
        b.iter(|| {
            let test_secret = generate_secure_jwt_secret();
            test_secret.len() >= 32
        })
    });
}

/// Benchmark for error sanitization
#[cfg(feature = "security")]
fn benchmark_error_sanitization(c: &mut Criterion) {
    use sdforge::security::audit::sanitize_error_message;
    
    let clean_message = "Operation failed";
    let sensitive_message = "Failed with password=secret123 and token=abc456";
    let long_message = "x".repeat(1000);
    
    c.bench_function("sanitize_clean_message", |b| {
        b.iter(|| sanitize_error_message(clean_message))
    });
    
    c.bench_function("sanitize_sensitive_message", |b| {
        b.iter(|| sanitize_error_message(sensitive_message))
    });
    
    c.bench_function("sanitize_long_message", |b| {
        b.iter(|| sanitize_error_message(&long_message))
    });
}

/// Benchmark for cache operations
#[cfg(feature = "cache")]
fn benchmark_cache_operations(c: &mut Criterion) {
    use sdforge::cache::{Cache, DashMapCache, SyncCache};
    use std::sync::Arc;
    
    let cache = Arc::new(DashMapCache::new());
    
    c.bench_function("cache_set_simple", |b| {
        b.iter(|| cache.set("key", "value".to_string()))
    });
    
    c.bench_function("cache_get_hit", |b| {
        cache.set("existing_key", "value".to_string());
        b.iter(|| cache.get("existing_key"))
    });
    
    c.bench_function("cache_get_miss", |b| {
        b.iter(|| cache.get::<String>("nonexistent_key"))
    });
    
    c.bench_function("cache_delete", |b| {
        cache.set("to_delete", "value".to_string());
        b.iter(|| cache.delete("to_delete"))
    });
}

/// Benchmark for regex caching
#[cfg(feature = "http")]
fn benchmark_regex_caching(c: &mut Criterion) {
    use sdforge::core::regex_cache::{get_regex, RegexCache};
    
    let pattern = r"^\d{3}-\d{3}-\d{4}$";
    
    c.bench_function("regex_first_compile", |b| {
        b.iter(|| get_regex(pattern))
    });
    
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

// Add benchmarks to criterion
#[cfg(feature = "security")]
criterion_group!(
    security_benches,
    benchmark_rate_limiter,
    benchmark_api_key_validation,
    benchmark_jwt_operations,
    benchmark_error_sanitization,
);

#[cfg(feature = "cache")]
criterion_group!(cache_benches, benchmark_cache_operations);

#[cfg(feature = "http")]
criterion_group!(http_benches, benchmark_regex_caching);

// Main benchmark groups - include all available benches
#[cfg(all(feature = "security", feature = "cache", feature = "http"))]
criterion_main!(benches, security_benches, cache_benches, http_benches);

#[cfg(all(feature = "security", feature = "cache", not(feature = "http")))]
criterion_main!(benches, security_benches, cache_benches);

#[cfg(all(feature = "security", not(feature = "cache"), feature = "http"))]
criterion_main!(benches, security_benches, http_benches);

#[cfg(all(not(feature = "security"), feature = "cache", feature = "http"))]
criterion_main!(benches, cache_benches, http_benches);

#[cfg(all(feature = "security", not(feature = "cache"), not(feature = "http")))]
criterion_main!(benches, security_benches);

#[cfg(all(not(feature = "security"), feature = "cache", not(feature = "http")))]
criterion_main!(benches, cache_benches);

#[cfg(all(not(feature = "security"), not(feature = "cache"), feature = "http"))]
criterion_main!(benches, http_benches);

#[cfg(all(not(feature = "security"), not(feature = "cache"), not(feature = "http")))]
criterion_main!(benches);
