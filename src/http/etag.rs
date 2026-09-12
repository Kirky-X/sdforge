// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! ETag / conditional requests.
//!
//! With the `etag` feature, `build_with_config` installs
//! [`etag_middleware`]: successful (2xx) GET responses get a strong ETag
//! (SHA-256 of the body, quoted per RFC 7232); a follow-up GET carrying
//! `If-None-Match` that matches the current ETag (or `*`) short-circuits to
//! **304 Not Modified** with an empty body.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Request-body cap when buffering a response for fingerprinting.
const MAX_ETAG_BODY_BYTES: usize = 10 * 1024 * 1024;

/// Compute a strong ETag for a body: quoted SHA-256 hex per RFC 7232.
fn strong_etag(body: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    let digest = Sha256::digest(body);
    let mut hex = String::with_capacity(64);
    for b in digest.iter() {
        let _ = write!(hex, "{b:02x}");
    }
    format!("\"{hex}\"")
}

/// Whether the request's `If-None-Match` header matches `etag`.
///
/// Uses RFC 7232 weak comparison: a `W/"..."` candidate matches the strong
/// etag produced here.
fn if_none_match_matches(header_value: &str, etag: &str) -> bool {
    header_value
        .split(',')
        .map(|t| t.trim())
        .map(|candidate| candidate.strip_prefix("W/").unwrap_or(candidate))
        .any(|candidate| candidate == "*" || candidate == etag)
}

/// ETag / If-None-Match middleware.
pub async fn etag_middleware(req: Request<Body>, next: Next) -> Response {
    let is_get = req.method() == axum::http::Method::GET;
    let if_none_match = req
        .headers()
        .get(axum::http::header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let mut response = next.run(req).await;

    // Only fingerprint cacheable successful GET responses that don't already
    // carry an ETag.
    if !is_get
        || !response.status().is_success()
        || response.headers().contains_key(axum::http::header::ETAG)
    {
        return response;
    }

    let (mut parts, body) = response.into_parts();

    // A body with a known-oversized Content-Length is passed through
    // untouched: buffering it would consume the stream, and a failed read
    // cannot be replayed to the client.
    let oversized = parts
        .headers
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|len| len > MAX_ETAG_BODY_BYTES as u64);
    if oversized {
        return Response::from_parts(parts, body);
    }

    let bytes = match axum::body::to_bytes(body, MAX_ETAG_BODY_BYTES).await {
        Ok(b) => b,
        Err(_) => {
            // The buffered body cannot be replayed; return an explicit error
            // rather than a silently empty 200.
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let etag = strong_etag(&bytes);
    let matches = if_none_match
        .as_deref()
        .map(|inm| if_none_match_matches(inm, &etag))
        .unwrap_or(false);

    // The etag is quoted hex — always a valid header value — so mutate the
    // original parts in place: every original header (and the body) survives.
    if let Ok(etag_value) = axum::http::HeaderValue::from_str(&etag) {
        parts.headers.insert(axum::http::header::ETAG, etag_value);
    }
    if matches {
        parts.status = StatusCode::NOT_MODIFIED;
        // RFC 7232: 304 must not carry Content-Length of the elided body.
        parts.headers.remove(axum::http::header::CONTENT_LENGTH);
        return Response::from_parts(parts, Body::empty());
    }
    Response::from_parts(parts, Body::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    fn app() -> axum::Router {
        axum::Router::new()
            .route(
                "/resource",
                axum::routing::get(|| async { "payload-v1" }),
            )
            .route(
                "/post-only",
                axum::routing::post(|| async { "created" }),
            )
            .layer(axum::middleware::from_fn(etag_middleware))
    }

    async fn send(
        method: &str,
        uri: &str,
        if_none_match: Option<&str>,
    ) -> Response {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(inm) = if_none_match {
            builder = builder.header(axum::http::header::IF_NONE_MATCH, inm);
        }
        app()
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn get_response_carries_strong_etag() {
        let res = send("GET", "/resource", None).await;
        assert_eq!(res.status(), 200);
        let etag = res
            .headers()
            .get(axum::http::header::ETAG)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(etag.starts_with('"') && etag.ends_with('"'), "quoted: {etag}");
        assert_eq!(etag.len(), 66, "quoted sha256 hex = 64 + 2 quotes");
    }

    #[tokio::test]
    async fn matching_if_none_match_returns_304() {
        let res = send("GET", "/resource", None).await;
        let etag = res
            .headers()
            .get(axum::http::header::ETAG)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let res = send("GET", "/resource", Some(&etag)).await;
        assert_eq!(res.status(), 304);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(body.is_empty(), "304 must elide the body");
    }

    #[tokio::test]
    async fn star_if_none_match_returns_304() {
        let res = send("GET", "/resource", Some("*")).await;
        assert_eq!(res.status(), 304);
    }

    #[tokio::test]
    async fn weak_if_none_match_candidate_matches_strong_etag() {
        let res = send("GET", "/resource", None).await;
        let etag = res
            .headers()
            .get(axum::http::header::ETAG)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // RFC 7232 weak comparison: W/"foo" must match "foo".
        let weak = format!("W/{etag}");
        let res = send("GET", "/resource", Some(&weak)).await;
        assert_eq!(res.status(), 304, "weak candidate must use weak comparison");
    }

    #[tokio::test]
    async fn mismatched_if_none_match_returns_200_with_body() {
        let res = send("GET", "/resource", Some("\"deadbeef\"")).await;
        assert_eq!(res.status(), 200);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body, "payload-v1");
    }

    #[tokio::test]
    async fn non_get_methods_are_not_fingerprinted() {
        let res = send("POST", "/post-only", None).await;
        assert_eq!(res.status(), 200);
        assert!(
            res.headers().get(axum::http::header::ETAG).is_none(),
            "POST responses must not gain an ETag"
        );
    }
}
