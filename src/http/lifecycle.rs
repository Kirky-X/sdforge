// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Endpoint lifecycle response headers（`#[forge(deprecated, sunset,
//! successor)]`，feature = `http`）。
//!
//! Declared lifecycle metadata on [`sdforge::core::ApiMetadata`] is injected
//! into HTTP responses as `Deprecation: true`, `Sunset: <value>` and
//! `Link: <successor>; rel="successor-version"` headers. Endpoint-level
//! declarations **take precedence** over the global
//! [`VersionRouterConfig::deprecated_versions`](crate::VersionRouterConfig)
//! fallback: the version-routing middleware skips its own injection when the
//! response already carries any lifecycle header (`Deprecation`, `Sunset` or
//! `Link`) — a sunset-only / successor-only endpoint must not be relabeled
//! deprecated nor lose its endpoint-declared values (the per-route layer runs
//! inside it).

use crate::core::LifecycleMeta;
use axum::body::Body;
use axum::http::HeaderName;
use axum::response::Response;

/// Inject lifecycle headers into a response header map.
///
/// Shared by the per-route HTTP layer and usable by hosts building custom
/// responses. `successor_link_target` (when both sides are non-empty) renders
/// a `Link: <target>; rel="successor-version"` header.
pub fn inject_lifecycle_headers(
    headers: &mut axum::http::HeaderMap,
    lifecycle: &LifecycleMeta,
    successor_link_target: Option<&str>,
) {
    if !lifecycle.is_present() {
        return;
    }
    if lifecycle.deprecated {
        headers.insert(
            HeaderName::from_static("deprecation"),
            axum::http::HeaderValue::from_static("true"),
        );
    }
    if let Some(sunset) = &lifecycle.sunset
        && let Ok(value) = axum::http::HeaderValue::from_str(sunset)
    {
        headers.insert(HeaderName::from_static("sunset"), value);
    }
    let link = match (successor_link_target, &lifecycle.successor) {
        (Some(target), Some(_)) => Some(format!("<{target}>; rel=\"successor-version\"")),
        (None, Some(successor)) => Some(format!("<{successor}>; rel=\"successor-version\"")),
        _ => None,
    };
    if let Some(link) = link
        && let Ok(value) = axum::http::HeaderValue::from_str(&link)
    {
        headers.insert(axum::http::header::LINK, value);
    }
}

/// Wrap a method router with the endpoint lifecycle header layer.
///
/// Emitted by the `#[forge]` macro for routes declaring
/// `deprecated` / `sunset` / `successor`; `None` (or an all-empty
/// [`LifecycleMeta`]) returns the router untouched so unannotated routes pay
/// zero overhead. Operates on [`axum::routing::MethodRouter`] — the type the
/// macro's per-route registration tail carries.
pub fn lifecycle_layer_maybe(
    router: axum::routing::MethodRouter,
    lifecycle: Option<LifecycleMeta>,
) -> axum::routing::MethodRouter {
    let Some(lifecycle) = lifecycle.filter(|lc| lc.is_present()) else {
        return router;
    };
    let lifecycle = std::sync::Arc::new(lifecycle);
    router.layer(axum::middleware::from_fn(
        move |req: axum::http::Request<Body>, next: axum::middleware::Next| {
            let lifecycle = std::sync::Arc::clone(&lifecycle);
            async move {
                let mut response: Response = next.run(req).await;
                // Successor link target: the endpoint's declared successor hint
                // is used verbatim (path or name) — the macro has no global
                // version table to derive `/api/<newer>` from.
                inject_lifecycle_headers(response.headers_mut(), &lifecycle, None);
                response
            }
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lc(deprecated: bool, sunset: Option<&str>, successor: Option<&str>) -> LifecycleMeta {
        LifecycleMeta {
            deprecated,
            sunset: sunset.map(str::to_string),
            successor: successor.map(str::to_string),
        }
    }

    #[test]
    fn injects_all_three_headers_when_declared() {
        let mut headers = axum::http::HeaderMap::new();
        inject_lifecycle_headers(
            &mut headers,
            &lc(true, Some("2026-12-31"), Some("/api/v2/users")),
            None,
        );
        assert_eq!(headers.get("deprecation").unwrap(), "true");
        assert_eq!(headers.get("sunset").unwrap(), "2026-12-31");
        assert_eq!(
            headers.get(axum::http::header::LINK).unwrap(),
            "</api/v2/users>; rel=\"successor-version\""
        );
    }

    #[test]
    fn empty_lifecycle_is_a_no_op() {
        let mut headers = axum::http::HeaderMap::new();
        inject_lifecycle_headers(&mut headers, &lc(false, None, None), None);
        assert!(headers.is_empty());
    }

    #[test]
    fn sunset_only_emits_sunset_without_deprecation() {
        let mut headers = axum::http::HeaderMap::new();
        inject_lifecycle_headers(&mut headers, &lc(false, Some("2027-01-01"), None), None);
        assert!(headers.get("deprecation").is_none());
        assert_eq!(headers.get("sunset").unwrap(), "2027-01-01");
    }

    #[test]
    fn invalid_header_values_are_skipped_without_panicking() {
        let mut headers = axum::http::HeaderMap::new();
        // Non-visible-ASCII values must be dropped, not panic.
        inject_lifecycle_headers(&mut headers, &lc(true, Some("无效\n值"), None), None);
        assert_eq!(headers.get("deprecation").unwrap(), "true");
        assert!(headers.get("sunset").is_none());
    }

    /// e2e：经 `lifecycle_layer_maybe` 包裹的 MethodRouter 在响应上注入
    /// 全部三个声明头（宏只对声明了生命周期的路由套层）。
    #[tokio::test]
    async fn lifecycle_layer_stamps_headers_on_routed_responses() {
        use axum::routing::get;
        use tower::ServiceExt;

        let router = lifecycle_layer_maybe(
            get(|| async { "legacy" }),
            Some(lc(true, Some("2026-12-31"), Some("/api/v2/legacy"))),
        );
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/legacy")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.headers().get("deprecation").unwrap(), "true");
        assert_eq!(response.headers().get("sunset").unwrap(), "2026-12-31");
        assert_eq!(
            response.headers().get(axum::http::header::LINK).unwrap(),
            "</api/v2/legacy>; rel=\"successor-version\""
        );
    }

    /// e2e：`None`（未注解路由）返回原 router——响应不得携带任何生命
    /// 周期头（零开销契约）。
    #[tokio::test]
    async fn lifecycle_layer_none_leaves_router_untouched() {
        use axum::routing::get;
        use tower::ServiceExt;

        let router = lifecycle_layer_maybe(get(|| async { "fresh" }), None);
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/fresh")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(response.headers().get("deprecation").is_none());
        assert!(response.headers().get("sunset").is_none());
    }
}
