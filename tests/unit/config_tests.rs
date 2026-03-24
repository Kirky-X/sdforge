#[cfg(feature = "http")]
mod config_tests {
    use sdforge::config::{
        AppConfig, AuthConfig, ServerConfig,
    };

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        // Just verify we can create a default config
        let _ = config.server.port;
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
        // Just verify we can create a default config
        let _ = config.port;
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
            prefix: "key1".to_string(),
        };

        match config {
            AuthConfig::ApiKey { header_name, prefix } => {
                assert_eq!(header_name, "X-API-Key");
                assert_eq!(prefix, "key1");
            }
            _ => panic!("Expected ApiKey config"),
        }
    }

    #[test]
    fn test_auth_config_jwt() {
        let config = AuthConfig::Jwt {
            secret: "secret_key".to_string(),
        };

        match config {
            AuthConfig::Jwt { secret } => {
                assert_eq!(secret, "secret_key");
            }
            _ => panic!("Expected Jwt config"),
        }
    }
}

#[cfg(not(feature = "http"))]
mod config_tests_placeholder {
    #[test]
    fn test_http_feature_required() {
        assert!(true, "Config tests require http feature");
    }
}
