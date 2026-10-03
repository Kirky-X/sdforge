// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! gRPC protocol support for Axiom
//!
//! This module provides gRPC protocol support using tonic.

/// gRPC protocol buffer module（生成物位于 OUT_DIR，见 build.rs）
pub mod sdforge_v1 {
    include!(concat!(env!("OUT_DIR"), "/sdforge.v1.rs"));
}

use crate::core::ApiMetadata;
use crate::define_registration;

mod grpc_impl;
#[cfg(all(feature = "grpc", feature = "security"))]
#[cfg(test)]
pub(crate) use grpc_impl::make_auth_interceptor;
#[allow(deprecated)]
pub use grpc_impl::{
    SdForgeGrpcService, build_server, build_server_with_config, build_server_with_graceful_shutdown,
};

/// gRPC handler registration (links `CallRequest.method` → forge handler).
pub mod handler;
pub use handler::GrpcHandlerRegistration;
#[cfg(all(feature = "grpc", feature = "streaming"))]
pub use handler::{
    GrpcStreamHandlerFn, GrpcStreamHandlerFuture, GrpcStreamHandlerRegistration, GrpcStreamItem,
    GrpcStreamOutput, stream_output_from,
};

/// gRPC route registration
#[derive(Debug, Clone)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "字段仅由 grpc 测试模块读取，非测试构建无消费者")
)]
pub struct GrpcRoute {
    /// The gRPC service name
    pub(crate) service_name: String,
    /// API metadata
    pub(crate) metadata: ApiMetadata,
}

define_registration!(GrpcRouteRegistration, GrpcRoute, ApiMetadata);

/// 自定义 tonic service 挂载回调：装配期收到 tonic
/// [`RoutesBuilder`](tonic::service::RoutesBuilder)，调用方在其上
/// `add_service(...)` 注册应用自有 proto service（消费方自建 proto 生成的
/// `XxxServer<T>` 等）。以 `Arc` 包装的 `Fn`：`GrpcServerConfig` 的 `Clone`
/// 共享同一回调列表，装配只消费每个回调一次。
pub type ExtraServiceMount =
    std::sync::Arc<dyn Fn(&mut tonic::service::RoutesBuilder) + Send + Sync>;

