// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Built-in health probes.
//!
//! `build_with_config` auto-mounts `/healthz` (liveness) and `/readyz`
//! (readiness) **after** the auth/rate-limit/security layers, so probes
//! bypass authentication by construction.
//!
//! # Readiness data sources
//!
//! - Built-in: no checks registered → `/readyz` reports ready immediately.
//! - Custom checks: register `ReadinessCheck` implementations via
//!   `register_readiness_check`; any failing check flips `/readyz` to 503.
//! - trait-kit data source: with the `kit` feature, `KitHealthSource`
//!   adapts an `AsyncKit<AsyncReady>` health report (trait-kit health
//!   aggregation) into the `/readyz` payload via `register_health_source`.
//!
//! # Example
//!
//! ```ignore
//! let config = SdForgeConfig::default();
//! let router = sdforge::http::build_with_config(&config)?;
//! // GET /healthz -> 200 {"status":"healthy",...}
//! // GET /readyz  -> 200 {"status":"ready","checks":[...]} or 503
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock, RwLock};
use std::task::{Context, Poll};

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Outcome of a single readiness check.
#[derive(Debug, Clone)]
pub struct CheckOutcome {
    /// Check name (module or dependency identifier).
    pub name: String,
    /// Whether this check passed.
    pub healthy: bool,
    /// Optional structured payload the renderer may surface or drop
    /// (canonical keys: `latency_ms`, `error`; kit aggregates embed their
    /// report object).
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
/// flags, etc., and register via `register_readiness_check`.
///
/// # Blocking probes
///
/// Sync checks run **inline** inside the (possibly async) aggregation
/// context: a blocking probe (synchronous DB/HTTP ping) occupies a tokio
/// worker thread for its full duration. Prefer [`AsyncReadinessCheck`] for
/// dependency probes that block; keep this port for cheap, non-blocking
/// flags.
pub trait ReadinessCheck: Send + Sync {
    /// Check name surfaced in the `/readyz` payload.
    fn name(&self) -> &str;
    /// Run the check.
    fn check(&self) -> CheckOutcome;
}

/// A readiness check port that awaits I/O: named, asynchronous dependency
/// probe, registered via [`register_async_readiness_check`].
///
/// A panicking check is reported as an unhealthy outcome ("check panicked"),
/// mirroring the synchronous `catch_unwind` policy of [`ReadinessCheck`].
///
/// # Evolution note (dual registration surface)
///
/// The sync and async registration pools are a transitional shape:
/// `/readyz` aggregates both whenever any async check is registered, so
/// existing sync registrars keep their exact current behavior. Direction:
/// async absorbs sync — sync checks are the degenerate never-awaiting case —
/// and the dual surface dissolves into one async pool without any consumer
/// change.
pub trait AsyncReadinessCheck: Send + Sync {
    /// Check name surfaced in the `/readyz` payload.
    fn name(&self) -> &str;
    /// Run the check.
    fn check(&self) -> Pin<Box<dyn Future<Output = CheckOutcome> + Send + '_>>;
}

/// Health data source port for kit-style aggregates.
///
/// The JSON payload mirrors trait-kit's `HealthAggregate` shape:
/// `{"status":"healthy","healthy":true,"modules":[...]}`. When a source is
/// registered via `register_health_source`, `/readyz` folds its overall
/// status into the readiness decision and embeds the payload under `source`.
pub trait HealthDataSource: Send + Sync {
    /// Aggregate health snapshot as a JSON string.
    fn health_json(&self) -> String;
}

/// Render the readiness aggregate into the `/readyz` response.
///
/// The status code and the body envelope are decided **entirely** by the
/// renderer: the library only folds registered checks (and the kit health
/// source, when present) into `(all_healthy, checks)` and hands them over.
/// A consumer may therefore serve `/readyz` as always-200 with its own
/// envelope, or replicate the default shape — both are legitimate.
///
/// Install a renderer via [`register_readiness_renderer`]; with none
/// registered, [`DefaultReadinessRenderer`] serves the historical envelope.
pub trait ReadinessRenderer: Send + Sync {
    /// Render `(all_healthy, checks)` into the `/readyz` response.
    fn render(&self, all_healthy: bool, checks: Vec<CheckOutcome>) -> Response;
}

/// Default `/readyz` renderer: the historical envelope, extracted verbatim.
///
/// 200 + `{"status":"ready","checks":[...]}` when every check passes,
/// 503 + `{"status":"unavailable","checks":[...]}` otherwise; each check
/// contributes `name`/`healthy`/`details`.
pub struct DefaultReadinessRenderer;

impl ReadinessRenderer for DefaultReadinessRenderer {
    fn render(&self, all_healthy: bool, checks: Vec<CheckOutcome>) -> Response {
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
static ASYNC_READINESS_CHECKS: OnceLock<RwLock<Vec<Arc<dyn AsyncReadinessCheck>>>> =
    OnceLock::new();
static HEALTH_SOURCE: OnceLock<RwLock<Option<Arc<dyn HealthDataSource>>>> = OnceLock::new();
static READINESS_RENDERER: OnceLock<RwLock<Option<Arc<dyn ReadinessRenderer>>>> = OnceLock::new();

fn readiness_checks() -> &'static RwLock<Vec<Arc<dyn ReadinessCheck>>> {
    READINESS_CHECKS.get_or_init(|| RwLock::new(Vec::new()))
}

fn async_readiness_checks() -> &'static RwLock<Vec<Arc<dyn AsyncReadinessCheck>>> {
    ASYNC_READINESS_CHECKS.get_or_init(|| RwLock::new(Vec::new()))
}

fn health_source() -> &'static RwLock<Option<Arc<dyn HealthDataSource>>> {
    HEALTH_SOURCE.get_or_init(|| RwLock::new(None))
}

