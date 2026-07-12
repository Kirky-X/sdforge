// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for core audit logger functionality.

use super::super::*;
use super::make_test_audit_log;

#[test]
fn test_sanitize_error_message() {
    // JWT must have three parts (header.payload.signature) to match the pattern
    let message = "Error with JWT: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let sanitized = sanitize_error_message(message);
    assert!(!sanitized.contains("eyJ"));
    assert!(sanitized.contains("[REDACTED_JWT]"));
}

#[test]
fn test_sanitize_error_message_removes_secrets() {
    // Test secret pattern removal
    let message = "Failed with password=secret123 and token: abc456";
    let sanitized = sanitize_error_message(message);
    assert!(sanitized.contains("password=[REDACTED]"));
    assert!(sanitized.contains("token=[REDACTED]"));
}

#[test]
fn test_sanitize_error_message_removes_api_keys() {
    // Test API key removal (20+ characters)
    // Note: API keys are caught by the secret pattern first, then by API key pattern.
    // Uses a clearly fake test key to avoid triggering secret scanners.
    let message = "API key: testapikey1234567890abcdefghij";
    let sanitized = sanitize_error_message(message);
    // The key should be redacted (either as [REDACTED] or [REDACTED_API_KEY])
    assert!(
        !sanitized.contains("testapikey1234567890"),
        "API key should be redacted, got: {}",
        sanitized
    );
}

#[test]
fn test_sanitize_error_message_removes_credit_cards() {
    // Test credit card number removal
    let message = "Card number: 1234-5678-9012-3456";
    let sanitized = sanitize_error_message(message);
    assert!(sanitized.contains("[REDACTED_CREDIT_CARD]"));
}

#[test]
fn test_sanitize_error_message_removes_ssn() {
    // Test SSN removal
    let message = "SSN: 123-45-6789";
    let sanitized = sanitize_error_message(message);
    assert!(sanitized.contains("[REDACTED_SSN]"));
}

#[test]
fn test_sanitize_error_message_removes_certificate_paths() {
    // Test certificate path removal
    let message = "Cannot load /etc/ssl/certs/server.pem";
    let sanitized = sanitize_error_message(message);
    assert!(sanitized.contains("[REDACTED_PATH]"));
}

#[test]
fn test_sanitize_error_message_truncation() {
    // Test truncation of long messages
    let long_message = "x".repeat(1000);
    let sanitized = sanitize_error_message(&long_message);
    assert!(sanitized.len() <= 520); // 500 + "...[TRUNCATED]"
    assert!(sanitized.ends_with("...[TRUNCATED]"));
}

#[test]
fn test_global_regex_patterns_are_initialized() {
    // Verify global regex patterns are properly initialized
    assert!(JWT_PATTERN.is_match("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxw"));
    assert!(SECRET_PATTERN.is_match("password=secret123"));
    assert!(PATH_PATTERN.is_match("/path/to/cert.pem"));
}

#[tokio::test]
async fn test_audit_logger() {
    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: Some("test_user".to_string()),
        permissions: vec![],
        metadata: crate::security::AuthMetadata::default(),
    };

    logger
        .log(&context, "test_action", "test_resource", true, None)
        .await;

    let logs = logger.get_logs("test_user");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].action(), "test_action");
}

#[tokio::test]
async fn test_get_logs_empty() {
    let logger = AppAuditLogger::new();
    let logs = logger.get_logs("nonexistent_user");
    assert_eq!(logs.len(), 0);
}

#[tokio::test]
async fn test_clear_logs() {
    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: Some("test_user".to_string()),
        permissions: vec![],
        metadata: crate::security::AuthMetadata::default(),
    };

    logger
        .log(&context, "test_action", "test_resource", true, None)
        .await;
    assert_eq!(logger.get_logs("test_user").len(), 1);

    logger.clear_logs("test_user");
    assert_eq!(logger.get_logs("test_user").len(), 0);
}

// ============================================================================
// AppAuditLogger::log_key_rotation Tests
// ============================================================================

#[tokio::test]
async fn test_log_key_rotation_success() {
    let logger = AppAuditLogger::with_limit(10);
    logger
        .log_key_rotation("key-123", "v1", "v2", true, None)
        .await;

    let logs = logger.get_logs("system");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].action(), "key_rotation");
    assert_eq!(logs[0].resource(), "api_key");
}

