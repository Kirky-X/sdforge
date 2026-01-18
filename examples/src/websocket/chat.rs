// Copyright (c) 2026 Kirky.X
//! Real-time chat example

use sdforge::prelude::*;

/// Chat WebSocket
///
/// This would be the main WebSocket endpoint for chat functionality.
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

/// Chat message endpoint
///
/// HTTP endpoint for sending chat messages.
#[service_api(
    name = "send_message",
    version = "v1",
    path = "/chat/message",
    method = "POST",
    tool_name = "send_message",
    description = "Send a chat message"
)]
async fn send_message(room: String, message: String) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "room": room,
        "message": message,
        "sent": true
    }))
}
