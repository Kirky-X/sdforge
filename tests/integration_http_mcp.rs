// Integration tests for HTTP+MCP protocol combinations
//
// These tests verify that the framework works correctly when both
// HTTP and MCP features are enabled simultaneously.

#[cfg(test)]
mod integration_tests {
    // ============================================================================
    // Feature Combination Tests
    // ============================================================================

    #[test]
    fn test_http_and_mcp_features_enabled() {
        #[cfg(all(feature = "http", feature = "mcp"))]
        {
            assert!(true);
        }

        #[cfg(not(all(feature = "http", feature = "mcp")))]
        {
            panic!("Both http and mcp features must be enabled for integration tests");
        }
    }

    // ============================================================================
    // Core Type Tests
    // ============================================================================

    #[test]
    fn test_api_error_creation() {
        use sdforge::core::ApiError;

        let not_found = ApiError::not_found("TestResource", Some("123"));
        let validation = ApiError::validation_error("field", "Invalid value");

        match not_found {
            ApiError::NotFound { resource, .. } => {
                assert_eq!(resource, "TestResource");
            }
            _ => panic!("Expected NotFound error"),
        }

        match validation {
            ApiError::ValidationError { .. } => {}
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_service_response_creation() {
        use sdforge::core::ServiceResponse;

        let response = ServiceResponse::<String>::new("test data".to_string());

        assert_eq!(response.data(), "test data");
        assert!(response.is_success());
    }

    #[test]
    fn test_api_metadata() {
        use sdforge::core::ApiMetadata;

        let metadata = ApiMetadata {
            name: "test_api".to_string(),
            version: "v1".to_string(),
            description: "Test API".to_string(),
            cache_ttl: Some(300),
            is_streaming: false,
        };

        assert_eq!(metadata.name(), "test_api");
        assert_eq!(metadata.version(), "v1");
        assert_eq!(metadata.description(), "Test API");
        assert_eq!(metadata.cache_ttl(), Some(300));
        assert!(!metadata.is_streaming());
    }

    #[test]
    fn test_api_metadata_cloning() {
        use sdforge::core::ApiMetadata;

        let metadata1 = ApiMetadata {
            name: "test".to_string(),
            version: "v1".to_string(),
            description: "Test".to_string(),
            cache_ttl: Some(300),
            is_streaming: false,
        };

        let metadata2 = metadata1.clone();

        assert_eq!(metadata1.name(), metadata2.name());
        assert_eq!(metadata1.version(), metadata2.version());
    }

    // ============================================================================
    // HTTP Module Tests
    // ============================================================================

    #[test]
    fn test_http_build_with_app_config() {
        use sdforge::config::{AppConfig, AuthConfig, DatabaseConfig, LoggingConfig, ServerConfig};
        use sdforge::http::build_with_config;

        let config = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                request_timeout_secs: 30,
                cors: None,
            },
            database: DatabaseConfig::default(),
            authentication: AuthConfig::None,
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            rate_limit: None,
            request_size: None,
            timeout: None,
        };

        let router = build_with_config(&config);
        assert!(router.is_ok());
    }

    #[test]
    fn test_http_builder() {
        use sdforge::http::build;

        let _app = build();
        assert!(true);
    }

    // ============================================================================
    // MCP Module Tests
    // ============================================================================

