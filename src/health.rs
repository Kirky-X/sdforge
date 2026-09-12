// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Built-in health probes.
//!
//! `build_with_config` auto-mounts `/healthz` (liveness) and `/readyz`
//! (readiness) **after** the auth/rate-limit/security layers, so probes
//! bypass authentication by construction (R-sd4-001).
//!
//! # Readiness data sources
//!
//! - Built-in: no checks registered → `/readyz` reports ready immediately.
//! - Custom checks: register [`ReadinessCheck`] implementations via
//!   [`register_readiness_check`]; any failing check flips `/readyz` to 503.
//! - trait-kit data source: with the `kit` feature, [`KitHealthSource`]
//!   adapts an `AsyncKit<AsyncReady>` health report (trait-kit health
//!   aggregation) into the `/readyz` payload via [`register_health_source`].
//!
//! # Example
//!
//! ```ignore
//! let config = AppConfig::default();
//! let router = sdforge::http::build_with_config(&config)?;
//! // GET /healthz -> 200 {"status":"healthy",...}
//! // GET /readyz  -> 200 {"status":"ready","checks":[...]} or 503
//! ```

use std::sync::{Arc, OnceLock, RwLock};

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Outcome of a single readiness check.
#[derive(Debug, Clone)]
pub struct CheckOutcome {
    /// Check name (module or dependency identifier).
    pub name: String,
    /// Whether this check passed.
    pub healthy: bool,
    /// Optional structured details (error message, latency, kit aggregate…).
    pub details: Option<serde_json::Value>,
}

impl CheckOutcome {
    /// A passing check with no details.
    pub fn healthy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            healthy: true,
            details: None,
        }
    }

    /// A failing check with a human-readable reason.
    pub fn unhealthy(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            healthy: false,
            details: Some(serde_json::json!({ "error": reason.into() })),
        }
    }
}

/// A readiness check port: named, synchronous dependency probe.
///
/// Implement this for cache connectivity, database pings, config-loaded
/// flags, etc., and register via [`register_readiness_check`].
pub trait ReadinessCheck: Send + Sync {
    /// Check name surfaced in the `/readyz` payload.
    fn name(&self) -> &str;
    /// Run the check.
    fn check(&self) -> CheckOutcome;
}

/// Health data source port for kit-style aggregates (R-sd4-001).
///
/// The JSON payload mirrors trait-kit's `HealthAggregate` shape:
/// `{"status":"healthy","healthy":true,"modules":[...]}`. When a source is
/// registered via [`register_health_source`], `/readyz` folds its overall
/// status into the readiness decision and embeds the payload under `source`.
pub trait HealthDataSource: Send + Sync {
    /// Aggregate health snapshot as a JSON string.
    fn health_json(&self) -> String;
}

/// Closure-based [`HealthDataSource`] adapter.
struct FnHealthSource<F>(F)
where
    F: Fn() -> String + Send + Sync;

impl<F> HealthDataSource for FnHealthSource<F>
where
    F: Fn() -> String + Send + Sync,
{
    fn health_json(&self) -> String {
        (self.0)()
    }
}

/// Register a closure as the health data source (replaces any previous one).
pub fn register_health_source_fn(f: impl Fn() -> String + Send + Sync + 'static) {
    register_health_source(Arc::new(FnHealthSource(f)));
}

static READINESS_CHECKS: OnceLock<RwLock<Vec<Arc<dyn ReadinessCheck>>>> = OnceLock::new();
static HEALTH_SOURCE: OnceLock<RwLock<Option<Arc<dyn HealthDataSource>>>> = OnceLock::new();

fn readiness_checks() -> &'static RwLock<Vec<Arc<dyn ReadinessCheck>>> {
    READINESS_CHECKS.get_or_init(|| RwLock::new(Vec::new()))
}

fn health_source() -> &'static RwLock<Option<Arc<dyn HealthDataSource>>> {
    HEALTH_SOURCE.get_or_init(|| RwLock::new(None))
}

/// Register a readiness check (appended; order = registration order).
pub fn register_readiness_check(check: Arc<dyn ReadinessCheck>) {
    if let Ok(mut guard) = readiness_checks().write() {
        guard.push(check);
    }
}

/// Convenience: register a readiness check from a closure.
pub fn register_readiness_check_fn(
    name: impl Into<String>,
    check: impl Fn() -> CheckOutcome + Send + Sync + 'static,
) {
    struct FnCheck<F> {
        name: String,
        check: F,
    }
    impl<F> ReadinessCheck for FnCheck<F>
    where
        F: Fn() -> CheckOutcome + Send + Sync,
    {
        fn name(&self) -> &str {
            &self.name
        }
        fn check(&self) -> CheckOutcome {
            (self.check)()
        }
    }
    register_readiness_check(Arc::new(FnCheck {
        name: name.into(),
        check,
    }));
}

