// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use super::*;
use crate::core::{HandlerArgs, HandlerFn, HandlerState, extract_value};
use crate::grpc::handler::GrpcHandlerRegistration;

#[cfg(feature = "grpc")]
use std::collections::HashMap;
#[cfg(feature = "grpc")]
use std::sync::OnceLock;

#[cfg(feature = "grpc")]
use tonic::{Request, Response, Status, transport::Server};

#[cfg(feature = "grpc")]
use sdforge_v1::{
    CallRequest, CallResponse, InfoRequest, InfoResponse,
    sd_forge_service_server::{SdForgeService, SdForgeServiceServer},
};

/// gRPC 参数载荷大小上限（与 MCP `MAX_ARGUMENTS_SIZE_BYTES` 对齐，1 MiB）。
///
/// vuln-0002 补强：gRPC `call` 路径此前跳过 MCP 的 schema/大小校验，
/// 攻击者可通过 `parameters`/`data` 推送超大载荷触发 DoS。
/// 此处施加与 MCP 一致的大小上限作为纵深防御。
#[cfg(feature = "grpc")]
const MAX_GRPC_ARGUMENTS_SIZE_BYTES: usize = 0x10_0000;

/// gRPC service implementation.
///
/// Holds an optional application state (mirrors `CliBuilder::with_dependencies`)
/// and a lazy-initialized handler lookup cache built from
/// `inventory::iter::<GrpcHandlerRegistration>`. The cache is built once on the
/// first `call` (OnceLock semantics) and reused across all subsequent calls —
/// O(1) lookup with no repeated inventory iteration.
#[cfg(feature = "grpc")]
#[derive(Clone)]
pub struct SdForgeGrpcService {
    /// Optional application state injected via `GrpcServerConfig.state`.
    /// Handlers with `State` parameters downcast this `Arc<dyn Any>` to
    /// their concrete type at call time.
    state: HandlerState,
    /// Lazy-built `method -> handler fn` lookup table.
    handlers: OnceLock<HashMap<&'static str, HandlerFn>>,
    /// Lazy-built `method -> body_param name` lookup table.
    body_params: OnceLock<HashMap<&'static str, Option<&'static str>>>,
    /// Lazy-built `method -> roles` lookup table (endpoint RBAC).
    roles: OnceLock<HashMap<&'static str, &'static [&'static str]>>,
    /// Lazy-built `method -> macro-level status` lookup table (fix).
    /// Carries the `#[forge(status = <code>)]` argument into the gRPC layer
    /// so `call`'s success path can apply the priority chain:
    /// `ServiceResponse.status_code` > `default_status` > 200.
    default_statuses: OnceLock<HashMap<&'static str, Option<u16>>>,
    /// Optional rate limiter (vuln-0006). When `Some`, each `call` request
    /// is checked against the limiter using the client's remote address.
    #[cfg(feature = "ratelimit")]
    rate_limiter: Option<std::sync::Arc<dyn crate::security::ratelimit::RateLimiter>>,
    /// optional auth interceptor. When `Some`, every `call` must carry
    /// credentials the verifier accepts (bearer JWT / API key), otherwise the
    /// request is rejected with `Status::unauthenticated`.
    #[cfg(feature = "security")]
    auth_interceptor: Option<std::sync::Arc<dyn crate::security::grpc_auth::GrpcAuthVerifier>>,
}

#[cfg(feature = "grpc")]
impl Default for SdForgeGrpcService {
    fn default() -> Self {
        Self {
            state: None,
            handlers: OnceLock::new(),
            body_params: OnceLock::new(),
            roles: OnceLock::new(),
            default_statuses: OnceLock::new(),
            #[cfg(feature = "ratelimit")]
            rate_limiter: None,
            #[cfg(feature = "security")]
            auth_interceptor: None,
        }
    }
}

#[cfg(feature = "grpc")]
impl SdForgeGrpcService {
    /// Construct a service with injected application state (used by
    /// `build_server_with_config` to pass `GrpcServerConfig.state` through).
    #[must_use]
    pub fn with_state(state: HandlerState) -> Self {
        Self {
            state,
            handlers: OnceLock::new(),
            body_params: OnceLock::new(),
            roles: OnceLock::new(),
            default_statuses: OnceLock::new(),
            #[cfg(feature = "ratelimit")]
            rate_limiter: None,
            #[cfg(feature = "security")]
            auth_interceptor: None,
        }
    }

    /// attach an auth interceptor (gRPC authentication).
    ///
    /// Every `call` is verified before dispatch; failures map to
    /// `Status::unauthenticated`. Mirrors `GrpcServerConfig.rate_limiter`
    /// wiring for the auth dimension.
    #[cfg(feature = "security")]
    #[must_use]
    pub fn with_auth_interceptor(
        mut self,
        verifier: std::sync::Arc<dyn crate::security::grpc_auth::GrpcAuthVerifier>,
    ) -> Self {
        self.auth_interceptor = Some(verifier);
        self
    }

    /// Construct a service with injected application state and rate limiter
    /// (vuln-0006). Used by `build_server_with_config` when `ratelimit`
    /// feature is enabled to pass `GrpcServerConfig.rate_limiter` through.
    #[cfg(feature = "ratelimit")]
    #[must_use]
    pub fn with_state_and_rate_limiter(
        state: HandlerState,
        rate_limiter: Option<std::sync::Arc<dyn crate::security::ratelimit::RateLimiter>>,
    ) -> Self {
        Self {
            state,
            handlers: OnceLock::new(),
            body_params: OnceLock::new(),
            roles: OnceLock::new(),
            default_statuses: OnceLock::new(),
            rate_limiter,
            #[cfg(feature = "security")]
            auth_interceptor: None,
        }
    }

    /// Build (or reuse) the `method -> handler` cache from inventory.
    /// Idempotent: subsequent calls return the cached map (OnceLock semantics).
    #[must_use]
    fn handlers(&self) -> &HashMap<&'static str, HandlerFn> {
        self.handlers.get_or_init(|| {
            inventory::iter::<GrpcHandlerRegistration>()
                .map(|r| (r.method, r.handler))
                .collect()
        })
    }

    /// Build (or reuse) the `method -> body_param` cache from inventory.
    #[must_use]
    fn body_params(&self) -> &HashMap<&'static str, Option<&'static str>> {
        self.body_params.get_or_init(|| {
            inventory::iter::<GrpcHandlerRegistration>()
                .map(|r| (r.method, r.body_param))
                .collect()
        })
    }

    /// Build (or reuse) the `method -> default_status` cache from inventory
    /// (fix). Carries the macro-level `#[forge(status = <code>)]`
    /// argument so the gRPC success path can mirror the HTTP success code.
    #[must_use]
    fn default_statuses(&self) -> &HashMap<&'static str, Option<u16>> {
        self.default_statuses.get_or_init(|| {
            inventory::iter::<GrpcHandlerRegistration>()
                .map(|r| (r.method, r.default_status))
                .collect()
        })
    }

    /// Build (or reuse) the `method -> roles` cache from inventory.
    #[must_use]
    fn roles(&self) -> &HashMap<&'static str, &'static [&'static str]> {
        self.roles.get_or_init(|| {
            inventory::iter::<GrpcHandlerRegistration>()
                .map(|r| (r.method, r.roles))
                .collect()
        })
    }
}

