// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `StreamResponse`, `StreamEvent`, and `create_stream_channel`:
//! construction, serialization, deserialization, channel behavior, and Debug impls.

use super::{
    create_test_complete_event, create_test_data_event, create_test_error_event,
    create_test_ping_event,
};
use crate::streaming::{create_stream_channel, StreamEvent, StreamResponse};
use futures_util::StreamExt;
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
        assert!(json_str.contains(r#""type":"data""#));
        assert!(json_str.contains("message"));
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
        assert!(json_str.contains(r#""type":"data""#));
        assert!(json_str.contains("test_value"));
    }
}

#[test]
fn test_stream_event_serialization_ping() {
    let event: StreamEvent<()> = StreamEvent::ping();
    let json = serde_json::to_string(&event);
    assert!(json.is_ok());

    if let Ok(json_str) = json {
        assert!(json_str.contains(r#""type":"ping""#));
        assert!(json_str.contains("timestamp"));
    }
}

#[test]
fn test_stream_event_serialization_error() {
    let event: StreamEvent<()> = StreamEvent::error("error msg".to_string());
    let json = serde_json::to_string(&event);
    assert!(json.is_ok());

    if let Ok(json_str) = json {
        assert!(json_str.contains(r#""type":"error""#));
        assert!(json_str.contains("error msg"));
    }
}

#[test]
fn test_stream_event_serialization_complete() {
    let event: StreamEvent<()> = StreamEvent::complete();
    let json = serde_json::to_string(&event);
    assert!(json.is_ok());

    if let Ok(json_str) = json {
        assert!(json_str.contains(r#""type":"complete""#));
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

#[test]
fn test_stream_event_data_deserialize() {
    let event = create_test_data_event();
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<serde_json::Value> = serde_json::from_str(&json).unwrap();
    match deserialized {
        StreamEvent::Data { data, .. } => {
            assert_eq!(data, serde_json::json!({"test": "value"}));
        }
        _ => unreachable!("Expected Data event"),
    }
}

#[test]
fn test_stream_event_ping_deserialize() {
    let event = create_test_ping_event();
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<()> = serde_json::from_str(&json).unwrap();
    match deserialized {
        StreamEvent::Ping { timestamp } => {
            assert!(timestamp > 0);
        }
        _ => unreachable!("Expected Ping event"),
    }
}

#[test]
fn test_stream_event_error_deserialize() {
    let event = create_test_error_event("test error");
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<()> = serde_json::from_str(&json).unwrap();
    match deserialized {
        StreamEvent::Error { message } => {
            assert_eq!(message, "test error");
        }
        _ => unreachable!("Expected Error event"),
    }
}

#[test]
fn test_stream_event_complete_deserialize() {
    let event = create_test_complete_event();
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<()> = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, StreamEvent::Complete));
}

#[test]
fn test_stream_event_data_with_id_and_name_serialization() {
    let event = StreamEvent::Data {
        id: Some("msg-123".to_string()),
        event_name: Some("user_update".to_string()),
        data: serde_json::json!({"user": "alice"}),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains(r#""id":"msg-123""#));
    assert!(json.contains(r#""event_name":"user_update""#));
    assert!(json.contains(r#""user":"alice""#));
}

#[test]
fn test_stream_event_data_with_id_and_name_deserialize() {
    let json = r#"{"type":"data","id":"msg-456","event_name":"notification","data":{"count":42}}"#;
    let event: StreamEvent<serde_json::Value> = serde_json::from_str(json).unwrap();
    match event {
        StreamEvent::Data {
            id,
            event_name,
            data,
        } => {
            assert_eq!(id, Some("msg-456".to_string()));
            assert_eq!(event_name, Some("notification".to_string()));
            assert_eq!(data, serde_json::json!({"count": 42}));
        }
        _ => unreachable!("Expected Data event"),
    }
}

#[test]
fn test_stream_event_numeric_data() {
    let event = StreamEvent::data(42i64);
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<i64> = serde_json::from_str(&json).unwrap();
    match deserialized {
        StreamEvent::Data { data, .. } => {
            assert_eq!(data, 42);
        }
        _ => unreachable!("Expected Data event"),
    }
}

#[test]
fn test_stream_event_array_data() {
    let event = StreamEvent::data(vec![1, 2, 3, 4, 5]);
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<Vec<i32>> = serde_json::from_str(&json).unwrap();
    match deserialized {
        StreamEvent::Data { data, .. } => {
            assert_eq!(data, vec![1, 2, 3, 4, 5]);
        }
        _ => unreachable!("Expected Data event"),
    }
}

#[test]
fn test_stream_event_null_data() {
    let event = StreamEvent::data(serde_json::Value::Null);
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<serde_json::Value> = serde_json::from_str(&json).unwrap();
    match deserialized {
        StreamEvent::Data { data, .. } => {
            assert!(data.is_null());
        }
        _ => unreachable!("Expected Data event"),
    }
}

#[test]
fn test_stream_event_nested_json_data() {
    let nested = serde_json::json!({
        "level1": {
            "level2": {
                "level3": "deep_value"
            }
        }
    });
    let event = StreamEvent::data(nested.clone());
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<serde_json::Value> = serde_json::from_str(&json).unwrap();
    match deserialized {
        StreamEvent::Data { data, .. } => {
            assert_eq!(data, nested);
        }
        _ => unreachable!("Expected Data event"),
    }
}

#[tokio::test]
async fn test_stream_response_into_stream_consumption() {
    let (tx, rx) = mpsc::channel(10);
    let stream_response = StreamResponse::new(ReceiverStream::new(rx));

    tx.send(Ok("item1".to_string())).await.unwrap();
    tx.send(Ok("item2".to_string())).await.unwrap();
    drop(tx);

    let mut stream = stream_response.into_stream();
    let item1 = stream.next().await.unwrap().unwrap();
    let item2 = stream.next().await.unwrap().unwrap();
    let item3 = stream.next().await;

    assert_eq!(item1, "item1");
    assert_eq!(item2, "item2");
    assert!(item3.is_none());
}

#[tokio::test]
async fn test_stream_response_is_final_false_for_new() {
    let (_, rx) = mpsc::channel::<Result<String, String>>(10);
    let stream_response = StreamResponse::new(ReceiverStream::new(rx));
    assert!(!stream_response.is_final());
}

#[tokio::test]
async fn test_stream_response_is_final_true_for_marker() {
    let marker: StreamResponse<String> = StreamResponse::final_marker();
    assert!(marker.is_final());
}

#[tokio::test]
async fn test_create_stream_channel_with_zero_buffer() {
    let (tx, response) = create_stream_channel::<String>(1);
    assert!(!tx.is_closed());

    drop(tx);
    use futures_util::StreamExt;
    let count = response.into_stream().count().await;
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_stream_channel_send_after_drop() {
    let (tx, response) = create_stream_channel::<i32>(10);

    tx.send(Ok(1)).await.unwrap();
    drop(tx);

    use futures_util::StreamExt;
    let results: Vec<_> = response.into_stream().collect().await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].as_ref().unwrap(), &1);
}

#[tokio::test]
async fn test_stream_response_single_cloned_data() {
    let original = "original".to_string();
    let response = StreamResponse::single(original.clone());

    use futures_util::StreamExt;
    let result = response.into_stream().next().await.unwrap().unwrap();

    assert_eq!(result, original);
    assert_eq!(original, "original");
}

#[test]
fn test_stream_event_unicode_data() {
    let unicode_data = "你好世界 🌍 emoji test";
    let event = StreamEvent::data(unicode_data);

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<String> = serde_json::from_str(&json).unwrap();

    match deserialized {
        StreamEvent::Data { data, .. } => {
            assert_eq!(data, "你好世界 🌍 emoji test");
        }
        _ => unreachable!("Expected Data event"),
    }
}

#[test]
fn test_stream_event_special_characters_in_error() {
    let error_msg = r#"Error: "quotes" and \backslashes\ \n newlines \t tabs"#;
    let event: StreamEvent<()> = StreamEvent::error(error_msg.to_string());

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<()> = serde_json::from_str(&json).unwrap();

    match deserialized {
        StreamEvent::Error { message } => {
            assert_eq!(message, error_msg);
        }
        _ => unreachable!("Expected Error event"),
    }
}

#[test]
fn test_stream_event_json_special_characters() {
    let special_json = serde_json::json!({
        "quotes": "\"quoted\"",
        "newline": "line1\nline2",
        "tab": "col1\tcol2",
        "backslash": "path\\to\\file"
    });

    let event = StreamEvent::data(special_json.clone());
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<serde_json::Value> = serde_json::from_str(&json).unwrap();

    match deserialized {
        StreamEvent::Data { data, .. } => {
            assert_eq!(data, special_json);
        }
        _ => unreachable!("Expected Data event"),
    }
}

#[tokio::test]
async fn test_stream_response_concurrent_send() {
    let (tx, response) = create_stream_channel::<i32>(100);
    let tx_clone = tx.clone();

    let handle1 = tokio::spawn(async move {
        for i in 0..50 {
            tx.send(Ok(i)).await.unwrap();
        }
    });

    let handle2 = tokio::spawn(async move {
        for i in 50..100 {
            tx_clone.send(Ok(i)).await.unwrap();
        }
    });

    handle1.await.unwrap();
    handle2.await.unwrap();

    use futures_util::StreamExt;
    let results: Vec<i32> = response.into_stream().map(|r| r.unwrap()).collect().await;

    assert_eq!(results.len(), 100);
}

#[tokio::test]
async fn test_stream_channel_high_throughput() {
    let (tx, response) = create_stream_channel::<usize>(1000);

    let producer = tokio::spawn(async move {
        for i in 0..1000 {
            tx.send(Ok(i)).await.unwrap();
        }
    });

    producer.await.unwrap();

    use futures_util::StreamExt;
    let results: Vec<usize> = response.into_stream().map(|r| r.unwrap()).collect().await;

    assert_eq!(results.len(), 1000);
    assert_eq!(results[0], 0);
    assert_eq!(results[999], 999);
}

#[test]
fn test_stream_event_error_empty_message() {
    let event: StreamEvent<()> = StreamEvent::error("".to_string());
    match event {
        StreamEvent::Error { message } => {
            assert_eq!(message, "");
        }
        _ => unreachable!("Expected Error event"),
    }
}

#[test]
fn test_stream_event_error_long_message() {
    let long_msg = "E".repeat(10000);
    let event: StreamEvent<()> = StreamEvent::error(long_msg.clone());

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: StreamEvent<()> = serde_json::from_str(&json).unwrap();

    match deserialized {
        StreamEvent::Error { message } => {
            assert_eq!(message.len(), 10000);
            assert!(message.chars().all(|c| c == 'E'));
        }
        _ => unreachable!("Expected Error event"),
    }
}

#[test]
fn test_stream_event_ping_timestamp_reasonable() {
    let before = chrono::Utc::now().timestamp();
    let event: StreamEvent<()> = StreamEvent::ping();
    let after = chrono::Utc::now().timestamp();

    match event {
        StreamEvent::Ping { timestamp } => {
            assert!(timestamp >= before);
            assert!(timestamp <= after);
        }
        _ => unreachable!("Expected Ping event"),
    }
}

#[tokio::test]
async fn test_stream_response_into_stream_only_once() {
    let response = StreamResponse::single("test");
    let _stream = response.into_stream();
}

#[tokio::test]
async fn test_create_stream_channel_different_types() {
    let (tx1, _rx1) = create_stream_channel::<String>(10);
    let (tx2, _rx2) = create_stream_channel::<i64>(10);
    let (tx3, _rx3) = create_stream_channel::<Vec<u8>>(10);

    tx1.send(Ok("string".to_string())).await.unwrap();
    tx2.send(Ok(42)).await.unwrap();
    tx3.send(Ok(vec![1, 2, 3])).await.unwrap();
}

#[test]
fn test_stream_event_data_with_none_id_and_name() {
    let event = StreamEvent::Data {
        id: None,
        event_name: None,
        data: serde_json::json!("test"),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains(r#""id":null"#));
    assert!(json.contains(r#""event_name":null"#));
}

#[tokio::test]
async fn test_stream_response_single_immediate_consumption() {
    let response = StreamResponse::single(42i32);

    use futures_util::StreamExt;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let result = response.into_stream().next().await;
    assert!(result.is_some());
    assert_eq!(result.unwrap().unwrap(), 42);
}

// ============================================================================
// StreamResponse new() Tests
// ============================================================================

#[tokio::test]
async fn test_stream_response_new_not_final() {
    let (_, rx) = mpsc::channel::<Result<String, String>>(10);
    let response = StreamResponse::new(ReceiverStream::new(rx));
    assert!(!response.is_final());
}

// ============================================================================
// StreamResponse Debug Tests
// ============================================================================

#[test]
fn test_stream_response_debug() {
    let (_, rx) = mpsc::channel::<Result<String, String>>(10);
    let response: StreamResponse<String> = StreamResponse::new(ReceiverStream::new(rx));
    let debug_str = format!("{:?}", response);
    assert!(debug_str.contains("StreamResponse"));
}

// ============================================================================
// StreamEvent Debug Tests
// ============================================================================

#[test]
fn test_stream_event_debug_data() {
    let event = StreamEvent::data("debug test");
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("Data"));
}

#[test]
fn test_stream_event_debug_ping() {
    let event: StreamEvent<()> = StreamEvent::ping();
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("Ping"));
}

#[test]
fn test_stream_event_debug_error() {
    let event: StreamEvent<()> = StreamEvent::error("debug error".to_string());
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("Error"));
}

#[test]
fn test_stream_event_debug_complete() {
    let event: StreamEvent<()> = StreamEvent::complete();
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("Complete"));
}

// ============================================================================
// StreamResponse Clone Tests (derive)
// ============================================================================

#[test]
fn test_stream_response_struct_is_debug() {
    // Verify StreamResponse derives Debug
    fn assert_debug<T: std::fmt::Debug>() {}
    assert_debug::<StreamResponse<String>>();
}
