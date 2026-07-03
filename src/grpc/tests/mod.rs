// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! gRPC module test suites.
//!
//! Tests are organized by responsibility:
//! - `grpc_service_tests`: `SdForgeGrpcService` methods, `GrpcRoute`/`GrpcRouteRegistration`
//!   construction & accessors, `GrpcServerConfig`, address validation, `build_server`,
//!   streaming/protobuf/error-propagation coverage
//! - `interceptor_tests`: `AuthGrpcInterceptor` JWT bearer token validation behavior

mod grpc_service_tests;
mod interceptor_tests;

// ============================================================================
// Shared test helpers — accessible by all sub-modules via `super::`
// ============================================================================

/// Generate a valid JWT for testing with the given secret and expiration timestamp
#[cfg(feature = "security")]
pub(super) fn make_test_jwt(secret: &str, exp_timestamp: i64) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let header = serde_json::json!({
        "alg": "HS256",
        "typ": "JWT"
    });
    let payload = serde_json::json!({
        "sub": "test-user",
        "exp": exp_timestamp,
        "iat": 1000000000
    });

    let header_b64 = base64url_encode(&serde_json::to_vec(&header).unwrap());
    let payload_b64 = base64url_encode(&serde_json::to_vec(&payload).unwrap());
    let signing_input = format!("{}.{}", header_b64, payload_b64);

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(signing_input.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = base64url_encode(&signature);

    format!("{}.{}.{}", header_b64, payload_b64, signature_b64)
}

/// Base64url encode (no padding) for JWT encoding.
/// Standard base64 uses `+/=`; base64url uses `-_` with no padding.
#[cfg(feature = "security")]
pub(super) fn base64url_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i] as usize;
        let b1 = if i + 1 < input.len() {
            input[i + 1] as usize
        } else {
            0
        };
        let b2 = if i + 2 < input.len() {
            input[i + 2] as usize
        } else {
            0
        };

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if i + 1 < input.len() {
            result.push(ALPHABET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        }
        if i + 2 < input.len() {
            result.push(ALPHABET[b2 & 0x3F] as char);
        }
        i += 3;
    }
    result
}
