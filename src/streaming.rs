// Copyright (c) 2026 Kirky.X
// Streaming response support
//!
//! This module provides utilities for streaming responses in both HTTP and MCP protocols.
//! Requires the `streaming` feature.

use futures_util::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

#[cfg(feature = "http")]
use axum::{body::Body, http::Response, response::IntoResponse};

/// Stream response wrapper
#[derive(Debug)]
pub struct StreamResponse<T> {
    /// The underlying stream
    stream: ReceiverStream<Result<T, String>>,
    /// Whether this is the last item in the stream
    is_final: bool,
}

impl<T: Send + 'static> StreamResponse<T> {
    /// Create a new stream response
    pub fn new(stream: ReceiverStream<Result<T, String>>) -> Self {
        Self {
            stream,
            is_final: false,
        }
    }

    /// Create a single-item stream response
    pub fn single(item: T) -> Self
    where
        T: Clone,
    {
        let (tx, rx) = mpsc::channel(1);
        let item = item.clone();
        tokio::spawn(async move {
            let _ = tx.send(Ok(item)).await;
        });
        Self::new(ReceiverStream::new(rx))
    }

    /// Create a final stream response marker
    pub fn final_marker() -> Self {
        let (_tx, rx) = mpsc::channel(1);
        Self {
            stream: ReceiverStream::new(rx),
            is_final: true,
        }
    }

    #[allow(missing_docs)]
    pub fn is_final(&self) -> bool {
        self.is_final
    }

    #[allow(missing_docs)]
    pub fn into_stream(self) -> ReceiverStream<Result<T, String>> {
        self.stream
    }
}

/// Create a streaming response channel
pub fn create_stream_channel<T: Send + 'static>(
    buffer_size: usize,
) -> (mpsc::Sender<Result<T, String>>, StreamResponse<T>) {
    let (tx, rx) = mpsc::channel(buffer_size);
    (tx, StreamResponse::new(ReceiverStream::new(rx)))
}

/// Stream item for SSE (Server-Sent Events)
#[derive(Debug, Clone, serde::Serialize)]
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

impl<T> StreamEvent<T> {
    /// Create a data event
    pub fn data(data: T) -> Self {
        Self::Data {
            id: None,
            event_name: None,
            data,
        }
    }

