// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `Idempotency-Key` replay protection middleware (feature = `idempotency`).
//!
//! Only requests that opt in by carrying an `Idempotency-Key` header
//! participate: POST/PUT/PATCH with the header are claimed in the
//! [`IdempotencyStore`] before dispatch; completed responses are cached and
//! replayed verbatim (plus `Idempotency-Replayed: true`) for duplicates.
//! Concurrent duplicates while the first is in flight get `409 CONFLICT`
//! (rendered through the unified error contract). Requests without the
//! header — and all other methods — pass through with zero overhead.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::cache::{IdempotencyOutcome, IdempotencyStore};

/// Header carrying the caller-supplied idempotency key.
pub const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
/// Marker header added to replayed responses.
pub const IDEMPOTENCY_REPLAYED_HEADER: &str = "idempotency-replayed";
/// Marker header added when a response intentionally bypasses the cache.
pub const IDEMPOTENCY_SKIPPED_HEADER: &str = "idempotency-skipped";
/// 响应缓冲硬上限（防 OOM）：超过它的响应 abort 后以 500 收场。
/// 正常业务响应远低于此；上限判定用配置的 `max_response_bytes`。
pub const IDEMPOTENCY_HARD_CAP: usize = 32 * 1024 * 1024;

/// 幂等作用域 = 路由模式 + 主体标识。
///
/// 主体维度不可省：`Idempotency-Key` 由客户端自选，两个不同主体复用同一个
/// key 时不得共享同一条记录（否则后者的响应体会被回放给前者，且后者的写
/// 操作被静默吞掉）。
pub(crate) fn idempotency_scope(route: &str, principal: &str) -> String {
    format!("{route}:{principal}")
}

/// 幂等作用域中的主体标识。
///
/// 认证中间件早于幂等中间件执行（鉴权层后施加 → 更外层），故
/// `AuthContext` 已在 request extensions 中。未开启鉴权时退化为客户端 IP
/// （可用时）或 `anonymous`。
fn request_principal(
    #[cfg_attr(
        not(any(feature = "security", feature = "ratelimit-http")),
        allow(unused_variables)
    )]
    req: &axum::extract::Request,
) -> String {
    #[cfg(feature = "security")]
    if let Some(ctx) = req.extensions().get::<crate::security::AuthContext>()
        && let Some(user_id) = ctx.user_id()
    {
        return user_id.to_string();
    }

    #[cfg(any(feature = "security", feature = "ratelimit-http"))]
    if let Some(ip) = crate::security::extract_client_ip(req) {
        return ip;
    }

    "anonymous".to_string()
}

