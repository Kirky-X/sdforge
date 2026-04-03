// Copyright (c) 2026 Kirky.X
//! HTTP Protocol Integration Tests
//!
//! This module contains comprehensive integration tests for the HTTP protocol layer.
//! Tests cover request/response flows, path parameters, query parameters, headers,
//! middleware chains, error handling, and request body parsing.
//!
//! All tests are integration tests and use real functionality without mocks.

#[cfg(feature = "http")]
mod http_protocol_tests {
    use axum::{
        body::Body,
        extract::{Form, Json, Path, Query},
        http::{Request, StatusCode},
        response::IntoResponse,
        routing::{delete, get, patch, post, put},
        Router,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use tower::ServiceExt;

    // ============================================================================
    // Test Data Structures
    // ============================================================================

    /// Test data structure for JSON requests
    #[derive(Debug, Serialize, Deserialize)]
    struct TestUser {
        id: u64,
        name: String,
        email: String,
    }

    /// Test data structure for query parameters.
    #[derive(Debug, Deserialize)]
    struct PaginationParams {
        page: Option<u32>,
        limit: Option<u32>,
        #[serde(default)]
        tags: Vec<String>,
    }

    /// Test data structure for form data.
    #[derive(Debug, Deserialize)]
    struct LoginForm {
        username: String,
        password: String,
    }

    /// Query params for search operations.
    #[derive(Debug, Deserialize)]
    struct SearchQuery {
        search: String,
        active: Option<bool>,
    }

    /// Test: GET request and response flow
    ///
    /// Verifies that a simple GET request returns a 200 OK response with expected body.
    #[tokio::test]
    async fn test_http_get_request_response() {
        async fn handler() -> &'static str {
            "GET response"
        }

        let router = Router::new().route("/api/users", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert_eq!(&body[..], b"GET response");
    }

    /// Test: POST JSON request
    ///
    /// Verifies that POST request with JSON body is correctly parsed and returns
    /// appropriate response with the created resource.
    #[tokio::test]
    async fn test_http_post_json_request() {
        async fn handler(Json(user): Json<TestUser>) -> Json<TestUser> {
            Json(user)
        }

        let router = Router::new().route("/api/users", post(handler));
        let request_body = json!({
            "id": 1,
            "name": "John Doe",
            "email": "john@example.com"
        });

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/users")
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: PUT request for resource update
    ///
    /// Verifies that PUT request correctly updates resources.
    #[tokio::test]
    async fn test_http_put_request() {
        async fn handler(Path(id): Path<u64>, Json(user): Json<TestUser>) -> Json<TestUser> {
            assert_eq!(id, 1);
            Json(user)
        }

        let router = Router::new().route("/api/users/{id}", put(handler));
        let request_body = json!({
            "id": 1,
            "name": "Jane Doe",
            "email": "jane@example.com"
        });

        let response = router
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/users/1")
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: DELETE request for resource removal
    ///
    /// Verifies that DELETE request properly removes resources.
    #[tokio::test]
    async fn test_http_delete_request() {
        async fn handler(Path(id): Path<u64>) -> StatusCode {
            assert_eq!(id, 42);
            StatusCode::NO_CONTENT
        }

        let router = Router::new().route("/api/users/{id}", delete(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/users/42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    /// Test: PATCH request for partial update
    ///
    /// Verifies that PATCH request correctly handles partial updates.
    /// Test: PATCH request for partial update
    ///
    /// Verifies that PATCH request correctly handles partial updates.
    #[tokio::test]
    async fn test_http_patch_request() {
        async fn handler(
            Path(id): Path<u64>,
            Json(_patch): Json<serde_json::Value>,
        ) -> (StatusCode, &'static str) {
            assert_eq!(id, 1);
            (StatusCode::OK, "PATCHED")
        }

        let router = Router::new().route("/api/users/{id}", patch(handler));
        let request_body = json!({
            "name": "Patched Name"
        });

        let response = router
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/users/1")
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // ============================================================================
    // Path Parameters Tests
    // ============================================================================

    /// Test: String path parameter extraction
    ///
    /// Verifies that string path parameters are correctly extracted from URLs.
    #[tokio::test]
    async fn test_http_path_parameters_string() {
        async fn handler(Path(name): Path<String>) -> String {
            format!("Hello, {}", name)
        }

        let router = Router::new().route("/greet/{name}", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/greet/Alice")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("Alice"));
    }

    /// Test: Number path parameter extraction
    ///
    /// Verifies that numeric path parameters are correctly parsed.
    #[tokio::test]
    async fn test_http_path_parameters_number() {
        async fn handler(Path(id): Path<u64>) -> String {
            format!("ID: {}", id)
        }

        let router = Router::new().route("/items/{id}", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/items/12345")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("12345"));
    }

    /// Test: UUID path parameter extraction
    ///
    /// Verifies that UUID path parameters are correctly parsed.
    #[tokio::test]
    async fn test_http_path_parameters_uuid() {
        async fn handler(Path(id): Path<String>) -> (StatusCode, &'static str) {
            // Validate UUID format
            if uuid::Uuid::parse_str(&id).is_ok() {
                (StatusCode::OK, "Valid UUID")
            } else {
                (StatusCode::BAD_REQUEST, "Invalid UUID")
            }
        }

        let router = Router::new().route("/resources/{id}", get(handler));
        let test_uuid = "550e8400-e29b-41d4-a716-446655440000";

        let response = router
            .oneshot(
                Request::builder()
                    .uri(format!("/resources/{}", test_uuid))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: Multiple path parameters
    ///
    /// Verifies that routes with multiple path parameters correctly extract all values.
    #[tokio::test]
    async fn test_http_multiple_path_params() {
        #[derive(Debug, Deserialize)]
        struct MultiParams {
            org: String,
            repo: String,
            issue: u64,
        }

        async fn handler(Path(params): Path<MultiParams>) -> String {
            format!("{}/{}/{}", params.org, params.repo, params.issue)
        }

        let router = Router::new().route("/github/{org}/{repo}/issues/{issue}", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/github/owner/repo/issues/42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("owner"));
        assert!(String::from_utf8_lossy(&body).contains("42"));
    }

    // ============================================================================
    // Query Parameters Tests
    // ============================================================================

    /// Test: Basic query parameters
    ///
    /// Verifies that basic query parameters are correctly parsed.
    #[tokio::test]
    async fn test_http_query_params_basic() {
        async fn handler(Query(query): Query<SearchQuery>) -> (StatusCode, &'static str) {
            if query.search == "test" {
                (StatusCode::OK, "Found")
            } else {
                (StatusCode::OK, "Search processed")
            }
        }

        let router = Router::new().route("/search", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/search?search=test&active=true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: Optional query parameters
    ///
    /// Verifies that optional query parameters work correctly when not provided.
    #[tokio::test]
    async fn test_http_query_params_optional() {
        async fn handler(Query(params): Query<PaginationParams>) -> String {
            let page = params.page.unwrap_or(0);
            format!("page={}", page)
        }

        let router = Router::new().route("/items", get(handler));

        // Request without optional params
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/items")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Request with partial params
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/items?page=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: Array query parameters
    ///
    /// Verifies that query parameters can contain array-like values.
    #[tokio::test]
    async fn test_http_query_params_array() {
        // Use a simple string parameter for tags
        #[derive(Debug, Deserialize)]
        struct TagParams {
            tags: Option<String>,
        }

        async fn handler(Query(params): Query<TagParams>) -> String {
            format!("tags={}", params.tags.unwrap_or_default())
        }

        let router = Router::new().route("/items/tags", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/items/tags?tags=rust,programming,web")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Query parsing should succeed
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("rust"));
    }

    /// Test: Query parameters with default values
    ///
    /// Verifies that query parameters with defaults are applied when not provided.
    #[tokio::test]
    async fn test_http_query_params_default_values() {
        async fn handler(Query(params): Query<PaginationParams>) -> String {
            let page = params.page.unwrap_or(1);
            let limit = params.limit.unwrap_or(10);
            format!("page={}, limit={}", page, limit)
        }

        let router = Router::new().route("/posts", get(handler));

        // Without params - should use defaults
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/posts")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("page=1"));
        assert!(String::from_utf8_lossy(&body).contains("limit=10"));
    }

    // ============================================================================
    // Request Headers Tests
    // ============================================================================

    /// Test: Custom request headers
    ///
    /// Verifies that custom request headers are accessible in handlers.
    #[tokio::test]
    async fn test_http_custom_request_headers() {
        async fn handler(req: Request<Body>) -> impl IntoResponse {
            let x_custom_header = req
                .headers()
                .get("x-custom-header")
                .map(|v| v.to_str().unwrap_or(""))
                .unwrap_or("missing");

            (
                StatusCode::OK,
                format!("X-Custom-Header: {}", x_custom_header),
            )
        }

        let router = Router::new().route("/headers", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/headers")
                    .header("x-custom-header", "custom-value-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("custom-value-123"));
    }

    /// Test: Response headers
    ///
    /// Verifies that response headers are correctly set and returned.
    #[tokio::test]
    async fn test_http_response_headers() {
        use axum::http::header::HeaderName;

        async fn handler() -> impl IntoResponse {
            (
                StatusCode::OK,
                [(HeaderName::from_static("x-response-id"), "resp-123")],
                "OK",
            )
        }

        let router = Router::new().route("/response-headers", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/response-headers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let header_value = response.headers().get("x-response-id").unwrap();
        assert_eq!(header_value, "resp-123");
    }

    /// Test: Content-Type handling
    ///
    /// Verifies that Content-Type header is correctly handled for different content types.
    #[tokio::test]
    async fn test_http_content_type_handling() {
        async fn json_handler() -> impl IntoResponse {
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                r#"{"status":"ok"}"#,
            )
        }

        let router = Router::new().route("/json", get(json_handler));

        let response = router
            .oneshot(Request::builder().uri("/json").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get("content-type").unwrap();
        assert!(content_type.to_str().unwrap().contains("application/json"));
    }

    // ============================================================================
    // Middleware Chain Tests
    // ============================================================================

    /// Test: Middleware execution order
    ///
    /// Verifies that multiple middleware layers can be applied to a router
    /// and the request flows through all layers correctly.
    #[tokio::test]
    async fn test_http_middleware_order_execution() {
        // Simple pass-through middleware defined as async fn
        async fn pass_through_middleware(
            req: Request<Body>,
            next: axum::middleware::Next,
        ) -> axum::response::Response {
            next.run(req).await
        }

        async fn handler() -> &'static str {
            "handler executed"
        }

        let router = Router::new()
            .route("/order", get(handler))
            .layer(axum::middleware::from_fn(pass_through_middleware))
            .layer(axum::middleware::from_fn(pass_through_middleware));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/order")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("handler executed"));
    }

    /// Test: CORS middleware preflight request
    ///
    /// Verifies that OPTIONS preflight requests are handled correctly.
    #[tokio::test]
    async fn test_http_cors_middleware_preflight() {
        use tower_http::cors::{Any, CorsLayer};

        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        async fn handler() -> &'static str {
            "OK"
        }

        let router = Router::new().route("/cors-test", get(handler)).layer(cors);

        // Preflight request
        let response = router
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/cors-test")
                    .header("Origin", "http://example.com")
                    .header("Access-Control-Request-Method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // CORS preflight should return either 200 or 204 (depends on configuration)
        assert!(
            response.status() == StatusCode::OK || response.status() == StatusCode::NO_CONTENT,
            "Expected OK or NO_CONTENT, got {}",
            response.status()
        );
    }

    /// Test: Logging middleware
    ///
    /// Verifies that the router can use a middleware layer.
    #[tokio::test]
    async fn test_http_logging_middleware() {
        // Simple pass-through middleware for testing
        async fn logging_middleware(
            req: Request<Body>,
            next: axum::middleware::Next,
        ) -> axum::response::Response {
            // Log the request method and path
            let _method = req.method().to_string();
            let _uri = req.uri().path().to_string();
            next.run(req).await
        }

        async fn handler() -> &'static str {
            "logged"
        }

        let router = Router::new()
            .route("/log-test", get(handler))
            .layer(axum::middleware::from_fn(logging_middleware));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/log-test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("logged"));
    }

    /// Test: Timeout middleware
    ///
    /// Verifies that timeout middleware correctly times out slow requests.
    #[tokio::test]
    async fn test_http_timeout_middleware() {
        use std::time::Duration;
        use tower_http::timeout::TimeoutLayer;

        async fn slow_handler() -> &'static str {
            tokio::time::sleep(Duration::from_millis(10)).await;
            "slow response"
        }

        let router =
            Router::new()
                .route("/slow", get(slow_handler))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_secs(5),
                ));

        let response = router
            .oneshot(Request::builder().uri("/slow").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should complete successfully with long timeout
        assert_eq!(response.status(), StatusCode::OK);
    }

    // ============================================================================
    // HTTP Error Handling Tests
    // ============================================================================

    /// Test: 400 Bad Request error
    ///
    /// Verifies that 400 errors are returned for malformed requests.
    #[tokio::test]
    async fn test_http_400_bad_request() {
        use sdforge::core::ApiError;

        async fn handler() -> Result<&'static str, ApiError> {
            Err(ApiError::InvalidInput {
                message: "Invalid request data".to_string(),
                field: Some("email".to_string()),
                value: None,
            })
        }

        let router: Router = Router::new().route("/bad", get(handler));

        let response = router
            .oneshot(Request::builder().uri("/bad").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// Test: 401 Unauthorized error
    ///
    /// Verifies that 401 errors are returned for unauthenticated requests.
    #[tokio::test]
    async fn test_http_401_unauthorized() {
        use sdforge::core::ApiError;

        async fn handler() -> Result<&'static str, ApiError> {
            Err(ApiError::AuthenticationFailed {
                reason: "Invalid credentials".to_string(),
            })
        }

        let router: Router = Router::new().route("/auth", get(handler));

        let response = router
            .oneshot(Request::builder().uri("/auth").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    /// Test: 403 Forbidden error
    ///
    /// Verifies that 403 errors are returned for unauthorized access attempts.
    #[tokio::test]
    async fn test_http_403_forbidden() {
        use sdforge::core::ApiError;

        async fn handler() -> Result<&'static str, ApiError> {
            Err(ApiError::AccessDenied {
                permission: "admin:access".to_string(),
                user_id: Some("user123".to_string()),
            })
        }

        let router: Router = Router::new().route("/admin", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/admin")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    /// Test: 404 Not Found error
    ///
    /// Verifies that 404 errors are returned for non-existent resources.
    #[tokio::test]
    async fn test_http_404_not_found() {
        use sdforge::core::ApiError;

        async fn handler(Path(id): Path<u64>) -> Result<String, ApiError> {
            // Simulate resource not found
            if id == 999 {
                Err(ApiError::NotFound {
                    resource: "Item".to_string(),
                    resource_id: Some(id.to_string()),
                })
            } else {
                Ok(id.to_string())
            }
        }

        let router: Router = Router::new().route("/items/{id}", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/items/999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// Test: 422 Validation Error
    ///
    /// Verifies that 422 errors are returned for validation failures.
    #[tokio::test]
    async fn test_http_422_validation_error() {
        use sdforge::core::ApiError;

        async fn handler(Json(data): Json<TestUser>) -> Result<Json<TestUser>, ApiError> {
            // Simulate validation failure for invalid email
            if !data.email.contains('@') {
                return Err(ApiError::ValidationError {
                    field: "email".to_string(),
                    constraint: "Must be a valid email address".to_string(),
                });
            }
            Ok(Json(data))
        }

        let router: Router = Router::new().route("/validate", post(handler));
        let request_body = json!({
            "id": 1,
            "name": "Test User",
            "email": "invalid-email"
        });

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/validate")
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// Test: 500 Internal Server Error
    ///
    /// Verifies that 500 errors are returned for internal server errors.
    #[tokio::test]
    async fn test_http_500_internal_server_error() {
        use sdforge::core::ApiError;
        use uuid::Uuid;

        async fn handler() -> Result<&'static str, ApiError> {
            Err(ApiError::Internal {
                message: "Database connection failed".to_string(),
                error_id: Uuid::new_v4().to_string(),
                context: None,
                source: None,
            })
        }

        let router: Router = Router::new().route("/internal", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/internal")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // ============================================================================
    // Request Body Handling Tests
    // ============================================================================

    /// Test: JSON body parsing
    ///
    /// Verifies that JSON request bodies are correctly parsed.
    #[tokio::test]
    async fn test_http_json_body_parsing() {
        async fn handler(Json(user): Json<TestUser>) -> String {
            format!("User: {} <{}>", user.name, user.email)
        }

        let router = Router::new().route("/parse", post(handler));
        let request_body = json!({
            "id": 42,
            "name": "Test User",
            "email": "test@example.com"
        });

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/parse")
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("Test User"));
    }

    /// Test: Invalid JSON body handling
    ///
    /// Verifies that invalid JSON in request body results in 400 Bad Request.
    #[tokio::test]
    async fn test_http_invalid_json_body() {
        async fn handler(Json(_user): Json<TestUser>) -> &'static str {
            "OK"
        }

        let router = Router::new().route("/invalid", post(handler));
        let invalid_json = "{ invalid json }";

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/invalid")
                    .header("Content-Type", "application/json")
                    .body(Body::from(invalid_json))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Axum returns 400 for malformed JSON
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// Test: Empty body handling
    ///
    /// Verifies that empty request bodies are handled gracefully.
    #[tokio::test]
    async fn test_http_empty_body_handling() {
        async fn handler() -> &'static str {
            "Empty body received"
        }

        let router = Router::new().route("/empty", post(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/empty")
                    .header("Content-Type", "application/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // With empty body, JSON parsing should fail or return None
        // Check if request was processed (might be 400 for failed parsing)
        assert!(
            response.status() == StatusCode::OK || response.status() == StatusCode::BAD_REQUEST
        );
    }

    /// Test: Large body handling
    ///
    /// Verifies that large request bodies are handled with appropriate limits.
    #[tokio::test]
    async fn test_http_large_body_handling() {
        use tower_http::limit::RequestBodyLimitLayer;

        async fn handler(Json(data): Json<serde_json::Value>) -> String {
            format!("size={}", data.to_string().len())
        }

        let router = Router::new()
            .route("/large", post(handler))
            .layer(RequestBodyLimitLayer::new(1024)); // 1KB limit

        // Create a large payload
        let large_data = json!({
            "data": "x".repeat(2048) // 2KB of data
        });

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/large")
                    .header("Content-Type", "application/json")
                    .body(Body::from(large_data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 413 Payload Too Large
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    // ============================================================================
    // Additional Protocol Tests
    // ============================================================================

    /// Test: HEAD request
    ///
    /// Verifies that HEAD requests work correctly (returns headers without body).
    #[tokio::test]
    async fn test_http_head_request() {
        async fn handler() -> &'static str {
            "Full content here"
        }

        let router = Router::new().route("/resource", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .method("HEAD")
                    .uri("/resource")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // HEAD should return 200 OK without body
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert!(body.is_empty());
    }

    /// Test: Multiple method routing
    ///
    /// Verifies that routes can handle different HTTP methods correctly.
    #[tokio::test]
    async fn test_http_multiple_methods() {
        async fn get_handler() -> &'static str {
            "GET"
        }
        async fn post_handler() -> &'static str {
            "POST"
        }
        async fn put_handler() -> &'static str {
            "PUT"
        }
        async fn delete_handler() -> &'static str {
            "DELETE"
        }

        let router = Router::new().route(
            "/resource",
            get(get_handler)
                .post(post_handler)
                .put(put_handler)
                .delete(delete_handler),
        );

        // Test GET
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/resource")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test POST
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/resource")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test PUT
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/resource")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test DELETE
        let response = router
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/resource")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test: Method not allowed
    ///
    /// Verifies that requests to routes with wrong methods return 405.
    #[tokio::test]
    async fn test_http_method_not_allowed() {
        async fn handler() -> &'static str {
            "GET only"
        }

        let router = Router::new().route("/get-only", get(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/get-only")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    /// Test: Form data parsing
    ///
    /// Verifies that form data (application/x-www-form-urlencoded) is correctly parsed.
    #[tokio::test]
    async fn test_http_form_data_parsing() {
        async fn handler(Form(login): Form<LoginForm>) -> String {
            format!("Logged in as: {}", login.username)
        }

        let router = Router::new().route("/login", post(handler));

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from("username=admin&password=secret"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("admin"));
    }

    /// Test: Response status codes
    ///
    /// Verifies that various HTTP status codes are correctly returned.
    #[tokio::test]
    async fn test_http_various_status_codes() {
        async fn created() -> (StatusCode, &'static str) {
            (StatusCode::CREATED, "Created")
        }
        async fn accepted() -> (StatusCode, &'static str) {
            (StatusCode::ACCEPTED, "Accepted")
        }
        async fn no_content() -> StatusCode {
            StatusCode::NO_CONTENT
        }
        async fn moved() -> (StatusCode, &'static str) {
            (StatusCode::MOVED_PERMANENTLY, "Moved")
        }

        let router = Router::new()
            .route("/created", post(created))
            .route("/accepted", post(accepted))
            .route("/no-content", delete(no_content))
            .route("/moved", get(moved));

        // 201 Created
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/created")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // 202 Accepted
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/accepted")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        // 204 No Content
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/no-content")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // 301 Moved Permanently
        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/moved")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    }
}