    /// Create a ping event
    pub fn ping() -> Self {
        Self::Ping {
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Create an error event
    pub fn error(message: String) -> Self {
        Self::Error { message }
    }

    /// Create a completion event
    pub fn complete() -> Self {
        Self::Complete
    }
}

/// Convert a stream to SSE format
pub fn stream_to_sse<S, T, F>(
    stream: S,
    mapper: F,
) -> impl Stream<Item = Result<String, std::convert::Infallible>> + Send + 'static
where
    S: Stream<Item = T> + Send + 'static,
    F: Fn(T) -> StreamEvent + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(async move {
        let mut stream = Box::pin(stream);

        while let Some(item) = tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => None,
            next = stream.next() => next,
        } {
            let event = mapper(item);

            // Security fix: Handle serialization errors properly instead of silently failing
            // Log the error and send an error event to the client
            let data = match serde_json::to_string(&event) {
                Ok(data) => data,
                Err(e) => {
                    #[cfg(feature = "logging")]
                    tracing::error!(error = %e, "Failed to serialize SSE event");
                    // Send error event instead of silently failing
                    serde_json::to_string(&StreamEvent::<()>::error(format!(
                        "Serialization error: {}",
                        e
                    )))
                    .unwrap_or_else(|_| r#"{"error":"Serialization failed"}"#.to_string())
                }
            };
            let sse = format!("data: {}\n\n", data);

            if tx.send(Ok(sse)).await.is_err() {
                break;
            }
        }

        // Send completion event
        let _ = tx
            .send(Ok("event: complete\ndata: {}\n\n".to_string()))
            .await;
    });

    ReceiverStream::new(rx)
}

/// Implement IntoResponse for StreamResponse to enable SSE streaming in HTTP handlers
#[cfg(feature = "http")]
impl<T> IntoResponse for StreamResponse<T>
where
    T: serde::Serialize + Send + 'static,
{
    fn into_response(self) -> Response<Body> {
        use axum::body::Body;
        use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};

        // Convert stream to SSE format
        let sse_stream = stream_to_sse(self.stream, |item| match item {
            Ok(data) => {
                StreamEvent::data(serde_json::to_value(data).unwrap_or(serde_json::Value::Null))
            }
            Err(err) => StreamEvent::error(err),
        });

        // Build SSE response with proper headers
        Response::builder()
            .status(200)
            .header(CONTENT_TYPE, "text/event-stream")
            .header(CACHE_CONTROL, "no-cache")
            .header("Connection", "keep-alive")
            .header("X-Accel-Buffering", "no") // Disable Nginx buffering
            .body(Body::from_stream(sse_stream))
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::ReceiverStream;

    #[tokio::test]
    async fn test_stream_response() {
        let (tx, rx) = mpsc::channel(32);
        let stream = StreamResponse::new(ReceiverStream::new(rx));

        tokio::spawn(async move {
            let _ = tx.send(Ok("test")).await;
        });

        assert!(!stream.is_final());
    }

    #[tokio::test]
    async fn test_stream_event_data() {
        let event = StreamEvent::data(serde_json::json!({"key": "value"}));
        match event {
            StreamEvent::Data {
                id,
                event_name: _,
                data,
            } => {
                assert!(id.is_none());
                assert_eq!(data, serde_json::json!({"key": "value"}));
            }
            _ => unreachable!("Unexpected variant in StreamEvent::Data test"),
        }
    }

    #[tokio::test]
    async fn test_stream_event_complete() {
        let event: StreamEvent<()> = StreamEvent::complete();

        assert!(
            matches!(event, StreamEvent::Complete),
            "Expected Complete event, got {:?}",
            event
        );
    }

    // ============================================================================
    // StreamEvent Ping Tests
    // ============================================================================

    #[test]
    fn test_stream_event_ping() {
        let event: StreamEvent<()> = StreamEvent::ping();
        match event {
            StreamEvent::Ping { timestamp } => {
                assert!(timestamp > 0);
                assert!(timestamp <= chrono::Utc::now().timestamp());
            }
            _ => unreachable!("Expected Ping event"),
        }
    }

    // ============================================================================
    // StreamEvent Error Tests
    // ============================================================================

    #[test]
    fn test_stream_event_error() {
        let error_msg = "Test error message";
        let event: StreamEvent<()> = StreamEvent::error(error_msg.to_string());

        match event {
            StreamEvent::Error { message } => {
                assert_eq!(message, error_msg);
            }
            _ => unreachable!("Expected Error event"),
        }
    }

    // ============================================================================
    // StreamEvent Data with Metadata Tests
    // ============================================================================

    #[test]
    fn test_stream_event_data_with_id() {
        let event = StreamEvent::data("test data");
        let modified_event = match event {
            StreamEvent::Data {
                id: _,
                event_name: _,
                data,
            } => StreamEvent::Data {
                id: Some("test-id".to_string()),
                event_name: Some("update".to_string()),
                data,
            },
            _ => unreachable!("Expected Data event"),
        };

        match modified_event {
            StreamEvent::Data {
                id,
                event_name,
                data: _,
            } => {
                assert_eq!(id, Some("test-id".to_string()));
                assert_eq!(event_name, Some("update".to_string()));
            }
            _ => unreachable!("Expected Data event with metadata"),
        }
    }

    #[test]
    fn test_stream_event_data_serialization() {
        let event = StreamEvent::data(serde_json::json!({"message": "test"}));
        let json = serde_json::to_string(&event);
        assert!(json.is_ok());

        if let Ok(json_str) = json {
            assert!(json_str.contains("\"type\":\"data\""));
            assert!(json_str.contains("\"message\""));
        }
    }

    // ============================================================================
    // StreamResponse Single Item Tests
    // ============================================================================

    #[tokio::test]
    async fn test_stream_response_single() {
        let response = StreamResponse::single("test_item");
        assert!(!response.is_final());

        // Try to collect from the stream
        use futures_util::StreamExt;
        let mut stream = response.into_stream();

        if let Some(result) = stream.next().await {
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "test_item");
        } else {
            panic!("Expected one item in stream");
        }
    }