/// Build the idempotency middleware for `axum::middleware::from_fn`.
pub async fn idempotency_middleware(
    store: Arc<IdempotencyStore>,
    ttl_secs: i64,
    inflight_ttl_secs: i64,
    max_response_bytes: usize,
    req: Request<Body>,
    next: Next,
) -> Response {
    // 仅 POST/PUT/PATCH 且带 key 的请求参与。
    let method = req.method().clone();
    if !matches!(
        method,
        axum::http::Method::POST | axum::http::Method::PUT | axum::http::Method::PATCH
    ) {
        return next.run(req).await;
    }
    let Some(key) = req
        .headers()
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
    else {
        return next.run(req).await;
    };
    // scope 绑定路由模式（防止跨端点键冲突）。实测本仓生产接线（`Router::layer`）
    // 下 MatchedPath 在中间件内已可用（见 layer_wiring_shares_matched_path_scope_across_ids），
    // 故正常请求都走上一分支。兜底值不用常量 "unmatched"：那会把所有未匹配请求
    // （以及任何使 MatchedPath 缺失的上游行为变化）并入同一 scope，使同一 key 跨
    // 端点互相重放——属可预防的隐患面，改用请求 URI 路径保持端点隔离。
    let route = req
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());
    // scope 还必须绑定已认证主体：Idempotency-Key 由客户端自选，两个不同主体
    // 复用同一个 key 时，后者的响应体会被回放给前者，且后者的写操作被静默吞掉。
    let scope = idempotency_scope(&route, &request_principal(&req));

    match store.begin(&scope, &key, inflight_ttl_secs) {
        IdempotencyOutcome::Replay {
            body,
            status,
            content_type,
        } => {
            let status =
                axum::http::StatusCode::from_u16(status).unwrap_or(axum::http::StatusCode::OK);
            // 重放还原首次响应的 media type（缺省回退 JSON）。
            let ct = content_type.unwrap_or_else(|| "application/json".to_string());
            let mut resp = Response::builder()
                .status(status)
                .header(axum::http::header::CONTENT_TYPE, ct)
                .header(IDEMPOTENCY_REPLAYED_HEADER, "true")
                .body(Body::from(body))
                .unwrap_or_else(|_| axum::response::Response::new(Body::empty()));
            attach_trace_id(&mut resp);
            resp
        }
        IdempotencyOutcome::InFlight => {
            let unified = crate::error::unified::UnifiedError::new(
                "CONFLICT",
                "request with this Idempotency-Key is already in flight",
            );
            let mut resp =
                crate::error::unified::render_http(axum::http::StatusCode::CONFLICT, &unified);
            resp.headers_mut().insert(
                IDEMPOTENCY_REPLAYED_HEADER,
                axum::http::HeaderValue::from_static("in-flight"),
            );
            resp
        }
        IdempotencyOutcome::Execute => {
            let resp = next.run(req).await;
            let ok = resp.status().is_success();
            let status = resp.status().as_u16();
            if !ok {
                // 失败不缓存：调用方可立即重试（claim 已 abort）。
                store.abort(&scope, &key);
                return resp;
            }
            let (parts, body) = resp.into_parts();
            // 缓冲到硬上限（32 MiB）：为"超限透行"保住完整响应体。
            // 超过硬上限的极端响应无法还原 —— abort 后返回 500（文档化边界）。
            match axum::body::to_bytes(body, IDEMPOTENCY_HARD_CAP).await {
                Ok(bytes) => {
                    if bytes.len() > max_response_bytes {
                        // 复查：handler 副作用已提交，响应必须原样
                        // 返回（spec："超限不缓存但正常返回"）。返回 413 会
                        // 诱导客户端按幂等语义重试 → 重复副作用。
                        store.abort(&scope, &key);
                        let mut passthrough = Response::from_parts(parts, Body::from(bytes));
                        passthrough.headers_mut().insert(
                            IDEMPOTENCY_SKIPPED_HEADER,
                            axum::http::HeaderValue::from_static("oversized"),
                        );
                        return passthrough;
                    }
                    let content_type = parts
                        .headers
                        .get(axum::http::header::CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok());
                    store.complete(&scope, &key, status, content_type, bytes.to_vec(), ttl_secs);
                    let mut rebuilt = Response::from_parts(parts, Body::from(bytes));
                    rebuilt.headers_mut().insert(
                        IDEMPOTENCY_REPLAYED_HEADER,
                        axum::http::HeaderValue::from_static("executed"),
                    );
                    rebuilt
                }
                Err(_) => {
                    // 超过硬上限：claim 释放让调用方可重试；响应体已消费
                    // 无法还原，只能以 500 语义收场（极端边界，见上方注释）。
                    store.abort(&scope, &key);
                    let unified = crate::error::unified::UnifiedError::new(
                        "RESPONSE_TOO_LARGE",
                        format!(
                            "response exceeds idempotency buffering cap ({IDEMPOTENCY_HARD_CAP} bytes)"
                        ),
                    );
                    crate::error::unified::render_http(
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        &unified,
                    )
                }
            }
        }
    }
}

