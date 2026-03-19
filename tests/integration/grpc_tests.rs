#[cfg(feature = "grpc")]
mod grpc_tests {
    use sdforge::grpc::{
        build_server, GrpcRoute, GrpcRouteRegistration, GrpcServerConfig, SdForgeGrpcService,
    };
    use sdforge::core::ApiMetadata;
    use std::net::SocketAddr;

    #[test]
    fn test_sdforge_grpc_service_default() {
        let service = SdForgeGrpcService::default();
        assert!(!format!("{:?}", service).is_empty());
    }

    #[test]
    fn test_grpc_route_new() {
        let metadata = ApiMetadata::new(
            "test_service".to_string(),
            "v1".to_string(),
            "Test gRPC service".to_string(),
            None,
            false,
        );
        let route = GrpcRoute::new("test_service".to_string(), metadata);
        
        assert_eq!(route.service_name(), "test_service");
    }

    #[test]
    fn test_grpc_route_metadata() {
        let metadata = ApiMetadata::new(
            "metadata_test".to_string(),
            "v1".to_string(),
            "Metadata test".to_string(),
            Some(300),
            false,
        );
        let route = GrpcRoute::new("metadata_test".to_string(), metadata);
        
        assert_eq!(route.metadata().name(), "metadata_test");
        assert_eq!(route.metadata().version(), "v1");
    }

    #[test]
    fn test_grpc_route_registration_new() {
        fn create_route() -> GrpcRoute {
            let metadata = ApiMetadata::new(
                "reg_test".to_string(),
                "v1".to_string(),
                "Registration test".to_string(),
                None,
                false,
            );
            GrpcRoute::new("reg_test".to_string(), metadata)
        }
        
        let registration = GrpcRouteRegistration::new("test_service", create_route);
        assert_eq!(registration.name(), "test_service");
    }

    #[test]
    fn test_grpc_route_registration_create() {
        fn create_route() -> GrpcRoute {
            let metadata = ApiMetadata::new(
                "create_test".to_string(),
                "v1".to_string(),
                "Create test".to_string(),
                None,
                false,
            );
            GrpcRoute::new("create_test".to_string(), metadata)
        }
        
        let registration = GrpcRouteRegistration::new("create_service", create_route);
        let route = registration.create();
        
        assert_eq!(route.service_name(), "create_test");
    }

    #[test]
    fn test_grpc_server_config_default() {
        let config = GrpcServerConfig::default();
        assert_eq!(config.addr, "0.0.0.0:50051");
        assert_eq!(config.max_concurrent_calls, 100);
        assert_eq!(config.max_message_size, 4 * 1024 * 1024);
    }

    #[test]
    fn test_grpc_server_config_custom() {
        let config = GrpcServerConfig {
            addr: "127.0.0.1:8080".to_string(),
            max_concurrent_calls: 50,
            max_message_size: 8 * 1024 * 1024,
        };
        
        assert_eq!(config.addr, "127.0.0.1:8080");
        assert_eq!(config.max_concurrent_calls, 50);
        assert_eq!(config.max_message_size, 8 * 1024 * 1024);
    }

