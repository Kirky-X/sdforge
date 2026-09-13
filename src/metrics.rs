// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Prometheus metrics.
//!
//! Lightweight, self-rendered Prometheus text format (no `prometheus` crate
//! dependency — keeps the trimming philosophy). With the `metrics` feature:
//!
//! - `build_with_config` auto-installs a request-recording middleware
//!   (request count, latency histogram, status distribution per route
//!   template via `MatchedPath` — no cardinality explosion from concrete
//!   paths), and
//! - mounts `GET /metrics` **after** the auth layer (bypasses authentication,
//!   same mechanism as the built-in probes), unless a user route claims it.
//!
//! Exported series (RED method):
//!
//! - `sdforge_http_requests_total{route,method,status}` — counter
//! - `sdforge_http_request_duration_seconds{route,method}` — histogram
//!   with fixed buckets (0.001s .. 10s)

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use axum::response::IntoResponse;

/// Latency histogram buckets in seconds (Prometheus convention: cumulative).
pub const LATENCY_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Aggregated series for one `(route, method, status-class)` key.
#[derive(Debug, Clone)]
struct RequestSeries {
    /// Total request count.
    count: u64,
    /// Cumulative latency seconds.
    duration_sum: f64,
    /// Cumulative bucket counts (parallel to [`LATENCY_BUCKETS`]).
    buckets: Vec<u64>,
}

impl Default for RequestSeries {
    fn default() -> Self {
        Self {
            count: 0,
            duration_sum: 0.0,
            buckets: vec![0; LATENCY_BUCKETS.len()],
        }
    }
}

impl RequestSeries {
    fn observe(&mut self, duration_secs: f64) {
        self.count += 1;
        self.duration_sum += duration_secs;
        // Increment only the FIRST bucket with le >= duration; `render`
        // accumulates per-bucket counts into Prometheus's cumulative form.
        // Durations above the last bucket land in the `+Inf` bucket only.
        if let Some(i) = LATENCY_BUCKETS.iter().position(|le| duration_secs <= *le) {
            self.buckets[i] += 1;
        }
    }
}

/// Metrics registry: `(route, method)` → per-status counts + latency histogram.
#[derive(Debug, Default)]
pub struct MetricsRegistry {
    /// Route template + method → series (latency histogram keyed here;
    /// status counts split out per status code).
    series: Mutex<BTreeMap<(String, String), RouteSeries>>,
}

#[derive(Debug, Default, Clone)]
struct RouteSeries {
    status_counts: BTreeMap<u16, u64>,
    latency: RequestSeries,
}

impl MetricsRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one request observation.
    pub fn record(&self, route: &str, method: &str, status: u16, duration_secs: f64) {
        let key = (route.to_string(), method.to_string());
        if let Ok(mut guard) = self.series.lock() {
            let entry = guard.entry(key).or_insert_with(RouteSeries::default);
            *entry.status_counts.entry(status).or_insert(0) += 1;
            entry.latency.observe(duration_secs);
        }
    }

    /// Reset all series (mainly for tests).
    pub fn reset(&self) {
        if let Ok(mut guard) = self.series.lock() {
            guard.clear();
        }
    }

    /// Number of distinct `(route, method)` series.
    pub fn route_count(&self) -> usize {
        self.series.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// Render the registry in Prometheus text exposition format.
    ///
    /// Deterministic: series are emitted sorted by (route, method, status).
    pub fn render(&self) -> String {
        let mut out = String::new();
        let guard = match self.series.lock() {
            Ok(g) => g,
            Err(_) => return out,
        };
        if guard.is_empty() {
            return out;
        }

        out.push_str("# HELP sdforge_http_requests_total Total HTTP requests.\n");
        out.push_str("# TYPE sdforge_http_requests_total counter\n");
        for ((route, method), series) in guard.iter() {
            for (status, count) in series.status_counts.iter() {
                out.push_str(&format!(
                    "sdforge_http_requests_total{{route=\"{route}\",method=\"{method}\",status=\"{status}\"}} {count}\n"
                ));
            }
        }

        out.push_str("# HELP sdforge_http_request_duration_seconds HTTP request latency.\n");
        out.push_str("# TYPE sdforge_http_request_duration_seconds histogram\n");
        for ((route, method), series) in guard.iter() {
            let mut cumulative = 0u64;
            for (i, le) in LATENCY_BUCKETS.iter().enumerate() {
                cumulative += series.latency.buckets[i];
                out.push_str(&format!(
                    "sdforge_http_request_duration_seconds_bucket{{route=\"{route}\",method=\"{method}\",le=\"{le}\"}} {cumulative}\n"
                ));
            }
            out.push_str(&format!(
                "sdforge_http_request_duration_seconds_bucket{{route=\"{route}\",method=\"{method}\",le=\"+Inf\"}} {}\n",
                series.latency.count
            ));
            out.push_str(&format!(
                "sdforge_http_request_duration_seconds_sum{{route=\"{route}\",method=\"{method}\"}} {}\n",
                series.latency.duration_sum
            ));
            out.push_str(&format!(
                "sdforge_http_request_duration_seconds_count{{route=\"{route}\",method=\"{method}\"}} {}\n",
                series.latency.count
            ));
        }
        out
    }
}

