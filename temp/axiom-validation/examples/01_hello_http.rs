//! 01_hello_http - HTTP 协议基础示例
//!
//! 这个示例演示如何使用 Axiom 框架创建一个简单的 HTTP API 服务。
//!
//! 运行方式:
//! ```bash
//! cargo run --bin 01_hello_http
//! ```
//!
//! 测试方式:
//! ```bash
//! # 获取用户列表
//! curl http://localhost:8080/api/v1/users
//!
//! # 获取单个用户
//! curl http://localhost:8080/api/v1/users/1
//!
//! # 创建用户
//! curl -X POST http://localhost:8080/api/v1/users \
//!   -H "Content-Type: application/json" \
//!   -d '{"name":"Alice","email":"alice@example.com"}'
//!
//! # 更新用户
//! curl -X PUT http://localhost:8080/api/v1/users/1 \
//!   -H "Content-Type: application/json" \
//!   -d '{"name":"Alice Updated","email":"alice.updated@example.com"}'
//!
//! # 删除用户
//! curl -X DELETE http://localhost:8080/api/v1/users/1
//! ```

use axiom::prelude::*;
use axiom::service_api;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// 数据模型
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct UpdateUserRequest {
    name: Option<String>,
    email: Option<String>,
}

// ============================================================================
// 数据库模拟
// ============================================================================

type UserDatabase = Arc<Mutex<HashMap<u64, User>>>;

// ============================================================================
// API 接口定义
// ============================================================================

/// 获取所有用户列表
#[service_api(
    name = "list_users",
    version = "v1",
    description = "Get all users",
    path = "/users",
    method = "GET"
)]
async fn list_users(db: UserDatabase) -> Result<Vec<User>, ApiError> {
    let users = db.lock().unwrap();
    Ok(users.values().cloned().collect())
}

/// 根据 ID 获取用户
#[service_api(
    name = "get_user",
    version = "v1",
    description = "Get user by ID",
    path = "/users/:id",
    method = "GET"
)]
async fn get_user(id: u64, db: UserDatabase) -> Result<User, ApiError> {
    let users = db.lock().unwrap();
    users.get(&id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some(id.to_string()),
        })
}

/// 创建新用户
#[service_api(
    name = "create_user",
    version = "v1",
    description = "Create a new user",
    path = "/users",
    method = "POST"
)]
async fn create_user(req: CreateUserRequest, db: UserDatabase) -> Result<User, ApiError> {
    let mut users = db.lock().unwrap();
    let new_id = users.len() as u64 + 1;

    // 验证输入
    if req.name.is_empty() {
        return Err(ApiError::InvalidInput {
            message: "Name cannot be empty".to_string(),
            field: Some("name".to_string()),
            value: Some(serde_json::json!(req.name)),
        });
    }

    if !req.email.contains('@') {
        return Err(ApiError::InvalidInput {
            message: "Invalid email address".to_string(),
            field: Some("email".to_string()),
            value: Some(serde_json::json!(req.email)),
        });
    }

    let user = User {
        id: new_id,
        name: req.name,
        email: req.email,
    };

    users.insert(new_id, user.clone());
    Ok(user)
}

/// 更新用户信息
#[service_api(
    name = "update_user",
    version = "v1",
    description = "Update user information",
    path = "/users/:id",
    method = "PUT"
)]
async fn update_user(
    id: u64,
    req: UpdateUserRequest,
    db: UserDatabase
) -> Result<User, ApiError> {
    let mut users = db.lock().unwrap();

    if let Some(user) = users.get_mut(&id) {
        if let Some(name) = req.name {
            if name.is_empty() {
                return Err(ApiError::InvalidInput {
                    message: "Name cannot be empty".to_string(),
                    field: Some("name".to_string()),
                    value: Some(serde_json::json!(name)),
                });
            }
            user.name = name;
        }

        if let Some(email) = req.email {
            if !email.contains('@') {
                return Err(ApiError::InvalidInput {
                    message: "Invalid email address".to_string(),
                    field: Some("email".to_string()),
                    value: Some(serde_json::json!(email)),
                });
            }
            user.email = email;
        }

        Ok(user.clone())
    } else {
        Err(ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some(id.to_string()),
        })
    }
}

/// 删除用户
#[service_api(
    name = "delete_user",
    version = "v1",
    description = "Delete a user",
    path = "/users/:id",
    method = "DELETE"
)]
async fn delete_user(id: u64, db: UserDatabase) -> Result<User, ApiError> {
    let mut users = db.lock().unwrap();

    users.remove(&id)
        .ok_or_else(|| ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some(id.to_string()),
        })
}

/// 健康检查
#[service_api(
    name = "health_check",
    version = "v1",
    description = "Health check endpoint",
    path = "/health",
    method = "GET"
)]
async fn health_check() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "status": "ok",
        "service": "axiom-http-example",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// ============================================================================
// 主函数
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    println!("========================================");
    println!("Axiom HTTP 示例服务");
    println!("========================================");
    println!();

    // 创建数据库（内存存储）
    let db: UserDatabase = Arc::new(Mutex::new(HashMap::new()));

    // 初始化一些测试数据
    {
        let mut users = db.lock().unwrap();
        users.insert(1, User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        });
        users.insert(2, User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
        });
    }

    // 构建 HTTP 路由器
    let router = axiom::http::build();

    println!("✅ HTTP 路由器构建成功");
    println!();
    println!("📡 服务地址: http://0.0.0.0:8080");
    println!();
    println!("📚 可用的 API 端点:");
    println!("  GET    /api/v1/health        - 健康检查");
    println!("  GET    /api/v1/users         - 获取用户列表");
    println!("  GET    /api/v1/users/:id     - 获取单个用户");
    println!("  POST   /api/v1/users         - 创建新用户");
    println!("  PUT    /api/v1/users/:id     - 更新用户");
    println!("  DELETE /api/v1/users/:id     - 删除用户");
    println!();
    println!("💡 测试命令:");
    println!("  curl http://localhost:8080/api/v1/users");
    println!("  curl http://localhost:8080/api/v1/users/1");
    println!("  curl -X POST http://localhost:8080/api/v1/users \\");
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"name\":\"Test\",\"email\":\"test@example.com\"}}'");
    println!();
    println!("按 Ctrl+C 停止服务");
    println!("========================================");
    println!();

    // 启动 HTTP 服务器
    let addr = "0.0.0.0:8080".parse::<std::net::SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    axum::serve(listener, router).await?;

    Ok(())
}