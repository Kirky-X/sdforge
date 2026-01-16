// Copyright (c) 2026 Kirky.X
//! HTTP Performance Benchmarks
//!
//! Benchmarks for HTTP request/response performance.

use axiom::core::{ApiError, ApiMetadata, ServiceResponse};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Benchmark: ServiceResponse creation
fn service_response_creation(c: &mut Criterion) {
    c.bench_function("service_response_creation", |b| {
        b.iter(|| {
            black_box(ServiceResponse::success("test_data"));
        });
    });
}

/// Benchmark: ApiError creation
fn api_error_creation(c: &mut Criterion) {
    c.bench_function("api_error_creation", |b| {
        b.iter(|| {
            black_box(ApiError::NotFound {
                resource: "resource".to_string(),
                resource_id: Some("id".to_string()),
            });
        });
    });
}

/// Benchmark: JSON serialization
fn json_serialization(c: &mut Criterion) {
    let response = ServiceResponse::success("test_data");
    c.bench_function("json_serialization", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&response).unwrap());
        });
    });
}

/// Benchmark: JSON deserialization
fn json_deserialization(c: &mut Criterion) {
    let json = r#"{"success":true,"data":"test_data"}"#;
    c.bench_function("json_deserialization", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<ServiceResponse<String>>(json).unwrap());
        });
    });
}

/// Benchmark: API metadata creation
fn api_metadata_creation(c: &mut Criterion) {
    c.bench_function("api_metadata_creation", |b| {
        b.iter(|| {
            black_box(ApiMetadata::new(
                "test_api".to_string(),
                "v1".to_string(),
                "Test API".to_string(),
                None,
                false,
            ));
        });
    });
}

/// Benchmark: Response building with data
fn response_with_data(c: &mut Criterion) {
    c.bench_function("response_with_data", |b| {
        b.iter(|| {
            let data = vec![1, 2, 3, 4, 5];
            black_box(ServiceResponse::success(data));
        });
    });
}

/// Benchmark: Error response creation
fn error_response_creation(c: &mut Criterion) {
    c.bench_function("error_response_creation", |b| {
        b.iter(|| {
            black_box(ServiceResponse::<String>::error(
                axiom::core::ServiceError::new("ERR", "error message", 500),
            ));
        });
    });
}

criterion_group!(
    benches,
    service_response_creation,
    api_error_creation,
    json_serialization,
    json_deserialization,
    api_metadata_creation,
    response_with_data,
    error_response_creation,
);

criterion_main!(benches);
