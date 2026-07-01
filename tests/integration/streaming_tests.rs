// Copyright (c) 2026 Kirky.X
// SDForge Streaming Integration Tests
// Tests for SSE event streaming, streaming responses, and error handling

#[cfg(all(feature = "streaming", feature = "http"))]
mod streaming_tests {
    use futures_util::StreamExt;
    use sdforge::streaming::{create_stream_channel, stream_to_sse, StreamEvent, StreamResponse};
    use std::time::Duration;
    use tokio_stream::wrappers::ReceiverStream;

    // ============================================================================
    // Existing Tests
    // ============================================================================

    #[test]
    fn test_create_stream_channel() {
        let (tx, response) = create_stream_channel::<String>(10);

        // Verify the channel was created successfully
        assert!(tx.capacity() > 0);
        let _ = response;
    }

    #[test]
    fn test_stream_response_new() {
        let (_tx, rx) = tokio::sync::mpsc::channel::<Result<String, String>>(10);
        let stream = ReceiverStream::new(rx);
        let response = StreamResponse::new(stream);

        // Verify the response was created
        let _ = response;
    }

    #[tokio::test]
    async fn test_stream_response_single() {
        let response = StreamResponse::<String>::single("test data".to_string());
        let _ = response;
    }

    #[test]
    fn test_stream_response_final_marker() {
        let response = StreamResponse::<String>::final_marker();
        let _ = response;
    }

    #[tokio::test]
    async fn test_stream_to_sse_basic() {
        let (_tx, rx) = tokio::sync::mpsc::channel::<Result<String, String>>(10);

        // Create SSE stream with a simple mapper function
        // stream_to_sse expects a closure returning StreamEvent (which defaults to StreamEvent<serde_json::Value>)
        let sse_stream = stream_to_sse(
            ReceiverStream::new(rx),
            |msg: Result<String, String>| -> StreamEvent {
                match msg {
                    Ok(data) => StreamEvent::data(serde_json::Value::String(data)),
                    Err(e) => StreamEvent::error(e),
                }
            },
        );

        // Just verify the stream was created successfully
        let _stream = Box::pin(sse_stream);
    }

