// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! gRPC 自定义 service 挂载点（`GrpcServerConfig.extra_services`）集成测试。
//!
//! P2-B：应用自有 tonic service（消费方自建 proto 生成的 server）与
//! `SdForgeService` 同端口共存。全部用例经真实网络（tonic Channel）走
//! `build_server_with_config` 装配路径——挂载接线只存在于该函数内部，
//! `serve_with_incoming` 式的绕行测不到目标行为。

#![cfg(feature = "grpc")]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use sdforge::grpc::sdforge_v1::{CallRequest, sd_forge_service_client::SdForgeServiceClient};
use sdforge::grpc::{GrpcServerConfig, build_server_with_config};

// ============================================================================
// 测试夹具
// ============================================================================

/// 探针请求/响应消息（手写 prost 消息——为测试引入 protoc 依赖不值得，
/// 且与挂载语义无关）。
#[derive(Clone, PartialEq, prost::Message)]
struct ProbeRequest {
    #[prost(string, tag = "1")]
    message: String,
}

#[derive(Clone, PartialEq, prost::Message)]
struct ProbeReply {
    #[prost(string, tag = "1")]
    message: String,
}

/// 手写最小 tonic service：只记录到达请求的 path（观测挂载路由是否命中）。
///
/// 响应故意不含 `grpc-status` trailer——本套测试断言「请求到达 + 客户端
/// 得到 Status 错误」，不构造合法 gRPC 帧（那需要复刻 tonic 的编码路径，
/// 与挂载语义无关）。
#[derive(Clone)]
struct ProbeService {
    hits: Arc<Mutex<Vec<String>>>,
}

impl tonic::server::NamedService for ProbeService {
    const NAME: &'static str = "test.v1.Probe";
}

impl tonic::codegen::Service<tonic::codegen::http::Request<tonic::body::Body>> for ProbeService {
    type Response = tonic::codegen::http::Response<tonic::body::Body>;
    type Error = std::convert::Infallible;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: tonic::codegen::http::Request<tonic::body::Body>) -> Self::Future {
        self.hits
            .lock()
            .expect("probe hits mutex poisoned")
            .push(req.uri().path().to_string());
        Box::pin(async move {
            Ok(tonic::codegen::http::Response::builder()
                .status(200)
                .header("content-type", "application/grpc")
                .body(tonic::body::Body::empty())
                .expect("static probe response"))
        })
    }
}

/// 经 `build_server_with_config` 启动真实服务器（后台任务）并等待端口就绪。
///
/// 端口获取：预 bind 127.0.0.1:0 拿可用端口后 drop，再交给服务器重 bind
/// ——测试进程内的 TOCTOU 窗口极小（本地回环 + 毫秒级），与既有 gRPC
/// 集成测试 setup 同口径。启动失败经 stderr 显性化（吞掉会把启动错误
/// 伪装成「端口永不就绪」的超时假象）。
async fn spawn_configured_server(config: GrpcServerConfig) -> SocketAddr {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe listener");
    let addr = probe.local_addr().expect("probe local_addr");
    drop(probe);

    let addr_str = addr.to_string();
    tokio::spawn(async move {
        if let Err(e) = build_server_with_config(&addr_str, config).await {
            eprintln!("gRPC server failed: {e}");
        }
    });

    for _ in 0..200 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return addr;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("gRPC server did not become ready within 5s at {addr}");
}

/// 装配带探针挂载点的 config：`extra_services` 闭包把 `ProbeService`
/// 注册到 RoutesBuilder（消费方用法与 API 草案逐行一致；service 以
/// `Arc` 捕获、闭包内克隆构造，对齐字段的 `Arc<dyn Fn>` 形态）。
fn probe_config(hits: Arc<Mutex<Vec<String>>>) -> GrpcServerConfig {
    let probe = Arc::new(ProbeService { hits });
    GrpcServerConfig {
        // require_auth fail-safe（vuln-0006）是面向部署的安全约束；本套测试
        // 关注挂载路由语义而非认证面，显式声明走 dev/test 路径（与既有
        // serve_with_incoming 绕行测试同口径）。认证语义由 security_gate 用例
        // 单独验证（配置真实 auth）。
        require_auth: false,
        extra_services: vec![Arc::new(
            move |routes: &mut tonic::service::RoutesBuilder| {
                routes.add_service(ProbeService::clone(&probe));
            },
        )],
        ..Default::default()
    }
}