#[cfg(feature = "grpc")]
impl SdForgeGrpcService {
    /// install a request context (request_id/trace_id) for the whole
    /// dispatch, so handlers and logs share the ambient correlation ids.
    async fn call_with_context(
        &self,
        request: Request<CallRequest>,
    ) -> Result<Response<CallResponse>, Status> {
        // verify credentials before any dispatch; 成功后保留身份供 RBAC 检查
        // （此前 `Result<(), _>` 把身份丢弃，gRPC 只能认证不能授权）。
        #[cfg(feature = "security")]
        let auth_ctx = if let Some(ref verifier) = self.auth_interceptor {
            let metadata = request.metadata();
            let authorization = metadata.get("authorization").and_then(|v| v.to_str().ok());
            let api_key = metadata.get("x-api-key").and_then(|v| v.to_str().ok());
            match verifier.verify(authorization, api_key) {
                Ok(ctx) => Some(ctx),
                Err(msg) => return Err(Status::unauthenticated(msg)),
            }
        } else {
            None
        };

        // endpoint RBAC（协议对等，对齐 HTTP `require_role`）：声明了 roles
        // 的方法必须由带任一匹配 permission 的已认证身份调用；security
        // feature 关闭时 fail-safe —— 一律拒绝（角色无法验证即不可满足）。
        let roles = self
            .roles()
            .get(request.get_ref().method.as_str())
            .copied()
            .unwrap_or(&[]);
        #[cfg(feature = "security")]
        let authorized =
            matches!(&auth_ctx, Some(ctx) if roles.iter().any(|r| ctx.has_permission(r)));
        #[cfg(not(feature = "security"))]
        let authorized = false;
        if !roles.is_empty() && !authorized {
            return Err(Status::permission_denied(format!(
                "missing required role: {}",
                roles.join(", ")
            )));
        }

        #[cfg(feature = "context")]
        {
            let ctx = crate::context::current_or_new();
            return crate::context::scope(ctx, self.call_inner(request)).await;
        }
        #[cfg(not(feature = "context"))]
        {
            return self.call_inner(request).await;
        }
    }
}

#[cfg(feature = "grpc")]
impl SdForgeGrpcService {
    /// 限流 guard（`call` 与 `get_info` 共用）。两个入口都始终传 Some
    /// identifier（缺 remote_addr 时 "unknown" 兜底），None 分支保留为
    /// 显式跳过语义以防未来新增调用方误伤。
    #[cfg(feature = "ratelimit")]
    async fn enforce_rate_limit(&self, identifier: Option<&str>) -> Result<(), Status> {
        let Some(ref limiter) = self.rate_limiter else {
            return Ok(());
        };
        let Some(identifier) = identifier else {
            return Ok(());
        };
        if let Err(e) = limiter.check(identifier).await {
            use crate::security::ratelimit::RateLimitError;
            let msg = match e {
                RateLimitError::Exceeded {
                    limit,
                    window_seconds,
                } => {
                    format!(
                        "rate limit exceeded: {} per {}s (client: {})",
                        limit, window_seconds, identifier
                    )
                }
                RateLimitError::Banned { reason } => {
                    format!("client banned: {} (client: {})", reason, identifier)
                }
                RateLimitError::CircuitOpen => {
                    format!("circuit breaker open (client: {})", identifier)
                }
                RateLimitError::QuotaExhausted { used, total } => {
                    format!(
                        "quota exhausted: {}/{} (client: {})",
                        used, total, identifier
                    )
                }
                RateLimitError::Limiteron(e) => {
                    format!("rate limiter error: {} (client: {})", e, identifier)
                }
            };
            return Err(Status::resource_exhausted(msg));
        }
        Ok(())
    }

    async fn call_inner(
        &self,
        request: Request<CallRequest>,
    ) -> Result<Response<CallResponse>, Status> {
        // Extract client IP from tonic's remote_addr (set by transport layer
        // from the actual TCP connection — unspoofable, unlike headers).
        // call 路径缺 remote_addr 时以 "unknown" 共享桶兜底（现行为不变）。
        #[cfg(feature = "ratelimit")]
        {
            let identifier = request
                .remote_addr()
                .map(|addr| addr.ip().to_string())
                .or_else(|| Some("unknown".to_string()));
            self.enforce_rate_limit(identifier.as_deref()).await?;
        }

        let req = request.into_inner();

        // vuln-0002 补强：gRPC 路径此前跳过 MCP 的大小校验。
        // 在 handler 调用前对 parameters + data 总大小设上限，防止超大载荷 DoS。
        let payload_size = req.parameters.values().map(|v| v.len()).sum::<usize>() + req.data.len();
        if payload_size > MAX_GRPC_ARGUMENTS_SIZE_BYTES {
            return Err(Status::invalid_argument(format!(
                "arguments payload size ({}) exceeds maximum allowed size ({})",
                payload_size, MAX_GRPC_ARGUMENTS_SIZE_BYTES
            )));
        }

        // lookup handler by method name
        let handler = self.handlers().get(req.method.as_str()).copied().ok_or_else(
            || {
                Status::not_found(format!(
                    "method '{}' not registered (no matching #[forge(grpc_method = \"...\")] declaration)",
                    req.method
                ))
            },
        )?;

        // parameters → args, data → body_param key
        let mut args: HandlerArgs = req.parameters.into_iter().collect();
        if !req.data.is_empty() {
            match self
                .body_params()
                .get(req.method.as_str())
                .copied()
                .flatten()
            {
                Some(bp) => {
                    args.insert(bp.to_string(), req.data);
                }
                None => {
                    return Err(Status::invalid_argument(format!(
                        "method '{}' has no body parameter but CallRequest.data is non-empty",
                        req.method
                    )));
                }
            }
        }

        // catch_unwind so a panicking handler never leaks internal
        // paths / stack data through gRPC error messages (security rule).
        use futures_util::FutureExt;
        use std::panic::AssertUnwindSafe;
        let outcome = AssertUnwindSafe(handler(args, self.state.clone()))
            .catch_unwind()
            .await;

        match outcome {
            Ok(Ok(value)) => {
                // smart extract_value (String → raw, others → JSON)
                // forge-success-status-code 优先级链 —
                //   ServiceResponse.status_code 字段（动态入口）
                //   > 宏 #[forge(status = <code>)] 参数（静态入口，default_status）
                //   > 200（零破坏默认）
                // extract_status_code 仅在序列化输出含 `success` 字段且带
                // `status_code` 时返回 Some（避免裸类型误判）；否则用
                // default_status fallback；两者皆无则 200。
                let default_status = self
                    .default_statuses()
                    .get(req.method.as_str())
                    .copied()
                    .flatten();
                let status_code = extract_status_code(&value)
                    .or(default_status.map(|s| s as i32))
                    .unwrap_or(200);
                Ok(Response::new(CallResponse {
                    success: true,
                    data: extract_value(&value),
                    error: String::new(),
                    status_code,
                }))
            }
            Ok(Err(e)) => {
                // business error → 真实 gRPC Status（vuln-SIMPL-002 修复）：
                // 此前走 Status::ok + body success:false，标准 gRPC 客户端、
                // 监控错误率与重试/熔断策略对该批错误全部失明。现按统一
                // 映射表落到 tonic::Code，details 携带 UnifiedError JSON
                //（机器可读 code/field/trace_id 不丢失）。
                let unified = crate::error::unified::UnifiedError::from(&e);
                let code = crate::error::unified::grpc_code_for(&e);
                Err(Status::with_details(
                    code,
                    unified.message.clone(),
                    tonic::codegen::Bytes::from(unified.to_json().to_string()),
                ))
            }
            Err(_panic) => {
                // handler panicked → generic internal error.
                // Never expose panic payload to the client (security).
                Err(Status::internal("handler panicked"))
            }
        }
    }
}

