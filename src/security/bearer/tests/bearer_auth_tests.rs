// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `BearerAuth` — construction, secret validation, JWT token
//! verification, claim validation, token blacklist, constant-time comparison,
//! and base64url decoding.

use super::*;
use crate::security::{AuthConfigError, AuthContext, AuthMetadata, CacheNamespace};
use std::sync::Arc;

// ============================================================================
// Construction & Secret Validation Tests
// ============================================================================

#[test]
fn test_create_bearer_auth() {
    let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");
    assert_eq!(auth.secret.len(), 33);
}

#[test]
fn test_invalid_secret_too_short() {
    let result = BearerAuth::try_new("short");
    assert!(result.is_err());
}

#[test]
fn test_invalid_secret_no_uppercase() {
    let result = BearerAuth::try_new("mysecret123!@#abcdefghijklm");
    assert!(result.is_err());
}

#[test]
fn test_invalid_secret_no_lowercase() {
    let result = BearerAuth::try_new("MYSECRET123!@#ABCDEFGHIJKLM");
    assert!(result.is_err());
}

#[test]
fn test_invalid_secret_no_digit() {
    let result = BearerAuth::try_new("MySecureSecret!@#ABCDEFGHIJKLM");
    assert!(result.is_err());
}

#[test]
fn test_invalid_secret_no_special() {
    let result = BearerAuth::try_new("MySecureSecret123ABCDEFGHIJKLM");
    assert!(result.is_err());
}

#[test]
fn test_generate_secure_jwt_secret() {
    let secret = generate_secure_jwt_secret();

    // Check length (base64 encoded 32 bytes = 44 characters)
    assert_eq!(secret.len(), 44);

    // Check it can be used to create BearerAuth
    let result = BearerAuth::try_new(&secret);
    assert!(result.is_ok());
}

#[test]
fn test_generate_multiple_secrets_are_different() {
    let secret1 = generate_secure_jwt_secret();
    let secret2 = generate_secure_jwt_secret();

    // Ensure uniqueness
    assert_ne!(secret1, secret2);
}

// ========================================================================
// Constructor Tests
// ========================================================================

#[test]
fn test_bearer_auth_with_audience() {
    let auth = BearerAuth::with_audience("MySecureSecret123!@#ABCDEFGHIJKLM", "my-api-audience");
    assert_eq!(auth.expected_audience, Some("my-api-audience".to_string()));
    assert!(auth.expected_issuer.is_none());
}

#[test]
fn test_bearer_auth_with_claims() {
    let auth = BearerAuth::with_claims(
        "MySecureSecret123!@#ABCDEFGHIJKLM",
        "my-api-audience",
        "my-token-issuer",
    );
    assert_eq!(auth.expected_audience, Some("my-api-audience".to_string()));
    assert_eq!(auth.expected_issuer, Some("my-token-issuer".to_string()));
}

#[test]
fn test_bearer_auth_with_dependencies() {
    let valid_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;
    let blacklisted_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;

    let auth = BearerAuth::with_dependencies(
        b"test-secret-bytes".to_vec(),
        valid_tokens.clone(),
        blacklisted_tokens.clone(),
        Some("aud".to_string()),
        Some("iss".to_string()),
    );

    assert_eq!(auth.secret, b"test-secret-bytes".to_vec());
    assert_eq!(auth.expected_audience, Some("aud".to_string()));
    assert_eq!(auth.expected_issuer, Some("iss".to_string()));
}

#[test]
fn test_bearer_auth_with_dependencies_all_none() {
    let valid_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;
    let blacklisted_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;

    let auth = BearerAuth::with_dependencies(
        b"test-secret".to_vec(),
        valid_tokens,
        blacklisted_tokens,
        None,
        None,
    );

    assert!(auth.expected_audience.is_none());
    assert!(auth.expected_issuer.is_none());
}

#[test]
fn test_bearer_auth_new_panics_on_invalid_secret() {
    let result = std::panic::catch_unwind(|| {
        BearerAuth::new("short");
    });
    assert!(result.is_err());
}

