// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use super::*;
use crate::core::{HandlerArgs, HandlerFn, HandlerState, extract_value};
#[cfg(feature = "grpc")]
use crate::grpc::handler::{GrpcHandlerRegistration, attach_lifecycle_metadata};

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

#[cfg(all(feature = "grpc", feature = "streaming"))]
use crate::grpc::handler::{GrpcStreamHandlerFn, GrpcStreamHandlerRegistration};
#[cfg(all(feature = "grpc", feature = "streaming"))]
use futures_util::StreamExt;

/// gRPC 参数载荷大小上限（与 MCP `MAX_ARGUMENTS_SIZE_BYTES` 对齐，1 MiB）。
///
/// vuln-0002 补强：gRPC `call` 路径此前跳过 MCP 的 schema/大小校验，
/// 攻击者可通过 `parameters`/`data` 推送超大载荷触发 DoS。
/// 此处施加与 MCP 一致的大小上限作为纵深防御。
#[cfg(feature = "grpc")]
const MAX_GRPC_ARGUMENTS_SIZE_BYTES: usize = 0x10_0000;

/// 幂等防护的绑定参数（store + 重放窗口），feature = `idempotency`。
#[cfg(all(feature = "grpc", feature = "idempotency"))]
#[derive(Clone)]
struct IdempotencyGuard {
    store: std::sync::Arc<crate::cache::IdempotencyStore>,
    ttl_secs: i64,
}

/// 端点生命周期声明的缓存形态（与 `GrpcHandlerRegistration` 的
/// `deprecated` / `sunset` / `successor` 字段同形，全 `Copy` 无分配）。
#[cfg(feature = "grpc")]
type GrpcLifecycle = (bool, Option<&'static str>, Option<&'static str>);

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
    /// Lazy-built `method -> endpoint lifecycle` lookup table
    /// (`#[forge(deprecated, sunset, successor)]` → 成功响应 metadata 注入).
    /// Copy-expansion 与注册结构体同形（`bool` + `Option<&str>`×2）。
    lifecycles: OnceLock<HashMap<&'static str, GrpcLifecycle>>,
    /// Lazy-built `method -> streaming handler fn` lookup table
    /// (feature = `streaming`). Streaming methods live apart from unary
    /// ones so each RPC reaches exactly its own registry.
    #[cfg(feature = "streaming")]
    stream_handlers: OnceLock<HashMap<&'static str, GrpcStreamHandlerFn>>,
    /// Lazy-built `method -> body_param name` lookup table for streaming
    /// methods (feature = `streaming`).
    #[cfg(feature = "streaming")]
    stream_body_params: OnceLock<HashMap<&'static str, Option<&'static str>>>,
    /// Lazy-built `method -> roles` lookup table for streaming methods
    /// (feature = `streaming`).
    #[cfg(feature = "streaming")]
    stream_roles: OnceLock<HashMap<&'static str, &'static [&'static str]>>,
    /// Lazy-built `method -> macro-level status` lookup table for streaming
    /// methods (feature = `streaming`); applied per stream item.
    #[cfg(feature = "streaming")]
    stream_default_statuses: OnceLock<HashMap<&'static str, Option<u16>>>,
    /// Lazy-built `method -> endpoint lifecycle` lookup table for streaming
    /// methods（与 unary 的 [`SdForgeGrpcService::lifecycles`] 同契约，
    /// feature = `streaming`）。
    #[cfg(feature = "streaming")]
    stream_lifecycles: OnceLock<HashMap<&'static str, GrpcLifecycle>>,
    /// Optional rate limiter (vuln-0006). When `Some`, each `call` request
    /// is checked against the limiter using the client's remote address.
    #[cfg(feature = "ratelimit")]
    rate_limiter: Option<std::sync::Arc<dyn crate::security::ratelimit::RateLimiter>>,
    /// optional auth interceptor. When `Some`, every `call` must carry
    /// credentials the verifier accepts (bearer JWT / API key), otherwise the
    /// request is rejected with `Status::unauthenticated`.
    #[cfg(feature = "security")]
    auth_interceptor: Option<std::sync::Arc<dyn crate::security::grpc_auth::GrpcAuthVerifier>>,
    /// 幂等重放防护（T022，feature = `idempotency`）：携带 `idempotency-key`
    /// metadata 的请求经 store 三态防护，其余零开销。
    #[cfg(feature = "idempotency")]
    idempotency: Option<IdempotencyGuard>,
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
            lifecycles: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_handlers: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_body_params: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_roles: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_default_statuses: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_lifecycles: OnceLock::new(),
            #[cfg(feature = "ratelimit")]
            rate_limiter: None,
            #[cfg(feature = "security")]
            auth_interceptor: None,
            #[cfg(feature = "idempotency")]
            idempotency: None,
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
            lifecycles: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_handlers: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_body_params: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_roles: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_default_statuses: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_lifecycles: OnceLock::new(),
            #[cfg(feature = "ratelimit")]
            rate_limiter: None,
            #[cfg(feature = "security")]
            auth_interceptor: None,
            #[cfg(feature = "idempotency")]
            idempotency: None,
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
            lifecycles: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_handlers: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_body_params: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_roles: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_default_statuses: OnceLock::new(),
            #[cfg(feature = "streaming")]
            stream_lifecycles: OnceLock::new(),
            rate_limiter,
            #[cfg(feature = "security")]
            auth_interceptor: None,
            #[cfg(feature = "idempotency")]
            idempotency: None,
        }
    }

    /// attach an idempotency store (T022)。携带 `idempotency-key` metadata
    /// 的请求进入三态防护（Execute / InFlight → already_exists / Replay）。
    #[cfg(all(feature = "grpc", feature = "idempotency"))]
    #[must_use]
    pub fn with_idempotency_store(
        mut self,
        store: std::sync::Arc<crate::cache::IdempotencyStore>,
        ttl_secs: i64,
    ) -> Self {
        self.idempotency = Some(IdempotencyGuard { store, ttl_secs });
        self
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

    /// Build (or reuse) the `method -> streaming handler` cache from
    /// inventory (feature = `streaming`). Streaming methods live in a
    /// separate registry so each RPC reaches exactly its own table.
    #[cfg(feature = "streaming")]
    #[must_use]
    fn stream_handlers(&self) -> &HashMap<&'static str, GrpcStreamHandlerFn> {
        self.stream_handlers.get_or_init(|| {
            inventory::iter::<GrpcStreamHandlerRegistration>()
                .map(|r| (r.method, r.handler))
                .collect()
        })
    }

    /// Build (or reuse) the `method -> body_param` cache for streaming
    /// methods (feature = `streaming`).
    #[cfg(feature = "streaming")]
    #[must_use]
    fn stream_body_params(&self) -> &HashMap<&'static str, Option<&'static str>> {
        self.stream_body_params.get_or_init(|| {
            inventory::iter::<GrpcStreamHandlerRegistration>()
                .map(|r| (r.method, r.body_param))
                .collect()
        })
    }

    /// Build (or reuse) the `method -> roles` cache for streaming methods
    /// (feature = `streaming`).
    #[cfg(feature = "streaming")]
    #[must_use]
    fn stream_roles(&self) -> &HashMap<&'static str, &'static [&'static str]> {
        self.stream_roles.get_or_init(|| {
            inventory::iter::<GrpcStreamHandlerRegistration>()
                .map(|r| (r.method, r.roles))
                .collect()
        })
    }

    /// Build (or reuse) the `method -> default_status` cache for streaming
    /// methods (feature = `streaming`); applied per stream item.
    #[cfg(feature = "streaming")]
    #[must_use]
    fn stream_default_statuses(&self) -> &HashMap<&'static str, Option<u16>> {
        self.stream_default_statuses.get_or_init(|| {
            inventory::iter::<GrpcStreamHandlerRegistration>()
                .map(|r| (r.method, r.default_status))
                .collect()
        })
    }

    /// Build (or reuse) the `method -> endpoint lifecycle` cache from
    /// inventory（`#[forge(deprecated, sunset, successor)]` → 成功响应
    /// metadata 注入的查找表）。
    #[cfg(feature = "grpc")]
    #[must_use]
    fn lifecycles(&self) -> &HashMap<&'static str, GrpcLifecycle> {
        self.lifecycles.get_or_init(|| {
            inventory::iter::<GrpcHandlerRegistration>()
                .map(|r| (r.method, (r.deprecated, r.sunset, r.successor)))
                .collect()
        })
    }

    /// Build (or reuse) the `method -> endpoint lifecycle` cache for
    /// streaming methods（与 unary 的 [`SdForgeGrpcService::lifecycles`]
    /// 同契约，feature = `streaming`）。
    #[cfg(feature = "streaming")]
    #[must_use]
    fn stream_lifecycles(&self) -> &HashMap<&'static str, GrpcLifecycle> {
        self.stream_lifecycles.get_or_init(|| {
            inventory::iter::<GrpcStreamHandlerRegistration>()
                .map(|r| (r.method, (r.deprecated, r.sunset, r.successor)))
                .collect()
        })
    }
}

