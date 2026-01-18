// Copyright (c) 2026 Kirky.X
//! Performance benchmarks for SdForge
//!
//! These benchmarks measure the performance of core SdForge operations.

use criterion::{criterion_group, criterion_main, Criterion};
use sdforge::prelude::{ApiError, ServiceError, ServiceResponse};

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
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