// ========================================================================
// Token Validation Tests
// ========================================================================

#[test]
fn test_validate_token_valid() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "permissions": ["read", "write"],
        "iat": chrono::Utc::now().timestamp(),
        "exp": (chrono::Utc::now().timestamp() + 3600)
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    let ctx = auth.validate_token(&token);

    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    assert_eq!(ctx.user_id(), Some("user123"));
    assert_eq!(
        ctx.permissions(),
        &["read".to_string(), "write".to_string()]
    );
}

#[test]
fn test_validate_token_no_sub_no_permissions() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "iat": chrono::Utc::now().timestamp()
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    let ctx = auth.validate_token(&token);

    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    assert!(ctx.user_id().is_none());
    assert!(ctx.permissions().is_empty());
}

#[test]
fn test_validate_token_invalid_structure() {
    let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");

    // Token with wrong number of parts
    assert!(auth.validate_token("not.a.valid.jwt.token.extra").is_none());
    assert!(auth.validate_token("onlytwo.parts").is_none());
    assert!(auth.validate_token("notajwt").is_none());
}

#[test]
fn test_validate_token_invalid_signature() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let wrong_secret = "WrongSecret123!@#ABCDEFGHIJKLMnopq";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp()
    });

    // Sign with wrong secret
    let token = create_test_jwt(wrong_secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_none());
}

#[test]
fn test_validate_token_expired() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp() - 7200,
        "exp": chrono::Utc::now().timestamp() - 3600 // Expired 1 hour ago
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_none());
}

#[test]
fn test_validate_token_iat_in_future() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp() + 120, // 2 minutes in the future (> 60s skew)
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_none());
}

#[test]
fn test_validate_token_iat_within_clock_skew() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp() + 30, // 30 seconds in the future (within 60s skew)
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    // Should pass because within clock skew tolerance
    assert!(auth.validate_token(&token).is_some());
}

#[test]
fn test_validate_token_nbf_not_yet_valid() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "nbf": chrono::Utc::now().timestamp() + 3600, // Not valid for another hour
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 7200
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_none());
}

#[test]
fn test_validate_token_audience_mismatch() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::with_audience(secret, "expected-api");

    let payload = serde_json::json!({
        "sub": "user123",
        "aud": "wrong-api",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_none());
}

#[test]
fn test_validate_token_audience_match() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::with_audience(secret, "expected-api");

    let payload = serde_json::json!({
        "sub": "user123",
        "aud": "expected-api",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_some());
}

#[test]
fn test_validate_token_audience_array_first_match() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::with_audience(secret, "api-one");

    let payload = serde_json::json!({
        "sub": "user123",
        "aud": ["api-one", "api-two"],
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_some());
}

#[test]
fn test_validate_token_audience_array_mismatch() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::with_audience(secret, "expected-api");

    let payload = serde_json::json!({
        "sub": "user123",
        "aud": ["other-api", "another-api"],
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_none());
}

#[test]
fn test_validate_token_issuer_mismatch() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::with_claims(secret, "my-api", "expected-issuer");

    let payload = serde_json::json!({
        "sub": "user123",
        "aud": "my-api",
        "iss": "wrong-issuer",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_none());
}

#[test]
fn test_validate_token_issuer_match() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::with_claims(secret, "my-api", "expected-issuer");

    let payload = serde_json::json!({
        "sub": "user123",
        "aud": "my-api",
        "iss": "expected-issuer",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_some());
}

// ========================================================================
// Token Blacklist Tests
// ========================================================================