#[cfg(feature = "grpc")]
impl SdForgeGrpcService {
    /// 凭据验证（`call` 与 `call_stream` 共用）：配置了拦截器时验证
    /// Bearer/API-key 凭据，成功后保留身份供 RBAC 检查（此前
    /// `Result<(), _>` 把身份丢弃，gRPC 只能认证不能授权）。走
    /// verify_async：ApiKeyVerifier 恒定时间防御会 sleep OS 线程，不能
    /// 阻塞 tokio worker（与 MCP call_tool 门同一通路）。
    #[cfg(feature = "security")]
    async fn authenticate(
        &self,
        request: &Request<CallRequest>,
    ) -> Result<Option<crate::security::AuthContext>, Status> {
        if let Some(ref verifier) = self.auth_interceptor {
            let metadata = request.metadata();
            let authorization = metadata
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            let api_key = metadata
                .get("x-api-key")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            match crate::security::grpc_auth::verify_async(
                std::sync::Arc::clone(verifier),
                authorization,
                api_key,
            )
            .await
            {
                Ok(ctx) => Ok(Some(ctx)),
                Err(msg) => Err(Status::unauthenticated(msg)),
            }
        } else {
            Ok(None)
        }
    }

    /// endpoint RBAC（协议对等，对齐 HTTP `require_role`；`call` 与
    /// `call_stream` 共用）：声明了 roles 的方法必须由带任一匹配
    /// permission 的已认证身份调用；security feature 关闭时 fail-safe ——
    /// 一律拒绝（角色无法验证即不可满足）。
    #[cfg(all(feature = "grpc", feature = "security"))]
    fn ensure_authorized(
        &self,
        roles: &[&str],
        auth_ctx: &Option<crate::security::AuthContext>,
    ) -> Result<(), Status> {
        let authorized =
            matches!(auth_ctx, Some(ctx) if roles.iter().any(|r| ctx.has_permission(r)));
        if !roles.is_empty() && !authorized {
            Err(Status::permission_denied(format!(
                "missing required role: {}",
                roles.join(", ")
            )))
        } else {
            Ok(())
        }
    }

    /// fail-safe 变体：security 关闭时非空 roles 一律拒绝（与开启路径的
    /// `authorized = false` 语义逐字一致）。
    #[cfg(all(feature = "grpc", not(feature = "security")))]
    fn ensure_authorized(&self, roles: &[&str]) -> Result<(), Status> {
        if !roles.is_empty() {
            Err(Status::permission_denied(format!(
                "missing required role: {}",
                roles.join(", ")
            )))
        } else {
            Ok(())
        }
    }

    /// 载荷上限校验（`call` 与 `call_stream` 共用；vuln-0002 补强）：
    /// parameters + data 总大小超限即拒绝，防止超大载荷 DoS。
    #[cfg(feature = "grpc")]
    fn ensure_payload_size(req: &CallRequest) -> Result<(), Status> {
        let payload_size = req.parameters.values().map(|v| v.len()).sum::<usize>() + req.data.len();
        if payload_size > MAX_GRPC_ARGUMENTS_SIZE_BYTES {
            Err(Status::invalid_argument(format!(
                "arguments payload size ({}) exceeds maximum allowed size ({})",
                payload_size, MAX_GRPC_ARGUMENTS_SIZE_BYTES
            )))
        } else {
            Ok(())
        }
    }

    /// 参数装配（`call` 与 `call_stream` 共用）：parameters → args，
    /// data → body_param 键（body_param 缺失而 data 非空 → 拒绝）。
    #[cfg(feature = "grpc")]
    fn build_handler_args(
        req: CallRequest,
        method: &str,
        body_param: Option<&'static str>,
    ) -> Result<HandlerArgs, Status> {
        let mut args: HandlerArgs = req.parameters.into_iter().collect();
        if !req.data.is_empty() {
            match body_param {
                Some(bp) => {
                    args.insert(bp.to_string(), req.data);
                }
                None => {
                    return Err(Status::invalid_argument(format!(
                        "method '{}' has no body parameter but CallRequest.data is non-empty",
                        method
                    )));
                }
            }
        }
        Ok(args)
    }

