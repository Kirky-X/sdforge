// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;
use futures_util::{Stream, StreamExt};
use tokio::sync::mpsc;

#[cfg(feature = "http")]
use axum::{body::Body, http::Response, response::IntoResponse};

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
        // Track whether the loop exited due to idle timeout. Previously the
        // 30s sleep timeout silently terminated the stream and the code below
        // unconditionally emitted a `complete` event — making it impossible
        // for clients to distinguish "source stream ended normally" from
        // "source stream went idle for 30s and was force-closed".
        let mut timed_out = false;

        while let Some(item) = tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                timed_out = true;
                None
            },
            next = stream.next() => next,
        } {
            let event = mapper(item);

            // Security fix: Handle serialization errors properly instead of silently failing
            // Log the error and send an error event to the client
            let data = match serde_json::to_string(&event) {
                Ok(data) => data,
                Err(e) => {
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

        // If the stream was terminated by the idle timeout, emit an Error
        // event so clients can distinguish timeout from normal completion.
        if timed_out {
            let timeout_event =
                StreamEvent::<()>::error("Stream closed due to 30s idle timeout".to_string());
            if let Ok(payload) = serde_json::to_string(&timeout_event) {
                let _ = tx.send(Ok(format!("data: {}\n\n", payload))).await;
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
