// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! OpenTelemetry observation — MVP OTLP/HTTP.
//!
//! Feature `otel`: request spans (route/method/status/duration) are recorded
//! per protocol entry point and exported to an OTLP/HTTP collector:
//!
//! - spans → `POST {endpoint}/v1/traces` (OTLP/HTTP JSON),
//! - request counter snapshot → `POST {endpoint}/v1/metrics`.
//!
//! The exporter is dependency-free (raw HTTP/1.1 POST over `TcpStream`, no
//! TLS in MVP — point it at a local collector/sidecar). `build_with_config`
//! installs the request-span middleware when the feature is enabled; other
//! protocols record spans via [`start_span`] / [`finish_span`] around their
//! dispatch.
//!
//! ```ignore
//! sdforge::otel::install(sdforge::otel::OtelConfig {
//!     endpoint: "http://127.0.0.1:4318".into(),
//!     service_name: "my-service".into(),
//!     ..Default::default()
//! });
//! ```

use std::sync::Mutex;
use std::sync::{OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// OTLP/HTTP exporter configuration.
#[derive(Debug, Clone)]
pub struct OtelConfig {
    /// Collector base URL (no trailing slash), e.g. `http://127.0.0.1:4318`.
    pub endpoint: String,
    /// `service.name` resource attribute.
    pub service_name: String,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:4318".to_string(),
            service_name: "sdforge".to_string(),
        }
    }
}

static SPAN_ID: AtomicCounter = AtomicCounter::new(1);
static TRACE_ID: AtomicCounter = AtomicCounter::new(1);

/// Tiny monotonic counter for dependency-free id generation.
struct AtomicCounter(std::sync::atomic::AtomicU64);

impl AtomicCounter {
    const fn new(start: u64) -> Self {
        Self(std::sync::atomic::AtomicU64::new(start))
    }
    fn next(&self) -> u64 {
        self.0
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

fn hex_id(value: u64, width: usize) -> String {
    format!("{:0width$x}", value, width = width)
}

/// A finished span awaiting export.
#[derive(Debug, Clone)]
pub struct SpanData {
    /// 32-hex trace id.
    pub trace_id: String,
    /// 16-hex span id.
    pub span_id: String,
    /// Human-readable span name (route template / operation).
    pub name: String,
    /// Start time (unix ms).
    pub start_unix_ms: u128,
    /// End time (unix ms).
    pub end_unix_ms: u128,
    /// Duration milliseconds.
    pub duration_ms: u64,
    /// Span attributes (OTLP key/values).
    pub attributes: Vec<(String, serde_json::Value)>,
}

/// An in-flight span; call [`finish_span`] to record it.
#[derive(Debug)]
pub struct ActiveSpan {
    trace_id: String,
    span_id: String,
    name: String,
    attributes: Vec<(String, serde_json::Value)>,
    start_instant: Instant,
    start_unix_ms: u128,
}

/// Start a span with generated trace/span ids.
pub fn start_span(name: impl Into<String>) -> ActiveSpan {
    let trace_id = hex_id(TRACE_ID.next(), 32);
    let span_id = hex_id(SPAN_ID.next(), 16);
    ActiveSpan {
        trace_id,
        span_id,
        name: name.into(),
        attributes: Vec::new(),
        start_instant: Instant::now(),
        start_unix_ms: unix_ms(),
    }
}

/// Attach an attribute to an active span.
pub fn with_attr(mut span: ActiveSpan, key: &str, value: serde_json::Value) -> ActiveSpan {
    span.attributes.push((key.to_string(), value));
    span
}

/// Finish a span, recording it into the export buffer.
pub fn finish_span(span: ActiveSpan) {
    let duration = span.start_instant.elapsed();
    let data = SpanData {
        trace_id: span.trace_id,
        span_id: span.span_id,
        name: span.name,
        start_unix_ms: span.start_unix_ms,
        end_unix_ms: unix_ms(),
        duration_ms: duration.as_millis() as u64,
        attributes: span.attributes,
    };
    if let Ok(mut guard) = span_buffer().lock() {
        guard.push(data);
    }
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

static SPAN_BUFFER: OnceLock<Mutex<Vec<SpanData>>> = OnceLock::new();

fn span_buffer() -> &'static Mutex<Vec<SpanData>> {
    SPAN_BUFFER.get_or_init(|| Mutex::new(Vec::new()))
}

/// Drain recorded spans (export path; also used by tests).
pub fn take_spans() -> Vec<SpanData> {
    span_buffer()
        .lock()
        .map(|mut guard| std::mem::take(&mut *guard))
        .unwrap_or_default()
}

/// Number of spans currently buffered.
pub fn buffered_span_count() -> usize {
    span_buffer().lock().map(|g| g.len()).unwrap_or(0)
}

/// Build the OTLP/HTTP JSON `resourceSpans` payload.
pub fn build_traces_payload(service_name: &str, spans: &[SpanData]) -> serde_json::Value {
    let spans_json: Vec<serde_json::Value> = spans
        .iter()
        .map(|s| {
            let attributes: Vec<serde_json::Value> = s
                .attributes
                .iter()
                .map(|(k, v)| {
                    serde_json::json!({
                        "key": k,
                        "value": {"stringValue": v.to_string()},
                    })
                })
                .collect();
            serde_json::json!({
                "traceId": s.trace_id,
                "spanId": s.span_id,
                "name": s.name,
                "kind": "SPAN_KIND_SERVER",
                "startTimeUnixNano": (s.start_unix_ms as u64) * 1_000_000u64,
                "endTimeUnixNano": (s.end_unix_ms as u64) * 1_000_000u64,
                "attributes": attributes,
            })
        })
        .collect();
    serde_json::json!({
        "resourceSpans": [{
            "resource": {
                "attributes": [
                    {"key": "service.name", "value": {"stringValue": service_name}},
                    {"key": "telemetry.sdk.name", "value": {"stringValue": "sdforge"}},
                ],
            },
            "scopeSpans": [{
                "scope": {"name": "sdforge-otel"},
                "spans": spans_json,
            }],
        }],
    })
}

/// Request-span middleware: wraps the whole chain in a `http.request` span.
pub async fn otel_span_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let route = req
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| "unmatched".to_string());
    let span = start_span("http.request");
    let span = with_attr(
        span,
        "http.method",
        serde_json::json!(req.method().to_string()),
    );
    let span = with_attr(span, "http.route", serde_json::json!(route));
    let response = next.run(req).await;
    let span = with_attr(
        span,
        "http.status_code",
        serde_json::json!(response.status().as_u16()),
    );
    finish_span(span);
    response
}

