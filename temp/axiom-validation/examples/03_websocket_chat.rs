//! 03_websocket_chat - WebSocket 聊天示例
//!
//! 这个示例演示如何使用 Axiom 框架创建 WebSocket 实时聊天服务。
//!
//! 运行方式:
//! ```bash
//! cargo run --bin 03_websocket_chat
//! ```
//!
//! 测试方式:
//! 使用 WebSocket 客户端连接 ws://localhost:8080/ws

use axiom::prelude::*;
use axiom::service_api;
use axiom::websocket::{ConnectionManager, WebSocketHandler, WebSocketMessage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::pin::Pin;
use std::future::Future;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// ============================================================================
// 数据模型
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    id: String,
    user: String,
    content: String,
    timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserJoin {
    user: String,
    timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserLeave {
    user: String,
    timestamp: i64,
}

// ============================================================================
// 聊天室状态
// ============================================================================

#[derive(Clone)]
struct ChatRoom {
    messages: Arc<RwLock<Vec<ChatMessage>>>,
    users: Arc<RwLock<Vec<String>>>,
}

impl ChatRoom {
    fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(Vec::new())),
            users: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn add_message(&self, message: ChatMessage) {
        self.messages.write().await.push(message);
    }

    async fn add_user(&self, user: String) {
        let mut users = self.users.write().await;
        if !users.contains(&user) {
            users.push(user);
        }
    }

    async fn remove_user(&self, user: String) {
        self.users.write().await.retain(|u| u != &user);
    }

    async fn get_users(&self) -> Vec<String> {
        self.users.read().await.clone()
    }
}

// ============================================================================
// WebSocket 处理器
// ============================================================================

struct ChatHandler {
    room: ChatRoom,
    manager: Arc<ConnectionManager>,
    user: String,
}

impl ChatHandler {
    fn new(room: ChatRoom, manager: Arc<ConnectionManager>, user: String) -> Self {
        Self { room, manager, user }
    }
}

impl WebSocketHandler for ChatHandler {
    fn handle(&self, message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage> {
        let room = self.room.clone();
        let manager = self.manager.clone();
        let user = self.user.clone();

        Box::pin(async move {
            match message {
                WebSocketMessage::Request { id, method, params } => {
                    match method.as_str() {
                        "send" => {
                            // 发送聊天消息
                            if let Some(content) = params.get("content") {
                                if let Some(content_str) = content.as_str() {
                                    let chat_msg = ChatMessage {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        user: user.clone(),
                                        content: content_str.to_string(),
                                        timestamp: chrono::Utc::now().timestamp(),
                                    };

                                    room.add_message(chat_msg.clone()).await;

                                    // 广播消息给所有用户
                                    let broadcast_msg = WebSocketMessage::Notification {
                                        event: "message".to_string(),
                                        data: serde_json::to_value(chat_msg).unwrap_or_default(),
                                    };
                                    manager.broadcast(broadcast_msg).await;

                                    WebSocketMessage::Response {
                                        id,
                                        result: serde_json::json!({"status": "sent"}),
                                    }
                                } else {
                                    WebSocketMessage::Error {
                                        id,
                                        error: "Invalid content format".to_string(),
                                    }
                                }
                            } else {
                                WebSocketMessage::Error {
                                    id,
                                    error: "Missing content parameter".to_string(),
                                }
                            }
                        }
                        "users" => {
                            // 获取在线用户列表
                            let users = room.get_users().await;
                            WebSocketMessage::Response {
                                id,
                                result: serde_json::json!({"users": users}),
                            }
                        }
                        "history" => {
                            // 获取聊天历史
                            let messages = room.messages.read().await.clone();
                            WebSocketMessage::Response {
                                id,
                                result: serde_json::json!({"messages": messages}),
                            }
                        }
                        _ => {
                            WebSocketMessage::Error {
                                id,
                                error: format!("Unknown method: {}", method),
                            }
                        }
                    }
                }
                _ => WebSocketMessage::Error {
                    id: String::new(),
                    error: "Unsupported message type".to_string(),
                },
            }
        })
    }
}

// ============================================================================
// 主函数
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    println!("========================================");
    println!("Axiom WebSocket 聊天示例");
    println!("========================================");
    println!();

    let room = ChatRoom::new();
    let manager = Arc::new(ConnectionManager::new());

    println!("✅ WebSocket 聊天室已创建");
    println!();
    println!("📡 WebSocket 服务地址: ws://localhost:8080/ws");
    println!();
    println!("📝 可用的 WebSocket 方法:");
    println!("  send     - 发送聊天消息");
    println!("  users    - 获取在线用户列表");
    println!("  history  - 获取聊天历史");
    println!();
    println!("💡 消息格式:");
    println!("  {");
    println!("    \"id\": \"request-id\",");
    println!("    \"method\": \"send\",");
    println!("    \"params\": {");
    println!("      \"content\": \"Hello, World!\"");
    println!("    }");
    println!("  }");
    println!();
    println!("按 Ctrl+C 停止服务");
    println!("========================================");
    println!();

    // 模拟运行
    println!("WebSocket 服务已启动（模拟模式）");
    println!("提示: 实际使用时需要完整的 HTTP 服务器支持");
    println!();

    tokio::signal::ctrl_c().await?;
    println!("\n👋 WebSocket 服务已停止");

    Ok(())
}