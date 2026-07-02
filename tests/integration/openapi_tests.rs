// Copyright (c) 2026 Kirky.X
//! OpenAPI integration tests.
//!
//! These tests exercise the end-to-end pipeline: `#[service_api]` macro
//! emits `OpenApiRouteInfo` entries (when `openapi` feature is enabled),
//! and `generate_openapi_spec()` collects them via `inventory` to build a
//! complete `utoipa::openapi::OpenApi` document.
//!
//! Feature requirement: run with `cargo test --features "http,openapi" --test openapi_tests`.

#![cfg(feature = "openapi")]

use sdforge::core::ApiError;
use sdforge::openapi::{generate_openapi_spec, OpenApiBuilder};
use sdforge::service_api;

// ============================================================================
// Test fixtures: two `#[service_api]` endpoints.
//
// These are compiled into the test binary, so their `OpenApiRouteInfo`
// entries are collected by `inventory` and visible to `generate_openapi_spec`.
// ============================================================================

/// Fetch a single user by id. The path uses `:id` syntax; the macro converts
/// it to the OpenAPI `{id}` template form.
#[service_api(
    name = "openapi_test_get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    description = "Fetch a user by id"
)]
async fn get_user(id: u64) -> Result<String, ApiError> {
    Ok(format!("user-{}", id))
}

/// List all users. No path parameters.
#[service_api(
    name = "openapi_test_list_users",
    version = "v1",
    path = "/users",
    method = "GET",
    description = "List all users"
)]
async fn list_users() -> Result<Vec<String>, ApiError> {
    Ok(vec!["alice".to_string(), "bob".to_string()])
}

// ============================================================================
// Tests
// ============================================================================

/// The generated spec must include both registered endpoints, with the path
/// parameter `:id` translated to the OpenAPI `{id}` template form.
#[test]
fn generated_spec_contains_both_endpoints() {
    let spec = generate_openapi_spec();
    let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
    let paths_obj = paths_json.as_object().expect("paths is a JSON object");
    let keys: Vec<&str> = paths_obj.keys().map(|k| k.as_str()).collect();

    // The macro prefixes with /api/{version}, so the full paths are
    // /api/v1/users and /api/v1/users/{id}.
    assert!(
        keys.contains(&"/api/v1/users"),
        "expected /api/v1/users in paths, got {:?}",
        keys
    );
    assert!(
        keys.contains(&"/api/v1/users/{id}"),
        "expected /api/v1/users/{{id}} in paths, got {:?}",
        keys
    );
}

/// The path-parameter endpoint must use the GET method operation (the
/// `PathItem.get` field), confirming `http_method()` mapping reaches the
/// generated `Operation`.
#[test]
fn path_parameter_endpoint_uses_get_operation() {
    let spec = generate_openapi_spec();
    let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
    let paths_obj = paths_json.as_object().expect("paths is a JSON object");

    let path_item = paths_obj
        .get("/api/v1/users/{id}")
        .expect("users/{id} path must exist");
    assert!(
        path_item.get("get").is_some(),
        "PathItem for /api/v1/users/{{id}} must have a `get` operation"
    );
}

/// The summary emitted by the macro must match the `description` argument
/// passed to `#[service_api]` (the macro currently maps description into the
/// `summary` field of `OpenApiRouteInfo`).
#[test]
fn endpoint_summary_matches_macro_description() {
    let spec = generate_openapi_spec();
    let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
    let paths_obj = paths_json.as_object().expect("paths is a JSON object");

    let path_item = paths_obj
        .get("/api/v1/users")
        .expect("users path must exist");
    let get_op = path_item
        .get("get")
        .expect("must have GET operation")
        .as_object()
        .expect("get op is object");

    // The macro passes `description` into both `summary` and `description`
    // fields of `OpenApiRouteInfo`; verify at least the summary is present.
    let summary = get_op
        .get("summary")
        .and_then(|v| v.as_str())
        .expect("summary must be present");
    assert!(
        summary.contains("List all users"),
        "summary must mention 'List all users', got: {}",
        summary
    );
}

/// `OpenApiBuilder` must allow customizing the title/version independently of
/// the registered routes. The routes themselves are always sourced from the
/// global inventory.
#[test]
fn custom_builder_preserves_routes_with_custom_metadata() {
    let spec = OpenApiBuilder::new()
        .title("Custom Service")
        .version("9.9.9")
        .build();
    assert_eq!(spec.info.title, "Custom Service");
    assert_eq!(spec.info.version, "9.9.9");

    // Routes are still collected from inventory regardless of builder metadata.
    let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
    let paths_obj = paths_json.as_object().expect("paths is a JSON object");
    assert!(
        paths_obj.keys().any(|k| k == "/api/v1/users"),
        "routes must still be collected with custom builder"
    );
}

/// The default spec from `generate_openapi_spec()` must use the crate name
/// and the compile-time crate version.
#[test]
fn default_spec_uses_crate_identity() {
    let spec = generate_openapi_spec();
    assert_eq!(spec.info.title, "SDForge API");
    assert_eq!(spec.info.version, env!("CARGO_PKG_VERSION"));
}

/// `operation_id` should be synthesized as `{version}_{path}` to give each
/// operation a stable identifier for client generators.
#[test]
fn operation_id_is_versioned_path() {
    let spec = generate_openapi_spec();
    let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
    let paths_obj = paths_json.as_object().expect("paths is a JSON object");

    let path_item = paths_obj
        .get("/api/v1/users")
        .expect("users path must exist");
    let get_op = path_item
        .get("get")
        .expect("must have GET operation")
        .as_object()
        .expect("get op is object");

    let op_id = get_op
        .get("operationId")
        .and_then(|v| v.as_str())
        .expect("operationId must be present");
    assert!(
        op_id.starts_with("v1_"),
        "operationId must start with version prefix, got: {}",
        op_id
    );
}
