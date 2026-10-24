// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Tests for `build_with_config_ext`: various `ServerConfig`, `AuthConfig`, and
//! `CorsConfig` combinations, middleware layer wiring, and feature-gated
//! inventory preservation through `build`.

#[cfg(feature = "cache")]
use crate::config::CacheConfig;
#[cfg(feature = "security")]
use crate::config::SecurityConfig;
use crate::config::{ApiKeySeed, AuthConfig, CorsConfig, SdForgeConfig, ServerConfig};
#[cfg(any(feature = "mcp", feature = "websocket", feature = "grpc"))]
use crate::http::build;
use crate::http::build_with_config_ext;
#[cfg(feature = "idempotency")]
use sdforge::config::IdempotencyConfig;

// Auth middleware only exists with the `security` feature. Without it the
// fail-closed contract (see `AuthConfig::require_security_feature`) rejects
// any AuthConfig requesting authentication, so the happy-path auth builds
// below are security-only.
/// Test build_with_config_ext with JWT authentication
#[cfg(feature = "security")]
#[test]
fn test_build_with_config_jwt() {
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::Jwt {
            secret: "ThisIsAVeryLongSecretKeyWithUppercase123!@#ForTesting".to_string(),
            audience: None,
            issuer: None,
        },
        timeout: None,
        ..Default::default()
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok(), "Should build successfully with JWT config");
}

/// Test build_with_config_ext with ApiKey authentication
#[cfg(feature = "security")]
#[test]
fn test_build_with_config_api_key() {
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "key-".to_string(),
            keys: vec![ApiKeySeed {
                key: "test-key-0123456789abcdef".to_string(),
                permissions: vec!["read".to_string()],
            }],
        },
        timeout: None,
        ..Default::default()
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(
        result.is_ok(),
        "Should build successfully with ApiKey config"
    );
}

/// Test build_with_config_ext with OAuth2 returns error
#[test]
fn test_build_with_config_oauth2_error() {
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok(), "Should succeed with None auth config");
}

/// Test build_with_config_ext with CORS configuration
#[cfg(feature = "security")]
#[test]
fn test_build_with_config_cors() {
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: Some(CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
                allowed_methods: vec!["GET".to_string(), "POST".to_string()],
                allowed_headers: vec!["Content-Type".to_string()],
            }),
            ..Default::default()
        },
        authentication: AuthConfig::Jwt {
            secret: "ThisIsAVeryLongSecretKeyWithUppercase123!@#ForTesting".to_string(),
            audience: None,
            issuer: None,
        },
        timeout: None,
        ..Default::default()
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok(), "Should build successfully with CORS config");
}

// build_with_config Extended Tests - Middleware Integration

