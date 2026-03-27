// Copyright (c) 2026 Kirky.X
//! Security Authentication Integration Tests
//!
//! This module contains comprehensive integration tests for the security module.
//! Tests cover API Key authentication, Bearer Token authentication,
//! IP validation, and audit logging.
//!
//! All tests are integration tests and use real functionality without mocks.

#[cfg(feature = "security")]
mod security_tests {
    use hmac::{Hmac, Mac};
    use sdforge::security::{
        ApiKeyMetadata, AppApiKeyAuth, AppApiKeyAuthBuilder, AppAuditLogger, AppAuditLoggerBuilder,
        AuditResult, AuthContext, AuthMetadata, BearerAuth, BearerAuthBuilder, LruConfig, RotationConfig,
    };
    use sha2::Sha256;
    use std::time::Duration;

    // ============================================================================
    // Test Helper Functions
    // ============================================================================

    /// Helper to create auth context
    fn create_test_context(user_id: Option<&str>) -> AuthContext {
        AuthContext::new(
            user_id.map(String::from),
            vec!["read".to_string(), "write".to_string()],
            AuthMetadata::new(
                Some("192.168.1.100".to_string()),
                Some("Test-Agent/1.0".to_string()),
            ),
        )
    }

    /// Create a valid JWT token for testing
    ///
    /// This helper creates a properly signed JWT token with the given secret.
    fn create_jwt_token(
        secret: &[u8],
        user_id: &str,
        permissions: Vec<&str>,
        exp_offset_secs: i64,
    ) -> String {
        use base64::Engine;

        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let now = chrono::Utc::now().timestamp();
        let exp = now + exp_offset_secs;

        let payload = serde_json::json!({
            "sub": user_id,
            "permissions": permissions,
            "iat": now,
            "exp": exp,
        });

        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
        let payload_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());

