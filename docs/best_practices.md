//! Best practices documentation for Axiom framework
//!
//! # Axiom Best Practices Guide
//!
//! This guide covers best practices for using the Axiom framework effectively.
//!
//! ## Table of Contents
//!
//! 1. [API Design](#api-design)
//! 2. [Error Handling](#error-handling)
//! 3. [Feature Selection](#feature-selection)
//! 4. [Performance Optimization](#performance-optimization)
//! 5. [Security Considerations](#security-considerations)
//! 6. [Testing Strategies](#testing-strategies)
//!
//! ## API Design
//!
//! ### Use Descriptive Names
//!
//! Choose clear, descriptive names for your APIs:
//!
//! ```rust
//! // Good
//! #[service_api(
//!     name = "get_user_profile",
//!     version = "v1",
//!     path = "/users/:id/profile",
//!     method = "GET",
//! )]
//!
//! // Avoid - too vague
//! #[service_api(
//!     name = "get_data",
//!     version = "v1",
//!     path = "/data",
//!     method = "GET",
//! )]
//! ```
//!
//! ### Version Your APIs
//!
//! Always include a version in your API path:
//!
//! ```rust
//! #[service_api(
//!     name = "get_user",
//!     version = "v1",  // Always include version
//!     path = "/users/:id",
//!     method = "GET",
//! )]
//! ```
//!
//! ### Use Consistent Naming Conventions
//!
//! - HTTP methods: GET, POST, PUT, DELETE, PATCH
//! - Path naming: `/plural-resource/:id` for individual resources
//! - Tool names: `snake_case` for MCP
//!
//! ## Error Handling
//!
//! ### Define a Custom Error Enum
//!
//! Create a comprehensive error enum for your application:
//!
//! ```rust
//! #[derive(Debug, thiserror::Error, Serialize, Deserialize)]
//! pub enum MyApiError {
//!     #[error("Resource not found: {resource}")]
//!     NotFound { resource: String, resource_id: Option<String> },
//!
//!     #[error("Validation failed: {field}")]
//!     Validation { field: String, message: String },
//!
//!     #[error("Authentication required")]
//!     Unauthorized,
//!
//!     #[error("Permission denied: {permission}")]
//!     Forbidden { permission: String },
//!
//!     #[error("Rate limit exceeded")]
//!     RateLimited { limit: u32, retry_after: u64 },
//!
//!     #[error("Internal error: {message}")]
//!     Internal { message: String, error_id: String },
//! }
//!
//! impl From<MyApiError> for ServiceError {
//!     fn from(err: MyApiError) -> Self {
//!         match err {
//!             MyApiError::NotFound { resource, resource_id } => {
//!                 ServiceError::with_details(
//!                     "NOT_FOUND",
//!                     format!("{} not found", resource),
//!                     serde_json::json!({ "resource": resource, "resource_id": resource_id }),
//!                     404,
//!                 )
//!             }
//!             // ... handle other variants
//!         }
//!     }
//! }
//! ```
//!
//! ### Use Specific Error Codes
//!
//! Provide actionable error messages:
//!
//! ```rust
//! // Good - specific error
//! Err(MyApiError::Validation {
//!     field: "email".to_string(),
//!     message: "Invalid email format".to_string(),
//! })
//!
//! // Avoid - generic error
//! Err(MyApiError::BadRequest)
//! ```
//!
//! ## Feature Selection
//!
//! ### Choose Minimal Features
//!
//! Only enable features you need:
//!
//! ```toml
//! # For HTTP-only service
//! [dependencies]
//! axiom = { version = "0.1", features = ["http"] }
//!
//! # For MCP-only (AI tools)
//! axiom = { version = "0.1", features = ["mcp"] }
//!
//! # For full-featured service
//! axiom = { version = "0.1", features = ["full"] }
//! ```
//!
//! ### Understand Feature Interactions
//!
//! - `streaming` requires `http`
//! - `timestamp` and `logging` are independent
//! - `security` adds validation overhead
//!
//! ## Performance Optimization
//!
//! ### Minimize Serialization Overhead
//!
//! Use `#[serde(skip_serializing_if = "...")]` for optional fields:
//!
//! ```rust
//! #[derive(Serialize)]
//! pub struct Response {
//!     pub data: User,
//!     #[serde(skip_serializing_if = "Option::is_none")]
//!     pub meta: Option<Meta>,
//! }
//! ```
//!
//! ### Use Appropriate HTTP Methods
//!
//! - `GET`: Read-only operations
//! - `POST`: Create resources
//! - `PUT`: Replace resources
//! - `PATCH`: Partial updates
//! - `DELETE`: Remove resources
//!
//! ### Avoid Blocking in Handlers
//!
//! ```rust
//! // Good - async operation
//! #[service_api(path = "/users", method = "GET")]
//! async fn get_users() -> Result<Vec<User>, ApiError> {
//!     let users = User::find_all().await?;  // Non-blocking
//!     Ok(users)
//! }
//!
//! // Avoid - blocking call
//! #[service_api(path = "/users", method = "GET")]
//! async fn get_users() -> Result<Vec<User>, ApiError> {
//!     let users = blocking_db_call()?;  // This blocks the async runtime!
//!     Ok(users)
//! }
//! ```
//!
//! ## Security Considerations
//!
//! ### Validate Input Early
//!
//! ```rust
//! #[service_api(path = "/users", method = "POST")]
//! async fn create_user(req: CreateUserRequest) -> Result<User, ApiError> {
//!     // Validate before processing
//!     if req.email.len() > 254 {
//!         return Err(ApiError::InvalidInput {
//!             message: "Email too long".to_string(),
//!             field: Some("email".to_string()),
//!             value: None,
//!         });
//!     }
//!     // ... continue with creation
//! }
//! ```
//!
//! ### Use Appropriate HTTP Status Codes
//!
//! | Status Code | Meaning | Usage |
//! |-------------|---------|-------|
//! | 200 | OK | Successful GET, PUT, PATCH |
//! | 201 | Created | Successful POST (new resource) |
//! | 204 | No Content | Successful DELETE |
//! | 400 | Bad Request | Invalid client input |
//! | 401 | Unauthorized | Missing/invalid authentication |
//! | 403 | Forbidden | Valid auth but no permission |
//! | 404 | Not Found | Resource doesn't exist |
//! | 429 | Too Many Requests | Rate limit exceeded |
//! | 500 | Internal Server Error | Server-side failure |
//!
//! ## Testing Strategies
//!
//! ### Unit Test Individual Functions
//!
//! ```rust
//! #[cfg(test)]
//! mod tests {
//!     use super::*;
//!
//!     #[tokio::test]
//!     async fn test_get_user_success() {
//!         let user = get_user(1).await.unwrap();
//!         assert_eq!(user.id, 1);
//!     }
//!
//!     #[tokio::test]
//!     async fn test_get_user_not_found() {
//!         let result = get_user(999).await;
//!         assert!(result.is_err());
//!     }
//! }
//! ```
//!
//! ### Integration Test with HTTP Client
//!
//! ```rust
//! #[tokio::test]
//! async fn test_full_flow() {
//!     let app = http::build();
//!
//!     // Create user
//!     let response = app
//!         .clone()
//!         .oneshot(
//!             Request::builder()
//!                 .uri("/api/v1/users")
//!                 .method("POST")
//!                 .body(Body::from(r#"{"name":"Test","email":"test@example.com"}"#))
//!                 .unwrap(),
//!         )
//!         .await
//!         .unwrap();
//!
//!     assert_eq!(response.status(), StatusCode::CREATED);
//! }
//! ```
//!
//! ## Module Organization
//!
//! ### Group Related APIs
//!
//! ```rust
//! #[service_module(prefix = "/users")]
//! mod user_api {
//!     use super::*;
//!
//!     #[service_api(path = "/", method = "GET")]
//!     async fn list_users() { ... }
//!
//!     #[service_api(path = "/:id", method = "GET")]
//!     async fn get_user() { ... }
//!
//!     #[service_api(path = "/:id/posts", method = "GET")]
//!     async fn get_user_posts() { ... }
//! }
//!
//! #[service_module(prefix = "/products")]
//! mod product_api {
//!     use super::*;
//!
//!     // Product-related endpoints
//! }
//! ```
//!
//! ### Avoid Deep Nesting
//!
//! Keep paths shallow for better usability:
//!
//! ```rust
//! // Good - shallow nesting
//! /api/v1/users/:id/profile
//!
//! // Avoid - deeply nested
//! /api/v1/organizations/:org_id/teams/:team_id/members/:id
//! ```
//!
//! ## Common Patterns
//!
//! ### Pagination
//!
//! ```rust
//! #[service_api(path = "/users", method = "GET")]
//! async fn list_users(
//!     page: u32,
//!     page_size: u32,
//! ) -> Result<Page<User>, ApiError> {
//!     let offset = (page - 1) * page_size;
//!     let users = User::find_paginated(offset, page_size).await?;
//!     let total = User::count().await?;
//!
//!     Ok(Page {
//!         items: users,
//!         total,
//!         page,
//!         page_size,
//!     })
//! }
//! ```
//!
//! ### Bulk Operations
//!
//! ```rust
//! #[service_api(path = "/users/bulk-delete", method = "POST")]
//! async fn bulk_delete_users(
//!     ids: Vec<u64>,
//! ) -> Result<BulkDeleteResult, ApiError> {
//!     let deleted = User::delete_by_ids(&ids).await?;
//!     Ok(BulkDeleteResult { deleted_count: deleted.len() })
//! }
//! ```
//!
//! ### Soft Delete
//!
//! ```rust
//! #[service_api(path = "/users/:id", method = "DELETE")]
//! async fn delete_user(id: u64) -> Result<(), ApiError> {
//!     // Instead of actual deletion, mark as deleted
//!     User::soft_delete(id).await?;
//!     Ok(())
//! }
//! ```