#[cfg(feature = "grpc")]
#[tonic::async_trait]
impl SdForgeService for SdForgeGrpcService {
    async fn call(&self, request: Request<CallRequest>) -> Result<Response<CallResponse>, Status> {
        self.call_with_context(request).await
    }

    async fn get_info(
        &self,
        request: Request<InfoRequest>,
    ) -> Result<Response<InfoResponse>, Status> {
        // 限流覆盖 get_info（T018）：与 call 统一以 "unknown" 兜底
        // （tonic 无 remote_addr 注入口，生产环境 TCP 连接恒有地址）。
        #[cfg(feature = "ratelimit")]
        {
            let identifier = request
                .remote_addr()
                .map(|addr| addr.ip().to_string())
                .or_else(|| Some("unknown".to_string()));
            self.enforce_rate_limit(identifier.as_deref()).await?;
        }
        let response = InfoResponse {
            name: "SdForge Service".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            methods: self
                .handlers()
                .keys()
                .map(|k| (*k).to_string())
                .collect::<Vec<_>>(),
            description: "SdForge Multi-Protocol SDK Framework".to_string(),
        };

        Ok(Response::new(response))
    }
}

/// Extract the success-side `status_code` from a handler return value.
///
/// gRPC handlers return `serde_json::Value` (the forge fn's return value
/// serialized via `serde_json::to_value`). When the fn returns a
/// `ServiceResponse`, the serialized object carries a `status_code` field
/// (only when set via `success_with_status` — `skip_serializing_if` omits it
/// otherwise). This helper reads that field so gRPC clients see the same
/// success status code as HTTP clients.
///
/// # Duck-typing contract
///
/// Detection is **structural**, not nominal: any JSON object that
/// simultaneously contains a `success` boolean field AND a numeric
/// `status_code` field will be matched, regardless of whether the upstream
/// Rust type is actually `ServiceResponse`. This is intentional — it is the
/// only way to inspect the serialized `Value` produced by the handler
/// without a downstream-type registry. Users who return custom envelope
/// types that happen to contain both fields will have their `status_code`
/// read here; this is a known, documented coupling rather than a bug.
/// To avoid surprises, custom envelope types should not use the
/// `(success, status_code)` field pair unless they intend to participate in
/// this protocol.
///
/// # Returns
///
/// - `Some(code)` when the value is a JSON object containing both a
///   `success` key and a numeric `status_code` key whose value fits in the
///   HTTP status code range `100..=999` (prevents `u64 → i32`
///   truncation from out-of-range inputs).
/// - `None` otherwise — the caller applies the `default_status` fallback
///   (macro `status` argument) and finally 200.
#[cfg(feature = "grpc")]
fn extract_status_code(value: &serde_json::Value) -> Option<i32> {
    value
        .as_object()
        .filter(|obj| obj.contains_key("success"))
        .and_then(|obj| obj.get("status_code"))
        .and_then(|v| v.as_u64())
        // 防止 u64 → i32 截断（仅接受有效 HTTP 状态码范围 100..=999）
        .filter(|&u| (100..=999).contains(&u))
        .map(|u| u as i32)
}

// ApiError → HTTP/gRPC 映射已收敛到 `crate::error::unified`
// （`mapping_for` / `grpc_code_for`）单一事实来源。

#[cfg(feature = "grpc")]
impl GrpcRoute {
    #[allow(missing_docs)]
    pub fn new(service_name: String, metadata: ApiMetadata) -> Self {
        Self {
            service_name,
            metadata,
        }
    }

    #[cfg(test)]
    pub(crate) fn service_name(&self) -> &str {
        &self.service_name
    }

    #[cfg(test)]
    pub(crate) fn metadata(&self) -> &ApiMetadata {
        &self.metadata
    }
}

/// Build gRPC server
///
/// # Deprecated
///
/// `build_server` starts an **unauthenticated** gRPC server with no way to
/// configure authentication. Use [`build_server_with_config`] with a
/// [`GrpcServerConfig`] that has `auth` configured instead.
#[cfg(feature = "grpc")]
#[deprecated(
    note = "use build_server_with_config with auth configured; build_server starts an unauthenticated server"
)]
pub async fn build_server(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Security fix: Validate address format before parsing to prevent information disclosure
    let addr = match addr.parse::<std::net::SocketAddr>() {
        Ok(addr) => addr,
        Err(e) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid gRPC server address format: {}", e),
            )));
        }
    };
    let service = SdForgeGrpcService::default();

    // Limit request size to 4MB to prevent large message attacks
    Server::builder()
        .add_service(SdForgeServiceServer::new(service).max_decoding_message_size(4 * 1024 * 1024))
        .serve(addr)
        .await?;

    Ok(())
}

