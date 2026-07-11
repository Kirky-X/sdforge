// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// Unit tests for SDForge framework
// Tests core functionality and module integration

#[cfg(test)]
mod core_tests {
    use sdforge::core::{ApiError, ApiMetadata, ServiceResponse};

    #[test]
    fn test_api_error_not_found() {
        let error = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("123".to_string()),
        };
        match error {
            ApiError::NotFound {
                resource,
                resource_id,
            } => {
                assert_eq!(resource, "User");
                assert_eq!(resource_id, Some("123".to_string()));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_api_error_validation() {
        let error = ApiError::validation_error("field_name", "Invalid value");
        match error {
            ApiError::InvalidInput { message, .. } => {
                assert_eq!(message, "Invalid value");
            }
            _ => panic!("Expected ValidationError (InvalidInput)"),
        }
    }

    #[test]
    fn test_api_error_invalid_input() {
        let error = ApiError::InvalidInput {
            message: "Query cannot be empty".to_string(),
            field: Some("query".to_string()),
            value: None,
        };
        match error {
            ApiError::InvalidInput { message, field, .. } => {
                assert_eq!(message, "Query cannot be empty");
                assert_eq!(field, Some("query".to_string()));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_service_response_success() {
        let response = ServiceResponse::success("test data".to_string());
        assert_eq!(response.data(), Some(&"test data".to_string()));
        assert!(response.is_success());
    }

    #[test]
    fn test_service_response_error() {
        let service_error = sdforge::core::response::ServiceError::new(
            "NOT_FOUND",
            "Resource not found: User",
            404,
        );
        let response = ServiceResponse::<String>::error(service_error);
        assert!(!response.is_success());
    }

    #[test]
    fn test_api_metadata() {
        let metadata = ApiMetadata::new(
            "get_user".to_string(),
            "v1".to_string(),
            "Get user by ID".to_string(),
            Some(300),
            false,
        );

        assert_eq!(metadata.name(), "get_user");
        assert_eq!(metadata.version(), "v1");
        assert_eq!(metadata.description(), "Get user by ID");
        assert_eq!(metadata.cache_ttl(), Some(300));
    }

    #[test]
    fn test_api_metadata_default() {
        let metadata = ApiMetadata::default();
        assert_eq!(metadata.name(), "");
        assert_eq!(metadata.version(), "");
    }

    // ============================================================================
    // Advanced Error Handling Tests
    // ============================================================================

    /// Test: Error context with extra information
    #[test]
    fn test_error_context_with_extra() {
        use sdforge::error::ErrorContext;

        let ctx = ErrorContext::new()
            .with_extra("field".to_string(), "email".to_string())
            .with_extra("value".to_string(), "invalid".to_string());

        assert_eq!(ctx.extra.get("field"), Some(&"email".to_string()));
    }

    /// Test: ApiError serialization roundtrip
    #[test]
    fn test_api_error_serialization() {
        use serde_json;

        let error = ApiError::not_found("User", Some("123".to_string()));
        let json = serde_json::to_string(&error).expect("Failed to serialize");

        // Verify it can be deserialized (as Value since we don't have Deserialize)
        let value: serde_json::Value = serde_json::from_str(&json).expect("Invalid JSON");
        // ApiError uses #[serde(tag = "type")], so check for "type" field
        assert!(value.get("type").is_some());
        assert!(value.get("resource").is_some());
    }

    // ============================================================================
    // Performance and Stress Tests
    // ============================================================================

    /// Test: Rapid error creation performance
    #[test]
    fn test_rapid_error_creation_performance() {
        let start = std::time::Instant::now();

        for i in 0..10000 {
            let _error = ApiError::internal_error(format!("Error {}", i), "PERF_TEST");
        }

        let elapsed = start.elapsed();
        // Should create 10000 errors in less than 100ms
        assert!(elapsed < std::time::Duration::from_millis(100));
    }

    /// Test: ServiceResponse with large data
    #[test]
    fn test_service_response_with_large_data() {
        let large_data = "x".repeat(1_000_000); // 1MB string
        let response = ServiceResponse::success(large_data.clone());

        assert_eq!(response.data().unwrap().len(), 1_000_000);
    }

    /// Test: Deeply nested error context
    #[test]
    fn test_deeply_nested_error_context() {
        use sdforge::error::ErrorContext;

        let mut ctx = ErrorContext::new();
        for i in 0..50 {
            ctx = ctx.with_extra(format!("key_{}", i), format!("value_{}", i));
        }

        assert_eq!(ctx.extra.len(), 50);
    }
}

#[cfg(test)]
mod config_tests {
    use sdforge::config::{AppConfig, AuthConfig, ServerConfig};

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        // Verify we can create a default config and access its fields
        let _ = config.server.port;
        let _ = config.server.host.clone();
    }

    #[test]
    fn test_app_config_builder() {
        let config = AppConfig::builder()
            .server(ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                request_timeout_secs: 30,
                cors: None,
            })
            .build()
            .expect("build should succeed with valid config");

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
    }

    #[test]
    fn test_auth_config_api_key() {
        let config = AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "key1".to_string(),
        };

        match config {
            AuthConfig::ApiKey {
                header_name,
                prefix,
            } => {
                assert_eq!(header_name, "X-API-Key");
                assert_eq!(prefix, "key1");
            }
            _ => panic!("Expected ApiKey config"),
        }
    }
}

// Enhanced Core module tests - comprehensive coverage
#[cfg(test)]
mod api_metadata_enhanced_tests {
    use sdforge::core::ApiMetadata;

    /// Test API name length boundaries
    #[test]
    fn test_api_name_length_boundaries() {
        // Empty name
        let empty = ApiMetadata::new(
            "".to_string(),
            "v1".to_string(),
            "Empty name".to_string(),
            None,
            false,
        );
        assert_eq!(empty.name(), "");

        // Single character
        let single = ApiMetadata::new(
            "a".to_string(),
            "v1".to_string(),
            "Single char".to_string(),
            None,
            false,
        );
        assert_eq!(single.name(), "a");

        // Unicode characters
        let unicode = ApiMetadata::new(
            "获取用户信息_中文".to_string(),
            "v1".to_string(),
            "Unicode test".to_string(),
            None,
            false,
        );
        assert_eq!(unicode.name(), "获取用户信息_中文");
    }

    /// Test version string formats
    #[test]
    fn test_version_formats() {
        // Semantic versioning
        let semver = ApiMetadata::new(
            "api".to_string(),
            "1.2.3".to_string(),
            "Semver".to_string(),
            None,
            false,
        );
        assert_eq!(semver.version(), "1.2.3");

        // Simple version
        let simple = ApiMetadata::new(
            "api".to_string(),
            "v1".to_string(),
            "Simple".to_string(),
            None,
            false,
        );
        assert_eq!(simple.version(), "v1");

        // Complex version with metadata
        let complex = ApiMetadata::new(
            "api".to_string(),
            "v1.0.0-beta.1+build.123".to_string(),
            "Complex version".to_string(),
            None,
            false,
        );
        assert_eq!(complex.version(), "v1.0.0-beta.1+build.123");
    }

    /// Test cache TTL boundaries
    #[test]
    fn test_cache_ttl_boundaries() {
        // No caching
        let no_cache = ApiMetadata::new(
            "api".to_string(),
            "v1".to_string(),
            "No cache".to_string(),
            None,
            false,
        );
        assert_eq!(no_cache.cache_ttl(), None);

        // Zero TTL (immediate expiration)
        let zero = ApiMetadata::new(
            "api".to_string(),
            "v1".to_string(),
            "Zero TTL".to_string(),
            Some(0),
            false,
        );
        assert_eq!(zero.cache_ttl(), Some(0));

        // Maximum reasonable TTL (1 year in seconds)
        let max = ApiMetadata::new(
            "api".to_string(),
            "v1".to_string(),
            "Max TTL".to_string(),
            Some(31536000),
            false,
        );
        assert_eq!(max.cache_ttl(), Some(31536000));
    }

    /// Test streaming flag combinations
    #[test]
    fn test_streaming_flag_combinations() {
        // Non-streaming
        let non_streaming = ApiMetadata::new(
            "sync-api".to_string(),
            "v1".to_string(),
            "Synchronous API".to_string(),
            None,
            false,
        );
        assert!(!non_streaming.is_streaming());

        // Streaming
        let streaming = ApiMetadata::new(
            "stream-api".to_string(),
            "v1".to_string(),
            "Streaming API".to_string(),
            None,
            true,
        );
        assert!(streaming.is_streaming());
    }

    /// Test Default trait implementation
    #[test]
    fn test_default_implementation() {
        let default = ApiMetadata::default();
        assert_eq!(default.name(), "");
        assert_eq!(default.version(), "");
        assert_eq!(default.description(), ""); // Default is empty string, not "SDForge API"
        assert_eq!(default.cache_ttl(), None);
        assert!(!default.is_streaming());
    }

    /// Test Debug trait implementation
    #[test]
    fn test_debug_implementation() {
        let metadata = ApiMetadata::new(
            "test-api".to_string(),
            "v1".to_string(),
            "Test API".to_string(),
            Some(60),
            true,
        );
        let debug_str = format!("{:?}", metadata);
        assert!(debug_str.contains("test-api"));
        assert!(debug_str.contains("v1"));
    }
}

// ServiceResponse comprehensive tests
#[cfg(test)]
mod service_response_enhanced_tests {
    use sdforge::core::{ServiceResponse, response::ServiceError};

    /// Test generic type support - String
    #[test]
    fn test_generic_string_support() {
        let response = ServiceResponse::success("test data".to_string());
        assert_eq!(response.data(), Some(&"test data".to_string()));
    }

    /// Test generic type support - integers
    #[test]
    fn test_generic_integer_support() {
        let response = ServiceResponse::success(42);
        assert_eq!(response.data(), Some(&42));
    }

    /// Test generic type support - booleans
    #[test]
    fn test_generic_boolean_support() {
        let response = ServiceResponse::success(true);
        assert_eq!(response.data(), Some(&true));
    }

    /// Test generic type support - tuples
    #[test]
    fn test_generic_tuple_support() {
        let response = ServiceResponse::success(("hello", 42));
        assert_eq!(response.data(), Some(&("hello", 42)));
    }

    /// Test generic type support - Vec
    #[test]
    fn test_generic_vec_support() {
        let vec_data = vec![1, 2, 3];
        let response = ServiceResponse::success(vec_data.clone());
        assert_eq!(response.data(), Some(&vec_data));
    }

    /// Test Option wrapping
    #[test]
    fn test_option_wrapping() {
        let response_with_data = ServiceResponse::success(Some("value".to_string()));
        assert_eq!(response_with_data.data(), Some(&Some("value".to_string())));

        let response_none = ServiceResponse::<Option<String>>::success(None);
        assert_eq!(response_none.data(), Some(&None));
    }

    /// Test Result wrapping
    #[test]
    fn test_result_wrapping() {
        let response_ok = ServiceResponse::success(Ok::<String, String>("ok".to_string()));
        assert_eq!(response_ok.data(), Some(&Ok("ok".to_string())));

        let response_err = ServiceResponse::success(Err::<String, String>("err".to_string()));
        assert_eq!(response_err.data(), Some(&Err("err".to_string())));
    }

    /// Test serialization to JSON
    #[test]
    fn test_json_serialization() {
        use serde_json;

        let response = ServiceResponse::success("test data".to_string());
        let json = serde_json::to_string(&response).expect("Failed to serialize");

        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":\"test data\""));
    }

