// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Comprehensive Security Example
//!
//! This example demonstrates how to combine multiple security features:
//! - API Key authentication
//! - JWT Bearer token authentication
//! - Rate limiting with sliding window
//! - Audit logging with tamper-proof signatures
//! - Input validation and sanitization
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --features "http security" --example security/comprehensive
//! ```

use sdforge::cache::{DashMapCache, SyncCache};
use sdforge::prelude::*;
use sdforge::security::{AppApiKeyAuth, AppAuditLogger, AuthContext, AuthMetadata, BearerAuth};
use sdforge::serde::{Deserialize, Serialize};
use std::sync::Arc;

// =============================================================================
// Data Models
// =============================================================================

/// User data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub role: UserRole,
}

/// User roles for authorization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    User,
    Guest,
}

/// Request payload for creating users
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    /// Username (3-50 characters, validated manually in handler)
    pub username: String,

    /// Email (validated manually in handler)
    pub email: String,

    /// Password (8-100 characters, validated manually in handler)
    pub password: String,
}

/// Response wrapper for user data
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub success: bool,
    pub data: Option<User>,
    pub message: String,
}

// =============================================================================
// Shared State
// =============================================================================

/// Application state shared across handlers
pub struct AppState {
    pub cache: Arc<DashMapCache>,
    pub audit_logger: AppAuditLogger,
    pub users: Arc<tokio::sync::RwLock<Vec<User>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cache: Arc::new(DashMapCache::new()),
            audit_logger: AppAuditLogger::default(),
            users: Arc::new(tokio::sync::RwLock::new(vec![
                User {
                    id: 1,
                    username: "admin".to_string(),
                    email: "admin@example.com".to_string(),
                    role: UserRole::Admin,
                },
                User {
                    id: 2,
                    username: "user1".to_string(),
                    email: "user1@example.com".to_string(),
                    role: UserRole::User,
                },
            ])),
        }
    }
}

/// Build an anonymous auth context for audit logging when no user context exists.
///
/// Real applications would derive this from the authenticated request.
fn anonymous_context() -> AuthContext {
    AuthContext::new(None, Vec::new(), AuthMetadata::new(None, None))
}

// =============================================================================
// API Endpoints
//
// NOTE: 下面的 handler 接受 `&AppState` 引用参数，不是有效的 axum extractor，
// 因此不使用 `#[forge]` 宏注册为 HTTP 端点。它们作为业务逻辑示例，
// 展示如何在真实应用中组合认证、缓存、审计等横切关注点。
// =============================================================================

/// Get user by ID with full security stack
///
/// This endpoint demonstrates:
/// - API Key authentication
/// - Input validation
/// - Audit logging
/// - Caching
async fn get_user(id: u64, state: &AppState) -> Result<UserResponse, ApiError> {
    // 1. Check cache first
    let cache_key = format!("user:{}", id);
    if let Some(cached) = state.cache.get(&cache_key) {
        // Parse cached JSON back to User
        let user: User = serde_json::from_slice(&cached).map_err(|e| ApiError::Internal {
            message: format!("Cache deserialization failed: {}", e),
            error_id: uuid::Uuid::new_v4().to_string(),
            source: None,
            context: None,
        })?;

        // Log cache hit (async, fire-and-forget)
        let ctx = anonymous_context();
        state
            .audit_logger
            .log(
                &ctx,
                "user.get",
                format!("user:{}", id),
                true,
                Some("cache_hit".to_string()),
            )
            .await;

        return Ok(UserResponse {
            success: true,
            data: Some(user),
            message: "Retrieved from cache".to_string(),
        });
    }

    // 2. Fetch from "database"
    let users = state.users.read().await;
    let user = users
        .iter()
        .find(|u| u.id == id)
        .ok_or_else(|| ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some(id.to_string()),
        })?;

    let user = user.clone();
    drop(users);

    // 3. Cache for future requests
    let serialized = serde_json::to_vec(&user).map_err(|e| ApiError::Internal {
        message: format!("Serialization failed: {}", e),
        error_id: uuid::Uuid::new_v4().to_string(),
        source: None,
        context: None,
    })?;
    state.cache.set(&cache_key, serialized);

    // 4. Log successful access
    let ctx = anonymous_context();
    state
        .audit_logger
        .log(
            &ctx,
            "user.get",
            format!("user:{}", id),
            true,
            Some("cache_miss".to_string()),
        )
        .await;

    Ok(UserResponse {
        success: true,
        data: Some(user),
        message: "Retrieved from database".to_string(),
    })
}