/// 经 `Grpc` 通用客户端打一条 `/test.v1.Probe/Ping` unary。
///
/// 响应非合法 gRPC 帧（无 grpc-status）→ 恒返回 `Status` 错误；错误到达
/// 本身即证明请求穿越了完整网络链路并被自定义 service 接收。
async fn probe_call(addr: SocketAddr) -> Result<tonic::Response<ProbeReply>, tonic::Status> {
    let channel = tonic::transport::Channel::builder(
        format!("http://{addr}")
            .parse()
            .expect("valid endpoint uri"),
    )
    .connect()
    .await
    .expect("channel connect");
    let mut grpc = tonic::client::Grpc::new(channel);
    let path = tonic::codegen::http::uri::PathAndQuery::from_static("/test.v1.Probe/Ping");
    grpc.ready().await.expect("grpc client ready");
    grpc.unary(
        tonic::Request::new(ProbeRequest {
            message: "ping".to_string(),
        }),
        path,
        tonic_prost::ProstCodec::<ProbeRequest, ProbeReply>::default(),
    )
    .await
}

// ============================================================================
// 挂载语义用例
// ============================================================================

/// 挂载的自定义 service 在共享端口上按 `/{NAME}/*rest` 路由可达。
#[tokio::test]
async fn extra_service_receives_requests_on_shared_port() {
    let hits = Arc::new(Mutex::new(Vec::new()));
    let addr = spawn_configured_server(probe_config(Arc::clone(&hits))).await;

    let _err = probe_call(addr)
        .await
        .expect_err("probe response is not a valid gRPC frame");

    let recorded = hits.lock().expect("poisoned").clone();
    assert_eq!(recorded, ["/test.v1.Probe/Ping".to_string()]);
}

/// 挂载点不排挤 SdForgeService：unary `Call` 在同一端口照常工作。
#[tokio::test]
async fn sdforge_call_still_works_alongside_extra_services() {
    let hits = Arc::new(Mutex::new(Vec::new()));
    let addr = spawn_configured_server(probe_config(Arc::clone(&hits))).await;

    let mut client = {
        let channel = tonic::transport::Channel::builder(
            format!("http://{addr}")
                .parse()
                .expect("valid endpoint uri"),
        )
        .connect()
        .await
        .expect("connect");
        SdForgeServiceClient::new(channel)
    };
    let resp = client
        .call(tonic::Request::new(CallRequest {
            method: "extra_svc_coexist_echo".to_string(),
            parameters: Default::default(),
            data: String::new(),
        }))
        .await
        .expect("call must succeed");
    assert!(resp.get_ref().success);
    assert_eq!(resp.get_ref().data, "coexist-ok");
}

/// 未配置挂载点时装配行为与既有形态一致（回归锚点：RoutesBuilder 空集
/// 经 `add_routes` 不改变 SdForgeService 的可达性）。
#[tokio::test]
async fn no_extra_services_keeps_default_wiring() {
    // 同 probe_config：显式走 dev/test 路径绕开部署约束的 fail-safe。
    let config = GrpcServerConfig {
        require_auth: false,
        ..Default::default()
    };
    let addr = spawn_configured_server(config).await;

    let mut client = {
        let channel = tonic::transport::Channel::builder(
            format!("http://{addr}")
                .parse()
                .expect("valid endpoint uri"),
        )
        .connect()
        .await
        .expect("connect");
        SdForgeServiceClient::new(channel)
    };
    let resp = client
        .call(tonic::Request::new(CallRequest {
            method: "extra_svc_coexist_echo".to_string(),
            parameters: Default::default(),
            data: String::new(),
        }))
        .await
        .expect("call must succeed");
    assert_eq!(resp.get_ref().data, "coexist-ok");
}

// ============================================================================
// 共享 handler（供 coexist 用例经 unary Call 调用）
// ============================================================================