static GLOBAL_REGISTRY: OnceLock<Arc<MetricsRegistry>> = OnceLock::new();

/// The process-global registry used by `build_with_config`'s middleware.
pub fn global_registry() -> Arc<MetricsRegistry> {
    GLOBAL_REGISTRY
        .get_or_init(|| Arc::new(MetricsRegistry::new()))
        .clone()
}

/// Record one request into the global registry.
pub fn record_request(route: &str, method: &str, status: u16, duration_secs: f64) {
    global_registry().record(route, method, status, duration_secs);
}

/// `GET /metrics` — render the global registry as Prometheus text.
pub async fn metrics_handler() -> impl IntoResponse {
    let body = global_registry().render();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

/// Mount `GET /metrics` unless a user route already claims the path.
pub(crate) fn mount_metrics(router: axum::Router) -> axum::Router {
    if !crate::http::route_path_taken("/metrics") {
        router.route("/metrics", axum::routing::get(metrics_handler))
    } else {
        router
    }
}

/// Request-recording middleware factory (installed by `build_with_config`).
///
/// Route label uses the matched route template ([`axum::extract::MatchedPath`])
/// so wildcard/path-param routes do not explode cardinality; unmatched
/// requests are attributed to `route="unmatched"`. Internal endpoints
/// (`/metrics`) are skipped.
pub(crate) async fn record_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().to_string();
    let route = req
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| "unmatched".to_string());
    let start = Instant::now();
    let response = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();
    let status = response.status().as_u16();
    if route != "/metrics" {
        record_request(&route, &method, status, elapsed);
    }
    response
}

#[cfg(all(test, feature = "metrics"))]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::ServiceExt;

    #[test]
    fn empty_registry_renders_empty() {
        let reg = MetricsRegistry::new();
        assert_eq!(reg.render(), "");
        assert_eq!(reg.route_count(), 0);
    }

    #[test]
    fn record_and_render_counter_series() {
        let reg = MetricsRegistry::new();
        reg.record("/api/v1/users", "GET", 200, 0.002);
        reg.record("/api/v1/users", "GET", 200, 0.02);
        reg.record("/api/v1/users", "GET", 404, 0.002);
        let text = reg.render();
        assert!(text.contains(
            "sdforge_http_requests_total{route=\"/api/v1/users\",method=\"GET\",status=\"200\"} 2"
        ));
        assert!(text.contains(
            "sdforge_http_requests_total{route=\"/api/v1/users\",method=\"GET\",status=\"404\"} 1"
        ));
        assert!(text.contains("# TYPE sdforge_http_requests_total counter"));
    }

    #[test]
    fn histogram_buckets_are_cumulative() {
        let reg = MetricsRegistry::new();
        reg.record("/x", "GET", 200, 0.002); // in le=0.005 only
        reg.record("/x", "GET", 200, 0.03); // in le=0.05 and up
        let text = reg.render();
        let line = |le: &str| {
            text.lines()
                .find(|l| l.contains(&format!("le=\"{le}\"}}")))
                .map(|l| l.rsplit(' ').next().unwrap().to_string())
                .unwrap()
        };
        assert_eq!(line("0.001"), "0");
        assert_eq!(line("0.005"), "1");
        assert_eq!(line("0.05"), "2");
        assert_eq!(line("+Inf"), "2");
        assert!(text.contains(
            "sdforge_http_request_duration_seconds_count{route=\"/x\",method=\"GET\"} 2"
        ));
        assert!(text.contains(
            "sdforge_http_request_duration_seconds_sum{route=\"/x\",method=\"GET\"} 0.032"
        ));
    }

    #[test]
    fn reset_clears_series() {
        let reg = MetricsRegistry::new();
        reg.record("/x", "GET", 200, 0.001);
        assert_eq!(reg.route_count(), 1);
        reg.reset();
        assert_eq!(reg.route_count(), 0);
        assert_eq!(reg.render(), "");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn middleware_records_status_and_route_template() {
        let reg = global_registry();
        reg.reset();
        let router = axum::Router::new()
            .route("/api/v1/items/{id}", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(record_middleware));
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/items/42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let text = reg.render();
        assert!(
            text.contains("route=\"/api/v1/items/{id}\""),
            "route template must be used as label, got: {text}"
        );
        reg.reset();
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn metrics_endpoint_serves_prometheus_text() {
        global_registry().reset();
        let router = crate::http::build_with_config(&crate::config::AppConfig::default()).unwrap();
        // generate some traffic through an inventory route? /metrics itself is skipped;
        // just verify the endpoint content type + format header lines.
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap()
            .to_string();
        assert!(ct.starts_with("text/plain"), "content-type: {ct}");
        global_registry().reset();
    }
}
