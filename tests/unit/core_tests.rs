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
            .build();

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
