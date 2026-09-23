// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Non-HTTP protocol authentication port.
//!
//! Protocol-neutral verifier consumed by the gRPC interceptor
//! (`SdForgeGrpcService::with_auth_interceptor`) — and usable by any other
//! transport — reusing the same credential stores as the HTTP stack
//! (`BearerAuth` JWT / `SdForgeApiKeyAuth` API keys).

use crate::security::{AuthContext, AuthMetadata, BearerAuth, SdForgeApiKeyAuth};
use std::sync::Arc;

/// Verifies transport credentials for non-HTTP protocols.
///
/// `authorization` is the raw `Authorization` header value (e.g.
/// `Bearer <jwt>`); `api_key` is the raw API key (e.g. from `x-api-key`).
/// `Err(message)` rejects the request.
///
/// On success the verifier returns the caller's [`AuthContext`] so callers
/// can enforce **authorization** (RBAC roles) — the previous
/// `Result<(), String>` shape discarded the identity and left non-HTTP
/// transports authenticate-only.
pub trait GrpcAuthVerifier: Send + Sync {
    /// Verify the supplied credentials; `Err` carries the rejection reason.
    fn verify(
        &self,
        authorization: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<AuthContext, String>;
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
    fn verify(
        &self,
        authorization: Option<&str>,
        _api_key: Option<&str>,
    ) -> Result<AuthContext, String> {
        let token = authorization
            .and_then(|v| v.strip_prefix("Bearer "))
            .filter(|t| !t.is_empty())
            .ok_or_else(|| "missing bearer token".to_string())?;
        self.auth
            .validate_token(token)
            .ok_or_else(|| "invalid bearer token".to_string())
    }
}

/// API-key verifier backed by [`SdForgeApiKeyAuth`] (same key store as the HTTP
/// middleware).
pub struct ApiKeyVerifier {
    auth: Arc<SdForgeApiKeyAuth>,
    /// Required key prefix (e.g. `sk_`); empty = raw keys.
    prefix: String,
}

impl ApiKeyVerifier {
    /// Build from a seeded key store and prefix.
    pub fn new(auth: Arc<SdForgeApiKeyAuth>, prefix: impl Into<String>) -> Self {
        Self {
            auth,
            prefix: prefix.into(),
        }
    }
}

impl GrpcAuthVerifier for ApiKeyVerifier {
    fn verify(
        &self,
        _authorization: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<AuthContext, String> {
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
        let permissions = self
            .auth
            .validate_key(key, "unknown")
            .ok_or_else(|| "invalid api key".to_string())?;
        Ok(AuthContext::new(
            None,
            permissions,
            AuthMetadata::new(None, None),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jwt_verifier() -> BearerVerifier {
        BearerVerifier::from_secret("Grpc-Test-Secret-Key-0123456789-AbCdEf").unwrap()
    }

    /// Mint a valid HS256 JWT for the verifier's test secret (mirrors the
    /// helper used by tests/integration/security_tests.rs).
    fn mint_jwt(user_id: &str, permissions: Vec<&str>) -> String {
        use base64::Engine;
        // 测试用假密钥，非真实凭据
        let secret = b"Grpc-Test-Secret-Key-0123456789-AbCdEf"; // pragma: allowlist secret
        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let now = chrono::Utc::now().timestamp();
        let payload = serde_json::json!({
            "sub": user_id,
            "permissions": permissions,
            "iat": now,
            "exp": now + 3600,
        });
        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
        let payload_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
        let signature_input = format!("{}.{}", header_b64, payload_b64);
        use hmac::{KeyInit, Mac};
        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
        mac.update(signature_input.as_bytes());
        let signature_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        format!("{}.{}.{}", header_b64, payload_b64, signature_b64)
    }

    /// 有效 Bearer token → Ok(AuthContext)，身份与 permissions 从 claims
    /// 提取（RBAC 下沉的身份来源）。
    #[test]
    fn bearer_verifier_returns_auth_context_with_permissions() {
        let v = jwt_verifier();
        let token = mint_jwt("grpc_test_user", vec!["admin", "read"]);
        let ctx = v
            .verify(Some(&format!("Bearer {}", token)), None)
            .expect("valid token must authenticate");
        assert_eq!(ctx.user_id(), Some("grpc_test_user"));
        assert!(ctx.has_permission("admin"));
        assert!(ctx.has_permission("read"));
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

    /// 有效 API key → Ok(AuthContext)，permissions 来自 key store
    /// （此前 `Result<(), _>` 把身份丢弃，gRPC 无法做 RBAC）。
    #[test]
    fn api_key_verifier_returns_auth_context_with_permissions() {
        let store = Arc::new(SdForgeApiKeyAuth::new());
        store.add_key("secret-key-1".to_string(), vec!["admin".to_string()]);
        let v = ApiKeyVerifier::new(store, "sk_");
        assert!(v.verify(None, None).is_err());
        assert!(v.verify(None, Some("sk_wrong")).is_err());
        let ctx = v
            .verify(None, Some("sk_secret-key-1"))
            .expect("valid key must authenticate");
        assert!(ctx.has_permission("admin"));
    }
}