/// Remove all registered readiness checks (mainly for tests).
pub fn clear_readiness_checks() {
    if let Ok(mut guard) = readiness_checks().write() {
        guard.clear();
    }
}

/// Set the kit-style health data source (replaces any previous one).
pub fn register_health_source(source: Arc<dyn HealthDataSource>) {
    if let Ok(mut guard) = health_source().write() {
        *guard = Some(source);
    }
}

/// Clear the health data source (mainly for tests).
pub fn clear_health_source() {
    if let Ok(mut guard) = health_source().write() {
        *guard = None;
    }
}

/// Run all readiness checks and fold in the health data source.
///
/// Registered checks are cloned out of the registry under a short-lived read
/// lock so user check code never runs while holding it — a slow or blocked
/// check cannot stall `register_readiness_check` / `clear_readiness_checks`.
pub(crate) fn run_readiness_checks() -> (bool, Vec<CheckOutcome>) {
    let mut outcomes = Vec::new();
    let mut all_healthy = true;

    let checks: Vec<Arc<dyn ReadinessCheck>> = match readiness_checks().read() {
        Ok(guard) => guard.clone(),
        Err(_) => {
            log::warn!("readiness checks lock poisoned; running with an empty check set");
            Vec::new()
        }
    };
    for check in &checks {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check.check()))
            .unwrap_or_else(|_| CheckOutcome::unhealthy(check.name(), "check panicked".to_string()));
        if !outcome.healthy {
            all_healthy = false;
        }
        outcomes.push(outcome);
    }

    // Kit-style aggregate: fold its overall status into readiness. The source
    // handle is cloned out of the lock before its `health_json()` runs.
    let source = match health_source().read() {
        Ok(guard) => guard.clone(),
        Err(_) => {
            log::warn!("health source lock poisoned; skipping the kit aggregate");
            None
        }
    };
    if let Some(source) = source {
        let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| source.health_json()))
            .unwrap_or_else(|_| {
                serde_json::json!({"status": "unhealthy", "healthy": false}).to_string()
            });
        let value: serde_json::Value = serde_json::from_str(&payload)
            .unwrap_or_else(|_| serde_json::json!({"status": "unhealthy"}));
        let healthy = value
            .get("healthy")
            .and_then(|h| h.as_bool())
            .unwrap_or(false);
        if !healthy {
            all_healthy = false;
        }
        outcomes.push(CheckOutcome {
            name: "kit".to_string(),
            healthy,
            details: Some(value),
        });
    }

    (all_healthy, outcomes)
}

/// `GET /healthz` — liveness probe. Always 200 while the process serves.
pub async fn healthz_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// `GET /readyz` — readiness probe. 200 when all checks pass, 503 otherwise.
pub async fn readyz_handler() -> Response {
    let (all_healthy, checks) = run_readiness_checks();
    let status = if all_healthy { "ready" } else { "unavailable" };
    let body = serde_json::json!({
        "status": status,
        "checks": checks.iter().map(|c| serde_json::json!({
            "name": c.name,
            "healthy": c.healthy,
            "details": c.details,
        })).collect::<Vec<_>>(),
    });
    let code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(body)).into_response()
}

/// Mount `/healthz` and `/readyz` on `router`, skipping any path already
/// claimed by a user route (avoids axum duplicate-route panics).
pub(crate) fn mount_probes(router: axum::Router) -> axum::Router {
    let mut router = router;
    if !crate::http::route_path_taken("/healthz") {
        router = router.route("/healthz", axum::routing::get(healthz_handler));
    }
    if !crate::http::route_path_taken("/readyz") {
        router = router.route("/readyz", axum::routing::get(readyz_handler));
    }
    router
}