/// Build gRPC server with custom configuration and optional JWT authentication.
///
/// When `config.auth` is `Some`, all gRPC requests must include a valid JWT bearer token
/// in the `authorization` metadata header. Invalid tokens result in `UNAUTHENTICATED` status.
///
/// # Security (vuln-0006)
///
/// When `config.require_auth` is `true` (the default) and `config.auth` is `None`,
/// this function refuses to start, preventing accidental deployment of an
/// unauthenticated gRPC server. Set `require_auth = false` only for
/// development/test environments.
#[cfg(feature = "grpc")]
pub async fn build_server_with_config(
    addr: &str,
    config: GrpcServerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Security fix: Validate address format before parsing to prevent information disclosure
    let addr = match addr.parse::<std::net::SocketAddr>() {
        Ok(addr) => addr,
        Err(e) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid gRPC server address format: {}", e),
            )));
        }
    };

    // vuln-0006: refuse to start an unauthenticated server when require_auth is true.
    // This check runs after address validation (so invalid addresses still report
    // the address error) but before any server binding (so it fails fast).
    #[cfg(feature = "security")]
    if config.require_auth && config.auth.is_none() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "gRPC server requires authentication but no auth configured \
             (set GrpcServerConfig.require_auth = false to override)",
        )));
    }
    // pass `config.state` into the service so handlers with State
    // parameters can downcast it at call time.
    // vuln-0006: pass `config.rate_limiter` when `ratelimit` feature is enabled.
    #[cfg(feature = "ratelimit")]
    let service =
        SdForgeGrpcService::with_state_and_rate_limiter(config.state, config.rate_limiter);
    #[cfg(not(feature = "ratelimit"))]
    let service = SdForgeGrpcService::with_state(config.state);

    // Build server with optional JWT auth interceptor
    #[cfg(feature = "security")]
    let mut builder = {
        let auth_interceptor = make_auth_interceptor(config.auth.clone());
        Server::builder().layer(tonic::service::InterceptorLayer::new(auth_interceptor))
    };
    #[cfg(not(feature = "security"))]
    let mut builder = { Server::builder() };

    if config.max_connections > 0 {
        builder = builder.concurrency_limit_per_connection(config.max_connections);
    }
    if config.timeout_seconds > 0 {
        builder = builder.timeout(std::time::Duration::from_secs(config.timeout_seconds));
    }

    builder
        .add_service(SdForgeServiceServer::new(service).max_decoding_message_size(4 * 1024 * 1024))
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(feature = "grpc")]
impl Default for GrpcServerConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            timeout_seconds: 30,
            require_auth: true, // vuln-0006: secure default
            #[cfg(feature = "security")]
            auth: None,
            state: None,
            #[cfg(feature = "ratelimit")]
            rate_limiter: None,
        }
    }
}

/// Create a gRPC authentication interceptor from an optional BearerAuth config.
#[cfg(all(feature = "grpc", feature = "security"))]
pub(crate) fn make_auth_interceptor(
    auth: Option<crate::security::BearerAuth>,
) -> AuthGrpcInterceptor {
    AuthGrpcInterceptor { auth }
}

#[cfg(all(feature = "grpc", feature = "security"))]
impl tonic::service::Interceptor for AuthGrpcInterceptor {
    fn call(&mut self, req: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        let Some(ref bearer_auth) = self.auth else {
            return Ok(req);
        };

        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(String::from);

        match token {
            Some(token_str) => {
                if bearer_auth.validate_token(&token_str).is_some() {
                    Ok(req)
                } else {
                    Err(Status::unauthenticated("Invalid or expired token"))
                }
            }
            None => Err(Status::unauthenticated("Missing authorization header")),
        }
    }
}

#[cfg(feature = "grpc")]
impl SdForgeGrpcService {
    /// Test-only accessor: borrow the body_param map.
    #[cfg(test)]
    pub(crate) fn body_params_map(&self) -> &HashMap<&'static str, Option<&'static str>> {
        self.body_params()
    }

    /// Test-only accessor: borrow the `default_status` map.
    #[cfg(test)]
    pub(crate) fn default_statuses_map(&self) -> &HashMap<&'static str, Option<u16>> {
        self.default_statuses()
    }
}

// ============================================================================
// Unit tests
// ============================================================================
#[cfg(all(test, feature = "grpc"))]
mod tests {
    use super::*;
    use crate::core::{ApiError, HandlerArgs};
    use serde_json::Value;
    use std::sync::Arc;
    use tonic::Request;

    /// Test probe handler — registered via `inventory::submit!` below.
    /// Returns its `msg` argument unchanged so the routing test can assert
    /// real handler invocation (not the stub `processed` response).
    fn echo_handler(args: HandlerArgs, _state: HandlerState) -> crate::core::HandlerFuture {
        let msg = args.get("msg").cloned().unwrap_or_default();
        Box::pin(async move { Ok(Value::String(msg)) })
    }

    inventory::submit! {
            GrpcHandlerRegistration {
                method: "test_echo",
                handler: echo_handler,
                body_param: None,
                default_status: None,
                roles: &[],
    }
        }

    /// Handler that always returns `Err(NotFound)` — verifies the error →
    /// `success:false` + `status_code:404` + `Status::ok` mapping.
    fn not_found_handler(_args: HandlerArgs, _state: HandlerState) -> crate::core::HandlerFuture {
        Box::pin(async {
            Err(ApiError::NotFound {
                resource: "test_resource".to_string(),
                resource_id: Some("123".to_string()),
            })
        })
    }

    inventory::submit! {
            GrpcHandlerRegistration {
                method: "test_not_found",
                handler: not_found_handler,
                body_param: None,
                default_status: None,
                roles: &[],
    }
        }

    /// Handler that panics — verifies `catch_unwind` returns `Status::internal`
    /// without leaking the panic payload.
    fn panic_handler(_args: HandlerArgs, _state: HandlerState) -> crate::core::HandlerFuture {
        Box::pin(async {
            panic!("boom — must not leak to client");
        })
    }

    inventory::submit! {
            GrpcHandlerRegistration {
                method: "test_panic",
                handler: panic_handler,
                body_param: None,
                default_status: None,
                roles: &[],
    }
        }

    /// Handler with a Body parameter — verifies `data` is injected into
    /// the body_param key.
    fn body_handler(args: HandlerArgs, _state: HandlerState) -> crate::core::HandlerFuture {
        let payload = args.get("payload").cloned().unwrap_or_default();
        Box::pin(async move { Ok(Value::String(payload)) })
    }

    inventory::submit! {
            GrpcHandlerRegistration {
                method: "test_body",
                handler: body_handler,
                body_param: Some("payload"),
                default_status: None,
                roles: &[],
    }
        }

    /// RBAC probe — declares `roles = &["admin"]`; all calls must present an
    /// identity carrying the `admin` permission (or be denied).
    fn rbac_admin_handler(_args: HandlerArgs, _state: HandlerState) -> crate::core::HandlerFuture {
        Box::pin(async { Ok(Value::String("admin-ok".to_string())) })
    }

    inventory::submit! {
        GrpcHandlerRegistration {
            method: "test_rbac_admin",
            handler: rbac_admin_handler,
            body_param: None,
            default_status: None,
            roles: &["admin"],
        }
    }

    // ========================================================================
    // forge-success-status-code: gRPC status_code 透传测试 handlers
    // ========================================================================

    /// Handler returning a `ServiceResponse` with `status_code = 201` —
    /// verifies the gRPC layer reads the field and populates
    /// `CallResponse.status_code` (dynamic path).
    fn status_code_handler(_args: HandlerArgs, _state: HandlerState) -> crate::core::HandlerFuture {
        Box::pin(async move {
            let resp = crate::core::ServiceResponse::success_with_status("created", 201);
            serde_json::to_value(&resp).map_err(|e| {
                ApiError::internal_error(
                    format!("failed to serialize ServiceResponse: {e}"),
                    "test.serialize",
                )
            })
        })
    }

    inventory::submit! {
            GrpcHandlerRegistration {
                method: "test_status_code",
                handler: status_code_handler,
                body_param: None,
                default_status: None,
                roles: &[],
    }
        }