/// gRPC server configuration with optional JWT authentication.
#[derive(Clone)]
pub struct GrpcServerConfig {
    /// 幂等重放防护 store（feature = `idempotency`）。None = 关闭。
    #[cfg(feature = "idempotency")]
    pub idempotency_store: Option<std::sync::Arc<crate::cache::IdempotencyStore>>,
    /// HTTP/2 keepalive ping 间隔（feature-gated 可选；None = tonic 默认）。
    pub http2_keepalive_interval: Option<std::time::Duration>,
    /// HTTP/2 keepalive 超时（None = tonic 默认）。
    pub http2_keepalive_timeout: Option<std::time::Duration>,
    /// 可选 TLS（feature = `grpc-tls`）。只做接线，不做证书加载/轮换。
    ///
    /// HTTP 侧的对应物是 `sdforge::http::tls` 模块（feature = `serve-tls`，
    /// rustls 终止：证书/密钥加载 + ALPN + 可选 `ReloadingTls` 热重载）。
    /// 两者文档互链、实现独立，可各自单独启用。门控取舍：本字段类型绑定
    /// tonic 必须 cfg 门控；HTTP 侧 `ServerConfig.tls` 是零 TLS 依赖的纯
    /// 数据，故不随 feature 裁剪。
    #[cfg(feature = "grpc-tls")]
    pub tls: Option<tonic::transport::server::ServerTlsConfig>,
    /// 重放窗口秒数（feature = `idempotency`，默认 86400）。
    #[cfg(feature = "idempotency")]
    pub idempotency_ttl_secs: i64,
    /// 在途 claim 阻塞上限秒数（feature = `idempotency`，默认 30）。
    #[cfg(feature = "idempotency")]
    pub idempotency_inflight_ttl_secs: i64,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Whether authentication is required to start the server (vuln-0006).
    ///
    /// Defaults to `true` (secure default). When `true`, `build_server_with_config`
    /// refuses to start if `auth` is `None`, preventing accidental deployment of
    /// an unauthenticated gRPC server. Set to `false` for development/test only.
    pub require_auth: bool,
    /// Optional JWT authentication.
    /// When `Some`, all gRPC requests must include a valid JWT bearer token
    /// in the `authorization` metadata header.
    #[cfg(feature = "security")]
    pub auth: Option<crate::security::BearerAuth>,
    /// `security` 关闭时的空壳（恒 `None`）——字段恒存在，结构体字面量
    /// 跨 feature 形态稳定（ws-R14 复核修复）。
    #[cfg(not(feature = "security"))]
    pub auth: Option<()>,
    /// Optional application state injected into `SdForgeGrpcService`.
    ///
    /// Mirrors `CliBuilder::with_dependencies`. Handlers with a `State`
    /// parameter downcast this `Arc<dyn Any>` to their concrete type at
    /// call time. Available without the `security` feature (design D5).
    pub state: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    /// 生产 RBAC 接线（feature = security）：装配 per-call verifier，
    /// 使 `#[forge(auth(role))]` 声明在 build_server_with_config 路径生效
    /// （此前只有手动 `with_auth_interceptor` 可达，生产形态是全拒绝死开关）。
    #[cfg(feature = "security")]
    pub auth_verifier: Option<std::sync::Arc<dyn crate::security::grpc_auth::GrpcAuthVerifier>>,
    /// `security` 关闭时的空壳（恒 `None`）。
    #[cfg(not(feature = "security"))]
    pub auth_verifier: Option<()>,
    /// Optional rate limiter for gRPC requests (vuln-0006).
    ///
    /// When `Some`, each incoming gRPC `call`/`get_info` request is checked
    /// against the rate limiter using the client's remote address as the
    /// identifier.
    /// Requests that exceed the limit are rejected with
    /// `Status::resource_exhausted`. This closes the DoS vector identified
    /// in vuln-0006 (gRPC had no rate limiting while HTTP had).
    ///
    /// # Feature availability
    ///
    /// Only available when both `grpc` and `ratelimit` features are enabled.
    /// The `ratelimit` feature is independent of `http` (no axum dependency),
    /// so gRPC-only builds can still use rate limiting.
    #[cfg(feature = "ratelimit")]
    pub rate_limiter: Option<std::sync::Arc<dyn crate::security::ratelimit::RateLimiter>>,
    /// `ratelimit` 关闭时的空壳（恒 `None`）。
    #[cfg(not(feature = "ratelimit"))]
    pub rate_limiter: Option<()>,
    /// 自定义 tonic service 挂载回调：应用自有 proto service（如消费方
    /// 自建 proto 生成的 `XxxServer<T>`）与 `SdForgeService`
    /// （`SdForgeGrpcService`，经 [`build_server_with_config`] 装配）同
    /// 端口共存，无需另起进程/端口。
    ///
    /// 每个回调在装配期收到 [`tonic::service::RoutesBuilder`]，调用方在其上
    /// `add_service(...)` 注册任意实现了 `NamedService` 的 tonic service；
    /// [`build_server_with_config`] 按回调声明顺序收集后一次性挂上（
    /// `SdForgeService` 最后注册）。挂载的服务共享 server 级配置（连接
    /// 上限、超时、keepalive、TLS）与 `security` feature 下的全局 JWT
    /// 认证拦截器；`auth_verifier`（`GrpcAuthVerifier`）是
    /// `SdForgeService` 的 per-call 校验，**不**作用于自定义 service
    /// ——需要等效认证的应用应在自己的 service 内自行实现。
    ///
    /// 回调以 `Arc` 包装的 `Fn`（见 [`ExtraServiceMount`]；
    /// `GrpcServerConfig` 保留 `Clone`，克隆的配置共享同一回调列表；装配
    /// 只消费每个回调一次——闭包内以 `Arc` 捕获 service 并克隆构造即可）。
    /// 路由形态为 `/{S::NAME}/*rest`（tonic 0.14 axum 路由）：自定义
    /// service 的 NAME 不得与 `sdforge.v1.SdForgeService` 冲突，冲突在
    /// 装配期 panic（fail-loud），不做静默改名。
    pub extra_services: Vec<ExtraServiceMount>,
}

/// gRPC authentication interceptor
#[cfg(all(feature = "grpc", feature = "security"))]
#[derive(Clone)]
pub(crate) struct AuthGrpcInterceptor {
    auth: Option<crate::security::BearerAuth>,
}

#[cfg(test)]
mod tests;
