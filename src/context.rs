// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Request context propagation (T705, R-sd4-003).
//!
//! A [`RequestContext`] (request_id + trace_id + start time) is generated per
//! request and carried across protocol boundaries via `tokio::task_local`:
//!
//! - **HTTP** — `context_middleware` (auto-installed by `build_with_config`)
//!   resolves/creates ids from `X-Request-Id` / `X-Trace-Id` / W3C
//!   `traceparent`, wraps the rest of the call chain, and echoes both ids on
//!   the response.
//! - **gRPC** — `SdForgeGrpcService::call` wraps dispatch with a context.
//! - **WebSocket** — `handle_socket` entry adopts/creates a context.
//! - **MCP** — the stateless handler wraps tool dispatch.
//! - **Logging** — `StructuredLogger` appends `request_id`/`trace_id` fields
//!   from the ambient context (`logging` + `context` features).
//!
//! Ids are process-unique without external dependencies (monotonic counter +
//! wall-clock nanos); correlation is what matters, not global uniqueness.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

tokio::task_local! {
    static REQUEST_CONTEXT: RequestContext;
}

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Process-unique, dependency-free identifier: `prefix-<nanos>-<counter>`.
pub fn generate_id(prefix: &str) -> String {
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}-{counter:x}")
}

/// Per-request correlation context.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Stable request identifier (echoed as `X-Request-Id`).
    request_id: String,
    /// Distributed trace identifier (echoed as `X-Trace-Id`; from W3C
    /// `traceparent` when present).
    trace_id: String,
    /// When the request entered the system.
    started_at: Instant,
}

impl RequestContext {
    /// Create a context with fresh ids.
    pub fn new() -> Self {
        Self {
            request_id: generate_id("req"),
            trace_id: generate_id("trace"),
            started_at: Instant::now(),
        }
    }

    /// Create a context with explicit ids (from inbound headers).
    pub fn with_ids(request_id: String, trace_id: String) -> Self {
        Self {
            request_id,
            trace_id,
            started_at: Instant::now(),
        }
    }

    /// Request identifier.
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Trace identifier.
    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    /// Elapsed time since request start.
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Run `fut` with `ctx` installed as the ambient request context.
pub async fn scope<F: std::future::Future>(ctx: RequestContext, fut: F) -> F::Output {
    REQUEST_CONTEXT.scope(ctx, fut).await
}

/// Run a synchronous closure with `ctx` installed (for sync dispatch paths,
/// e.g. MCP's internal tool dispatch).
pub fn scope_sync<R>(ctx: RequestContext, f: impl FnOnce() -> R) -> R {
    REQUEST_CONTEXT.sync_scope(ctx, f)
}

/// Read the ambient context, if any.
pub fn current() -> Option<RequestContext> {
    REQUEST_CONTEXT
        .try_with(|ctx| ctx.clone())
        .ok()
}

/// Read the ambient context or synthesize a fresh one (never fails).
pub fn current_or_new() -> RequestContext {
    current().unwrap_or_default()
}

/// Log correlation fields for the ambient context (empty when none active).
pub fn log_fields() -> Vec<(String, serde_json::Value)> {
    match current() {
        Some(ctx) => vec![
            ("request_id".to_string(), serde_json::json!(ctx.request_id())),
            ("trace_id".to_string(), serde_json::json!(ctx.trace_id())),
        ],
        None => Vec::new(),
    }
}

/// HTTP middleware: resolve/create ids, install context for the rest of the
/// chain, echo ids on the response.
#[cfg(feature = "http")]
pub async fn context_middleware(
    mut req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| generate_id("req"));
    let trace_id = req
        .headers()
        .get("x-trace-id")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            // W3C Trace Context: version-traceid-spanid-flags
            req.headers()
                .get("traceparent")
                .and_then(|v| v.to_str().ok())
                .and_then(|tp| tp.split('-').nth(1))
                .filter(|t| !t.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| generate_id("trace"));

    let header_req = axum::http::HeaderValue::from_str(&request_id)
        .unwrap_or_else(|_| axum::http::HeaderValue::from_static("invalid-request-id"));
    let header_trace = axum::http::HeaderValue::from_str(&trace_id)
        .unwrap_or_else(|_| axum::http::HeaderValue::from_static("invalid-trace-id"));

    // Surface the resolved ids to inner layers/handlers via request headers.
    req.headers_mut().insert(
        axum::http::header::HeaderName::from_static("x-request-id"),
        header_req.clone(),
    );

