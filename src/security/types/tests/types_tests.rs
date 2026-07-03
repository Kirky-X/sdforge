// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for security module shared types.
//!
//! Covers `AuditLog` signatures, `CacheNamespace` key generation, serialization
//! roundtrips, `AuthContext`/`AuthMetadata`/`AuthExtractor` accessors, and error
//! `Display` implementations.

use super::super::*;

#[test]
fn test_audit_log_signature_generation() {
    let mut log = AuditLog {
        id: "test-id-123".to_string(),
        timestamp: 1234567890,
        user_id: Some("user123".to_string()),
        action: "LOGIN".to_string(),
        resource: "/api/auth".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata {
            client_ip: Some("192.168.1.1".to_string()),
            user_agent: Some("TestAgent/1.0".to_string()),
            request_id: "req-123".to_string(),
            timestamp: 1234567890,
        },
        signature: None,
    };

    // Generate signature
    let secret_key = b"test-secret-key";
    let signature = log.generate_signature(secret_key);

    // Verify signature was generated and stored
    assert!(!signature.is_empty());
    assert!(log.signature.is_some());
    assert_eq!(log.signature.as_ref().unwrap(), &signature);
}

#[test]
fn test_audit_log_signature_verification_success() {
    let mut log = AuditLog {
        id: "test-id-456".to_string(),
        timestamp: 1234567890,
        user_id: Some("user456".to_string()),
        action: "LOGOUT".to_string(),
        resource: "/api/auth".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata {
            client_ip: Some("192.168.1.1".to_string()),
            user_agent: Some("TestAgent/1.0".to_string()),
            request_id: "req-456".to_string(),
            timestamp: 1234567890,
        },
        signature: None,
    };

    // Sign the log
    let secret_key = b"test-secret-key";
    let _signature = log.generate_signature(secret_key);

    // Verify with correct key
    let result = log.verify_signature(secret_key);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_audit_log_signature_verification_wrong_key() {
    let mut log = AuditLog {
        id: "test-id-789".to_string(),
        timestamp: 1234567890,
        user_id: Some("user789".to_string()),
        action: "DELETE".to_string(),
        resource: "/api/resource".to_string(),
        result: AuditResult::Failure {
            message: "Not authorized".to_string(),
        },
        metadata: AuthMetadata {
            client_ip: Some("192.168.1.1".to_string()),
            user_agent: Some("TestAgent/1.0".to_string()),
            request_id: "req-789".to_string(),
            timestamp: 1234567890,
        },
        signature: None,
    };

    // Sign with one key
    let secret_key = b"test-secret-key";
    let _signature = log.generate_signature(secret_key);

    // Try to verify with wrong key
    let wrong_key = b"wrong-secret-key";
    let result = log.verify_signature(wrong_key);
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_audit_log_signature_tamper_detection() {
    let mut log = AuditLog {
        id: "test-id-tamper".to_string(),
        timestamp: 1234567890,
        user_id: Some("usertamper".to_string()),
        action: "UPDATE".to_string(),
        resource: "/api/resource/123".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata {
            client_ip: Some("192.168.1.1".to_string()),
            user_agent: Some("TestAgent/1.0".to_string()),
            request_id: "req-tamper".to_string(),
            timestamp: 1234567890,
        },
        signature: None,
    };

    // Sign the log
    let secret_key = b"test-secret-key";
    let _signature = log.generate_signature(secret_key);

    // Tamper with the log (change action)
    log.action = "DELETED".to_string();

    // Verification should fail
    let result = log.verify_signature(secret_key);
    assert!(result.is_ok());
    assert!(!result.unwrap(), "Tampered log should fail verification");
}

#[test]
fn test_audit_log_no_signature() {
    let log = AuditLog {
        id: "test-id-nosig".to_string(),
        timestamp: 1234567890,
        user_id: Some("usernosig".to_string()),
        action: "READ".to_string(),
        resource: "/api/public".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata {
            client_ip: Some("192.168.1.1".to_string()),
            user_agent: Some("TestAgent/1.0".to_string()),
            request_id: "req-nosig".to_string(),
            timestamp: 1234567890,
        },
        signature: None,
    };

    // Verify should return error when no signature present
    let result = log.verify_signature(b"any-key");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "No signature present");
}

#[test]
fn test_audit_log_signature_getter() {
    let mut log = AuditLog {
        id: "test-id-getter".to_string(),
        timestamp: 1234567890,
        user_id: None,
        action: "ANONYMOUS".to_string(),
        resource: "/public".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata {
            client_ip: None,
            user_agent: None,
            request_id: "req-getter".to_string(),
            timestamp: 1234567890,
        },
        signature: None,
    };

    // Initially no signature
    assert!(log.signature().is_none());

    // Generate signature
    let _signature = log.generate_signature(b"test-key");

    // Now signature should be present
    assert!(log.signature().is_some());
    assert!(!log.signature().unwrap().is_empty());
}

// ============================================================================
// CacheNamespace Tests
// ============================================================================

#[test]
fn test_cache_namespace_api_key() {
    let ns = CacheNamespace::ApiKey;
    assert_eq!(ns.key("hash123"), "sdforge:apikey:hash123");
}

#[test]
fn test_cache_namespace_bearer_blacklist() {
    let ns = CacheNamespace::BearerBlacklist;
    assert_eq!(ns.key("token_abc"), "sdforge:bearer:blacklist:token_abc");
}

#[test]
fn test_cache_namespace_bearer_valid() {
    let ns = CacheNamespace::BearerValid;
    assert_eq!(ns.key("token_xyz"), "sdforge:bearer:valid:token_xyz");
}

#[test]
fn test_cache_namespace_idempotency() {
    let ns = CacheNamespace::Idempotency;
    assert_eq!(ns.key("idem_key_123"), "sdforge:idempotency:idem_key_123");
}

// ============================================================================
// Serialization Roundtrip Tests
// ============================================================================

#[test]
fn test_serialize_deserialize_permissions_roundtrip() {
    let perms = vec!["read".to_string(), "write".to_string(), "admin".to_string()];
    let bytes = serialize_permissions(&perms);
    let result = deserialize_permissions(&bytes);
    assert_eq!(result, perms);
}

#[test]
fn test_serialize_deserialize_permissions_empty() {
    let perms: Vec<String> = vec![];
    let bytes = serialize_permissions(&perms);
    let result = deserialize_permissions(&bytes);
    assert_eq!(result, perms);
}

#[test]
fn test_deserialize_permissions_invalid_data() {
    let result = deserialize_permissions(b"invalid bincode data");
    assert_eq!(result, Vec::<String>::new());
}

#[test]
fn test_serialize_deserialize_auth_context_roundtrip() {
    let ctx = AuthContext {
        user_id: Some("user123".to_string()),
        permissions: vec!["read".to_string()],
        metadata: AuthMetadata {
            client_ip: Some("192.168.1.1".to_string()),
            user_agent: Some("TestAgent".to_string()),
            request_id: "req-123".to_string(),
            timestamp: 1234567890,
        },
    };
    let bytes = serialize_auth_context(&ctx);
    let result = deserialize_auth_context(&bytes);
    assert!(result.is_some());
    let restored = result.unwrap();
    assert_eq!(restored.user_id, Some("user123".to_string()));
    assert_eq!(restored.permissions, vec!["read".to_string()]);
}

#[test]
fn test_deserialize_auth_context_invalid_data() {
    let result = deserialize_auth_context(b"invalid data");
    assert!(result.is_none());
}

// ============================================================================
// parse_audit_log Tests
// ============================================================================

#[test]
fn test_parse_audit_log_valid_success() {
    let value = serde_json::json!({
        "id": "log-1",
        "timestamp": 1234567890i64,
        "user_id": "user1",
        "action": "LOGIN",
        "resource": "/api/auth",
        "result": {"status": "success"},
        "metadata": {
            "client_ip": "10.0.0.1",
            "user_agent": "TestAgent",
            "request_id": "req-1",
            "timestamp": 1234567890i64
        }
    });
    let log = parse_audit_log(&value);
    assert!(log.is_some());
    let log = log.unwrap();
    assert_eq!(log.id, "log-1");
    assert_eq!(log.action, "LOGIN");
    assert!(matches!(log.result, AuditResult::Success));
}

#[test]
fn test_parse_audit_log_valid_failure() {
    let value = serde_json::json!({
        "id": "log-2",
        "timestamp": 1234567890i64,
        "user_id": null,
        "action": "DELETE",
        "resource": "/api/resource/1",
        "result": {"status": "failure", "message": "Not authorized"},
        "metadata": {
            "client_ip": null,
            "user_agent": null,
            "request_id": "req-2",
            "timestamp": 1234567890i64
        }
    });
    let log = parse_audit_log(&value);
    assert!(log.is_some());
    let log = log.unwrap();
    assert!(log.user_id.is_none());
    match log.result {
        AuditResult::Failure { message } => {
            assert_eq!(message, "Not authorized");
        }
        _ => panic!("Expected Failure"),
    }
}

#[test]
fn test_parse_audit_log_invalid_missing_id() {
    let value = serde_json::json!({
        "timestamp": 1234567890i64,
        "action": "LOGIN",
        "resource": "/api/auth",
        "result": {"status": "success"},
        "metadata": {
            "request_id": "req-1",
            "timestamp": 1234567890i64
        }
    });
    assert!(parse_audit_log(&value).is_none());
}

#[test]
fn test_parse_audit_log_invalid_result_status() {
    let value = serde_json::json!({
        "id": "log-3",
        "timestamp": 1234567890i64,
        "action": "LOGIN",
        "resource": "/api/auth",
        "result": {"status": "unknown"},
        "metadata": {
            "request_id": "req-3",
            "timestamp": 1234567890i64
        }
    });
    assert!(parse_audit_log(&value).is_none());
}

#[test]
fn test_parse_audit_log_not_an_object() {
    let value = serde_json::json!("not an object");
    assert!(parse_audit_log(&value).is_none());
}

// ============================================================================
// serialize_audit_logs / deserialize_audit_logs Roundtrip Tests
// ============================================================================

#[test]
fn test_serialize_deserialize_audit_logs_roundtrip() {
    let logs = vec![
        AuditLog {
            id: "log-1".to_string(),
            timestamp: 1234567890,
            user_id: Some("user1".to_string()),
            action: "LOGIN".to_string(),
            resource: "/api/auth".to_string(),
            result: AuditResult::Success,
            metadata: AuthMetadata::default(),
            signature: None,
        },
        AuditLog {
            id: "log-2".to_string(),
            timestamp: 1234567891,
            user_id: Some("user2".to_string()),
            action: "LOGOUT".to_string(),
            resource: "/api/auth".to_string(),
            result: AuditResult::Failure {
                message: "Timeout".to_string(),
            },
            metadata: AuthMetadata::default(),
            signature: None,
        },
    ];
    let bytes = serialize_audit_logs(&logs);
    let result = deserialize_audit_logs(&bytes);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].id, "log-1");
    assert_eq!(result[1].id, "log-2");
}