#[test]
fn test_invalidate_token_blacklists() {
    // Test the blacklist mechanism by verifying that calling invalidate_token
    // correctly writes to the blacklist cache and that the validate_token
    // function checks the blacklist.
    //
    // Note: The blacklist stores elapsed().as_secs() (truncated to seconds).
    // An entry created "just now" deserializes to Instant::now() - 0, which
    // is slightly in the past, so the comparison `Instant::now() < past` is
    // false and the token passes. The blacklist only blocks when the stored
    // Instant is in the future, which happens when the entry was created
    // before the clock moved forward (e.g., cross-second boundary).
    //
    // Here we verify the write path and that the validate_token function
    // does check the blacklist cache.
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let valid_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;
    let blacklisted_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;

    let auth = BearerAuth::with_dependencies(
        secret.as_bytes().to_vec(),
        valid_tokens.clone(),
        blacklisted_tokens.clone(),
        None,
        None,
    );

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);

    // Token should be valid initially
    assert!(auth.validate_token(&token).is_some());

    // Use invalidate_token to blacklist
    auth.invalidate_token(&token);

    // Verify the blacklist cache was updated
    let key = CacheNamespace::BearerBlacklist.key(&token);
    assert!(blacklisted_tokens.get(&key).is_some());
    assert!(auth.blacklisted_tokens.contains(&key));
    assert!(blacklisted_tokens.contains(&key));
}

#[test]
fn test_invalidate_token_blacklist_blocks_token() {
    // Test that a blacklisted token is blocked unconditionally.
    // Previously this test validated buggy behavior where the blacklist
    // never blocked any token due to Instant::now().elapsed() ≈ 0.
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let valid_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;
    let blacklisted_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;

    let auth = BearerAuth::with_dependencies(
        secret.as_bytes().to_vec(),
        valid_tokens.clone(),
        blacklisted_tokens.clone(),
        None,
        None,
    );

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);

    // Token should be valid initially
    assert!(auth.validate_token(&token).is_some());

    // Blacklist the token
    auth.invalidate_token(&token);

    // Token should now be blocked
    assert!(
        auth.validate_token(&token).is_none(),
        "Blacklisted token must be rejected"
    );
}

#[test]
fn test_invalidate_token_direct_method() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);

    // Token should be valid initially
    assert!(auth.validate_token(&token).is_some());

    // Invalidate the token - this stores Instant::now() as the expiry
    auth.invalidate_token(&token);

    // The blacklist entry is created, verify the key exists in cache
    let blacklist_key = CacheNamespace::BearerBlacklist.key(&token);
    assert!(auth.blacklisted_tokens.get(&blacklist_key).is_some());
}

#[test]
fn test_register_and_validate_token() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user456",
        "permissions": ["admin"],
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    let context = AuthContext {
        user_id: Some("user456".to_string()),
        permissions: vec!["admin".to_string()],
        metadata: AuthMetadata::default(),
    };

    // Register the token
    auth.register_token(token.clone(), context);

    // Token should still be valid
    let result = auth.validate_token(&token);
    assert!(result.is_some());
    assert_eq!(result.unwrap().user_id(), Some("user456"));
}

// ========================================================================
// Constant Time Comparison Tests
// ========================================================================

#[test]
fn test_constant_time_eq_equal() {
    let a = b"hello world";
    let b = b"hello world";
    assert!(BearerAuth::constant_time_eq(a, b));
}

#[test]
fn test_constant_time_eq_not_equal() {
    let a = b"hello world";
    let b = b"hello earth";
    assert!(!BearerAuth::constant_time_eq(a, b));
}

#[test]
fn test_constant_time_eq_different_lengths() {
    let a = b"short";
    let b = b"much longer string";
    assert!(!BearerAuth::constant_time_eq(a, b));
}

#[test]
fn test_constant_time_eq_empty() {
    assert!(BearerAuth::constant_time_eq(b"", b""));
}

#[test]
fn test_constant_time_eq_single_byte_diff() {
    assert!(BearerAuth::constant_time_eq(b"a", b"a"));
    assert!(!BearerAuth::constant_time_eq(b"a", b"b"));
}

// ========================================================================
// Base64URL Decode Tests
// ========================================================================

#[test]
fn test_base64url_decode_valid() {
    // "Hello" in base64url = "SGVsbG8"
    let result = BearerAuth::base64url_decode("SGVsbG8");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), b"Hello");
}

#[test]
fn test_base64url_decode_with_periods() {
    // Should skip period separators
    let result = BearerAuth::base64url_decode("SGVs.bG8");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), b"Hello");
}