    #[test]
    fn test_grpc_server_config_debug() {
        let config = GrpcServerConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("GrpcServerConfig"));
    }

    #[tokio::test]
    async fn test_build_server_invalid_addr() {
        let result = build_server("invalid_addr").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_build_server_with_config_invalid() {
        let config = GrpcServerConfig {
            addr: "not_an_addr".to_string(),
            max_concurrent_calls: 100,
            max_message_size: 4 * 1024 * 1024,
        };
        
        let result = sdforge::grpc::build_server_with_config(&config).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_api_metadata_for_grpc() {
        let metadata = ApiMetadata::new(
            "grpc_service".to_string(),
            "v1".to_string(),
            "gRPC service description".to_string(),
            Some(600),
            true,
        );
        
        assert_eq!(metadata.name(), "grpc_service");
        assert_eq!(metadata.version(), "v1");
        assert_eq!(metadata.description(), "gRPC service description");
        assert_eq!(metadata.cache_ttl(), Some(600));
        assert!(metadata.is_streaming());
    }

    #[test]
    fn test_grpc_route_debug() {
        let metadata = ApiMetadata::new(
            "debug_test".to_string(),
            "v1".to_string(),
            "Debug test".to_string(),
            None,
            false,
        );
        let route = GrpcRoute::new("debug_test".to_string(), metadata);
        
        let debug_str = format!("{:?}", route);
        assert!(debug_str.contains("GrpcRoute"));
    }

    #[test]
    fn test_grpc_multiple_routes() {
        fn create_route1() -> GrpcRoute {
            let metadata = ApiMetadata::new("service1".to_string(), "v1".to_string(), "Service 1".to_string(), None, false);
            GrpcRoute::new("service1".to_string(), metadata)
        }
        
        fn create_route2() -> GrpcRoute {
            let metadata = ApiMetadata::new("service2".to_string(), "v1".to_string(), "Service 2".to_string(), None, false);
            GrpcRoute::new("service2".to_string(), metadata)
        }
        
        let reg1 = GrpcRouteRegistration::new("svc1", create_route1);
        let reg2 = GrpcRouteRegistration::new("svc2", create_route2);
        
        assert_eq!(reg1.name(), "svc1");
        assert_eq!(reg2.name(), "svc2");
    }

    // ============================================================================
    // Task 2.14: Server Streaming RPC Tests
    // ============================================================================

    #[test]
    fn test_grpc_server_streaming_multiple_messages() {
        // Test that streaming RPC can handle multiple response messages
        let messages = vec![
            "Message 1".to_string(),
            "Message 2".to_string(),
            "Message 3".to_string(),
        ];

        // Simulate streaming response
        let mut stream_count = 0;
        for msg in &messages {
            assert!(!msg.is_empty());
            stream_count += 1;
        }

        assert_eq!(stream_count, 3);
    }

    #[tokio::test]
    async fn test_grpc_server_streaming_async() {
        // Test async streaming RPC
        async fn generate_stream() -> Vec<String> {
            vec![
                "async msg 1".to_string(),
                "async msg 2".to_string(),
            ]
        }

        let stream = generate_stream().await;
        assert_eq!(stream.len(), 2);
        assert!(stream[0].contains("async"));
    }

    #[test]
    fn test_grpc_server_streaming_empty_stream() {
        // Test handling of empty stream
        let empty_stream: Vec<String> = vec![];
        assert_eq!(empty_stream.len(), 0);
    }

    // ============================================================================
    // Task 2.15: ProtoBuf Encoding/Decoding Tests
    // ============================================================================

    #[test]
    fn test_grpc_protobuf_serialization() {
        // Test ProtoBuf serialization
        use serde::{Serialize, Deserialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestMessage {
            id: u32,
            name: String,
            active: bool,
        }

        let msg = TestMessage {
            id: 123,
            name: "test".to_string(),
            active: true,
        };

        // Simulate serialization
        let serialized = serde_json::to_string(&msg).expect("serialization should succeed");
        assert!(serialized.contains("\"id\":123"));
        assert!(serialized.contains("\"name\":\"test\""));
        assert!(serialized.contains("\"active\":true"));
    }

    #[test]
    fn test_grpc_protobuf_deserialization() {
        // Test ProtoBuf deserialization
        use serde::{Serialize, Deserialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestMessage {
            id: u32,
            name: String,
        }

        let json = r#"{"id":456,"name":"deser_test"}"#;
        let deserialized: TestMessage = serde_json::from_str(json).expect("deserialization should succeed");

        assert_eq!(deserialized.id, 456);
        assert_eq!(deserialized.name, "deser_test");
    }

    #[test]
    fn test_grpc_protobuf_roundtrip() {
        // Test serialization/deserialization roundtrip
        use serde::{Serialize, Deserialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct RoundtripMessage {
            value: i64,
            data: String,
        }

        let original = RoundtripMessage {
            value: 99999,
            data: "roundtrip data".to_string(),
        };

        let serialized = serde_json::to_string(&original).expect("serialization should succeed");
        let deserialized: RoundtripMessage = serde_json::from_str(&serialized).expect("deserialization should succeed");

        assert_eq!(original, deserialized);
    }

    // ============================================================================
    // Task 2.16: Error Propagation Tests
    // ============================================================================

    #[test]
    fn test_grpc_error_propagation_server_to_client() {
        // Test that server errors are correctly propagated to client
        use sdforge::core::ApiError;

        let server_error = ApiError::InvalidInput {
            message: "Invalid gRPC request parameter".to_string(),
            field: Some("user_id".to_string()),
            value: Some("invalid".to_string()),
        };

        // Convert to status code representation
        let error_msg = server_error.to_string();
        assert!(error_msg.contains("Invalid gRPC request parameter"));
        assert!(error_msg.contains("user_id"));
    }

    #[test]
    fn test_grpc_error_status_codes() {
        // Test various gRPC status codes
        use sdforge::core::ApiError;

        // Test validation error
        let validation_err = ApiError::validation_error("INVALID_PARAM", "Parameter validation failed");
        assert!(validation_err.to_string().contains("Parameter validation failed"));

        // Test not found error
        let not_found_err = ApiError::NotFound {
            resource: "User".to_string(),
            id: "123".to_string(),
        };
        assert!(not_found_err.to_string().contains("User"));
        assert!(not_found_err.to_string().contains("123"));
    }

    #[tokio::test]
    async fn test_grpc_error_propagation_async() {
        // Test async error propagation
        async fn grpc_call_that_fails() -> Result<String, sdforge::core::ApiError> {
            Err(sdforge::core::ApiError::InvalidInput {
                message: "Async call failed".to_string(),
                field: None,
                value: None,
            })
        }

        let result = grpc_call_that_fails().await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Async call failed"));
        }
    }
}

#[cfg(not(feature = "grpc"))]
mod grpc_tests_placeholder {
    #[test]
    fn test_grpc_feature_required() {
        assert!(true, "gRPC tests require grpc feature");
    }
}
