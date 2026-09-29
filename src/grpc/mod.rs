// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! gRPC protocol support for Axiom
//!
//! This module provides gRPC protocol support using tonic.

#[cfg(feature = "grpc")]
/// gRPC protocol buffer module（生成物位于 OUT_DIR，见 build.rs）
pub mod sdforge_v1 {
    include!(concat!(env!("OUT_DIR"), "/sdforge.v1.rs"));
}

#[cfg(feature = "grpc")]
use crate::core::ApiMetadata;
#[cfg(feature = "grpc")]
use crate::define_registration;

mod grpc_impl;
#[cfg(all(feature = "grpc", feature = "security"))]
#[cfg(test)]
pub(crate) use grpc_impl::make_auth_interceptor;
#[cfg(feature = "grpc")]
#[allow(deprecated)]
pub use grpc_impl::{SdForgeGrpcService, build_server, build_server_with_config};

#[cfg(feature = "grpc")]
/// gRPC handler registration (links `CallRequest.method` → forge handler).
pub mod handler;
#[cfg(feature = "grpc")]
pub use handler::GrpcHandlerRegistration;
#[cfg(all(feature = "grpc", feature = "streaming"))]
pub use handler::{
    GrpcStreamHandlerFn, GrpcStreamHandlerFuture, GrpcStreamHandlerRegistration, GrpcStreamItem,
    GrpcStreamOutput, stream_output_from,
};

#[cfg(feature = "grpc")]
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

#[cfg(feature = "grpc")]
define_registration!(GrpcRouteRegistration, GrpcRoute, ApiMetadata);

#[cfg(feature = "grpc")]
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
    /// Optional application state injected into `SdForgeGrpcService`.
    ///
    /// Mirrors `CliBuilder::with_dependencies`. Handlers with a `State`
    /// parameter downcast this `Arc<dyn Any>` to their concrete type at
    /// call time. Available without the `security` feature (design D5).
    pub state: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    /// 生产 RBAC 接线（T001，feature = security）：装配 per-call verifier，
    /// 使 `#[forge(auth(role))]` 声明在 build_server_with_config 路径生效
    /// （此前只有手动 `with_auth_interceptor` 可达，生产形态是全拒绝死开关）。
    #[cfg(feature = "security")]
    pub auth_verifier: Option<std::sync::Arc<dyn crate::security::grpc_auth::GrpcAuthVerifier>>,
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
}

/// gRPC authentication interceptor
#[cfg(all(feature = "grpc", feature = "security"))]
#[derive(Clone)]
pub(crate) struct AuthGrpcInterceptor {
    auth: Option<crate::security::BearerAuth>,
}

#[cfg(test)]
#[cfg(feature = "grpc")]
mod tests;