    let ctx = RequestContext::with_ids(request_id, trace_id);
    let mut response = scope(ctx, next.run(req)).await;
    response
        .headers_mut()
        .insert(axum::http::header::HeaderName::from_static("x-request-id"), header_req);
    response
        .headers_mut()
        .insert(axum::http::header::HeaderName::from_static("x-trace-id"), header_trace);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn scope_installs_and_restores_context() {
        let ctx = RequestContext::with_ids("req-1".into(), "trace-1".into());
        scope(ctx.clone(), async {
            let current = current().expect("context visible inside scope");
            assert_eq!(current.request_id(), "req-1");
            assert_eq!(current.trace_id(), "trace-1");
        })
        .await;
        assert!(current().is_none(), "context must not leak outside scope");
    }

    #[tokio::test]
    async fn nested_scopes_shadow_outer() {
        scope(RequestContext::with_ids("outer".into(), "t-outer".into()), async {
            scope(RequestContext::with_ids("inner".into(), "t-inner".into()), async {
                assert_eq!(current().unwrap().request_id(), "inner");
            })
            .await;
            assert_eq!(current().unwrap().request_id(), "outer");
        })
        .await;
    }

    #[test]
    fn generated_ids_are_unique_and_prefixed() {
        let a = generate_id("req");
        let b = generate_id("req");
        assert_ne!(a, b);
        assert!(a.starts_with("req-"));
    }

    #[tokio::test]
    async fn current_or_new_synthesizes_fresh_context() {
        let ctx = current_or_new();
        assert!(ctx.request_id().starts_with("req-"));
        assert!(ctx.trace_id().starts_with("trace-"));
    }

    #[tokio::test]
    async fn context_survives_spawned_subtask_only_via_explicit_scope() {
        // task_local does not propagate into tokio::spawn — document the
        // semantics: ids must be passed explicitly (e.g. via headers).
        let ctx = RequestContext::with_ids("parent".into(), "t".into());
        let fetched = scope(ctx, async {
            tokio::spawn(async { current().is_none() }).await.unwrap()
        })
        .await;
        assert!(fetched, "spawned task sees no ambient context (by design)");
    }

    #[tokio::test]
    async fn log_fields_reflect_ambient_context() {
        assert!(log_fields().is_empty());
        scope(RequestContext::with_ids("rq".into(), "tr".into()), async {
            let fields = log_fields();
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].0, "request_id");
            assert_eq!(fields[0].1, "rq");
            assert_eq!(fields[1].1, "tr");
        })
        .await;
    }

    #[cfg(feature = "http")]
    mod http_mw {
        #![allow(clippy::needless_return)]
        use super::*;
        use axum::body::Body;
        use tower::ServiceExt;

        fn app() -> axum::Router {
            axum::Router::new()
                .route(
                    "/echo",
                    axum::routing::get(|| async {
                        let ctx = current().expect("context visible in handler");
                        (axum::http::StatusCode::OK, format!("{}/{}", ctx.request_id(), ctx.trace_id()))
                    }),
                )
                .layer(axum::middleware::from_fn(context_middleware))
        }

        async fn get_with(headers: &[(&str, &str)]) -> axum::http::Response<Body> {
            let mut builder = axum::http::Request::builder().uri("/echo");
            for (k, v) in headers {
                builder = builder.header(*k, *v);
            }
            app()
                .oneshot(builder.body(Body::empty()).unwrap())
                .await
                .unwrap()
        }

        #[tokio::test]
        async fn middleware_generates_and_echoes_ids() {
            let resp = get_with(&[]).await;
            assert_eq!(resp.status(), 200);
            let req_id = resp
                .headers()
                .get("x-request-id")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let trace_id = resp
                .headers()
                .get("x-trace-id")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            assert!(req_id.starts_with("req-"));
            assert!(trace_id.starts_with("trace-"));
            let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap();
            assert_eq!(body, format!("{req_id}/{trace_id}"));
        }

        #[tokio::test]
        async fn middleware_preserves_inbound_ids() {
            let resp =
                get_with(&[("x-request-id", "my-req"), ("x-trace-id", "my-trace")]).await;
            let req_id = resp
                .headers()
                .get("x-request-id")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let trace_id = resp
                .headers()
                .get("x-trace-id")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            assert_eq!(req_id, "my-req");
            assert_eq!(trace_id, "my-trace");
        }

        #[tokio::test]
        async fn middleware_extracts_trace_from_w3c_traceparent() {
            let resp = get_with(&[(
                "traceparent",
                "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            )])
            .await;
            let trace_id = resp.headers().get("x-trace-id").unwrap().to_str().unwrap();
            assert_eq!(trace_id, "0af7651916cd43dd8448eb211c80319c");
        }
    }
}
