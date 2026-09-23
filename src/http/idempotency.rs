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
    // scope 绑定路由模式（防止跨端点键冲突）。
    let scope = req
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "unmatched".to_string());

    match store.begin(&scope, &key, inflight_ttl_secs) {
        IdempotencyOutcome::Replay {
            body,
            status,
            content_type,
        } => {
            let status =
                axum::http::StatusCode::from_u16(status).unwrap_or(axum::http::StatusCode::OK);
            // T008：重放还原首次响应的 media type（缺省回退 JSON）。
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
                        // 复查 H-2 修复：handler 副作用已提交，响应必须原样
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
