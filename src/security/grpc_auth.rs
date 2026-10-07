// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Non-HTTP protocol authentication port.
//!
//! Protocol-neutral verifier consumed by the gRPC interceptor
//! (`SdForgeGrpcService::with_auth_interceptor`), the MCP `call_tool` gate
//! (`SdForgeMcpServer::with_auth_verifier`) and the CLI gate
//! (`CliBuilder::with_auth_verifier`) — reusing the same credential stores
//! as the HTTP stack (`BearerAuth` JWT / `SdForgeApiKeyAuth` API keys).
//! [`ProtocolAuthVerifier`] is the recommended protocol-neutral alias.
//!
//! Each protocol entry wires its verifier manually (no shared config pass);
//! [`make_verifier`] builds one from an [`AuthConfig`] when a single
//! construction point is desired.

use crate::config::AuthConfig;
use crate::security::{AuthConfigError, AuthContext, AuthMetadata, BearerAuth, SdForgeApiKeyAuth};
use std::sync::Arc;

/// Protocol-neutral alias for [`GrpcAuthVerifier`].
///
/// The `Grpc` prefix is historical (the port debuted on the gRPC
/// interceptor); new code and documentation should prefer this name.
pub type ProtocolAuthVerifier = dyn GrpcAuthVerifier;

/// Verifies transport credentials for non-HTTP protocols.
///
/// `authorization` is the raw `Authorization` header value (e.g.
/// `Bearer <jwt>`); `api_key` is the raw API key (e.g. from `x-api-key`).
/// `Err(message)` rejects the request.
///
/// # Contract on the `Err` payload
///
/// The rejection reason is echoed to the remote peer (gRPC
/// `Status::unauthenticated`, MCP `-32001` error data): it must be a fixed,
/// low-detail phrase (`missing bearer token`, `invalid api key`, …) and
/// MUST NOT contain credential material or other sensitive details.
///
/// This is a documentation-level contract — the framework cannot sanitize a
/// custom `Err`. A violating implementation leaks through to untrusted
/// callers, e.g. `Err(format!("rejected key {api_key}"))` would hand the
/// submitted key (or, worse, server-side key material) to every client that
/// sends bad credentials; send the detail to your audit/log pipeline
/// instead and return `"invalid api key"`.
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

/// Verify credentials off the async worker thread.
///
/// `ApiKeyVerifier`'s constant-time defense sleeps the OS thread for up to
/// 100µs (`SdForgeApiKeyAuth::validate_key`); running `verify` directly
/// inside an async handler would block a tokio worker. Async entry points
/// (gRPC interceptor, MCP `call_tool`) route through this helper so the
/// sleep happens on the blocking pool. The verdict is identical to the
/// synchronous `verify`.
///
/// # Errors
///
/// Returns the verifier's rejection reason. A panicking verifier yields the
/// fixed phrase `authentication worker failed` (panic details go to the
/// server log, never into the returned payload — same contract as
/// [`GrpcAuthVerifier`]'s `Err`).
#[cfg(feature = "security")]
pub async fn verify_async(
    verifier: Arc<dyn GrpcAuthVerifier>,
    authorization: Option<String>,
    api_key: Option<String>,
) -> Result<AuthContext, String> {
    tokio::task::spawn_blocking(move || {
        verifier.verify(authorization.as_deref(), api_key.as_deref())
    })
    .await
    .map_err(|e| {
        log::error!("auth verifier worker failed: {e}");
        "authentication worker failed".to_string()
    })?
}

