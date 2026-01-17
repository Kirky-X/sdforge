// Copyright (c) 2026 Kirky.X
//! Performance benchmarks for Axiom
//!
//! These benchmarks measure the performance of core Axiom operations.

use axiom::prelude::{ApiError, ServiceError, ServiceResponse};
use axiom::security::{RateLimitConfig, RateLimiter};
use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("api_error_not_found_creation", |b| {
        b.iter(|| ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("123".to_string()),
        })
    });

    c.bench_function("service_error_creation", |b| {
        b.iter(|| ServiceError::new("ERR", "Error message", 500))
    });

    c.bench_function("service_response_success_creation", |b| {
        b.iter(|| ServiceResponse::success("test data"))
    });

    c.bench_function("service_response_error_creation", |b| {
        b.iter(|| ServiceResponse::<()>::error(ServiceError::new("ERR", "Error", 500)))
    });

    c.bench_function("rate_limiter_check", |b| {
        let config = RateLimitConfig {
            max_requests: 1000,
            window: Duration::from_secs(60),
            include_headers: true,
        };
        let limiter = RateLimiter::new(Some(config));

        b.iter(|| {
            let _ = limiter.check("bench-key");
        })
    });

    c.bench_function("rate_limiter_many_keys", |b| {
        let config = RateLimitConfig {
            max_requests: 100,
            window: Duration::from_secs(60),
            include_headers: true,
        };
        let limiter = RateLimiter::new(Some(config));

        b.iter(|| {
            for i in 0..100 {
                let _ = limiter.check(&format!("bench-key-{}", i));
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
