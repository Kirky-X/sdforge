// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Performance Benchmark Tests for SDForge
//!
//! This module contains performance benchmarks for:
//! - Core operations (error creation, response building)
//! - Security operations (API key validation)
//! - HTTP request handling
//! - Cache operations
//! - Concurrent scenarios

#[cfg(test)]
mod performance_benchmarks {
    use std::time::Duration;

    // ============================================================================
    // Core Module Benchmarks
    // ============================================================================

    /// Benchmark: ApiError creation speed
    #[test]
    fn benchmark_api_error_creation() {
        use sdforge::core::ApiError;

        let start = std::time::Instant::now();
        let iterations = 100_000;

        for i in 0..iterations {
            let _error = ApiError::internal_error(&format!("Error {}", i), "BENCH");
        }

        let elapsed = start.elapsed();
        let per_second = iterations as f64 / elapsed.as_secs_f64();

        println!("ApiError creation: {:.0} errors/sec", per_second);
        assert!(elapsed < Duration::from_millis(500));
    }

    /// Benchmark: ServiceResponse creation
    #[test]
    fn benchmark_service_response_creation() {
        use sdforge::core::ServiceResponse;

        let start = std::time::Instant::now();
        let iterations = 100_000;

        for _ in 0..iterations {
            let _response = ServiceResponse::success("test data".to_string());
        }

        let elapsed = start.elapsed();
        let per_second = iterations as f64 / elapsed.as_secs_f64();

        println!("ServiceResponse creation: {:.0} responses/sec", per_second);
        assert!(elapsed < Duration::from_millis(200));
    }

    /// Benchmark: JSON serialization
    #[test]
    fn benchmark_json_serialization() {
        use sdforge::core::ServiceResponse;
        use serde_json;

        let response = ServiceResponse::success("test data".to_string());
        
        let start = std::time::Instant::now();
        let iterations = 10_000;

        for _ in 0..iterations {
            let _json = serde_json::to_string(&response).unwrap();
        }

        let elapsed = start.elapsed();
        let per_second = iterations as f64 / elapsed.as_secs_f64();

        println!("JSON serialization: {:.0} ops/sec", per_second);
        assert!(elapsed < Duration::from_millis(500));
    }

    // ============================================================================
    // Security Module Benchmarks
    // ============================================================================

    /// Benchmark: API key validation
    #[test]
    fn benchmark_api_key_validation() {
        use sdforge::security::{ApiKeyMetadata, AppApiKeyAuth};

        let auth = AppApiKeyAuth::new();
        let _metadata = ApiKeyMetadata::new("bench-key".to_string(), None);

        let start = std::time::Instant::now();
        let iterations = 10_000;

        for _ in 0..iterations {
            let _ = auth.validate_key("bench-key", "127.0.0.1");
        }

        let elapsed = start.elapsed();
        let per_second = iterations as f64 / elapsed.as_secs_f64();

        println!("API key validation: {:.0} validations/sec", per_second);
        assert!(elapsed < Duration::from_millis(100));
    }

    /// Benchmark: Multiple API key management
    #[test]
    fn benchmark_multiple_api_keys() {
        use sdforge::security::{ApiKeyMetadata, AppApiKeyAuth};

        let auth = AppApiKeyAuth::new();

        let start = std::time::Instant::now();
        let iterations = 1_000;

        for i in 0..iterations {
            let key = format!("key-{}", i);
            let _metadata = ApiKeyMetadata::new(key.clone(), None);
            let _ = auth.validate_key(&key, "127.0.0.1");
        }

        let elapsed = start.elapsed();
        let per_second = iterations as f64 / elapsed.as_secs_f64();

        println!("Multiple API keys: {:.0} keys/sec", per_second);
        assert!(elapsed < Duration::from_millis(200));
    }

    // ============================================================================
    // HTTP Module Benchmarks
    // ============================================================================