    /// Handler returning a `ServiceResponse` without `status_code` (plain
    /// `success`) — verifies the gRPC layer defaults to 200 when the field
    /// is absent (zero-breaking).
    fn service_response_no_status_handler(
        _args: HandlerArgs,
        _state: HandlerState,
    ) -> crate::core::HandlerFuture {
        Box::pin(async move {
            let resp = crate::core::ServiceResponse::success("plain");
            serde_json::to_value(&resp).map_err(|e| {
                ApiError::internal_error(
                    format!("failed to serialize ServiceResponse: {e}"),
                    "test.serialize",
                )
            })
        })
    }

    inventory::submit! {
            GrpcHandlerRegistration {
                method: "test_service_response_no_status",
                handler: service_response_no_status_handler,
                body_param: None,
                default_status: None,
                roles: &[],
    }
        }

    // ========================================================================
    // forge-success-status-code gRPC 路径消费宏 `status` 参数测试
    //
    // 验证优先级链：ServiceResponse.status_code 字段 > 宏 default_status > 200。
    // ========================================================================

    /// 裸类型返回值 + `default_status = Some(201)` — 模拟
    /// `#[forge(grpc_method = "test_bare_with_default_status", status = 201)]`
    /// 的宏展开效果。handler 返回 `Value::String`（无 `status_code` 字段），
    /// 期望 CallResponse.status_code == 201（来自 default_status fallback）。
    fn bare_type_with_default_status_handler(
        args: HandlerArgs,
        _state: HandlerState,
    ) -> crate::core::HandlerFuture {
        let msg = args.get("msg").cloned().unwrap_or_default();
        Box::pin(async move { Ok(Value::String(msg)) })
    }

    inventory::submit! {
            GrpcHandlerRegistration {
                method: "test_bare_with_default_status",
                handler: bare_type_with_default_status_handler,
                body_param: None,
                default_status: Some(201),
                roles: &[],
    }
        }

    /// `ServiceResponse::success`（无 status_code 字段）+ `default_status = Some(202)` —
    /// 验证当 ServiceResponse 自身未设置 status_code 时，default_status 作为 fallback 生效。
    fn service_response_with_default_status_handler(
        _args: HandlerArgs,
        _state: HandlerState,
    ) -> crate::core::HandlerFuture {
        Box::pin(async move {
            let resp = crate::core::ServiceResponse::success("accepted");
            serde_json::to_value(&resp).map_err(|e| {
                ApiError::internal_error(
                    format!("failed to serialize ServiceResponse: {e}"),
                    "test.serialize",
                )
            })
        })
    }

    inventory::submit! {
            GrpcHandlerRegistration {
                method: "test_service_response_with_default_status",
                handler: service_response_with_default_status_handler,
                body_param: None,
                default_status: Some(202),
                roles: &[],
    }
        }

    /// `ServiceResponse::success_with_status("x", 208)` + `default_status = Some(201)` —
    /// 验证 ServiceResponse.status_code 字段优先于 default_status（字段 > 宏 > 200）。
    fn service_response_field_overrides_default_status_handler(
        _args: HandlerArgs,
        _state: HandlerState,
    ) -> crate::core::HandlerFuture {
        Box::pin(async move {
            let resp = crate::core::ServiceResponse::success_with_status("override", 208);
            serde_json::to_value(&resp).map_err(|e| {
                ApiError::internal_error(
                    format!("failed to serialize ServiceResponse: {e}"),
                    "test.serialize",
                )
            })
        })
    }

    inventory::submit! {
            GrpcHandlerRegistration {
                method: "test_service_response_field_overrides_default",
                handler: service_response_field_overrides_default_status_handler,
                body_param: None,
                default_status: Some(201),
                roles: &[],
    }
        }

    #[test]
    fn lookup_builds_cache_from_inventory() {
        // OnceLock is initialized lazily on first `handlers()` call.
        let service = SdForgeGrpcService::default();
        let table = service.handlers();
        assert!(table.contains_key("test_echo"));
        assert!(table.contains_key("test_not_found"));
        assert!(table.contains_key("test_panic"));
        assert!(table.contains_key("test_body"));
        // 新增的 default_status 测试 handler 也必须被 inventory 收集
        assert!(table.contains_key("test_bare_with_default_status"));
        assert!(table.contains_key("test_service_response_with_default_status"));
        assert!(table.contains_key("test_service_response_field_overrides_default"));
    }

    /// `default_statuses` cache 从 inventory 正确构建。
    #[test]
    fn default_statuses_cache_built_correctly() {
        let service = SdForgeGrpcService::default();
        let map = service.default_statuses_map();
        // 无宏 status 参数 → None
        assert_eq!(map.get("test_echo"), Some(&None));
        assert_eq!(map.get("test_status_code"), Some(&None));
        // 宏 status 参数 → Some(code)
        assert_eq!(
            map.get("test_bare_with_default_status"),
            Some(&Some(201u16))
        );
        assert_eq!(
            map.get("test_service_response_with_default_status"),
            Some(&Some(202u16))
        );
        assert_eq!(
            map.get("test_service_response_field_overrides_default"),
            Some(&Some(201u16))
        );
    }

    #[test]
    fn lookup_cache_is_idempotent() {
        // subsequent `handlers()` calls return the same map.
        let service = SdForgeGrpcService::default();
        let first = service.handlers();
        let second = service.handlers();
        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn body_params_cache_built_correctly() {
        let service = SdForgeGrpcService::default();
        let map = service.body_params_map();
        assert_eq!(map.get("test_echo"), Some(&None));
        assert_eq!(map.get("test_body"), Some(&Some("payload")));
    }

    #[tokio::test]
    async fn call_routes_to_registered_handler() {
        // real routing replaces stub `processed` response.
        let service = SdForgeGrpcService::default();
        let mut params = HashMap::new();
        params.insert("msg".to_string(), "hello world".to_string());
        let req = Request::new(CallRequest {
            method: "test_echo".to_string(),
            parameters: params,
            data: String::new(),
        });
        let resp = service.call(req).await.unwrap().into_inner();
        assert!(resp.success);
        assert_eq!(resp.data, "hello world");
        assert_eq!(resp.status_code, 200);
        assert!(resp.error.is_empty());
    }

    #[tokio::test]
    async fn call_unknown_method_returns_not_found() {
        // method not registered → Status::not_found
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "no_such_method".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let err = service.call(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
        assert!(err.message().contains("no_such_method"));
    }

    #[tokio::test]
    async fn call_data_without_body_param_returns_invalid_argument() {
        // data non-empty but method has no body_param → invalid_argument
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_echo".to_string(),
            parameters: HashMap::new(),
            data: "unexpected payload".to_string(),
        });
        let err = service.call(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn call_routes_data_to_body_param() {
        // data injected into body_param key
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_body".to_string(),
            parameters: HashMap::new(),
            data: "{\"x\":1}".to_string(),
        });
        let resp = service.call(req).await.unwrap().into_inner();
        assert!(resp.success);
        assert_eq!(resp.data, "{\"x\":1}");
    }

