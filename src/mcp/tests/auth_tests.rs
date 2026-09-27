// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `SdForgeMcpServer` authentication tests.
//!
//! With the `security` feature a verifier can be attached via
//! `SdForgeMcpServer::with_auth_verifier`; every `ServerHandler::call_tool`
//! then requires transport credentials (rmcp `RequestContext.extensions`
//! carrying [`McpCredentials`]) accepted by the verifier — the same
//! `GrpcAuthVerifier` port the gRPC interceptor consumes. Without the
//! feature (or without a verifier) dispatch behavior is unchanged.

#![cfg(all(feature = "mcp", feature = "security"))]

use crate::mcp::{McpCredentials, SdForgeMcpServer};
use crate::security::SdForgeApiKeyAuth;
use crate::security::grpc_auth::{ApiKeyVerifier, BearerVerifier, GrpcAuthVerifier};
use rmcp::handler::server::ServerHandler;
use rmcp::model::ErrorCode;
use rmcp::service::serve_directly;
use std::sync::Arc;

fn jwt_verifier() -> BearerVerifier {
    BearerVerifier::from_secret("Mcp-Test-Secret-Key-0123456789-AbCdEf").unwrap()
}

/// Mint a valid HS256 JWT for [`jwt_verifier`]'s test secret (same shape the
/// HTTP middleware and the gRPC interceptor validate).
fn mint_jwt() -> String {
    use base64::Engine;
    // 测试用假密钥，非真实凭据
    let secret = b"Mcp-Test-Secret-Key-0123456789-AbCdEf"; // pragma: allowlist secret
    let header = r#"{"alg":"HS256","typ":"JWT"}"#;
    let now = chrono::Utc::now().timestamp();
    let payload = serde_json::json!({ "sub": "mcp-test", "exp": now + 3600 });
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

fn server_with(verifier: Arc<dyn GrpcAuthVerifier>) -> SdForgeMcpServer {
    SdForgeMcpServer::new().with_auth_verifier(verifier)
}

/// Attach credentials to a `RequestContext` the way a transport adapter
/// would (HTTP adapters extract headers into `McpCredentials`).
fn context_with_credentials(
    credentials: Option<McpCredentials>,
) -> rmcp::service::RequestContext<rmcp::RoleServer> {
    let server = SdForgeMcpServer::new();
    let running = serve_directly(server, DummyTransport, None);
    let peer = running.peer().clone();
    let mut context =
        rmcp::service::RequestContext::new(rmcp::model::NumberOrString::Number(0), peer);
    if let Some(creds) = credentials {
        context.extensions.insert(creds);
    }
    context
}

fn call_request(tool: &'static str) -> rmcp::model::CallToolRequestParams {
    rmcp::model::CallToolRequestParams::new(tool)
}

/// 未配置 verifier 时行为完全不变：无凭据的调用照常派发。
#[tokio::test]
async fn call_tool_without_verifier_accepts_unauthenticated_requests() {
    let server = SdForgeMcpServer::new();
    assert!(server.auth_verifier.is_none());
    let context = context_with_credentials(None);
    let result = server
        .call_tool(call_request("coverage_test_tool"), context)
        .await;
    assert!(
        result.is_ok(),
        "no verifier configured → dispatch must be unchanged"
    );
}

/// verifier 已配置但请求未携带任何凭据 → 拒绝（此前可绕过鉴权的安全债）。
#[tokio::test]
async fn call_tool_without_credentials_is_rejected() {
    let server = server_with(Arc::new(jwt_verifier()));
    let context = context_with_credentials(None);
    let err = server
        .call_tool(call_request("coverage_test_tool"), context)
        .await
        .unwrap_err();
    assert_eq!(err.code, ErrorCode(-32001));
    assert!(err.message.contains("unauthenticated"));
}

/// 携带无效 bearer token → 拒绝，工具不会执行。
#[tokio::test]
async fn call_tool_with_invalid_bearer_is_rejected() {
    let server = server_with(Arc::new(jwt_verifier()));
    let context = context_with_credentials(Some(McpCredentials {
        authorization: Some("Bearer bogus.token.here".to_string()),
        api_key: None,
    }));
    let err = server
        .call_tool(call_request("coverage_test_tool"), context)
        .await
        .unwrap_err();
    assert_eq!(err.code, ErrorCode(-32001));
}

/// 携带有效 bearer token → 通过校验并派发到工具。
#[tokio::test]
async fn call_tool_with_valid_bearer_proceeds_to_dispatch() {
    let server = server_with(Arc::new(jwt_verifier()));
    let context = context_with_credentials(Some(McpCredentials {
        authorization: Some(format!("Bearer {}", mint_jwt())),
        api_key: None,
    }));
    let result = server
        .call_tool(call_request("coverage_test_tool"), context)
        .await;
    assert!(result.is_ok(), "valid bearer must authenticate");
}

/// API key 凭据走同一 verifier 管道：错误 key 拒绝，有效 key 放行。
#[tokio::test]
async fn call_tool_with_api_key_uses_same_credential_pipeline() {
    let store = Arc::new(SdForgeApiKeyAuth::new());
    store.add_key("mcp-secret-key-1".to_string(), vec!["admin".to_string()]);
    let server = server_with(Arc::new(ApiKeyVerifier::new(store, "sk_")));

    let rejected = server
        .call_tool(
            call_request("coverage_test_tool"),
            context_with_credentials(Some(McpCredentials {
                authorization: None,
                api_key: Some("sk_wrong".to_string()),
            })),
        )
        .await
        .unwrap_err();
    assert_eq!(rejected.code, ErrorCode(-32001));

    let accepted = server
        .call_tool(
            call_request("coverage_test_tool"),
            context_with_credentials(Some(McpCredentials {
                authorization: None,
                api_key: Some("sk_mcp-secret-key-1".to_string()),
            })),
        )
        .await;
    assert!(accepted.is_ok(), "valid api key must authenticate");
}

/// stateless 适配层委托 inner.call_tool，认证强制随上下文继承：
/// 未携带凭据同样被拒绝。
#[tokio::test]
async fn stateless_handler_inherits_call_tool_enforcement() {
    let handler = crate::mcp::StatelessServerHandler::new(server_with(Arc::new(jwt_verifier())));
    let server = SdForgeMcpServer::new();
    let running = serve_directly(server, DummyTransport, None);
    let peer = running.peer().clone();
    let context = rmcp::service::RequestContext::new(rmcp::model::NumberOrString::Number(0), peer);
    let err = handler
        .call_tool(call_request("coverage_test_tool"), context)
        .await
        .unwrap_err();
    assert_eq!(err.code, ErrorCode(-32001));
}

/// stateless 适配层对 list_tools 的门控继承与 call_tool 对称：
/// 委托实现一行，但枚举防线不得因包装层遗漏。
#[tokio::test]
async fn stateless_handler_inherits_list_tools_enforcement() {
    let handler = crate::mcp::StatelessServerHandler::new(server_with(Arc::new(jwt_verifier())));
    let server = SdForgeMcpServer::new();
    let running = serve_directly(server, DummyTransport, None);
    let peer = running.peer().clone();
    let context = rmcp::service::RequestContext::new(rmcp::model::NumberOrString::Number(0), peer);
    let err = handler
        .list_tools(None, context)
        .await
        .expect_err("unauthenticated stateless list_tools must be rejected");
    assert_eq!(err.code, ErrorCode(-32001));
}

/// call_tool_with_credentials：进程内带外认证后的显式带凭据入口，
/// 与协议路径共用同一 enforce_auth 防线——无效凭据拒绝、有效凭据派发。
#[tokio::test]
async fn call_tool_with_credentials_shares_enforcement() {
    let server = server_with(Arc::new(jwt_verifier()));

    let err = server
        .call_tool_with_credentials(
            "coverage_test_tool",
            None,
            &McpCredentials::from_headers(Some("Bearer bogus.token.here"), None),
        )
        .await
        .expect_err("invalid credentials must be rejected on the explicit path");
    assert_eq!(err.code, ErrorCode(-32001));

    let ok = server
        .call_tool_with_credentials(
            "coverage_test_tool",
            None,
            &McpCredentials::from_headers(Some(&format!("Bearer {}", mint_jwt())), None),
        )
        .await;
    assert!(ok.is_ok(), "valid credentials must dispatch");
}

/// 无 verifier 时 call_tool_with_credentials 行为不变（放行派发）。
#[tokio::test]
async fn call_tool_with_credentials_admits_without_verifier() {
    let server = SdForgeMcpServer::new();
    let ok = server
        .call_tool_with_credentials(
            "coverage_test_tool",
            None,
            &McpCredentials::from_headers(None, None),
        )
        .await;
    assert!(ok.is_ok(), "no verifier → explicit path must be unchanged");
}

/// verifier 已配置时 list_tools 同样拒绝未认证请求：
/// 工具清单（名称/描述/input schema）与 call_tool 共用同一枚举防线。
#[tokio::test]
async fn list_tools_without_credentials_is_rejected() {
    let server = server_with(Arc::new(jwt_verifier()));
    let context = context_with_credentials(None);
    let err = server
        .list_tools(None, context)
        .await
        .expect_err("unauthenticated list_tools must be rejected");
    assert_eq!(err.code, ErrorCode(-32001));
}

/// 携带有效凭据的 list_tools 正常返回工具清单。
#[tokio::test]
async fn list_tools_with_valid_bearer_lists_tools() {
    let server = server_with(Arc::new(jwt_verifier()));
    let context = context_with_credentials(Some(McpCredentials {
        authorization: Some(format!("Bearer {}", mint_jwt())),
        api_key: None,
    }));
    let result = server.list_tools(None, context).await;
    assert!(result.is_ok(), "valid bearer must authenticate list_tools");
    assert!(!result.unwrap().tools.is_empty());
}

/// verifier 已配置时 call_tool_internal（无凭据通道的同步路径）
/// fail-closed 拒绝，杜绝进程内公开分发旁路。
#[test]
fn call_tool_internal_rejects_when_verifier_configured() {
    let server = server_with(Arc::new(jwt_verifier()));
    let err = server
        .call_tool_internal("coverage_test_tool", None)
        .expect_err("post-auth sync path must fail closed under a verifier");
    assert_eq!(err.code, ErrorCode(-32001));
}

/// verifier 未配置时 call_tool_internal 行为不变（既有测试与发现路径）。
#[test]
fn call_tool_internal_unaffected_without_verifier() {
    let server = SdForgeMcpServer::new();
    assert!(
        server
            .call_tool_internal("coverage_test_tool", None)
            .is_ok(),
        "no verifier → internal dispatch must be unchanged"
    );
}

/// from_headers 构造器：传输适配层从原始头值构建凭据的注入入口。
#[test]
fn mcp_credentials_from_headers_maps_raw_values() {
    let creds = McpCredentials::from_headers(Some("Bearer tok"), Some("sk_key"));
    assert_eq!(creds.authorization.as_deref(), Some("Bearer tok"));
    assert_eq!(creds.api_key.as_deref(), Some("sk_key"));
    let empty = McpCredentials::from_headers(None, None);
    assert!(empty.authorization.is_none() && empty.api_key.is_none());
}

/// Debug 输出脱敏：凭据值不得出现在格式化结果中。
#[test]
fn mcp_credentials_debug_redacts_values() {
    let creds = McpCredentials::from_headers(Some("Bearer secret-jwt"), Some("sk-secret"));
    let rendered = format!("{creds:?}");
    assert!(
        !rendered.contains("secret-jwt"),
        "leaked bearer: {rendered}"
    );
    assert!(
        !rendered.contains("sk-secret"),
        "leaked api key: {rendered}"
    );
    assert!(rendered.contains("[REDACTED]"));
}

/// 复用 server.rs tests 里的 DummyTransport（trait bound 需要本模块可见）。
#[derive(Debug)]
struct DummyTransportError;

impl std::fmt::Display for DummyTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dummy transport error")
    }
}

impl std::error::Error for DummyTransportError {}

struct DummyTransport;

#[allow(clippy::manual_async_fn)] // trait requires `+ 'static`; async fn borrows self
impl rmcp::transport::Transport<rmcp::RoleServer> for DummyTransport {
    type Error = DummyTransportError;

    fn send(
        &mut self,
        _item: rmcp::service::TxJsonRpcMessage<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send + 'static {
        async { Ok(()) }
    }

    fn receive(
        &mut self,
    ) -> impl std::future::Future<Output = Option<rmcp::service::RxJsonRpcMessage<rmcp::RoleServer>>>
    + Send {
        async { None }
    }

    fn close(&mut self) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
}
