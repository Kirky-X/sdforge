// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;
use crate::core::{ApiError, HandlerArgs, HandlerFn, HandlerState, extract_value};
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
}

#[cfg(feature = "grpc")]
impl Default for SdForgeGrpcService {
    fn default() -> Self {
        Self {
            state: None,
            handlers: OnceLock::new(),
            body_params: OnceLock::new(),
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
}

#[cfg(feature = "grpc")]
#[tonic::async_trait]
impl SdForgeService for SdForgeGrpcService {
    async fn call(&self, request: Request<CallRequest>) -> Result<Response<CallResponse>, Status> {
        let req = request.into_inner();

        // R-grpc-001: lookup handler by method name
        let handler = self.handlers().get(req.method.as_str()).copied().ok_or_else(
            || {
                Status::not_found(format!(
                    "method '{}' not registered (no matching #[forge(grpc_method = \"...\")] declaration)",
                    req.method
                ))
            },
        )?;

        // R-grpc-003: parameters → args, data → body_param key
        let mut args: HandlerArgs = req.parameters.into_iter().collect();
        if !req.data.is_empty() {
            match self.body_params().get(req.method.as_str()).copied().flatten() {
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

        // R-grpc-005: catch_unwind so a panicking handler never leaks internal
        // paths / stack data through gRPC error messages (security rule).
        use futures_util::FutureExt;
        use std::panic::AssertUnwindSafe;
        let outcome = AssertUnwindSafe(handler(args, self.state.clone()))
            .catch_unwind()
            .await;

        match outcome {
            Ok(Ok(value)) => {
                // R-grpc-004: smart extract_value (String → raw, others → JSON)
                Ok(Response::new(CallResponse {
                    success: true,
                    data: extract_value(&value),
                    error: String::new(),
                    status_code: 200,
                }))
            }
            Ok(Err(e)) => {
                // R-grpc-005: business error → success:false + Status::ok (so
                // the client can read the body for error details).
                let status_code = map_error_to_http(&e);
                Ok(Response::new(CallResponse {
                    success: false,
                    data: String::new(),
                    error: e.to_string(),
                    status_code,
                }))
            }
            Err(_panic) => {
                // R-grpc-005: handler panicked → generic internal error.
                // Never expose panic payload to the client (security).
                Err(Status::internal("handler panicked"))
            }
        }
    }

    async fn get_info(
        &self,
        _request: Request<InfoRequest>,
    ) -> Result<Response<InfoResponse>, Status> {
        let response = InfoResponse {
            name: "SdForge Service".to_string(),
            version: "0.1.0".to_string(),
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

/// Map an `ApiError` variant to its HTTP-equivalent status code.
///
/// Used to populate `CallResponse.status_code` so gRPC clients can read the
/// semantic HTTP code of a business error without parsing the error message.
/// Mirrors the HTTP error mapping convention.
#[cfg(feature = "grpc")]
fn map_error_to_http(e: &ApiError) -> i32 {
    match e {
        ApiError::NotFound { .. } => 404,
        ApiError::InvalidInput { .. } | ApiError::ValidationError { .. } => 422,
        ApiError::AuthenticationFailed { .. } => 401,
        ApiError::AccessDenied { .. } => 403,
        ApiError::RateLimitExceeded { .. } => 429,
        ApiError::ServiceUnavailable { .. } => 503,
        ApiError::Internal { .. } => 500,
    }
}

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
#[deprecated(note = "use build_server_with_config with auth configured; build_server starts an unauthenticated server")]
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
    // T008: pass `config.state` into the service so handlers with State
    // parameters can downcast it at call time.
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
    pub(crate) fn body_params_map(
        &self,
    ) -> &HashMap<&'static str, Option<&'static str>> {
        self.body_params()
    }
}

// ============================================================================
// T006/T007 unit tests
// ============================================================================
#[cfg(all(test, feature = "grpc"))]
mod tests {
    use super::*;
    use crate::core::HandlerArgs;
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
        }
    }

    /// Handler that always returns `Err(NotFound)` — verifies the error →
    /// `success:false` + `status_code:404` + `Status::ok` mapping (R-grpc-005).
    fn not_found_handler(
        _args: HandlerArgs,
        _state: HandlerState,
    ) -> crate::core::HandlerFuture {
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
        }
    }

    #[test]
    fn lookup_builds_cache_from_inventory() {
        // T006: OnceLock is initialized lazily on first `handlers()` call.
        let service = SdForgeGrpcService::default();
        let table = service.handlers();
        assert!(table.contains_key("test_echo"));
        assert!(table.contains_key("test_not_found"));
        assert!(table.contains_key("test_panic"));
        assert!(table.contains_key("test_body"));
    }

    #[test]
    fn lookup_cache_is_idempotent() {
        // R-grpc-006: subsequent `handlers()` calls return the same map.
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
        // R-grpc-001: real routing replaces stub `processed` response.
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
        // R-grpc-005: method not registered → Status::not_found
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
        // R-grpc-003: data non-empty but method has no body_param → invalid_argument
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
        // R-grpc-003: data injected into body_param key
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
    async fn call_business_error_returns_success_false_with_status_code() {
        // R-grpc-005: business error → Status::ok, success:false, error in body
        let service = SdForgeGrpcService::default();
        let req = Request::new(CallRequest {
            method: "test_not_found".to_string(),
            parameters: HashMap::new(),
            data: String::new(),
        });
        let resp = service.call(req).await.unwrap().into_inner();
        assert!(!resp.success);
        assert_eq!(resp.status_code, 404);
        assert!(resp.error.contains("test_resource"));
        assert!(resp.data.is_empty());
    }

    #[tokio::test]
    async fn call_panic_handler_returns_status_internal() {
        // R-grpc-005: handler panic → Status::internal, message generic
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

    #[test]
    fn map_error_to_http_covers_all_variants() {
        // R-grpc-005: exhaustive ApiError → HTTP code mapping
        let cases: Vec<(ApiError, i32)> = vec![
            (
                ApiError::NotFound {
                    resource: "x".into(),
                    resource_id: None,
                },
                404,
            ),
            (
                ApiError::InvalidInput {
                    message: "x".into(),
                    field: None,
                    value: None,
                },
                422,
            ),
            (
                ApiError::ValidationError {
                    field: "x".into(),
                    constraint: "required".into(),
                },
                422,
            ),
            (
                ApiError::AuthenticationFailed {
                    reason: "x".into(),
                },
                401,
            ),
            (
                ApiError::AccessDenied {
                    permission: "x".into(),
                    user_id: None,
                },
                403,
            ),
            (
                ApiError::RateLimitExceeded {
                    limit: 10,
                    window_seconds: 60,
                },
                429,
            ),
            (
                ApiError::ServiceUnavailable {
                    service: "x".into(),
                    retry_after: None,
                    source: None,
                },
                503,
            ),
            (
                ApiError::Internal {
                    message: "x".into(),
                    error_id: "x".into(),
                    source: None,
                    context: None,
                },
                500,
            ),
        ];
        for (e, expected) in cases {
            assert_eq!(map_error_to_http(&e), expected, "mismatch for {e:?}");
        }
    }

    #[test]
    fn default_state_is_none() {
        // R-grpc-007: Default::default().state == None
        let config = GrpcServerConfig::default();
        assert!(config.state.is_none());
    }

    #[tokio::test]
    async fn state_injected_to_service_can_be_downcast() {
        // R-grpc-007: state injected via GrpcServerConfig reaches the service.
        use std::any::Any;
        let state: Arc<dyn Any + Send + Sync> = Arc::new(42_i32);
        let service = SdForgeGrpcService::with_state(Some(state));
        // Downcast back to i32 to verify the value survived.
        // (Real handlers use `downcast_state` from core::handler — T011.)
        let borrowed = service.state.clone().unwrap();
        let downcast = borrowed.downcast_ref::<i32>();
        assert_eq!(downcast, Some(&42_i32));
    }
}