#[test]
fn test_deserialize_audit_logs_empty_array() {
    let bytes = b"[]";
    let result = deserialize_audit_logs(bytes);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_deserialize_audit_logs_invalid_json() {
    let bytes = b"not json";
    let result = deserialize_audit_logs(bytes);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_serialize_audit_logs_empty() {
    let logs: Vec<AuditLog> = vec![];
    let bytes = serialize_audit_logs(&logs);
    assert_eq!(bytes, b"[]");
}

// ============================================================================
// AuthContext Tests
// ============================================================================

#[test]
fn test_auth_context_new() {
    let metadata = AuthMetadata::default();
    let ctx = AuthContext::new(
        Some("user1".to_string()),
        vec!["read".to_string(), "write".to_string()],
        metadata,
    );
    assert_eq!(ctx.user_id(), Some("user1"));
    assert_eq!(ctx.permissions().len(), 2);
}

#[test]
fn test_auth_context_has_permission_true() {
    let ctx = AuthContext {
        user_id: Some("user1".to_string()),
        permissions: vec!["read".to_string(), "admin".to_string()],
        metadata: AuthMetadata::default(),
    };
    assert!(ctx.has_permission("read"));
    assert!(ctx.has_permission("admin"));
}

#[test]
fn test_auth_context_has_permission_false() {
    let ctx = AuthContext {
        user_id: Some("user1".to_string()),
        permissions: vec!["read".to_string()],
        metadata: AuthMetadata::default(),
    };
    assert!(!ctx.has_permission("write"));
    assert!(!ctx.has_permission("admin"));
}

#[test]
fn test_auth_context_has_permission_empty() {
    let ctx = AuthContext {
        user_id: None,
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };
    assert!(!ctx.has_permission("anything"));
}

#[test]
fn test_auth_context_accessors() {
    let metadata = AuthMetadata {
        client_ip: Some("10.0.0.1".to_string()),
        user_agent: Some("Agent".to_string()),
        request_id: "req-1".to_string(),
        timestamp: 1234567890,
    };
    let ctx = AuthContext {
        user_id: Some("user1".to_string()),
        permissions: vec!["read".to_string()],
        metadata: metadata.clone(),
    };
    assert_eq!(ctx.user_id(), Some("user1"));
    assert_eq!(ctx.permissions(), &["read".to_string()]);
    assert_eq!(ctx.metadata().client_ip(), Some("10.0.0.1"));
}

// ============================================================================
// AuthMetadata Tests
// ============================================================================

#[test]
fn test_auth_metadata_new() {
    let meta = AuthMetadata::new(Some("10.0.0.1".to_string()), Some("Agent".to_string()));
    assert_eq!(meta.client_ip(), Some("10.0.0.1"));
    assert_eq!(meta.user_agent(), Some("Agent"));
    assert!(!meta.request_id().is_empty());
    assert!(meta.timestamp() > 0);
}

#[test]
fn test_auth_metadata_accessors_with_none() {
    let meta = AuthMetadata {
        client_ip: None,
        user_agent: None,
        request_id: "req-1".to_string(),
        timestamp: 1234567890,
    };
    assert!(meta.client_ip().is_none());
    assert!(meta.user_agent().is_none());
    assert_eq!(meta.request_id(), "req-1");
    assert_eq!(meta.timestamp(), 1234567890);
}

#[test]
fn test_auth_metadata_default() {
    let meta = AuthMetadata::default();
    assert!(meta.client_ip().is_none());
    assert!(meta.user_agent().is_none());
    assert_eq!(meta.request_id(), "");
    assert_eq!(meta.timestamp(), 0);
}

// ============================================================================
// AuthExtractor Tests
// ============================================================================

#[test]
fn test_auth_extractor_new() {
    let ctx = AuthContext {
        user_id: Some("user1".to_string()),
        permissions: vec!["read".to_string()],
        metadata: AuthMetadata::default(),
    };
    let extractor = AuthExtractor::new(ctx.clone());
    assert_eq!(extractor.context().user_id, ctx.user_id);
}

#[test]
fn test_auth_extractor_context() {
    let ctx = AuthContext {
        user_id: Some("extractor_user".to_string()),
        permissions: vec!["admin".to_string()],
        metadata: AuthMetadata::default(),
    };
    let extractor = AuthExtractor::new(ctx.clone());
    let retrieved = extractor.context();
    assert_eq!(retrieved.user_id(), Some("extractor_user"));
}

#[test]
fn test_auth_extractor_into_inner() {
    let ctx = AuthContext {
        user_id: Some("inner_user".to_string()),
        permissions: vec![],
        metadata: AuthMetadata::default(),
    };
    let extractor = AuthExtractor::new(ctx.clone());
    let inner = extractor.into_inner();
    assert_eq!(inner.user_id, Some("inner_user".to_string()));
}

// ============================================================================
// JwtError Display Tests
// ============================================================================

#[test]
fn test_jwt_error_invalid_format() {
    let err = JwtError::InvalidFormat;
    assert_eq!(format!("{}", err), "Invalid JWT format");
}

#[test]
fn test_jwt_error_base64_decode() {
    let err = JwtError::Base64DecodeError;
    assert_eq!(format!("{}", err), "Failed to decode base64");
}

#[test]
fn test_jwt_error_invalid_signature() {
    let err = JwtError::InvalidSignature;
    assert_eq!(format!("{}", err), "Invalid JWT signature");
}

#[test]
fn test_jwt_error_expired() {
    let err = JwtError::Expired;
    assert_eq!(format!("{}", err), "JWT token expired");
}

#[test]
fn test_jwt_error_not_yet_valid() {
    let err = JwtError::NotYetValid;
    assert_eq!(format!("{}", err), "JWT token not yet valid");
}

#[test]
fn test_jwt_error_invalid_payload() {
    let err = JwtError::InvalidPayload;
    assert_eq!(format!("{}", err), "Invalid JWT payload");
}

#[test]
fn test_jwt_error_clock_skew() {
    let err = JwtError::ClockSkew;
    assert_eq!(format!("{}", err), "Clock skew too large");
}

#[test]
fn test_jwt_error_is_std_error() {
    let err = JwtError::Expired;
    let _: &dyn std::error::Error = &err;
}

// ============================================================================
// AuthConfigError Display Tests
// ============================================================================

#[test]
fn test_auth_config_error_invalid_secret() {
    let err = AuthConfigError::InvalidSecret("too weak".to_string());
    assert!(format!("{}", err).contains("Invalid secret"));
}

#[test]
fn test_auth_config_error_secret_too_short() {
    let err = AuthConfigError::SecretTooShort { length: 10 };
    let msg = format!("{}", err);
    assert!(msg.contains("Secret too short"));
    assert!(msg.contains("10"));
}

#[test]
fn test_auth_config_error_missing_character_class() {
    let err = AuthConfigError::MissingCharacterClass {
        required_type: "uppercase letter",
    };
    let msg = format!("{}", err);
    assert!(msg.contains("uppercase letter"));
}

#[test]
fn test_auth_config_error_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = AuthConfigError::from(io_err);
    let msg = format!("{}", err);
    assert!(msg.contains("Configuration I/O error"));
}

// ============================================================================
// AuditResult Custom Serialize Tests
// ============================================================================

#[test]
fn test_audit_result_serialize_success() {
    let result = AuditResult::Success;
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(r#""status":"success""#));
}

#[test]
fn test_audit_result_serialize_failure() {
    let result = AuditResult::Failure {
        message: "Access denied".to_string(),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains(r#""status":"failure""#));
    assert!(json.contains("Access denied"));
}

#[test]
fn test_audit_log_serialize_with_audit_result() {
    let log = AuditLog {
        id: "ser-test".to_string(),
        timestamp: 1234567890,
        user_id: Some("user1".to_string()),
        action: "TEST".to_string(),
        resource: "/test".to_string(),
        result: AuditResult::Success,
        metadata: AuthMetadata::default(),
        signature: None,
    };
    let json = serde_json::to_string(&log).unwrap();
    assert!(json.contains(r#""status":"success""#));
}

#[test]
fn test_audit_result_clone() {
    let result = AuditResult::Failure {
        message: "clone test".to_string(),
    };
    let cloned = result.clone();
    match cloned {
        AuditResult::Failure { message } => {
            assert_eq!(message, "clone test");
        }
        _ => panic!("Expected Failure"),
    }
}

#[test]
fn test_audit_result_debug() {
    let result = AuditResult::Success;
    let debug_str = format!("{:?}", result);
    assert!(debug_str.contains("Success"));
}

// ============================================================================
// AuthError Display Tests
// ============================================================================

#[test]
fn test_auth_error_missing_auth() {
    let err = AuthError::MissingAuth;
    assert_eq!(
        format!("{}", err),
        "Missing or invalid authorization header"
    );
}

#[test]
fn test_auth_error_invalid_token() {
    let err = AuthError::InvalidToken;
    assert_eq!(format!("{}", err), "Invalid or expired token");
}

#[test]
fn test_auth_error_insufficient_permissions() {
    let err = AuthError::InsufficientPermissions {
        required: "admin".to_string(),
        user_permissions: vec!["read".to_string()],
    };
    let msg = format!("{}", err);
    assert!(msg.contains("Insufficient permissions"));
    assert!(msg.contains("admin"));
}

// ============================================================================
// Additional CacheNamespace Tests (Copy, Clone, PartialEq, Debug)
// ============================================================================

#[test]
fn test_cache_namespace_copy() {
    let ns = CacheNamespace::ApiKey;
    let ns_copy = ns;
    assert_eq!(ns, ns_copy);
}

#[test]
fn test_cache_namespace_clone() {
    let ns = CacheNamespace::BearerBlacklist;
    let ns_copy = ns;
    assert_eq!(ns, ns_copy);
}

#[test]
fn test_cache_namespace_partial_eq() {
    let ns1 = CacheNamespace::BearerValid;
    let ns2 = CacheNamespace::BearerValid;
    let ns3 = CacheNamespace::ApiKey;
    assert_eq!(ns1, ns2);
    assert_ne!(ns1, ns3);
}

#[test]
fn test_cache_namespace_debug() {
    let ns = CacheNamespace::Idempotency;
    let debug_str = format!("{:?}", ns);
    assert!(debug_str.contains("Idempotency"));
}

// ============================================================================
// Additional Serialization Tests (Special Cases)
// ============================================================================

#[test]
fn test_serialize_permissions_special_chars() {
    let perms = vec![
        "read:admin".to_string(),
        "write@user".to_string(),
        "delete#1".to_string(),
    ];
    let bytes = serialize_permissions(&perms);
    let result = deserialize_permissions(&bytes);
    assert_eq!(result, perms);
}

#[test]
fn test_deserialize_permissions_empty_data() {
    let result = deserialize_permissions(b"");
    assert_eq!(result, Vec::<String>::new());
}

// ============================================================================
// Additional AuthContext Tests (All Fields, Empty Fields)
// ============================================================================

#[test]
fn test_auth_context_with_all_fields() {
    let ctx = AuthContext {
        user_id: Some("admin_user".to_string()),
        permissions: vec![
            "read".to_string(),
            "write".to_string(),
            "delete".to_string(),
            "admin".to_string(),
        ],
        metadata: AuthMetadata {
            client_ip: Some("10.20.30.40".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            request_id: "full-req-123".to_string(),
            timestamp: 1700000000,
        },
    };
    assert_eq!(ctx.user_id(), Some("admin_user"));
    assert_eq!(ctx.permissions().len(), 4);
    assert!(ctx.has_permission("admin"));
    assert_eq!(ctx.metadata().client_ip(), Some("10.20.30.40"));
}

#[test]
fn test_auth_context_empty_fields() {
    let ctx = AuthContext {
        user_id: None,
        permissions: vec![],
        metadata: AuthMetadata {
            client_ip: None,
            user_agent: None,
            request_id: "".to_string(),
            timestamp: 0,
        },
    };
    assert_eq!(ctx.user_id(), None);
    assert!(ctx.permissions().is_empty());
    assert!(!ctx.has_permission("anything"));
}

// ============================================================================
// Additional AuthConfigError Tests
// ============================================================================

#[test]
fn test_auth_config_error_display() {
    let err = AuthConfigError::InvalidSecret("test error".to_string());
    let msg = format!("{}", err);
    assert_eq!(msg, "Invalid secret: test error");
}

// ============================================================================
// Edge Case Tests for Serialization
// ============================================================================

#[test]
fn test_serialize_auth_context_empty() {
    let ctx = AuthContext::new(None, vec![], AuthMetadata::default());
    let bytes = serialize_auth_context(&ctx);
    let result = deserialize_auth_context(&bytes);
    assert!(result.is_some());
    let restored = result.unwrap();
    assert_eq!(restored.user_id, None);
    assert!(restored.permissions.is_empty());
}

#[test]
fn test_serialize_auth_context_roundtrip() {
    let ctx = AuthContext {
        user_id: Some("roundtrip_user".to_string()),
        permissions: vec!["test".to_string()],
        metadata: AuthMetadata {
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: None,
            request_id: "test-roundtrip".to_string(),
            timestamp: 9999999999,
        },
    };
    let bytes = serialize_auth_context(&ctx);
    let result = deserialize_auth_context(&bytes);
    assert!(result.is_some());
    let restored = result.unwrap();
    assert_eq!(restored.user_id, ctx.user_id);
    assert_eq!(restored.permissions, ctx.permissions);
}

// ============================================================================
// AuthMetadata Additional Tests
// ============================================================================

#[test]
fn test_auth_metadata_accessors() {
    let meta = AuthMetadata {
        client_ip: Some("192.168.0.1".to_string()),
        user_agent: Some("TestBrowser".to_string()),
        request_id: "accessor-test".to_string(),
        timestamp: 1234567890,
    };
    assert_eq!(meta.client_ip(), Some("192.168.0.1"));
    assert_eq!(meta.user_agent(), Some("TestBrowser"));
    assert_eq!(meta.request_id(), "accessor-test");
    assert_eq!(meta.timestamp(), 1234567890);
}

// ============================================================================
// CacheNamespace Additional Tests
// ============================================================================

#[test]
fn test_cache_namespace_key_empty_suffix() {
    let ns = CacheNamespace::ApiKey;
    let key = ns.key("");
    assert_eq!(key, "sdforge:apikey:");
}

#[test]
fn test_cache_namespace_key_special_suffix() {
    let ns = CacheNamespace::BearerValid;
    let key = ns.key("token/special?chars=here");
    assert_eq!(key, "sdforge:bearer:valid:token/special?chars=here");
}

// ============================================================================
// Additional Permission Tests
// ============================================================================

#[test]
fn test_permissions_large_list() {
    let perms: Vec<String> = (0..1000).map(|i| format!("perm_{}", i)).collect();
    let bytes = serialize_permissions(&perms);
    let result = deserialize_permissions(&bytes);
    assert_eq!(result.len(), 1000);
    assert_eq!(result[0], "perm_0");
    assert_eq!(result[999], "perm_999");
}

// ============================================================================
// deserialize_audit_logs single-object branch coverage
// ============================================================================

/// Deserialize a single JSON object (not an array) — exercises the
/// Ok(serde_json::Value::Object(_)) branch of deserialize_audit_logs,
/// which re-parses the object via parse_audit_log and returns a 1-element Vec.
#[test]
fn test_deserialize_audit_logs_single_object() {
    let json = serde_json::json!({
        "id": "single-log",
        "timestamp": 1700000000,
        "user_id": "user-single",
        "action": "LOGIN",
        "resource": "/api/auth",
        "result": {"status": "success"},
        "metadata": {
            "client_ip": "203.0.113.5",
            "user_agent": "test-agent",
            "request_id": "req-001",
            "timestamp": 1700000000
        },
        "signature": null
    });
    let bytes = serde_json::to_vec(&json).unwrap();
    let result = deserialize_audit_logs(&bytes);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "single-log");
    assert_eq!(result[0].user_id, Some("user-single".to_string()));
    assert_eq!(result[0].action, "LOGIN");
    assert_eq!(result[0].resource, "/api/auth");
}

/// Deserialize a single JSON object that is NOT a valid AuditLog —
/// parse_audit_log returns None, so the Object branch yields an empty Vec.
/// Covers the `.and_then(|v| parse_audit_log(&v))` None sub-branch.
#[test]
fn test_deserialize_audit_logs_single_object_invalid_returns_empty() {
    // Valid JSON object but missing required "action" field.
    let json = serde_json::json!({
        "id": "bad-log",
        "timestamp": 1700000000
    });
    let bytes = serde_json::to_vec(&json).unwrap();
    let result = deserialize_audit_logs(&bytes);
    assert_eq!(result.len(), 0);
}