#[tokio::test]
async fn test_log_key_rotation_failure() {
    let logger = AppAuditLogger::with_limit(10);
    logger
        .log_key_rotation(
            "key-456",
            "v1",
            "v2",
            false,
            Some("Rotation failed".to_string()),
        )
        .await;

    let logs = logger.get_logs("system");
    assert_eq!(logs.len(), 1);
    assert!(matches!(logs[0].result(), AuditResult::Failure { .. }));
}

// ============================================================================
// AppAuditLogger::total_log_count Tests
// ============================================================================

#[tokio::test]
async fn test_total_log_count_returns_zero() {
    let logger = AppAuditLogger::new();
    assert_eq!(logger.total_log_count(), 0);
}

#[tokio::test]
async fn test_total_log_count_after_logging() {
    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: Some("test_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };
    logger
        .log(&context, "action1", "resource1", true, None)
        .await;
    // total_log_count now returns the actual count (previously always 0).
    assert_eq!(logger.total_log_count(), 1);
}

// ============================================================================
// AppAuditLogger::dropped_log_count Tests
// ============================================================================

#[tokio::test]
async fn test_dropped_log_count_initial_value() {
    let logger = AppAuditLogger::new();
    assert_eq!(logger.dropped_log_count(), 0);
}

#[tokio::test]
async fn test_dropped_log_count_after_logging() {
    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: Some("test_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };
    logger.log(&context, "action", "resource", true, None).await;
    // Channel is not full, so dropped count should still be 0
    assert_eq!(logger.dropped_log_count(), 0);
}

// ============================================================================
// AppAuditLogger::log() with Failure + Error Sanitization Tests
// ============================================================================

#[tokio::test]
async fn test_log_failure_with_message() {
    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: Some("test_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };

    logger
        .log(
            &context,
            "delete_resource",
            "/api/resource/123",
            false,
            Some("Permission denied".to_string()),
        )
        .await;

    let logs = logger.get_logs("test_user");
    assert_eq!(logs.len(), 1);
    match logs[0].result() {
        AuditResult::Failure { message } => {
            assert_eq!(message, "Permission denied");
        }
        _ => panic!("Expected Failure result"),
    }
}

#[tokio::test]
async fn test_log_failure_sanitizes_sensitive_data() {
    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: Some("test_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };

    // Error message with password should be sanitized
    logger
        .log(
            &context,
            "auth",
            "/api/login",
            false,
            Some("Failed with password=secret123".to_string()),
        )
        .await;

    let logs = logger.get_logs("test_user");
    assert_eq!(logs.len(), 1);
    match logs[0].result() {
        AuditResult::Failure { message } => {
            assert!(
                !message.contains("secret123"),
                "Password should be sanitized"
            );
            assert!(
                message.contains("[REDACTED]"),
                "Should contain redacted marker"
            );
        }
        _ => panic!("Expected Failure result"),
    }
}

#[tokio::test]
async fn test_log_failure_default_message() {
    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: Some("test_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };

    logger
        .log(&context, "action", "resource", false, None)
        .await;

    let logs = logger.get_logs("test_user");
    match logs[0].result() {
        AuditResult::Failure { message } => {
            assert_eq!(message, "Unknown error");
        }
        _ => panic!("Expected Failure result"),
    }
}

// ============================================================================
// AuditLogger Trait Implementation Tests
// ============================================================================

#[test]
fn test_audit_logger_trait_log() {
    use crate::security::AuditLogger;

    // Use a runtime to ensure tokio tasks are properly scheduled
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let logger = AppAuditLogger::with_limit(10);
        let log = AuditLog {
            id: "trait-test-id".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            user_id: Some("trait_user".to_string()),
            action: "trait_action".to_string(),
            resource: "trait_resource".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        };

        // Call the trait method - it spawns a background thread
        AuditLogger::log(&logger, log);

        // Give the background thread time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let logs = logger.get_logs("trait_user");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action(), "trait_action");
    });
}

#[test]
fn test_audit_logger_trait_log_failure() {
    use crate::security::AuditLogger;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let logger = AppAuditLogger::with_limit(10);
        let log = AuditLog {
            id: "trait-fail-id".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            user_id: Some("trait_fail_user".to_string()),
            action: "trait_fail_action".to_string(),
            resource: "trait_fail_resource".to_string(),
            result: AuditResult::Failure {
                message: "Trait test failure".to_string(),
            },
            metadata: AuthMetadata::default(),
            signature: None,
        };

        AuditLogger::log(&logger, log);

        // Give the background thread time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let logs = logger.get_logs("trait_fail_user");
        assert_eq!(logs.len(), 1);
        match logs[0].result() {
            AuditResult::Failure { message } => {
                assert_eq!(message, "Trait test failure");
            }
            _ => panic!("Expected Failure result"),
        }
    });
}

// ============================================================================
// Multiple Log Entries and Truncation Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_log_entries_per_user() {
    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: Some("multi_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };

    for i in 0..5 {
        logger
            .log(
                &context,
                format!("action_{}", i),
                format!("resource_{}", i),
                true,
                None,
            )
            .await;
    }

    let logs = logger.get_logs("multi_user");
    assert_eq!(logs.len(), 5);
}

#[tokio::test]
async fn test_log_truncation_when_exceeding_limit() {
    let logger = AppAuditLogger::with_limit(3);
    let context = AuthContext {
        user_id: Some("trunc_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };

    for i in 0..5 {
        logger
            .log(
                &context,
                format!("action_{}", i),
                format!("resource_{}", i),
                true,
                None,
            )
            .await;
    }

    let logs = logger.get_logs("trunc_user");
    // Should be truncated to max_logs_per_user (3)
    assert_eq!(logs.len(), 3);
}

#[tokio::test]
async fn test_multiple_users_independent_logs() {
    let logger = AppAuditLogger::with_limit(10);

    let context1 = AuthContext {
        user_id: Some("user_a".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };
    let context2 = AuthContext {
        user_id: Some("user_b".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };

    logger
        .log(&context1, "action_a", "resource_a", true, None)
        .await;
    logger
        .log(&context2, "action_b", "resource_b", true, None)
        .await;

    let logs_a = logger.get_logs("user_a");
    let logs_b = logger.get_logs("user_b");

    assert_eq!(logs_a.len(), 1);
    assert_eq!(logs_a[0].action(), "action_a");
    assert_eq!(logs_b.len(), 1);
    assert_eq!(logs_b[0].action(), "action_b");
}

#[tokio::test]
async fn test_log_with_anonymous_user() {
    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: None,
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };

    logger
        .log(&context, "anonymous_action", "resource", true, None)
        .await;

    let logs = logger.get_logs("anonymous");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].action(), "anonymous_action");
}

// ============================================================================
// Additional Boundary Tests for sanitize_error_message
// ============================================================================

#[test]
fn test_sanitize_jwt_token() {
    let message = "Authentication failed for JWT: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let sanitized = sanitize_error_message(message);
    assert!(sanitized.contains("[REDACTED_JWT]"));
    assert!(!sanitized.contains("eyJ"));
}

#[test]
fn test_sanitize_password_in_error() {
    let message = "Connection failed: password=my_secret_123";
    let sanitized = sanitize_error_message(message);
    assert!(sanitized.contains("password=[REDACTED]"));
    assert!(!sanitized.contains("my_secret_123"));
}

#[test]
fn test_sanitize_secret_in_json() {
    let message = r#"Error: password="secret123" token="abc456""#;
    let sanitized = sanitize_error_message(message);
    assert!(
        !sanitized.contains("secret123"),
        "Password should be redacted, got: {}",
        sanitized
    );
    assert!(
        !sanitized.contains("abc456"),
        "Token should be redacted, got: {}",
        sanitized
    );
}

#[test]
fn test_sanitize_certificate_path() {
    let message = "Failed to load certificate from /etc/ssl/certs/server.pem";
    let sanitized = sanitize_error_message(message);
    assert!(sanitized.contains("[REDACTED_PATH]"));
    assert!(!sanitized.contains("server.pem"));
}

#[test]
fn test_sanitize_multiple_secrets() {
    let message = "Auth failed with password=secret123 and token: abcdef123456 and api_key: testapikey1234567890abcdefghij";
    let sanitized = sanitize_error_message(message);
    assert!(sanitized.contains("password=[REDACTED]"));
    assert!(sanitized.contains("token=[REDACTED]"));
    assert!(!sanitized.contains("secret123"));
    assert!(!sanitized.contains("abcdef123456"));
}

#[test]
fn test_sanitize_no_secrets() {
    let message = "Database connection timeout after 30 seconds";
    let sanitized = sanitize_error_message(message);
    assert_eq!(sanitized, message);
}

#[test]
fn test_sanitize_empty_message() {
    let message = "";
    let sanitized = sanitize_error_message(message);
    assert_eq!(sanitized, "");
}

#[test]
fn test_sanitize_bearer_token() {
    let message = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0In0.signature validation failed";
    let sanitized = sanitize_error_message(message);
    assert!(sanitized.contains("[REDACTED_JWT]"));
    assert!(!sanitized.contains("eyJ"));
}

// ============================================================================
// AuditLog Structure Tests
// ============================================================================

#[test]
fn test_audit_log_creation() {
    let log = AuditLog {
        id: "test-id-123".to_string(),
        timestamp: 1234567890,
        user_id: Some("user_123".to_string()),
        action: "login".to_string(),
        resource: "/auth/login".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata::default(),
        signature: None,
    };

    assert_eq!(log.id(), "test-id-123");
    assert_eq!(log.timestamp(), 1234567890);
    assert_eq!(log.user_id(), Some("user_123"));
    assert_eq!(log.action(), "login");
    assert_eq!(log.resource(), "/auth/login");
    assert!(matches!(log.result(), AuditResult::Success));
}

#[test]
fn test_audit_log_serialization() {
    let log = AuditLog {
        id: "test-id-456".to_string(),
        timestamp: 1234567890,
        user_id: Some("user_456".to_string()),
        action: "logout".to_string(),
        resource: "/auth/logout".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata::default(),
        signature: None,
    };

    let json = serde_json::to_string(&log).unwrap();
    assert!(json.contains("\"id\":\"test-id-456\""));
    assert!(json.contains("\"action\":\"logout\""));
    assert!(json.contains("\"status\":\"success\""));
}

#[test]
fn test_audit_log_deserialization() {
    let json = r#"[{
        "id": "test-id-789",
        "timestamp": 1234567890,
        "user_id": "user_789",
        "action": "delete",
        "resource": "/api/resource/123",
        "result": {"status": "failure", "message": "Permission denied"},
        "metadata": {
            "client_ip": "192.168.1.1",
            "user_agent": "Mozilla/5.0",
            "request_id": "req-123",
            "timestamp": 1234567890
        },
        "signature": null
    }]"#;

    let logs = deserialize_audit_logs(json.as_bytes());
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].id(), "test-id-789");
    assert_eq!(logs[0].action(), "delete");
    match logs[0].result() {
        AuditResult::Failure { message } => {
            assert_eq!(message, "Permission denied");
        }
        _ => panic!("Expected Failure result"),
    }
}