    #[tokio::test]
    async fn call_business_error_returns_real_grpc_status() {
        // business error → 真实 Status（不再是 Status::ok + success:false body）。
        // NotFound → Status::not_found，details 携带 UnifiedError JSON。
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_not_found".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let err = service.call(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
        let details: serde_json::Value =
            serde_json::from_slice(err.details()).expect("details must be UnifiedError JSON");
        assert_eq!(details["code"], "NOT_FOUND");
        assert!(
            details["message"]
                .as_str()
                .unwrap()
                .contains("test_resource")
        );
    }

    /// RateLimitExceeded → Status::resource_exhausted（与限流器拦截路径同码，
    /// 消除双形态）。
    #[tokio::test]
    async fn call_rate_limit_error_maps_to_resource_exhausted() {
        fn rate_limit_handler(
            _args: HandlerArgs,
            _state: HandlerState,
        ) -> crate::core::HandlerFuture {
            Box::pin(async {
                Err(ApiError::RateLimitExceeded {
                    limit: 10,
                    window_seconds: 60,
                })
            })
        }
        inventory::submit! {
                    GrpcHandlerRegistration {
                        method: "test_rate_limit_err",
                        handler: rate_limit_handler,
                        body_param: None,
                        default_status: None,
                        roles: &[],
        }
                }
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_rate_limit_err".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let err = service.call(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::ResourceExhausted);
        let details: serde_json::Value = serde_json::from_slice(err.details()).unwrap();
        assert_eq!(details["code"], "TOO_MANY_REQUESTS");
    }

    /// ValidationError → Status::invalid_argument，details 带 422 语义码与
    /// field 字段。
    #[tokio::test]
    async fn call_validation_error_maps_to_invalid_argument_with_field() {
        fn validation_handler(
            _args: HandlerArgs,
            _state: HandlerState,
        ) -> crate::core::HandlerFuture {
            Box::pin(async {
                Err(ApiError::ValidationError {
                    field: "email".to_string(),
                    constraint: "format".to_string(),
                })
            })
        }
        inventory::submit! {
                    GrpcHandlerRegistration {
                        method: "test_validation_err",
                        handler: validation_handler,
                        body_param: None,
                        default_status: None,
                        roles: &[],
        }
                }
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_validation_err".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let err = service.call(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        let details: serde_json::Value = serde_json::from_slice(err.details()).unwrap();
        assert_eq!(details["code"], "UNPROCESSABLE_ENTITY");
        assert_eq!(details["field"], "email");
    }

    #[tokio::test]
    async fn call_panic_handler_returns_status_internal() {
        // handler panic → Status::internal, message generic
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_panic".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let err = service.call(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::Internal);
        // security: panic payload must NOT leak
        assert!(!err.message().contains("boom"));
        assert!(!err.message().contains("leak"));
    }

    // ========================================================================
    // forge-success-status-code: gRPC status_code 透传测试
    //
    // 成功 status_code 透传（ServiceResponse 字段 → CallResponse）
    // 错误路径不回归（已由 call_business_error_returns_success_false_with_status_code 覆盖）
    // ========================================================================

    /// fn 返回 success_with_status(d, 201) → CallResponse.status_code == 201。
    #[tokio::test]
    async fn call_returns_status_code_from_service_response_field() {
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_status_code".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let resp = service.call(req).await.unwrap().into_inner();
        assert!(resp.success);
        assert_eq!(
            resp.status_code, 201,
            "CallResponse.status_code must reflect ServiceResponse.status_code field"
        );
    }

    /// 裸类型返回值（无 ServiceResponse）→ CallResponse.status_code == 200。
    #[tokio::test]
    async fn call_bare_type_returns_default_200() {
        let service = SdForgeGrpcService::default();
        let mut params = HashMap::new();
        params.insert("msg".to_string(), "hello".to_string());
        let req = Request::new(CallRequest {
            method: "test_echo".to_string(),
            parameters: params,
            data: String::new(),
        });
        let resp = service.call(req).await.unwrap().into_inner();
        assert!(resp.success);
        assert_eq!(
            resp.status_code, 200,
            "bare-type return must default to 200 (no ServiceResponse field)"
        );
    }

    /// ServiceResponse::success（无 status_code 字段）→ 200（零破坏）。
    #[tokio::test]
    async fn call_service_response_without_status_code_defaults_200() {
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_service_response_no_status".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let resp = service.call(req).await.unwrap().into_inner();
        assert!(resp.success);
        assert_eq!(
            resp.status_code, 200,
            "ServiceResponse without status_code field must default to 200"
        );
    }

    /// extract_status_code 单元测试：边界与防御逻辑。
    ///
    /// 函数返回 `Option<i32>`（None 表示 caller 应使用 default_status fallback）。
    /// 范围 100..=999 之外的 status_code 被过滤为 None（防 u64→i32 截断）。
    #[test]
    fn extract_status_code_handles_various_values() {
        use serde_json::json;
        // ServiceResponse 带 status_code
        let v = json!({"success": true, "data": "x", "status_code": 201});
        assert_eq!(extract_status_code(&v), Some(201));
        // ServiceResponse 无 status_code（skip_serializing_if）→ None
        let v = json!({"success": true, "data": "x"});
        assert_eq!(extract_status_code(&v), None);
        // 裸类型（无 success 字段）即使有 status_code 也不误判 → None
        let v = json!({"name": "alice", "status_code": 999});
        assert_eq!(extract_status_code(&v), None);
        // 非 object → None
        assert_eq!(extract_status_code(&json!("string")), None);
        assert_eq!(extract_status_code(&json!(42)), None);
        assert_eq!(extract_status_code(&json!(null)), None);
        // 边界码：100/999 在范围内
        let v = json!({"success": true, "status_code": 100});
        assert_eq!(extract_status_code(&v), Some(100));
        let v = json!({"success": true, "status_code": 999});
        assert_eq!(extract_status_code(&v), Some(999));
        // 范围外（< 100 或 > 999）→ None（防截断）
        let v = json!({"success": true, "status_code": 99});
        assert_eq!(
            extract_status_code(&v),
            None,
            "status_code < 100 must be rejected (LOW-2)"
        );
        let v = json!({"success": true, "status_code": 1000});
        assert_eq!(
            extract_status_code(&v),
            None,
            "status_code > 999 must be rejected (LOW-2)"
        );
        let v = json!({"success": true, "status_code": 65535});
        assert_eq!(
            extract_status_code(&v),
            None,
            "u16 max value must be rejected (LOW-2 truncation guard)"
        );
        // status_code 字段为非数字 → None
        let v = json!({"success": true, "status_code": "201"});
        assert_eq!(extract_status_code(&v), None);
        let v = json!({"success": true, "status_code": null});
        assert_eq!(extract_status_code(&v), None);
    }