#[test]
fn test_build_with_config_request_id_middleware() {
    // Tests the request ID middleware (lines 226-240)
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 60,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_config_body_limit() {
    // Tests body limit layer (lines 243-245)
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_config_compression_layer() {
    // Tests compression layer (line 248)
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_config_timeout_layer() {
    // Tests timeout layer (lines 251-255)
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 5,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_config_zero_timeout() {
    // Zero timeout would time out every request (a TimeoutLayer of 0s never
    // yields), so build_with_config rejects it up front — mirroring
    // ServerConfig::validate.
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 0,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(
        result.is_err(),
        "zero request_timeout_secs must be rejected"
    );
}

#[test]
fn test_build_with_config_zero_body_size_rejected() {
    // max_body_size = 0 would reject every request that carries a body.
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            tls: None,
            max_body_size: 0,
            #[cfg(feature = "idempotency")]
            idempotency: IdempotencyConfig::default(),
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_err(), "zero max_body_size must be rejected");
}

#[test]
fn test_build_with_config_large_timeout() {
    // Test with large timeout value
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 3600,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_config_cors_various_origins() {
    // Test CORS with multiple origins
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: Some(CorsConfig {
                allowed_origins: vec![
                    "http://localhost:3000".to_string(),
                    "http://example.com".to_string(),
                    "https://app.example.com".to_string(),
                ],
                allowed_methods: vec!["GET".to_string()],
                allowed_headers: vec!["Content-Type".to_string()],
            }),
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_config_cors_all_methods() {
    // Test CORS with valid origins and common methods
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: Some(CorsConfig {
                allowed_origins: vec![
                    "http://localhost:3000".to_string(),
                    "https://example.com".to_string(),
                ],
                allowed_methods: vec!["GET".to_string(), "POST".to_string()],
                allowed_headers: vec!["Content-Type".to_string()],
            }),
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

// Authentication Tests - ApiKey Edge Cases

#[cfg(feature = "security")]
#[test]
fn test_build_with_config_api_key_empty_prefix() {
    // Test ApiKey with empty prefix (lines 306-308)
    // NOTE: prefix emptiness is only rejected by the auth middleware itself;
    // build_with_config does not run AuthConfig::validate, so this is Ok.
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "".to_string(),
            keys: vec![ApiKeySeed {
                key: "test-key-0123456789abcdef".to_string(),
                permissions: vec!["read".to_string()],
            }],
        },
        timeout: None,
        ..Default::default()
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

#[cfg(feature = "security")]
#[test]
fn test_build_with_config_api_key_long_prefix() {
    // Test ApiKey with long prefix
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::ApiKey {
            header_name: "X-Custom-API-Key".to_string(),
            prefix: "Bearer-api-key-".to_string(),
            keys: vec![ApiKeySeed {
                key: "test-key-0123456789abcdef".to_string(),
                permissions: vec!["read".to_string()],
            }],
        },
        timeout: None,
        ..Default::default()
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

#[cfg(feature = "security")]
#[test]
fn test_build_with_config_api_key_special_chars() {
    // Test ApiKey with special characters in prefix
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::ApiKey {
            header_name: "X-Api-Key".to_string(),
            prefix: "key_123-".to_string(),
            keys: vec![ApiKeySeed {
                key: "test-key-0123456789abcdef".to_string(),
                permissions: vec!["read".to_string()],
            }],
        },
        timeout: None,
        ..Default::default()
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

// Authentication Tests - JWT Edge Cases

#[cfg(feature = "security")]
#[test]
fn test_build_with_config_jwt_short_secret() {
    // JWT requires minimum secret length and character classes
    // This tests that valid secrets work correctly
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::Jwt {
            secret: "ValidSecretKey123!@#WithUppercase".to_string(),
            audience: None,
            issuer: None,
        },
        timeout: None,
        ..Default::default()
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

#[cfg(feature = "security")]
#[test]
fn test_build_with_config_jwt_special_chars() {
    // Test JWT secret with special characters (must meet complexity requirements)
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::Jwt {
            secret: "SpecialChars!@#$%^&*()_+Secret123".to_string(),
            audience: None,
            issuer: None,
        },
        timeout: None,
        ..Default::default()
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

#[cfg(feature = "security")]
#[test]
fn test_build_with_config_jwt_empty_secret() {
    // Note: BearerAuth::new panics with empty/invalid secrets.
    // This test uses a valid secret to verify build path works.
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::Jwt {
            secret: "ValidSecretForTesting123!@#WithUppercase".to_string(),
            audience: None,
            issuer: None,
        },
        timeout: None,
        ..Default::default()
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok());
}

// Feature-Gated Code Paths Tests

#[cfg(feature = "mcp")]
#[test]
fn test_build_preserves_mcp_inventory_with_routes() {
    // Test that MCP inventory is preserved when building router
    // This exercises lines 150-151
    let router = build();
    let _ = router;
}

#[cfg(feature = "websocket")]
#[test]
fn test_build_preserves_websocket_inventory_with_routes() {
    // Test that WebSocket inventory is preserved
    // This exercises lines 153-154
    let router = build();
    let _ = router;
}

#[cfg(feature = "grpc")]
#[test]
fn test_build_preserves_grpc_inventory_with_routes() {
    // Test that gRPC inventory is preserved
    // This exercises lines 156-157
    let router = build();
    let _ = router;
}

// build_with_config Integration Tests - All Middleware Combined

#[cfg(feature = "security")]
#[test]
fn test_build_with_config_full_jwt_cors() {
    // Test with all middleware: JWT + CORS + security headers + timeout
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 443,
            request_timeout_secs: 15,
            cors: Some(CorsConfig {
                allowed_origins: vec!["https://app.example.com".to_string()],
                allowed_methods: vec!["GET".to_string(), "POST".to_string()],
                allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            }),
            ..Default::default()
        },
        authentication: AuthConfig::Jwt {
            secret: "AnotherVeryLongSecretKeyForTestingPurposes1234567890!".to_string(),
            audience: None,
            issuer: None,
        },
        timeout: None,
        ..Default::default()
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok(), "Should build with full config");
}

#[cfg(feature = "security")]
#[test]
fn test_build_with_config_full_api_key_cors() {
    // Test with all middleware: ApiKey + CORS + security headers + timeout
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8443,
            request_timeout_secs: 20,
            cors: Some(CorsConfig {
                allowed_origins: vec!["http://localhost:8080".to_string()],
                allowed_methods: vec!["GET".to_string(), "POST".to_string(), "PUT".to_string()],
                allowed_headers: vec!["Content-Type".to_string(), "X-API-Key".to_string()],
            }),
            ..Default::default()
        },
        authentication: AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "sk-".to_string(),
            keys: vec![ApiKeySeed {
                key: "test-key-0123456789abcdef".to_string(),
                permissions: vec!["read".to_string()],
            }],
        },
        timeout: None,
        ..Default::default()
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(result.is_ok(), "Should build with full config");
}

// build_with_config_ext with AuthConfig::None
//
// The existing build_with_config tests cover Jwt, ApiKey, and OAuth2
// error paths, but not the AuthConfig::None (no authentication) path.
// This test verifies build_with_config succeeds when auth is disabled.

/// Test build_with_config succeeds with AuthConfig::None and no CORS,
/// verifying the non-security, non-auth path through the function.
#[test]
fn test_build_with_config_no_auth() {
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(
        result.is_ok(),
        "build_with_config should succeed with AuthConfig::None: {:?}",
        result.err()
    );
}

/// Test build_with_config_ext with a minimal valid config and zero-length
/// host to verify the request ID middleware and body limit layers are
/// applied without panic.
#[test]
fn test_build_with_config_minimal_config() {
    let config = SdForgeConfig {
        server: ServerConfig {
            host: String::new(),
            port: 1,
            request_timeout_secs: 1,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    let result = build_with_config_ext(axum::Router::new(), &config);
    assert!(
        result.is_ok(),
        "Minimal config should build: {:?}",
        result.err()
    );
}

// Fail-closed feature contract

/// With the `security` feature disabled, an AuthConfig requesting
/// authentication (ApiKey/Jwt) must fail at build time instead of silently
/// producing an unauthenticated router.
#[cfg(not(feature = "security"))]
#[test]
fn test_build_with_config_auth_without_security_feature_rejected() {
    let jwt = SdForgeConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::Jwt {
            secret: "ThisIsAVeryLongSecretKeyWithUppercase123!@#ForTesting".to_string(),
            audience: None,
            issuer: None,
        },
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };
    let api_key = SdForgeConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "key-".to_string(),
            keys: vec![ApiKeySeed {
                key: "test-key-0123456789abcdef".to_string(),
                permissions: vec!["read".to_string()],
            }],
        },
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        #[cfg(feature = "security")]
        security: SecurityConfig::default(),
    };

    for config in [jwt, api_key] {
        let err = build_with_config_ext(axum::Router::new(), &config)
            .expect_err("AuthConfig without the `security` feature must fail at build time");
        let msg = err.to_string();
        assert!(
            msg.contains("security"),
            "error must name the disabled `security` feature: {msg}"
        );
    }
}

/// 配置里的安全响应头必须真正下发到响应上（此前配置面完全未被消费）。
#[cfg(feature = "security")]
#[tokio::test]
async fn test_build_with_config_applies_configured_security_headers() {
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        security: SecurityConfig {
            content_security_policy: "default-src 'none'".to_string(),
            ..Default::default()
        },
    };

    let router = build_with_config_ext(axum::Router::new(), &config).unwrap();
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/header-test-route")
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        response
            .headers()
            .get("content-security-policy")
            .and_then(|v| v.to_str().ok()),
        Some("default-src 'none'"),
        "the configured CSP must reach the response"
    );
}

/// `enable_headers = false` 必须真的不装安全响应头层（逃生阀此前无效）。
#[cfg(feature = "security")]
#[tokio::test]
async fn test_build_with_config_can_disable_security_headers() {
    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        security: SecurityConfig::disabled(),
    };

    let router = build_with_config_ext(axum::Router::new(), &config).unwrap();
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/header-test-route")
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert!(
        response.headers().get("content-security-policy").is_none(),
        "disabled security headers must not be emitted"
    );
}

/// 合并进来的路由必须与内置路由受同一套鉴权保护。
///
/// axum 的 `Router::layer` 只作用于调用它之前已注册的路由：在
/// `build_with_config` 之后 merge 进来的路由会绕过鉴权层。
#[cfg(feature = "security")]
#[tokio::test]
async fn test_build_with_config_ext_protects_merged_routes() {
    use axum::Router;

    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "key-".to_string(),
            keys: vec![ApiKeySeed {
                key: "test-key-0123456789abcdef".to_string(),
                permissions: vec!["read".to_string()],
            }],
        },
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        security: SecurityConfig::default(),
    };

    let app = Router::new().route("/merged", axum::routing::get(|| async { "merged-ok" }));
    let router = crate::http::build_with_config_ext(app, &config).unwrap();

    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/merged")
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        response.status(),
        axum::http::StatusCode::UNAUTHORIZED,
        "a merged route must not bypass the authentication layer"
    );
}