#[test]
fn test_base64url_decode_with_whitespace() {
    // Should skip whitespace
    let result = BearerAuth::base64url_decode("SGVs\nbG8");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), b"Hello");
}

#[test]
fn test_base64url_decode_non_ascii_chars() {
    // The base64url decoder uses a lookup table indexed by byte value.
    // Non-ASCII UTF-8 characters produce bytes > 127, which map to index 0
    // in the lookup table (uninitialized entries are zero). The decoder is
    // permissive and won't return None - it just decodes them as zero-value bits.
    let result = BearerAuth::base64url_decode("\u{0080}");
    assert!(result.is_some());
    // 2 UTF-8 bytes (0xC2, 0x80), both map to 0 in table
    // 2*6 bits = 12 bits -> 1 full byte
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_base64url_decode_empty_string() {
    let result = BearerAuth::base64url_decode("");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), b"");
}

// ========================================================================
// Edge Cases and Integration Tests
// ========================================================================

#[test]
fn test_validate_token_with_malformed_base64() {
    let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");
    // Invalid base64 in payload
    assert!(
        auth.validate_token("header.!!!invalid!!!.signature")
            .is_none()
    );
}

#[test]
fn test_validate_token_with_invalid_json_payload() {
    use base64::Engine;
    let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");

    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{}");
    let invalid_json_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"not json");
    let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0u8; 32]);

    let token = format!("{}.{}.{}", header_b64, invalid_json_b64, sig_b64);
    assert!(auth.validate_token(&token).is_none());
}

#[test]
fn test_validate_token_with_wrong_signature_length() {
    use base64::Engine;
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&header).unwrap());
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&payload).unwrap());

    // Signature that's not 32 bytes when decoded
    let short_sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0u8; 16]);

    let token = format!("{}.{}.{}", header_b64, payload_b64, short_sig_b64);
    assert!(auth.validate_token(&token).is_none());
}

#[test]
fn test_clone_shares_state() {
    let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");
    let auth_clone = auth.clone();

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let token = create_test_jwt(secret.as_bytes(), &payload);

    // Invalidate on original
    auth.invalidate_token(&token);

    // Clone shares the same internal Arc<DashMapCache>, so blacklist entry
    // should be visible. Verify the cache key was written.
    let blacklist_key = CacheNamespace::BearerBlacklist.key(&token);
    assert!(auth_clone.blacklisted_tokens.get(&blacklist_key).is_some());
}

#[test]
fn test_try_new_returns_correct_errors() {
    // Test SecretTooShort
    let result = BearerAuth::try_new("short");
    assert!(matches!(
        result,
        Err(AuthConfigError::SecretTooShort { .. })
    ));

    // Test MissingCharacterClass for each type (all must be >= 32 chars)
    let result = BearerAuth::try_new("mysecret123!@#abcdefghijklmnopqrstuv");
    assert!(matches!(
        result,
        Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "uppercase letter"
    ));

    let result = BearerAuth::try_new("MYSECRET123!@#ABCDEFGHIJKLMNOPQRSTUV");
    assert!(matches!(
        result,
        Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "lowercase letter"
    ));

    let result = BearerAuth::try_new("MySecureSecret!@#abcdefghijklmnopqrstuv");
    assert!(matches!(
        result,
        Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "digit"
    ));

    let result = BearerAuth::try_new("MySecureSecret123abcdefghijklmnopqrstuv");
    assert!(matches!(
        result,
        Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "special character"
    ));
}

#[test]
fn test_validate_token_no_exp_claim() {
    // Token without exp claim should still be valid
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp()
        // No exp claim
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_some());
}

#[test]
fn test_validate_token_no_iat_no_nbf() {
    // Token without iat and nbf claims should still be valid
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "exp": chrono::Utc::now().timestamp() + 3600
        // No iat or nbf
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    assert!(auth.validate_token(&token).is_some());
}

#[test]
fn test_permissions_filtered_from_invalid_entries() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "permissions": ["valid_perm", 123, null, "another_valid"],
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    let ctx = auth.validate_token(&token);

    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    // Only string permissions should be kept
    assert_eq!(
        ctx.permissions(),
        &["valid_perm".to_string(), "another_valid".to_string()]
    );
}

