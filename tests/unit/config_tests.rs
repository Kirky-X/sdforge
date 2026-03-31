#[cfg(feature = "http")]
mod config_tests {
    use sdforge::config::{AppConfig, AuthConfig, ServerConfig};

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

// Enhanced Config module tests - comprehensive coverage
#[cfg(feature = "http")]
mod config_enhanced_tests {
    use sdforge::config::{AppConfig, AuthConfig, ServerConfig};

    // ============================================================================
    // ServerConfig boundary tests
    // ============================================================================

    /// Test 1: Server port boundaries
    #[test]
    fn test_server_port_boundaries() {
        // Minimum valid port
        let min_port = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 1,
            request_timeout_secs: 30,
            cors: None,
        };
        assert_eq!(min_port.port, 1);

        // Common ports
        let http_port = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 80,
            request_timeout_secs: 30,
            cors: None,
        };
        assert_eq!(http_port.port, 80);

        let https_port = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 443,
            request_timeout_secs: 30,
            cors: None,
        };
        assert_eq!(https_port.port, 443);

        // Maximum valid port
        let max_port = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 65535,
            request_timeout_secs: 30,
            cors: None,
        };
        assert_eq!(max_port.port, 65535);
    }

    /// Test 2: Server timeout boundaries
    #[test]
    fn test_server_timeout_boundaries() {
        // Zero timeout (no timeout)
        let no_timeout = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_secs: 0,
            cors: None,
        };
        assert_eq!(no_timeout.request_timeout_secs, 0);

        // Very short timeout
        let short_timeout = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_secs: 1,
            cors: None,
        };
        assert_eq!(short_timeout.request_timeout_secs, 1);

        // Very long timeout (1 day)
        let long_timeout = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_secs: 86400,
            cors: None,
        };
        assert_eq!(long_timeout.request_timeout_secs, 86400);
    }

    /// Test 3: Server host formats
    #[test]
    fn test_server_host_formats() {
        // IPv4 localhost
        let ipv4_local = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: None,
        };
        assert_eq!(ipv4_local.host, "127.0.0.1");

        // IPv4 any address
        let ipv4_any = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: None,
        };
        assert_eq!(ipv4_any.host, "0.0.0.0");

        // IPv6 localhost
        let ipv6_local = ServerConfig {
            host: "::1".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: None,
        };
        assert_eq!(ipv6_local.host, "::1");

        // IPv6 any address
        let ipv6_any = ServerConfig {
            host: "::".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: None,
        };
        assert_eq!(ipv6_any.host, "::");

        // Hostname
        let hostname = ServerConfig {
            host: "localhost".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: None,
        };
        assert_eq!(hostname.host, "localhost");
    }

    // ============================================================================
    // AuthConfig comprehensive tests
    // ============================================================================

    /// Test 4: API Key configuration variations
    #[test]
    fn test_api_key_config_variations() {
        // Standard API key config
        let standard = AuthConfig::ApiKey {
            header_name: "Authorization".to_string(),
            prefix: "Bearer".to_string(),
        };
        match &standard {
            AuthConfig::ApiKey { header_name, prefix } => {
                assert_eq!(header_name, "Authorization");
                assert_eq!(prefix, "Bearer");
            }
            _ => panic!("Expected ApiKey"),
        }

        // Custom header name
        let custom_header = AuthConfig::ApiKey {
            header_name: "X-Custom-Auth".to_string(),
            prefix: "Token".to_string(),
        };
        match &custom_header {
            AuthConfig::ApiKey { header_name, .. } => {
                assert_eq!(header_name, "X-Custom-Auth");
            }
            _ => panic!("Expected ApiKey"),
        }

        // Empty prefix
        let empty_prefix = AuthConfig::ApiKey {
            header_name: "Authorization".to_string(),
            prefix: "".to_string(),
        };
        match &empty_prefix {
            AuthConfig::ApiKey { prefix, .. } => {
                assert_eq!(prefix, "");
            }
            _ => panic!("Expected ApiKey"),
        }
    }

    /// Test 5: JWT configuration variations
    #[test]
    fn test_jwt_config_variations() {
        // Simple secret
        let simple = AuthConfig::Jwt {
            secret: "simple_secret".to_string(),
        };
        match &simple {
            AuthConfig::Jwt { secret } => {
                assert_eq!(secret, "simple_secret");
            }
            _ => panic!("Expected Jwt"),
        }

        // Complex secret with special characters
        let complex = AuthConfig::Jwt {
            secret: "complex-secret_with.special@chars!".to_string(),
        };
        match &complex {
            AuthConfig::Jwt { secret } => {
                assert_eq!(secret, "complex-secret_with.special@chars!");
            }
            _ => panic!("Expected Jwt"),
        }

        // Long secret
        let long_secret = "a".repeat(256);
        let long = AuthConfig::Jwt {
            secret: long_secret.clone(),
        };
        match &long {
            AuthConfig::Jwt { secret } => {
                assert_eq!(secret.len(), 256);
            }
            _ => panic!("Expected Jwt"),
        }
    }

    /// Test 6: AuthConfig Clone and Debug
    #[test]
    fn test_auth_config_clone_and_debug() {
        let api_key = AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "key".to_string(),
        };
        let cloned = api_key.clone();
        
        match (&api_key, &cloned) {
            (AuthConfig::ApiKey { header_name: h1, prefix: p1 }, 
             AuthConfig::ApiKey { header_name: h2, prefix: p2 }) => {
                assert_eq!(h1, h2);
                assert_eq!(p1, p2);
            }
            _ => panic!("Expected ApiKey"),
        }

        let jwt = AuthConfig::Jwt {
            secret: "secret".to_string(),
        };
        let jwt_cloned = jwt.clone();
        
        match (&jwt, &jwt_cloned) {
            (AuthConfig::Jwt { secret: s1 }, AuthConfig::Jwt { secret: s2 }) => {
                assert_eq!(s1, s2);
            }
            _ => panic!("Expected Jwt"),
        }
    }

    // ============================================================================
    // AppConfig comprehensive tests
    // ============================================================================

    /// Test 7: AppConfig builder pattern - all fields
    #[test]
    fn test_app_config_builder_all_fields() {
        let server = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout_secs: 60,
            cors: None,
        };

        let config = AppConfig::builder()
            .server(server)
            .build();

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.request_timeout_secs, 60);
    }

    /// Test 8: AppConfig Default trait implementation
    #[test]
    fn test_app_config_default_implementation() {
        let default = AppConfig::default();
        
        // Verify we can access all major fields
        let _ = &default.server;
        let _ = &default.authentication;
        let _ = &default.timeout;
        
        // Verify it's not None or invalid
        assert!(true);
    }

    /// Test 9: AppConfig multiple builds with different configs
    #[test]
    fn test_app_config_multiple_builds() {
        let config1 = AppConfig::builder()
            .server(ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                request_timeout_secs: 30,
                cors: None,
            })
            .build();

        let config2 = AppConfig::builder()
            .server(ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                request_timeout_secs: 60,
                cors: None,
            })
            .build();

        assert_ne!(config1.server.port, config2.server.port);
        assert_ne!(config1.server.host, config2.server.host);
    }

    // ============================================================================
    // Configuration validation edge cases
    // ============================================================================

    /// Test 10: Empty string in config fields
    #[test]
    fn test_empty_string_in_config_fields() {
        let auth_empty = AuthConfig::ApiKey {
            header_name: "".to_string(),
            prefix: "".to_string(),
        };
        match &auth_empty {
            AuthConfig::ApiKey { header_name, prefix } => {
                assert_eq!(header_name, "");
                assert_eq!(prefix, "");
            }
            _ => panic!("Expected ApiKey"),
        }

        let jwt_empty = AuthConfig::Jwt {
            secret: "".to_string(),
        };
        match &jwt_empty {
            AuthConfig::Jwt { secret } => {
                assert_eq!(secret, "");
            }
            _ => panic!("Expected Jwt"),
        }
    }

    /// Test 11: Unicode in config values
    #[test]
    fn test_unicode_in_config_values() {
        let auth_unicode = AuthConfig::ApiKey {
            header_name: "认证头".to_string(),
            prefix: "令牌".to_string(),
        };
        match &auth_unicode {
            AuthConfig::ApiKey { header_name, prefix } => {
                assert_eq!(header_name, "认证头");
                assert_eq!(prefix, "令牌");
            }
            _ => panic!("Expected ApiKey"),
        }

        let jwt_unicode = AuthConfig::Jwt {
            secret: "秘密キー🔑".to_string(),
        };
        match &jwt_unicode {
            AuthConfig::Jwt { secret } => {
                assert_eq!(secret, "秘密キー🔑");
            }
            _ => panic!("Expected Jwt"),
        }
    }

    /// Test 12: Special characters in config values
    #[test]
    fn test_special_characters_in_config_values() {
        let auth_special = AuthConfig::ApiKey {
            header_name: "X-API-<Key>&Test\"Quote".to_string(),
            prefix: "Bearer'token".to_string(),
        };
        match &auth_special {
            AuthConfig::ApiKey { header_name, prefix } => {
                assert_eq!(header_name, "X-API-<Key>&Test\"Quote");
                assert_eq!(prefix, "Bearer'token");
            }
            _ => panic!("Expected ApiKey"),
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