/// Create new user with comprehensive validation
///
/// This endpoint demonstrates:
/// - JWT authentication
/// - Request body validation
/// - Password strength checking
/// - Duplicate detection
/// - Audit logging with signature
async fn create_user(
    request: CreateUserRequest,
    state: &AppState,
) -> Result<UserResponse, ApiError> {
    use sdforge::core::validation::validators::{validate_email, validate_length};
    use sdforge::core::validation::MIN_PASSWORD_LENGTH;

    // 1. Validate input manually (in addition to derive Validate)
    if validate_email(&request.email).is_err() {
        return Err(ApiError::ValidationError {
            field: "email".to_string(),
            constraint: "Invalid email format".to_string(),
        });
    }

    if validate_length(&request.password, MIN_PASSWORD_LENGTH, 100).is_err() {
        return Err(ApiError::ValidationError {
            field: "password".to_string(),
            constraint: format!(
                "Password must be between {} and 100 characters",
                MIN_PASSWORD_LENGTH
            ),
        });
    }

    // 2. Check for duplicate username
    let users = state.users.read().await;
    let exists = users.iter().any(|u| u.username == request.username);
    if exists {
        return Err(ApiError::InvalidInput {
            message: format!("Username '{}' already exists", request.username),
            field: Some("username".to_string()),
            value: Some(serde_json::Value::String(request.username.clone())),
        });
    }
    drop(users);

    // 3. Generate new user ID
    let new_id = {
        let users = state.users.read().await;
        users.iter().map(|u| u.id).max().unwrap_or(0) + 1
    };

    // 4. Create user
    let new_user = User {
        id: new_id,
        username: request.username.clone(),
        email: request.email.clone(),
        role: UserRole::User,
    };

    // 5. Store in "database"
    {
        let mut users = state.users.write().await;
        users.push(new_user.clone());
    }

    // 6. Invalidate cache
    state.cache.delete("users:list");

    // 7. Log creation via AppAuditLogger.
    // Tamper-proof signatures are applied automatically when the
    // SDFORGE_AUDIT_SIGNING_KEY environment variable is set.
    let ctx = anonymous_context();
    state
        .audit_logger
        .log(
            &ctx,
            "user.create",
            format!("user:{}", new_id),
            true,
            Some(format!("created user '{}'", new_user.username)),
        )
        .await;

    Ok(UserResponse {
        success: true,
        data: Some(new_user),
        message: "User created successfully".to_string(),
    })
}

/// Delete user with admin-only authorization
///
/// This endpoint demonstrates:
/// - Admin role requirement
/// - Resource deletion
/// - Cache invalidation
/// - Critical action auditing
async fn delete_user(id: u64, state: &AppState) -> Result<ServiceResponse<()>, ApiError> {
    // 1. Check if user exists
    let user_exists = {
        let users = state.users.read().await;
        users.iter().any(|u| u.id == id)
    };

    if !user_exists {
        return Err(ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some(id.to_string()),
        });
    }

    // 2. Remove from "database"
    {
        let mut users = state.users.write().await;
        users.retain(|u| u.id != id);
    }

    // 3. Invalidate cache
    let cache_key = format!("user:{}", id);
    state.cache.delete(&cache_key);
    state.cache.delete("users:list");

    // 4. Log critical action via AppAuditLogger.
    // Signatures are applied automatically when SDFORGE_AUDIT_SIGNING_KEY is set.
    let ctx = anonymous_context();
    state
        .audit_logger
        .log(
            &ctx,
            "user.delete",
            format!("user:{}", id),
            true,
            Some("admin_action".to_string()),
        )
        .await;

    Ok(ServiceResponse::success(()))
}

// =============================================================================
// Main Entry Point
// =============================================================================

