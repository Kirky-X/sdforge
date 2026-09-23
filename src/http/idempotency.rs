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

/// Build the idempotency middleware for `axum::middleware::from_fn`.
pub async fn idempotency_middleware(
    store: Arc<IdempotencyStore>,
    ttl_secs: i64,
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
    // scope 绑定路由模式（防止跨端点键冲突）。
    let scope = req
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "unmatched".to_string());

    match store.begin(&scope, &key, 30) {
        IdempotencyOutcome::Replay(body, status) => {
            let status =
                axum::http::StatusCode::from_u16(status).unwrap_or(axum::http::StatusCode::OK);
            let mut resp = Response::builder()
                .status(status)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
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
            match axum::body::to_bytes(body, max_response_bytes).await {
                Ok(bytes) => {
                    store.complete(&scope, &key, status, bytes.to_vec(), ttl_secs);
                    let mut rebuilt = Response::from_parts(parts, Body::from(bytes));
                    rebuilt.headers_mut().insert(
                        IDEMPOTENCY_REPLAYED_HEADER,
                        axum::http::HeaderValue::from_static("executed"),
                    );
                    rebuilt
                }
                Err(_) => {
                    // 超过 max_response_bytes：正常返回但不缓存。
                    store.abort(&scope, &key);
                    // body 已被消费且无法还原 —— 返回 413 语义的统一错误。
                    let unified = crate::error::unified::UnifiedError::new(
                        "PAYLOAD_TOO_LARGE",
                        format!(
                            "response exceeds idempotency max_response_bytes ({max_response_bytes})"
                        ),
                    );
                    crate::error::unified::render_http(
                        axum::http::StatusCode::PAYLOAD_TOO_LARGE,
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