    /// Benchmark: Simple HTTP handler
    #[tokio::test]
    async fn benchmark_simple_http_handler() {
        use axum::{body::Body, http::{Request, StatusCode}, routing::get, Router};
        use tower::ServiceExt;

        async fn handler() -> &'static str { "OK" }
        let app = Router::new().route("/bench", get(handler));

        let start = std::time::Instant::now();
        let iterations = 1_000;

        for _ in 0..iterations {
            let response = app
                .clone()
                .oneshot(Request::builder().uri("/bench").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let elapsed = start.elapsed();
        let per_second = iterations as f64 / elapsed.as_secs_f64();

        println!("HTTP handler: {:.0} requests/sec", per_second);
        assert!(elapsed < Duration::from_millis(500));
    }

    /// Benchmark: HTTP routing with multiple routes
    #[tokio::test]
    async fn benchmark_http_routing() {
        use axum::{body::Body, http::{Request, StatusCode}, routing::get, Router};
        use tower::ServiceExt;

        async fn h1() -> &'static str { "Route 1" }
        async fn h2() -> &'static str { "Route 2" }
        async fn h3() -> &'static str { "Route 3" }

        let app = Router::new()
            .route("/api/v1/users", get(h1))
            .route("/api/v1/posts", get(h2))
            .route("/api/v2/users", get(h3));

        let routes = vec!["/api/v1/users", "/api/v1/posts", "/api/v2/users"];
        
        let start = std::time::Instant::now();
        let iterations = 300; // 100 per route

        for _ in 0..iterations {
            for route in &routes {
                let response = app
                    .clone()
                    .oneshot(Request::builder().uri(route).body(Body::empty()).unwrap())
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }
        }

        let elapsed = start.elapsed();
        let per_second = (iterations * routes.len()) as f64 / elapsed.as_secs_f64();

        println!("HTTP routing: {:.0} routes/sec", per_second);
        assert!(elapsed < Duration::from_millis(500));
    }

    // ============================================================================
    // Concurrency Benchmarks
    // ============================================================================

    /// Benchmark: Concurrent error creation
    #[tokio::test]
    async fn benchmark_concurrent_error_creation() {
        use sdforge::core::ApiError;
        use std::sync::Arc;

        let error_data = Arc::new("concurrent error".to_string());
        let mut handles = vec![];

        let start = std::time::Instant::now();

        // Spawn 50 concurrent tasks
        for i in 0..50 {
            let data_clone = Arc::clone(&error_data);
            let handle = tokio::spawn(async move {
                for j in 0..100 {
                    let _error = ApiError::internal_error(&format!("{}-{}", data_clone, j), "CONC");
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        let elapsed = start.elapsed();
        let total_errors = 50 * 100;
        let per_second = total_errors as f64 / elapsed.as_secs_f64();

        println!("Concurrent error creation: {:.0} errors/sec", per_second);
        assert!(elapsed < Duration::from_millis(200));
    }

    /// Benchmark: Concurrent API key validation
    #[tokio::test]
    async fn benchmark_concurrent_api_key_validation() {
        use sdforge::security::{ApiKeyMetadata, AppApiKeyAuth};
        use std::sync::Arc;

        let auth = Arc::new(AppApiKeyAuth::new());
        let mut handles = vec![];

        let start = std::time::Instant::now();

        // Spawn 20 concurrent validation tasks
        for i in 0..20 {
            let auth_clone = Arc::clone(&auth);
            let handle = tokio::spawn(async move {
                for j in 0..50 {
                    let key = format!("concurrent-key-{}-{}", i, j);
                    let _metadata = ApiKeyMetadata::new(key.clone(), None);
                    let _ = auth_clone.validate_key(&key, "127.0.0.1");
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        let elapsed = start.elapsed();
        let total_validations = 20 * 50;
        let per_second = total_validations as f64 / elapsed.as_secs_f64();

        println!("Concurrent API key validation: {:.0} validations/sec", per_second);
        assert!(elapsed < Duration::from_millis(300));
    }

    /// Benchmark: Memory allocation stress
    #[test]
    fn benchmark_memory_allocation_stress() {
        use sdforge::core::{ApiError, ServiceResponse};

        let start = std::time::Instant::now();
        let iterations = 10_000;

        for i in 0..iterations {
            // Create error
            let error = ApiError::not_found("Resource", Some("id"));
            
            // Create response
            let response = ServiceResponse::<String>::error(
                sdforge::core::response::ServiceError::new("CODE", "msg", 500)
            );

            // Drop both
            drop(error);
            drop(response);

            // Allocate some memory
            let _data = "x".repeat(i % 1000);
        }

        let elapsed = start.elapsed();
        println!("Memory allocation stress: {} iterations in {:?}", iterations, elapsed);
        assert!(elapsed < Duration::from_millis(1000));
    }

    // ============================================================================
    // Edge Case Performance Tests
    // ============================================================================

    /// Test: Performance with very large strings
    #[test]
    fn benchmark_large_string_handling() {
        use sdforge::core::ServiceResponse;

        let large_data = "x".repeat(1_000_000); // 1MB
        
        let start = std::time::Instant::now();
        let iterations = 100;

        for _ in 0..iterations {
            let response = ServiceResponse::success(large_data.clone());
            let _data = response.data();
        }

        let elapsed = start.elapsed();
        println!("Large string handling: {} iterations in {:?}", iterations, elapsed);
        assert!(elapsed < Duration::from_millis(500));
    }

    /// Test: Performance with many small allocations
    #[test]
    fn benchmark_many_small_allocations() {
        use sdforge::core::ApiError;

        let start = std::time::Instant::now();
        let iterations = 100_000;

        for i in 0..iterations {
            let _error = ApiError::validation_error(&format!("f{}", i), &format!("v{}", i));
        }

        let elapsed = start.elapsed();
        let per_second = iterations as f64 / elapsed.as_secs_f64();

        println!("Small allocations: {:.0} allocs/sec", per_second);
        assert!(elapsed < Duration::from_millis(500));
    }
}
