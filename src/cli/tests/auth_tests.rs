// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! CLI authentication tests.
//!
//! With the `security` feature `CliBuilder` accepts a verifier via
//! `with_auth_verifier`; `execute()` then requires credentials from the
//! `SDFORGE_TOKEN` (bearer) / `SDFORGE_API_KEY` environment variables and
//! rejects the run through the standard `error: …` + exit(1) channel before
//! any dispatch. The enforcement logic is exercised here through
//! `authenticate_cli` (the function `execute()` delegates to).

#![cfg(all(feature = "cli", feature = "security"))]

use crate::cli::CliBuilder;
use crate::cli::dispatch::{CLI_API_KEY_ENV, CLI_TOKEN_ENV, authenticate_cli};
use crate::security::SdForgeApiKeyAuth;
use crate::security::grpc_auth::{ApiKeyVerifier, BearerVerifier, GrpcAuthVerifier};
use serial_test::serial;
use std::sync::Arc;

fn clear_credentials() {
    // edition 2024：env 变更属 unsafe（进程全局状态），参照 audit 测试先例
    unsafe {
        std::env::remove_var(CLI_TOKEN_ENV);
        std::env::remove_var(CLI_API_KEY_ENV);
    }
}

fn jwt_verifier() -> BearerVerifier {
    BearerVerifier::from_secret("Cli-Test-Secret-Key-0123456789-AbCdEf").unwrap()
}

/// Mint a valid HS256 JWT for [`jwt_verifier`]'s test secret.
fn mint_jwt() -> String {
    use base64::Engine;
    // 测试用假密钥，非真实凭据
    let secret = b"Cli-Test-Secret-Key-0123456789-AbCdEf"; // pragma: allowlist secret
    let header = r#"{"alg":"HS256","typ":"JWT"}"#;
    let now = chrono::Utc::now().timestamp();
    let payload = serde_json::json!({ "sub": "cli-test", "exp": now + 3600 });
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
    let signature_input = format!("{header_b64}.{payload_b64}");
    use hmac::{KeyInit, Mac};
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
    mac.update(signature_input.as_bytes());
    let signature_b64 =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    format!("{signature_input}.{signature_b64}")
}

/// 未设置任何凭据环境变量 → 拒绝（此前 CLI 入口对鉴权零覆盖的安全债）。
#[test]
#[serial]
fn authenticate_cli_without_credentials_is_rejected() {
    clear_credentials();
    let err = authenticate_cli(&jwt_verifier()).unwrap_err();
    assert_eq!(err, "missing bearer token");
}

/// SDFORGE_TOKEN 携带无效 token → 拒绝。
#[test]
#[serial]
fn authenticate_cli_with_invalid_token_is_rejected() {
    clear_credentials();
    unsafe { std::env::set_var(CLI_TOKEN_ENV, "bogus.token.here") };
    let err = authenticate_cli(&jwt_verifier()).unwrap_err();
    assert_eq!(err, "invalid bearer token");
    clear_credentials();
}

/// SDFORGE_TOKEN 携带有效 JWT → 通过，身份与 permissions 保留。
#[test]
#[serial]
fn authenticate_cli_with_valid_token_succeeds() {
    clear_credentials();
    unsafe { std::env::set_var(CLI_TOKEN_ENV, mint_jwt()) };
    let ctx = authenticate_cli(&jwt_verifier()).expect("valid token must authenticate");
    assert_eq!(ctx.user_id(), Some("cli-test"));
    clear_credentials();
}

/// SDFORGE_API_KEY 走同一凭据管道：错误 key 拒绝，有效 key 放行。
#[test]
#[serial]
fn authenticate_cli_with_api_key_uses_same_credential_pipeline() {
    clear_credentials();
    let store = Arc::new(SdForgeApiKeyAuth::new());
    store.add_key("cli-secret-key-1".to_string(), vec!["admin".to_string()]);
    let verifier = ApiKeyVerifier::new(store, "sk_");

    unsafe { std::env::set_var(CLI_API_KEY_ENV, "sk_wrong") };
    assert_eq!(authenticate_cli(&verifier).unwrap_err(), "invalid api key");

    unsafe { std::env::set_var(CLI_API_KEY_ENV, "sk_cli-secret-key-1") };
    let ctx = authenticate_cli(&verifier).expect("valid api key must authenticate");
    assert!(ctx.has_permission("admin"));
    clear_credentials();
}

/// 未配置 verifier 的 builder 行为不变（execute 跳过认证直接派发）。
#[test]
fn builder_without_verifier_still_builds() {
    let cmd = CliBuilder::new().build();
    assert!(!cmd.get_name().is_empty());
}

/// with_auth_verifier 返回 builder 本身，链式构造可用且 build 正常。
#[test]
fn builder_with_verifier_chains_and_builds() {
    let verifier: Arc<dyn GrpcAuthVerifier> = Arc::new(jwt_verifier());
    let cmd = CliBuilder::new()
        .with_name("auth_cli")
        .with_auth_verifier(verifier)
        .build();
    assert_eq!(cmd.get_name(), "auth_cli");
}

/// execute 的认证 gate：verifier 已配置且凭据缺失 → Some(拒绝原因)，
/// execute 据此走 error: + exit(1) 通道。
#[test]
#[serial]
fn authentication_failure_reports_missing_credentials() {
    clear_credentials();
    let builder = CliBuilder::new().with_auth_verifier(Arc::new(jwt_verifier()));
    assert_eq!(
        builder.authentication_failure().as_deref(),
        Some("missing bearer token"),
        "configured verifier + absent credentials must yield a rejection"
    );
    clear_credentials();
}

/// gate 三态：无 verifier → None（行为不变）；有效凭据 → None（放行）。
#[test]
#[serial]
fn authentication_failure_passes_through_when_allowed() {
    clear_credentials();
    assert!(
        CliBuilder::new().authentication_failure().is_none(),
        "no verifier → no gate"
    );

    // edition 2024：env 变更属 unsafe（进程全局状态），参照 audit 测试先例
    unsafe {
        std::env::set_var(CLI_TOKEN_ENV, mint_jwt());
    }
    let builder = CliBuilder::new().with_auth_verifier(Arc::new(jwt_verifier()));
    assert!(
        builder.authentication_failure().is_none(),
        "valid credentials → run proceeds"
    );
    clear_credentials();
}