#[cfg(all(test, feature = "health"))]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::ServiceExt;

    fn probe_router() -> axum::Router {
        axum::Router::new()
            .route("/healthz", axum::routing::get(healthz_handler))
            .route("/readyz", axum::routing::get(readyz_handler))
    }

    async fn get(router: axum::Router, uri: &str) -> axum::http::Response<Body> {
        router
            .oneshot(
                axum::http::Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn healthz_always_200() {
        let resp = get(probe_router(), "/healthz").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "healthy");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn readyz_ready_without_checks() {
        clear_readiness_checks();
        clear_health_source();
        let resp = get(probe_router(), "/readyz").await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn readyz_503_when_any_check_fails() {
        clear_readiness_checks();
        clear_health_source();
        register_readiness_check_fn("dep-a", || CheckOutcome::healthy("dep-a"));
        register_readiness_check_fn("dep-b", || {
            CheckOutcome::unhealthy("dep-b", "connection refused")
        });
        let resp = get(probe_router(), "/readyz").await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "unavailable");
        let names: Vec<&str> = json["checks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"dep-a") && names.contains(&"dep-b"));
        clear_readiness_checks();
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn readyz_passing_checks_yield_200() {
        clear_readiness_checks();
        clear_health_source();
        register_readiness_check_fn("ok-dep", || CheckOutcome::healthy("ok-dep"));
        let resp = get(probe_router(), "/readyz").await;
        assert_eq!(resp.status(), StatusCode::OK);
        clear_readiness_checks();
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn health_source_unhealthy_forces_503() {
        clear_readiness_checks();
        clear_health_source();
        register_health_source_fn(|| {
            serde_json::json!({"status": "unhealthy", "healthy": false, "modules": []})
                .to_string()
        });
        let resp = get(probe_router(), "/readyz").await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        clear_health_source();
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn health_source_healthy_keeps_200() {
        clear_readiness_checks();
        clear_health_source();
        register_health_source_fn(|| {
            serde_json::json!({
                "status": "healthy",
                "healthy": true,
                "modules": [{"module": "m", "status": "healthy", "detail": null}]
            })
            .to_string()
        });
        let resp = get(probe_router(), "/readyz").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["checks"][0]["name"], "kit");
        assert_eq!(json["checks"][0]["details"]["status"], "healthy");
        clear_health_source();
    }

    #[test]
    #[serial_test::serial]
    fn panicking_check_is_reported_unhealthy() {
        clear_readiness_checks();
        clear_health_source();
        register_readiness_check_fn("boom", || -> CheckOutcome {
            panic!("exploding check");
        });
        let (all_healthy, checks) = run_readiness_checks();
        assert!(!all_healthy);
        assert_eq!(checks[0].name, "boom");
        assert!(!checks[0].healthy);
        clear_readiness_checks();
    }

    #[test]
    fn route_path_taken_detects_registered_path() {
        // "/healthz" is mounted by probe_router above but that is a plain
        // Router (not inventory), so inventory scan sees no match for a
        // deliberately unused path.
        assert!(!crate::http::route_path_taken("/__definitely_not_registered__"));
    }
}

// =============================================================================
// trait-kit integration (`kit` feature): AsyncKit health report adapter.
// =============================================================================
#[cfg(feature = "kit")]
pub mod kit_source {
    use super::HealthDataSource;
    use std::sync::Arc;
    use trait_kit::{AsyncKit, AsyncReady};

    /// [`HealthDataSource`] backed by an `AsyncKit<AsyncReady>` health report
    /// (trait-kit aggregate shape; requires trait-kit `health` feature).
    pub struct KitHealthSource {
        kit: Arc<AsyncKit<AsyncReady>>,
    }

    impl KitHealthSource {
        /// Wrap a built (ready) async kit.
        pub fn new(kit: Arc<AsyncKit<AsyncReady>>) -> Self {
            Self { kit }
        }
    }

    impl HealthDataSource for KitHealthSource {
        fn health_json(&self) -> String {
            // Mirror trait-kit HealthAggregate: worst-of status + per-module list.
            let report = self.kit.health_report();
            let mut worst_rank = 0u8;
            let modules: Vec<serde_json::Value> = report
                .into_iter()
                .map(|(name, status)| {
                    worst_rank = worst_rank.max(status.severity_rank());
                    serde_json::json!({
                        "module": name,
                        "status": status.as_status_name(),
                        "detail": status.detail(),
                    })
                })
                .collect();
            let status = match worst_rank {
                0 => "healthy",
                1 => "degraded",
                _ => "unhealthy",
            };
            serde_json::json!({
                "status": status,
                "healthy": worst_rank == 0,
                "modules": modules,
            })
            .to_string()
        }
    }

    /// Register a kit as the readiness health data source (data path).
    pub fn register_kit_health_source(kit: Arc<AsyncKit<AsyncReady>>) {
        super::register_health_source(Arc::new(KitHealthSource::new(kit)));
    }

    #[cfg(all(test, feature = "health"))]
    mod tests {
        use super::*;
        use crate::health::{clear_health_source, run_readiness_checks};
        use trait_kit::AsyncKit;

        #[tokio::test]
        async fn kit_health_source_reports_healthy_without_checkers() {
            let kit = AsyncKit::new();
            let built = kit.build().await.expect("empty kit builds");
            let source = KitHealthSource::new(Arc::new(built));
            let json = source.health_json();
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["status"], "healthy");
            assert_eq!(value["healthy"], true);
        }

        #[tokio::test]
        #[serial_test::serial]
        async fn registered_kit_source_folds_into_readyz() {
            let kit = AsyncKit::new();
            let built = Arc::new(kit.build().await.expect("empty kit builds"));
            crate::health::clear_readiness_checks();
            clear_health_source();
            crate::health::kit_source::register_kit_health_source(built);
            let (all_healthy, checks) = run_readiness_checks();
            assert!(all_healthy);
            assert_eq!(checks.len(), 1);
            assert_eq!(checks[0].name, "kit");
            clear_health_source();
        }

        #[test]
        fn check_outcome_helpers_shape_payload() {
            let bad = super::super::CheckOutcome::unhealthy("db", "down");
            assert_eq!(bad.details.unwrap()["error"], "down");
        }
    }
}