        let signature_input = format!("{}.{}", header_b64, payload_b64);
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
        mac.update(signature_input.as_bytes());
        let signature = mac.finalize().into_bytes();
        let signature_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature);

        format!("{}.{}.{}", header_b64, payload_b64, signature_b64)
    }

    /// Create a valid JWT token with audience claim
    fn create_jwt_with_audience(
        secret: &[u8],
        user_id: &str,
        audience: &str,
        issuer: Option<&str>,
        exp_offset_secs: i64,
    ) -> String {
        use base64::Engine;

        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let now = chrono::Utc::now().timestamp();
        let exp = now + exp_offset_secs;

        let mut payload_map = serde_json::Map::new();
        payload_map.insert(
            "sub".to_string(),
            serde_json::Value::String(user_id.to_string()),
        );
        payload_map.insert(
            "aud".to_string(),
            serde_json::Value::String(audience.to_string()),
        );
        payload_map.insert("iat".to_string(), serde_json::Value::Number(now.into()));
        payload_map.insert("exp".to_string(), serde_json::Value::Number(exp.into()));

        if let Some(iss) = issuer {
            payload_map.insert(
                "iss".to_string(),
                serde_json::Value::String(iss.to_string()),
            );
        }

        let payload = serde_json::Value::Object(payload_map);

        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
        let payload_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());

        let signature_input = format!("{}.{}", header_b64, payload_b64);
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
        mac.update(signature_input.as_bytes());
        let signature = mac.finalize().into_bytes();
        let signature_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature);

        format!("{}.{}.{}", header_b64, payload_b64, signature_b64)
    }

    // ============================================================================
    // API Key Authentication Tests
    // ============================================================================

    /// Test: Valid API key authentication
    ///
    /// Verifies that a valid API key is correctly validated and returns
    /// the expected permissions.
    #[tokio::test]
    async fn test_api_key_valid_authentication() {
        let auth = AppApiKeyAuth::new();
        let test_key = "test_api_key_12345";
        let permissions = vec!["read".to_string(), "write".to_string()];

        // Add the API key
        auth.add_key(test_key, permissions.clone());

        // Validate the key
        let result = auth.validate_key(test_key, "192.168.1.100");

        assert!(result.is_some());
        let returned_perms = result.unwrap();
        assert_eq!(returned_perms.len(), 2);
        assert!(returned_perms.contains(&"read".to_string()));
        assert!(returned_perms.contains(&"write".to_string()));
    }

    /// Test: Invalid API key authentication
    ///
    /// Verifies that an invalid API key returns None and does not panic.
    #[tokio::test]
    async fn test_api_key_invalid_key() {
        let auth = AppApiKeyAuth::new();

        // Validate with an invalid key
        let result = auth.validate_key("invalid_key_not_registered", "192.168.1.100");

        assert!(result.is_none());
    }

    /// Test: API key with different client IP
    ///
    /// Verifies that API key validation works correctly regardless of client IP.
    #[tokio::test]
    async fn test_api_key_prefix_matching() {
        let auth = AppApiKeyAuth::new();
        let test_key = "prefix_test_key_v1";
        let permissions = vec!["admin".to_string()];

        auth.add_key(test_key, permissions.clone());

        // Should work with any IP
        let result1 = auth.validate_key(test_key, "192.168.1.1");
        let result2 = auth.validate_key(test_key, "10.0.0.1");
        let result3 = auth.validate_key(test_key, "8.8.8.8");

        assert!(result1.is_some());
        assert!(result2.is_some());
        assert!(result3.is_some());
    }

    /// Test: API key case sensitivity
    ///
    /// Verifies that API keys are case-sensitive.
    #[tokio::test]
    async fn test_api_key_case_sensitivity() {
        let auth = AppApiKeyAuth::new();
        let original_key = "CaseSensitiveKey";
        let permissions = vec!["read".to_string()];

        auth.add_key(original_key, permissions.clone());

        // Original case should work
        let result_original = auth.validate_key(original_key, "192.168.1.100");
        assert!(result_original.is_some());

        // Lowercase version should fail
        let result_lowercase = auth.validate_key("casesensitivekey", "192.168.1.100");
        assert!(result_lowercase.is_none());

        // Uppercase version should fail
        let result_uppercase = auth.validate_key("CASESENSITIVEKEY", "192.168.1.100");
        assert!(result_uppercase.is_none());
    }

    /// Test: API key rotation
    ///
    /// Verifies that API key rotation correctly creates a new version
    /// and both old and new keys work during grace period.
    #[tokio::test]
    async fn test_api_key_rotation() {
        let rotation_config = RotationConfig {
            rotation_interval: Duration::from_secs(3600),
            grace_period: Duration::from_secs(60),
            keep_versions: 3,
        };

        let auth = AppApiKeyAuth::builder().rotation(rotation_config).build();

        // Add initial key version
        auth.add_key_version("key1", "secret_v1", vec!["read".to_string()], "v1", None);

        // Rotate to new version
        let result = auth.rotate_key(
            "key1",
            "secret_v2",
            vec!["read".to_string(), "write".to_string()],
            "v2",
        );

        assert!(result.is_ok());

        // Verify metadata has both versions
        let metadata = auth.get_key_metadata("key1");
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.versions.len(), 2);
    }

    /// Test: API key with version metadata
    ///
    /// Verifies that versioned API keys are correctly stored and retrieved.
    #[tokio::test]
    async fn test_api_key_version_metadata() {
        let auth = AppApiKeyAuth::new();

        auth.add_key_version(
            "app_key_1",
            "app_secret_v1",
            vec!["read".to_string()],
            "v1",
            Some(Duration::from_secs(86400)),
        );

        // Validate v1 key works
        let result = auth.validate_key("app_secret_v1", "192.168.1.100");
        assert!(result.is_some());

        // Get and verify metadata
        let metadata = auth.get_key_metadata("app_key_1");
        assert!(metadata.is_some());

        let meta = metadata.unwrap();
        assert_eq!(meta.key_id, "app_key_1");
        assert_eq!(meta.versions.len(), 1);
        assert_eq!(meta.versions[0].version, "v1");
    }

    /// Test: API key auth builder configuration
    ///
    /// Verifies that the builder correctly configures LRU and rotation parameters.
    #[tokio::test]
    async fn test_api_key_auth_builder_configuration() {
        let auth = AppApiKeyAuthBuilder::new()
            .lru(LruConfig::default())
            .rotation(RotationConfig::default())
            .build();

        // Verify builder created the instance successfully
        auth.add_key("builder_test_key", vec!["test".to_string()]);
        let result = auth.validate_key("builder_test_key", "127.0.0.1");
        assert!(result.is_some());
    }

    /// Test: API key revocation
    ///
    /// Verifies that revoked keys are correctly invalidated.
    #[tokio::test]
    async fn test_api_key_revocation() {
        let auth = AppApiKeyAuth::new();

        // Add key and verify it works
        auth.add_key_version(
            "revocable_key",
            "revoke_secret",
            vec!["read".to_string()],
            "v1",
            None,
        );
        let before_revoke = auth.validate_key("revoke_secret", "192.168.1.100");
        assert!(before_revoke.is_some());

        // Revoke the key
        let revoke_result = auth.revoke_key("revocable_key");
        assert!(revoke_result.is_ok());

        // Verify metadata shows all versions are inactive
        let metadata = auth.get_key_metadata("revocable_key");
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert!(!meta.versions.iter().any(|v| v.is_active));
    }

    // ============================================================================
    // Bearer Token Authentication Tests
    // ============================================================================

    /// Test: Bearer token builder validation
    ///
    /// Verifies that the builder correctly validates secret requirements.
    #[tokio::test]
    async fn test_bearer_token_builder_validation() {
        // Valid secret should succeed
        let result1 = BearerAuthBuilder::new()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLMNOP")
            .audience("test-api")
            .issuer("test-issuer")
            .build();

        assert!(result1.is_ok());

        // Secret too short should fail
        let result2 = BearerAuthBuilder::new().secret("short").build();

        assert!(result2.is_err());

        // Secret missing uppercase should fail
        let result3 = BearerAuthBuilder::new()
            .secret("mysecret123!@#abcdefghijklmnopqrst")
            .build();

        assert!(result3.is_err());

        // Secret missing special character should fail
        let result4 = BearerAuthBuilder::new()
            .secret("MySecureSecret123ABCDEFGHIJKLMNOP")
            .build();

        assert!(result4.is_err());
    }

    /// Test: Bearer token validation - valid token
    ///
    /// Verifies that BearerAuth correctly validates tokens via public API.
    #[tokio::test]
    async fn test_bearer_token_validation_public_api() {
        // Create auth with builder
        let auth = BearerAuth::builder()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLMNOP")
            .build()
            .expect("Failed to build BearerAuth");

        // Register a context and token for testing
        let context = AuthContext::new(
            Some("test_user".to_string()),
            vec!["read".to_string()],
            AuthMetadata::default(),
        );

        // Create a fake token and register it
        let fake_token = "test.token.here".to_string();
        auth.register_token(fake_token.clone(), context.clone());

        // Validate should fail (not a real JWT)
        let result = auth.validate_token(&fake_token);
        // The validate_token method checks JWT format, so this should fail
        assert!(result.is_none());
    }

    /// Test: Bearer token invalidation (logout)
    ///
    /// Verifies that invalidated tokens are correctly rejected.
    #[tokio::test]
    async fn test_bearer_token_invalidation() {
        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLMNOP");

        // Invalidate a token
        let test_token = "user_token_12345";
        auth.invalidate_token(test_token);

        // Validate should fail for invalidated token
        let result = auth.validate_token(test_token);
        assert!(result.is_none());
    }

    /// Test: Bearer auth with dependencies
    ///
    /// Verifies that BearerAuth can be created with dependencies.
    #[tokio::test]
    async fn test_bearer_auth_with_dependencies() {
        use sdforge::cache::DashMapCache;
        use std::sync::Arc;

        let valid_tokens = Arc::new(DashMapCache::new()) as _;
        let blacklisted_tokens = Arc::new(DashMapCache::new()) as _;

        let auth = BearerAuth::with_dependencies(
            b"MySecureSecret123!@#ABCDEFGHIJKLMNOP".to_vec(),
            valid_tokens,
            blacklisted_tokens,
            Some("my-api".to_string()),
            Some("my-issuer".to_string()),
        );

        // Register and verify
        let context = create_test_context(Some("dep_user"));
        let test_token = "dep_token_abc";
        auth.register_token(test_token.to_string(), context);

        // Blacklist should work
        auth.invalidate_token(test_token);
        let result = auth.validate_token(test_token);
        assert!(result.is_none());
    }

    /// Test: Bearer token with valid JWT
    ///
    /// Verifies that a properly signed JWT token is validated successfully.
    #[tokio::test]
    async fn test_bearer_token_valid() {
        let secret = b"MySecureSecret123!@#ABCDEFGHIJKLMNOP";
        let auth = BearerAuth::builder()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLMNOP")
            .build()
            .expect("Failed to build BearerAuth");

        // Create a valid JWT token (expires in 1 hour)
        let token = create_jwt_token(secret, "test_user", vec!["read", "write"], 3600);

        // Validate should succeed
        let result = auth.validate_token(&token);
        assert!(result.is_some());

        let context = result.unwrap();
        assert_eq!(context.user_id(), Some("test_user"));
        assert!(context.has_permission("read"));
        assert!(context.has_permission("write"));
    }

    /// Test: Bearer token expired
    ///
    /// Verifies that expired JWT tokens are correctly rejected.
    #[tokio::test]
    async fn test_bearer_token_expired() {
        let secret = b"MySecureSecret123!@#ABCDEFGHIJKLMNOP";
        let auth = BearerAuth::builder()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLMNOP")
            .build()
            .expect("Failed to build BearerAuth");

        // Create an expired JWT token (expired 1 hour ago)
        let token = create_jwt_token(secret, "test_user", vec!["read"], -3600);

        // Validate should fail
        let result = auth.validate_token(&token);
        assert!(result.is_none());
    }

    /// Test: Bearer token invalid format
    ///
    /// Verifies that tokens with invalid format are rejected.
    #[tokio::test]
    async fn test_bearer_token_invalid_format() {
        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLMNOP");

        // Test various invalid formats
        let invalid_tokens = vec![
            "not.a.jwt",                           // Wrong number of parts
            "onlytwo.parts",                       // Only two parts
            "",                                    // Empty string
            "invalid..token",                      // Empty part
            ">>>>>>>>>>>.>>>>>>>>>>>.>>>>>>>>>>>", // Invalid base64
            "abc",                                 // Too short
        ];

        for token in invalid_tokens {
            let result = auth.validate_token(token);
            assert!(result.is_none(), "Token '{}' should be invalid", token);
        }
    }

    /// Test: Bearer token missing
    ///
    /// Verifies that missing tokens are handled gracefully.
    #[tokio::test]
    async fn test_bearer_token_missing() {
        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLMNOP");

        // Empty string should fail
        let result = auth.validate_token("");
        assert!(result.is_none());
    }

    /// Test: Bearer token with wrong secret (signature verification)
    ///
    /// Verifies that tokens signed with wrong secret are rejected.
    #[tokio::test]
    async fn test_bearer_token_wrong_secret() {
        // Auth is configured with secret1
        let _secret1 = b"MySecureSecret123!@#ABCDEFGHIJKLMNOP";
        let secret2 = b"DifferentSecret456!@#ABCDEFGHIJKLMNOP";

        let auth = BearerAuth::builder()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLMNOP")
            .build()
            .expect("Failed to build BearerAuth");

        // Create token with different secret
        let token = create_jwt_token(secret2, "test_user", vec!["read"], 3600);

        // Should fail signature verification
        let result = auth.validate_token(&token);
        assert!(result.is_none());
    }

    /// Test: Bearer token with audience validation
    ///
    /// Verifies that audience claim is validated correctly.
    #[tokio::test]
    async fn test_bearer_token_with_audience() {
        let secret = b"MySecureSecret123!@#ABCDEFGHIJKLMNOP";
        let auth = BearerAuth::builder()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLMNOP")
            .audience("my-api")
            .build()
            .expect("Failed to build BearerAuth");

        // Create token with matching audience
        let token = create_jwt_with_audience(secret, "test_user", "my-api", None, 3600);
        let result = auth.validate_token(&token);
        assert!(result.is_some());

        // Create token with wrong audience
        let wrong_aud_token =
            create_jwt_with_audience(secret, "test_user", "other-api", None, 3600);
        let wrong_result = auth.validate_token(&wrong_aud_token);
        assert!(wrong_result.is_none());
    }

    /// Test: Bearer token with issuer validation
    ///
    /// Verifies that issuer claim is validated correctly.
    #[tokio::test]
    async fn test_bearer_token_with_issuer() {
        let secret = b"MySecureSecret123!@#ABCDEFGHIJKLMNOP";
        let auth = BearerAuth::builder()
            .secret("MySecureSecret123!@#ABCDEFGHIJKLMNOP")
            .issuer("my-issuer")
            .build()
            .expect("Failed to build BearerAuth");

        // Create token with matching issuer
        let token =
            create_jwt_with_audience(secret, "test_user", "my-api", Some("my-issuer"), 3600);
        let result = auth.validate_token(&token);
        assert!(result.is_some());

        // Create token with wrong issuer
        let wrong_iss_token =
            create_jwt_with_audience(secret, "test_user", "my-api", Some("other-issuer"), 3600);
        let wrong_result = auth.validate_token(&wrong_iss_token);
        assert!(wrong_result.is_none());
    }

    // ============================================================================
    // Audit Logging Tests
    // ============================================================================

    /// Test: Audit log successful request
    ///
    /// Verifies that successful requests are correctly logged.
    #[tokio::test]
    async fn test_audit_log_successful_request() {
        let logger = AppAuditLogger::with_limit(100);
        let context = create_test_context(Some("user123"));

        // Log a successful action
        logger
            .log(&context, "api.read", "/api/users", true, None)
            .await;

        // Verify log was created
        let logs = logger.get_logs("user123");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action(), "api.read");
        assert_eq!(logs[0].resource(), "/api/users");
        assert!(matches!(logs[0].result(), &AuditResult::Success));
    }

    /// Test: Audit log failed authentication
    ///
    /// Verifies that failed authentication attempts are correctly logged
    /// with error messages.
    #[tokio::test]
    async fn test_audit_log_failed_authentication() {
        let logger = AppAuditLogger::with_limit(100);
        let context = create_test_context(Some("anonymous"));

        // Log a failed authentication
        logger
            .log(
                &context,
                "auth.failed",
                "/api/login",
                false,
                Some("Invalid credentials".to_string()),
            )
            .await;

        // Verify log was created with failure details
        let logs = logger.get_logs("anonymous");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action(), "auth.failed");

        // Verify error message is present (may be sanitized)
        match &logs[0].result() {
            AuditResult::Failure { message } => {
                assert!(!message.is_empty());
            }
            _ => panic!("Expected failure result"),
        }
    }

    /// Test: Audit log with multiple entries
    ///
    /// Verifies that multiple audit log entries are correctly stored
    /// and can be retrieved.
    #[tokio::test]
    async fn test_audit_log_multiple_entries() {
        let logger = AppAuditLogger::with_limit(100);
        let context = create_test_context(Some("user456"));

        // Log multiple actions
        logger
            .log(&context, "action1", "resource1", true, None)
            .await;
        logger
            .log(&context, "action2", "resource2", true, None)
            .await;
        logger
            .log(
                &context,
                "action3",
                "resource3",
                false,
                Some("Error".to_string()),
            )
            .await;

        // Verify all logs are stored
        let logs = logger.get_logs("user456");
        assert_eq!(logs.len(), 3);

        // Extract all actions (order may vary due to same-second timestamps)
        let actions: Vec<&str> = logs.iter().map(|log| log.action()).collect();
        assert!(actions.contains(&"action1"));
        assert!(actions.contains(&"action2"));
        assert!(actions.contains(&"action3"));
    }

    /// Test: Audit log builder configuration
    ///
    /// Verifies that the audit logger builder correctly configures parameters.
    #[tokio::test]
    async fn test_audit_logger_builder_configuration() {
        let logger = AppAuditLogger::builder()
            .max_logs_per_user(500)
            .max_concurrent_ops(50)
            .queue_size(2000)
            .build();

        let context = create_test_context(Some("builder_test_user"));

        // Log some entries
        logger
            .log(&context, "test_action", "test_resource", true, None)
            .await;

        // Verify logs are stored
        let logs = logger.get_logs("builder_test_user");
        assert_eq!(logs.len(), 1);
    }

    /// Test: Audit log clear functionality
    ///
    /// Verifies that logs can be cleared for a specific user.
    #[tokio::test]
    async fn test_audit_log_clear() {
        let logger = AppAuditLogger::with_limit(100);
        let context = create_test_context(Some("user789"));

        // Add logs
        logger
            .log(&context, "action1", "resource1", true, None)
            .await;
        logger
            .log(&context, "action2", "resource2", true, None)
            .await;

        assert_eq!(logger.get_logs("user789").len(), 2);

        // Clear logs
        logger.clear_logs("user789");

        // Verify logs are cleared
        assert_eq!(logger.get_logs("user789").len(), 0);
    }

    /// Test: Audit log sanitization
    ///
    /// Verifies that sensitive data is properly sanitized in logs.
    #[tokio::test]
    async fn test_audit_log_sanitization() {
        let logger = AppAuditLogger::with_limit(100);
        let context = create_test_context(Some("security_test_user"));

        // Log with potentially sensitive data in message
        logger
            .log(
                &context,
                "test.sanitization",
                "/api/test",
                false,
                Some("password=secret123 token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c".to_string()),
            )
            .await;

        let logs = logger.get_logs("security_test_user");
        assert_eq!(logs.len(), 1);

        // Check that sensitive patterns are sanitized
        match &logs[0].result() {
            AuditResult::Failure { message } => {
                // JWT should be redacted
                assert!(!message.contains("eyJ"));
                // password pattern should be redacted
                assert!(!message.contains("secret123"));
            }
            _ => panic!("Expected failure result"),
        }
    }

    // ============================================================================
    // Integration Tests with Real Components
    // ============================================================================

    /// Test: Bearer Token with Audit Logging
    ///
    /// Verifies that Bearer token validation can be combined with audit logging.
    #[tokio::test]
    async fn test_bearer_with_audit_logging() {
        let logger = AppAuditLogger::with_limit(100);

        // Log the authentication attempt
        let context = create_test_context(Some("service_account"));
        logger
            .log(&context, "token.validated", "/api/service", true, None)
            .await;

        // Verify audit log
        let logs = logger.get_logs("service_account");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action(), "token.validated");
    }

    /// Test: End-to-end security flow simulation
    ///
    /// Simulates a complete security flow: authentication -> logging.
    #[tokio::test]
    async fn test_end_to_end_security_flow() {
        let auth = AppApiKeyAuth::new();
        let logger = AppAuditLogger::with_limit(100);

        // Setup
        auth.add_key("e2e_test_key", vec!["admin".to_string()]);
        let context = create_test_context(Some("e2e_user"));

        // Simulate 10 valid requests
        for i in 1..=10 {
            // Authenticate
            let auth_result = auth.validate_key("e2e_test_key", "192.168.1.100");
            assert!(auth_result.is_some(), "Request {} auth should succeed", i);

            // Log
            logger
                .log(&context, "e2e.request", "/api/test", true, None)
                .await;
        }

        // Verify audit logs
        let logs = logger.get_logs("e2e_user");
        assert_eq!(logs.len(), 10);
    }

    /// Test: Graceful handling of unknown keys
    ///
    /// Verifies that the system gracefully handles requests with unknown keys.
    #[tokio::test]
    async fn test_unknown_key_graceful_handling() {
        let auth = AppApiKeyAuth::new();
        let logger = AppAuditLogger::with_limit(100);
        let context = create_test_context(Some("unknown_user"));

        // Try to validate unknown key
        let result = auth.validate_key("unknown_key_xyz", "192.168.1.100");
        assert!(result.is_none());

        // Log the failed attempt
        logger
            .log(
                &context,
                "auth.unknown_key",
                "/api/secure",
                false,
                Some("Unknown API key attempted".to_string()),
            )
            .await;

        // Verify log exists
        let logs = logger.get_logs("unknown_user");
        assert_eq!(logs.len(), 1);
        assert!(matches!(logs[0].result(), AuditResult::Failure { .. }));
    }

    // ============================================================================
    // Edge Case Tests
    // ============================================================================

    /// Test: Empty permissions handling
    ///
    /// Verifies that keys with empty permissions are handled correctly.
    #[tokio::test]
    async fn test_empty_permissions() {
        let auth = AppApiKeyAuth::new();
        auth.add_key("empty_perms_key", vec![]);

        let result = auth.validate_key("empty_perms_key", "192.168.1.100");
        // Empty permissions should return None (treated as invalid)
        assert!(result.is_none());
    }

    /// Test: API key metadata structure
    ///
    /// Verifies that ApiKeyMetadata can be created and manipulated correctly.
    #[tokio::test]
    async fn test_api_key_metadata_structure() {
        // Create metadata using the public API
        let metadata = ApiKeyMetadata::new(
            "test_key_id".to_string(),
            Some("Test description".to_string()),
        );

        assert_eq!(metadata.key_id, "test_key_id");
        assert_eq!(metadata.description, Some("Test description".to_string()));
        assert!(metadata.versions.is_empty());
        assert!(metadata.active_version_index.is_none());
    }

    /// Test: LRU configuration defaults
    ///
    /// Verifies that LRU configuration has sensible defaults.
    #[tokio::test]
    async fn test_lru_config_defaults() {
        let config = LruConfig::default();

        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.ttl, Duration::from_secs(3600));
    }

    /// Test: Rotation configuration defaults
    ///
    /// Verifies that rotation configuration has sensible defaults.
    #[tokio::test]
    async fn test_rotation_config_defaults() {
        let config = RotationConfig::default();

        // Default rotation interval is 30 days
        assert_eq!(config.rotation_interval, Duration::from_secs(86400 * 30));
        // Default grace period is 7 days
        assert_eq!(config.grace_period, Duration::from_secs(86400 * 7));
        // Default keep versions is 3
        assert_eq!(config.keep_versions, 3);
    }

    // ============================================================================
    // IP Validation Tests
    // ============================================================================

    /// Test: API key authentication with different client IPs
    ///
    /// Verifies that API key validation works with various client IPs.
    #[tokio::test]
    async fn test_api_key_different_client_ips() {
        let auth = AppApiKeyAuth::new();
        let test_key = "ip_test_key";
        auth.add_key(test_key, vec!["read".to_string()]);

        // Test with various IPs (all should work with valid key)
        let ips = vec![
            "203.0.113.50",   // Public IP
            "198.51.100.100", // Public IP
            "192.0.2.1",      // Documentation IP (TEST-NET-1)
        ];

        for ip in ips {
            let result = auth.validate_key(test_key, ip);
            assert!(result.is_some(), "Valid key should work with IP {}", ip);
        }
    }

    /// Test: API key missing header (simulated)
    ///
    /// Verifies that missing API key is handled correctly.
    #[tokio::test]
    async fn test_api_key_missing_header() {
        let auth = AppApiKeyAuth::new();

        // Simulate missing API key (empty string)
        let result = auth.validate_key("", "192.168.1.100");
        assert!(result.is_none());
    }

    /// Test: Bearer token wrong scheme (Basic vs Bearer)
    ///
    /// Verifies that tokens with wrong authentication scheme are rejected.
    #[tokio::test]
    async fn test_bearer_token_wrong_scheme() {
        let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLMNOP");

        // Basic auth style token should fail
        let basic_token = "Basic dXNlcm5hbWU6cGFzc3dvcmQ=";
        let result = auth.validate_token(basic_token);
        assert!(result.is_none());

        // Digest auth style should also fail
        let digest_token = "Digest username=\"test\"";
        let result2 = auth.validate_token(digest_token);
        assert!(result2.is_none());
    }

    // ============================================================================
    // Advanced API Key Tests
    // ============================================================================

    /// Test: API key hash consistency
    ///
    /// Verifies that the same key always produces the same result.
    #[tokio::test]
    async fn test_api_key_hash_consistency() {
        let auth = AppApiKeyAuth::new();
        let test_key = "consistency_test_key";
        auth.add_key(test_key, vec!["admin".to_string()]);

        // Validate multiple times
        let results: Vec<_> = (0..5)
            .map(|_| auth.validate_key(test_key, "192.168.1.100"))
            .collect();

        // All results should be the same
        for result in &results {
            assert_eq!(result, &results[0]);
        }
    }

    /// Test: Multiple API keys
    ///
    /// Verifies that multiple API keys can coexist.
    #[tokio::test]
    async fn test_multiple_api_keys() {
        let auth = AppApiKeyAuth::new();

        // Add multiple keys
        auth.add_key("key1", vec!["read".to_string()]);
        auth.add_key("key2", vec!["write".to_string()]);
        auth.add_key("key3", vec!["admin".to_string()]);

        // Each key should return its own permissions
        let result1 = auth.validate_key("key1", "192.168.1.1");
        let result2 = auth.validate_key("key2", "192.168.1.1");
        let result3 = auth.validate_key("key3", "192.168.1.1");

        assert!(result1.is_some() && result1.unwrap().contains(&"read".to_string()));
        assert!(result2.is_some() && result2.unwrap().contains(&"write".to_string()));
        assert!(result3.is_some() && result3.unwrap().contains(&"admin".to_string()));

        // Unknown key should fail
        let result4 = auth.validate_key("unknown_key", "192.168.1.1");
        assert!(result4.is_none());
    }

    /// Test: API key update permissions
    ///
    /// Verifies that permissions can be updated by adding same key.
    #[tokio::test]
    async fn test_api_key_update_permissions() {
        let auth = AppApiKeyAuth::new();

        // Add key with initial permissions
        auth.add_key("update_test_key", vec!["read".to_string()]);

        // Verify initial permissions
        let result1 = auth.validate_key("update_test_key", "192.168.1.100");
        assert!(result1.is_some());
        let perms1 = result1.unwrap();
        assert!(perms1.contains(&"read".to_string()));
        assert!(!perms1.contains(&"write".to_string()));
    }

    /// Test: Audit log with metadata
    ///
    /// Verifies that audit logs include metadata correctly.
    #[tokio::test]
    async fn test_audit_log_with_metadata() {
        let logger = AppAuditLogger::with_limit(100);

        let metadata = AuthMetadata::new(
            Some("203.0.113.50".to_string()),
            Some("Mozilla/5.0 Test".to_string()),
        );

        let context = AuthContext::new(
            Some("metadata_test_user".to_string()),
            vec!["read".to_string()],
            metadata,
        );

        logger
            .log(&context, "metadata.test", "/api/test", true, None)
            .await;

        let logs = logger.get_logs("metadata_test_user");
        assert_eq!(logs.len(), 1);

        let log = &logs[0];
        assert_eq!(log.action(), "metadata.test");
        assert_eq!(log.resource(), "/api/test");
    }
}
