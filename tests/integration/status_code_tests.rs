// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! E2E tests for `forge-success-status-code` change.
//!
//! Verifies that:
//! - (a) `#[forge(method = "post", status = 201)]` on a bare-type fn produces
//!   HTTP 201.
//! - (b) `ServiceResponse::success_with_status(data, 201)` produces HTTP 201
//!   even without the macro `status` arg (dynamic path).
//! - (c) A bare-type fn without `status` produces HTTP 200 (zero-breaking
//!   regression).
//!
//! Run with: `cargo test --test status_code_tests --features http`.

#![cfg(feature = "http")]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use sdforge::prelude::ServiceResponse;
use sdforge::{forge, http};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;

// ============================================================================
// Test fixtures
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct CreateUserPayload {
    name: String,
    email: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

// ============================================================================
// (a) Bare type + `#[forge(status = 201)]` → HTTP 201
// ============================================================================

#[forge(
    name = "status_code_create_with_status",
    version = "v1",
    path = "/create-with-status",
    method = "POST",
    status = 201,
    description = "Create user with explicit status=201"
)]
async fn create_with_status(payload: CreateUserPayload) -> User {
    User {
        id: 1,
        name: payload.name,
        email: payload.email,
    }
}

// ============================================================================
// (b) `ServiceResponse::success_with_status(data, 201)` → HTTP 201 (dynamic)
// ============================================================================

#[forge(
    name = "status_code_create_service_response",
    version = "v1",
    path = "/create-service-response",
    method = "POST",
    description = "Create user with ServiceResponse::success_with_status"
)]
async fn create_service_response(payload: CreateUserPayload) -> ServiceResponse<User> {
    let user = User {
        id: 2,
        name: payload.name,
        email: payload.email,
    };
    ServiceResponse::success_with_status(user, 201)
}

// ============================================================================
// (c) Bare type without `status` → HTTP 200 (zero-breaking regression)
// ============================================================================

#[forge(
    name = "status_code_create_default",
    version = "v1",
    path = "/create-default",
    method = "POST",
    description = "Create user with default 200"
)]
async fn create_default(payload: CreateUserPayload) -> User {
    User {
        id: 3,
        name: payload.name,
        email: payload.email,
    }
}

// ============================================================================
// Tests
// ============================================================================

/// Build the HTTP router from all registered `#[forge]` routes.
fn build_router() -> axum::Router {
    http::build()
}

/// Helper: send a POST request with a JSON body and return the response.
async fn send_post(uri: &str, body: serde_json::Value) -> axum::http::Response<Body> {
    let router = build_router();
    let body_json = serde_json::to_string(&body).unwrap();
    router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body_json))
                .unwrap(),
        )
        .await
        .unwrap()
}

fn sample_payload() -> serde_json::Value {
    serde_json::json!({
        "name": "Alice",
        "email": "alice@example.com"
    })
}

/// (a) `#[forge(status = 201)]` on a bare type fn → HTTP 201.
#[tokio::test]
async fn test_forge_status_arg_produces_201() {
    let response = send_post("/api/v1/create-with-status", sample_payload()).await;
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "expected 201 Created from #[forge(status = 201)]"
    );
}

/// (b) `ServiceResponse::success_with_status(data, 201)` → HTTP 201 (dynamic).
#[tokio::test]
async fn test_service_response_success_with_status_produces_201() {
    let response = send_post("/api/v1/create-service-response", sample_payload()).await;
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "expected 201 Created from ServiceResponse::success_with_status(_, 201)"
    );
}

/// (c) Bare type without `status` → HTTP 200 (zero-breaking regression).
#[tokio::test]
async fn test_bare_type_without_status_defaults_200() {
    let response = send_post("/api/v1/create-default", sample_payload()).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "expected 200 OK for bare type without status arg (zero-breaking)"
    );
}

/// Verify the response body is valid JSON and contains the expected user data.
#[tokio::test]
async fn test_status_201_response_body_contains_user() {
    let response = send_post("/api/v1/create-with-status", sample_payload()).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["name"], "Alice");
    assert_eq!(body_json["email"], "alice@example.com");
    assert_eq!(body_json["id"], 1);
}

/// Verify `ServiceResponse` path also serializes the `status_code` field in
/// the JSON body (clients can read it from the body as well as the HTTP
/// status header).
#[tokio::test]
async fn test_service_response_body_contains_status_code_field() {
    let response = send_post("/api/v1/create-service-response", sample_payload()).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["success"], true);
    assert_eq!(body_json["status_code"], 201);
    assert_eq!(body_json["data"]["name"], "Alice");
}

// ============================================================================
// (d) `status = 204` → HTTP 204 无 body（bare-type Result 与裸值两条路径）
// ============================================================================

#[forge(
    name = "status_code_delete_with_204",
    version = "v1",
    path = "/items-with-204/{id}",
    method = "DELETE",
    status = 204,
    description = "Delete returning 204 no-content (Result path)"
)]
async fn delete_with_204(id: u64) -> Result<(), sdforge::core::ApiError> {
    let _ = id;
    Ok(())
}

#[forge(
    name = "status_code_delete_with_204_bare",
    version = "v1",
    path = "/items-with-204-bare/{id}",
    method = "DELETE",
    status = 204,
    description = "Delete returning 204 no-content (non-Result path)"
)]
async fn delete_with_204_bare(id: u64) {
    let _ = id;
}

/// Helper: send a DELETE request and return the response.
async fn send_delete(uri: &str) -> axum::http::Response<Body> {
    let router = build_router();
    router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

/// `#[forge(status = 204)]` on a Result-returning DELETE → HTTP 204, empty body.
#[tokio::test]
async fn test_forge_status_204_delete_result_path_no_body() {
    let response = send_delete("/api/v1/items-with-204/42").await;
    assert_eq!(
        response.status(),
        StatusCode::NO_CONTENT,
        "expected 204 No Content from #[forge(status = 204)] (Result path)"
    );
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(
        body_bytes.is_empty(),
        "204 response must not carry a body, got: {:?}",
        String::from_utf8_lossy(&body_bytes)
    );
}

/// `#[forge(status = 204)]` on a bare (non-Result) DELETE → HTTP 204, empty body.
#[tokio::test]
async fn test_forge_status_204_delete_bare_path_no_body() {
    let response = send_delete("/api/v1/items-with-204-bare/7").await;
    assert_eq!(
        response.status(),
        StatusCode::NO_CONTENT,
        "expected 204 No Content from #[forge(status = 204)] (non-Result path)"
    );
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(
        body_bytes.is_empty(),
        "204 response must not carry a body, got: {:?}",
        String::from_utf8_lossy(&body_bytes)
    );
}
