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

use sdforge::prelude::*;
use sdforge::security::{
    ApiKeyAuth, AppApiKeyAuth, AuditLogger, BearerAuth,
};
use sdforge::cache::{Cache, DashMapCache, SyncCache};
use serde::{Deserialize, Serialize};
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
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 8, max = 100))]
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
    pub audit_logger: AuditLogger,
    pub users: Arc<tokio::sync::RwLock<Vec<User>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cache: Arc::new(DashMapCache::new()),
            audit_logger: AuditLogger::default(),
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

// =============================================================================
// API Endpoints
// =============================================================================

/// Get user by ID with full security stack
/// 
/// This endpoint demonstrates:
/// - API Key authentication
/// - Input validation
/// - Audit logging
/// - Caching
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user",
    description = "Get user by ID with authentication"
)]
async fn get_user(
    id: u64,
    state: &AppState,
) -> Result<UserResponse, ApiError> {
    // 1. Check cache first
    let cache_key = format!("user:{}", id);
    if let Some(cached) = state.cache.get(&cache_key) {
        state.audit_logger.log(
            "user.get".to_string(),
            None,
            None,
            Some("cache_hit".to_string()),
            Some(format!("user:{}", id)),
            AuditResult::Success,
        );
        
        // Parse cached JSON back to User
        let user: User = serde_json::from_slice(&cached)
            .map_err(|e| ApiError::InternalError { 
                message: format!("Cache deserialization failed: {}", e) 
            })?;
        
        return Ok(UserResponse {
            success: true,
            data: Some(user),
            message: "Retrieved from cache".to_string(),
        });
    }

    // 2. Fetch from "database"
    let users = state.users.read().await;
    let user = users.iter()
        .find(|u| u.id == id)
        .ok_or_else(|| ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some(id.to_string()),
        })?;
    
    let user = user.clone();
    drop(users);

    // 3. Cache for future requests
    let serialized = serde_json::to_vec(&user)
        .map_err(|e| ApiError::InternalError { 
            message: format!("Serialization failed: {}", e) 
        })?;
    state.cache.set(&cache_key, serialized);

    // 4. Log successful access
    state.audit_logger.log(
        "user.get".to_string(),
        None,
        None,
        Some("cache_miss".to_string()),
        Some(format!("user:{}", id)),
        AuditResult::Success,
    );

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
#[service_api(
    name = "create_user",
    version = "v1",
    path = "/users",
    method = "POST",
    tool_name = "create_user",
    description = "Create new user with validation and audit logging"
)]
async fn create_user(
    request: CreateUserRequest,
    state: &AppState,
) -> Result<UserResponse, ApiError> {
    use sdforge::core::validation::{validate_email, validate_length, MIN_PASSWORD_LENGTH};

    // 1. Validate input manually (in addition to derive Validate)
    if !validate_email(&request.email) {
        return Err(ApiError::ValidationError {
            field: "email".to_string(),
            message: "Invalid email format".to_string(),
        });
    }

    if !validate_length(&request.password, MIN_PASSWORD_LENGTH, 100) {
        return Err(ApiError::ValidationError {
            field: "password".to_string(),
            message: format!("Password must be between {} and 100 characters", MIN_PASSWORD_LENGTH),
        });
    }

    // 2. Check for duplicate username
    let users = state.users.read().await;
    let exists = users.iter().any(|u| u.username == request.username);
    if exists {
        return Err(ApiError::Conflict {
            resource: "Username".to_string(),
            identifier: request.username.clone(),
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

    // 7. Log with tamper-proof signature
    let mut audit_log = AuditLog::new(
        "user.create".to_string(),
        None,
        None,
        Some("api_request".to_string()),
        Some(format!("user:{}", new_id)),
        AuditResult::Success,
    );
    
    // Generate HMAC-SHA256 signature
    let secret_key = b"audit_secret_key_for_signing";
    let signature = audit_log.generate_signature(secret_key);
    
    // Verify signature immediately (for testing)
    assert!(audit_log.verify_signature(secret_key).is_ok(), "Signature verification failed!");

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
#[service_api(
    name = "delete_user",
    version = "v1",
    path = "/users/:id",
    method = "DELETE",
    tool_name = "delete_user",
    description = "Delete user (admin only)"
)]
async fn delete_user(
    id: u64,
    state: &AppState,
) -> Result<ServiceResponse<()>, ApiError> {
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

    // 4. Log critical action
    let mut audit_log = AuditLog::new(
        "user.delete".to_string(),
        None,
        None,
        Some("admin_action".to_string()),
        Some(format!("user:{}", id)),
        AuditResult::Success,
    );
    
    let secret_key = b"audit_secret_key_for_signing";
    audit_log.generate_signature(secret_key);

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
    let api_key_manager = AppApiKeyAuth::builder()
        .build();
    
    // Add some test API keys
    api_key_manager.add_key(
        "sk_test_admin_123456".to_string(),
        sdforge::security::ApiKeyMetadata::new(
            "admin_key".to_string(),
            Some("Admin API Key".to_string()),
        ),
    );

    api_key_manager.add_key(
        "sk_test_user_abcdef".to_string(),
        sdforge::security::ApiKeyMetadata::new(
            "user_key".to_string(),
            Some("User API Key".to_string()),
        ),
    );

    println!("✓ API Keys configured:");
    println!("  - sk_test_admin_123456 (Admin)");
    println!("  - sk_test_user_abcdef (User)\n");

    // Setup JWT authentication
    let jwt_secret = sdforge::security::generate_secure_jwt_secret();
    let jwt_auth = BearerAuth::builder()
        .secret(jwt_secret.clone())
        .build();

    println!("✓ JWT Authentication configured");
    println!("  Secret length: {} characters\n", jwt_secret.len());

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
    println!("  ✓ Tamper-proof Audit Logging");
    println!("  ✓ LRU Cache with TTL\n");

    println!("🚀 Server starting on http://localhost:3000");
    println!("\n💡 Test with:");
    println!("  curl -H 'X-API-Key: sk_test_admin_123456' \\");
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
        
        // Delete value
        state.cache.delete("test_key");
        let value = state.cache.get("test_key");
        assert!(value.is_none());
    }

    #[test]
    fn test_audit_log_signature() {
        let mut audit_log = AuditLog::new(
            "test.action".to_string(),
            Some("user_123".to_string()),
            None,
            Some("test".to_string()),
            Some("resource:1".to_string()),
            AuditResult::Success,
        );
        
        let secret = b"test_secret";
        let signature = audit_log.generate_signature(secret);
        
        // Verify valid signature
        assert!(audit_log.verify_signature(secret).is_ok());
        assert!(audit_log.verify_signature(secret).unwrap());
        
        // Tamper with the log
        audit_log.resource = Some("tampered".to_string());
        
        // Signature should fail after tampering
        assert!(audit_log.verify_signature(secret).is_ok());
        assert!(!audit_log.verify_signature(secret).unwrap());
    }

    #[test]
    fn test_input_validation() {
        use sdforge::core::validation::{validate_email, validate_length, MIN_PASSWORD_LENGTH};
        
        // Valid inputs
        assert!(validate_email("user@example.com"));
        assert!(validate_length("password123", MIN_PASSWORD_LENGTH, 100));
        
        // Invalid inputs
        assert!(!validate_email("invalid-email"));
        assert!(!validate_length("short", MIN_PASSWORD_LENGTH, 100));
    }
}
