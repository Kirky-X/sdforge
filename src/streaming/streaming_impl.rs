// Copyright (c) 2026 Kirky.X🌠
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

/// Idle heartbeat interval: while the source stream is quiet, a comment frame
/// is sent so proxies/load balancers do not cut a long-lived SSE connection on
/// their own idle timeout. The timer resets on every data frame (heartbeats
/// are idle-only).
const SSE_HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(10);

/// Heartbeat frame. A comment line (`:` prefix) per the SSE spec — EventSource
/// clients ignore it, and sdforge's own [`crate::streaming::parser`] skips
/// comment lines, so the keep-alive never surfaces as application data.
const SSE_HEARTBEAT_FRAME: &str = ": keep-alive\n\n";

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
        // 记录是否因空闲超时退出：客户端据此区分「正常结束」与「30s 空闲被强制关闭」。
        let mut timed_out = false;
        // 两个独立截止时刻：数据帧到达时都重置；心跳只重置自己——
        // 心跳若重置空闲计时，源永久静默时 30s 空闲终止将永不触发。
        let idle_window = std::time::Duration::from_secs(30);
        let now = tokio::time::Instant::now();
        let mut idle_deadline = now + idle_window;
        let mut heartbeat_deadline = now + SSE_HEARTBEAT_INTERVAL;

        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(idle_deadline) => {
                    timed_out = true;
                    break;
                },
                _ = tokio::time::sleep_until(heartbeat_deadline) => {
                    if tx.send(Ok(SSE_HEARTBEAT_FRAME.to_string())).await.is_err() {
                        break;
                    }
                    heartbeat_deadline = tokio::time::Instant::now() + SSE_HEARTBEAT_INTERVAL;
                },
                next = stream.next() => {
                    let Some(item) = next else { break };

                    let event = mapper(item);

                    // Security fix: Handle serialization errors properly instead of silently failing
                    // Log the error and send an error event to the client
                    let data = match serde_json::to_string(&event) {
                        Ok(data) => data,
                        Err(e) => {
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

                    let now = tokio::time::Instant::now();
                    idle_deadline = now + idle_window;
                    heartbeat_deadline = now + SSE_HEARTBEAT_INTERVAL;
                }
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

        let sse_stream = stream_to_sse(self.stream, |item| match item {
            Ok(data) => {
                StreamEvent::data(serde_json::to_value(data).unwrap_or(serde_json::Value::Null))
            }
            Err(err) => StreamEvent::error(err),
        });

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
