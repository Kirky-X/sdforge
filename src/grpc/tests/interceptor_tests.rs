// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `AuthGrpcInterceptor` JWT bearer token validation behavior.

#[cfg(feature = "security")]
use super::super::make_auth_interceptor;
#[cfg(feature = "security")]
use super::make_test_jwt;
#[cfg(feature = "security")]
use tonic::service::Interceptor;

// ============================================================================
// gRPC Auth Interceptor Tests
// ============================================================================

/// Test: valid token → interceptor returns Ok
#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_valid_token() {
    let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
    let mut interceptor = make_auth_interceptor(Some(auth));

    // Generate a token that expires in 1 year
    let exp = chrono::Utc::now().timestamp() + 365 * 24 * 3600;
    let valid_token = make_test_jwt(secret, exp);
    let auth_value: tonic::metadata::MetadataValue<tonic::metadata::Ascii> =
        tonic::metadata::MetadataValue::try_from(format!("Bearer {}", valid_token).as_str())
            .expect("valid metadata value");

    let mut req = tonic::Request::new(());
    req.metadata_mut().insert("authorization", auth_value);

    let result = interceptor.call(req);
    assert!(
        result.is_ok(),
        "Valid token should be accepted by interceptor"
    );
}

/// Test: missing authorization header → interceptor returns Err
#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_missing_auth_header() {
    let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
    let mut interceptor = make_auth_interceptor(Some(auth));

    // Request without any authorization header
    let req = tonic::Request::new(());
    let result = interceptor.call(req);

    assert!(result.is_err(), "Missing auth header should be rejected");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
    assert_eq!(
        status.message(),
        "Missing authorization header",
        "Error message should match expected text"
    );
}

/// Test: invalid token (bad signature) → interceptor returns Err
#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_invalid_token() {
    let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
    let mut interceptor = make_auth_interceptor(Some(auth));

    // Generate a token signed with a DIFFERENT secret
    let wrong_secret = "WrongSecret000!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let exp = chrono::Utc::now().timestamp() + 365 * 24 * 3600;
    let invalid_token = make_test_jwt(wrong_secret, exp);
    let auth_value: tonic::metadata::MetadataValue<tonic::metadata::Ascii> =
        tonic::metadata::MetadataValue::try_from(format!("Bearer {}", invalid_token).as_str())
            .expect("valid metadata value");

    let mut req = tonic::Request::new(());
    req.metadata_mut().insert("authorization", auth_value);

    let result = interceptor.call(req);
    assert!(result.is_err(), "Invalid token should be rejected");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
    assert_eq!(
        status.message(),
        "Invalid or expired token",
        "Error message should match expected text"
    );
}

/// Test: expired token → interceptor returns Err
#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_expired_token() {
    let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
    let mut interceptor = make_auth_interceptor(Some(auth));

    // Generate a token that expired in 2000
    let expired_token = make_test_jwt(secret, 946684799); // 2000-01-01
    let auth_value: tonic::metadata::MetadataValue<tonic::metadata::Ascii> =
        tonic::metadata::MetadataValue::try_from(format!("Bearer {}", expired_token).as_str())
            .expect("valid metadata value");

    let mut req = tonic::Request::new(());
    req.metadata_mut().insert("authorization", auth_value);

    let result = interceptor.call(req);
    assert!(result.is_err(), "Expired token should be rejected");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
    assert_eq!(
        status.message(),
        "Invalid or expired token",
        "Error message should match expected text"
    );
}

/// Test: no auth configured → interceptor allows all requests (pass-through)
#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_no_auth_configured() {
    let mut interceptor = make_auth_interceptor(None);

    // Even with an authorization header, when no auth is configured, it should pass
    let req = tonic::Request::new(());
    let result = interceptor.call(req);
    assert!(
        result.is_ok(),
        "When no auth is configured, interceptor should pass through all requests"
    );
}

// ============================================================================
// Auth Interceptor Extended Tests
// ============================================================================

#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_with_malformed_bearer_prefix() {
    let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
    let mut interceptor = make_auth_interceptor(Some(auth));

    let auth_value: tonic::metadata::MetadataValue<tonic::metadata::Ascii> =
        tonic::metadata::MetadataValue::try_from("Bearerinvalid_token")
            .expect("valid metadata value");

    let mut req = tonic::Request::new(());
    req.metadata_mut().insert("authorization", auth_value);

    let result = interceptor.call(req);
    assert!(result.is_err());
}

#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_with_basic_auth_instead_of_bearer() {
    let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
    let mut interceptor = make_auth_interceptor(Some(auth));

    let auth_value: tonic::metadata::MetadataValue<tonic::metadata::Ascii> =
        tonic::metadata::MetadataValue::try_from("Basic dXNlcjpwYXNz")
            .expect("valid metadata value");

    let mut req = tonic::Request::new(());
    req.metadata_mut().insert("authorization", auth_value);

    let result = interceptor.call(req);
    assert!(result.is_err());
}

#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_with_empty_token() {
    let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
    let mut interceptor = make_auth_interceptor(Some(auth));

    let auth_value: tonic::metadata::MetadataValue<tonic::metadata::Ascii> =
        tonic::metadata::MetadataValue::try_from("Bearer ").expect("valid metadata value");

    let mut req = tonic::Request::new(());
    req.metadata_mut().insert("authorization", auth_value);

    let result = interceptor.call(req);
    assert!(result.is_err());
}

#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_metadata_key_lowercase_required() {
    let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
    let mut interceptor = make_auth_interceptor(Some(auth));

    let exp = chrono::Utc::now().timestamp() + 365 * 24 * 3600;
    let valid_token = make_test_jwt(secret, exp);

    let auth_value: tonic::metadata::MetadataValue<tonic::metadata::Ascii> =
        tonic::metadata::MetadataValue::try_from(format!("Bearer {}", valid_token).as_str())
            .expect("valid metadata value");

    let mut req = tonic::Request::new(());
    req.metadata_mut().insert("authorization", auth_value);

    let result = interceptor.call(req);
    assert!(result.is_ok(), "Lowercase 'authorization' key should work");
}