/// Build a verifier from an [`AuthConfig`] (single construction point for
/// manually-wired protocol entries).
///
/// - `Jwt` → [`BearerVerifier`] over the configured secret.
/// - `ApiKey` → [`ApiKeyVerifier`] over a store seeded from the configured
///   keys (empty key list is rejected, mirroring the HTTP build's explicit
///   error for an unusable auth configuration).
/// - `None` → error: there is no authentication to enforce; wiring a
///   verifier for it would be dead code.
///
/// # Errors
///
/// Returns [`AuthConfigError`] for disabled auth, empty key lists, or
/// secrets failing complexity validation.
pub fn make_verifier(config: &AuthConfig) -> Result<Arc<dyn GrpcAuthVerifier>, AuthConfigError> {
    match config {
        AuthConfig::None => Err(AuthConfigError::InvalidSecret(
            "authentication is disabled (AuthConfig::None); nothing to enforce".to_string(),
        )),
        AuthConfig::Jwt { secret } => Ok(Arc::new(BearerVerifier::from_secret(secret.clone())?)),
        AuthConfig::ApiKey {
            header_name: _,
            prefix,
            keys,
        } => {
            if keys.is_empty() {
                return Err(AuthConfigError::InvalidSecret(
                    "api key auth configured without any seeded keys".to_string(),
                ));
            }
            let store = Arc::new(SdForgeApiKeyAuth::new());
            for seed in keys {
                store.add_key(seed.key.clone(), seed.permissions.clone());
            }
            Ok(Arc::new(ApiKeyVerifier::new(store, prefix.clone())))
        }
    }
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
    use super::{ApiKeyVerifier, BearerVerifier, GrpcAuthVerifier, make_verifier, verify_async};
    use crate::security::SdForgeApiKeyAuth;
    use std::sync::Arc;

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

    /// verify_async 与同步 verify 判定一致（有效放行/无效拒绝），
    /// 且不阻塞调用方 worker 线程（ApiKeyVerifier 恒定时间防御会 sleep）。
    #[tokio::test]
    async fn verify_async_preserves_verifier_verdict() {
        let store = Arc::new(SdForgeApiKeyAuth::new());
        store.add_key("async-key-1".to_string(), vec!["read".to_string()]);
        let verifier: Arc<dyn GrpcAuthVerifier> = Arc::new(ApiKeyVerifier::new(store, "sk_"));

        assert!(
            verify_async(Arc::clone(&verifier), None, None)
                .await
                .is_err()
        );
        assert!(
            verify_async(Arc::clone(&verifier), None, Some("sk_wrong".into()))
                .await
                .is_err()
        );
        let ctx = verify_async(verifier, None, Some("sk_async-key-1".into()))
            .await
            .expect("valid key must authenticate off-thread");
        assert!(ctx.has_permission("read"));
    }

    /// verifier panic → 固定短语拒绝，panic 载荷不得随 Err 回显远端
    /// （与 trait Err 载荷契约一致，详情只进服务端日志）。
    #[tokio::test]
    async fn verify_async_worker_panic_returns_fixed_phrase() {
        struct PanickingVerifier;
        impl GrpcAuthVerifier for PanickingVerifier {
            fn verify(
                &self,
                _authorization: Option<&str>,
                _api_key: Option<&str>,
            ) -> Result<crate::security::AuthContext, String> {
                panic!("boom internal verifier detail");
            }
        }

        let err = verify_async(Arc::new(PanickingVerifier), None, None)
            .await
            .unwrap_err();
        assert_eq!(err, "authentication worker failed");
        assert!(!err.contains("boom"), "panic payload leaked: {err}");
    }

    /// make_verifier：JWT 配置 → BearerVerifier；API key 配置 → 种子键
    /// 预载的 ApiKeyVerifier；None → 显式报错（fail-closed，不返回空 verifier）。
    #[test]
    fn make_verifier_builds_from_auth_config() {
        use crate::config::AuthConfig;

        let jwt = make_verifier(&AuthConfig::Jwt {
            // 测试用假密钥，非真实凭据
            secret: "Make-Verifier-Secret-Key-0123456789".to_string(), // pragma: allowlist secret
        })
        .expect("jwt config must build a bearer verifier");
        assert!(jwt.verify(Some("Bearer junk"), None).is_err());

        let api_key = make_verifier(&AuthConfig::ApiKey {
            header_name: "x-api-key".to_string(),
            prefix: "sk_".to_string(),
            keys: vec![crate::config::ApiKeySeed {
                key: "seeded-key-1".to_string(),
                permissions: vec!["admin".to_string()],
            }],
        })
        .expect("api key config must build a seeded verifier");
        assert!(api_key.verify(None, Some("sk_unknown")).is_err());
        let ctx = api_key
            .verify(None, Some("sk_seeded-key-1"))
            .expect("seeded key must authenticate");
        assert!(ctx.has_permission("admin"));

        assert!(make_verifier(&AuthConfig::None).is_err());
    }
}
