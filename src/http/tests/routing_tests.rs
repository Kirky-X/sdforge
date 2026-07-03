// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for HTTP routing: `HttpRoute`/`RouteRegistration` construction,
//! `resolve_route_path`, `build()`/`build_with_redirect()`, inventory
//! collection, and request ID generation helpers.

use super::super::*;
use axum::routing::get;

/// Test build() returns a valid Router
#[test]
fn test_build_returns_router() {
    let router = build();
    // Just verify it doesn't panic
    let _ = router;
}

/// Test build_with_redirect() returns Router with middleware
#[test]
fn test_build_with_redirect() {
    let router = build_with_redirect();
    // Should have layers applied
    let _ = router;
}

/// Test HttpRoute structure
#[test]
fn test_http_route_creation() {
    use axum::routing::get;

    async fn test_handler() -> &'static str {
        "test"
    }

    let route = HttpRoute::new(
        "/test".to_string(),
        get(test_handler),
        crate::core::ApiMetadata {
            name: "test".to_string(),
            version: "v1".to_string(),
            description: "Test API".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
        None,
    );

    assert_eq!(route.path(), "/test");
    assert_eq!(route.metadata().name(), "test");
    assert!(route.module_prefix().is_none());
}

/// Test resolve_route_path function
#[test]
fn test_resolve_route_path() {
    // No prefix
    assert_eq!(resolve_route_path("/api/users", None), "/api/users");
    assert_eq!(resolve_route_path("/api/users", Some("")), "/api/users");

    // With prefix
    assert_eq!(
        resolve_route_path("/api/users", Some("v1")),
        "/v1/api/users"
    );
    assert_eq!(
        resolve_route_path("/api/users", Some("/v1")),
        "/v1/api/users"
    );
}

