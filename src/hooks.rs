// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Processor pre/post hook pipeline.
//!
//! Middleware-style hooks around every HTTP request handled by
//! `build_with_config`: [`install_hooks`] registers one process-global
//! [`RequestHooks`] implementation; the pipeline invokes `before` when the
//! request enters and `after` when the response leaves (with the final
//! status). Hooks are protocol-agnostic by construction and composed with the
//! request context for correlation.
//!
//! Hook panics are isolated: a panicking `before`/`after` never breaks the
//! request pipeline.

use std::sync::{Arc, OnceLock};

/// Snapshot of request facts handed to hooks.
#[derive(Debug, Clone)]
pub struct RequestInfo {
    /// HTTP method.
    pub method: String,
    /// Request path (concrete path, not route template).
    pub path: String,
    /// Request id from the ambient context, when available.
    pub request_id: Option<String>,
}

impl RequestInfo {
    /// Capture facts from an incoming request.
    pub fn capture(req: &axum::http::Request<axum::body::Body>) -> Self {
        #[cfg(feature = "context")]
        let request_id = crate::context::current().map(|c| c.request_id().to_string());
        #[cfg(not(feature = "context"))]
        let request_id = None;
        Self {
            method: req.method().to_string(),
            path: req.uri().path().to_string(),
            request_id,
        }
    }

    /// Capture without a real request (tests/custom pipelines).
    pub fn synthetic(method: &str, path: &str) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            request_id: None,
        }
    }
}

/// Pre/post request hooks.
pub trait RequestHooks: Send + Sync {
    /// Called before the request is dispatched.
    fn before(&self, info: &RequestInfo);
    /// Called after the handler chain produced a response.
    fn after(&self, info: &RequestInfo, status: u16);
}

static GLOBAL_HOOKS: OnceLock<Arc<dyn RequestHooks>> = OnceLock::new();

/// Install the process-global hook pipeline (idempotent: first install wins;
/// returns `false` when hooks were already installed).
pub fn install_hooks(hooks: Arc<dyn RequestHooks>) -> bool {
    GLOBAL_HOOKS.set(hooks).is_ok()
}

/// The installed hooks, if any.
pub fn installed() -> Option<Arc<dyn RequestHooks>> {
    GLOBAL_HOOKS.get().cloned()
}

/// The hook pipeline middleware (installed by `build_with_config` when hooks
/// are registered).
pub async fn hooks_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let Some(hooks) = installed() else {
        return next.run(req).await;
    };
    let info = RequestInfo::capture(&req);
    run_before(&*hooks, &info);
    let response = next.run(req).await;
    let status = response.status().as_u16();
    run_after(&*hooks, &info, status);
    response
}

fn run_before(hooks: &dyn RequestHooks, info: &RequestInfo) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hooks.before(info)));
}

fn run_after(hooks: &dyn RequestHooks, info: &RequestInfo, status: u16) {
    let _ =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hooks.after(info, status)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tower::ServiceExt;

    struct CountingHooks {
        before: AtomicUsize,
        after: AtomicUsize,
        last_status: std::sync::Mutex<Option<u16>>,
    }

    impl RequestHooks for CountingHooks {
        fn before(&self, _info: &RequestInfo) {
            self.before.fetch_add(1, Ordering::SeqCst);
        }
        fn after(&self, _info: &RequestInfo, status: u16) {
            self.after.fetch_add(1, Ordering::SeqCst);
            *self.last_status.lock().unwrap() = Some(status);
        }
    }

    struct PanickingBefore;
    impl RequestHooks for PanickingBefore {
        fn before(&self, _info: &RequestInfo) {
            panic!("hook explosion");
        }
        fn after(&self, _info: &RequestInfo, _status: u16) {}
    }

    fn test_app() -> axum::Router {
        axum::Router::new()
            .route("/hooked", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(hooks_middleware))
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn hooks_fire_before_and_after() {
        let hooks = Arc::new(CountingHooks {
            before: AtomicUsize::new(0),
            after: AtomicUsize::new(0),
            last_status: std::sync::Mutex::new(None),
        });
        assert!(install_hooks(hooks.clone()));
        assert!(!install_hooks(hooks.clone()), "second install is rejected");

        let resp = test_app()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/hooked")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        assert_eq!(hooks.before.load(Ordering::SeqCst), 1);
        assert_eq!(hooks.after.load(Ordering::SeqCst), 1);
        assert_eq!(*hooks.last_status.lock().unwrap(), Some(200));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn panicking_before_hook_does_not_break_pipeline() {
        install_hooks(Arc::new(PanickingBefore));
        let resp = test_app()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/hooked")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "request must complete despite hook panic");
    }

    #[test]
    fn request_info_capture_fields() {
        let info = RequestInfo::synthetic("GET", "/x");
        assert_eq!(info.method, "GET");
        assert_eq!(info.path, "/x");
        assert!(info.request_id.is_none());
    }
}
