use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub room: String,
    pub message: String,
}

#[service_api(
    name = "chat_ws",
    version = "v1",
    path = "/ws/chat",
    method = "GET",
    tool_name = "chat_ws",
    description = "Chat WebSocket endpoint"
)]
async fn chat_ws() -> Result<String, ApiError> {
    Ok("WebSocket chat connection".to_string())
}

#[service_api(
    name = "send_message",
    version = "v1",
    path = "/chat/message",
    method = "POST",
    tool_name = "send_message",
    description = "Send a chat message"
)]
async fn send_message(message: ChatMessage) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "room": message.room,
        "message": message.message,
        "sent": true
    }))
}
