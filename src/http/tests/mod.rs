// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! HTTP module test suites.
//!
//! Tests are organized by responsibility:
//! - `routing_tests`: `HttpRoute`/`RouteRegistration` construction, `resolve_route_path`,
//!   `build()`/`build_with_redirect()`, inventory collection, and request ID helpers
//! - `config_tests`: `build_with_config()` with various `ServerConfig`/`AuthConfig`/`CorsConfig`
//!   combinations and middleware layer wiring
//! - `middleware_tests`: request ID, JWT, and ApiKey middleware execution via real requests,
//!   plus hot-reload integration

mod config_tests;
mod middleware_tests;
mod routing_tests;

use super::{HttpRoute, RouteRegistration};
use axum::routing::get;

use crate::core::ApiMetadata;

// ============================================================================
// Inventory-registered test route (covers build() route collection, lines 162-183)
// ============================================================================
//
// Registered here at the `tests` module level so that every sub-module can
// exercise the route at `/cov-test-route`. The handler returns a static body
// so middleware tests can assert on the response.

fn coverage_test_route_create() -> HttpRoute {
    HttpRoute::new(
        "/cov-test-route".to_string(),
        get(|| async { "coverage-test-ok" }),
        ApiMetadata {
            name: "cov_test_route".to_string(),
            version: "v1".to_string(),
            description: "Coverage test route".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
        None,
    )
}

fn coverage_test_route_metadata() -> ApiMetadata {
    ApiMetadata {
        name: "cov_test_route".to_string(),
        version: "v1".to_string(),
        description: "Coverage test route".to_string(),
        cache_ttl: None,
        is_streaming: false,
    }
}

inventory::submit!(RouteRegistration::new(
    "cov_test_route",
    "v1",
    coverage_test_route_create,
    coverage_test_route_metadata,
));