    /// Test deserialization from JSON
    #[test]
    fn test_json_deserialization() {
        use serde_json;

        let json = r#"{"success":true,"data":"test data"}"#;
        let response: ServiceResponse<String> =
            serde_json::from_str(json).expect("Failed to deserialize");

        assert!(response.is_success());
        assert_eq!(response.data(), Some(&"test data".to_string()));
    }

    /// Test round-trip serialization
    #[test]
    fn test_roundtrip_serialization() {
        use serde_json;

        let original = ServiceResponse::success("test data".to_string());

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ServiceResponse<String> = serde_json::from_str(&json).unwrap();

        assert_eq!(original.is_success(), deserialized.is_success());
        assert_eq!(original.data(), deserialized.data());
    }

    /// Test error response creation
    #[test]
    fn test_error_response_creation() {
        let error = ServiceError::new("VALIDATION_ERROR", "Invalid input", 400);
        let response = ServiceResponse::<String>::error(error);

        assert!(!response.is_success());
        assert!(response.error_ref().is_some());
        assert_eq!(response.error_ref().unwrap().code(), "VALIDATION_ERROR");
    }

    /// Test is_success method
    #[test]
    fn test_is_success_method() {
        let success_response = ServiceResponse::success("data".to_string());
        assert!(success_response.is_success());

        let error = ServiceError::new("ERROR", "msg", 500);
        let error_response = ServiceResponse::<String>::error(error);
        assert!(!error_response.is_success());
    }

    /// Test edge case - zero value
    #[test]
    fn test_zero_value() {
        let response = ServiceResponse::success(0);
        assert_eq!(response.data(), Some(&0));
    }

    /// Test edge case - empty string
    #[test]
    fn test_empty_string() {
        let response = ServiceResponse::success("".to_string());
        assert_eq!(response.data(), Some(&"".to_string()));
    }

    /// Test edge case - special characters
    #[test]
    fn test_special_characters() {
        let response = ServiceResponse::success("<>&\"'\\n\\t\\r".to_string());
        assert_eq!(response.data(), Some(&"<>&\"'\\n\\t\\r".to_string()));
    }

    /// Test very large string (10000 chars)
    #[test]
    fn test_large_string() {
        let large_data = "x".repeat(10000);
        let response = ServiceResponse::success(large_data.clone());
        assert_eq!(response.data().unwrap().len(), 10000);
    }
}