    // ============================================================================
    // StreamResponse Final Marker Tests
    // ============================================================================

    #[test]
    fn test_stream_response_final_marker() {
        let response: StreamResponse<()> = StreamResponse::final_marker();
        assert!(response.is_final());
    }

    // ============================================================================
    // create_stream_channel Tests
    // ============================================================================

    #[tokio::test]
    async fn test_create_stream_channel() {
        let (tx, response) = create_stream_channel::<String>(10);

        // Send some data
        assert!(tx.send(Ok("item1".to_string())).await.is_ok());
        assert!(tx.send(Ok("item2".to_string())).await.is_ok());

        // Receive from stream
        use futures_util::StreamExt;
        let mut stream = response.into_stream();
        let item1 = stream.next().await;
        assert!(item1.is_some());
        assert_eq!(item1.unwrap().unwrap(), "item1");

        let item2 = stream.next().await;
        assert!(item2.is_some());
        assert_eq!(item2.unwrap().unwrap(), "item2");
    }

    // ============================================================================
    // Stream Event Serialization Tests
    // ============================================================================

    #[test]
    fn test_stream_event_serialization_data() {
        let event = StreamEvent::data("test_value");
        let json = serde_json::to_string(&event);
        assert!(json.is_ok());

        if let Ok(json_str) = json {
            assert!(json_str.contains("\"type\":\"data\""));
            assert!(json_str.contains("test_value"));
        }
    }

    #[test]
    fn test_stream_event_serialization_ping() {
        let event: StreamEvent<()> = StreamEvent::ping();
        let json = serde_json::to_string(&event);
        assert!(json.is_ok());

        if let Ok(json_str) = json {
            assert!(json_str.contains("\"type\":\"ping\""));
            assert!(json_str.contains("\"timestamp\""));
        }
    }

    #[test]
    fn test_stream_event_serialization_error() {
        let event: StreamEvent<()> = StreamEvent::error("error msg".to_string());
        let json = serde_json::to_string(&event);
        assert!(json.is_ok());

        if let Ok(json_str) = json {
            assert!(json_str.contains("\"type\":\"error\""));
            assert!(json_str.contains("error msg"));
        }
    }

    #[test]
    fn test_stream_event_serialization_complete() {
        let event: StreamEvent<()> = StreamEvent::complete();
        let json = serde_json::to_string(&event);
        assert!(json.is_ok());

        if let Ok(json_str) = json {
            assert!(json_str.contains("\"type\":\"complete\""));
        }
    }

    // ============================================================================
    // Stream Event with Complex Data Tests
    // ============================================================================

    #[tokio::test]
    async fn test_stream_response_complex_data() {
        let complex_data = serde_json::json!({
            "user": "Alice",
            "age": 30,
            "active": true
        });

        let response = StreamResponse::single(complex_data.clone());

        use futures_util::StreamExt;
        let mut stream = response.into_stream();
        let result = stream.next().await;
        assert!(result.is_some());

        let value = result.unwrap().unwrap();
        assert_eq!(value, complex_data);
    }

    #[test]
    fn test_stream_event_ping_creation() {
        let event1: StreamEvent<()> = StreamEvent::ping();
        let event2: StreamEvent<()> = StreamEvent::ping();

        match (&event1, &event2) {
            (StreamEvent::Ping { timestamp: t1 }, StreamEvent::Ping { timestamp: t2 }) => {
                assert!(*t1 > 0);
                assert!(*t2 > 0);
                assert!((*t2 - *t1).abs() < 1000);
            }
            _ => unreachable!("Both should be Ping events"),
        }
    }

