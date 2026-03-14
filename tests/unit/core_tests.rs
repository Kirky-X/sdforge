// Unit tests for SDForge framework
// Tests core functionality and module integration

#[cfg(test)]
mod core_tests {
    use sdforge::core::{ApiError, ServiceResponse, ApiMetadata};

    #[test]
    fn test_api_error_not_found() {
        let error = ApiError::not_found("User", Some("123"));
        match error {
            ApiError::NotFound { resource, resource_id } => {
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
            ApiError::ValidationError { field, message } => {
                assert_eq!(field, "field_name");
                assert_eq!(message, "Invalid value");
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_api_error_invalid_input() {
        let error = ApiError::invalid_input("Query cannot be empty", Some("query".to_string()));
        match error {
            ApiError::InvalidInput { message, field } => {
                assert_eq!(message, "Query cannot be empty");
                assert_eq!(field, Some("query".to_string()));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_service_response_success() {
        let response = ServiceResponse::new("test data".to_string());
        assert_eq!(response.data(), "test data");
        assert!(response.is_success());
    }

    #[test]
    fn test_service_response_error() {
        let error = ApiError::not_found("User", None);
        let response = ServiceResponse::<String>::from_error(error.clone());
        assert!(!response.is_success());
    }

    #[test]
    fn test_api_metadata() {
        let metadata = ApiMetadata {
            name: "get_user".to_string(),
            version: "v1".to_string(),
            description: "Get user by ID".to_string(),
            cache_ttl: Some(300),
            is_streaming: false,
        };

        assert_eq!(metadata.name(), "get_user");
        assert_eq!(metadata.version(), "v1");
        assert_eq!(metadata.description(), "Get user by ID");
        assert_eq!(metadata.cache_ttl(), Some(300));
    }

    #[test]
    fn test_api_metadata_default() {
        let metadata = ApiMetadata::default();
        assert_eq!(metadata.name(), "");
        assert_eq!(metadata.version(), "v1");
    }
}

#[cfg(test)]
mod config_tests {
    use sdforge::config::{AppConfig, ServerConfig, AuthConfig, LoggingConfig};

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
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
            keys: vec!["key1".to_string()],
        };

        match config {
            AuthConfig::ApiKey { header_name, keys } => {
                assert_eq!(header_name, "X-API-Key");
                assert!(keys.contains(&"key1".to_string()));
            }
            _ => panic!("Expected ApiKey config"),
        }
    }

    #[test]
    fn test_logging_config() {
        let config = LoggingConfig {
            level: "debug".to_string(),
            format: "json".to_string(),
        };

        assert_eq!(config.level, "debug");
        assert_eq!(config.format, "json");
    }
}

#[cfg(test)]
mod cache_tests {
    use sdforge::CacheConfig;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.ttl, 300);
        assert_eq!(config.max_size, 100);
    }

    #[test]
    fn test_cache_config_custom() {
        let config = CacheConfig::new(600, 200);
        assert_eq!(config.ttl, 600);
        assert_eq!(config.max_size, 200);
    }

    #[test]
    fn test_cache_config_with_ttl() {
        let config = CacheConfig::with_ttl(1200);
        assert_eq!(config.ttl, 1200);
    }
}

#[cfg(test)]
mod validation_tests {
    use sdforge::core::validation::{validate_string, validate_email, validate_url};

    #[test]
    fn test_validate_string_valid() {
        let result = validate_string("valid_string", 1, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_string_too_long() {
        let long_string = "a".repeat(200);
        let result = validate_string(&long_string, 1, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_email_valid() {
        let result = validate_email("test@example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        let result = validate_email("invalid-email");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_valid() {
        let result = validate_url("https://example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_url_invalid() {
        let result = validate_url("not-a-url");
        assert!(result.is_err());
    }
}
