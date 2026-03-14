#[cfg(feature = "http")]
mod config_tests {
    use sdforge::config::{
        AppConfig, AuthConfig, CorsConfig, LoggingConfig, RateLimitConfigFile,
        RateLimitEndpointConfig, ServerConfig, TlsConfig,
    };

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
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.request_timeout_secs, 30);
    }

    #[test]
    fn test_server_config_custom() {
        let config = ServerConfig {
            host: "192.168.1.1".to_string(),
            port: 9090,
            request_timeout_secs: 60,
            cors: None,
        };

        assert_eq!(config.host, "192.168.1.1");
        assert_eq!(config.port, 9090);
        assert_eq!(config.request_timeout_secs, 60);
    }

    #[test]
    fn test_auth_config_api_key() {
        let config = AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            keys: vec!["key1".to_string(), "key2".to_string()],
        };

        match config {
            AuthConfig::ApiKey { header_name, keys } => {
                assert_eq!(header_name, "X-API-Key");
                assert_eq!(keys.len(), 2);
            }
            _ => panic!("Expected ApiKey config"),
        }
    }

    #[test]
    fn test_auth_config_jwt() {
        let config = AuthConfig::Jwt {
            secret: "secret_key".to_string(),
            issuer: "issuer".to_string(),
            audience: "audience".to_string(),
        };

        match config {
            AuthConfig::Jwt { secret, issuer, audience } => {
                assert_eq!(secret, "secret_key");
                assert_eq!(issuer, "issuer");
                assert_eq!(audience, "audience");
            }
            _ => panic!("Expected Jwt config"),
        }
    }

    #[test]
    fn test_cors_config_default() {
        let config = CorsConfig::default();
        assert!(config.allow_origins.is_empty());
        assert!(config.allow_methods.is_empty());
        assert!(config.allow_headers.is_empty());
    }

    #[test]
    fn test_cors_config_custom() {
        let config = CorsConfig {
            allow_origins: vec!["https://example.com".to_string()],
            allow_methods: vec!["GET".to_string(), "POST".to_string()],
            allow_headers: vec!["Content-Type".to_string()],
            expose_headers: vec!["X-Custom-Header".to_string()],
            allow_credentials: true,
            max_age: 3600,
        };

        assert_eq!(config.allow_origins.len(), 1);
        assert_eq!(config.allow_methods.len(), 2);
        assert!(config.allow_credentials);
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert_eq!(config.format, "text");
    }

    #[test]
    fn test_logging_config_custom() {
        let config = LoggingConfig {
            level: "debug".to_string(),
            format: "json".to_string(),
            output: Some("logs/app.log".to_string()),
        };

        assert_eq!(config.level, "debug");
        assert_eq!(config.format, "json");
        assert_eq!(config.output, Some("logs/app.log".to_string()));
    }

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::default();
        assert_eq!(config.cert_path, "");
        assert_eq!(config.key_path, "");
    }

    #[test]
    fn test_tls_config_custom() {
        let config = TlsConfig {
            cert_path: "/path/to/cert.pem".to_string(),
            key_path: "/path/to/key.pem".to_string(),
        };

        assert_eq!(config.cert_path, "/path/to/cert.pem");
        assert_eq!(config.key_path, "/path/to/key.pem");
    }

    #[test]
    fn test_rate_limit_config_file_default() {
        let config = RateLimitConfigFile::default();
        assert_eq!(config.enabled, false);
        assert_eq!(config.default_limit, 100);
    }

    #[test]
    fn test_rate_limit_config_file_custom() {
        let config = RateLimitConfigFile {
            enabled: true,
            default_limit: 1000,
            endpoints: vec![RateLimitEndpointConfig {
                path: "/api/users".to_string(),
                limit: 500,
            }],
        };

        assert!(config.enabled);
        assert_eq!(config.default_limit, 1000);
        assert_eq!(config.endpoints.len(), 1);
    }

    #[test]
    fn test_rate_limit_endpoint_config() {
        let config = RateLimitEndpointConfig {
            path: "/api/test".to_string(),
            limit: 100,
        };

        assert_eq!(config.path, "/api/test");
        assert_eq!(config.limit, 100);
    }

    #[test]
    fn test_app_config_with_all_fields() {
        let config = AppConfig {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                request_timeout_secs: 60,
                cors: Some(CorsConfig::default()),
            },
            auth: Some(AuthConfig::ApiKey {
                header_name: "X-API-Key".to_string(),
                keys: vec!["test_key".to_string()],
            }),
            logging: LoggingConfig {
                level: "debug".to_string(),
                format: "json".to_string(),
                output: None,
            },
        };

        assert_eq!(config.server.port, 8080);
        assert!(config.auth.is_some());
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_server_config_with_cors() {
        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: Some(CorsConfig {
                allow_origins: vec!["*".to_string()],
                allow_methods: vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string()],
                allow_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
                expose_headers: vec![],
                allow_credentials: false,
                max_age: 86400,
            }),
        };

        assert!(config.cors.is_some());
        let cors = config.cors.unwrap();
        assert_eq!(cors.allow_methods.len(), 4);
    }
}

#[cfg(not(feature = "http"))]
mod config_tests_placeholder {
    #[test]
    fn test_http_feature_required() {
        assert!(true, "Config tests require http feature");
    }
}
