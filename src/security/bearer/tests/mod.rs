// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Bearer module test suites.
//!
//! Tests are organized by responsibility:
//! - `bearer_auth_tests`: Bearer authentication tests — construction, secret
//!   validation, JWT token verification, claim validation (aud/iss/exp/iat/nbf),
//!   token blacklist, constant-time comparison, and base64url decoding.
//! - `builder_tests`: `BearerAuthBuilder` tests — fluent builder pattern,
//!   secret validation at build time, and audience/issuer configuration.

mod bearer_auth_tests;
mod builder_tests;

use super::*;

// ============================================================================
// Shared test helpers — accessible by all sub-modules via `super::`
// ============================================================================

/// Create a valid JWT token for testing.
///
/// Builds an HS256-signed JWT with the provided payload using the given secret.
pub(super) fn create_test_jwt(secret: &[u8], payload: &serde_json::Value) -> String {
    use base64::Engine;
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&header).unwrap());
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(payload).unwrap());

    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(signing_input.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature);

    format!("{}.{}", signing_input, signature_b64)
}
