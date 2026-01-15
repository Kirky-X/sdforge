//! Configuration integration tests
//!
//! Tests for configuration loading, conversion, and application.

#[cfg(all(test, feature = "http"))]
mod config_integration_tests {
    use axiom::config::{
        build_cors_layer, AppConfig, ConfigError, CorsConfig, RateLimitConfigFile,
        RateLimitEndpointConfig,
    };
    #[cfg(feature = "security")]
    use axiom::security::{RateLimitConfig, RateLimiter};
    use std::convert::TryFrom;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.api.name, "axiom-api");
        assert_eq!(config.api.version, "0.1.0");
        assert!(config.server.cors.is_none());
        assert!(config.rate_limit.is_none());
        assert!(config.authentication.is_none());
    }

    #[test]
    #[cfg(feature = "security")]
    fn test_rate_limit_config_conversion() {
        let file_config = RateLimitConfigFile {
            max_requests: 100,
            window_seconds: 60,
            endpoints: std::collections::HashMap::new(),
        };

        let rate_config = RateLimitConfig::try_from(file_config).unwrap();
        assert_eq!(rate_config.max_requests, 100);
        assert_eq!(rate_config.window.as_secs(), 60);
        assert!(rate_config.include_headers);
    }

    #[test]
    #[cfg(feature = "security")]
    fn test_rate_limit_config_conversion_with_endpoints() {
        let mut endpoints = std::collections::HashMap::new();
        endpoints.insert(
            "api/v1/users".to_string(),
            RateLimitEndpointConfig {
                max_requests: 50,
                window_seconds: 30,
            },
        );

        let file_config = RateLimitConfigFile {
            max_requests: 100,
            window_seconds: 60,
            endpoints,
        };

        let rate_config = RateLimitConfig::try_from(file_config).unwrap();
        assert_eq!(rate_config.max_requests, 100);
        assert_eq!(rate_config.window.as_secs(), 60);
    }

    #[test]
    fn test_cors_config() {
        let config = CorsConfig {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
            allow_credentials: false,
            max_age: Some(3600),
        };

        assert!(config.allowed_origins.contains(&"*".to_string()));
        assert_eq!(config.allowed_methods.len(), 2);
        assert_eq!(config.max_age, Some(3600));
    }

    #[test]
    fn test_build_cors_layer() {
        let config = CorsConfig {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "https://example.com".to_string(),
            ],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
            allow_credentials: false,
            max_age: Some(3600),
        };

        let cors_layer = build_cors_layer(&config);
        assert!(cors_layer.is_ok());
    }

    #[test]
    fn test_build_cors_layer_empty_origins() {
        let config = CorsConfig {
            allowed_origins: vec![],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec![],
            allow_credentials: false,
            max_age: None,
        };

        let result = build_cors_layer(&config);
        // Empty origins should be allowed
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "security")]
    fn test_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 3,
            window: std::time::Duration::from_secs(60),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(config));

        // First 3 requests should succeed
        for _ in 0..3 {
            assert!(limiter.check("test-ip").is_ok());
        }

        // 4th request should fail
        assert!(limiter.check("test-ip").is_err());

        let err = limiter.check("test-ip").unwrap_err();
        assert_eq!(err.limit, 3);
        assert_eq!(err.remaining, 0);
        assert!(err.retry_after > 0);
    }

    #[test]
    #[cfg(feature = "security")]
    fn test_rate_limiter_different_keys() {
        let config = RateLimitConfig {
            max_requests: 2,
            window: std::time::Duration::from_secs(60),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(config));

        // Different keys should have separate limits
        assert!(limiter.check("ip1").is_ok());
        assert!(limiter.check("ip1").is_ok());
        assert!(limiter.check("ip1").is_err());

        assert!(limiter.check("ip2").is_ok());
        assert!(limiter.check("ip2").is_ok());
        assert!(limiter.check("ip2").is_err());
    }

    #[test]
    fn test_config_error_types() {
        use serde::de::Error;

        let error = ConfigError::LoadError(
            std::path::PathBuf::from("/nonexistent/config.toml"),
            std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
        );

        assert!(matches!(error, ConfigError::LoadError(_, _)));

        let error = ConfigError::ParseError(toml::de::Error::custom("Parse error"));
        assert!(matches!(error, ConfigError::ParseError(_)));

        let error = ConfigError::ValidationError("Validation error".to_string());
        assert!(matches!(error, ConfigError::ValidationError(_)));
    }
}