fn readiness_renderer_slot() -> &'static RwLock<Option<Arc<dyn ReadinessRenderer>>> {
    READINESS_RENDERER.get_or_init(|| RwLock::new(None))
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

/// Register an asynchronous readiness check (appended; order = registration
/// order). Its outcome joins the `/readyz` aggregate alongside any sync
/// checks.
///
/// Register probes during application startup: the sync/async aggregation
/// path is chosen per request, so registering or clearing at runtime while
/// requests are in flight is not a stable contract (an in-flight request may
/// take either path).
pub fn register_async_readiness_check(check: Arc<dyn AsyncReadinessCheck>) {
    if let Ok(mut guard) = async_readiness_checks().write() {
        guard.push(check);
    }
}

/// Convenience: register an asynchronous readiness check from a closure.
pub fn register_async_readiness_check_fn<F, Fut>(name: impl Into<String>, check: F)
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = CheckOutcome> + Send + 'static,
{
    struct FnAsyncCheck<F> {
        name: String,
        check: F,
    }
    impl<F, Fut> AsyncReadinessCheck for FnAsyncCheck<F>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = CheckOutcome> + Send + 'static,
    {
        fn name(&self) -> &str {
            &self.name
        }
        fn check(&self) -> Pin<Box<dyn Future<Output = CheckOutcome> + Send + '_>> {
            Box::pin((self.check)())
        }
    }
    register_async_readiness_check(Arc::new(FnAsyncCheck {
        name: name.into(),
        check,
    }));
}

/// Remove all registered asynchronous readiness checks (mainly for tests).
pub fn clear_async_readiness_checks() {
    if let Ok(mut guard) = async_readiness_checks().write() {
        guard.clear();
    }
}

