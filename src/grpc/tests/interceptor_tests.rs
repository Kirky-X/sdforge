// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Tests for `AuthGrpcInterceptor` JWT bearer token validation behavior.

#[cfg(feature = "security")]
use super::super::make_auth_interceptor;
#[cfg(feature = "security")]
use super::make_test_jwt;
#[cfg(feature = "security")]
use tonic::service::Interceptor;

// gRPC Auth Interceptor Tests

/// Test: valid token → interceptor returns Ok
#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_valid_token() {
    let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
    let mut interceptor = make_auth_interceptor(Some(auth), None);

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
    let mut interceptor = make_auth_interceptor(Some(auth), None);

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
    let mut interceptor = make_auth_interceptor(Some(auth), None);

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
    let mut interceptor = make_auth_interceptor(Some(auth), None);

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
    let mut interceptor = make_auth_interceptor(None, None);

    // Even with an authorization header, when no auth is configured, it should pass
    let req = tonic::Request::new(());
    let result = interceptor.call(req);
    assert!(
        result.is_ok(),
        "When no auth is configured, interceptor should pass through all requests"
    );
}

// Auth Interceptor Extended Tests

#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_with_malformed_bearer_prefix() {
    let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
    let mut interceptor = make_auth_interceptor(Some(auth), None);

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
    let mut interceptor = make_auth_interceptor(Some(auth), None);

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
    let mut interceptor = make_auth_interceptor(Some(auth), None);

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
    let mut interceptor = make_auth_interceptor(Some(auth), None);

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

// Protocol-neutral verifier branch (`config.auth_verifier`)
//
// The interceptor is installed at the server level (`InterceptorLayer`), so
// the same instance guards the framework service *and* the services mounted
// through `extra_services`. The verifier branch is what makes non-bearer
// credentials (e.g. an API key) work on those services.

/// Verifier that records the credentials it was handed.
#[cfg(feature = "security")]
struct RecordingVerifier {
    accept: bool,
    seen: std::sync::Mutex<(Option<String>, Option<String>)>,
}

#[cfg(feature = "security")]
impl crate::security::grpc_auth::GrpcAuthVerifier for RecordingVerifier {
    fn verify(
        &self,
        authorization: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<crate::security::AuthContext, String> {
        *self.seen.lock().unwrap() = (
            authorization.map(str::to_string),
            api_key.map(str::to_string),
        );
        if self.accept {
            Ok(crate::security::AuthContext::new(
                Some("verifier-user".to_string()),
                vec!["read".to_string()],
                crate::security::AuthMetadata::default(),
            ))
        } else {
            Err("invalid api key".to_string())
        }
    }
}

/// verifier 放行：仅凭 API key（无 bearer）也必须通过——extra_services 上
/// 挂载的自定义 service 依赖这条路径。
#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_verifier_accepts_api_key_only_request() {
    let verifier = std::sync::Arc::new(RecordingVerifier {
        accept: true,
        seen: std::sync::Mutex::new((None, None)),
    });
    let mut interceptor = make_auth_interceptor(None, Some(verifier.clone()));

    let mut req = tonic::Request::new(());
    req.metadata_mut()
        .insert("x-api-key", "sk-valid".parse().unwrap());

    let result = interceptor.call(req);
    assert!(
        result.is_ok(),
        "the verifier must accept a request carrying only an api key"
    );
    let seen = verifier.seen.lock().unwrap();
    assert_eq!(seen.1.as_deref(), Some("sk-valid"));
    assert_eq!(seen.0, None);
}

/// verifier 拒绝：必须以 `Unauthenticated` 收场，拒绝理由按 verifier 的
/// 固定短语回显（不得回显凭据材料）。
#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_verifier_rejects_with_unauthenticated() {
    let verifier = std::sync::Arc::new(RecordingVerifier {
        accept: false,
        seen: std::sync::Mutex::new((None, None)),
    });
    let mut interceptor = make_auth_interceptor(None, Some(verifier));

    let mut req = tonic::Request::new(());
    req.metadata_mut()
        .insert("x-api-key", "sk-bad".parse().unwrap());

    let status = interceptor.call(req).unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
    assert_eq!(status.message(), "invalid api key");
}

/// verifier 优先于 bearer 配置：同时装配时，verifier 的判定为准（只配
/// `config.auth` 时 API key 请求会被拒）。
#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_verifier_takes_precedence_over_bearer() {
    let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let auth = crate::security::BearerAuth::try_new(secret).expect("valid secret");
    let verifier = std::sync::Arc::new(RecordingVerifier {
        accept: true,
        seen: std::sync::Mutex::new((None, None)),
    });
    let mut interceptor = make_auth_interceptor(Some(auth), Some(verifier));

    // 无 bearer 头：仅 bearer 的分支会拒绝，verifier 分支放行。
    let mut req = tonic::Request::new(());
    req.metadata_mut()
        .insert("x-api-key", "sk-valid".parse().unwrap());

    let result = interceptor.call(req);
    assert!(
        result.is_ok(),
        "the verifier must decide when both are configured"
    );
}

/// bearer 与 api key 两种凭据都要原样交给 verifier（协议无关校验器需要
/// 两种来源的原始值）。
#[cfg(feature = "security")]
#[test]
fn test_auth_interceptor_verifier_receives_both_credentials() {
    let verifier = std::sync::Arc::new(RecordingVerifier {
        accept: true,
        seen: std::sync::Mutex::new((None, None)),
    });
    let mut interceptor = make_auth_interceptor(None, Some(verifier.clone()));

    let mut req = tonic::Request::new(());
    req.metadata_mut()
        .insert("authorization", "Bearer token-1".parse().unwrap());
    req.metadata_mut()
        .insert("x-api-key", "sk-valid".parse().unwrap());

    let result = interceptor.call(req);
    assert!(result.is_ok());
    let seen = verifier.seen.lock().unwrap();
    assert_eq!(seen.0.as_deref(), Some("Bearer token-1"));
    assert_eq!(seen.1.as_deref(), Some("sk-valid"));
}