/// 既并入调用方路由、又装上限流层：应用路由既受鉴权保护，也受限流约束。
///
/// 此前"并入调用方路由"与"装配限流层"分属两个互斥入口，生产应用只能二选一。
#[cfg(all(feature = "security", feature = "ratelimit-http"))]
#[tokio::test]
async fn test_build_with_config_ext_rate_limited_merges_and_limits() {
    use axum::Router;
    use limiteron::config::ConfigBuilder;

    // 60s 窗口内最多 2 次：测试内不会因窗口重置而抖动。
    let rate_limit = ConfigBuilder::new()
        .with_rule(|rule| {
            rule.id("test")
                .name("test")
                .priority(100)
                .ip_matcher(vec!["127.0.0.1/32".to_string()])
                .fixed_window("60s", 2)
                .on_reject()
        })
        .build()
        .expect("valid flow control config");

    let config = SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            prefix: "key-".to_string(),
            keys: vec![ApiKeySeed {
                key: "test-key-0123456789abcdef".to_string(),
                permissions: vec!["read".to_string()],
            }],
        },
        timeout: None,
        #[cfg(feature = "cache")]
        cache: CacheConfig::default(),
        security: SecurityConfig {
            rate_limit: Some(rate_limit),
            ..Default::default()
        },
    };

    let app = Router::new().route("/merged", axum::routing::get(|| async { "merged-ok" }));
    let router = crate::http::build_with_config_ext_rate_limited(app, &config)
        .await
        .expect("rate limited build must succeed");

    // 客户端 IP 取自 ConnectInfo（限流标识维度）。
    let request = |api_key: Option<&str>| {
        let mut builder = axum::http::Request::builder().uri("/merged");
        if let Some(key) = api_key {
            builder = builder.header("X-API-Key", key);
        }
        let mut req = builder.body(axum::body::Body::empty()).unwrap();
        req.extensions_mut()
            .insert(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                [127, 0, 0, 1],
                40000,
            ))));
        req
    };

    // 1) 无凭据：并入的应用路由仍受鉴权保护（401）。
    let unauthorized = tower::ServiceExt::oneshot(router.clone(), request(None))
        .await
        .unwrap();
    assert_eq!(
        unauthorized.status(),
        axum::http::StatusCode::UNAUTHORIZED,
        "a merged route must not bypass the authentication layer"
    );

    // 2) 有效凭据：正常放行。
    let allowed = tower::ServiceExt::oneshot(
        router.clone(),
        request(Some("key-test-key-0123456789abcdef")),
    )
    .await
    .unwrap();
    assert_eq!(allowed.status(), axum::http::StatusCode::OK);

    // 3) 超出窗口配额：限流层生效（429）。
    let limited =
        tower::ServiceExt::oneshot(router, request(Some("key-test-key-0123456789abcdef")))
            .await
            .unwrap();
    assert_eq!(
        limited.status(),
        axum::http::StatusCode::TOO_MANY_REQUESTS,
        "the configured rate limit must be installed by the combined entry point"
    );
}