/// True when at least one async readiness check is registered — the signal
/// for `readyz_handler` to aggregate via the async path.
fn has_async_readiness_checks() -> bool {
    match async_readiness_checks().read() {
        Ok(guard) => !guard.is_empty(),
        Err(_) => {
            log::warn!("async readiness checks lock poisoned; falling back to sync path");
            false
        }
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

/// Set the `/readyz` renderer (replaces any previous one).
///
/// With no renderer registered, [`DefaultReadinessRenderer`] serves the
/// historical envelope.
pub fn register_readiness_renderer(renderer: Arc<dyn ReadinessRenderer>) {
    if let Ok(mut guard) = readiness_renderer_slot().write() {
        *guard = Some(renderer);
    }
}

/// Remove the registered readiness renderer, restoring the default envelope
/// (mainly for tests).
pub fn clear_readiness_renderer() {
    if let Ok(mut guard) = readiness_renderer_slot().write() {
        *guard = None;
    }
}

/// The active readiness renderer: the registered one, or the default
/// (cached — `/readyz` is polled frequently and the default is immutable).
fn active_readiness_renderer() -> Arc<dyn ReadinessRenderer> {
    static DEFAULT: OnceLock<Arc<dyn ReadinessRenderer>> = OnceLock::new();
    match readiness_renderer_slot().read() {
        Ok(guard) => guard.clone().unwrap_or_else(|| {
            DEFAULT
                .get_or_init(|| Arc::new(DefaultReadinessRenderer))
                .clone()
        }),
        Err(_) => {
            log::warn!("readiness renderer lock poisoned; serving default envelope");
            DEFAULT
                .get_or_init(|| Arc::new(DefaultReadinessRenderer))
                .clone()
        }
    }
}

/// Run all readiness checks and fold in the health data source.
///
/// Registered checks are cloned out of the registry under a short-lived read
/// lock so user check code never runs while holding it — a slow or blocked
/// check cannot stall `register_readiness_check` / `clear_readiness_checks`.
pub fn run_readiness_checks() -> (bool, Vec<CheckOutcome>) {
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
            .unwrap_or_else(|_| {
                CheckOutcome::unhealthy(check.name(), "check panicked".to_string())
            });
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
        let payload =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| source.health_json()))
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

/// Wrap a user probe: a panic in `check()` construction or during future
/// `poll` becomes `unhealthy("check panicked")`, aligned with the sync
/// path's `catch_unwind` policy. The abandoned future is dropped once its
/// panic is caught.
///
/// Guarding every user-code path here means the JoinSet task body cannot
/// panic on behalf of a check: construction failure is named immediately
/// and `join_next` yields `Ok` for every spawned probe, so a check result
/// can never silently vanish from the `/readyz` aggregate.
struct CatchProbePanic<'a> {
    name: String,
    check: &'a dyn AsyncReadinessCheck,
    fut: Option<Pin<Box<dyn Future<Output = CheckOutcome> + Send + 'a>>>,
}

impl<'a> CatchProbePanic<'a> {
    fn new(name: String, check: &'a dyn AsyncReadinessCheck) -> Self {
        Self {
            name,
            check,
            fut: None,
        }
    }
}

impl<'a> Future for CatchProbePanic<'a> {
    type Output = CheckOutcome;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.fut.is_none() {
            // 构造段：惰性执行一次，panic 即具名 unhealthy。
            let check: &'a dyn AsyncReadinessCheck = this.check;
            this.fut =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check.check())).ok();
            let Some(fut) = this.fut.as_mut() else {
                return Poll::Ready(CheckOutcome::unhealthy(
                    this.name.clone(),
                    "check panicked".to_string(),
                ));
            };
            // 已构造：本次 poll 直接推进,避免空转一轮。
            return poll_guarded(&mut this.name, fut.as_mut(), cx);
        }
        let fut = this.fut.as_mut().expect("checked above");
        poll_guarded(&mut this.name, fut.as_mut(), cx)
    }
}

/// poll 一次,panic 转具名 unhealthy;被捕获后原 future 即被放弃。
///
/// name 借用传入:正常 Pending/Ready 路径零分配,仅 panic 分支经
/// `mem::take` 取走所有权供 outcome 使用(此后 wrapper 不再被 poll)。
fn poll_guarded(
    name: &mut String,
    fut: Pin<&mut (dyn Future<Output = CheckOutcome> + Send)>,
    cx: &mut Context<'_>,
) -> Poll<CheckOutcome> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| fut.poll(cx))) {
        Ok(Poll::Ready(outcome)) => Poll::Ready(outcome),
        Ok(Poll::Pending) => Poll::Pending,
        Err(_) => Poll::Ready(CheckOutcome::unhealthy(
            std::mem::take(name),
            "check panicked".to_string(),
        )),
    }
}