    // ========================================================================
    // forge-success-status-code gRPC 路径消费宏 `status` 参数 e2e 测试
    //
    // 优先级链：ServiceResponse.status_code 字段 > 宏 default_status > 200
    // ========================================================================

    /// 裸类型 + `default_status = Some(201)` → CallResponse.status_code == 201。
    /// 模拟 `#[forge(grpc_method = "...", status = 201)]` 的端到端行为。
    #[tokio::test]
    async fn call_bare_type_with_default_status_returns_201() {
        let service = SdForgeGrpcService::default();
        let mut params = HashMap::new();
        params.insert("msg".to_string(), "created".to_string());
        let req = Request::new(CallRequest {
            method: "test_bare_with_default_status".to_string(),
            parameters: params,
            data: String::new(),
        });
        let resp = service.call(req).await.unwrap().into_inner();
        assert!(resp.success);
        assert_eq!(
            resp.status_code, 201,
            "H-1: bare-type with macro status=201 must return 201 via default_status fallback"
        );
        assert_eq!(resp.data, "created");
    }

    /// ServiceResponse::success（无字段）+ `default_status = Some(202)` → 202。
    #[tokio::test]
    async fn call_service_response_no_field_with_default_status_returns_202() {
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_service_response_with_default_status".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let resp = service.call(req).await.unwrap().into_inner();
        assert!(resp.success);
        assert_eq!(
            resp.status_code, 202,
            "H-1: ServiceResponse::success with macro status=202 must return 202 via fallback"
        );
    }

    /// 优先级链：ServiceResponse.status_code 字段(208) > 宏 default_status(201)。
    #[tokio::test]
    async fn call_service_response_field_overrides_default_status() {
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_service_response_field_overrides_default".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let resp = service.call(req).await.unwrap().into_inner();
        assert!(resp.success);
        assert_eq!(
            resp.status_code, 208,
            "H-1: ServiceResponse.status_code field (208) must override macro default_status (201)"
        );
    }

    #[test]
    fn grpc_error_mapping_smoke_uses_unified_table() {
        // smoke：映射职责在 unified::grpc_code_for（全变体单测见 unified.rs），
        // 此处仅锚定本模块错误路径引用的入口未被移除。
        let e = ApiError::NotFound {
            resource: "x".into(),
            resource_id: None,
        };
        assert_eq!(
            crate::error::unified::grpc_code_for(&e),
            tonic::Code::NotFound
        );
        let (status, code) = crate::error::unified::mapping_for(&e);
        assert_eq!((status, code), (404, "NOT_FOUND"));
    }

    #[test]
    fn default_state_is_none() {
        // Default::default().state == None
        let config = GrpcServerConfig::default();
        assert!(config.state.is_none());
    }

    #[tokio::test]
    async fn state_injected_to_service_can_be_downcast() {
        // state injected via GrpcServerConfig reaches the service.
        use std::any::Any;
        let state: Arc<dyn Any + Send + Sync> = Arc::new(42_i32);
        let service = SdForgeGrpcService::with_state(Some(state));
        // Downcast back to i32 to verify the value survived.
        // (Real handlers use `downcast_state` from core::handler.)
        let borrowed = service.state.clone().unwrap();
        let downcast = borrowed.downcast_ref::<i32>();
        assert_eq!(downcast, Some(&42_i32));
    }

    // ========================================================================
    // vuln-0006: gRPC rate limiting tests
    // ========================================================================
    #[cfg(feature = "ratelimit")]
    mod vuln_0006_ratelimit_tests {
        use super::*;
        use crate::security::ratelimit::{RateLimitError, RateLimiter};
        use std::future::Future;
        use std::pin::Pin;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        /// Mock rate limiter that always allows. Verifies that gRPC calls
        /// proceed normally when the rate limit is not exceeded.
        struct AlwaysAllowLimiter;

        impl RateLimiter for AlwaysAllowLimiter {
            fn check<'a>(
                &'a self,
                _identifier: &'a str,
            ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
                Box::pin(async { Ok(()) })
            }
        }

        /// Mock rate limiter that always rejects with `Exceeded`. Verifies
        /// that gRPC calls are rejected with `Status::resource_exhausted`.
        struct AlwaysRejectLimiter;