fn extra_svc_coexist_handler(
    args: std::collections::HashMap<String, String>,
    _state: sdforge::core::HandlerState,
) -> sdforge::core::HandlerFuture {
    let msg = args
        .get("msg")
        .cloned()
        .unwrap_or_else(|| "coexist-ok".to_string());
    Box::pin(async move { Ok(serde_json::Value::String(msg)) })
}

sdforge::inventory::submit!(sdforge::grpc::GrpcHandlerRegistration {
    method: "extra_svc_coexist_echo",
    handler: extra_svc_coexist_handler,
    body_param: None,
    default_status: None,
    roles: &[],
    i18n_key: None,
    deprecated: false,
    sunset: None,
    successor: None,
});

// ============================================================================
// security 组合：全局 JWT 拦截器对自定义 service 的覆盖
// ============================================================================

/// 全局认证拦截器挂在 server 级 layer 上（作用于整个 Routes），自定义
/// service 与 SdForgeService 同受覆盖；`auth_verifier` 是 SdForgeService
/// 的 per-call 校验，不在本用例语义内。
#[cfg(feature = "security")]
mod security_gate {
    use super::*;

    const SECRET: &str = "ExtraSvc-Test-Secret-0123456789-AbCdEf";

    // Minimal HS256 JWT minter（与 grpc_ws_auth_tests 同形）。
    fn mint_jwt(secret: &str) -> String {
        use base64::Engine;
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;

        let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(&header).unwrap());
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            serde_json::to_string(
                &serde_json::json!({"sub": "extra-svc-test", "exp": 9999999999u64}),
            )
            .unwrap(),
        );
        let signing_input = format!("{header_b64}.{payload_b64}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signing_input.as_bytes());
        let sig_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        format!("{signing_input}.{sig_b64}")
    }

    /// 携带凭证发起探针调用（凭证在请求 metadata 上）。
    async fn probe_call_with_token(
        addr: SocketAddr,
        token: Option<&str>,
    ) -> Result<tonic::Response<ProbeReply>, tonic::Status> {
        let channel = tonic::transport::Channel::builder(
            format!("http://{addr}")
                .parse()
                .expect("valid endpoint uri"),
        )
        .connect()
        .await
        .expect("channel connect");
        let mut grpc = tonic::client::Grpc::new(channel);
        let path = tonic::codegen::http::uri::PathAndQuery::from_static("/test.v1.Probe/Ping");
        let mut request = tonic::Request::new(ProbeRequest {
            message: "ping".to_string(),
        });
        if let Some(token) = token {
            request.metadata_mut().insert(
                "authorization",
                format!("Bearer {token}").parse().expect("metadata value"),
            );
        }
        grpc.ready().await.expect("grpc client ready");
        grpc.unary(
            request,
            path,
            tonic_prost::ProstCodec::<ProbeRequest, ProbeReply>::default(),
        )
        .await
    }

    #[tokio::test]
    async fn global_jwt_interceptor_covers_extra_services() {
        let hits = Arc::new(Mutex::new(Vec::new()));
        let probe = Arc::new(ProbeService {
            hits: Arc::clone(&hits),
        });
        let config = GrpcServerConfig {
            require_auth: false,
            extra_services: vec![Arc::new(
                move |routes: &mut tonic::service::RoutesBuilder| {
                    routes.add_service(ProbeService::clone(&probe));
                },
            )],
            auth: Some(sdforge::security::BearerAuth::new(SECRET)),
            ..Default::default()
        };
        let addr = spawn_configured_server(config).await;

        // 无凭证：全局拦截器在请求触达自定义 service 之前拒绝。
        let err = probe_call_with_token(addr, None)
            .await
            .expect_err("unauthenticated must be rejected");
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
        assert!(
            hits.lock().expect("poisoned").is_empty(),
            "unauthenticated request must not reach the extra service"
        );

        // 合法 JWT：请求穿越拦截器到达自定义 service（响应非法帧 →
        // Err 可接受，服务端 hits 已记录到达）。
        let token = mint_jwt(SECRET);
        let _ = probe_call_with_token(addr, Some(&token)).await;
        assert_eq!(
            hits.lock().expect("poisoned").as_slice(),
            ["/test.v1.Probe/Ping"]
        );
    }
}