/// Asynchronous counterpart of [`run_readiness_checks`]: sync checks run
/// exactly as in the sync path, async checks run concurrently on a
/// `tokio::task::JoinSet`, and the kit health source folds in last.
///
/// Every user-code path is guarded so a check's outcome can never silently
/// disappear: `name()` is pre-fetched under `catch_unwind` before spawn (a
/// panicking name yields an unhealthy outcome without entering the pool),
/// the future's construction is guarded in-task, and polls are guarded by
/// [`CatchProbePanic`]. Cancellation propagates correctly — there is exactly
/// one task per check, so dropping the aggregate future aborts slow probes
/// instead of leaving them running detached. Outcomes are returned in
/// registration order (sync first, then async), not completion order.
///
/// The sync aggregate body is deliberately not refactored into this path —
/// its source and output stay untouched.
pub async fn run_readiness_checks_async() -> (bool, Vec<CheckOutcome>) {
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
            .unwrap_or_else(|_| {
                CheckOutcome::unhealthy(check.name(), "check panicked".to_string())
            });
        if !outcome.healthy {
            all_healthy = false;
        }
        outcomes.push(outcome);
    }

    let async_checks: Vec<Arc<dyn AsyncReadinessCheck>> = match async_readiness_checks().read() {
        Ok(guard) => guard.clone(),
        Err(_) => {
            log::warn!("async readiness checks lock poisoned; running with an empty check set");
            Vec::new()
        }
    };
    if !async_checks.is_empty() {
        let mut set = tokio::task::JoinSet::new();
        let mut async_outcomes: Vec<(usize, CheckOutcome)> = Vec::with_capacity(async_checks.len());
        // name() 属用户代码：spawn 前以 catch_unwind 预取，panic 的检查直接
        // 产出 unhealthy 且不进入并发池——用户代码的任何 panic 都不得让
        // 探针结果从 /readyz 结果集中消失。
        let names: Vec<Option<String>> = async_checks
            .iter()
            .map(|check| {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check.name().to_string()))
                    .ok()
            })
            .collect();
        for (idx, name) in names.iter().enumerate() {
            if name.is_none() {
                all_healthy = false;
                async_outcomes.push((
                    idx,
                    CheckOutcome::unhealthy("<unnamed async check>", "check panicked".to_string()),
                ));
            }
        }
        for (idx, check) in async_checks.into_iter().enumerate() {
            let Some(name) = names[idx].clone() else {
                continue;
            };
            set.spawn(async move {
                // 构造段与 poll 段的 panic 均由 CatchProbePanic 具名兜底为
                // unhealthy——任务体不含未受保护的用户代码,join_next 恒返
                // 回 Ok;单层 spawn 保证聚合 future 被 drop 时慢探针任务随
                // 之被 abort,不会脱离取消范围继续打向下游依赖。
                let probe = CatchProbePanic::new(name, check.as_ref());
                (idx, probe.await)
            });
        }
        while let Some(joined) = set.join_next().await {
            match joined {
                Ok((idx, outcome)) => {
                    if !outcome.healthy {
                        all_healthy = false;
                    }
                    async_outcomes.push((idx, outcome));
                }
                // 任务体的 poll 段已受保护；此臂覆盖 check() 同步构造段
                // panic 与取消等残余路径，兜底显性化，不让任何检查静默
                // 消失（无法归名，以占位名呈现并排至结果末尾）。
                Err(join_err) => {
                    log::warn!("async readiness check task did not complete: {join_err}");
                    all_healthy = false;
                    async_outcomes.push((
                        usize::MAX,
                        CheckOutcome::unhealthy(
                            "<incomplete async check>",
                            "check task did not complete".to_string(),
                        ),
                    ));
                }
            }
        }
        async_outcomes.sort_by_key(|(idx, _)| *idx);
        outcomes.extend(async_outcomes.into_iter().map(|(_, outcome)| outcome));
    }

    // Kit-style aggregate: mirrors the sync path — the source handle is
    // cloned out of the lock before its `health_json()` runs.
    let source = match health_source().read() {
        Ok(guard) => guard.clone(),
        Err(_) => {
            log::warn!("health source lock poisoned; skipping the kit aggregate");
            None
        }
    };
    if let Some(source) = source {
        let payload =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| source.health_json()))
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

