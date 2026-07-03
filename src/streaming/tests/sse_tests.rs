// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `stream_to_sse` SSE conversion and `StreamResponse`'s `IntoResponse`
//! HTTP SSE response impl.

use crate::streaming::{stream_to_sse, StreamEvent, StreamResponse};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

#[tokio::test]
async fn test_stream_to_sse_basic() {
    use futures_util::stream;

    let input_stream = stream::iter(vec![1i32, 2, 3]);
    let sse_stream = stream_to_sse(input_stream, |n| StreamEvent::data(serde_json::json!(n)));

    let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

    assert!(results.len() >= 3);
    assert!(results[0].starts_with("data: "));
    assert!(results[0].contains(r#""type":"data""#));
    assert!(results[results.len() - 1].contains("complete"));
}

#[tokio::test]
async fn test_stream_to_sse_with_ping() {
    use futures_util::stream;

    let input_stream = stream::iter(vec![()]);
    let sse_stream = stream_to_sse(input_stream, |_| StreamEvent::ping());

    let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

    assert!(!results.is_empty());
    assert!(results[0].contains(r#""type":"ping""#));
    assert!(results[0].contains("timestamp"));
}

#[tokio::test]
async fn test_stream_to_sse_with_error() {
    use futures_util::stream;

    let input_stream = stream::iter(vec![()]);
    let sse_stream = stream_to_sse(input_stream, |_| {
        StreamEvent::error("test error".to_string())
    });

    let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

    assert!(!results.is_empty());
    assert!(results[0].contains(r#""type":"error""#));
    assert!(results[0].contains("test error"));
}

#[tokio::test]
async fn test_stream_to_sse_with_complete() {
    use futures_util::stream;

    let input_stream = stream::iter(vec![()]);
    let sse_stream = stream_to_sse(input_stream, |_| StreamEvent::complete());

    let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

    assert!(!results.is_empty());
    let has_complete = results.iter().any(|r| r.contains("complete"));
    assert!(has_complete);
}

#[tokio::test]
async fn test_stream_to_sse_empty_stream() {
    use futures_util::stream;

    let input_stream = stream::iter(Vec::<i32>::new());
    let sse_stream = stream_to_sse(input_stream, |n| StreamEvent::data(serde_json::json!(n)));

    let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(results.len(), 1);
    assert!(results[0].contains("complete"));
}

#[tokio::test]
async fn test_stream_to_sse_format() {
    use futures_util::stream;

    let input_stream = stream::iter(vec!["hello"]);
    let sse_stream = stream_to_sse(input_stream, |s| StreamEvent::data(serde_json::json!(s)));

    let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

    assert!(results[0].starts_with("data: "));
    assert!(results[0].ends_with("\n\n"));
}

#[tokio::test]
async fn test_stream_to_sse_multiple_items() {
    use futures_util::stream;

    let input_stream = stream::iter(vec![1, 2, 3, 4, 5]);
    let sse_stream = stream_to_sse(input_stream, |n| StreamEvent::data(serde_json::json!(n)));

    let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

    let data_events: Vec<_> = results
        .iter()
        .filter(|r| r.contains(r#""type":"data""#))
        .collect();
    assert_eq!(data_events.len(), 5);
}

#[tokio::test]
async fn test_stream_to_sse_json_value_mapper() {
    use futures_util::stream;

    let input_stream = stream::iter(vec![serde_json::json!({"key": "value"})]);
    let sse_stream = stream_to_sse(input_stream, StreamEvent::data);

    let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

    assert!(results[0].contains(r#""key":"value""#));
}

// ============================================================================
// stream_to_sse Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_stream_to_sse_single_item() {
    use futures_util::stream;

    let input_stream = stream::iter(vec![42i64]);
    let sse_stream = stream_to_sse(input_stream, |n| {
        StreamEvent::data(serde_json::json!({"value": n}))
    });

    let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

    // Should have at least the data event and completion event
    assert!(!results.is_empty());
    let has_data = results.iter().any(|r| r.contains(r#""type":"data""#));
    assert!(has_data);
}

#[tokio::test]
async fn test_stream_to_sse_error_in_mapper() {
    use futures_util::stream;

    let input_stream = stream::iter(vec![()]);
    // Mapper returns StreamEvent (default serde_json::Value type)
    let sse_stream = stream_to_sse(input_stream, |_| {
        StreamEvent::error("mapper error".to_string())
    });

    let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

    assert!(!results.is_empty());
    assert!(results[0].contains(r#""type":"error""#));
}

// ============================================================================
// stream_to_sse normal data passthrough tests
//
// stream_to_sse serializes each StreamEvent<serde_json::Value> produced by
// the mapper and emits it as an SSE "data:" line, followed by a completion
// event when the source stream ends.
// ============================================================================

#[tokio::test]
async fn test_stream_to_sse_normal_data_passthrough() {
    use futures_util::StreamExt;

    // Verify that serializable items pass through the Ok branch.
    let stream = futures_util::stream::iter(vec![1i32, 2i32, 3i32]);
    let sse_stream = stream_to_sse(stream, |item| {
        StreamEvent::data(serde_json::Value::from(item))
    });

    let mut sse_stream = Box::pin(sse_stream);
    let mut count = 0;
    while let Some(item) = sse_stream.next().await {
        let data = item.unwrap();
        if data.contains("complete") {
            break;
        }
        assert!(data.starts_with("data: "), "SSE should start with 'data: '");
        count += 1;
    }
    assert_eq!(count, 3, "Should have received 3 data events");
}

// ============================================================================
// stream_to_sse send error (receiver dropped) tests
//
// When the ReceiverStream is dropped before the spawned task finishes,
// tx.send returns an error and the task breaks out of the loop (line 168).
// ============================================================================

#[tokio::test]
async fn test_stream_to_sse_send_error_when_receiver_dropped() {
    // Create a stream that yields many items slowly. By dropping the
    // ReceiverStream immediately, the spawned task's tx.send will fail
    // and the loop will break.
    let (tx_stream, rx_stream) = mpsc::channel::<i32>(100);
    let stream = ReceiverStream::new(rx_stream);

    let sse_stream = stream_to_sse(stream, |item| {
        StreamEvent::data(serde_json::Value::from(item))
    });

    // Drop the receiver side immediately to cause send errors.
    drop(sse_stream);

    // Feed items into the source stream; the spawned task will try to
    // forward them via tx.send, which fails because the receiver was
    // dropped. The task should break out of its loop gracefully.
    for i in 0..10 {
        // try_send may also fail once the receiver is gone; ignore errors
        let _ = tx_stream.send(i).await;
    }
    drop(tx_stream);

    // Give the spawned task time to observe the send error and break.
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    // If we reach here without hanging, the send-error break path
    // executed successfully.
}

// ============================================================================
// IntoResponse impl tests
//
// StreamResponse<T> implements axum's IntoResponse trait to produce an
// SSE HTTP response. These tests cover the into_response() method.
// ============================================================================

#[cfg(feature = "http")]
#[tokio::test]
async fn test_stream_response_into_response_headers() {
    use axum::response::IntoResponse;

    let response = StreamResponse::single("test_item");
    let http_response = response.into_response();

    // Verify SSE headers are set correctly
    assert_eq!(http_response.status(), 200);
    let headers = http_response.headers();
    assert_eq!(headers.get("content-type").unwrap(), "text/event-stream");
    assert_eq!(headers.get("cache-control").unwrap(), "no-cache");
    assert_eq!(headers.get("connection").unwrap(), "keep-alive");
    assert_eq!(headers.get("x-accel-buffering").unwrap(), "no");
}

#[cfg(feature = "http")]
#[tokio::test]
async fn test_stream_response_into_response_with_error_item() {
    use axum::response::IntoResponse;

    // Build a stream that yields an error item to cover the
    // StreamEvent::error branch in the IntoResponse mapper.
    let (tx, rx) = mpsc::channel::<Result<String, String>>(10);
    let stream = StreamResponse::new(ReceiverStream::new(rx));

    tokio::spawn(async move {
        let _ = tx.send(Err("test error".to_string())).await;
    });

    let http_response = stream.into_response();
    assert_eq!(http_response.status(), 200);
    // The response body is a stream; we just verify the response builds.
}

#[cfg(feature = "http")]
#[tokio::test]
async fn test_stream_response_into_response_collects_body() {
    use axum::response::IntoResponse;

    // Build a single-item stream and convert to response, then collect
    // the body to verify the SSE data flows through.
    let response = StreamResponse::single(serde_json::json!({"hello": "world"}));
    let http_response = response.into_response();

    assert_eq!(http_response.status(), 200);

    // Collect the body stream to verify data flows through.
    let body = http_response.into_body();
    let bytes = axum::body::to_bytes(body, 1024 * 1024).await.unwrap();
    let body_str = String::from_utf8_lossy(&bytes);
    assert!(
        body_str.contains("data: "),
        "Body should contain SSE data prefix, got: {}",
        body_str
    );
    assert!(
        body_str.contains("hello"),
        "Body should contain the JSON data, got: {}",
        body_str
    );
}

// ============================================================================
// stream_to_sse completion event tests
//
// The `Err` branch of `serde_json::to_string(&event)` in stream_to_sse
// (lines 156–163) is defensive code that handles the theoretical case where
// serialization fails. In practice, `stream_to_sse`'s mapper returns
// `StreamEvent<serde_json::Value>` (the default type parameter), and
// `serde_json::Value`'s `Serialize` implementation never returns `Err`.
// Therefore this branch is unreachable through the public API and cannot
// be covered by integration tests without modifying production code.
//
// The tests below verify the reachable behavior: the stream always
// terminates with a completion event after all data events.
// ============================================================================

/// Verify that stream_to_sse always emits a completion event as the last
/// SSE message, regardless of how many data items were in the input stream.
#[tokio::test]
async fn test_stream_to_sse_always_emits_completion_event() {
    use futures_util::stream;
    use futures_util::StreamExt;

    let values = vec![
        serde_json::json!(1),
        serde_json::json!(2),
        serde_json::json!(3),
    ];
    let input_stream = stream::iter(values);
    let sse_stream = stream_to_sse(input_stream, StreamEvent::data);

    let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

    // Should have 3 data events + 1 completion event = 4 total.
    assert_eq!(
        results.len(),
        4,
        "Expected 4 events (3 data + 1 complete), got: {:?}",
        results
    );

    // The last event should be the completion event.
    let last = results.last().expect("should have at least one result");
    assert!(
        last.contains("complete"),
        "Last event should be the completion event, got: {}",
        last
    );

    // All prior events should be data events.
    for data_event in &results[..results.len() - 1] {
        assert!(
            data_event.contains(r#""type":"data""#),
            "Non-completion event should be a data event, got: {}",
            data_event
        );
    }
}
