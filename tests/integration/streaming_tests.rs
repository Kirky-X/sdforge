#[cfg(all(feature = "streaming", feature = "http"))]
mod streaming_tests {
    use sdforge::streaming::{
        create_stream_channel, stream_to_sse, StreamEvent, StreamResponse,
    };
    use futures_util::StreamExt;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::ReceiverStream;

    #[test]
    fn test_create_stream_channel() {
        let (tx, response) = create_stream_channel::<String>(10);
        
        assert!(tx.capacity() > 0);
        assert!(!tx.is_closed());
    }

    #[test]
    fn test_stream_response_new() {
        let (tx, rx) = mpsc::channel::<Result<String, String>>(10);
        let stream = ReceiverStream::new(rx);
        let response = StreamResponse::new(stream);
        
        assert!(!response.is_final());
    }

    #[test]
    fn test_stream_response_single() {
        let response = StreamResponse::<String>::single("test data".to_string());
        assert!(!response.is_final());
    }

    #[test]
    fn test_stream_response_final_marker() {
        let response = StreamResponse::<String>::final_marker();
        assert!(response.is_final());
    }

    #[tokio::test]
    async fn test_stream_to_sse_basic() {
        let (tx, rx) = mpsc::channel::<Result<String, String>>(10);
        
        let sse_stream = stream_to_sse(rx);
        
        tx.send(Ok("first message".to_string())).await.unwrap();
        tx.send(Ok("second message".to_string())).await.unwrap();
        drop(tx);
        
        let mut stream = Box::pin(sse_stream);
        let first = stream.next().await;
        assert!(first.is_some());
    }

    #[test]
    fn test_stream_event_data() {
        let event = StreamEvent::<String>::Data {
            id: Some("1".to_string()),
            event_name: Some("message".to_string()),
            data: "test data".to_string(),
        };
        
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("test data"));
    }

    #[test]
    fn test_stream_event_ping() {
        let event = StreamEvent::<String>::Ping {
            timestamp: 1234567890,
        };
        
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("ping"));
        assert!(serialized.contains("1234567890"));
    }

    #[test]
    fn test_stream_event_error() {
        let event = StreamEvent::<String>::Error {
            message: "Something went wrong".to_string(),
        };
        
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("error"));
        assert!(serialized.contains("Something went wrong"));
    }

    #[test]
    fn test_stream_event_complete() {
        let event = StreamEvent::<String>::Complete {
            message: Some("Stream completed".to_string()),
        };
        
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("complete"));
    }

    #[test]
    fn test_stream_event_default_id() {
        let event = StreamEvent::<String>::Data {
            id: None,
            event_name: None,
            data: "no id event".to_string(),
        };
        
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("no id event"));
    }

    #[tokio::test]
    async fn test_stream_channel_send_and_receive() {
        let (tx, rx) = create_stream_channel::<i32>(5);
        
        tx.send(Ok(1)).await.unwrap();
        tx.send(Ok(2)).await.unwrap();
        tx.send(Ok(3)).await.unwrap();
        drop(tx);
        
        let mut stream = Box::pin(ReceiverStream::new(rx));
        
        assert_eq!(stream.next().await.unwrap().unwrap(), 1);
        assert_eq!(stream.next().await.unwrap().unwrap(), 2);
        assert_eq!(stream.next().await.unwrap().unwrap(), 3);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_stream_channel_error_handling() {
        let (tx, rx) = create_stream_channel::<String>(5);
        
        tx.send(Ok("success".to_string())).await.unwrap();
        tx.send(Err("error occurred".to_string())).await.unwrap();
        drop(tx);
        
        let mut stream = Box::pin(ReceiverStream::new(rx));
        
        assert!(stream.next().await.unwrap().is_ok());
        assert!(stream.next().await.unwrap().is_err());
    }

    #[test]
    fn test_stream_response_into_stream() {
        let (tx, rx) = mpsc::channel::<Result<String, String>>(10);
        let stream = ReceiverStream::new(rx);
        let response = StreamResponse::new(stream);
        
        let _ = response.into_stream();
    }

    #[test]
    fn test_multiple_stream_events_serialization() {
        let events = vec![
            StreamEvent::<String>::Data {
                id: Some("1".to_string()),
                event_name: None,
                data: "first".to_string(),
            },
            StreamEvent::<String>::Data {
                id: Some("2".to_string()),
                event_name: None,
                data: "second".to_string(),
            },
            StreamEvent::<String>::Ping { timestamp: 100 },
            StreamEvent::<String>::Complete { message: None },
        ];
        
        for event in events {
            let serialized = serde_json::to_string(&event);
            assert!(serialized.is_ok());
        }
    }
}

#[cfg(not(all(feature = "streaming", feature = "http")))]
mod streaming_tests_placeholder {
    #[test]
    fn test_streaming_feature_required() {
        assert!(true, "Streaming tests require both streaming and http features");
    }
}