/// `GET /readyz` — readiness probe.
///
/// Aggregation path: when at least one async readiness check is registered,
/// the aggregate runs through [`run_readiness_checks_async`] (which also
/// folds the sync checks); otherwise the sync path is taken unchanged. The
/// registration test and the aggregate re-read the registry, so runtime
/// register/clear racing a request can take either path — register probes at
/// startup. The response then comes from the active [`ReadinessRenderer`];
/// with none registered, [`DefaultReadinessRenderer`] serves 200/`ready`
/// when all checks pass and 503/`unavailable` otherwise.
pub async fn readyz_handler() -> Response {
    let (all_healthy, checks) = if has_async_readiness_checks() {
        run_readiness_checks_async().await
    } else {
        run_readiness_checks()
    };
    active_readiness_renderer().render(all_healthy, checks)
}

/// Mount `/healthz` and `/readyz` on `router`, skipping any path already
/// claimed by a user route (avoids axum duplicate-route panics).
pub fn mount_probes(router: axum::Router) -> axum::Router {
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

    /// 恒 200 消费方 renderer：忽略聚合结果，只回自定义包络。
    struct AlwaysOkRenderer;

    impl ReadinessRenderer for AlwaysOkRenderer {
        fn render(&self, _all_healthy: bool, _checks: Vec<CheckOutcome>) -> Response {
            (StatusCode::OK, "ok").into_response()
        }
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
            serde_json::json!({"status": "unhealthy", "healthy": false, "modules": []}).to_string()
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

    #[tokio::test]
    #[serial_test::serial]
    async fn default_readyz_envelope_is_byte_exact() {
        clear_readiness_checks();
        clear_health_source();

        // 无检查：200 + 空 checks 包络（键序 = serde_json Map 字典序）。
        let resp = get(probe_router(), "/readyz").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8(body.to_vec()).unwrap(),
            "{\"checks\":[],\"status\":\"ready\"}"
        );

        // 失败检查：503 + error 详情包络。
        register_readiness_check_fn("dep-b", || {
            CheckOutcome::unhealthy("dep-b", "connection refused")
        });
        let resp = get(probe_router(), "/readyz").await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8(body.to_vec()).unwrap(),
            "{\"checks\":[{\"details\":{\"error\":\"connection refused\"},\"healthy\":false,\"name\":\"dep-b\"}],\"status\":\"unavailable\"}"
        );
        clear_readiness_checks();
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

    #[tokio::test]
    #[serial_test::serial]
    async fn custom_renderer_replaces_envelope_and_clear_restores_default() {
        clear_readiness_checks();
        clear_health_source();
        clear_readiness_renderer();

        // 恒 200 消费方 renderer：状态码/包络完全由 renderer 决定。
        register_readiness_renderer(Arc::new(AlwaysOkRenderer));
        let resp = get(probe_router(), "/readyz").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(String::from_utf8(body.to_vec()).unwrap(), "ok");

        // 清除后恢复默认包络（与快照一致）。
        clear_readiness_renderer();
        let resp = get(probe_router(), "/readyz").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8(body.to_vec()).unwrap(),
            "{\"checks\":[],\"status\":\"ready\"}"
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn async_panicking_check_is_reported_unhealthy() {
        clear_readiness_checks();
        clear_health_source();
        clear_async_readiness_checks();
        register_async_readiness_check_fn("boom-async", || async {
            panic!("exploding async check");
        });
        let (all_healthy, checks) = run_readiness_checks_async().await;
        assert!(!all_healthy);
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].name, "boom-async");
        assert!(!checks[0].healthy);
        assert_eq!(
            checks[0].details.as_ref().unwrap()["error"],
            "check panicked"
        );
        clear_async_readiness_checks();
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn sync_and_async_checks_aggregate_together() {
        clear_readiness_checks();
        clear_health_source();
        clear_async_readiness_checks();
        register_readiness_check_fn("dep-sync", || CheckOutcome::healthy("dep-sync"));
        register_async_readiness_check_fn("dep-async-ok", || async {
            CheckOutcome::healthy("dep-async-ok")
        });
        register_async_readiness_check_fn("dep-async-bad", || async {
            CheckOutcome::unhealthy("dep-async-bad", "connection refused")
        });

        // 聚合序确定性：同步注册序在前，异步按注册序（非完成序）。
        let (all_healthy, checks) = run_readiness_checks_async().await;
        assert!(!all_healthy);
        assert_eq!(checks.len(), 3);
        assert_eq!(checks[0].name, "dep-sync");
        assert_eq!(checks[1].name, "dep-async-ok");
        assert_eq!(checks[2].name, "dep-async-bad");

        // handler 检测到 async 注册走 async 版：/readyz 翻 503 且含全部三项。
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
        assert_eq!(names, vec!["dep-sync", "dep-async-ok", "dep-async-bad"]);
        clear_async_readiness_checks();
        clear_readiness_checks();
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn async_aggregate_matches_sync_when_no_async_registered() {
        clear_readiness_checks();
        clear_health_source();
        clear_async_readiness_checks();
        register_readiness_check_fn("only-sync", || CheckOutcome::healthy("only-sync"));
        let (healthy, checks) = run_readiness_checks_async().await;
        assert!(healthy);
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].name, "only-sync");
        clear_readiness_checks();
    }

    /// name() panic 的检查同样必须出现在结果集中并翻红聚合：任何用户代码
    /// 路径的 panic 都不得让探针结果消失（否则 /readyz 可对上游假性 ready）。
    struct PanickingName;

    impl AsyncReadinessCheck for PanickingName {
        fn name(&self) -> &str {
            panic!("name exploded");
        }
        fn check(&self) -> Pin<Box<dyn Future<Output = CheckOutcome> + Send + '_>> {
            Box::pin(async { CheckOutcome::healthy("never-reached") })
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn async_panicking_name_is_reported_unhealthy() {
        clear_readiness_checks();
        clear_health_source();
        clear_async_readiness_checks();
        register_async_readiness_check_fn("healthy-async", || async {
            CheckOutcome::healthy("healthy-async")
        });
        register_async_readiness_check(Arc::new(PanickingName));

        let (all_healthy, checks) = run_readiness_checks_async().await;
        assert!(!all_healthy, "panicking name() must flip the aggregate red");
        assert_eq!(
            checks.len(),
            2,
            "every registered check must surface an outcome"
        );
        assert_eq!(checks[0].name, "healthy-async");
        assert!(checks[0].healthy);
        assert_eq!(checks[1].name, "<unnamed async check>");
        assert!(!checks[1].healthy);
        assert_eq!(
            checks[1].details.as_ref().unwrap()["error"],
            "check panicked"
        );
        clear_async_readiness_checks();
    }

    #[test]
    fn route_path_taken_detects_registered_path() {
        // "/healthz" is mounted by probe_router above but that is a plain
        // Router (not inventory), so inventory scan sees no match for a
        // deliberately unused path.
        assert!(!crate::http::route_path_taken(
            "/__definitely_not_registered__"
        ));
    }
}

// =============================================================================
// trait-kit integration (`kit` feature): AsyncKit health report adapter.
// =============================================================================
/// AsyncKit 健康报告适配：将 trait-kit `AsyncKit<AsyncReady>` 的模块健康
/// 状态映射为 `HealthDataSource`，并入聚合健康报告。
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