/// Dependency-free HTTP/1.1 POST of a JSON body (MVP: no TLS).
/// Returns the response status code, or an error string.
pub fn post_json(url: &str, body: &serde_json::Value) -> Result<u16, String> {
    use std::io::{Read, Write};
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "otel endpoint must be http:// in MVP".to_string())?;
    let (host_port, path) = rest.split_once('/').unwrap_or((rest, ""));
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().map_err(|e| e.to_string())?),
        None => (host_port.to_string(), 80u16),
    };
    let path = format!("/{path}");
    let payload = body.to_string();

    let mut stream = std::net::TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("connect failed: {e}"))?;
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("write failed: {e}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("read failed: {e}"))?;
    let status_line = response.lines().next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| "malformed response".to_string())?;
    Ok(status)
}

/// Export buffered spans to the collector; returns the HTTP status.
pub fn flush_spans(config: &OtelConfig) -> Result<u16, String> {
    let spans = take_spans();
    if spans.is_empty() {
        return Ok(200);
    }
    post_json(
        &format!("{}/v1/traces", config.endpoint),
        &build_traces_payload(&config.service_name, &spans),
    )
}

/// Install the periodic background exporter (spans flushed every
/// `interval`; MVP also flushes metrics counters from the `metrics`
/// feature when enabled).
pub fn install(config: OtelConfig, interval: std::time::Duration) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            // Export failures must be observable: a silent drop would hide a
            // downed collector from operators.
            if let Err(e) = flush_spans(&config) {
                log::warn!("otel: span flush failed: {e}");
            }
            #[cfg(feature = "metrics")]
            {
                if let Err(e) = flush_metrics(&config) {
                    log::warn!("otel: metrics flush failed: {e}");
                }
            }
        }
    });
}

/// Export a minimal request-counter snapshot to `/v1/metrics`
/// (requires the `metrics` feature for the data source).
#[cfg(feature = "metrics")]
pub fn flush_metrics(config: &OtelConfig) -> Result<u16, String> {
    let registry = crate::metrics::global_registry();
    let text = registry.render();
    let mut total: u64 = 0;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("sdforge_http_requests_total{") {
            if let Some(value) = rest.rsplit(' ').next().and_then(|v| v.parse::<u64>().ok()) {
                total += value;
            }
        }
    }
    let payload = serde_json::json!({
        "resourceMetrics": [{
            "resource": {"attributes": [
                {"key": "service.name", "value": {"stringValue": config.service_name}},
            ]},
            "scopeMetrics": [{
                "scope": {"name": "sdforge-otel"},
                "metrics": [{
                    "name": "sdforge_http_requests_total",
                    "sum": {"dataPoints": [{"asInt": total}]},
                }],
            }],
        }],
    });
    post_json(
        &format!("{}/v1/metrics", config.endpoint),
        &payload,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_lifecycle_records_data() {
        let span = start_span("unit.op");
        let span = with_attr(span, "k", serde_json::json!("v"));
        finish_span(span);
        assert!(buffered_span_count() >= 1);
        let spans = take_spans();
        assert!(spans.iter().any(|s| s.name == "unit.op" && s
            .attributes
            .iter()
            .any(|(k, v)| k == "k" && v == &serde_json::json!("v"))));
        assert_eq!(buffered_span_count(), 0);
    }

    #[test]
    fn trace_payload_shape_is_otlp_json() {
        let span = start_span("shape.check");
        finish_span(span);
        let spans = take_spans();
        let payload = build_traces_payload("svc-under-test", &spans);
        let rs = payload["resourceSpans"][0].clone();
        assert_eq!(
            rs["resource"]["attributes"][0]["key"], "service.name",
            "resource attributes must carry service.name"
        );
        assert_eq!(
            rs["resource"]["attributes"][0]["value"]["stringValue"],
            "svc-under-test"
        );
        let span_json = &rs["scopeSpans"][0]["spans"][0];
        assert_eq!(span_json["name"], "shape.check");
        assert_eq!(span_json["traceId"].as_str().unwrap().len(), 32);
        assert_eq!(span_json["spanId"].as_str().unwrap().len(), 16);
    }

    #[test]
    fn post_json_rejects_non_http_schemes() {
        assert!(post_json("https://collector:4318/v1/traces", &serde_json::json!({})).is_err());
    }
}