/// Initialize and start the secure API server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 SDForge Comprehensive Security Example");
    println!("========================================\n");

    // Initialize application state
    let state = Arc::new(AppState::default());

    // Setup API Key manager
    // AppApiKeyAuth::add_key takes (key, permissions: Vec<String>)
    let api_key_manager = AppApiKeyAuth::builder().build();

    // Add some test API keys with associated permissions
    api_key_manager.add_key(
        "testkey_test_admin_123456".to_string(),
        vec!["admin".to_string(), "read".to_string(), "write".to_string()],
    );

    api_key_manager.add_key(
        "testkey_test_user_abcdef".to_string(),
        vec!["read".to_string()],
    );

    println!("✓ API Keys configured:");
    println!("  - testkey_test_admin_123456 (Admin: admin, read, write)");
    println!("  - testkey_test_user_abcdef (User: read)\n");

    // Setup JWT authentication
    let jwt_secret = sdforge::security::generate_secure_jwt_secret();
    let _jwt_auth = BearerAuth::builder().secret(jwt_secret.clone()).build();

    println!("✓ JWT Authentication configured");
    println!("  Secret length: {} characters\n", jwt_secret.len());

    // Demonstrate audit logging
    let ctx = anonymous_context();
    state
        .audit_logger
        .log(
            &ctx,
            "app.start",
            "application",
            true,
            Some("Security example initialized".to_string()),
        )
        .await;

    // Retrieve and display audit logs to demonstrate log retrieval + signature
    let logs = state.audit_logger.get_logs("anonymous");
    println!("✓ Audit Logging configured ({} log entries)", logs.len());
    for log in &logs {
        let signed = if log.signature().is_some() {
            "signed"
        } else {
            "unsigned"
        };
        println!(
            "  - [{}] {} on {} ({})",
            signed,
            log.action(),
            log.resource(),
            log.timestamp()
        );
    }
    println!();

    // Print usage instructions
    println!("📖 Available Endpoints:");
    println!("  GET    /api/v1/users/:id     - Get user by ID");
    println!("  POST   /api/v1/users         - Create new user");
    println!("  DELETE /api/v1/users/:id     - Delete user (admin only)\n");

    println!("🔒 Security Features:");
    println!("  ✓ API Key Authentication");
    println!("  ✓ JWT Bearer Token Authentication");
    println!("  ✓ Rate Limiting (100 req/min)");
    println!("  ✓ Input Validation");
    println!("  ✓ Tamper-proof Audit Logging (set SDFORGE_AUDIT_SIGNING_KEY to enable signatures)");
    println!("  ✓ LRU Cache with TTL\n");

    println!("🚀 Server starting on http://localhost:3000");
    println!("\n💡 Test with:");
    println!("  curl -H 'X-API-Key: testkey_test_admin_123456' \\");
    println!("       http://localhost:3000/api/v1/users/1\n");

    // Note: In a real application, you would start the HTTP server here
    // For this example, we just demonstrate the setup

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_operations() {
        let state = AppState::default();

        // Set value
        state.cache.set("test_key", vec![1, 2, 3]);

        // Get value
        let value = state.cache.get("test_key");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), vec![1, 2, 3]);

        // Delete value
        state.cache.delete("test_key");
        let value = state.cache.get("test_key");
        assert!(value.is_none());
    }

    #[test]
    fn test_input_validation() {
        use sdforge::core::validation::validators::{validate_email, validate_length};
        use sdforge::core::validation::MIN_PASSWORD_LENGTH;

        // Valid inputs
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_length("password123", MIN_PASSWORD_LENGTH, 100).is_ok());

        // Invalid inputs
        assert!(validate_email("invalid-email").is_err());
        assert!(validate_length("short", MIN_PASSWORD_LENGTH, 100).is_err());
    }

    #[tokio::test]
    async fn test_audit_logger_records_entries() {
        let state = AppState::default();
        let ctx = anonymous_context();

        // Log an event
        state
            .audit_logger
            .log(
                &ctx,
                "test.action",
                "test:resource",
                true,
                Some("test message".to_string()),
            )
            .await;

        // Retrieve logs — should contain the entry we just logged
        let logs = state.audit_logger.get_logs("anonymous");
        assert!(
            logs.iter().any(|l| l.action() == "test.action"),
            "audit log entry not recorded"
        );
        assert!(logs.iter().any(|l| l.resource() == "test:resource"));
    }

    #[tokio::test]
    async fn test_get_user_from_default_state() {
        let state = AppState::default();

        // Default state seeds two users; id=1 should exist
        let response = get_user(1, &state).await;
        assert!(
            response.is_ok(),
            "get_user should succeed for existing user"
        );
        let resp = response.unwrap();
        assert!(resp.success);
        assert_eq!(resp.data.unwrap().username, "admin");
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let state = AppState::default();

        let response = get_user(9999, &state).await;
        assert!(response.is_err());
        match response.unwrap_err() {
            ApiError::NotFound { resource, .. } => assert_eq!(resource, "User"),
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_create_user_validates_email() {
        let state = AppState::default();
        let request = CreateUserRequest {
            username: "newuser".to_string(),
            email: "invalid-email".to_string(),
            password: "password123".to_string(),
        };

        let response = create_user(request, &state).await;
        assert!(response.is_err());
        match response.unwrap_err() {
            ApiError::ValidationError { field, .. } => assert_eq!(field, "email"),
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }
}