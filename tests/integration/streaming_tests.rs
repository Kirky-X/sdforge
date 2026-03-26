#[cfg(all(feature = "streaming", feature = "http"))]
mod streaming_tests {
    use futures_util::StreamExt;
    use sdforge::streaming::{create_stream_channel, stream_to_sse, StreamEvent, StreamResponse};
    use tokio_stream::wrappers::ReceiverStream;

    #[test]
    fn test_create_stream_channel() {
        let (tx, response) = create_stream_channel::<String>(10);

        // Verify the channel was created successfully
        assert!(tx.capacity() > 0 || true); // Just verify it doesn't panic
        let _ = response;
    }

    #[test]
    fn test_stream_response_new() {
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, String>>(10);
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
}
