// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `BearerAuthBuilder` — fluent builder pattern, secret validation
//! at build time, and audience/issuer configuration.

use super::*;
use crate::security::AuthConfigError;

#[test]
fn test_bearer_auth_builder() {
    let auth = BearerAuthBuilder::new()
        .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
        .audience("my-api")
        .issuer("my-issuer")
        .build()
        .expect("Failed to build BearerAuth");

    assert!(auth.expected_audience.is_some());
    assert_eq!(auth.expected_audience.as_ref().unwrap(), "my-api");
    assert!(auth.expected_issuer.is_some());
    assert_eq!(auth.expected_issuer.as_ref().unwrap(), "my-issuer");
}

// ========================================================================
// Builder Tests
// ========================================================================

#[test]
fn test_bearer_auth_builder_minimal() {
    let auth = BearerAuth::builder()
        .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
        .build()
        .expect("Should build with valid secret");

    assert!(auth.expected_audience.is_none());
    assert!(auth.expected_issuer.is_none());
}

#[test]
fn test_bearer_auth_builder_with_audience_only() {
    let auth = BearerAuth::builder()
        .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
        .audience("my-api")
        .build()
        .expect("Should build");

    assert_eq!(auth.expected_audience, Some("my-api".to_string()));
    assert!(auth.expected_issuer.is_none());
}

#[test]
fn test_bearer_auth_builder_with_issuer_only() {
    let auth = BearerAuth::builder()
        .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
        .issuer("my-issuer")
        .build()
        .expect("Should build");

    assert!(auth.expected_audience.is_none());
    assert_eq!(auth.expected_issuer, Some("my-issuer".to_string()));
}

#[test]
fn test_bearer_auth_builder_no_secret_error() {
    let result = BearerAuthBuilder::new().build();
    assert!(result.is_err());
    let err = result.as_ref().err().unwrap();
    match err {
        AuthConfigError::InvalidSecret(msg) => {
            assert!(msg.contains("Secret is required"));
        }
        _ => panic!("Expected InvalidSecret error"),
    }
}

#[test]
fn test_bearer_auth_builder_secret_too_short() {
    let result = BearerAuthBuilder::new().secret("short").build();
    assert!(matches!(result, Err(AuthConfigError::SecretTooShort { length }) if length == 5));
}

#[test]
fn test_bearer_auth_builder_missing_uppercase() {
    let result = BearerAuthBuilder::new()
        .secret("mysecret123!@#abcdefghijklmnopqrstuv") // 36 chars, no uppercase
        .build();
    assert!(matches!(
        result,
        Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "uppercase letter"
    ));
}

#[test]
fn test_bearer_auth_builder_missing_lowercase() {
    let result = BearerAuthBuilder::new()
        .secret("MYSECRET123!@#ABCDEFGHIJKLMNOPQRSTUV") // 36 chars, no lowercase
        .build();
    assert!(matches!(
        result,
        Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "lowercase letter"
    ));
}

#[test]
fn test_bearer_auth_builder_missing_digit() {
    let result = BearerAuthBuilder::new()
        .secret("MySecureSecret!@#abcdefghijklmnopqrstuv") // 38 chars, no digit
        .build();
    assert!(matches!(
        result,
        Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "digit"
    ));
}

#[test]
fn test_bearer_auth_builder_missing_special_char() {
    let result = BearerAuthBuilder::new()
        .secret("MySecureSecret123abcdefghijklmnopqrstuv") // 41 chars, no special
        .build();
    assert!(matches!(
        result,
        Err(AuthConfigError::MissingCharacterClass { required_type }) if required_type == "special character"
    ));
}

#[test]
fn test_builder_pattern_chaining() {
    let auth = BearerAuth::builder()
        .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
        .audience("test-api")
        .issuer("test-issuer")
        .build()
        .expect("Should build with all options");

    assert_eq!(auth.expected_audience, Some("test-api".to_string()));
    assert_eq!(auth.expected_issuer, Some("test-issuer".to_string()));
}