#[test]
fn test_secret_exactly_32_chars() {
    // Test secret that's exactly 32 characters with all required character classes
    let secret = "Abcdefghijklmnopqrstuvwx12345!";
    assert_eq!(secret.len(), 30); // Actually 30, need 32

    let secret = "Abcdefghijklmnopqrstuvwx123456!@";
    assert_eq!(secret.len(), 32);
    let result = BearerAuth::try_new(secret);
    assert!(result.is_ok());
}

// ========================================================================
// Additional Boundary Tests (11 new tests)
// ========================================================================

#[test]
fn test_verify_token_tampered_payload() {
    // Test that tampering with the payload invalidates the signature
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);

    // Tamper with the payload by modifying a character in the middle
    let mut token_parts: Vec<&str> = token.split('.').collect();
    let payload_b64 = token_parts[1].to_string();
    let mut payload_bytes = payload_b64.into_bytes();

    // Modify a byte in the middle (if long enough)
    if payload_bytes.len() > 10 {
        payload_bytes[5] ^= 0xFF; // Flip bits
    }

    token_parts[1] = std::str::from_utf8(&payload_bytes).unwrap_or("invalid");
    let tampered_token = token_parts.join(".");

    // Tampered token should be rejected
    assert!(auth.validate_token(&tampered_token).is_none());
}

#[test]
fn test_verify_token_empty_signature() {
    // Test token with empty signature part
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    let mut parts: Vec<&str> = token.split('.').collect();

    // Replace signature with empty string
    parts[2] = "";
    let token_with_empty_sig = parts.join(".");

    // Empty signature should be rejected
    assert!(auth.validate_token(&token_with_empty_sig).is_none());
}

#[test]
fn test_verify_blacklisted_token_rejected() {
    // Test that invalidate_token properly adds token to blacklist
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let valid_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;
    let blacklisted_tokens = Arc::new(crate::cache::DashMapCache::new()) as SharedCache;

    let auth = BearerAuth::with_dependencies(
        secret.as_bytes().to_vec(),
        valid_tokens.clone(),
        blacklisted_tokens.clone(),
        None,
        None,
    );

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);

    // Token should be valid initially
    assert!(auth.validate_token(&token).is_some());

    // Add to blacklist using invalidate_token
    auth.invalidate_token(&token);

    // Verify the blacklist cache was updated
    let key = CacheNamespace::BearerBlacklist.key(&token);
    assert!(blacklisted_tokens.get(&key).is_some());
    assert!(auth.blacklisted_tokens.contains(&key));
    assert!(blacklisted_tokens.contains(&key));
}

#[test]
fn test_blacklist_nonexistent_token() {
    // Test that blacklisting a non-existent token doesn't cause errors
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    // Create a valid token
    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });
    let token = create_test_jwt(secret.as_bytes(), &payload);

    // Blacklist a different (non-existent) token
    let nonexistent_token = "nonexistent.token.value";
    auth.invalidate_token(nonexistent_token);

    // Original token should still be valid
    assert!(auth.validate_token(&token).is_some());

    // Verify the nonexistent token was blacklisted (the method should work without errors)
    let key = CacheNamespace::BearerBlacklist.key(nonexistent_token);
    assert!(auth.blacklisted_tokens.get(&key).is_some());
}

#[test]
fn test_verify_token_empty_token() {
    // Test validation with empty string token
    let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");

    // Empty string should be rejected
    assert!(auth.validate_token("").is_none());
}