    /// install a request context (request_id/trace_id) for the whole
    /// dispatch, so handlers and logs share the ambient correlation ids.
    /// 守卫顺序：限流 → 认证 → RBAC（与 `CallStream` 同序，见
    /// `call_stream` 文档的顺序取舍说明）。
    async fn call_with_context(
        &self,
        request: Request<CallRequest>,
    ) -> Result<Response<CallResponse>, Status> {
        // 限流最外层：未认证/坏凭据的洪水在进入恒时验证（sleep OS 线程）
        // 之前即被拒绝——认证前置会让每条被拒请求都烧一次线程睡眠。
        #[cfg(feature = "ratelimit")]
        {
            let identifier = request
                .remote_addr()
                .map(|addr| addr.ip().to_string())
                .or_else(|| Some("unknown".to_string()));
            self.enforce_rate_limit(identifier.as_deref()).await?;
        }

        #[cfg(feature = "security")]
        let auth_ctx = self.authenticate(&request).await?;

        let roles = self
            .roles()
            .get(request.get_ref().method.as_str())
            .copied()
            .unwrap_or(&[]);
        #[cfg(feature = "security")]
        self.ensure_authorized(roles, &auth_ctx)?;
        #[cfg(not(feature = "security"))]
        self.ensure_authorized(roles)?;

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
        // 限流已上移至 call_with_context 最外层（与 CallStream 同序）。

        // T022: 幂等 key 提取（仅当配置了 store 且请求携带 idempotency-key
        // metadata 时参与）。metadata 须在 into_inner 消费前读取；scope 绑定
        // gRPC method 名防跨端点键冲突。
        // T003（复查修复）：claim 后移到全部前置校验之后 —— 此前 begin 早于
        // payload/handler/body_param 校验，early-return 会把 InFlight claim
        // 泄漏 30s，卡死同 key 的合法重试。
        #[cfg(all(feature = "grpc", feature = "idempotency"))]
        let idem_key = match &self.idempotency {
            Some(guard) => request
                .metadata()
                .get("idempotency-key")
                .and_then(|v| v.to_str().ok())
                .map(|k| (guard, request.get_ref().method.clone(), k.to_string())),
            None => None,
        };

        let req = request.into_inner();

        // vuln-0002 补强：gRPC 路径此前跳过 MCP 的大小校验。
        // 在 handler 调用前对 parameters + data 总大小设上限，防止超大载荷 DoS。
        Self::ensure_payload_size(&req)?;

        // method 名在 parameters/data 消费前克隆（build_handler_args 按值
        // 接收 req，错误消息与状态链仍需 method 名）。
        let method = req.method.clone();

        // lookup handler by method name
        let handler = self.handlers().get(method.as_str()).copied().ok_or_else(|| {
            #[cfg(feature = "streaming")]
            if self.stream_handlers().contains_key(method.as_str()) {
                return Status::failed_precondition(format!(
                    "method '{}' is a streaming method; invoke it via CallStream",
                    method
                ));
            }
            Status::not_found(format!(
                "method '{}' not registered (no matching #[forge(grpc_method = \"...\")] declaration)",
                method
            ))
        })?;

        // parameters → args, data → body_param key
        let body_param = self.body_params().get(method.as_str()).copied().flatten();
        let args = Self::build_handler_args(req, &method, body_param)?;

        // T003: 前置校验全部通过 —— 此刻才 claim（InFlight → already_exists；
        // Replay → 返回缓存；Execute → 继续）。
        #[cfg(all(feature = "grpc", feature = "idempotency"))]
        if let Some((guard, scope, key)) = &idem_key {
            match guard.store.begin(scope, key, 30) {
                crate::cache::IdempotencyOutcome::InFlight => {
                    return Err(Status::already_exists(
                        "request with this idempotency-key is already in flight",
                    ));
                }
                crate::cache::IdempotencyOutcome::Replay { body, .. } => {
                    if let Ok(cached) = serde_json::from_slice::<CachedCallResponse>(&body) {
                        // 生命周期是端点属性而非执行属性：重放响应与首次
                        // 执行同样携带 deprecation 元数据。
                        let mut replay = Response::new(CallResponse {
                            success: cached.success,
                            data: cached.data,
                            error: cached.error,
                            status_code: cached.status_code,
                        });
                        if let Some(&(deprecated, sunset, successor)) =
                            self.lifecycles().get(method.as_str())
                        {
                            attach_lifecycle_metadata(&mut replay, deprecated, sunset, successor);
                        }
                        return Ok(replay);
                    }
                    // 缓存损坏 → abort 让调用方重试
                    guard.store.abort(scope, key);
                }
                crate::cache::IdempotencyOutcome::Execute => {}
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
                    .get(method.as_str())
                    .copied()
                    .flatten();
                let status_code = extract_status_code(&value)
                    .or(default_status.map(|s| s as i32))
                    .unwrap_or(200);
                let response = CallResponse {
                    success: true,
                    data: extract_value(&value),
                    error: String::new(),
                    status_code,
                };
                // T022: 成功响应入缓存供重放。
                #[cfg(all(feature = "grpc", feature = "idempotency"))]
                if let Some((guard, scope, key)) = &idem_key {
                    // T007：缓存体积上限（1 MiB 对齐协议 payload cap）——
                    // 超限 abort 不缓存，避免无界内存增长。
                    let serialized = serde_json::to_vec(&CachedCallResponse {
                        success: response.success,
                        data: response.data.clone(),
                        error: String::new(),
                        status_code: response.status_code,
                    })
                    .unwrap_or_default();
                    if serialized.len() <= MAX_GRPC_ARGUMENTS_SIZE_BYTES {
                        guard.store.complete(
                            scope,
                            key,
                            status_code.clamp(0, u16::MAX as i32) as u16,
                            None,
                            serialized,
                            guard.ttl_secs,
                        );
                    } else {
                        guard.store.abort(scope, key);
                    }
                }
                // 端点生命周期（`#[forge(deprecated, sunset, successor)]`）
                // 镜像为成功响应 metadata（deprecation / sunset /
                // successor-version），未注解端点无注入开销。
                let mut grpc_response = Response::new(response);
                // is_empty() 短路：全仓库未声明生命周期时免去逐请求的
                // 哈希查找（与 HTTP 侧 unannotated-no-layer 契约对齐）。
                let lifecycles = self.lifecycles();
                if !lifecycles.is_empty()
                    && let Some(&(deprecated, sunset, successor)) = lifecycles.get(method.as_str())
                {
                    attach_lifecycle_metadata(&mut grpc_response, deprecated, sunset, successor);
                }
                Ok(grpc_response)
            }
            Ok(Err(e)) => {
                // business error → 真实 gRPC Status（vuln-SIMPL-002 修复）：
                // 此前走 Status::ok + body success:false，标准 gRPC 客户端、
                // 监控错误率与重试/熔断策略对该批错误全部失明。现按统一
                // 映射表落到 tonic::Code，details 携带 UnifiedError JSON
                //（机器可读 code/field/trace_id 不丢失）。
                let unified = crate::error::unified::UnifiedError::from(&e);
                let code = crate::error::unified::grpc_code_for(&e);
                // 业务失败不缓存：清除在途 claim，调用方可立即重试。
                #[cfg(all(feature = "grpc", feature = "idempotency"))]
                if let Some((guard, scope, key)) = &idem_key {
                    guard.store.abort(scope, key);
                }
                Err(Status::with_details(
                    code,
                    unified.message.clone(),
                    tonic::codegen::Bytes::from(unified.to_json().to_string()),
                ))
            }
            Err(_panic) => {
                // handler panicked → generic internal error.
                // Never expose panic payload to the client (security).
                #[cfg(all(feature = "grpc", feature = "idempotency"))]
                if let Some((guard, scope, key)) = &idem_key {
                    guard.store.abort(scope, key);
                }
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

    /// Server-streaming dispatch（feature = `streaming`）：前置守卫链
    /// （限流 → 认证 → RBAC → 载荷上限）与 unary 路径**同集合同序**（两
    /// 条 RPC 均为限流最外层——未认证洪水不进入恒时验证的 OS 线程睡眠，
    /// 对齐 HTTP 栈「限流层在认证外层」的顺序），随后解析流式 handler，
    /// 每项映射为一条 `CallResponse`。流式路径不参与幂等重放（无单点可
    /// 缓存响应体，携带 `idempotency-key` 以 `failed_precondition` 显式
    /// 拒绝）；`streaming` 关闭时 fail-loud 返回 `unimplemented`（而非让
    /// 方法无声消失）。
    ///
    /// context（feature = `context`）：handler 主体（参数装配 → 用户 fn
    /// 执行到返回 `StreamResponse`）在 request_id/trace_id 作用域内执行，
    /// 与 unary 对齐；作用域不跨越后续的流产出阶段（该阶段由 tonic 连接
    /// 任务在 scope 之外 poll，用户自行 spawn 的生产者任务也不继承
    /// task_local）——需要在逐项日志里带关联 id 的调用方应在生产者任务
    /// 中显式携带上下文字段。
    #[cfg(feature = "streaming")]
    type CallStreamStream =
        std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<CallResponse, Status>> + Send>>;

    #[cfg(feature = "streaming")]
    async fn call_stream(
        &self,
        request: Request<CallRequest>,
    ) -> Result<Response<Self::CallStreamStream>, Status> {
        // 限流最外层（与 unary 同序）：remote_addr 不可伪造，缺省
        // "unknown" 兜底。
        #[cfg(feature = "ratelimit")]
        {
            let identifier = request
                .remote_addr()
                .map(|addr| addr.ip().to_string())
                .or_else(|| Some("unknown".to_string()));
            self.enforce_rate_limit(identifier.as_deref()).await?;
        }

        #[cfg(feature = "security")]
        let auth_ctx = self.authenticate(&request).await?;
        #[cfg(not(feature = "security"))]
        let auth_ctx: Option<()> = None;
        let _ = &auth_ctx;

        let roles = self
            .stream_roles()
            .get(request.get_ref().method.as_str())
            .copied()
            .unwrap_or(&[]);
        #[cfg(feature = "security")]
        self.ensure_authorized(roles, &auth_ctx)?;
        #[cfg(not(feature = "security"))]
        self.ensure_authorized(roles)?;

        // 流式路径不参与幂等重放：流式响应无单点可缓存体，重放语义不
        // 成立——显式拒绝而非静默忽略（错误必须可见，禁止静默降级）。
        #[cfg(feature = "idempotency")]
        if self.idempotency.is_some() && request.metadata().contains_key("idempotency-key") {
            return Err(Status::failed_precondition(
                "CallStream does not support idempotency replay; drop the idempotency-key metadata",
            ));
        }

        let req = request.into_inner();
        Self::ensure_payload_size(&req)?;

        let method = req.method.clone();

        // handler 缺失时区分方向指引：unary 表里有 → 指回 Call；
        // 两表皆无 → not_found（与 unary 缺失消息同文案口径）。
        let stream_handler = self
            .stream_handlers()
            .get(method.as_str())
            .copied()
            .ok_or_else(|| {
                if self.handlers().contains_key(method.as_str()) {
                    Status::failed_precondition(format!(
                        "method '{}' is not a streaming method; invoke it via Call",
                        method
                    ))
                } else {
                    Status::not_found(format!(
                        "method '{}' not registered (no matching #[forge(grpc_method = \"...\")] declaration)",
                        method
                    ))
                }
            })?;

        let body_param = self
            .stream_body_params()
            .get(method.as_str())
            .copied()
            .flatten();
        let args = Self::build_handler_args(req, &method, body_param)?;

        // catch_unwind：panicking handler 不得经 gRPC 泄漏内部信息（与
        // unary 同一安全规则）。context 开启时 handler 主体在
        // request_id/trace_id 作用域内执行（作用域不跨越流产出阶段——
        // 见本函数文档的边界说明）。handler 的调用本身包在 async 块内：
        // 其同步构造段（参数 move、闭包体开头）必须已在 scope 内执行。
        use futures_util::FutureExt;
        use std::panic::AssertUnwindSafe;
        let dispatch = AssertUnwindSafe(async move { stream_handler(args, self.state.clone()) })
            .catch_unwind();
        #[cfg(feature = "context")]
        let outcome = {
            let ctx = crate::context::current_or_new();
            crate::context::scope(ctx, dispatch).await
        };
        #[cfg(not(feature = "context"))]
        let outcome = dispatch.await;

        match outcome {
            Ok(handler_future) => match handler_future.await {
                Ok(output) => {
                    let default_status = self
                        .stream_default_statuses()
                        .get(method.as_str())
                        .copied()
                        .flatten();
                    // 逐项映射：Ok(value) → success:true 的 CallResponse（状态
                    // 优先级链与 unary 同源）；Err(msg) → success:false 的项级
                    // 错误（流继续，与 SSE 错误事件语义对齐）。
                    let stream = output.stream.map(move |item| match item {
                        Ok(value) => {
                            let status_code = extract_status_code(&value)
                                .or(default_status.map(|s| s as i32))
                                .unwrap_or(200);
                            Ok(CallResponse {
                                success: true,
                                data: extract_value(&value),
                                error: String::new(),
                                status_code,
                            })
                        }
                        Err(message) => Ok(CallResponse {
                            success: false,
                            data: String::new(),
                            error: message,
                            status_code: 500,
                        }),
                    });
                    // 端点生命周期镜像为流式响应的（外层）metadata，
                    // 客户端在首个流项前即可读取——与 unary 同契约。
                    let mut response = Response::new(Box::pin(stream) as Self::CallStreamStream);
                    // is_empty() 短路：与 unary 路径同理由。
                    let lifecycles = self.stream_lifecycles();
                    if !lifecycles.is_empty()
                        && let Some(&(deprecated, sunset, successor)) =
                            lifecycles.get(method.as_str())
                    {
                        attach_lifecycle_metadata(&mut response, deprecated, sunset, successor);
                    }
                    Ok(response)
                }
                Err(e) => {
                    // handler 启动失败（参数/校验等业务错误）→ 真实 gRPC
                    // Status，details 携带 UnifiedError JSON（与 unary 同映射）。
                    let unified = crate::error::unified::UnifiedError::from(&e);
                    let code = crate::error::unified::grpc_code_for(&e);
                    Err(Status::with_details(
                        code,
                        unified.message.clone(),
                        tonic::codegen::Bytes::from(unified.to_json().to_string()),
                    ))
                }
            },
            Err(_panic) => Err(Status::internal("handler panicked")),
        }
    }

    /// `streaming` 关闭时的占位实现：编译必须过（trait 有 CallStream
    /// 成员），但调用 fail-loud——`unimplemented` 指明需启用 `streaming`
    /// feature，而非让方法无声 404。
    #[cfg(not(feature = "streaming"))]
    type CallStreamStream =
        std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<CallResponse, Status>> + Send>>;

    #[cfg(not(feature = "streaming"))]
    async fn call_stream(
        &self,
        _request: Request<CallRequest>,
    ) -> Result<Response<Self::CallStreamStream>, Status> {
        Err(Status::unimplemented(
            "server-streaming dispatch requires the `streaming` feature on the server",
        ))
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
        #[cfg(not(feature = "ratelimit"))]
        let _ = &request;
        let response = InfoResponse {
            name: "SdForge Service".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            methods: {
                // unary + streaming 两个注册表的方法并集（streaming 关闭时
                // 与既有行为逐字一致）。
                #[allow(unused_mut)]
                let mut methods = self
                    .handlers()
                    .keys()
                    .map(|k| (*k).to_string())
                    .collect::<Vec<_>>();
                #[cfg(feature = "streaming")]
                methods.extend(self.stream_handlers().keys().map(|k| (*k).to_string()));
                methods
            },
            // 服务级描述经 i18n 注册表按当前 locale 翻译（宿主可经
            // register_translation 注册 "sdforge.service.description" 键）；
            // 无翻译时回退英文原文——gRPC wire 唯一的描述输出点。
            description: crate::i18n::translate_or_fallback(
                "SdForge Multi-Protocol SDK Framework",
                Some("sdforge.service.description"),
            ),
        };

        Ok(Response::new(response))
    }
}

/// 幂等缓存的可序列化响应载荷（prost 消息不带 serde，自行镜像字段）。
#[cfg(all(feature = "grpc", feature = "idempotency"))]
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedCallResponse {
    success: bool,
    data: String,
    error: String,
    status_code: i32,
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
    // T001: RBAC 生产接线 —— per-call verifier 装配，auth(role) 声明生效。
    #[cfg(feature = "security")]
    let service = match config.auth_verifier {
        Some(ref verifier) => service.with_auth_interceptor(std::sync::Arc::clone(verifier)),
        None => service,
    };
    // T022: 幂等 store 配置传递（None = 关闭，默认）。
    #[cfg(feature = "idempotency")]
    let service = match config.idempotency_store {
        Some(ref store) => service
            .with_idempotency_store(std::sync::Arc::clone(store), config.idempotency_ttl_secs),
        None => service,
    };

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
    // T024: HTTP/2 keepalive 配置暴露（None = tonic 默认）。
    builder = builder
        .http2_keepalive_interval(config.http2_keepalive_interval)
        .http2_keepalive_timeout(config.http2_keepalive_timeout);
    // T025: 可选 TLS 接线（feature = grpc-tls；证书加载由调用方负责）。
    #[cfg(feature = "grpc-tls")]
    if let Some(tls) = config.tls.clone() {
        builder = builder
            .tls_config(tls)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;
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
            #[cfg(feature = "security")]
            auth_verifier: None,
            state: None,
            #[cfg(feature = "ratelimit")]
            rate_limiter: None,
            #[cfg(feature = "idempotency")]
            idempotency_store: None,
            #[cfg(feature = "idempotency")]
            idempotency_ttl_secs: 86_400,
            #[cfg(feature = "idempotency")]
            idempotency_inflight_ttl_secs: 30,
            http2_keepalive_interval: None,
            http2_keepalive_timeout: None,
            #[cfg(feature = "grpc-tls")]
            tls: None,
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
        i18n_key: None,
        deprecated: false,
        sunset: None,
        successor: None,
    }
        }

    /// Handler that always returns `Err(NotFound)` — verifies the business
    /// error → `Status::not_found` + details `UnifiedError` mapping.
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
        i18n_key: None,
        deprecated: false,
        sunset: None,
        successor: None,
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
        i18n_key: None,
        deprecated: false,
        sunset: None,
        successor: None,
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
        i18n_key: None,
        deprecated: false,
        sunset: None,
        successor: None,
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
            i18n_key: None,
            deprecated: false,
            sunset: None,
            successor: None,
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
        i18n_key: None,
        deprecated: false,
        sunset: None,
        successor: None,
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
        i18n_key: None,
        deprecated: false,
        sunset: None,
        successor: None,
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
        i18n_key: None,
        deprecated: false,
        sunset: None,
        successor: None,
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
        i18n_key: None,
        deprecated: false,
        sunset: None,
        successor: None,
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
        i18n_key: None,
        deprecated: false,
        sunset: None,
        successor: None,
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

    /// GetInfo 服务级描述经 i18n 注册表按 locale 翻译（gRPC wire 唯一
    /// 的描述输出点）；未注册翻译回退英文原文。
    #[tokio::test]
    #[serial_test::serial]
    async fn get_info_description_translates_by_locale() {
        sdforge::i18n::clear_translations();
        let service = SdForgeGrpcService::default();

        // 未注册翻译 → 英文回退。
        sdforge::i18n::set_locale("zh-CN");
        let fallback = service
            .get_info(Request::new(InfoRequest::default()))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(
            fallback.description, "SdForge Multi-Protocol SDK Framework",
            "unregistered key must fall back to English"
        );

        // 宿主注册 zh 翻译（键为服务级 "sdforge.service.description"）
        // → GetInfo.description 随 locale 变化。
        sdforge::i18n::register_translation(
            "zh-CN",
            "sdforge.service.description",
            "SDForge 多协议 SDK 框架",
        );
        let translated = service
            .get_info(Request::new(InfoRequest::default()))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(
            translated.description, "SDForge 多协议 SDK 框架",
            "registered zh translation must appear in GetInfo.description"
        );

        sdforge::i18n::clear_translations();
        sdforge::i18n::set_locale("en");
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
            i18n_key: None,
            deprecated: false,
            sunset: None,
            successor: None,
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
            i18n_key: None,
            deprecated: false,
            sunset: None,
            successor: None,
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
    // 错误路径不回归（已由 call_business_error_returns_real_grpc_status 覆盖）
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

    /// T001: auth_verifier 默认 None。
    #[cfg(feature = "security")]
    #[test]
    fn grpc_server_config_auth_verifier_defaults_none() {
        assert!(GrpcServerConfig::default().auth_verifier.is_none());
    }

    /// T024: keepalive 配置默认 None（tonic 默认行为）。
    #[test]
    fn grpc_server_config_keepalive_defaults_none() {
        let config = GrpcServerConfig::default();
        assert!(config.http2_keepalive_interval.is_none());
        assert!(config.http2_keepalive_timeout.is_none());
    }

    /// T025: TLS 默认 None（grpc-tls feature 下也保持关闭默认）。
    #[cfg(feature = "grpc-tls")]
    #[test]
    fn grpc_server_config_tls_defaults_none() {
        assert!(GrpcServerConfig::default().tls.is_none());
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
    // ========================================================================
    // T022: gRPC idempotency-key metadata 幂等防护
    // ========================================================================
    #[cfg(all(feature = "grpc", feature = "idempotency"))]
    mod idempotency_tests {
        use super::*;
        use crate::cache::{IdempotencyOutcome, IdempotencyStore};

        static EXEC_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

        fn counting_handler(args: HandlerArgs, _state: HandlerState) -> crate::core::HandlerFuture {
            EXEC_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let msg = args.get("msg").cloned().unwrap_or_default();
            Box::pin(async move { Ok(Value::String(msg)) })
        }

        inventory::submit! {
            GrpcHandlerRegistration {
                method: "test_idem_echo",
                handler: counting_handler,
                body_param: None,
                default_status: None,
                roles: &[],
                i18n_key: None,
                deprecated: false,
                sunset: None,
                successor: None,
            }
        }

        fn request_with_key(key: Option<&str>) -> Request<CallRequest> {
            let mut req = Request::new(CallRequest {
                method: "test_idem_echo".to_string(),
                parameters: HashMap::new(),
                data: String::new(),
            });
            if let Some(k) = key {
                req.metadata_mut()
                    .insert("idempotency-key", k.parse().unwrap());
            }
            req
        }

        fn service_with_store() -> (SdForgeGrpcService, Arc<IdempotencyStore>) {
            let store = Arc::new(IdempotencyStore::new());
            (
                SdForgeGrpcService::default().with_idempotency_store(store.clone(), 3600),
                store,
            )
        }

        #[tokio::test]
        #[serial_test::serial]
        async fn duplicate_key_replays_without_reexecution() {
            EXEC_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
            let (service, _store) = service_with_store();

            let first = service
                .call(request_with_key(Some("g-dup")))
                .await
                .unwrap()
                .into_inner();
            assert!(first.success);

            let second = service
                .call(request_with_key(Some("g-dup")))
                .await
                .unwrap()
                .into_inner();
            assert_eq!(second.data, first.data, "重放数据一致");
            assert_eq!(
                EXEC_COUNT.load(std::sync::atomic::Ordering::SeqCst),
                1,
                "handler 只执行一次"
            );
        }

        #[tokio::test]
        async fn in_flight_key_returns_already_exists() {
            let (service, store) = service_with_store();
            // 预占在途 claim
            assert_eq!(
                store.begin("test_idem_echo", "g-inflight", 30),
                IdempotencyOutcome::Execute
            );
            let err = service
                .call(request_with_key(Some("g-inflight")))
                .await
                .unwrap_err();
            assert_eq!(err.code(), tonic::Code::AlreadyExists);
        }

        /// T003（复查修复）：early-return（未知 method）不泄漏 InFlight claim
        /// —— 立即同 key 重试不受 already_exists 卡 30s。
        #[tokio::test]
        async fn early_return_does_not_leak_inflight_claim() {
            let (service, store) = service_with_store();

            // 未知 method：claim 发生在前置校验之后 → 此路径根本不 claim，
            // 直接 not_found。
            let mut req = Request::new(CallRequest {
                method: "no_such_method_t003".to_string(),
                parameters: HashMap::new(),
                data: String::new(),
            });
            req.metadata_mut()
                .insert("idempotency-key", "leak-1".parse().unwrap());
            let err = service.call(req).await.unwrap_err();
            assert_eq!(err.code(), tonic::Code::NotFound);

            // store 无残留 claim：同 key 打合法方法立即 Execute。
            let outcome = store.begin("test_idem_echo", "leak-1", 30);
            assert_eq!(
                outcome,
                crate::cache::IdempotencyOutcome::Execute,
                "early-return 不得泄漏 InFlight claim"
            );
        }

        /// T003（复查修复）：data 无 body_param 的 early-return 后同 key 立即可重试。
        #[tokio::test]
        async fn body_param_rejection_does_not_leak_claim() {
            let (service, store) = service_with_store();

            // test_echo 无 body_param，data 非空 → invalid_argument。
            let mut req = Request::new(CallRequest {
                method: "test_idem_echo".to_string(),
                parameters: HashMap::new(),
                data: "unexpected".to_string(),
            });
            req.metadata_mut()
                .insert("idempotency-key", "leak-2".parse().unwrap());
            let err = service.call(req).await.unwrap_err();
            assert_eq!(err.code(), tonic::Code::InvalidArgument);

            let outcome = store.begin("test_idem_echo", "leak-2", 30);
            assert_eq!(outcome, crate::cache::IdempotencyOutcome::Execute);
        }

        #[tokio::test]
        #[serial_test::serial]
        async fn without_key_proceeds_every_time() {
            EXEC_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
            let (service, _store) = service_with_store();
            for _ in 0..2 {
                let resp = service
                    .call(request_with_key(None))
                    .await
                    .unwrap()
                    .into_inner();
                assert!(resp.success);
            }
            assert_eq!(
                EXEC_COUNT.load(std::sync::atomic::Ordering::SeqCst),
                2,
                "无 key 不参与幂等"
            );
        }
    }

    // ========================================================================
    // server-streaming dispatch（feature = streaming）
    // ========================================================================

    #[cfg(feature = "streaming")]
    mod grpc_streaming {
        use super::*;
        use futures_util::StreamExt;
        // 守卫顺序契约测试的 RateLimiter impl 以 Pin<Box<dyn Future>> 声明
        // check（对齐 trait 签名惯例）；非全特性组合下该测试不编译。
        #[cfg(all(feature = "security", feature = "ratelimit"))]
        use std::future::Future;
        #[cfg(all(feature = "security", feature = "ratelimit"))]
        use std::pin::Pin;

        /// 三条目流式 handler：逐项产出 JSON 值（`inventory::submit!`
        /// 注册，与 unary 探针同模式）。
        fn range_stream_handler(
            args: HandlerArgs,
            _state: HandlerState,
        ) -> crate::grpc::GrpcStreamHandlerFuture {
            let count: u64 = args.get("count").and_then(|s| s.parse().ok()).unwrap_or(3);
            let (tx, rx) = tokio::sync::mpsc::channel::<crate::grpc::GrpcStreamItem>(8);
            tokio::spawn(async move {
                for i in 0..count {
                    let _ = tx.send(Ok(Value::String(format!("item-{i}")))).await;
                }
            });
            Box::pin(async move {
                Ok(crate::grpc::GrpcStreamOutput {
                    stream: Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)),
                })
            })
        }

        inventory::submit! {
            crate::grpc::GrpcStreamHandlerRegistration {
                method: "test_stream_range",
                handler: range_stream_handler,
                body_param: None,
                default_status: None,
                roles: &[],
                i18n_key: None,
                deprecated: false,
                sunset: None,
                successor: None,
            }
        }

        /// 每项带 `ServiceResponse.status_code` 语义的流式 handler：奇数项
        /// 返回 201（验证 per-item 状态优先级链的字段入口）。
        fn status_stream_handler(
            _args: HandlerArgs,
            _state: HandlerState,
        ) -> crate::grpc::GrpcStreamHandlerFuture {
            let (tx, rx) = tokio::sync::mpsc::channel::<crate::grpc::GrpcStreamItem>(4);
            tokio::spawn(async move {
                let _ = tx
                    .send(Ok(serde_json::json!({
                        "success": true,
                        "status_code": 201,
                        "payload": "created"
                    })))
                    .await;
                let _ = tx.send(Ok(Value::String("plain-item".to_string()))).await;
            });
            Box::pin(async move {
                Ok(crate::grpc::GrpcStreamOutput {
                    stream: Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)),
                })
            })
        }

        inventory::submit! {
            crate::grpc::GrpcStreamHandlerRegistration {
                method: "test_stream_status",
                handler: status_stream_handler,
                body_param: None,
                default_status: None,
                roles: &[],
                i18n_key: None,
                deprecated: false,
                sunset: None,
                successor: None,
            }
        }

        /// 项级错误流：第二项返回 Err —— 流必须继续到第三项（与 SSE 错误
        /// 事件语义对齐，不终止整个流）。
        fn item_error_stream_handler(
            _args: HandlerArgs,
            _state: HandlerState,
        ) -> crate::grpc::GrpcStreamHandlerFuture {
            let (tx, rx) = tokio::sync::mpsc::channel::<crate::grpc::GrpcStreamItem>(4);
            tokio::spawn(async move {
                let _ = tx.send(Ok(Value::String("first".to_string()))).await;
                let _ = tx.send(Err("item-level failure".to_string())).await;
                let _ = tx.send(Ok(Value::String("third".to_string()))).await;
            });
            Box::pin(async move {
                Ok(crate::grpc::GrpcStreamOutput {
                    stream: Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)),
                })
            })
        }

        inventory::submit! {
            crate::grpc::GrpcStreamHandlerRegistration {
                method: "test_stream_item_error",
                handler: item_error_stream_handler,
                body_param: None,
                default_status: None,
                roles: &[],
                i18n_key: None,
                deprecated: false,
                sunset: None,
                successor: None,
            }
        }

        /// body_param 注入：`CallRequest.data` → `payload` 键。
        fn body_stream_handler(
            args: HandlerArgs,
            _state: HandlerState,
        ) -> crate::grpc::GrpcStreamHandlerFuture {
            let payload = args.get("payload").cloned().unwrap_or_default();
            let (tx, rx) = tokio::sync::mpsc::channel::<crate::grpc::GrpcStreamItem>(2);
            tokio::spawn(async move {
                let _ = tx.send(Ok(Value::String(payload))).await;
            });
            Box::pin(async move {
                Ok(crate::grpc::GrpcStreamOutput {
                    stream: Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)),
                })
            })
        }

        inventory::submit! {
            crate::grpc::GrpcStreamHandlerRegistration {
                method: "test_stream_body",
                handler: body_stream_handler,
                body_param: Some("payload"),
                default_status: None,
                roles: &[],
                i18n_key: None,
                deprecated: false,
                sunset: None,
                successor: None,
            }
        }

        fn stream_call_request(method: &str, data: &str) -> Request<CallRequest> {
            Request::new(CallRequest {
                method: method.to_string(),
                parameters: HashMap::new(),
                data: data.to_string(),
            })
        }

        async fn collect_stream(
            stream: <SdForgeGrpcService as SdForgeService>::CallStreamStream,
        ) -> Vec<Result<CallResponse, Status>> {
            stream.collect::<Vec<_>>().await
        }

        #[test]
        fn stream_registries_built_from_inventory() {
            let service = SdForgeGrpcService::default();
            assert!(service.stream_handlers().contains_key("test_stream_range"));
            // 双表互斥：流式方法不在 unary 表，unary 方法不在流式表。
            assert!(!service.handlers().contains_key("test_stream_range"));
            assert!(!service.stream_handlers().contains_key("test_echo"));
        }

        #[tokio::test]
        async fn call_stream_dispatches_items_in_order() {
            let service = SdForgeGrpcService::default();
            let response = service
                .call_stream(stream_call_request("test_stream_range", ""))
                .await
                .expect("stream dispatch succeeds");
            let items = collect_stream(response.into_inner()).await;
            assert_eq!(items.len(), 3, "three items must be streamed: {items:?}");
            for (i, item) in items.iter().enumerate() {
                let resp = item.as_ref().expect("each item is Ok(CallResponse)");
                assert!(resp.success, "item {i} must be success: {resp:?}");
                assert_eq!(resp.data, format!("item-{i}"));
                assert_eq!(resp.status_code, 200);
            }
        }

        #[tokio::test]
        async fn call_stream_empty_stream_yields_no_items() {
            // count=0 → 空流：合法产出，零条消息，不报错。
            let service = SdForgeGrpcService::default();
            let mut req = stream_call_request("test_stream_range", "");
            req.get_mut()
                .parameters
                .insert("count".to_string(), "0".to_string());
            let response = service.call_stream(req).await.expect("empty stream ok");
            let items = collect_stream(response.into_inner()).await;
            assert!(
                items.is_empty(),
                "empty stream must carry no items: {items:?}"
            );
        }

        #[tokio::test]
        async fn call_stream_per_item_status_chain() {
            let service = SdForgeGrpcService::default();
            let response = service
                .call_stream(stream_call_request("test_stream_status", ""))
                .await
                .expect("dispatch");
            let items = collect_stream(response.into_inner()).await;
            assert_eq!(items.len(), 2);
            let first = items[0].as_ref().expect("first item");
            assert_eq!(first.status_code, 201, "field status_code must win");
            let second = items[1].as_ref().expect("second item");
            assert_eq!(second.status_code, 200, "bare value falls back to 200");
        }

        #[tokio::test]
        async fn call_stream_item_error_does_not_abort_stream() {
            let service = SdForgeGrpcService::default();
            let response = service
                .call_stream(stream_call_request("test_stream_item_error", ""))
                .await
                .expect("dispatch");
            let items = collect_stream(response.into_inner()).await;
            assert_eq!(items.len(), 3, "error item must not abort the stream");
            let failed = items[1].as_ref().expect("error arrives as CallResponse");
            assert!(!failed.success, "item-level error maps to success:false");
            assert_eq!(failed.error, "item-level failure");
            let third = items[2].as_ref().expect("stream continues after error");
            assert_eq!(third.data, "third");
        }

        #[tokio::test]
        async fn call_stream_injects_data_into_body_param() {
            let service = SdForgeGrpcService::default();
            let response = service
                .call_stream(stream_call_request("test_stream_body", "the-payload"))
                .await
                .expect("dispatch");
            let items = collect_stream(response.into_inner()).await;
            assert_eq!(items.len(), 1);
            assert_eq!(
                items[0].as_ref().expect("item").data,
                "the-payload",
                "CallRequest.data must reach the declared body_param"
            );
        }

        #[tokio::test]
        async fn call_stream_data_without_body_param_is_invalid_argument() {
            let service = SdForgeGrpcService::default();
            let err = service
                .call_stream(stream_call_request("test_stream_range", "unexpected"))
                .await;
            let err = match err {
                Err(status) => status,
                Ok(_) => panic!("data with no body_param must fail"),
            };
            assert_eq!(err.code(), tonic::Code::InvalidArgument);
        }

        #[tokio::test]
        async fn call_stream_unknown_method_is_not_found() {
            let service = SdForgeGrpcService::default();
            let err = service
                .call_stream(stream_call_request("totally_absent", ""))
                .await;
            let err = match err {
                Err(status) => status,
                Ok(_) => panic!("unknown method must fail"),
            };
            assert_eq!(err.code(), tonic::Code::NotFound);
        }

        #[tokio::test]
        async fn call_stream_to_unary_method_is_failed_precondition() {
            // test_echo 在 unary 表：CallStream 必须给出方向指引而非 not_found。
            let service = SdForgeGrpcService::default();
            let err = service
                .call_stream(stream_call_request("test_echo", ""))
                .await;
            let err = match err {
                Err(status) => status,
                Ok(_) => panic!("unary method must be rejected on CallStream"),
            };
            assert_eq!(err.code(), tonic::Code::FailedPrecondition);
            assert!(
                err.message().contains("Call"),
                "error must point the caller at the unary RPC: {err}"
            );
        }

        #[tokio::test]
        async fn unary_call_to_streaming_method_is_failed_precondition() {
            // test_stream_range 在流式表：Call 必须给出方向指引而非 not_found。
            let service = SdForgeGrpcService::default();
            let err = service
                .call(stream_call_request("test_stream_range", ""))
                .await;
            let err = match err {
                Err(status) => status,
                Ok(_) => panic!("streaming method must be rejected on unary Call"),
            };
            assert_eq!(err.code(), tonic::Code::FailedPrecondition);
            assert!(
                err.message().contains("CallStream"),
                "error must point the caller at CallStream: {err}"
            );
        }

        #[cfg(feature = "idempotency")]
        #[tokio::test]
        async fn call_stream_rejects_idempotency_key() {
            use crate::cache::IdempotencyStore;
            let service = SdForgeGrpcService::default()
                .with_idempotency_store(Arc::new(IdempotencyStore::new()), 86_400);
            let mut req = stream_call_request("test_stream_range", "");
            req.metadata_mut()
                .insert("idempotency-key", "stream-key".parse().unwrap());
            let err = match service.call_stream(req).await {
                Err(status) => status,
                Ok(_) => panic!("streaming + idempotency-key must be rejected"),
            };
            assert_eq!(err.code(), tonic::Code::FailedPrecondition);
        }

        #[tokio::test]
        async fn call_stream_oversized_payload_is_invalid_argument() {
            let service = SdForgeGrpcService::default();
            let err = service
                .call_stream(stream_call_request(
                    "test_stream_range",
                    &"x".repeat(MAX_GRPC_ARGUMENTS_SIZE_BYTES + 1),
                ))
                .await;
            let err = match err {
                Err(status) => status,
                Ok(_) => panic!("oversized payload must fail"),
            };
            assert_eq!(err.code(), tonic::Code::InvalidArgument);
        }

        #[tokio::test]
        async fn get_info_lists_streaming_methods() {
            let service = SdForgeGrpcService::default();
            let info = service
                .get_info(Request::new(InfoRequest::default()))
                .await
                .unwrap()
                .into_inner();
            assert!(
                info.methods.iter().any(|m| m == "test_stream_range"),
                "get_info must list streaming methods: {:?}",
                info.methods
            );
        }

        /// context 注入（feature = `context`）：handler 主体在
        /// request_id/trace_id 作用域内执行，与 unary 对齐。
        #[cfg(feature = "context")]
        #[tokio::test]
        async fn call_stream_installs_request_context_in_handler() {
            fn context_probe_handler(
                _args: HandlerArgs,
                _state: HandlerState,
            ) -> crate::grpc::GrpcStreamHandlerFuture {
                let observed = crate::context::current();
                let (tx, rx) = tokio::sync::mpsc::channel::<crate::grpc::GrpcStreamItem>(2);
                tokio::spawn(async move {
                    match observed {
                        Some(ctx) => {
                            let _ = tx
                                .send(Ok(Value::String(format!(
                                    "{}|{}",
                                    ctx.request_id(),
                                    ctx.trace_id()
                                ))))
                                .await;
                        }
                        None => {
                            let _ = tx.send(Err("no ambient context".to_string())).await;
                        }
                    }
                });
                Box::pin(async move {
                    Ok(crate::grpc::GrpcStreamOutput {
                        stream: Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)),
                    })
                })
            }

            inventory::submit! {
                crate::grpc::GrpcStreamHandlerRegistration {
                    method: "test_stream_context",
                    handler: context_probe_handler,
                    body_param: None,
                    default_status: None,
                    roles: &[],
                    i18n_key: None,
                    deprecated: false,
                    sunset: None,
                    successor: None,
                }
            }

            let service = SdForgeGrpcService::default();
            let response = service
                .call_stream(stream_call_request("test_stream_context", ""))
                .await
                .expect("dispatch");
            let items = collect_stream(response.into_inner()).await;
            assert_eq!(items.len(), 1);
            let resp = items[0].as_ref().expect("item");
            assert!(
                resp.success,
                "ambient context must be installed: {}",
                resp.error
            );
            assert!(
                resp.data.starts_with("req-") && resp.data.contains("|trace-"),
                "handler must observe request_id/trace_id inside the scope: {}",
                resp.data
            );
        }

        /// 契约测试（显性化既有行为）：生产者任务 panic → tx drop →
        /// ReceiverStream 耗尽 → 流以**正常收尾**结束，已产出项照常送达，
        /// 但客户端无法区分完整流与截断流。该边界是当前流式语义的一部分
        /// （handler.rs / USER_GUIDE 均有记载）——生产者需要 fail-visible
        /// 的调用方应自行 JoinHandle 监测并注入项级错误。此测试锁定该
        /// 契约：若未来实现改为 panic 传播，此处会失败并提示更新文档。
        #[tokio::test]
        async fn producer_panic_truncates_stream_silently_by_contract() {
            fn panicking_producer_handler(
                _args: HandlerArgs,
                _state: HandlerState,
            ) -> crate::grpc::GrpcStreamHandlerFuture {
                let (tx, rx) = tokio::sync::mpsc::channel::<crate::grpc::GrpcStreamItem>(4);
                tokio::spawn(async move {
                    let _ = tx.send(Ok(Value::String("before".to_string()))).await;
                    panic!("producer exploded — must not crash the dispatcher");
                });
                Box::pin(async move {
                    Ok(crate::grpc::GrpcStreamOutput {
                        stream: Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)),
                    })
                })
            }

            inventory::submit! {
                crate::grpc::GrpcStreamHandlerRegistration {
                    method: "test_stream_producer_panic",
                    handler: panicking_producer_handler,
                    body_param: None,
                    default_status: None,
                    roles: &[],
                    i18n_key: None,
                    deprecated: false,
                    sunset: None,
                    successor: None,
                }
            }

            let service = SdForgeGrpcService::default();
            let response = service
                .call_stream(stream_call_request("test_stream_producer_panic", ""))
                .await
                .expect("dispatch survives producer panic");
            let items = collect_stream(response.into_inner()).await;
            assert_eq!(
                items.len(),
                1,
                "items produced before the panic must arrive: {items:?}"
            );
            assert!(items[0].as_ref().expect("item").success);
            // 流以 None 耗尽收尾（不是 Status 错误）——collect 已证明这一点。
        }

        /// 守卫顺序契约（限流 → 认证，两条 RPC 同序）：拒绝性限流器 +
        /// 无凭据请求必须得到 resource_exhausted——若认证先于限流，同一
        /// 请求会返回 unauthenticated。
        #[cfg(all(feature = "security", feature = "ratelimit"))]
        #[tokio::test]
        async fn guard_order_rate_limit_precedes_auth() {
            use crate::security::grpc_auth::ApiKeyVerifier;

            struct RejectLimiter;
            impl crate::security::ratelimit::RateLimiter for RejectLimiter {
                fn check<'a>(
                    &'a self,
                    _identifier: &'a str,
                ) -> Pin<
                    Box<
                        dyn Future<Output = Result<(), crate::security::ratelimit::RateLimitError>>
                            + Send
                            + 'a,
                    >,
                > {
                    Box::pin(async {
                        Err(crate::security::ratelimit::RateLimitError::Exceeded {
                            limit: 1,
                            window_seconds: 60,
                        })
                    })
                }
            }

            let store = std::sync::Arc::new(crate::security::SdForgeApiKeyAuth::new());
            let verifier: std::sync::Arc<dyn crate::security::grpc_auth::GrpcAuthVerifier> =
                std::sync::Arc::new(ApiKeyVerifier::new(store, ""));
            let limiter: std::sync::Arc<dyn crate::security::ratelimit::RateLimiter> =
                std::sync::Arc::new(RejectLimiter);

            // unary Call：限流先行 → resource_exhausted（而非 unauthenticated）。
            let unary = SdForgeGrpcService::with_state_and_rate_limiter(
                None,
                Some(std::sync::Arc::clone(&limiter)),
            )
            .with_auth_interceptor(std::sync::Arc::clone(&verifier));
            let err = match unary.call(stream_call_request("test_echo", "")).await {
                Err(status) => status,
                Ok(_) => panic!("rejected by limiter"),
            };
            assert_eq!(
                err.code(),
                tonic::Code::ResourceExhausted,
                "rate limit must fire before auth on the unary path: {err}"
            );

            // CallStream：同序。
            let streaming = SdForgeGrpcService::with_state_and_rate_limiter(None, Some(limiter))
                .with_auth_interceptor(verifier);
            let err = match streaming
                .call_stream(stream_call_request("test_stream_range", ""))
                .await
            {
                Err(status) => status,
                Ok(_) => panic!("rejected by limiter"),
            };
            assert_eq!(
                err.code(),
                tonic::Code::ResourceExhausted,
                "rate limit must fire before auth on the streaming path: {err}"
            );
        }

        // ====================================================================
        // 端点生命周期（`#[forge(deprecated, sunset, successor)]`）→ 响应
        // metadata 注入（unary `Call` 与 streaming `CallStream` 同契约）
        // ====================================================================

        fn lifecycle_echo_handler(
            args: HandlerArgs,
            _state: HandlerState,
        ) -> crate::core::HandlerFuture {
            let msg = args.get("msg").cloned().unwrap_or_default();
            Box::pin(async move { Ok(Value::String(msg)) })
        }

        inventory::submit! {
            GrpcHandlerRegistration {
                method: "test_lifecycle_deprecated",
                handler: lifecycle_echo_handler,
                body_param: None,
                default_status: None,
                roles: &[],
                i18n_key: None,
                deprecated: true,
                sunset: Some("2026-12-31"),
                successor: Some("/api/v2/thing"),
            }
        }

        /// unary 成功响应必须携带端点生命周期 metadata（与 HTTP 的
        /// `Deprecation` / `Sunset` / `Link` 头镜像同名键）。
        #[tokio::test]
        async fn call_attaches_lifecycle_metadata() {
            let service = SdForgeGrpcService::default();
            let resp = service
                .call(stream_call_request("test_lifecycle_deprecated", ""))
                .await
                .expect("lifecycle-annotated method dispatches");
            let metadata = resp.metadata();
            assert_eq!(
                metadata.get("deprecation").and_then(|v| v.to_str().ok()),
                Some("true"),
                "deprecation key must mirror the HTTP header: {metadata:?}"
            );
            assert_eq!(
                metadata.get("sunset").and_then(|v| v.to_str().ok()),
                Some("2026-12-31")
            );
            assert_eq!(
                metadata
                    .get("successor-version")
                    .and_then(|v| v.to_str().ok()),
                Some("/api/v2/thing")
            );
        }

        /// 未注解端点不得携带生命周期 metadata 键（无注入开销契约）。
        #[tokio::test]
        async fn call_without_lifecycle_has_no_lifecycle_metadata() {
            let service = SdForgeGrpcService::default();
            let resp = service
                .call(stream_call_request("test_echo", ""))
                .await
                .expect("plain method dispatches");
            let metadata = resp.metadata();
            assert!(metadata.get("deprecation").is_none());
            assert!(metadata.get("sunset").is_none());
            assert!(metadata.get("successor-version").is_none());
        }

        /// streaming 成功响应在外层 Response metadata 携带生命周期键
        /// （客户端在首个流项之前即可读取）。
        #[cfg(feature = "streaming")]
        fn lifecycle_stream_handler(
            _args: HandlerArgs,
            _state: HandlerState,
        ) -> crate::grpc::GrpcStreamHandlerFuture {
            let (tx, rx) = tokio::sync::mpsc::channel::<crate::grpc::GrpcStreamItem>(2);
            tokio::spawn(async move {
                let _ = tx.send(Ok(Value::String("legacy-item".to_string()))).await;
            });
            Box::pin(async move {
                Ok(crate::grpc::GrpcStreamOutput {
                    stream: Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)),
                })
            })
        }

        #[cfg(feature = "streaming")]
        inventory::submit! {
            GrpcStreamHandlerRegistration {
                method: "test_stream_lifecycle",
                handler: lifecycle_stream_handler,
                body_param: None,
                default_status: None,
                roles: &[],
                i18n_key: None,
                deprecated: true,
                sunset: None,
                successor: Some("/api/v2/thing"),
            }
        }

        #[cfg(feature = "streaming")]
        #[tokio::test]
        async fn call_stream_attaches_lifecycle_metadata() {
            let service = SdForgeGrpcService::default();
            let response = service
                .call_stream(stream_call_request("test_stream_lifecycle", ""))
                .await
                .expect("streaming lifecycle method dispatches");
            let metadata = response.metadata();
            assert_eq!(
                metadata.get("deprecation").and_then(|v| v.to_str().ok()),
                Some("true")
            );
            assert!(
                metadata.get("sunset").is_none(),
                "undeclared sunset must stay absent"
            );
            assert_eq!(
                metadata
                    .get("successor-version")
                    .and_then(|v| v.to_str().ok()),
                Some("/api/v2/thing")
            );
            // 流项照常送达：metadata 注入不影响项产出。
            let items = collect_stream(response.into_inner()).await;
            assert_eq!(items.len(), 1);
        }
    }
}