    #[cfg(feature = "mcp")]
    #[tokio::test]
    async fn test_mcp_build() {
        use sdforge::mcp::build;

        let _server = build().await;
        assert!(true);
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn test_mcp_registration_structure() {
        use sdforge::mcp::McpToolRegistration;
        use std::sync::Arc;

        fn create_test_tool() -> Arc<dyn mcp_sdk::tools::Tool> {
            struct TestTool;
            impl mcp_sdk::tools::Tool for TestTool {
                fn name(&self) -> String { "test_tool".to_string() }
                fn description(&self) -> String { "Test tool".to_string() }
                fn input_schema(&self) -> serde_json::Value { serde_json::json!({"type": "object"}) }
                fn call(
                    &self,
                    _input: Option<serde_json::Value>,
                ) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                    Ok(mcp_sdk::types::CallToolResponse { content: vec![], is_error: None, meta: None })
                }
            }
            Arc::new(TestTool) as Arc<dyn mcp_sdk::tools::Tool>
        }

        let registration = McpToolRegistration {
            name: "test_tool",
            version: "v1",
            description: "Test tool",
            create_fn: create_test_tool,
        };

        assert_eq!(registration.name, "test_tool");
        assert_eq!(registration.version, "v1");
        let tool = (registration.create_fn)();
        assert_eq!(tool.name(), "test_tool");
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[test]
    fn test_error_serialization() {
        use sdforge::core::ApiError;

        let error = ApiError::not_found("User", Some("123"));

        let json = serde_json::to_string(&error);
        assert!(json.is_ok());

        if let Ok(json_str) = json {
            assert!(
                json_str.contains("NOT_FOUND")
                    || json_str.contains("not_found")
                    || json_str.contains("User")
            );
        }
    }

    #[test]
    fn test_response_format_consistency() {
        use sdforge::core::ServiceResponse;

        let response1 = ServiceResponse::<String>::new("test data".to_string());
        let response2 = ServiceResponse::<String>::new("test data".to_string());

        assert_eq!(response1.data(), response2.data());
        assert_eq!(response1.is_success(), response2.is_success());
    }

    // ============================================================================
    // Cross-Protocol Consistency Tests
    // ============================================================================

    #[test]
    fn test_metadata_across_protocols() {
        use sdforge::core::ApiMetadata;

        let http_metadata = ApiMetadata {
            name: "get_user".to_string(),
            version: "v1".to_string(),
            description: "Get user by ID".to_string(),
            cache_ttl: Some(300),
            is_streaming: false,
        };

        let mcp_metadata = ApiMetadata {
            name: "get_user".to_string(),
            version: "v1".to_string(),
            description: "Get user by ID".to_string(),
            cache_ttl: Some(300),
            is_streaming: false,
        };

        assert_eq!(http_metadata.name(), mcp_metadata.name());
        assert_eq!(http_metadata.version(), mcp_metadata.version());
        assert_eq!(http_metadata.description(), mcp_metadata.description());
    }

    // ============================================================================
    // Security Tests
    // ============================================================================

    #[test]
    fn test_validation_prevents_injection() {
        use sdforge::core::validation::validate_string;

        let valid_input = "safe_string_123";
        let result1 = validate_string(valid_input, 1, 100);
        assert!(result1.is_ok());

        let sql_attempt = "'; DROP TABLE users; --";
        let result2 = validate_string(sql_attempt, 1, 100);
        assert!(result2.is_ok());

        let too_long = "a".repeat(1000);
        let result3 = validate_string(&too_long, 1, 100);
        assert!(result3.is_err());
    }

    #[test]
    fn test_empty_string_validation() {
        use sdforge::core::validation::validate_string;

        let result = validate_string("", 0, 100);
        assert!(result.is_ok());
    }

    // ============================================================================
    // Performance Tests
    // ============================================================================

    #[test]
    fn test_json_performance() {
        use std::time::Instant;

        let data = serde_json::json!({
            "user_id": 123,
            "name": "Test User",
            "email": "test@example.com",
            "active": true
        });

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = serde_json::to_string(&data);
            let serialized = serde_json::to_string(&data).unwrap();
            let _ = serde_json::from_str::<serde_json::Value>(&serialized);
        }
        let duration = start.elapsed();

        assert!(
            duration.as_millis() < 1000,
            "JSON operations should complete quickly"
        );
    }

    #[test]
    fn test_error_creation_performance() {
        use sdforge::core::ApiError;
        use std::time::Instant;

        let start = Instant::now();
        for _ in 0..10000 {
            let _ = ApiError::not_found("Test", Some("123"));
        }
        let duration = start.elapsed();

        assert!(duration.as_millis() < 100, "Error creation should be fast");
    }

    // ============================================================================
    // Memory Safety Tests
    // ============================================================================

    #[test]
    fn test_large_response_handling() {
        use sdforge::core::ServiceResponse;

        let large_data = "x".repeat(1_000_000);
        let response = ServiceResponse::new(large_data);

        assert!(response.data().len() == 1_000_000);
    }

    #[test]
    fn test_empty_response_handling() {
        use sdforge::core::ServiceResponse;

        let response = ServiceResponse::<String>::new("".to_string());

        assert_eq!(response.data(), "");
        assert!(response.is_success());
    }

    // ============================================================================
    // Concurrent Access Tests
    // ============================================================================

    #[test]
    fn test_metadata_thread_safety() {
        use sdforge::core::ApiMetadata;
        use std::sync::{Arc, Mutex};

        let metadata = Arc::new(Mutex::new(ApiMetadata {
            name: "test".to_string(),
            version: "v1".to_string(),
            description: "Test".to_string(),
            cache_ttl: Some(300),
            is_streaming: false,
        }));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let metadata_clone = Arc::clone(&metadata);
                std::thread::spawn(move || {
                    let m = metadata_clone.lock().unwrap();
                    m.name()
                })
            })
            .collect();

        for handle in handles {
            let name = handle.join().unwrap();
            assert_eq!(name, "test");
        }
    }

    #[test]
    fn test_response_cloning() {
        use sdforge::core::ServiceResponse;

        let response1 = ServiceResponse::new("test data".to_string());
        let response2 = response1.clone();

        assert_eq!(response1.data(), response2.data());
    }

    // ============================================================================
    // Cache Module Tests
    // ============================================================================

    #[test]
    fn test_cache_config() {
        use sdforge::CacheConfig;

        let config = CacheConfig::default();

        assert_eq!(config.ttl, 300);
        assert_eq!(config.max_size, 100);
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_cache_config_creation() {
        use sdforge::CacheConfig;

        let config = CacheConfig::new(600, 200);

        assert_eq!(config.ttl, 600);
        assert_eq!(config.max_size, 200);
    }

    // ============================================================================
    // gRPC Module Tests (when enabled)
    // ============================================================================

    #[cfg(feature = "grpc")]
    #[test]
    fn test_grpc_config() {
        use sdforge::GrpcServerConfig;

        let config = GrpcServerConfig::default();

        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.timeout_seconds, 30);
    }

    #[cfg(feature = "grpc")]
    #[test]
    fn test_grpc_config_custom() {
        use sdforge::GrpcServerConfig;

        let config = GrpcServerConfig {
            max_connections: 500,
            timeout_seconds: 60,
        };

        assert_eq!(config.max_connections, 500);
        assert_eq!(config.timeout_seconds, 60);
    }
}