    // ============================================================================
    // Additional Stream Event Tests
    // ============================================================================

    #[test]
    fn test_stream_event_error_creation() {
        let event: StreamEvent<()> = StreamEvent::error("Failed to process".to_string());
        assert!(matches!(event, StreamEvent::Error { .. }));

        let error_str = match event {
            StreamEvent::Error { message } => message.clone(),
            _ => String::new(),
        };
        assert_eq!(error_str, "Failed to process");
    }

    // ============================================================================
    // Additional StreamResponse Tests
    // ============================================================================

    #[tokio::test]
    async fn test_stream_response_final_empty_stream() {
        let response: StreamResponse<()> = StreamResponse::final_marker();

        use futures_util::StreamExt;
        let mut stream = response.into_stream();
        let result = stream.next().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_create_stream_channel_buffer_size_1() {
        let (tx, response) = create_stream_channel::<i32>(1);
        drop(tx);

        use futures_util::StreamExt;
        let mut stream = response.into_stream();
        let result = stream.next().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_create_stream_channel_sender_drop() {
        let (tx, response) = create_stream_channel::<String>(10);
        drop(tx);

        use futures_util::StreamExt;
        let mut stream = response.into_stream();
        let result = stream.next().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_stream_backpressure() {
        let (tx, response) = create_stream_channel::<String>(2);

        assert!(tx.try_send(Ok("item1".to_string())).is_ok());
        assert!(tx.try_send(Ok("item2".to_string())).is_ok());

        let send_result = tx.try_send(Ok("item3".to_string()));
        assert!(send_result.is_err());
        drop(tx);

        use futures_util::StreamExt;
        let mut stream = response.into_stream();
        let item1 = stream.next().await;
        let item2 = stream.next().await;

        assert_eq!(item1.unwrap().unwrap(), "item1");
        assert_eq!(item2.unwrap().unwrap(), "item2");
    }

    #[tokio::test]
    async fn test_stream_response_multiple_items() {
        let (tx, response) = create_stream_channel::<i32>(10);

        for i in 1..=5 {
            assert!(tx.send(Ok(i * 10)).await.is_ok());
        }
        drop(tx);

        use futures_util::StreamExt;
        let results: Vec<i32> = response.into_stream().map(|r| r.unwrap()).collect().await;

        assert_eq!(results, vec![10, 20, 30, 40, 50]);
    }

    #[tokio::test]
    async fn test_stream_with_errors() {
        let (tx, response) = create_stream_channel::<String>(10);

        assert!(tx.send(Ok("success".to_string())).await.is_ok());
        assert!(tx.send(Err("error1".to_string())).await.is_ok());
        assert!(tx.send(Ok("success2".to_string())).await.is_ok());
        drop(tx);

        use futures_util::StreamExt;
        let results: Vec<String> = response
            .into_stream()
            .map(|r| r.unwrap_or_else(|e| e))
            .collect()
            .await;

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], "success");
        assert_eq!(results[1], "error1");
        assert_eq!(results[2], "success2");
    }

    #[test]
    fn test_stream_event_empty_string_data() {
        let event = StreamEvent::data("");
        match event {
            StreamEvent::Data { data, .. } => {
                assert_eq!(data, "");
            }
            _ => unreachable!("Expected Data event"),
        }
    }

    #[tokio::test]
    async fn test_stream_response_empty_channel() {
        let (tx, response) = create_stream_channel::<String>(10);
        drop(tx);

        use futures_util::StreamExt;
        let count = response.into_stream().count().await;
        assert_eq!(count, 0);
    }

    #[test]
    fn test_stream_event_large_data() {
        let large_data = "x".repeat(100000);
        let event = StreamEvent::data(large_data.clone());
        match event {
            StreamEvent::Data { data, .. } => {
                assert_eq!(data.len(), 100000);
                assert!(data.chars().all(|c| c == 'x'));
            }
            _ => unreachable!("Expected Data event"),
        }
    }
}