        impl RateLimiter for AlwaysRejectLimiter {
            fn check<'a>(
                &'a self,
                _identifier: &'a str,
            ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
                Box::pin(async {
                    Err(RateLimitError::Exceeded {
                        limit: 10,
                        window_seconds: 60,
                    })
                })
            }
        }

        /// Mock rate limiter that counts check calls. Verifies the limiter
        /// is actually invoked once per gRPC call.
        struct CountingLimiter {
            count: AtomicU32,
        }

        impl RateLimiter for CountingLimiter {
            fn check<'a>(
                &'a self,
                _identifier: &'a str,
            ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
                self.count.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok(()) })
            }
        }

        #[tokio::test]
        async fn call_without_rate_limiter_proceeds_normally() {
            // Baseline: no rate limiter → call should succeed as before.
            let service = SdForgeGrpcService::default();
            let mut params = HashMap::new();
            params.insert("msg".to_string(), "hello".to_string());
            let req = Request::new(CallRequest {
                method: "test_echo".to_string(),
                parameters: params,
                data: String::new(),
            });
            let resp = service.call(req).await.unwrap().into_inner();
            assert!(resp.success);
            assert_eq!(resp.data, "hello");
        }

        #[tokio::test]
        async fn call_with_allowing_limiter_proceeds_normally() {
            // vuln-0006: rate limiter that allows → call should succeed.
            let limiter: Arc<dyn RateLimiter> = Arc::new(AlwaysAllowLimiter);
            let service = SdForgeGrpcService::with_state_and_rate_limiter(None, Some(limiter));
            let mut params = HashMap::new();
            params.insert("msg".to_string(), "allowed".to_string());
            let req = Request::new(CallRequest {
                method: "test_echo".to_string(),
                parameters: params,
                data: String::new(),
            });
            let resp = service.call(req).await.unwrap().into_inner();
            assert!(resp.success);
            assert_eq!(resp.data, "allowed");
        }

        #[tokio::test]
        async fn call_with_rejecting_limiter_returns_resource_exhausted() {
            // vuln-0006: rate limiter that rejects → Status::resource_exhausted.
            // The handler must NOT be invoked (rate limit check is before dispatch).
            let limiter: Arc<dyn RateLimiter> = Arc::new(AlwaysRejectLimiter);
            let service = SdForgeGrpcService::with_state_and_rate_limiter(None, Some(limiter));
            let req = Request::new(CallRequest {
                method: "test_echo".to_string(),
                parameters: HashMap::new(),
                data: String::new(),
            });
            let err = service.call(req).await.unwrap_err();
            assert_eq!(
                err.code(),
                tonic::Code::ResourceExhausted,
                "vuln-0006: rejected rate limit must return ResourceExhausted, got {:?}",
                err.code()
            );
            assert!(
                err.message().contains("rate limit exceeded"),
                "error message should mention rate limit, got: {}",
                err.message()
            );
        }

        #[tokio::test]
        async fn call_with_rate_limiter_invokes_check_once_per_call() {
            // vuln-0006: verify the limiter is actually called exactly once
            // per gRPC call (not zero times due to a bug, not multiple times).
            let limiter = Arc::new(CountingLimiter {
                count: AtomicU32::new(0),
            });
            let count_clone = Arc::clone(&limiter);
            let limiter_dyn: Arc<dyn RateLimiter> = limiter as Arc<dyn RateLimiter>;
            let service = SdForgeGrpcService::with_state_and_rate_limiter(None, Some(limiter_dyn));

            let req = Request::new(CallRequest {
                method: "test_echo".to_string(),
                parameters: HashMap::new(),
                data: String::new(),
            });
            let _ = service.call(req).await;

            assert_eq!(
                count_clone.count.load(Ordering::SeqCst),
                1,
                "vuln-0006: rate limiter check must be called exactly once per gRPC call"
            );
        }

        #[tokio::test]
        async fn call_with_banned_limiter_returns_resource_exhausted() {
            // vuln-0006: banned identifier → ResourceExhausted with ban reason.
            struct AlwaysBannedLimiter;
            impl RateLimiter for AlwaysBannedLimiter {
                fn check<'a>(
                    &'a self,
                    _identifier: &'a str,
                ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>>
                {
                    Box::pin(async {
                        Err(RateLimitError::Banned {
                            reason: "abuse detected".to_string(),
                        })
                    })
                }
            }
            let limiter: Arc<dyn RateLimiter> = Arc::new(AlwaysBannedLimiter);
            let service = SdForgeGrpcService::with_state_and_rate_limiter(None, Some(limiter));
            let req = Request::new(CallRequest {
                method: "test_echo".to_string(),
                parameters: HashMap::new(),
                data: String::new(),
            });
            let err = service.call(req).await.unwrap_err();
            assert_eq!(err.code(), tonic::Code::ResourceExhausted);
            assert!(
                err.message().contains("banned") && err.message().contains("abuse detected"),
                "error should mention ban reason, got: {}",
                err.message()
            );
        }

        #[test]
        fn default_config_has_no_rate_limiter() {
            // vuln-0006: default GrpcServerConfig.rate_limiter is None
            // (opt-in, backward compatible).
            let config = GrpcServerConfig::default();
            assert!(
                config.rate_limiter.is_none(),
                "default config should not enable rate limiting"
            );
        }

        /// T018: 拒绝型限流器下 get_info → resource_exhausted，不泄漏方法清单。
        #[tokio::test]
        async fn get_info_rate_limited_returns_resource_exhausted() {
            let limiter: Arc<dyn RateLimiter> = Arc::new(AlwaysRejectLimiter);
            let service = SdForgeGrpcService::with_state_and_rate_limiter(None, Some(limiter));
            let err = service
                .get_info(Request::new(InfoRequest {
                    version: String::new(),
                }))
                .await
                .unwrap_err();
            assert_eq!(err.code(), tonic::Code::ResourceExhausted);
        }

        /// T018: 计数型限流器下 call 与 get_info 各恰好触发一次 check。
        #[tokio::test]
        async fn rate_limit_guard_covers_call_and_get_info_once() {
            let limiter = Arc::new(CountingLimiter {
                count: AtomicU32::new(0),
            });
            let count_clone = Arc::clone(&limiter);
            let limiter_dyn: Arc<dyn RateLimiter> = limiter as Arc<dyn RateLimiter>;
            let service = SdForgeGrpcService::with_state_and_rate_limiter(None, Some(limiter_dyn));

            let req = Request::new(CallRequest {
                method: "test_echo".to_string(),
                parameters: HashMap::new(),
                data: String::new(),
            });
            let _ = service.call(req).await;
            let _ = service
                .get_info(Request::new(InfoRequest {
                    version: String::new(),
                }))
                .await;
            assert_eq!(
                count_clone.count.load(Ordering::SeqCst),
                2,
                "call 与 get_info 各触发一次限流检查"
            );
        }
    }

    // ========================================================================
    // T011: gRPC endpoint RBAC（协议对等）— fail-safe / 放行 / 低权限拒绝
    // ========================================================================

    /// T017: get_info 版本号取 CARGO_PKG_VERSION（不再硬编码 0.1.0）。
    #[tokio::test]
    async fn get_info_returns_crate_version() {
        let service = SdForgeGrpcService::default();
        let resp = service
            .get_info(Request::new(InfoRequest {
                version: String::new(),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(resp.version, env!("CARGO_PKG_VERSION"));
    }

    /// 未配置任何认证（或身份缺失）时，声明了 roles 的方法必须拒绝
    /// （fail-safe：角色无法验证即不可满足）。
    #[cfg(feature = "security")]
    #[tokio::test]
    async fn rbac_denies_unauthenticated_caller() {
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_rbac_admin".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let err = service.call(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::PermissionDenied);
        assert!(err.message().contains("admin"));
    }

    /// 持有匹配 permission 的合法凭证放行；低权限凭证 permission_denied。
    #[cfg(feature = "security")]
    #[tokio::test]
    async fn rbac_allows_authorized_key_and_rejects_underprivileged() {
        use crate::security::grpc_auth::ApiKeyVerifier;

        let store = std::sync::Arc::new(crate::security::SdForgeApiKeyAuth::new());
        store.add_key("admin-key".to_string(), vec!["admin".to_string()]);
        store.add_key("basic-key".to_string(), vec!["basic".to_string()]);
        let service = SdForgeGrpcService::default()
            .with_auth_interceptor(std::sync::Arc::new(ApiKeyVerifier::new(store, "")));

        // admin key → 放行
        let mut req = Request::new(CallRequest {
            method: "test_rbac_admin".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        req.metadata_mut()
            .insert("x-api-key", "admin-key".parse().unwrap());
        let resp = service.call(req).await.unwrap().into_inner();
        assert_eq!(resp.data, "admin-ok");

        // basic key → permission_denied，handler 未执行
        let mut req = Request::new(CallRequest {
            method: "test_rbac_admin".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        req.metadata_mut()
            .insert("x-api-key", "basic-key".parse().unwrap());
        let err = service.call(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::PermissionDenied);
    }

    /// roles 为空的方法不做角色检查（认证是否要求由 interceptor 配置决定）。
    #[cfg(feature = "security")]
    #[tokio::test]
    async fn rbac_skips_methods_without_roles() {
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_echo".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let resp = service.call(req).await.unwrap().into_inner();
        assert!(resp.success);
    }

    /// security feature 关闭时 fail-safe：声明了 roles 的方法一律拒绝
    /// （无认证栈可验证角色）。`cargo test --features grpc`（不含
    /// security）下编译并生效。
    #[cfg(not(feature = "security"))]
    #[tokio::test]
    async fn rbac_fail_safe_denies_without_security_feature() {
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_rbac_admin".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let err = service.call(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::PermissionDenied);
    }
}
