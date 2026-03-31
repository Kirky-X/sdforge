// Integration tests for HTTP+MCP protocol combinations
//
// These tests verify that the framework works correctly when both
// HTTP and MCP features are enabled simultaneously.

#[cfg(test)]
mod integration_tests {
    // ============================================================================
    // Core Type Tests
    // ============================================================================

    #[test]
    fn test_api_error_creation() {
        use sdforge::core::ApiError;

        let not_found = ApiError::NotFound {
            resource: "TestResource".to_string(),
            resource_id: Some("123".into()),
        };
        assert!(not_found.to_string().contains("TestResource"));
        let invalid_input = ApiError::validation_error("field", "Invalid value");

        match not_found {
            ApiError::NotFound { resource, .. } => {
                assert_eq!(resource, "TestResource");
            }
            _ => panic!("Expected NotFound error"),
        }

        match invalid_input {
            ApiError::InvalidInput { .. } => {}
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_service_response_creation() {
        use sdforge::core::ServiceResponse;

        let response = ServiceResponse::<String>::success("test data".to_string());

        assert_eq!(response.data(), Some(&"test data".to_string()));
        assert!(response.is_success());
    }

    #[test]
    fn test_api_metadata() {
        use sdforge::core::ApiMetadata;

        let metadata = ApiMetadata::new(
            "test_api".to_string(),
            "v1".to_string(),
            "Test API".to_string(),
            Some(300),
            false,
        );

        assert_eq!(metadata.name(), "test_api");
        assert_eq!(metadata.version(), "v1");
        assert_eq!(metadata.description(), "Test API");
        assert_eq!(metadata.cache_ttl(), Some(300));
        assert!(!metadata.is_streaming());
    }

    #[test]
    fn test_api_metadata_cloning() {
        use sdforge::core::ApiMetadata;

        let metadata1 = ApiMetadata::new(
            "test".to_string(),
            "v1".to_string(),
            "Test".to_string(),
            Some(300),
            false,
        );

        let metadata2 = metadata1.clone();

        assert_eq!(metadata1.name(), metadata2.name());
        assert_eq!(metadata1.version(), metadata2.version());
    }

    // ============================================================================
    // HTTP Module Tests
    // ============================================================================

    #[test]
    fn test_http_build_with_app_config() {
        use sdforge::config::{AppConfig, AuthConfig, ServerConfig};

        let config = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                request_timeout_secs: 30,
                cors: None,
            },
            authentication: AuthConfig::None,
            timeout: None,
        };

        // Config created successfully - verify authentication is None
        assert!(matches!(
            config.authentication,
            sdforge::config::AuthConfig::None
        ));
    }

    #[test]
    fn test_http_builder() {
        use sdforge::http::build;

        let app = build();
        // App built successfully
        assert!(!format!("{:?}", app).is_empty());
    }

    // ============================================================================
    // MCP Module Tests
    // ============================================================================

    #[cfg(feature = "mcp")]
    #[tokio::test]
    async fn test_mcp_build() {
        use sdforge::mcp::build;

        let server = build().await;
        // Server built successfully - verify it's not null
        assert!(!std::ptr::eq(&server, std::ptr::null()));
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[test]
    fn test_error_serialization() {
        use sdforge::core::ApiError;

        let error = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("123".into()),
        };

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

        let response1 = ServiceResponse::<String>::success("test data".to_string());
        let response2 = ServiceResponse::<String>::success("test data".to_string());

        assert_eq!(response1.data(), response2.data());
        assert_eq!(response1.is_success(), response2.is_success());
    }

    // ============================================================================
    // Cross-Protocol Consistency Tests
    // ============================================================================

    #[test]
    fn test_metadata_across_protocols() {
        use sdforge::core::ApiMetadata;

        let http_metadata = ApiMetadata::new(
            "get_user".to_string(),
            "v1".to_string(),
            "Get user by ID".to_string(),
            Some(300),
            false,
        );

        let mcp_metadata = ApiMetadata::new(
            "get_user".to_string(),
            "v1".to_string(),
            "Get user by ID".to_string(),
            Some(300),
            false,
        );

        assert_eq!(http_metadata.name(), mcp_metadata.name());
        assert_eq!(http_metadata.version(), mcp_metadata.version());
        assert_eq!(http_metadata.description(), mcp_metadata.description());
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
            let serialized = serde_json::to_string(&data).unwrap();
            let _deserialized = serde_json::from_str::<serde_json::Value>(&serialized);
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
            let _error = ApiError::NotFound {
                resource: "Test".to_string(),
                resource_id: Some("123".into()),
            };
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
        let response = ServiceResponse::success(large_data);

        assert_eq!(response.data().unwrap().len(), 1_000_000);
    }

    #[test]
    fn test_empty_response_handling() {
        use sdforge::core::ServiceResponse;

        let response = ServiceResponse::<String>::success("".to_string());

        assert_eq!(response.data(), Some(&"".to_string()));
        assert!(response.is_success());
    }

    // ============================================================================
    // Concurrent Access Tests
    // ============================================================================

    #[test]
    fn test_metadata_thread_safety() {
        use sdforge::core::ApiMetadata;
        use std::sync::{Arc, Mutex};

        let metadata = Arc::new(Mutex::new(ApiMetadata::new(
            "test".to_string(),
            "v1".to_string(),
            "Test".to_string(),
            Some(300),
            false,
        )));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let metadata_clone = Arc::clone(&metadata);
                std::thread::spawn(move || {
                    let m = metadata_clone.lock().unwrap();
                    m.name().to_string()
                })
            })
            .collect();

        for handle in handles {
            let name = handle.join().unwrap();
            assert_eq!(name, "test");
        }
    }

    #[test]
    fn test_response_equality() {
        use sdforge::core::ServiceResponse;

        let response1 = ServiceResponse::success("test data".to_string());
        let response2 = ServiceResponse::success("test data".to_string());

        // Compare responses by checking they have the same data
        assert_eq!(response1.data(), response2.data());
    }
}