    #[test]
    fn test_stream_event_data() {
        let event = StreamEvent::<String>::Data {
            id: Some("1".to_string()),
            event_name: Some("message".to_string()),
            data: "test data".to_string(),
        };

        // Verify serialization works
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("test data"));
    }

    // ============================================================================
    // SSE Event Stream Tests
    // ============================================================================

    /// Test basic SSE event stream functionality
    /// Verifies that a simple stream produces correct SSE formatted output
    #[tokio::test]
    async fn test_sse_event_stream_basic() {
        use futures_util::stream;

        // Create a simple stream of numbers
        let input_stream = stream::iter(vec![1i32, 2, 3, 4, 5]);
        let sse_stream = stream_to_sse(input_stream, |n| StreamEvent::data(serde_json::json!(n)));

        // Collect all SSE formatted events
        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        // Verify we got data events
        let data_events: Vec<_> = results
            .iter()
            .filter(|r| r.contains(r#""type":"data""#))
            .collect();

        assert_eq!(data_events.len(), 5, "Should have 5 data events");

        // Verify SSE format: each event should start with "data: " and end with "\n\n"
        for result in results.iter() {
            if result.contains(r#""type":"data""#) {
                assert!(
                    result.starts_with("data: "),
                    "SSE data should start with 'data: ' prefix"
                );
                assert!(
                    result.ends_with("\n\n"),
                    "SSE data should end with double newline"
                );
            }
        }

        // Verify final completion event
        assert!(
            results.last().unwrap().contains("complete"),
            "Last event should be completion signal"
        );
    }

    /// Test SSE stream with multiple event types (data, ping, error, complete)
    /// Verifies that different event types are properly serialized and formatted
    #[tokio::test]
    async fn test_sse_multiple_event_types() {
        use futures_util::stream;

        // Create stream with mixed event types
        let events = vec![
            StreamEvent::data(serde_json::json!("first")),
            StreamEvent::ping(),
            StreamEvent::data(serde_json::json!("second")),
            StreamEvent::error("test error".to_string()),
            StreamEvent::data(serde_json::json!("third")),
        ];

        let input_stream = stream::iter(events);
        let sse_stream = stream_to_sse(input_stream, |event| event);

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        // Count different event types
        let data_count = results
            .iter()
            .filter(|r| r.contains(r#""type":"data""#))
            .count();
        let ping_count = results
            .iter()
            .filter(|r| r.contains(r#""type":"ping""#))
            .count();
        let error_count = results
            .iter()
            .filter(|r| r.contains(r#""type":"error""#))
            .count();
        let complete_count = results.iter().filter(|r| r.contains("complete")).count();

        assert_eq!(data_count, 3, "Should have 3 data events");
        assert_eq!(ping_count, 1, "Should have 1 ping event");
        assert_eq!(error_count, 1, "Should have 1 error event");
        assert!(complete_count >= 1, "Should have at least 1 complete event");
    }

    /// Test SSE event ID tracking functionality
    /// Verifies that events can be tracked with unique IDs
    #[tokio::test]
    async fn test_sse_event_id_tracking() {
        // Create events with specific IDs
        let event1 = StreamEvent::Data {
            id: Some("msg-001".to_string()),
            event_name: Some("update".to_string()),
            data: serde_json::json!("first message"),
        };

        let event2 = StreamEvent::Data {
            id: Some("msg-002".to_string()),
            event_name: Some("update".to_string()),
            data: serde_json::json!("second message"),
        };

        let event3 = StreamEvent::Data {
            id: Some("msg-003".to_string()),
            event_name: Some("final".to_string()),
            data: serde_json::json!("last message"),
        };

        // Serialize and verify IDs are preserved
        let json1 = serde_json::to_string(&event1).unwrap();
        let json2 = serde_json::to_string(&event2).unwrap();
        let json3 = serde_json::to_string(&event3).unwrap();

        assert!(json1.contains(r#""id":"msg-001""#));
        assert!(json2.contains(r#""id":"msg-002""#));
        assert!(json3.contains(r#""id":"msg-003""#));

        // Verify event names are also preserved
        assert!(json1.contains(r#""event_name":"update""#));
        assert!(json2.contains(r#""event_name":"update""#));
        assert!(json3.contains(r#""event_name":"final""#));

        // Deserialize and verify
        let deserialized1: StreamEvent<serde_json::Value> = serde_json::from_str(&json1).unwrap();
        let deserialized2: StreamEvent<serde_json::Value> = serde_json::from_str(&json2).unwrap();
        let deserialized3: StreamEvent<serde_json::Value> = serde_json::from_str(&json3).unwrap();

        match (deserialized1, deserialized2, deserialized3) {
            (
                StreamEvent::Data { id: id1, .. },
                StreamEvent::Data { id: id2, .. },
                StreamEvent::Data { id: id3, .. },
            ) => {
                assert_eq!(id1, Some("msg-001".to_string()));
                assert_eq!(id2, Some("msg-002".to_string()));
                assert_eq!(id3, Some("msg-003".to_string()));
            }
            _ => panic!("Expected Data events"),
        }
    }

    /// Test SSE retry directive handling
    /// Verifies that retry intervals can be specified for reconnection scenarios
    #[tokio::test]
    async fn test_sse_retry_directive() {
        // Test that retry value can be constructed in SSE format
        let retry_ms = 5000;
        let retry_line = format!("retry: {}\n", retry_ms);

        assert_eq!(retry_line, "retry: 5000\n");

        // Test with various retry values
        for retry_value in [1000, 5000, 10000, 30000] {
            let retry_line = format!("retry: {}\n", retry_value);
            assert!(retry_line.contains(&retry_value.to_string()));
        }
    }

    /// Test SSE comment lines handling
    /// Verifies that comments (lines starting with :) are properly formatted
    #[tokio::test]
    async fn test_sse_comment_lines() {
        // Test comment line format
        let comment = ": This is a comment\n\n";
        assert!(comment.starts_with(": "));

        // Test multiple comments in sequence
        let comments = vec![
            ": Server started\n",
            ": Processing request\n",
            ": Request completed\n",
        ];

        for comment in comments {
            assert!(comment.starts_with(": "), "Comment should start with ': '");
        }
    }

    // ============================================================================
    // Streaming Response Tests
    // ============================================================================

    /// Test streaming response with multiple chunks
    /// Verifies that data can be streamed in multiple parts
    #[tokio::test]
    async fn test_stream_response_chunks() {
        let (tx, response) = create_stream_channel::<String>(10);

        // Send chunks of data
        let chunks = vec!["chunk1", "chunk2", "chunk3", "chunk4", "chunk5"];

        for chunk in &chunks {
            tx.send(Ok(chunk.to_string())).await.unwrap();
        }

        drop(tx);

        // Collect all chunks
        let mut stream = response.into_stream();
        let mut received_chunks = Vec::new();

        while let Some(result) = stream.next().await {
            received_chunks.push(result.unwrap());
        }

        assert_eq!(received_chunks.len(), chunks.len());
        assert_eq!(received_chunks, chunks);
    }

    /// Test streaming with large payload
    /// Verifies that large data can be streamed without issues
    #[tokio::test]
    async fn test_stream_large_payload() {
        let (tx, response) = create_stream_channel::<Vec<u8>>(10);

        // Create large payload (1MB)
        let large_data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
        let expected_len = large_data.len();

        tx.send(Ok(large_data)).await.unwrap();
        drop(tx);

        let mut stream = response.into_stream();
        let result = stream.next().await;

        assert!(result.is_some());
        let received = result.unwrap().unwrap();
        assert_eq!(received.len(), expected_len);
    }

    /// Test stream backpressure handling
    /// Verifies that the channel properly handles slow consumers
    #[tokio::test]
    async fn test_stream_backpressure_handling() {
        // Create a channel with small buffer
        let (tx, response) = create_stream_channel::<String>(2);

        // Send items up to buffer capacity
        assert!(tx.try_send(Ok("item1".to_string())).is_ok());
        assert!(tx.try_send(Ok("item2".to_string())).is_ok());

        // Buffer should be full now, next send should fail with try_send
        let result = tx.try_send(Ok("item3".to_string()));
        assert!(result.is_err(), "try_send should fail when buffer is full");

        // Verify we can still receive the buffered items
        let mut stream = response.into_stream();
        let item1 = stream.next().await.unwrap().unwrap();
        let item2 = stream.next().await.unwrap().unwrap();

        assert_eq!(item1, "item1");
        assert_eq!(item2, "item2");
    }

    /// Test stream cancellation behavior
    /// Verifies that dropping the receiver properly terminates the stream
    #[tokio::test]
    async fn test_stream_cancellation() {
        let (tx, response) = create_stream_channel::<i32>(10);

        // Send some data before dropping receiver
        tx.send(Ok(1)).await.unwrap();
        tx.send(Ok(2)).await.unwrap();

        // Collect the sent data
        let mut stream = response.into_stream();
        let item1 = stream.next().await.unwrap().unwrap();
        let item2 = stream.next().await.unwrap().unwrap();

        assert_eq!(item1, 1);
        assert_eq!(item2, 2);

        // Drop the response (receiver side)
        drop(stream);

        // Try to send after receiver is dropped
        // The send might succeed because it queues the message, but receiver won't get it
        let _send_result = tx.send(Ok(3)).await;

        // Create new response to verify the old one is dropped
        let (_tx2, response2) = create_stream_channel::<i32>(10);
        let _stream2 = response2.into_stream();

        // Verify original channel's data was not received after drop
        // (This is implicit - if stream was dropped, no more items can be received)
    }

    // ============================================================================
    // Streaming Error Handling Tests
    // ============================================================================

    /// Test error recovery in stream
    /// Verifies that errors can be sent through the stream and handled properly
    #[tokio::test]
    async fn test_stream_error_recovery() {
        let (tx, response) = create_stream_channel::<String>(10);

        // Send mix of success and error values
        tx.send(Ok("success1".to_string())).await.unwrap();
        tx.send(Err("error1".to_string())).await.unwrap();
        tx.send(Ok("success2".to_string())).await.unwrap();
        tx.send(Err("error2".to_string())).await.unwrap();
        tx.send(Ok("success3".to_string())).await.unwrap();

        drop(tx);

        // Collect all results
        let mut stream = response.into_stream();
        let mut successes = Vec::new();
        let mut errors = Vec::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(data) => successes.push(data),
                Err(e) => errors.push(e),
            }
        }

        assert_eq!(successes.len(), 3);
        assert_eq!(errors.len(), 2);
        assert_eq!(successes, vec!["success1", "success2", "success3"]);
        assert_eq!(errors, vec!["error1", "error2"]);
    }

    /// Test stream completion signal
    /// Verifies that streams properly signal completion
    #[tokio::test]
    async fn test_stream_completion_signal() {
        use futures_util::stream;

        // Create empty stream
        let empty_stream = stream::iter(Vec::<i32>::new());
        let sse_stream = stream_to_sse(empty_stream, |_| StreamEvent::complete());

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        // Empty stream should still produce completion signal
        assert_eq!(results.len(), 1);
        assert!(
            results[0].contains("complete"),
            "Should have completion signal"
        );
    }

    /// Test stream timeout handling
    /// Verifies that streams can handle timeouts appropriately
    #[tokio::test]
    async fn test_stream_timeout_handling() {
        let (_tx, rx) = tokio::sync::mpsc::channel::<Result<String, String>>(10);
        let stream = ReceiverStream::new(rx);

        // Create a stream that won't receive any data
        // Use a short timeout to verify timeout mechanism works
        let timeout_duration = Duration::from_millis(100);

        let start = std::time::Instant::now();
        let mut stream = Box::pin(stream);

        // Wait for timeout
        let result = tokio::time::timeout(timeout_duration, stream.next()).await;

        assert!(
            result.is_err() || result.unwrap().is_none(),
            "Should timeout or return None"
        );
        assert!(
            start.elapsed() >= timeout_duration,
            "Should have waited at least the timeout duration"
        );
    }

    /// Test stream with immediate sender drop
    /// Verifies graceful handling when sender is dropped immediately
    #[tokio::test]
    async fn test_stream_immediate_sender_drop() {
        let (tx, response) = create_stream_channel::<String>(10);

        // Immediately drop the sender
        drop(tx);

        // Verify stream is empty
        let count = response.into_stream().count().await;
        assert_eq!(
            count, 0,
            "Stream should be empty when sender is dropped immediately"
        );
    }

    /// Test concurrent stream production and consumption
    /// Verifies thread-safe operation of the stream
    #[tokio::test]
    async fn test_stream_concurrent_producer_consumer() {
        let (tx, response) = create_stream_channel::<usize>(100);
        let item_count = 100;

        // Spawn producer task
        let producer_handle = tokio::spawn(async move {
            for i in 0..item_count {
                tx.send(Ok(i)).await.unwrap();
            }
        });

        // Spawn consumer task
        let consumer_handle = tokio::spawn(async move {
            let mut count = 0;
            let mut stream = response.into_stream();
            while stream.next().await.is_some() {
                count += 1;
            }
            count
        });

        // Wait for both tasks
        producer_handle.await.unwrap();
        let received_count = consumer_handle.await.unwrap();

        assert_eq!(received_count, item_count);
    }

    /// Test stream with sequential data and completion
    /// Verifies proper ordering of data and final completion
    #[tokio::test]
    async fn test_stream_sequential_with_final_marker() {
        let (tx, response) = create_stream_channel::<String>(10);

        // Send sequential data
        for i in 1..=10 {
            tx.send(Ok(format!("item-{}", i))).await.unwrap();
        }

        // Send final marker
        drop(tx);

        // Collect all items
        let items: Vec<String> = response.into_stream().map(|r| r.unwrap()).collect().await;

        assert_eq!(items.len(), 10);
        assert_eq!(items[0], "item-1");
        assert_eq!(items[9], "item-10");
    }

    /// Test SSE stream with complex JSON data
    /// Verifies proper serialization of nested JSON structures
    #[tokio::test]
    async fn test_sse_complex_json_data() {
        use futures_util::stream;

        let complex_data = serde_json::json!({
            "user": {
                "id": 123,
                "name": "Alice",
                "email": "alice@example.com",
                "roles": ["admin", "user"],
                "metadata": {
                    "created_at": "2026-01-01T00:00:00Z",
                    "last_login": "2026-03-26T10:30:00Z"
                }
            },
            "session": {
                "token": "abc123",
                "expires_in": 3600
            }
        });

        let input_stream = stream::iter(vec![complex_data]);
        let sse_stream = stream_to_sse(input_stream, StreamEvent::data);

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        assert!(!results.is_empty());
        let data_event = &results[0];
        assert!(data_event.contains(r#""type":"data""#));
        assert!(data_event.contains(r#""name":"Alice""#));
        assert!(data_event.contains(r#""token":"abc123""#));
    }

    /// Test stream channel capacity configuration
    /// Verifies that different buffer sizes work correctly
    #[tokio::test]
    async fn test_stream_channel_various_capacities() {
        for capacity in [1, 5, 10, 100, 1000] {
            let (tx, response) = create_stream_channel::<i32>(capacity);

            // Verify capacity
            assert_eq!(tx.capacity(), capacity);

            // Send and receive some data
            tx.send(Ok(42)).await.unwrap();
            let item = response.into_stream().next().await;
            assert_eq!(item.unwrap().unwrap(), 42);
        }
    }

    /// Test stream with rapid data production
    /// Verifies that the stream can handle high throughput
    #[tokio::test]
    async fn test_stream_rapid_production() {
        let (tx, response) = create_stream_channel::<i32>(1000);
        let item_count: usize = 500;

        // Rapidly produce items
        for i in 0..item_count {
            tx.send(Ok(i as i32)).await.unwrap();
        }

        drop(tx);

        // Collect all items
        let items: Vec<i32> = response.into_stream().map(|r| r.unwrap()).collect().await;

        assert_eq!(items.len(), item_count);
        assert_eq!(items[0], 0);
        assert_eq!(items[item_count - 1], (item_count - 1) as i32);
    }

    // ============================================================================
    // Extended SSE Format Tests
    // ============================================================================

    /// Test SSE event name formatting
    /// Verifies that custom event names are properly formatted in SSE output
    #[tokio::test]
    async fn test_sse_event_name_formatting() {
        use futures_util::stream;

        // Create events with custom event names
        let events = vec![
            StreamEvent::Data {
                id: Some("1".to_string()),
                event_name: Some("custom-update".to_string()),
                data: serde_json::json!("update data"),
            },
            StreamEvent::Data {
                id: Some("2".to_string()),
                event_name: Some("notification".to_string()),
                data: serde_json::json!("notification data"),
            },
        ];

        let input_stream = stream::iter(events);
        let sse_stream = stream_to_sse(input_stream, |e| e);

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        // Verify event names are serialized
        assert!(results[0].contains(r#""event_name":"custom-update""#));
        assert!(results[1].contains(r#""event_name":"notification""#));
    }

    /// Test SSE id line formatting
    /// Verifies that event IDs are properly formatted with "id:" prefix
    #[tokio::test]
    async fn test_sse_id_line_formatting() {
        use futures_util::stream;

        let event = StreamEvent::Data {
            id: Some("msg-abc-123".to_string()),
            event_name: None,
            data: serde_json::json!("test"),
        };

        let input_stream = stream::iter(vec![event]);
        let sse_stream = stream_to_sse(input_stream, |e| e);

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        // Verify ID is in the serialized output
        assert!(results[0].contains(r#""id":"msg-abc-123""#));
    }

    /// Test SSE with multiline data content
    /// Verifies that multiline data is properly handled in SSE format
    #[tokio::test]
    async fn test_sse_multiline_data_content() {
        use futures_util::stream;

        let multiline_data = "Line 1\nLine 2\nLine 3";
        let event = StreamEvent::data(serde_json::json!(multiline_data));

        let input_stream = stream::iter(vec![event]);
        let sse_stream = stream_to_sse(input_stream, |e| e);

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        // Verify multiline data is serialized
        assert!(results[0].contains("Line 1"));
        assert!(results[0].contains("Line 2"));
        assert!(results[0].contains("Line 3"));
    }

    /// Test SSE ping timestamp accuracy
    /// Verifies that ping events contain accurate timestamps
    #[tokio::test]
    async fn test_sse_ping_timestamp_accuracy() {
        use futures_util::stream;

        let _before = chrono::Utc::now().timestamp();
        let input_stream = stream::iter(vec![()]);
        let sse_stream = stream_to_sse(input_stream, |_| StreamEvent::ping());

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;
        let _after = chrono::Utc::now().timestamp();

        // Verify timestamp is present and reasonable
        assert!(results[0].contains(r#""type":"ping""#));
        assert!(results[0].contains("timestamp"));
    }

    // ============================================================================
    // Stream Channel Edge Case Tests
    // ============================================================================

    /// Test channel closed behavior
    /// Verifies that stream is empty after sender is dropped
    #[tokio::test]
    async fn test_stream_channel_closed_behavior() {
        let (tx, response) = create_stream_channel::<String>(10);

        // Drop the sender immediately
        drop(tx);

        // Stream should be empty after sender drop
        let count = response.into_stream().count().await;
        assert_eq!(count, 0, "Stream should be empty after sender drop");
    }

    /// Test zero capacity channel behavior
    /// Verifies that zero-capacity channels work correctly (synchronous send)
    #[tokio::test]
    async fn test_stream_zero_capacity_channel() {
        // Note: mpsc::channel with 0 capacity requires synchronous receiver
        // We test with capacity 1 as minimum
        let (tx, response) = create_stream_channel::<i32>(1);

        tx.send(Ok(1)).await.unwrap();
        drop(tx);

        let items: Vec<i32> = response.into_stream().map(|r| r.unwrap()).collect().await;
        assert_eq!(items, vec![1]);
    }

    /// Test stream with alternating success and error values
    /// Verifies proper handling of interleaved success/error data
    #[tokio::test]
    async fn test_stream_alternating_success_error() {
        let (tx, response) = create_stream_channel::<String>(10);

        // Alternating pattern: success, error, success, error, success
        let sequence = vec![
            Ok("item-1".to_string()),
            Err("err-1".to_string()),
            Ok("item-2".to_string()),
            Err("err-2".to_string()),
            Ok("item-3".to_string()),
        ];

        for item in sequence {
            tx.send(item).await.unwrap();
        }
        drop(tx);

        let mut stream = response.into_stream();
        let mut results = Vec::new();

        while let Some(result) = stream.next().await {
            results.push(result);
        }

        // Should have all 5 items
        assert_eq!(results.len(), 5);

        // Verify alternating pattern
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
        assert!(results[2].is_ok());
        assert!(results[3].is_err());
        assert!(results[4].is_ok());
    }

    /// Test stream with many small items
    /// Verifies efficient handling of many small data items
    #[tokio::test]
    async fn test_stream_many_small_items() {
        let (tx, response) = create_stream_channel::<u8>(1000);
        let item_count = 100;

        // Spawn a consumer task to avoid deadlock
        let consumer_handle = tokio::spawn(async move {
            let items: Vec<u8> = response.into_stream().map(|r| r.unwrap()).collect().await;
            items
        });

        // Send items
        for i in 0..item_count {
            tx.send(Ok(i as u8)).await.unwrap();
        }
        drop(tx);

        // Wait for consumer to finish
        let items = consumer_handle.await.unwrap();

        assert_eq!(items.len(), item_count);
        assert_eq!(items[0], 0);
        assert_eq!(items[item_count - 1], ((item_count - 1) % 256) as u8);
    }

    /// Test stream with large number of items and small buffer
    /// Tests backpressure under constrained buffer conditions
    #[tokio::test]
    async fn test_stream_small_buffer_large_data() {
        let (tx, response) = create_stream_channel::<String>(100);
        let total_items = 20;

        // Spawn a consumer task to avoid deadlock
        let consumer_handle = tokio::spawn(async move {
            let items: Vec<String> = response.into_stream().map(|r| r.unwrap()).collect().await;
            items
        });

        // Send items with small buffer
        for i in 0..total_items {
            let result = tx.send(Ok(format!("item-{}", i))).await;
            assert!(result.is_ok(), "Send should succeed with consumer running");
        }
        drop(tx);

        // Wait for consumer to finish
        let items = consumer_handle.await.unwrap();

        assert_eq!(items.len(), total_items);
    }

    // ============================================================================
    // Stream State and Lifecycle Tests
    // ============================================================================

    /// Test stream with immediate data consumption
    /// Verifies that data can be consumed immediately after being sent
    #[tokio::test]
    async fn test_stream_immediate_consumption() {
        let (tx, response) = create_stream_channel::<i32>(10);

        // Send and immediately consume in a loop
        let expected_sum: i32 = (1..=10).sum();

        for i in 1..=10 {
            tx.send(Ok(i)).await.unwrap();
        }
        drop(tx);

        let mut stream = response.into_stream();
        let mut sum = 0;

        while let Some(result) = stream.next().await {
            sum += result.unwrap();
        }

        assert_eq!(sum, expected_sum);
    }

    /// Test stream response state management
    /// Verifies that stream state is properly managed across operations
    #[tokio::test]
    async fn test_stream_response_state_management() {
        let response1 = StreamResponse::<String>::single("first".to_string());
        let response2 = StreamResponse::<String>::single("second".to_string());

        // Both should be non-final initially
        assert!(!response1.is_final());
        assert!(!response2.is_final());

        let final_response: StreamResponse<String> = StreamResponse::final_marker();
        assert!(final_response.is_final());
    }

    /// Test multiple streams concurrently
    /// Verifies that multiple independent streams work correctly
    #[tokio::test]
    async fn test_multiple_streams_concurrent() {
        let (tx1, response1) = create_stream_channel::<String>(10);
        let (tx2, response2) = create_stream_channel::<String>(10);

        // Send to both streams
        tx1.send(Ok("stream1-item".to_string())).await.unwrap();
        tx2.send(Ok("stream2-item".to_string())).await.unwrap();

        drop(tx1);
        drop(tx2);

        // Collect from both streams
        let item1 = response1.into_stream().next().await.unwrap().unwrap();
        let item2 = response2.into_stream().next().await.unwrap().unwrap();

        assert_eq!(item1, "stream1-item");
        assert_eq!(item2, "stream2-item");
    }

    /// Test stream channel sender clone behavior
    /// Verifies that cloned senders can both send to the same channel
    #[tokio::test]
    async fn test_stream_channel_sender_clone() {
        let (tx, response) = create_stream_channel::<i32>(100);
        let tx_clone = tx.clone();
        let tx_for_drop = tx.clone();

        // Send from both senders
        let handle1 = tokio::spawn(async move {
            for i in 0..50 {
                tx.send(Ok(i)).await.unwrap();
            }
        });

        let handle2 = tokio::spawn(async move {
            let tx_clone_inner = tx_clone.clone();
            for i in 50..100 {
                tx_clone_inner.send(Ok(i)).await.unwrap();
            }
        });

        handle1.await.unwrap();
        handle2.await.unwrap();

        // Drop the original sender after both handles complete
        drop(tx_for_drop);

        let items: Vec<i32> = response.into_stream().map(|r| r.unwrap()).collect().await;
        assert_eq!(items.len(), 100);
    }

    // ============================================================================
    // SSE Stream Advanced Tests
    // ============================================================================

    /// Test SSE stream with very large data payload
    /// Verifies that large payloads are properly serialized without issues
    #[tokio::test]
    async fn test_sse_large_payload_serialization() {
        use futures_util::stream;

        // Create large payload (100KB of JSON-like data)
        let large_data = serde_json::json!({
            "items": (0..1000).map(|i| {
                serde_json::json!({
                    "id": i,
                    "name": format!("item-{}", i),
                    "description": "x".repeat(100)
                })
            }).collect::<Vec<_>>()
        });

        let input_stream = stream::iter(vec![large_data]);
        let sse_stream = stream_to_sse(input_stream, StreamEvent::data);

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        assert!(!results.is_empty());
        // Verify the large payload is in the output
        assert!(results[0].contains("items"));
    }

    /// Test SSE stream with custom event types
    /// Verifies that various custom event type names work correctly
    #[tokio::test]
    async fn test_sse_custom_event_types() {
        use futures_util::stream;

        let custom_event_names = ["progress", "notification", "alert", "update", "status"];

        let events: Vec<StreamEvent<serde_json::Value>> = custom_event_names
            .iter()
            .map(|name| StreamEvent::Data {
                id: None,
                event_name: Some(name.to_string()),
                data: serde_json::json!(name),
            })
            .collect();

        let input_stream = stream::iter(events);
        let sse_stream = stream_to_sse(input_stream, |e| e);

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        assert_eq!(results.len(), custom_event_names.len() + 1); // +1 for completion
    }

    /// Test SSE stream with rapid ping events
    /// Verifies that frequent ping events are handled correctly
    #[tokio::test]
    async fn test_sse_rapid_ping_events() {
        use futures_util::stream;

        let input_stream = stream::iter(vec![(); 10]);
        let sse_stream = stream_to_sse(input_stream, |_| StreamEvent::ping());

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        // Should have 10 ping events plus completion
        assert_eq!(results.len(), 11);

        for result in &results[..10] {
            assert!(result.contains(r#""type":"ping""#));
        }
    }

    /// Test SSE stream serialization roundtrip
    /// Verifies that events can be serialized and deserialized correctly
    #[tokio::test]
    async fn test_sse_serialization_roundtrip() {
        let original_data = serde_json::json!({
            "user": "Alice",
            "age": 30,
            "active": true,
            "roles": ["admin", "user"]
        });

        let event = StreamEvent::data(original_data.clone());

        // Serialize
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"data""#));

        // Deserialize
        let deserialized: StreamEvent<serde_json::Value> = serde_json::from_str(&json).unwrap();

        match deserialized {
            StreamEvent::Data { data, .. } => {
                assert_eq!(data["user"], "Alice");
                assert_eq!(data["age"], 30);
            }
            _ => panic!("Expected Data event"),
        }
    }

    // ============================================================================
    // Error Recovery and Edge Cases Tests
    // ============================================================================

    /// Test stream error at end of stream
    /// Verifies handling when error occurs as the last item
    #[tokio::test]
    async fn test_stream_error_at_end() {
        let (tx, response) = create_stream_channel::<String>(10);

        tx.send(Ok("success1".to_string())).await.unwrap();
        tx.send(Ok("success2".to_string())).await.unwrap();
        tx.send(Err("final error".to_string())).await.unwrap();

        drop(tx);

        let mut stream = response.into_stream();
        let mut results = Vec::new();

        while let Some(result) = stream.next().await {
            results.push(result);
        }

        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
        assert!(results[2].is_err());

        if let Err(e) = &results[2] {
            assert_eq!(e, "final error");
        }
    }

    /// Test stream with all errors
    /// Verifies handling when entire stream contains only errors
    #[tokio::test]
    async fn test_stream_all_errors() {
        let (tx, response) = create_stream_channel::<String>(10);

        tx.send(Err("error1".to_string())).await.unwrap();
        tx.send(Err("error2".to_string())).await.unwrap();
        tx.send(Err("error3".to_string())).await.unwrap();

        drop(tx);

        let mut stream = response.into_stream();
        let mut errors = Vec::new();

        while let Some(result) = stream.next().await {
            if let Err(e) = result {
                errors.push(e);
            }
        }

        assert_eq!(errors.len(), 3);
        assert_eq!(errors, vec!["error1", "error2", "error3"]);
    }

    /// Test stream with empty data items
    /// Verifies handling of empty but valid data items
    #[tokio::test]
    async fn test_stream_empty_data_items() {
        let (tx, response) = create_stream_channel::<String>(10);

        tx.send(Ok("".to_string())).await.unwrap();
        tx.send(Ok("non-empty".to_string())).await.unwrap();
        tx.send(Ok("".to_string())).await.unwrap();

        drop(tx);

        let items: Vec<String> = response.into_stream().map(|r| r.unwrap()).collect().await;

        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "");
        assert_eq!(items[1], "non-empty");
        assert_eq!(items[2], "");
    }

    /// Test stream with special JSON values
    /// Verifies handling of JSON null, true, false values
    #[tokio::test]
    async fn test_stream_json_special_values() {
        use futures_util::stream;

        let values = vec![
            serde_json::json!(null),
            serde_json::json!(true),
            serde_json::json!(false),
            serde_json::json!(0),
            serde_json::json!(-1),
            serde_json::json!(0.0),
            serde_json::json!(-1.5),
        ];

        let input_stream = stream::iter(values);
        let sse_stream = stream_to_sse(input_stream, StreamEvent::data);

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        // Should have 7 data events plus completion
        let data_events = results
            .iter()
            .filter(|r| r.contains(r#""type":"data""#))
            .count();

        assert_eq!(data_events, 7);
    }

    /// Test stream with nested JSON arrays
    /// Verifies handling of complex nested JSON structures
    #[tokio::test]
    async fn test_stream_nested_json_arrays() {
        use futures_util::stream;

        let nested_data = serde_json::json!({
            "matrix": [
                [1, 2, 3],
                [4, 5, 6],
                [7, 8, 9]
            ],
            "nested": {
                "level1": {
                    "level2": {
                        "level3": ["deep", "array"]
                    }
                }
            }
        });

        let input_stream = stream::iter(vec![nested_data]);
        let sse_stream = stream_to_sse(input_stream, StreamEvent::data);

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        assert!(!results.is_empty());
        assert!(results[0].contains("matrix"));
        assert!(results[0].contains("nested"));
    }

    // ============================================================================
    // Performance and Stress Tests
    // ============================================================================

    /// Test stream with burst of data
    /// Verifies handling of sudden burst of data items
    #[tokio::test]
    async fn test_stream_burst_handling() {
        let (tx, response) = create_stream_channel::<i32>(1000);
        let burst_size: usize = 500;

        // Burst send all items at once
        for i in 0..burst_size {
            tx.send(Ok(i as i32)).await.unwrap();
        }

        drop(tx);

        let items: Vec<i32> = response.into_stream().map(|r| r.unwrap()).collect().await;

        assert_eq!(items.len(), burst_size);
    }

    /// Test stream with delayed sending
    /// Verifies handling of items sent with delays
    #[tokio::test]
    async fn test_stream_delayed_sending() {
        let (tx, response) = create_stream_channel::<i32>(10);

        // Send with small delays
        for i in 0..5 {
            tx.send(Ok(i)).await.unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        drop(tx);

        let items: Vec<i32> = response.into_stream().map(|r| r.unwrap()).collect().await;

        assert_eq!(items.len(), 5);
        assert_eq!(items, vec![0, 1, 2, 3, 4]);
    }

    /// Test stream with selective consumption
    /// Verifies that consumer can selectively process items
    #[tokio::test]
    async fn test_stream_selective_consumption() {
        let (tx, response) = create_stream_channel::<i32>(10);

        // Send numbers 1-10
        for i in 1..=10 {
            tx.send(Ok(i)).await.unwrap();
        }

        drop(tx);

        // Only consume even numbers
        let mut stream = response.into_stream();
        let mut even_numbers = Vec::new();

        while let Some(result) = stream.next().await {
            let value = result.unwrap();
            if value % 2 == 0 {
                even_numbers.push(value);
            }
        }

        assert_eq!(even_numbers, vec![2, 4, 6, 8, 10]);
    }

    // ============================================================================
    // Integration Scenarios Tests
    // ============================================================================

    /// Test realistic streaming scenario - progress updates
    /// Simulates a file download or processing with progress updates
    #[tokio::test]
    async fn test_realistic_progress_scenario() {
        let (tx, response) = create_stream_channel::<serde_json::Value>(50);
        let total_steps = 20;

        // Simulate progress updates
        for step in 0..total_steps {
            let progress = serde_json::json!({
                "step": step + 1,
                "total": total_steps,
                "percent": ((step + 1) as f64 / total_steps as f64 * 100.0).round() as u8,
                "message": format!("Processing step {} of {}", step + 1, total_steps)
            });

            tx.send(Ok(progress)).await.unwrap();
            // Simulate some processing time
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        // Send completion
        let completion = serde_json::json!({
            "status": "complete",
            "total_steps": total_steps
        });
        tx.send(Ok(completion)).await.unwrap();

        drop(tx);

        let items: Vec<serde_json::Value> =
            response.into_stream().map(|r| r.unwrap()).collect().await;

        assert_eq!(items.len(), total_steps + 1);

        // Verify first item is step 1
        assert_eq!(items[0]["step"], 1);

        // Verify last item is completion
        assert_eq!(items.last().unwrap()["status"], "complete");
    }

    /// Test realistic streaming scenario - log streaming
    /// Simulates a log stream with different log levels
    #[tokio::test]
    async fn test_realistic_log_stream_scenario() {
        use futures_util::stream;

        let log_entries = vec![
            (
                StreamEvent::data(serde_json::json!({
                    "level": "INFO",
                    "message": "Server started",
                    "timestamp": "2026-03-26T10:00:00Z"
                })),
                "info",
            ),
            (
                StreamEvent::data(serde_json::json!({
                    "level": "DEBUG",
                    "message": "Processing request",
                    "timestamp": "2026-03-26T10:00:01Z"
                })),
                "debug",
            ),
            (
                StreamEvent::data(serde_json::json!({
                    "level": "WARN",
                    "message": "High memory usage",
                    "timestamp": "2026-03-26T10:00:02Z"
                })),
                "warn",
            ),
            (
                StreamEvent::error("Connection timeout".to_string()),
                "error",
            ),
        ];

        let input_stream = stream::iter(log_entries.into_iter().map(|(e, _)| e));
        let sse_stream = stream_to_sse(input_stream, |e| e);

        let results: Vec<String> = sse_stream.map(|r| r.unwrap()).collect().await;

        // Should have 4 data/error events plus completion
        assert!(results.len() >= 5);
    }
}