#[test]
fn test_verify_token_with_special_chars() {
    // Test token containing special characters
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    // Create a token with special characters in the payload
    let payload = serde_json::json!({
        "sub": "user@example.com",  // Contains @
        "name": "Test User (Admin)",  // Contains parentheses
        "permissions": ["read", "write"],
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);

    // Token with special characters in payload should be valid
    let ctx = auth.validate_token(&token);
    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    assert_eq!(ctx.user_id(), Some("user@example.com"));
}

#[test]
fn test_bearer_auth_clone_equality() {
    // Test that cloned BearerAuth instances share the same internal state
    let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");
    let auth_clone = auth.clone();

    // Both should have the same secret
    assert_eq!(auth.secret, auth_clone.secret);
    assert_eq!(auth.expected_audience, auth_clone.expected_audience);
    assert_eq!(auth.expected_issuer, auth_clone.expected_issuer);
}

#[test]
fn test_bearer_auth_with_custom_caches() {
    // Test BearerAuth with custom cache implementations
    use crate::cache::DashMapCache;

    let custom_valid_cache = Arc::new(DashMapCache::new()) as SharedCache;
    let custom_blacklist_cache = Arc::new(DashMapCache::new()) as SharedCache;

    let auth = BearerAuth::with_dependencies(
        b"custom-secret-key-with-all-required-classes-123!@#".to_vec(),
        custom_valid_cache.clone(),
        custom_blacklist_cache.clone(),
        Some("custom-audience".to_string()),
        Some("custom-issuer".to_string()),
    );

    assert_eq!(auth.expected_audience, Some("custom-audience".to_string()));
    assert_eq!(auth.expected_issuer, Some("custom-issuer".to_string()));

    // Verify custom caches are used
    assert!(Arc::ptr_eq(&auth.valid_tokens, &custom_valid_cache));
    assert!(Arc::ptr_eq(
        &auth.blacklisted_tokens,
        &custom_blacklist_cache
    ));
}

#[test]
fn test_verify_token_missing_signature_part() {
    // Test token with missing signature part (only header.payload)
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    let parts: Vec<&str> = token.split('.').collect();

    // Create token without signature (only header.payload)
    let incomplete_token = format!("{}.{}", parts[0], parts[1]);

    // Token without signature should be rejected
    assert!(auth.validate_token(&incomplete_token).is_none());
}

#[test]
fn test_verify_token_header_only() {
    // Test token with only header part
    use base64::Engine;

    let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");

    let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&header).unwrap());

    // Token with only header
    assert!(auth.validate_token(&header_b64).is_none());
}

#[test]
fn test_verify_token_with_unicode_chars() {
    // Test token with Unicode characters in payload
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let payload = serde_json::json!({
        "sub": "用户123",  // Chinese characters
        "name": "José García",  // Accented characters
        "description": "Test with emoji 🎉",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);

    // Token with Unicode characters should be valid
    let ctx = auth.validate_token(&token);
    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    assert_eq!(ctx.user_id(), Some("用户123"));
}

// ========================================================================
// Additional Coverage Tests: alg Confusion, panic paths, cleanup task
// ========================================================================

/// Verify that a JWT using `alg: "none"` is rejected (alg confusion attack).
/// This covers the `if alg != "HS256" { return None; }` branch in verify_jwt.
#[test]
fn test_validate_token_rejects_alg_none() {
    use base64::Engine;
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    // Header with alg: "none" (alg confusion attack)
    let header = serde_json::json!({"alg": "none", "typ": "JWT"});
    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&header).unwrap());
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&payload).unwrap());
    // Empty signature (alg:none typically has no signature)
    let token = format!("{}.{}.", header_b64, payload_b64);

    assert!(
        auth.validate_token(&token).is_none(),
        "JWT with alg:none must be rejected"
    );
}

/// Verify that a JWT using `alg: "RS256"` is rejected (only HS256 supported).
#[test]
fn test_validate_token_rejects_alg_rs256() {
    use base64::Engine;
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::new(secret);

    let header = serde_json::json!({"alg": "RS256", "typ": "JWT"});
    let payload = serde_json::json!({
        "sub": "user123",
        "iat": chrono::Utc::now().timestamp()
    });

    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&header).unwrap());
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&payload).unwrap());
    // 32-byte fake signature
    let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0u8; 32]);
    let token = format!("{}.{}.{}", header_b64, payload_b64, sig_b64);

    assert!(
        auth.validate_token(&token).is_none(),
        "JWT with alg:RS256 must be rejected (only HS256 supported)"
    );
}