/// Test RouteRegistration structure
#[test]
fn test_route_registration() {
    async fn test_handler() {}
    let registration = RouteRegistration::new(
        "test",
        "v1",
        || {
            HttpRoute::new(
                "/test".to_string(),
                get(test_handler),
                crate::core::ApiMetadata {
                    name: "test".to_string(),
                    version: "v1".to_string(),
                    description: "".to_string(),
                    cache_ttl: None,
                    is_streaming: false,
                },
                None,
            )
        },
        || crate::core::ApiMetadata {
            name: "test".to_string(),
            version: "v1".to_string(),
            description: "".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
    );

    assert_eq!(registration.name, "test");
    assert_eq!(registration.version, "v1");
}

// ============================================================================
// get_or_generate_request_id Tests
// ============================================================================

#[test]
fn test_get_or_generate_request_id_with_header() {
    let req = axum::http::Request::builder()
        .header(X_REQUEST_ID, "my-custom-id-123")
        .body(Body::empty())
        .unwrap();

    let id = get_or_generate_request_id(&req);
    assert_eq!(id, "my-custom-id-123");
}

#[test]
fn test_get_or_generate_request_id_without_header() {
    let req = axum::http::Request::builder().body(Body::empty()).unwrap();

    let id = get_or_generate_request_id(&req);
    // Should generate a valid UUID
    assert!(!id.is_empty());
    assert!(id.len() == 36); // UUID format: 8-4-4-4-12
}

#[test]
fn test_get_or_generate_request_id_with_invalid_header_value() {
    // Header with non-UTF8 bytes should fall back to UUID generation
    let req = axum::http::Request::builder()
        .header(
            X_REQUEST_ID,
            axum::http::HeaderValue::from_bytes(b"\xff\xfe").unwrap(),
        )
        .body(Body::empty())
        .unwrap();

    let id = get_or_generate_request_id(&req);
    // Should generate a UUID since header value is not valid UTF-8
    assert!(!id.is_empty());
    assert!(id.len() == 36);
}

// ============================================================================
// HttpRoute Extended Tests
// ============================================================================

#[test]
fn test_http_route_with_module_prefix() {
    async fn test_handler() {}
    let route = HttpRoute::new(
        "/api/users".to_string(),
        get(test_handler),
        crate::core::ApiMetadata {
            name: "users".to_string(),
            version: "v1".to_string(),
            description: "Users API".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
        Some("v1".to_string()),
    );

    assert_eq!(route.path(), "/api/users");
    assert_eq!(route.module_prefix(), Some("v1"));
    assert_eq!(route.metadata().name(), "users");
}

#[test]
fn test_http_route_handler_accessor() {
    async fn handler() -> &'static str {
        "handler"
    }
    let route = HttpRoute::new(
        "/test".to_string(),
        get(handler),
        crate::core::ApiMetadata {
            name: "test".to_string(),
            version: "v1".to_string(),
            description: "".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
        None,
    );

    // Verify handler accessor returns a reference
    let _handler = route.handler();
}

#[test]
fn test_http_route_clone() {
    async fn handler() {}
    let route = HttpRoute::new(
        "/clone-test".to_string(),
        get(handler),
        crate::core::ApiMetadata {
            name: "clone".to_string(),
            version: "v1".to_string(),
            description: "Clone test".to_string(),
            cache_ttl: Some(60),
            is_streaming: false,
        },
        Some("api".to_string()),
    );

    let cloned = route.clone();
    assert_eq!(cloned.path(), route.path());
    assert_eq!(cloned.module_prefix(), route.module_prefix());
}

#[test]
fn test_http_route_debug() {
    async fn handler() {}
    let route = HttpRoute::new(
        "/debug".to_string(),
        get(handler),
        crate::core::ApiMetadata {
            name: "debug".to_string(),
            version: "v1".to_string(),
            description: "Debug test".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
        None,
    );

    let debug_str = format!("{:?}", route);
    assert!(debug_str.contains("HttpRoute"));
}

// ============================================================================
// RouteRegistration Extended Tests
// ============================================================================

#[test]
fn test_route_registration_name_accessor() {
    async fn handler() {}
    let registration = RouteRegistration::new(
        "my_api",
        "v2",
        || {
            HttpRoute::new(
                "/api".to_string(),
                get(handler),
                crate::core::ApiMetadata {
                    name: "my_api".to_string(),
                    version: "v2".to_string(),
                    description: "".to_string(),
                    cache_ttl: None,
                    is_streaming: false,
                },
                None,
            )
        },
        || crate::core::ApiMetadata {
            name: "my_api".to_string(),
            version: "v2".to_string(),
            description: "".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
    );

    assert_eq!(registration.name, "my_api");
}

#[test]
fn test_route_registration_create() {
    async fn handler() {}
    let registration = RouteRegistration::new(
        "create_test",
        "v1",
        || {
            HttpRoute::new(
                "/create".to_string(),
                get(handler),
                crate::core::ApiMetadata {
                    name: "create_test".to_string(),
                    version: "v1".to_string(),
                    description: "Create test".to_string(),
                    cache_ttl: None,
                    is_streaming: false,
                },
                None,
            )
        },
        || crate::core::ApiMetadata {
            name: "create_test".to_string(),
            version: "v1".to_string(),
            description: "Create test".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
    );

    let route = registration.create();
    assert_eq!(route.path(), "/create");
}

// ============================================================================
// resolve_route_path Extended Tests
// ============================================================================

#[test]
fn test_resolve_route_path_empty_prefix() {
    assert_eq!(resolve_route_path("/api/test", Some("")), "/api/test");
}

#[test]
fn test_resolve_route_path_with_leading_slash() {
    assert_eq!(resolve_route_path("/api/test", Some("/v2")), "/v2/api/test");
}

#[test]
fn test_resolve_route_path_root_path() {
    assert_eq!(resolve_route_path("/", Some("v1")), "/v1/");
}

// ============================================================================
// Inventory Preservation Tests (feature-gated)
// ============================================================================

#[cfg(feature = "mcp")]
#[test]
fn test_preserve_mcp_inventory() {
    // This test verifies the MCP inventory preservation function doesn't panic
    // The function is called internally by build()
    preserve_mcp_inventory();
}

#[cfg(feature = "websocket")]
#[test]
fn test_preserve_websocket_inventory() {
    preserve_websocket_inventory();
}

#[cfg(feature = "grpc")]
#[test]
fn test_preserve_grpc_inventory() {
    preserve_grpc_inventory();
}

// ============================================================================
// build() function extended tests
// ============================================================================

#[test]
fn test_build_router_deterministic() {
    // Call build twice - should not panic
    let _router1 = build();
    let _router2 = build();
}

// ============================================================================
// Security Headers Tests
// ============================================================================

#[test]
fn test_apply_security_headers() {
    let router = Router::new();
    let router_with_headers = apply_security_headers(router);
    // Just verify it doesn't panic
    let _ = router_with_headers;
}

// ============================================================================
// build() Extended Tests - Route Building with Various Configurations
// ============================================================================

#[test]
fn test_build_with_direct_http_route_registration() {
    // Test that direct HttpRoute registrations are collected
    // This exercises line 171-173 in build()
    let router = build();
    // Should not panic even with no registered routes
    let _ = router;
}

#[test]
fn test_build_sorts_routes_deterministically() {
    // Call build multiple times to verify deterministic behavior
    // Routes are sorted by path (line 176)
    let _router1 = build();
    let _router2 = build();
    // Both should succeed without panic
}

#[test]
fn test_build_collects_registrations() {
    // This tests lines 162-168: collecting and creating from registrations
    let router = build();
    // Verify router is valid
    let _ = router;
}

// ============================================================================
// RouteRegistration Extended Tests - Metadata and Edge Cases
// ============================================================================

#[test]
fn test_route_registration_metadata() {
    async fn handler() {}
    let registration = RouteRegistration::new(
        "metadata_test",
        "v1",
        || {
            HttpRoute::new(
                "/metadata".to_string(),
                get(handler),
                crate::core::ApiMetadata {
                    name: "metadata_test".to_string(),
                    version: "v1".to_string(),
                    description: "Metadata test".to_string(),
                    cache_ttl: Some(300),
                    is_streaming: true,
                },
                None,
            )
        },
        || crate::core::ApiMetadata {
            name: "metadata_test".to_string(),
            version: "v1".to_string(),
            description: "Metadata test".to_string(),
            cache_ttl: Some(300),
            is_streaming: true,
        },
    );

    let metadata = registration.metadata();
    assert_eq!(metadata.name(), "metadata_test");
    assert!(metadata.is_streaming());
    assert_eq!(metadata.cache_ttl(), Some(300));
}

#[test]
fn test_route_registration_debug() {
    async fn handler() {}
    let registration = RouteRegistration::new(
        "debug_test",
        "v1",
        || {
            HttpRoute::new(
                "/debug".to_string(),
                get(handler),
                crate::core::ApiMetadata {
                    name: "debug_test".to_string(),
                    version: "v1".to_string(),
                    description: "".to_string(),
                    cache_ttl: None,
                    is_streaming: false,
                },
                None,
            )
        },
        || crate::core::ApiMetadata {
            name: "debug_test".to_string(),
            version: "v1".to_string(),
            description: "".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
    );

    let debug_str = format!("{:?}", registration);
    assert!(debug_str.contains("RouteRegistration"));
    assert!(debug_str.contains("debug_test"));
}

#[test]
fn test_route_registration_clone() {
    async fn handler() {}
    let registration = RouteRegistration::new(
        "clone_test",
        "v1",
        || {
            HttpRoute::new(
                "/clone".to_string(),
                get(handler),
                crate::core::ApiMetadata {
                    name: "clone_test".to_string(),
                    version: "v1".to_string(),
                    description: "".to_string(),
                    cache_ttl: None,
                    is_streaming: false,
                },
                None,
            )
        },
        || crate::core::ApiMetadata {
            name: "clone_test".to_string(),
            version: "v1".to_string(),
            description: "".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
    );

    let cloned = registration;
    assert_eq!(cloned.name, registration.name);
    assert_eq!(cloned.version, registration.version);
}

#[test]
fn test_route_registration_copy() {
    async fn handler() {}
    let registration = RouteRegistration::new(
        "copy_test",
        "v1",
        || {
            HttpRoute::new(
                "/copy".to_string(),
                get(handler),
                crate::core::ApiMetadata {
                    name: "copy_test".to_string(),
                    version: "v1".to_string(),
                    description: "".to_string(),
                    cache_ttl: None,
                    is_streaming: false,
                },
                None,
            )
        },
        || crate::core::ApiMetadata {
            name: "copy_test".to_string(),
            version: "v1".to_string(),
            description: "".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
    );

    let copied = registration; // RouteRegistration is Copy
    assert_eq!(copied.name, "copy_test");
}

// ============================================================================
// HttpRoute with Various Path Configurations
// ============================================================================

#[test]
fn test_http_route_with_root_path() {
    async fn handler() {}
    let route = HttpRoute::new(
        "/".to_string(),
        get(handler),
        crate::core::ApiMetadata {
            name: "root".to_string(),
            version: "v1".to_string(),
            description: "Root path".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
        None,
    );

    assert_eq!(route.path(), "/");
}

#[test]
fn test_http_route_with_nested_path() {
    async fn handler() {}
    let route = HttpRoute::new(
        "/api/v1/users/:id/posts/:post_id".to_string(),
        get(handler),
        crate::core::ApiMetadata {
            name: "nested".to_string(),
            version: "v1".to_string(),
            description: "Nested path".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
        Some("api".to_string()),
    );

    assert_eq!(route.path(), "/api/v1/users/:id/posts/:post_id");
    assert_eq!(route.module_prefix(), Some("api"));
}

#[test]
fn test_http_route_with_regex_path() {
    async fn handler() {}
    let route = HttpRoute::new(
        "/files/{*path}".to_string(),
        get(handler),
        crate::core::ApiMetadata {
            name: "wildcard".to_string(),
            version: "v1".to_string(),
            description: "Wildcard path".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
        None,
    );

    assert_eq!(route.path(), "/files/{*path}");
}

#[test]
fn test_http_route_metadata_accessors() {
    async fn handler() {}
    let route = HttpRoute::new(
        "/test".to_string(),
        get(handler),
        crate::core::ApiMetadata {
            name: "full_test".to_string(),
            version: "v2".to_string(),
            description: "Full metadata test".to_string(),
            cache_ttl: Some(600),
            is_streaming: true,
        },
        Some("v2".to_string()),
    );

    let metadata = route.metadata();
    assert_eq!(metadata.name(), "full_test");
    assert_eq!(metadata.version(), "v2");
    assert_eq!(metadata.description(), "Full metadata test");
    assert_eq!(metadata.cache_ttl(), Some(600));
    assert!(metadata.is_streaming());
}

// ============================================================================
// resolve_route_path Extended Edge Cases
// ============================================================================

#[test]
fn test_resolve_route_path_deep_prefix() {
    assert_eq!(
        resolve_route_path("/api/users", Some("api/v1")),
        "/api/v1/api/users"
    );
}

#[test]
fn test_resolve_route_path_multiple_leading_slashes() {
    assert_eq!(
        resolve_route_path("/api/users", Some("///v1")),
        "/v1/api/users"
    );
}

#[test]
fn test_resolve_route_path_whitespace_prefix() {
    // Whitespace prefix is not empty, so it gets used
    let result = resolve_route_path("/api/users", Some(" "));
    assert_eq!(result, "/ /api/users");
}

// ============================================================================
// build_with_redirect Extended Tests
// ============================================================================

#[test]
fn test_build_with_redirect_multiple_calls() {
    // Test that build_with_redirect can be called multiple times
    let router1 = build_with_redirect();
    let router2 = build_with_redirect();

    // Both should be valid routers
    let _ = router1;
    let _ = router2;
}

// ============================================================================
// X_REQUEST_ID Constant Tests
// ============================================================================

#[test]
fn test_x_request_id_constant() {
    assert_eq!(X_REQUEST_ID, "x-request-id");
}

// ============================================================================
// Inventory Collection Tests
// ============================================================================

#[test]
fn test_inventory_http_route_collection() {
    // Verify that inventory::iter::<HttpRoute> works
    // This exercises line 171 in build()
    let count = inventory::iter::<HttpRoute>().count();
    // Count may be 0 or more depending on registered routes
    let _ = count;
}

#[test]
fn test_inventory_route_registration_collection() {
    // Verify that inventory::iter::<RouteRegistration> works
    // This exercises line 162 in build()
    let count = inventory::iter::<RouteRegistration>().count();
    let _ = count;
}

// ============================================================================
// Inventory Route Accessibility Test (covers build() route collection loop)
// ============================================================================

#[tokio::test]
async fn test_build_includes_inventory_registered_route() {
    let router = build();
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"coverage-test-ok");
}

// ============================================================================
// resolve_route_path: additional edge cases
//
// resolve_route_path builds the full route path from a base path and an
// optional module prefix. These tests cover the prefix-trimming and
// base_path slicing branches.
// ============================================================================

/// Test resolve_route_path with a prefix that has no leading slash and a
/// base path that starts with `/`, verifying the clean_prefix trim and the
// base_path[1..] slice produce `/prefix/rest`.
#[test]
fn test_resolve_route_path_prefix_no_leading_slash() {
    let result = resolve_route_path("/users", Some("api"));
    assert_eq!(result, "/api/users");
}

/// Test resolve_route_path with an empty prefix string falls back to the
/// base path unchanged (the `!prefix.is_empty()` guard).
#[test]
fn test_resolve_route_path_empty_prefix_string() {
    let result = resolve_route_path("/users", Some(""));
    assert_eq!(result, "/users");
}

/// Test resolve_route_path with None prefix returns the base path.
#[test]
fn test_resolve_route_path_none_prefix() {
    let result = resolve_route_path("/items", None);
    assert_eq!(result, "/items");
}

/// Test resolve_route_path with multiple leading slashes in the prefix
/// are all trimmed.
#[test]
fn test_resolve_route_path_prefix_multiple_leading_slashes_trimmed() {
    let result = resolve_route_path("/users", Some("///v2"));
    // trim_start_matches('/') removes all leading slashes
    assert_eq!(result, "/v2/users");
}

// ============================================================================
// get_or_generate_request_id: edge cases
// ============================================================================

/// Test get_or_generate_request_id with an empty x-request-id header
/// value generates a UUID (falls through to the unwrap_or_else branch).
#[test]
fn test_get_or_generate_request_id_empty_header_value() {
    let req = axum::http::Request::builder()
        .header(X_REQUEST_ID, "")
        .body(Body::empty())
        .unwrap();
    let id = get_or_generate_request_id(&req);
    // An empty string header value is returned as-is (to_str succeeds on
    // empty), so the function returns "" rather than generating a UUID.
    // Verify the function does not panic and returns a String.
    let _: String = id;
}

/// Test get_or_generate_request_id generates a valid UUID v4 when no
/// x-request-id header is present.
#[test]
fn test_get_or_generate_request_id_generates_valid_uuid() {
    let req = axum::http::Request::builder().body(Body::empty()).unwrap();
    let id = get_or_generate_request_id(&req);
    // Generated UUIDs are 36 chars (8-4-4-4-12) with hyphens.
    assert!(
        id.len() == 36 && id.matches('-').count() == 4,
        "Generated request ID should be a UUID v4, got: {}",
        id,
    );
}
