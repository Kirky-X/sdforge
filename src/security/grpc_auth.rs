// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Non-HTTP protocol authentication port.
//!
//! Protocol-neutral verifier consumed by the gRPC interceptor
//! (`SdForgeGrpcService::with_auth_interceptor`) — and usable by any other
//! transport — reusing the same credential stores as the HTTP stack
//! (`BearerAuth` JWT / `AppApiKeyAuth` API keys).

use crate::security::{AppApiKeyAuth, BearerAuth};
use std::sync::Arc;

/// Verifies transport credentials for non-HTTP protocols.
///
/// `authorization` is the raw `Authorization` header value (e.g.
/// `Bearer <jwt>`); `api_key` is the raw API key (e.g. from `x-api-key`).
/// `Err(message)` rejects the request.
pub trait GrpcAuthVerifier: Send + Sync {
    /// Verify the supplied credentials; `Err` carries the rejection reason.
    fn verify(&self, authorization: Option<&str>, api_key: Option<&str>) -> Result<(), String>;
}

/// JWT bearer verifier backed by [`BearerAuth`] (same secret/validation as
/// the HTTP middleware).
pub struct BearerVerifier {
    auth: BearerAuth,
}

impl BearerVerifier {
    /// Build from a configured `BearerAuth`.
    pub fn new(auth: BearerAuth) -> Self {
        Self { auth }
    }

    /// Build from a JWT secret (complexity-validated like the HTTP config).
    pub fn from_secret(
        secret: impl Into<String>,
    ) -> Result<Self, crate::security::AuthConfigError> {
        Ok(Self {
            auth: BearerAuth::try_new(secret)?,
        })
    }
}

impl GrpcAuthVerifier for BearerVerifier {
    fn verify(&self, authorization: Option<&str>, _api_key: Option<&str>) -> Result<(), String> {
        let token = authorization
            .and_then(|v| v.strip_prefix("Bearer "))
            .filter(|t| !t.is_empty())
            .ok_or_else(|| "missing bearer token".to_string())?;
        self.auth
            .validate_token(token)
            .map(|_| ())
            .ok_or_else(|| "invalid bearer token".to_string())
    }
}

/// API-key verifier backed by [`AppApiKeyAuth`] (same key store as the HTTP
/// middleware).
pub struct ApiKeyVerifier {
    auth: Arc<AppApiKeyAuth>,
    /// Required key prefix (e.g. `sk_`); empty = raw keys.
    prefix: String,
}

impl ApiKeyVerifier {
    /// Build from a seeded key store and prefix.
    pub fn new(auth: Arc<AppApiKeyAuth>, prefix: impl Into<String>) -> Self {
        Self {
            auth,
            prefix: prefix.into(),
        }
    }
}

impl GrpcAuthVerifier for ApiKeyVerifier {
    fn verify(&self, _authorization: Option<&str>, api_key: Option<&str>) -> Result<(), String> {
        let raw = api_key.ok_or_else(|| "missing api key".to_string())?;
        let key = if !self.prefix.is_empty() {
            raw.strip_prefix(&self.prefix)
                .ok_or_else(|| "invalid api key".to_string())?
        } else {
            raw
        };
        if key.is_empty() {
            return Err("invalid api key".to_string());
        }
        self.auth
            .validate_key(key, "unknown")
            .map(|_| ())
            .ok_or_else(|| "invalid api key".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jwt_verifier() -> BearerVerifier {
        BearerVerifier::from_secret("Grpc-Test-Secret-Key-0123456789-AbCdEf").unwrap()
    }

    #[test]
    fn bearer_verifier_accepts_only_wellformed_header() {
        let v = jwt_verifier();
        assert_eq!(v.verify(None, None).unwrap_err(), "missing bearer token");
        assert_eq!(
            v.verify(Some("Basic abc"), None).unwrap_err(),
            "missing bearer token"
        );
        assert_eq!(
            v.verify(Some("Bearer not-a-jwt"), None).unwrap_err(),
            "invalid bearer token"
        );
    }

    #[test]
    fn api_key_verifier_rejects_missing_and_wrong_keys() {
        let store = Arc::new(AppApiKeyAuth::new());
        store.add_key("secret-key-1".to_string(), vec!["admin".to_string()]);
        let v = ApiKeyVerifier::new(store, "sk_");
        assert!(v.verify(None, None).is_err());
        assert!(v.verify(None, Some("sk_wrong")).is_err());
        assert!(v.verify(None, Some("sk_secret-key-1")).is_ok());
    }
}