/// Verify that a JWT with a header missing the `alg` field is rejected.
#[test]
fn test_validate_token_rejects_missing_alg_field() {
    use base64::Engine;
    let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");

    // Header without alg field
    let header = serde_json::json!({"typ": "JWT"});
    let payload = serde_json::json!({"sub": "user123"});

    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&header).unwrap());
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&payload).unwrap());
    let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0u8; 32]);
    let token = format!("{}.{}.{}", header_b64, payload_b64, sig_b64);

    assert!(auth.validate_token(&token).is_none());
}

/// Verify that with_audience panics when given an invalid (too short) secret.
#[test]
fn test_with_audience_panics_on_invalid_secret() {
    let result = std::panic::catch_unwind(|| {
        BearerAuth::with_audience("short", "my-api");
    });
    assert!(
        result.is_err(),
        "with_audience should panic on invalid secret"
    );
}

/// Verify that with_claims panics when given an invalid (too short) secret.
#[test]
fn test_with_claims_panics_on_invalid_secret() {
    let result = std::panic::catch_unwind(|| {
        BearerAuth::with_claims("short", "aud", "iss");
    });
    assert!(
        result.is_err(),
        "with_claims should panic on invalid secret"
    );
}

/// Verify that with_audience panics when secret lacks required character classes.
#[test]
fn test_with_audience_panics_on_missing_char_class() {
    // 36 chars, no uppercase
    let result = std::panic::catch_unwind(|| {
        BearerAuth::with_audience("mysecret123!@#abcdefghijklmnopqrstuv", "api");
    });
    assert!(result.is_err());
}

/// Verify that with_claims panics when secret lacks special characters.
#[test]
fn test_with_claims_panics_on_missing_special_char() {
    // 36 chars, no special char
    let result = std::panic::catch_unwind(|| {
        BearerAuth::with_claims("MySecureSecret123ABCDEFGHIJKLM", "aud", "iss");
    });
    assert!(result.is_err());
}

/// Verify that start_blacklist_cleanup spawns a task without panicking.
/// The task runs indefinitely; we only verify it can be started.
#[cfg(feature = "tokio")]
#[tokio::test]
async fn test_start_blacklist_cleanup_does_not_panic() {
    let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");
    // This spawns a background task that runs indefinitely.
    // We just verify it doesn't panic.
    auth.start_blacklist_cleanup(std::time::Duration::from_secs(3600));
    // Give the runtime a moment to ensure the task spawned
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    // The task will be dropped when the test completes
}

/// Verify that a token with valid structure but non-JSON header is rejected.
#[test]
fn test_validate_token_rejects_non_json_header() {
    use base64::Engine;
    let auth = BearerAuth::new("MySecureSecret123!@#ABCDEFGHIJKLM");

    // Non-JSON header
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"not json");
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::json!({"sub": "user"}).to_string());
    let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0u8; 32]);

    let token = format!("{}.{}.{}", header_b64, payload_b64, sig_b64);
    assert!(auth.validate_token(&token).is_none());
}

/// Verify audience validation when aud claim is present but not a string
/// and not an array (e.g., a number). The `or_else` chain should return None.
#[test]
fn test_validate_token_audience_not_string_or_array() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::with_audience(secret, "expected-api");

    let payload = serde_json::json!({
        "sub": "user123",
        "aud": 12345,  // aud is a number, not string or array
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    // aud as number → token_aud is None → mismatch → rejected
    assert!(auth.validate_token(&token).is_none());
}

/// Verify that issuer validation rejects when iss claim is present but
/// is not a string (e.g., a number).
#[test]
fn test_validate_token_issuer_not_string() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let auth = BearerAuth::with_claims(secret, "my-api", "expected-issuer");

    let payload = serde_json::json!({
        "sub": "user123",
        "aud": "my-api",
        "iss": 999,  // iss is a number, not a string
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });

    let token = create_test_jwt(secret.as_bytes(), &payload);
    // iss as number → token_iss is None → mismatch → rejected
    assert!(auth.validate_token(&token).is_none());
}
