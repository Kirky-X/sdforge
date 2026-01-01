//! Streaming response integration tests
//!
//! Tests for SSE streaming functionality.

#[cfg(all(test, feature = "http", feature = "streaming"))]
mod streaming_integration_tests {
    use axiom::streaming::{StreamEvent, StreamResponse};
    use serde::{Deserialize, Serialize};
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::ReceiverStream;

    #[derive(Debug, Serialize, Deserialize)]
    struct LogEntry {
        timestamp: i64,
        level: String,
        message: String,
    }

    #[tokio::test]
    async fn test_stream_response_creation() {
        let (_tx, rx) = mpsc::channel(32);
        let stream_response: StreamResponse<String> = StreamResponse::new(ReceiverStream::new(rx));

        assert!(!stream_response.is_final);
    }

    #[tokio::test]
    async fn test_stream_response_single() {
        let stream_response = StreamResponse::single("test data");

        assert!(!stream_response.is_final);

        let mut items = Vec::new();
        let mut stream = stream_response.stream;

        loop {
            match tokio::time::timeout(
                std::time::Duration::from_secs(1),
                tokio_stream::StreamExt::next(&mut stream),
            )
            .await
            {
                Ok(Some(result)) => {
                    items.push(result.unwrap());
                }
                Ok(None) => break,
                Err(_) => break, // Timeout
            }
        }

        assert_eq!(items.len(), 1);
        assert_eq!(items[0], "test data");
    }

    #[tokio::test]
    async fn test_stream_response_final_marker() {
        let stream_response: StreamResponse<()> = StreamResponse::final_marker();
        assert!(stream_response.is_final);
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
            _ => panic!("Expected Data event"),
        }
    }

    #[tokio::test]
    async fn test_stream_event_ping() {
        let event: StreamEvent<()> = StreamEvent::ping();
        match event {
            StreamEvent::Ping { timestamp } => {
                assert!(timestamp > 0);
            }
            _ => panic!("Expected Ping event"),
        }
    }

    #[tokio::test]
    async fn test_stream_event_error() {
        let event: StreamEvent<()> = StreamEvent::error("test error".to_string());
        match event {
            StreamEvent::Error { message } => {
                assert_eq!(message, "test error");
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[tokio::test]
    async fn test_stream_event_complete() {
        let event: StreamEvent<()> = StreamEvent::complete();
        match event {
            StreamEvent::Complete => {}
            _ => panic!("Expected Complete event"),
        }
    }

    #[tokio::test]
    async fn test_stream_to_sse() {
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            for i in 1..=3 {
                let _ = tx.send(Ok(format!("Item {}", i))).await;
            }
        });

        let sse_stream =
            axiom::streaming::stream_to_sse(ReceiverStream::new(rx), |item| match item {
                Ok(data) => {
                    StreamEvent::data(serde_json::to_value(data).unwrap_or(serde_json::Value::Null))
                }
                Err(err) => StreamEvent::error(err),
            });

        let mut items = Vec::new();
        let mut stream = sse_stream;

        loop {
            match tokio::time::timeout(
                std::time::Duration::from_secs(1),
                tokio_stream::StreamExt::next(&mut stream),
            )
            .await
            {
                Ok(Some(result)) => {
                    items.push(result.unwrap());
                }
                Ok(None) => break,
                Err(_) => break, // Timeout
            }
        }

        assert!(items.len() >= 3);
        assert!(items[0].starts_with("data:"));
    }

    #[tokio::test]
    async fn test_create_stream_channel() {
        let (tx, rx) = axiom::streaming::create_stream_channel::<String>(10);

        tokio::spawn(async move {
            for i in 0..3 {
                let _ = tx.send(Ok(format!("Item {}", i))).await;
            }
        });

        let mut items = Vec::new();
        let mut stream = rx.stream;

        loop {
            match tokio::time::timeout(
                std::time::Duration::from_secs(1),
                tokio_stream::StreamExt::next(&mut stream),
            )
            .await
            {
                Ok(Some(result)) => {
                    items.push(result.unwrap());
                }
                Ok(None) => break,
                Err(_) => break, // Timeout
            }
        }

        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "Item 0");
        assert_eq!(items[1], "Item 1");
        assert_eq!(items[2], "Item 2");
    }

    #[tokio::test]
    async fn test_stream_with_complex_type() {
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            let entries = vec![
                LogEntry {
                    timestamp: 1704067200,
                    level: "INFO".to_string(),
                    message: "Test message 1".to_string(),
                },
                LogEntry {
                    timestamp: 1704067201,
                    level: "DEBUG".to_string(),
                    message: "Test message 2".to_string(),
                },
            ];

            for entry in entries {
                let _ = tx.send(Ok(entry)).await;
            }
        });

        let stream_response: StreamResponse<LogEntry> =
            StreamResponse::new(ReceiverStream::new(rx));
        let mut items = Vec::new();
        let mut stream = stream_response.stream;

        loop {
            match tokio::time::timeout(
                std::time::Duration::from_secs(1),
                tokio_stream::StreamExt::next(&mut stream),
            )
            .await
            {
                Ok(Some(result)) => {
                    items.push(result.unwrap());
                }
                Ok(None) => break,
                Err(_) => break, // Timeout
            }
        }

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].level, "INFO");
        assert_eq!(items[1].level, "DEBUG");
    }

    #[tokio::test]
    async fn test_stream_with_errors() {
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            for i in 1..=6 {
                if i % 3 == 0 {
                    let _ = tx.send(Err(format!("Error at item {}", i))).await;
                } else {
                    let _ = tx.send(Ok(format!("Item {}", i))).await;
                }
            }
        });

        let stream_response = StreamResponse::new(ReceiverStream::new(rx));
        let mut items = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_response.stream;

        loop {
            match tokio::time::timeout(
                std::time::Duration::from_secs(1),
                tokio_stream::StreamExt::next(&mut stream),
            )
            .await
            {
                Ok(Some(result)) => match result {
                    Ok(data) => items.push(data),
                    Err(err) => errors.push(err),
                },
                Ok(None) => break,
                Err(_) => break, // Timeout
            }
        }

        assert_eq!(items.len(), 4); // Items 1, 2, 4, 5
        assert_eq!(errors.len(), 2); // Errors at 3, 6
        assert_eq!(errors[0], "Error at item 3");
        assert_eq!(errors[1], "Error at item 6");
    }

    #[tokio::test]
    async fn test_stream_empty() {
        let (tx, rx) = mpsc::channel(32);
        drop(tx); // Don't send any data

        let stream_response: StreamResponse<String> = StreamResponse::new(ReceiverStream::new(rx));
        let mut items = Vec::new();
        let mut stream = stream_response.stream;

        loop {
            match tokio::time::timeout(
                std::time::Duration::from_secs(1),
                tokio_stream::StreamExt::next(&mut stream),
            )
            .await
            {
                Ok(Some(result)) => {
                    items.push(result.unwrap());
                }
                Ok(None) => break,
                Err(_) => break, // Timeout
            }
        }

        assert_eq!(items.len(), 0);
    }
}
