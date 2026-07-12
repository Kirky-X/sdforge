// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// Streaming response support
//!
//! This module provides utilities for streaming responses in both HTTP and MCP protocols.
//! Requires the `streaming` feature.

use tokio_stream::wrappers::ReceiverStream;

mod streaming_impl;
pub use streaming_impl::{create_stream_channel, stream_to_sse};

/// Stream response wrapper
#[derive(Debug)]
pub struct StreamResponse<T> {
    /// The underlying stream
    stream: ReceiverStream<Result<T, String>>,
    /// Whether this is the last item in the stream
    is_final: bool,
}

/// Stream item for SSE (Server-Sent Events)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent<T = serde_json::Value> {
    /// Data event
    #[serde(rename = "data")]
    Data {
        /// Event ID
        id: Option<String>,
        /// Event type name
        event_name: Option<String>,
        /// Payload
        data: T,
    },
    /// Keep-alive ping
    #[serde(rename = "ping")]
    Ping {
        /// Timestamp
        timestamp: i64,
    },
    /// Error event
    #[serde(rename = "error")]
    Error {
        /// Error message
        message: String,
    },
    /// Stream completion event
    #[serde(rename = "complete")]
    Complete,
}

#[cfg(test)]
mod tests;
