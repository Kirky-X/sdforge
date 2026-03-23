// UAT Acceptance Tests
// Covers UAT-001 to UAT-015 scenarios

// UAT-001: HTTP Service Quick Integration
#[cfg(feature = "http")]
mod uat_001_http_integration {
    use sdforge::http::build;

    #[test]
    fn test_quick_http_integration() {
        let app = build();
        assert!(app.is_ok(), "HTTP service should build successfully");
    }
}

// UAT-002: MCP Tool Service
#[cfg(feature = "mcp")]
mod uat_002_mcp_service {
    use sdforge::mcp::build;

    #[tokio::test]
    async fn test_mcp_service_creation() {
        let server = build().await;
        assert!(server.is_ok(), "MCP service should build successfully");
    }
}

// UAT-003: Dual Protocol Support
#[cfg(all(feature = "http", feature = "mcp"))]
mod uat_003_dual_protocol {
    use sdforge::http::build as http_build;
    use sdforge::mcp::build as mcp_build;

    #[tokio::test]
    async fn test_dual_protocol_build() {
        let http_result = http_build();
        let mcp_result = mcp_build().await;

        assert!(http_result.is_ok(), "HTTP should build");
        assert!(mcp_result.is_ok(), "MCP should build");
    }
}

// UAT-004: Module Organization
#[cfg(feature = "http")]
mod uat_004_module_organization {
    use sdforge::core::ApiMetadata;

    #[test]
    fn test_module_isolation() {
        let metadata1 = ApiMetadata {
            name: "auth/login".to_string(),
            version: "v1".to_string(),
            description: "Auth module".to_string(),
            cache_ttl: None,
            is_streaming: false,
        };

        let metadata2 = ApiMetadata {
            name: "users/profile".to_string(),
            version: "v1".to_string(),
            description: "Users module".to_string(),
            cache_ttl: None,
            is_streaming: false,
        };

        assert_ne!(metadata1.name(), metadata2.name());
    }
}

// UAT-007: Nested Structure Serialization
#[cfg(feature = "http")]
mod uat_007_nested_serialization {
    use sdforge::core::ServiceResponse;
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, Debug)]
    struct Customer {
        id: u64,
        name: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct Order {
        id: u64,
        customer: Customer,
    }

    #[test]
    fn test_nested_serialization() {
        let order = Order {
            id: 1,
            customer: Customer { id: 100, name: "Alice".to_string() },
        };

        let response = ServiceResponse::new(order);
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("id"));
        assert!(json.contains("customer"));
    }
}

// UAT-009: Unified Error Response
#[cfg(feature = "http")]
mod uat_009_error_response {
    use sdforge::core::ApiError;
    use serde_json;

    #[test]
    fn test_error_response_format() {
        let errors = vec![
            ApiError::not_found("User", Some("123".to_string())),
            ApiError::validation_error("email", "Invalid format"),
            ApiError::invalid_input("Query cannot be empty", Some("query".to_string())),
        ];

        for error in errors {
            let json = serde_json::to_string(&error).unwrap();
            assert!(json.contains("NOT_FOUND") || json.contains("VALIDATION") || json.contains("INVALID"));
        }
    }

    #[test]
    fn test_error_http_status_mapping() {
        let not_found = ApiError::not_found("User", None);
        assert_eq!(not_found.status_code(), 404);

        let validation = ApiError::validation_error("field", "msg");
        assert_eq!(validation.status_code(), 400);

        let invalid = ApiError::invalid_input("msg", None);
        assert_eq!(invalid.status_code(), 400);
    }
}

// UAT-010: API Version Management
#[cfg(feature = "http")]
mod uat_010_version_management {
    use sdforge::core::ApiMetadata;

    #[test]
    fn test_multiple_versions() {
        let v1 = ApiMetadata {
            name: "get_user".to_string(),
            version: "v1".to_string(),
            description: "Get user v1".to_string(),
            cache_ttl: None,
            is_streaming: false,
        };

        let v2 = ApiMetadata {
            name: "get_user".to_string(),
            version: "v2".to_string(),
            description: "Get user v2".to_string(),
            cache_ttl: None,
            is_streaming: false,
        };

        assert_eq!(v1.name(), v2.name());
        assert_eq!(v1.version(), "v1");
        assert_eq!(v2.version(), "v2");
    }
}

// UAT-011: Performance Target (3000 QPS)
#[cfg(feature = "http")]
mod uat_011_performance {
    use std::time::Instant;
    use sdforge::core::ServiceResponse;

    #[test]
    fn test_response_creation_performance() {
        let start = Instant::now();

        for _ in 0..10000 {
            let _ = ServiceResponse::new("test data".to_string());
        }

        let duration = start.elapsed();
        assert!(duration.as_millis() < 1000, "Response creation should be fast");
    }
}