#[test]
fn test_audit_log_serialization_roundtrip() {
    let original = AuditLog {
        id: "test-id-roundtrip".to_string(),
        timestamp: 1234567890,
        user_id: Some("user_roundtrip".to_string()),
        action: "update".to_string(),
        resource: "/api/resource/456".to_string(),
        result: AuditResult::Failure {
            message: "Validation failed".to_string(),
        },
        metadata: AuthMetadata {
            client_ip: Some("10.0.0.1".to_string()),
            user_agent: Some("TestClient/1.0".to_string()),
            request_id: "req-456".to_string(),
            timestamp: 1234567890,
        },
        signature: Some("test-signature".to_string()),
    };

    let serialized = serialize_audit_logs(std::slice::from_ref(&original));
    let deserialized = deserialize_audit_logs(&serialized);

    assert_eq!(deserialized.len(), 1);
    let log = &deserialized[0];
    assert_eq!(log.id(), original.id());
    assert_eq!(log.timestamp(), original.timestamp());
    assert_eq!(log.user_id(), original.user_id());
    assert_eq!(log.action(), original.action());
    assert_eq!(log.resource(), original.resource());
    assert_eq!(log.metadata().request_id, original.metadata().request_id);
}

// ============================================================================
// Edge Case Tests for sanitize_error_message
// ============================================================================

