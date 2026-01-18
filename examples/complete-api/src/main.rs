// Complete SDForge API Example
// Demonstrates all major features: HTTP, MCP, Security, Caching, WebSocket, etc.

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// In-Memory Data Store
// ============================================================================

type DataStore = Arc<RwLock<HashMap<u64, User>>>;

async fn create_data_store() -> DataStore {
    let store = Arc::new(RwLock::new(HashMap::new()));
    
    // Add some sample data
    let mut users = store.write().await;
    users.insert(1, User {
        id: 1,
        username: "admin".to_string(),
        email: "admin@example.com".to_string(),
        created_at: chrono::Utc::now(),
    });
    users.insert(2, User {
        id: 2,
        username: "user1".to_string(),
        email: "user1@example.com".to_string(),
        created_at: chrono::Utc::now(),
    });
    
    store
}

// ============================================================================
// Authentication Module
// ============================================================================

#[service_module(prefix = "/auth")]
mod auth_api {
    use super::*;

    #[service_api(
        name = "login",
        version = "v1",
        path = "/login",
        method = "POST",
        description = "Authenticate user and return token"
    )]
    async fn login(
        credentials: LoginRequest,
        #[state] data_store: DataStore
    ) -> Result<Token, ApiError> {
        // Simple authentication logic (in production, use proper password hashing)
        let users = data_store.read().await;
        
        for user in users.values() {
            if user.username == credentials.username && credentials.password == "password" {
                let token = Token {
                    token: Uuid::new_v4().to_string(),
                    expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
                };
                return Ok(token);
            }
        }
        
        Err(ApiError::AuthenticationFailed {
            message: "Invalid credentials".to_string(),
        })
    }

    #[service_api(
        name = "logout",
        version = "v1",
        path = "/logout",
        method = "POST",
        auth_required = true,
        description = "Logout user (invalidate token)"
    )]
    async fn logout(
        auth_context: AuthContext
    ) -> Result<(), ApiError> {
        // In a real implementation, you would invalidate the token
        println!("User {} logged out", auth_context.user_id().unwrap_or("unknown"));
        Ok(())
    }
}

// ============================================================================
// User Management Module
// ============================================================================

#[service_module(prefix = "/users")]
mod user_api {
    use super::*;

    #[service_api(
        name = "get_user",
        version = "v1",
        path = "/:id",
        method = "GET",
        description = "Get user by ID"
    )]
    async fn get_user(
        id: u64,
        #[state] data_store: DataStore
    ) -> Result<User, ApiError> {
        let users = data_store.read().await;
        
        match users.get(&id) {
            Some(user) => Ok(user.clone()),
            None => Err(ApiError::NotFound {
                resource: "user".to_string(),
                id: id.to_string(),
            }),
        }
    }

    #[service_api(
        name = "list_users",
        version = "v1",
        path = "/",
        method = "GET",
        description = "List all users"
    )]
    async fn list_users(
        #[state] data_store: DataStore
    ) -> Result<Vec<User>, ApiError> {
        let users = data_store.read().await;
        Ok(users.values().cloned().collect())
    }

    #[service_api(
        name = "create_user",
        version = "v1",
        path = "/",
        method = "POST",
        description = "Create a new user"
    )]
    async fn create_user(
        request: CreateUserRequest,
        #[state] data_store: DataStore
    ) -> Result<User, ApiError> {
        let mut users = data_store.write().await;
        
        // Generate new ID (in production, use proper ID generation)
        let new_id = users.len() as u64 + 1;
        
        let new_user = User {
            id: new_id,
            username: request.username.clone(),
            email: request.email,
            created_at: chrono::Utc::now(),
        };
        
        users.insert(new_id, new_user.clone());
        
        Ok(new_user)
    }

    #[service_api(
        name = "delete_user",
        version = "v1",
        path = "/:id",
        method = "DELETE",
        auth_required = true,
        description = "Delete user by ID"
    )]
    async fn delete_user(
        id: u64,
        auth_context: AuthContext,
        #[state] data_store: DataStore
    ) -> Result<(), ApiError> {
        // Check if user has permission to delete
        if auth_context.user_id() != Some("admin") {
            return Err(ApiError::AccessDenied {
                required_permission: "admin".to_string(),
                user_permissions: auth_context.permissions().to_vec(),
            });
        }
        
        let mut users = data_store.write().await;
        
        match users.remove(&id) {
            Some(_) => Ok(()),
            None => Err(ApiError::NotFound {
                resource: "user".to_string(),
                id: id.to_string(),
            }),
        }
    }
}

// ============================================================================
// System Module
// ============================================================================

#[service_module(prefix = "/system")]
mod system_api {
    use super::*;

    static START_TIME: std::sync::OnceLock<chrono::DateTime<chrono::Utc>> = std::sync::OnceLock::new();

    fn get_start_time() -> &'static chrono::DateTime<chrono::Utc> {
        START_TIME.get_or_init(|| chrono::Utc::now())
    }

    #[service_api(
        name = "health_check",
        version = "v1",
        path = "/health",
        method = "GET",
        description = "System health check"
    )]
    async fn health_check() -> Result<HealthStatus, ApiError> {
        let start_time = get_start_time();
        let uptime = chrono::Utc::now() - *start_time;
        
        Ok(HealthStatus {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime.num_seconds() as u64,
            timestamp: chrono::Utc::now(),
        })
    }

    #[service_api(
        name = "system_info",
        version = "v1",
        path = "/info",
        method = "GET",
        auth_required = true,
        description = "Get system information"
    )]
    async fn system_info(
        auth_context: AuthContext
    ) -> Result<serde_json::Value, ApiError> {
        Ok(serde_json::json!({
            "user": auth_context.user_id(),
            "permissions": auth_context.permissions(),
            "metadata": auth_context.metadata(),
            "features": ["http", "mcp", "security", "cache", "websocket", "grpc"],
            "rust_version": "2021",
            "framework": "SDForge"
        }))
    }
}

// ============================================================================
// Main Application
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Create shared data store
    let data_store = create_data_store().await;
    
    println!("🚀 Starting SDForge Complete API Example");
    println!("📊 Available endpoints:");
    println!("  - POST /auth/api/v1/login");
    println!("  - POST /auth/api/v1/logout");
    println!("  - GET  /users/api/v1/");
    println!("  - GET  /users/api/v1/:id");
    println!("  - POST /users/api/v1/");
    println!("  - DELETE /users/api/v1/:id");
    println!("  - GET  /system/api/v1/health");
    println!("  - GET  /system/api/v1/info");
    
    // Build the application with all features
    let app = sdforge::http::build();
    
    // Add shared state
    let app_with_state = app.with_state(data_store);
    
    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("🌐 Server running on http://0.0.0.0:3000");
    
    axum::serve(listener, app_with_state).await?;
    
    Ok(())
}