/// Attach ambient trace id header when the `context` feature is active.
fn attach_trace_id(resp: &mut Response) {
    #[cfg(feature = "context")]
    if let Some(ctx) = crate::context::current()
        && let Ok(value) = axum::http::HeaderValue::from_str(ctx.trace_id())
    {
        resp.headers_mut().insert("trace-id", value);
    }
    #[cfg(not(feature = "context"))]
    {
        let _ = resp;
    }
}

#[cfg(all(test, feature = "http", feature = "tokio"))]
mod idempotency_middleware_tests {
    use super::*;
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{Method, StatusCode};
    use axum::routing::{get, post};
    use axum::{Router, middleware};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tower::ServiceExt;

    /// 与生产接线完全同构：`middleware::from_fn` + 闭包捕获（`from_fn_with_state`
    /// 还要求状态类型 `Sync`，而 IdempotencyStore 的内部缓存句柄不满足）。
    macro_rules! idem_layer {
        ($pair:expr) => {{
            let (store, max_bytes) = $pair;
            middleware::from_fn(move |req: Request<Body>, next: Next| {
                let store = Arc::clone(&store);
                async move { idempotency_middleware(store, 60, 30, max_bytes, req, next).await }
            })
        }};
    }

    /// 外层 layer 接线（与 `http_impl.rs` 一致），/x /a /b 共用成功 handler。
    fn app_ok(state: (Arc<IdempotencyStore>, usize), hits: Arc<AtomicUsize>) -> Router {
        let counter = Arc::clone(&hits);
        let handler = move || {
            let c = Arc::clone(&counter);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                    .body(Body::from("payload"))
                    .unwrap()
            }
        };
        Router::new()
            .route("/x", post(handler.clone()))
            .route("/a", post(handler.clone()))
            .route("/b", post(handler))
            .route("/get", get(|| async { "read" }))
            .layer(idem_layer!(state))
    }

    /// 指定状态码/响应体的单路由应用（失败与超限分支）。
    fn app_with_response(
        state: (Arc<IdempotencyStore>, usize),
        hits: Arc<AtomicUsize>,
        status: StatusCode,
        body: Vec<u8>,
    ) -> Router {
        let counter = Arc::clone(&hits);
        let handler = move || {
            let c = Arc::clone(&counter);
            let payload = body.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Response::builder()
                    .status(status)
                    .body(Body::from(payload))
                    .unwrap()
            }
        };
        Router::new()
            .route("/x", post(handler))
            .layer(idem_layer!(state))
    }

    fn req(method: Method, uri: &str, key: Option<&str>) -> Request<Body> {
        let mut b = Request::builder().method(method).uri(uri);
        if let Some(k) = key {
            b = b.header(IDEMPOTENCY_KEY_HEADER, k);
        }
        b.body(Body::empty()).unwrap()
    }

    async fn body_text(resp: Response) -> String {
        let bytes = axum::body::to_bytes(resp.into_body(), IDEMPOTENCY_HARD_CAP)
            .await
            .unwrap_or_default();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn replayed(resp: &Response) -> Option<String> {
        resp.headers()
            .get(IDEMPOTENCY_REPLAYED_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    }

    fn state(store: Arc<IdempotencyStore>, max_bytes: usize) -> (Arc<IdempotencyStore>, usize) {
        (store, max_bytes)
    }

    #[tokio::test]
    async fn non_post_method_passes_through_without_participation() {
        let store = Arc::new(IdempotencyStore::new());
        let hits = Arc::new(AtomicUsize::new(0));
        let resp = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req(Method::GET, "/get", Some("k1")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_text(resp).await, "read");
        assert_eq!(
            hits.load(Ordering::SeqCst),
            0,
            "GET must not enter the idempotency counting path"
        );

        // GET 不得占用 key：同 key 的 POST 仍应真实执行
        let resp = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req(Method::POST, "/x", Some("k1")))
            .await
            .unwrap();
        assert_eq!(replayed(&resp).as_deref(), Some("executed"));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn missing_key_passes_through_and_never_caches() {
        let hits = Arc::new(AtomicUsize::new(0));
        let store = Arc::new(IdempotencyStore::new());
        for expect in 1..=2 {
            let resp = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
                .oneshot(req(Method::POST, "/x", None))
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            assert!(
                replayed(&resp).is_none(),
                "requests without a key must not get an idempotency header"
            );
            assert_eq!(
                hits.load(Ordering::SeqCst),
                expect,
                "each keyless request must really execute"
            );
        }
    }

    #[tokio::test]
    async fn execute_then_replay_restores_body_status_and_content_type() {
        let hits = Arc::new(AtomicUsize::new(0));
        let store = Arc::new(IdempotencyStore::new());
        let first = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req(Method::POST, "/x", Some("same-key")))
            .await
            .unwrap();
        assert_eq!(replayed(&first).as_deref(), Some("executed"));
        assert_eq!(
            first
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("text/plain; charset=utf-8")
        );
        assert_eq!(body_text(first).await, "payload");

        let second = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req(Method::POST, "/x", Some("same-key")))
            .await
            .unwrap();
        assert_eq!(
            replayed(&second).as_deref(),
            Some("true"),
            "replays must be marked true"
        );
        assert_eq!(
            second
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("text/plain; charset=utf-8"),
            "replays must restore the first response media type"
        );
        assert_eq!(body_text(second).await, "payload");
        assert_eq!(
            hits.load(Ordering::SeqCst),
            1,
            "replays must not run the handler again"
        );
    }

    #[tokio::test]
    async fn inflight_claim_yields_conflict_with_unified_error() {
        let store = Arc::new(IdempotencyStore::new());
        // 先手工 claim（等价于并发中的另一路请求正在执行），使中间件命中 InFlight。
        assert!(matches!(
            store.begin(&idempotency_scope("/x", "anonymous"), "busy-key", 30),
            IdempotencyOutcome::Execute
        ));
        let hits = Arc::new(AtomicUsize::new(0));
        let resp = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req(Method::POST, "/x", Some("busy-key")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        assert_eq!(replayed(&resp).as_deref(), Some("in-flight"));
        let text = body_text(resp).await;
        assert!(
            text.contains("CONFLICT"),
            "error body must have the UnifiedError shape: {text}"
        );
        assert_eq!(
            hits.load(Ordering::SeqCst),
            0,
            "InFlight must not run the handler"
        );
    }

    #[tokio::test]
    async fn failed_response_is_not_cached_so_retry_executes_again() {
        let hits = Arc::new(AtomicUsize::new(0));
        let store = Arc::new(IdempotencyStore::new());
        for expect in 1..=2 {
            let resp = app_with_response(
                state(Arc::clone(&store), 4096),
                Arc::clone(&hits),
                StatusCode::INTERNAL_SERVER_ERROR,
                b"boom".to_vec(),
            )
            .oneshot(req(Method::POST, "/x", Some("err-key")))
            .await
            .unwrap();
            assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(
                hits.load(Ordering::SeqCst),
                expect,
                "failed responses must not be cached: the caller must be able to retry immediately"
            );
        }
    }

    #[tokio::test]
    async fn oversized_response_passes_through_intact_with_skipped_header() {
        let hits = Arc::new(AtomicUsize::new(0));
        let store = Arc::new(IdempotencyStore::new());
        for expect in 1..=2 {
            let resp = app_with_response(
                state(Arc::clone(&store), 1_000),
                Arc::clone(&hits),
                StatusCode::OK,
                vec![b'x'; 8_000],
            )
            .oneshot(req(Method::POST, "/x", Some("big-key")))
            .await
            .unwrap();
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "oversized responses must pass through as-is; a 413 would invite clients to retry under idempotency semantics, duplicating side effects"
            );
            assert_eq!(
                resp.headers()
                    .get(IDEMPOTENCY_SKIPPED_HEADER)
                    .and_then(|v| v.to_str().ok()),
                Some("oversized")
            );
            assert_eq!(
                body_text(resp).await.len(),
                8_000,
                "the response body must stay complete and untruncated"
            );
            assert_eq!(
                hits.load(Ordering::SeqCst),
                expect,
                "oversized responses must not be cached"
            );
        }
    }

    /// 端点隔离（MatchedPath 可用时）：同一 key 打到 /a 与 /b 不得互重放。
    /// 兜底分支（MatchedPath 缺失）的隔离性由
    /// unmatched_paths_keep_separate_scope_in_fallback_branch 单独钉住。
    #[tokio::test]
    async fn same_key_on_different_endpoints_does_not_replay() {
        let hits = Arc::new(AtomicUsize::new(0));
        let store = Arc::new(IdempotencyStore::new());
        let a = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req(Method::POST, "/a", Some("shared")))
            .await
            .unwrap();
        assert_eq!(replayed(&a).as_deref(), Some("executed"));
        let b = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req(Method::POST, "/b", Some("shared")))
            .await
            .unwrap();
        assert_eq!(
            replayed(&b).as_deref(),
            Some("executed"),
            "the same key on different endpoints must not replay the other endpoint's response"
        );
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    /// MatchedPath 优先分支：`route_layer` 下路由已匹配，参数化路由的不同资源
    /// id 共享同一 scope（模式级幂等），第二个 id 直接重放首个响应。
    #[tokio::test]
    async fn route_layer_uses_matched_path_scope_across_resource_ids() {
        let hits = Arc::new(AtomicUsize::new(0));
        let store = Arc::new(IdempotencyStore::new());
        // axum Router 不实现 Clone（oneshot 消费服务），故按请求重建，store 共享。
        let counter = Arc::clone(&hits);
        let build = move || {
            let c = Arc::clone(&counter);
            let handler = move || {
                let c = Arc::clone(&c);
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("item"))
                        .unwrap()
                }
            };
            Router::new()
                .route("/item/{id}", post(handler))
                .route_layer(idem_layer!(state(Arc::clone(&store), 4096)))
        };
        let first = build()
            .oneshot(req(Method::POST, "/item/1", Some("pat-key")))
            .await
            .unwrap();
        assert_eq!(replayed(&first).as_deref(), Some("executed"));
        let second = build()
            .oneshot(req(Method::POST, "/item/2", Some("pat-key")))
            .await
            .unwrap();
        assert_eq!(
            replayed(&second).as_deref(),
            Some("true"),
            "the same parameterized pattern should share the scope (MatchedPath branch)"
        );
        assert_eq!(body_text(second).await, "item");
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    /// 超过缓冲硬上限（32 MiB）：响应体已被消费无法还原，abort 后以 500 显式报错
    /// （文档化边界），并释放 claim 让调用方可重试。
    #[tokio::test]
    async fn response_beyond_hard_cap_yields_server_error_and_releases_claim() {
        let hits = Arc::new(AtomicUsize::new(0));
        let store = Arc::new(IdempotencyStore::new());
        let resp = app_with_response(
            state(Arc::clone(&store), IDEMPOTENCY_HARD_CAP),
            Arc::clone(&hits),
            StatusCode::OK,
            vec![b'y'; IDEMPOTENCY_HARD_CAP + 1],
        )
        .oneshot(req(Method::POST, "/x", Some("huge")))
        .await
        .unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let text = body_text(resp).await;
        assert!(
            text.contains("RESPONSE_TOO_LARGE"),
            "responses beyond the hard cap must fail explicitly rather than being silently truncated: {text}"
        );
        // claim 已 abort：同 key 重试应再次进入执行分支而非 InFlight
        let again = app_with_response(
            state(Arc::clone(&store), IDEMPOTENCY_HARD_CAP),
            Arc::clone(&hits),
            StatusCode::OK,
            vec![b'z'; 16],
        )
        .oneshot(req(Method::POST, "/x", Some("huge")))
        .await
        .unwrap();
        assert_eq!(again.status(), StatusCode::OK);
        assert_eq!(
            hits.load(Ordering::SeqCst),
            2,
            "a retry after abort must really execute"
        );
    }

    #[tokio::test]
    async fn layer_wiring_shares_matched_path_scope_across_ids() {
        let hits = Arc::new(AtomicUsize::new(0));
        let store = Arc::new(IdempotencyStore::new());
        let counter = Arc::clone(&hits);
        let build = move || {
            let c = Arc::clone(&counter);
            let handler = move || {
                let c = Arc::clone(&c);
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("p"))
                        .unwrap()
                }
            };
            Router::new()
                .route("/item/{id}", post(handler))
                .layer(idem_layer!(state(Arc::clone(&store), 4096)))
        };
        build()
            .oneshot(req(Method::POST, "/item/1", Some("probe")))
            .await
            .unwrap();
        let second = build()
            .oneshot(req(Method::POST, "/item/2", Some("probe")))
            .await
            .unwrap();
        // 实测结论（axum 0.8 + 本仓生产接线 Router::layer）：外层 layer 也能拿到
        // MatchedPath，故参数化路由的不同 id 共享同一 scope（模式级幂等）。
        // 该断言同时钉住"未退化为按 URI 路径分 scope"——若上游改变 layer 与路由
        // 匹配的先后顺序，此处会先红，而不是让跨 id 语义悄悄漂移。
        assert_eq!(
            replayed(&second).as_deref(),
            Some("true"),
            "MatchedPath must be visible to the outer layer; parameterized routes share one scope per pattern"
        );
    }

    /// 兜底分支判别：走 `fallback` 的未知路径拿不到 MatchedPath，scope 必须改取
    /// 请求 URI 路径。若退回常量 "unmatched"，两条不同路径的同一 key 会互相
    /// 重放另一路径的响应体（本例两条路径均返回 2xx，因此写得进缓存）。
    #[tokio::test]
    async fn unmatched_paths_keep_separate_scope_in_fallback_branch() {
        let hits = Arc::new(AtomicUsize::new(0));
        let store = Arc::new(IdempotencyStore::new());
        let counter = Arc::clone(&hits);
        let build = move || {
            let c = Arc::clone(&counter);
            Router::new()
                .fallback(move || {
                    let c = Arc::clone(&c);
                    async move {
                        c.fetch_add(1, Ordering::SeqCst);
                        Response::builder()
                            .status(StatusCode::OK)
                            .body(Body::from("fallback"))
                            .unwrap()
                    }
                })
                .layer(idem_layer!(state(Arc::clone(&store), 4096)))
        };

        let first = build()
            .oneshot(req(Method::POST, "/path-a", Some("fb-key")))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(replayed(&first).as_deref(), Some("executed"));

        let second = build()
            .oneshot(req(Method::POST, "/path-b", Some("fb-key")))
            .await
            .unwrap();
        assert_eq!(
            replayed(&second).as_deref(),
            Some("executed"),
            "two unknown paths must not share a scope (they take the URI fallback, not \"unmatched\")"
        );
        assert_eq!(
            hits.load(Ordering::SeqCst),
            2,
            "each path must go to the origin on its own"
        );
    }

    #[tokio::test]
    async fn replay_with_unparsable_cached_status_falls_back_to_ok() {
        // 缓存条目状态码非法（历史数据/外部写入）时重放不得 panic，回退 200。
        let store = Arc::new(IdempotencyStore::new());
        store.complete(
            &idempotency_scope("/x", "anonymous"),
            "bad-status",
            1_000,
            None,
            b"body".to_vec(),
            60,
        );
        let hits = Arc::new(AtomicUsize::new(0));
        let resp = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req(Method::POST, "/x", Some("bad-status")))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "an illegal cacheable status code must fall back to 200 rather than panic"
        );
        assert_eq!(replayed(&resp).as_deref(), Some("true"));
        assert_eq!(
            resp.headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/json"),
            "replay must fall back to JSON when content_type is missing"
        );
        assert_eq!(body_text(resp).await, "body");
    }

    /// 构造带已认证主体的请求（生产链路由鉴权中间件注入 `AuthContext`）。
    #[cfg(feature = "security")]
    fn req_as(method: Method, uri: &str, key: Option<&str>, user_id: &str) -> Request<Body> {
        let mut request = req(method, uri, key);
        request
            .extensions_mut()
            .insert(crate::security::AuthContext::new(
                Some(user_id.to_string()),
                vec!["read".to_string()],
                crate::security::AuthMetadata::default(),
            ));
        request
    }

    /// 同一 `Idempotency-Key` 打到两个不同主体：不得共享响应——第二个主体
    /// 的写操作不得被静默吞掉，响应体也不得回放给另一个主体。
    #[cfg(feature = "security")]
    #[tokio::test]
    async fn same_key_from_different_principals_does_not_replay() {
        let store = Arc::new(IdempotencyStore::new());
        let hits = Arc::new(AtomicUsize::new(0));

        let first = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req_as(Method::POST, "/x", Some("shared-key"), "user-a"))
            .await
            .unwrap();
        assert_eq!(replayed(&first).as_deref(), Some("executed"));

        let second = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req_as(Method::POST, "/x", Some("shared-key"), "user-b"))
            .await
            .unwrap();
        assert_eq!(
            replayed(&second).as_deref(),
            Some("executed"),
            "a different principal must not replay the first principal's response"
        );
        assert_eq!(
            hits.load(Ordering::SeqCst),
            2,
            "both principals must reach the handler"
        );
    }

    /// 同一主体的同一 key 仍然重放（主体隔离不得误伤正常重放）。
    #[cfg(feature = "security")]
    #[tokio::test]
    async fn same_key_from_same_principal_replays() {
        let store = Arc::new(IdempotencyStore::new());
        let hits = Arc::new(AtomicUsize::new(0));

        let first = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req_as(Method::POST, "/x", Some("own-key"), "user-a"))
            .await
            .unwrap();
        assert_eq!(replayed(&first).as_deref(), Some("executed"));

        let second = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits))
            .oneshot(req_as(Method::POST, "/x", Some("own-key"), "user-a"))
            .await
            .unwrap();
        assert_eq!(
            replayed(&second).as_deref(),
            Some("true"),
            "the same principal must still get the replayed response"
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    /// 两个主体并发使用同一 key：两条请求各自执行，不得出现"在途冲突"
    /// （409）或互相重放。
    #[cfg(feature = "security")]
    #[tokio::test]
    async fn concurrent_same_key_from_different_principals_both_execute() {
        let store = Arc::new(IdempotencyStore::new());
        let hits = Arc::new(AtomicUsize::new(0));

        let app_a = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits));
        let app_b = app_ok(state(Arc::clone(&store), 4096), Arc::clone(&hits));
        let (resp_a, resp_b) = tokio::join!(
            app_a.oneshot(req_as(Method::POST, "/x", Some("race-key"), "user-a")),
            app_b.oneshot(req_as(Method::POST, "/x", Some("race-key"), "user-b")),
        );
        let resp_a = resp_a.unwrap();
        let resp_b = resp_b.unwrap();
        assert_eq!(
            resp_a.status(),
            StatusCode::OK,
            "the first principal must not be rejected"
        );
        assert_eq!(
            resp_b.status(),
            StatusCode::OK,
            "a concurrent request from another principal must not be rejected as in-flight"
        );
        assert_eq!(replayed(&resp_a).as_deref(), Some("executed"));
        assert_eq!(replayed(&resp_b).as_deref(), Some("executed"));
        assert_eq!(
            hits.load(Ordering::SeqCst),
            2,
            "both principals must reach the handler"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_constants_are_lowercase() {
        // HTTP/2 要求小写 header 名。
        assert_eq!(IDEMPOTENCY_KEY_HEADER, "idempotency-key");
        assert_eq!(IDEMPOTENCY_REPLAYED_HEADER, "idempotency-replayed");
    }
}