#[test]
fn test_sanitize_very_long_jwt_token() {
    let long_payload = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".to_string()
        + ".eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ"
        + &"A".repeat(600)
        + ".SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    let message = format!("Auth error: {}", long_payload);
    let sanitized = sanitize_error_message(&message);

    assert!(sanitized.contains("[REDACTED_JWT]") || sanitized.contains("...[TRUNCATED]"));
    assert!(!sanitized.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
}

#[test]
fn test_sanitize_token_in_url() {
    let message = "Request failed: https://api.example.com/data?token=secret_token_123&user=test";
    let sanitized = sanitize_error_message(message);
    assert!(sanitized.contains("token=[REDACTED]"));
    assert!(!sanitized.contains("secret_token_123"));
}

// ============================================================================
// Signing key environment variable tests
//
// The log() method checks SDFORGE_AUDIT_SIGNING_KEY env var to optionally
// sign audit logs. These tests use #[serial] to safely set/unset the env
// var without interfering with other tests.
// ============================================================================

#[tokio::test]
#[serial_test::serial]
async fn test_log_with_signing_key_generates_signature() {
    // Set a non-empty signing key — log.generate_signature should be called
    // and the resulting log should have a non-None signature.
    unsafe {
        std::env::set_var(
            "SDFORGE_AUDIT_SIGNING_KEY",
            "test_signing_key_min_32_bytes_long!!!",
        );
    }

    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: Some("signing_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };

    logger
        .log(&context, "signed_action", "resource", true, None)
        .await;

    let logs = logger.get_logs("signing_user");
    assert_eq!(logs.len(), 1);
    assert!(
        logs[0].signature.is_some(),
        "Audit log should have a signature when signing key is set"
    );

    // Clean up
    unsafe {
        std::env::remove_var("SDFORGE_AUDIT_SIGNING_KEY");
    }
}

#[tokio::test]
#[serial_test::serial]
async fn test_log_with_empty_signing_key_warns() {
    // Set an empty signing key — the empty-key warning branch should be
    // taken and the log should NOT have a signature.
    unsafe {
        std::env::set_var("SDFORGE_AUDIT_SIGNING_KEY", "");
    }

    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: Some("empty_key_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };

    logger
        .log(&context, "unsigned_action", "resource", true, None)
        .await;

    let logs = logger.get_logs("empty_key_user");
    assert_eq!(logs.len(), 1);
    assert!(
        logs[0].signature.is_none(),
        "Audit log should NOT have a signature when signing key is empty"
    );

    // Clean up
    unsafe {
        std::env::remove_var("SDFORGE_AUDIT_SIGNING_KEY");
    }
}

#[tokio::test]
#[serial_test::serial]
async fn test_log_without_signing_key_warns() {
    // Ensure the env var is NOT set — the "not set" warning branch should
    // be taken and the log should NOT have a signature.
    unsafe {
        std::env::remove_var("SDFORGE_AUDIT_SIGNING_KEY");
    }

    let logger = AppAuditLogger::with_limit(10);
    let context = AuthContext {
        user_id: Some("no_key_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };

    logger
        .log(&context, "unsigned_action", "resource", true, None)
        .await;

    let logs = logger.get_logs("no_key_user");
    assert_eq!(logs.len(), 1);
    assert!(
        logs[0].signature.is_none(),
        "Audit log should NOT have a signature when signing key is not set"
    );
}

// ============================================================================
// log() fallback merge tests
//
// log() synchronously merges fallback logs into primary storage before
// sending to the queue. These tests manually populate fallback_logs to
// exercise that merge path (lines 324-333).
// ============================================================================

#[tokio::test]
async fn test_log_merges_fallback_logs_synchronously() {
    let logger = AppAuditLogger::with_limit(100);

    // Manually populate fallback_logs with a serialized AuditLog
    let fallback_log = AuditLog {
        id: "fallback-log-1".to_string(),
        timestamp: chrono::Utc::now().timestamp() - 100,
        user_id: Some("merge_user".to_string()),
        action: "fallback_action".to_string(),
        resource: "fallback_resource".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata::default(),
        signature: None,
    };
    logger.fallback_logs.set(
        "merge_user",
        serialize_audit_logs(std::slice::from_ref(&fallback_log)),
    );

    // Verify fallback was stored
    assert!(logger.fallback_logs.get("merge_user").is_some());

    // Now call log() for the same user — this should merge the fallback
    // into primary storage and delete the fallback.
    let context = AuthContext {
        user_id: Some("merge_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };
    logger
        .log(&context, "new_action", "new_resource", true, None)
        .await;

    // The fallback should have been merged and deleted
    assert!(
        logger.fallback_logs.get("merge_user").is_none(),
        "Fallback logs should be deleted after merge"
    );

    // Primary storage should contain both the fallback log and the new log
    let logs = logger.get_logs("merge_user");
    assert_eq!(logs.len(), 2, "Should have 2 logs after merge");

    // Verify both logs are present
    let actions: Vec<&str> = logs.iter().map(|l| l.action()).collect();
    assert!(
        actions.contains(&"fallback_action"),
        "Fallback action should be present"
    );
    assert!(
        actions.contains(&"new_action"),
        "New action should be present"
    );
}

#[tokio::test]
async fn test_log_fallback_merge_respects_max_logs() {
    // When merging fallback + primary exceeds max_logs_per_user, the
    // merged result should be truncated.
    let logger = AppAuditLogger::with_limit(2); // Very small limit

    // Populate primary with 1 log
    let primary_log = AuditLog {
        id: "primary-1".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        user_id: Some("trunc_user".to_string()),
        action: "primary_action".to_string(),
        resource: "res".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata::default(),
        signature: None,
    };
    logger
        .logs
        .set("trunc_user", serialize_audit_logs(&[primary_log]));

    // Populate fallback with 2 logs
    let fallback_logs = vec![
        AuditLog {
            id: "fallback-1".to_string(),
            timestamp: chrono::Utc::now().timestamp() - 200,
            user_id: Some("trunc_user".to_string()),
            action: "fb1".to_string(),
            resource: "res".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        },
        AuditLog {
            id: "fallback-2".to_string(),
            timestamp: chrono::Utc::now().timestamp() - 100,
            user_id: Some("trunc_user".to_string()),
            action: "fb2".to_string(),
            resource: "res".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        },
    ];
    logger
        .fallback_logs
        .set("trunc_user", serialize_audit_logs(&fallback_logs));

    // Call log() which merges fallback into primary
    let context = AuthContext {
        user_id: Some("trunc_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };
    logger.log(&context, "new", "res", true, None).await;

    // After merge + truncate, should have at most max_logs_per_user (2)
    let logs = logger.get_logs("trunc_user");
    assert!(
        logs.len() <= 2,
        "Logs should be truncated to max_logs_per_user (2), got {}",
        logs.len()
    );
}

// ============================================================================
// get_logs dedup tests
//
// get_logs merges primary and fallback, deduplicating by log ID.
// These tests populate both stores with overlapping IDs (lines 386-389).
// ============================================================================

#[tokio::test]
async fn test_get_logs_deduplicates_by_id() {
    let logger = AppAuditLogger::with_limit(100);

    // Create a log that appears in BOTH primary and fallback (same ID)
    let shared_log = AuditLog {
        id: "shared-id-1".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        user_id: Some("dedup_user".to_string()),
        action: "shared_action".to_string(),
        resource: "res".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata::default(),
        signature: None,
    };

    // A log that only appears in fallback
    let fallback_only = AuditLog {
        id: "fallback-only-1".to_string(),
        timestamp: chrono::Utc::now().timestamp() - 50,
        user_id: Some("dedup_user".to_string()),
        action: "fallback_only_action".to_string(),
        resource: "res".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata::default(),
        signature: None,
    };

    // Populate primary with the shared log
    logger.logs.set(
        "dedup_user",
        serialize_audit_logs(std::slice::from_ref(&shared_log)),
    );

    // Populate fallback with both the shared log and the fallback-only log
    logger.fallback_logs.set(
        "dedup_user",
        serialize_audit_logs(&[shared_log, fallback_only]),
    );

    // get_logs should deduplicate: shared log appears once, fallback-only appears once
    let logs = logger.get_logs("dedup_user");
    assert_eq!(
        logs.len(),
        2,
        "Should have 2 logs after dedup (shared + fallback-only), got {}",
        logs.len()
    );

    // Verify the shared log appears only once
    let shared_count = logs.iter().filter(|l| l.id() == "shared-id-1").count();
    assert_eq!(shared_count, 1, "Shared log should appear exactly once");

    // Verify the fallback-only log appears
    let fallback_count = logs.iter().filter(|l| l.id() == "fallback-only-1").count();
    assert_eq!(fallback_count, 1, "Fallback-only log should appear once");
}

#[tokio::test]
async fn test_get_logs_with_only_fallback() {
    let logger = AppAuditLogger::with_limit(100);

    // Populate only fallback (no primary)
    let fallback_log = AuditLog {
        id: "fb-only-2".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        user_id: Some("fb_user".to_string()),
        action: "fb_action".to_string(),
        resource: "res".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata::default(),
        signature: None,
    };
    logger
        .fallback_logs
        .set("fb_user", serialize_audit_logs(&[fallback_log]));

    let logs = logger.get_logs("fb_user");
    assert_eq!(
        logs.len(),
        1,
        "Should return fallback log when primary is empty"
    );
    assert_eq!(logs[0].id(), "fb-only-2");
}

// ============================================================================
// Worker fallback merge tests
//
// The spawned worker task in with_limit() and build() drains the queue and
// merges any remaining fallback logs. Since log() already merges fallback
// synchronously, the worker's fallback merge is only triggered when
// fallback data exists at the time the worker processes a batch. These
// tests manually inject fallback data and send a batch to trigger the
// worker's merge path.
// ============================================================================

#[tokio::test]
async fn test_worker_merges_fallback_from_queue() {
    let logger = AppAuditLogger::with_limit(100);

    // Populate fallback_logs for "worker_user"
    let fallback_log = AuditLog {
        id: "worker-fb-1".to_string(),
        timestamp: chrono::Utc::now().timestamp() - 100,
        user_id: Some("worker_user".to_string()),
        action: "worker_fb_action".to_string(),
        resource: "res".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata::default(),
        signature: None,
    };
    logger
        .fallback_logs
        .set("worker_user", serialize_audit_logs(&[fallback_log]));

    // Populate primary with an existing log
    let primary_log = AuditLog {
        id: "worker-primary-1".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        user_id: Some("worker_user".to_string()),
        action: "worker_primary_action".to_string(),
        resource: "res".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata::default(),
        signature: None,
    };
    logger
        .logs
        .set("worker_user", serialize_audit_logs(&[primary_log]));

    // Send a batch to the queue to trigger the worker's fallback merge
    let batch = AuditLogBatch {
        user_id: "worker_user".to_string(),
        log: make_test_audit_log("worker_user", "queued_action"),
    };
    let _ = logger.queue_sender.send(batch).await;

    // Wait for the worker to process the batch
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // The worker should have merged the fallback into primary and deleted it
    assert!(
        logger.fallback_logs.get("worker_user").is_none(),
        "Worker should have deleted fallback after merging"
    );

    // Primary should contain the merged logs (primary + fallback)
    let logs = logger.get_logs("worker_user");
    let actions: Vec<&str> = logs.iter().map(|l| l.action()).collect();
    assert!(
        actions.contains(&"worker_fb_action"),
        "Merged logs should contain the fallback action"
    );
}

#[tokio::test]
async fn test_worker_no_fallback_does_nothing() {
    // When there's no fallback data, the worker should just drain the
    // queue without modifying primary storage.
    let logger = AppAuditLogger::with_limit(100);

    // Populate primary only (no fallback)
    let primary_log = AuditLog {
        id: "no-fb-primary".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        user_id: Some("no_fb_user".to_string()),
        action: "primary_action".to_string(),
        resource: "res".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata::default(),
        signature: None,
    };
    logger
        .logs
        .set("no_fb_user", serialize_audit_logs(&[primary_log]));

    // Send a batch
    let batch = AuditLogBatch {
        user_id: "no_fb_user".to_string(),
        log: make_test_audit_log("no_fb_user", "queued"),
    };
    let _ = logger.queue_sender.send(batch).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Primary should be unchanged (worker found no fallback to merge)
    let logs = logger.get_logs("no_fb_user");
    assert_eq!(
        logs.len(),
        1,
        "Primary should have 1 log (no merge occurred)"
    );
}

// ============================================================================
// AuditLogger trait no-runtime path test
//
// The AuditLogger trait impl's log() method checks for a tokio runtime. If
// none is available, it falls back to printing to stderr (lines 498-503).
// This test calls the trait method from a plain std::thread (no runtime).
// ============================================================================

#[tokio::test]
async fn test_audit_logger_trait_no_runtime_path() {
    use crate::security::AuditLogger as AuditLoggerTrait;

    // with_limit() calls tokio::spawn(), so we need a runtime.
    // The spawned thread below has NO runtime, testing the no-runtime fallback path.
    let logger = AppAuditLogger::with_limit(10);
    let log = make_test_audit_log("no_rt_user", "no_runtime_action");

    // Spawn a plain OS thread with NO tokio runtime. The trait impl's
    // log() should detect the missing runtime and print to stderr instead
    // of panicking.
    let logger_clone = logger.clone();
    let handle = std::thread::spawn(move || {
        // This calls the AuditLogger trait method, not the async log()
        AuditLoggerTrait::log(&logger_clone, log);
    });

    // The thread should complete without panicking
    handle.join().expect("Thread should not panic");

    // No logs should be stored (they were dropped to stderr)
    let logs = logger.get_logs("no_rt_user");
    assert_eq!(
        logs.len(),
        0,
        "No logs should be stored when no runtime is available"
    );
}

#[tokio::test]
async fn test_audit_logger_trait_with_runtime_spawns_task() {
    use crate::security::AuditLogger as AuditLoggerTrait;

    let logger = AppAuditLogger::with_limit(10);
    let log = make_test_audit_log("rt_user", "runtime_action");

    // Call the trait method from within a tokio runtime — it should
    // spawn a task that calls the async log() method.
    AuditLoggerTrait::log(&logger, log);

    // Wait for the spawned task to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // The log should have been stored
    let logs = logger.get_logs("rt_user");
    assert_eq!(
        logs.len(),
        1,
        "Log should be stored when runtime is available"
    );
    assert_eq!(logs[0].action(), "runtime_action");
}

// ============================================================================
// Channel Full/Closed and semaphore timeout branch tests
//
// log() sends an AuditLogBatch via try_send(). The Full and Closed error
// arms are hard to reach in normal operation because log() also writes
// synchronously and the channel is created by the logger. These tests
// exercise both arms by constructing a logger with a controlled channel
// state (no spawned worker to drain the queue).
// ============================================================================

/// Verify that log() still stores to primary storage when the async queue
/// is full, hitting the `TrySendError::Full(_)` arm (lines 370-373).
///
/// Constructs a logger with a capacity-1 channel that is pre-filled, so
/// the next try_send from log() returns Full. The receiver is held (not
/// dropped) to keep the channel open, and no worker is spawned so the
/// queue stays full deterministically.
#[tokio::test]
async fn test_log_handles_full_queue() {
    let (sender, _receiver) = tokio::sync::mpsc::channel::<AuditLogBatch>(1);

    // Fill the queue to capacity
    let filler = AuditLogBatch {
        user_id: "filler_user".to_string(),
        log: make_test_audit_log("filler_user", "filler"),
    };
    assert!(sender.try_send(filler).is_ok());

    let logger = AppAuditLogger {
        logs: Arc::new(crate::cache::DashMapCache::new()),
        max_logs_per_user: 100,
        semaphore: Arc::new(tokio::sync::Semaphore::new(10)),
        queue_sender: Arc::new(sender),
        fallback_logs: Arc::new(crate::cache::DashMapCache::new()),
        dropped_log_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        total_log_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
    };

    let context = AuthContext {
        user_id: Some("full_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };
    logger.log(&context, "full_action", "res", true, None).await;

    // The log should still be stored in primary storage (synchronous path)
    let logs = logger.get_logs("full_user");
    assert_eq!(
        logs.len(),
        1,
        "Log should be stored in primary storage even when queue is full"
    );
    assert_eq!(logs[0].action(), "full_action");
}

/// Verify that log() still stores to primary storage when the async
/// channel is closed (receiver dropped), hitting the
/// `TrySendError::Closed(_)` arm (lines 374-376).
#[tokio::test]
async fn test_log_handles_closed_channel() {
    let (sender, receiver) = tokio::sync::mpsc::channel::<AuditLogBatch>(1);
    drop(receiver); // closes the channel

    let logger = AppAuditLogger {
        logs: Arc::new(crate::cache::DashMapCache::new()),
        max_logs_per_user: 100,
        semaphore: Arc::new(tokio::sync::Semaphore::new(10)),
        queue_sender: Arc::new(sender),
        fallback_logs: Arc::new(crate::cache::DashMapCache::new()),
        dropped_log_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        total_log_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
    };

    let context = AuthContext {
        user_id: Some("closed_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };
    logger
        .log(&context, "closed_action", "res", true, None)
        .await;

    // The log should still be stored in primary storage
    let logs = logger.get_logs("closed_user");
    assert_eq!(
        logs.len(),
        1,
        "Log should be stored in primary storage even when channel is closed"
    );
    assert_eq!(logs[0].action(), "closed_action");
}

/// Verify that log() skips storage entirely when the semaphore permit
/// cannot be acquired within the 1-second timeout (lines 271-274).
///
/// Builds a logger with max_concurrent_ops=1, acquires the only permit,
/// then calls log() which must wait and eventually time out.
#[tokio::test]
async fn test_log_skips_when_semaphore_times_out() {
    let logger = AppAuditLogger::builder()
        .max_logs_per_user(100)
        .max_concurrent_ops(1)
        .queue_size(100)
        .build();

    // Acquire the only permit and hold it for the duration of the test
    let _held_permit = logger.semaphore.clone().acquire_owned().await.unwrap();

    let context = AuthContext {
        user_id: Some("timeout_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };

    let start = std::time::Instant::now();
    logger
        .log(&context, "timeout_action", "res", true, None)
        .await;
    let elapsed = start.elapsed();

    // Should have waited approximately 1 second (the timeout duration)
    assert!(
        elapsed >= std::time::Duration::from_millis(900),
        "Should have waited ~1s for the semaphore timeout, elapsed: {:?}",
        elapsed
    );

    // No log should be stored because the permit was never acquired
    let logs = logger.get_logs("timeout_user");
    assert_eq!(
        logs.len(),
        0,
        "No log should be stored on semaphore timeout"
    );
}
