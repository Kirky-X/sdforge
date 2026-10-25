// Copyright (c) 2026 Kirky.X🌠
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
//! TLS in MVP — point it at a local collector/sidecar). `build_with_config_ext`
//! installs the request-span middleware when the feature is enabled; other
//! protocols record spans via `start_span` / `finish_span` around their
//! dispatch.
//!
//! ```ignore
//! sdforge::otel::install(sdforge::otel::OtelConfig {
//!     endpoint: "http://127.0.0.1:4318".into,
//!     service_name: "my-service".into,
//!     ..Default::default
//! });
//! ```

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::OnceLock;
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
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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

/// An in-flight span; call `finish_span` to record it.
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
    let data = span_data(span);
    if let Ok(mut guard) = span_buffer().lock() {
        // 缓冲不得无界增长：采集端停滞时丢弃最旧的 span。
        make_room(&mut guard);
        guard.push_back(data);
    }
}

/// Turn an in-flight span into the data recorded for export.
fn span_data(span: ActiveSpan) -> SpanData {
    let duration = span.start_instant.elapsed();
    SpanData {
        trace_id: span.trace_id,
        span_id: span.span_id,
        name: span.name,
        start_unix_ms: span.start_unix_ms,
        end_unix_ms: unix_ms(),
        duration_ms: duration.as_millis() as u64,
        attributes: span.attributes,
    }
}

/// 为新 span 腾位：缓冲已满时淘汰最旧的（先进先出），缓冲不得无界增长。
fn make_room(buffer: &mut VecDeque<SpanData>) {
    if buffer.len() >= MAX_BUFFERED_SPANS {
        let overflow = buffer.len() + 1 - MAX_BUFFERED_SPANS;
        for _ in 0..overflow {
            let _ = buffer.pop_front();
        }
    }
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

static SPAN_BUFFER: OnceLock<Mutex<VecDeque<SpanData>>> = OnceLock::new();

/// span 缓冲上限：导出停滞（采集端不可达/超时）时缓冲不得无界增长。
const MAX_BUFFERED_SPANS: usize = 10_000;

/// 导出 I/O 超时（秒）：采集端不可信，连接与读写都不得无限期挂起。
const EXPORT_IO_TIMEOUT_SECS: u64 = 5;

/// 单次导出的总时长上限（秒）。
///
/// `EXPORT_IO_TIMEOUT_SECS` 只是**单次**读/写的空闲超时：采集端每次只吐
/// 1 字节就能让读循环一直续命，总时长无上界。总时限从导出入口起算，覆盖
/// 地址解析、连接、请求写入与读响应全过程，到点即放弃本次导出（失败批次
/// 回填缓冲，下一轮重试；缓冲仍受 [`MAX_BUFFERED_SPANS`] 约束，溢出时淘汰
/// 最旧的 span）。
const EXPORT_DEADLINE_SECS: u64 = 30;

/// 导出响应体读取上限（字节）：采集端（或明文链路上的中间人）不得让
/// 导出进程无界增内存。
const MAX_EXPORT_RESPONSE_BYTES: u64 = 64 * 1024;

/// 解析出的地址数上限：多 A 记录时逐个连接会把连接阶段拉长到 N×超时。
const MAX_RESOLVED_ADDRS: usize = 4;

/// 总时限耗尽的统一错误文本。
/// Canonical export-deadline error, localized through the message catalog.
///
/// The English pattern is kept identical to the catalog entry
/// `otel-export-deadline-exceeded` ("export deadline exceeded").
fn export_deadline_err() -> String {
    crate::i18n::t("otel-export-deadline-exceeded", &[])
}

fn span_buffer() -> &'static Mutex<VecDeque<SpanData>> {
    SPAN_BUFFER.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// Drain recorded spans (export path; also used by tests).
pub fn take_spans() -> Vec<SpanData> {
    span_buffer()
        .lock()
        .map(|mut guard| std::mem::take(&mut *guard).into_iter().collect())
        .unwrap_or_default()
}

/// Put an exported batch back at the head of the buffer after a failed
/// export.
///
/// 批次比导出期间新到的 span 更旧，必须前置；缓冲总量仍受
/// [`MAX_BUFFERED_SPANS`] 约束，超出时淘汰最旧的（批次中最旧的部分）。
fn restore_spans(batch: Vec<SpanData>) {
    if batch.is_empty() {
        return;
    }
    if let Ok(mut guard) = span_buffer().lock() {
        for span in batch.into_iter().rev() {
            guard.push_front(span);
        }
        while guard.len() > MAX_BUFFERED_SPANS {
            let _ = guard.pop_front();
        }
    }
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

/// Connect to the collector with a bounded timeout.
///
/// 采集端地址由部署方配置（可能指向被攻陷的采集器或明文链路上的中间人），
/// 故连接本身也必须有上限——`TcpStream::connect` 会无限期阻塞。
///
/// `deadline` 是本次导出的**绝对**总时限（从导出入口起算）：地址解析与逐
/// 地址连接都必须在此时限内完成，单个地址的连接超时取 `min(剩余预算,
/// io_timeout)`。剩余预算恒为 `deadline - now`，故解析耗掉的时间从同一份
/// 总时限里扣，连接阶段不会另拿到一份完整预算。
fn connect_with_timeout(
    host: &str,
    port: u16,
    deadline: Instant,
    io_timeout: std::time::Duration,
) -> Result<std::net::TcpStream, String> {
    connect_with_resolver(host, port, deadline, io_timeout, resolve_addrs_with_timeout)
}

/// [`connect_with_timeout`] with an injectable address resolver.
fn connect_with_resolver<F>(
    host: &str,
    port: u16,
    deadline: Instant,
    io_timeout: std::time::Duration,
    resolve: F,
) -> Result<std::net::TcpStream, String>
where
    F: FnOnce(&str, u16, Instant) -> Result<Vec<std::net::SocketAddr>, String>,
{
    connect_with_resolver_and_connector(
        host,
        port,
        deadline,
        io_timeout,
        resolve,
        std::net::TcpStream::connect_timeout,
    )
}

/// [`connect_with_resolver`] with an injectable connector.
///
/// The connector is a parameter so the deadline accounting can be exercised
/// without depending on a real unreachable address: a full-tunnel VPN or a
/// transparent proxy answers connections to every address, so no reserved
/// range stays unreachable in such an environment.
fn connect_with_resolver_and_connector<F, C>(
    host: &str,
    port: u16,
    deadline: Instant,
    io_timeout: std::time::Duration,
    resolve: F,
    mut connect: C,
) -> Result<std::net::TcpStream, String>
where
    F: FnOnce(&str, u16, Instant) -> Result<Vec<std::net::SocketAddr>, String>,
    C: FnMut(&std::net::SocketAddr, std::time::Duration) -> std::io::Result<std::net::TcpStream>,
{
    let addrs = resolve(host, port, deadline)?;
    if addrs.is_empty() {
        return Err(crate::i18n::t(
            "otel-resolve-failed",
            &[("error", format!("no address for {host}:{port}"))],
        ));
    }

    let mut last_error = None;
    for addr in addrs {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(export_deadline_err());
        }
        let timeout = remaining.min(io_timeout);
        match connect(&addr, timeout) {
            Ok(stream) => {
                let stream = stream;
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|e| format!("set read timeout failed: {e}"))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|e| format!("set write timeout failed: {e}"))?;
                return Ok(stream);
            }
            Err(e) => {
                let timed_out = matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                );
                last_error = Some(e);
                // 连接超时且超时值取自剩余预算：剩余预算已被这次连接耗尽，
                // 总时限已到，不再尝试剩余地址。
                if (timed_out && remaining <= io_timeout)
                    || deadline.saturating_duration_since(Instant::now()).is_zero()
                {
                    return Err(export_deadline_err());
                }
            }
        }
    }
    if deadline.saturating_duration_since(Instant::now()).is_zero() {
        return Err(export_deadline_err());
    }
    Err(crate::i18n::t(
        "otel-connect-failed",
        &[(
            "error",
            last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "no address".to_string()),
        )],
    ))
}

/// Resolve the collector address under the remaining total budget.
///
/// `ToSocketAddrs` 是阻塞系统调用（DNS 查询可能长时间挂起），故放到独立线程
/// 执行并用 `recv_timeout` 卡住：预算耗尽即放弃本次导出，解析线程在解析
/// 返回后自然结束。
///
/// `deadline` 为绝对总时限：解析只占用其剩余部分，解析耗掉的时间计入同一
/// 份总时限（后续连接阶段不再另算一份）。
fn resolve_addrs_with_timeout(
    host: &str,
    port: u16,
    deadline: Instant,
) -> Result<Vec<std::net::SocketAddr>, String> {
    use std::net::ToSocketAddrs;

    let budget = deadline.saturating_duration_since(Instant::now());
    if budget.is_zero() {
        return Err(export_deadline_err());
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let host = host.to_string();
    std::thread::spawn(move || {
        let resolved = (host.as_str(), port)
            .to_socket_addrs()
            .map(|iter| iter.take(MAX_RESOLVED_ADDRS).collect::<Vec<_>>());
        let _ = tx.send(resolved);
    });
    match rx.recv_timeout(budget) {
        Ok(Ok(addrs)) => Ok(addrs),
        Ok(Err(e)) => Err(crate::i18n::t(
            "otel-resolve-failed",
            &[("error", e.to_string())],
        )),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(export_deadline_err()),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(crate::i18n::t(
            "otel-resolve-failed",
            &[("error", "resolver thread ended unexpectedly".to_string())],
        )),
    }
}

/// Dependency-free HTTP/1.1 POST of a JSON body (MVP: no TLS).
/// Returns the response status code, or an error string.
///
/// The whole exchange is bounded by [`EXPORT_DEADLINE_SECS`]: the collector
/// may dribble bytes out one at a time, and the per-read idle timeout alone
/// would let such a peer keep the export alive indefinitely. The deadline is
/// measured from the start of the export, so address resolution, connect and
/// the request write are bounded by it as well.
pub fn post_json(url: &str, body: &serde_json::Value) -> Result<u16, String> {
    post_json_with_limits(
        url,
        body,
        std::time::Duration::from_secs(EXPORT_IO_TIMEOUT_SECS),
        std::time::Duration::from_secs(EXPORT_DEADLINE_SECS),
    )
}

/// [`post_json`] with explicit I/O and total-duration limits.
///
/// `io_timeout` bounds a single read/write; `deadline` bounds the whole
/// exchange — a collector that dribbles one byte per read must not keep the
/// export alive past it.
fn post_json_with_limits(
    url: &str,
    body: &serde_json::Value,
    io_timeout: std::time::Duration,
    deadline_limit: std::time::Duration,
) -> Result<u16, String> {
    use std::io::Read;
    // 总时限从入口起算：解析、连接、写入、读响应都在此预算内。
    let deadline = Instant::now() + deadline_limit;
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| crate::i18n::t("otel-endpoint-must-be-http", &[]))?;
    let (host_port, path) = rest.split_once('/').unwrap_or((rest, ""));
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().map_err(|e| e.to_string())?),
        None => (host_port.to_string(), 80u16),
    };
    let path = format!("/{path}");
    let payload = body.to_string();

    let mut stream = connect_with_timeout(&host, port, deadline, io_timeout)?;
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    write_all_with_deadline(&mut stream, req.as_bytes(), deadline, io_timeout)?;

    // 读取状态行即可（状态码在首行），但总时长受总时限约束：采集端每次
    // 只吐 1 字节也不得把导出拖成无界等待。
    let mut response: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    // 已扫描到的位置：只扫新到的字节，避免每轮对整个响应重扫。
    let mut scanned = 0usize;
    let mut read_timeout: Option<std::time::Duration> = None;
    loop {
        if status_line_complete(&response, &mut scanned) {
            // 状态行已完整，无需继续读响应体。
            break;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(export_deadline_err());
        }
        let idle = remaining.min(io_timeout);
        if read_timeout != Some(idle) {
            stream
                .set_read_timeout(Some(idle))
                .map_err(|e| format!("set read timeout failed: {e}"))?;
            read_timeout = Some(idle);
        }
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                if response.len() as u64 + n as u64 > MAX_EXPORT_RESPONSE_BYTES {
                    return Err(crate::i18n::t("otel-response-exceeds-byte-cap", &[]));
                }
                response.extend_from_slice(&chunk[..n]);
            }
            // 空闲超时：慢速滴流的采集端。总时限未到则继续等，到点放弃。
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(e) => {
                return Err(crate::i18n::t(
                    "otel-read-failed",
                    &[("error", e.to_string())],
                ));
            }
        }
    }

    let status_line = String::from_utf8_lossy(&response);
    let status_line = status_line.lines().next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| crate::i18n::t("otel-malformed-response", &[]))?;
    Ok(status)
}

/// Whether the HTTP status line is complete (a CRLF has been seen).
///
/// 只扫描上次扫描之后新到的字节（回退 1 字节以捕获跨读边界拆分的 CRLF），
/// 避免每轮对整个响应做全量扫描。
fn status_line_complete(response: &[u8], scanned: &mut usize) -> bool {
    let start = scanned.saturating_sub(1);
    if response[start..].windows(2).any(|w| w == b"\r\n") {
        return true;
    }
    *scanned = response.len();
    false
}

/// Write the request in chunks, checking the total deadline before each
/// chunk.
///
/// `write_all` 无法在内部中断：一次调用可能阻塞到 socket 写超时，期间总时
/// 限无从检查。故自行分片写，预算耗尽即放弃。
fn write_all_with_deadline(
    stream: &mut std::net::TcpStream,
    bytes: &[u8],
    deadline: Instant,
    io_timeout: std::time::Duration,
) -> Result<(), String> {
    use std::io::Write;

    let mut written = 0usize;
    let mut write_timeout: Option<std::time::Duration> = None;
    while written < bytes.len() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(export_deadline_err());
        }
        let idle = remaining.min(io_timeout);
        if write_timeout != Some(idle) {
            stream
                .set_write_timeout(Some(idle))
                .map_err(|e| format!("set write timeout failed: {e}"))?;
            write_timeout = Some(idle);
        }
        match stream.write(&bytes[written..]) {
            Ok(0) => {
                return Err(crate::i18n::t(
                    "otel-write-failed",
                    &[("error", "the collector closed the connection".to_string())],
                ));
            }
            Ok(n) => written += n,
            // 写空闲超时：总时限未到则继续写剩余分片，到点放弃。
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                if deadline.saturating_duration_since(Instant::now()).is_zero() {
                    return Err(export_deadline_err());
                }
            }
            Err(e) => {
                return Err(crate::i18n::t(
                    "otel-write-failed",
                    &[("error", e.to_string())],
                ));
            }
        }
    }
    Ok(())
}

/// Export buffered spans to the collector; returns the HTTP status.
///
/// 传输失败（连接/写入失败、状态行畸形）时批次回填缓冲，下一轮重试；采集端
/// 返回非 2xx 同样表示导出失败：状态码照常返回给调用方，批次一并回填。
/// 回填后缓冲仍受 [`MAX_BUFFERED_SPANS`] 约束，溢出时淘汰最旧的 span。
pub fn flush_spans(config: &OtelConfig) -> Result<u16, String> {
    let spans = take_spans();
    if spans.is_empty() {
        return Ok(200);
    }
    let payload = build_traces_payload(&config.service_name, &spans);
    match post_json(&format!("{}/v1/traces", config.endpoint), &payload) {
        Ok(status) => {
            if !(200..300).contains(&status) {
                restore_spans(spans);
            }
            Ok(status)
        }
        Err(e) => {
            restore_spans(spans);
            Err(e)
        }
    }
}

/// Install the periodic background exporter (spans flushed every
/// `interval`; MVP also flushes metrics counters from the `metrics`
/// feature when enabled).
///
/// The export itself is blocking I/O (`std::net::TcpStream`), so it runs on
/// the blocking thread pool: a slow or hostile collector must not park a
/// tokio worker thread.
pub fn install(config: OtelConfig, interval: std::time::Duration) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;

            // 导出是阻塞 I/O（std::net::TcpStream）：放到阻塞线程池执行，
            // 采集端不可达时不得占住 tokio worker 线程。
            let span_config = config.clone();
            match tokio::task::spawn_blocking(move || flush_spans(&span_config)).await {
                Ok(Ok(status)) if (200..300).contains(&status) => {}
                // 采集端返回非 2xx：导出失败必须可观测，否则停机的采集端会被
                // 当成正常导出静默通过。
                Ok(Ok(status)) => log::warn!(
                    "{}",
                    crate::i18n::t(
                        "otel-span-flush-http-error",
                        &[("status", status.to_string())]
                    )
                ),
                Ok(Err(e)) => log::warn!(
                    "{}",
                    crate::i18n::t("otel-span-flush-failed", &[("error", e.to_string())])
                ),
                Err(e) => log::warn!(
                    "{}",
                    crate::i18n::t("otel-span-flush-failed", &[("error", e.to_string())])
                ),
            }

            #[cfg(feature = "metrics")]
            {
                let metrics_config = config.clone();
                match tokio::task::spawn_blocking(move || flush_metrics(&metrics_config)).await {
                    Ok(Ok(status)) if (200..300).contains(&status) => {}
                    Ok(Ok(status)) => log::warn!(
                        "{}",
                        crate::i18n::t(
                            "otel-metrics-flush-http-error",
                            &[("status", status.to_string())]
                        )
                    ),
                    Ok(Err(e)) => log::warn!(
                        "{}",
                        crate::i18n::t("otel-metrics-flush-failed", &[("error", e.to_string())])
                    ),
                    Err(e) => log::warn!(
                        "{}",
                        crate::i18n::t("otel-metrics-flush-failed", &[("error", e.to_string())])
                    ),
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
        if let Some(rest) = line.strip_prefix("sdforge_http_requests_total{")
            && let Some(value) = rest.rsplit(' ').next().and_then(|v| v.parse::<u64>().ok())
        {
            total += value;
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
    post_json(&format!("{}/v1/metrics", config.endpoint), &payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// take_spans 清空进程级缓冲;并行测试共享该缓冲,必须串行触碰。
    static BUF_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn span_lifecycle_records_data() {
        let _guard = BUF_LOCK.lock().unwrap();
        // 进程级缓冲可能被同进程其它测试（如 http middleware 的 otel 中间
        // 件）并发写入：断言前先清空，避免串扰数据混入。
        take_spans();
        let span = start_span("unit.op");
        let span = with_attr(span, "k", serde_json::json!("v"));
        finish_span(span);
        assert!(buffered_span_count() >= 1);
        let spans = take_spans();
        assert!(spans.iter().any(|s| {
            s.name == "unit.op"
                && s.attributes
                    .iter()
                    .any(|(k, v)| k == "k" && v == &serde_json::json!("v"))
        }));
        // 本 span 已被取走即视为清空生效；并发写者可能产生新数据，不对其
        // 计数做强断言。
        let again = take_spans();
        assert!(!again.iter().any(|s| s.name == "unit.op"));
    }

    #[test]
    fn trace_payload_shape_is_otlp_json() {
        let _guard = BUF_LOCK.lock().unwrap();
        // 清空并发写者的既有串扰数据后再产生本测试的 span。
        take_spans();
        let span = start_span("shape.check");
        finish_span(span);
        let spans = take_spans();
        // 只取本测试产生的 span，隔离 middleware 并发写入的 http.request。
        let own: Vec<SpanData> = spans
            .into_iter()
            .filter(|s| s.name == "shape.check")
            .collect();
        assert_eq!(own.len(), 1, "exactly one own span expected");
        let payload = build_traces_payload("svc-under-test", &own);
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

    /// 慢速滴流的采集端不得把导出拖成无界等待：总时限到点即放弃。
    ///
    /// 采集端每次只吐 1 字节（且永不补齐状态行），单次读写的空闲超时
    /// 会被反复重置——只有总时限能终止这次导出。
    #[test]
    fn slow_drip_collector_is_cut_off_by_the_total_deadline() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::Duration;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            if let Ok((mut socket, _)) = listener.accept() {
                // 读掉请求，然后每次只吐 1 字节且永不结束响应。
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf);
                for _ in 0..200 {
                    if socket.write_all(b" ").is_err() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        });

        let started = Instant::now();
        let result = post_json_with_limits(
            &format!("http://{addr}/v1/traces"),
            &serde_json::json!({"spans": []}),
            Duration::from_millis(100),
            Duration::from_secs(1),
        );
        let elapsed = started.elapsed();

        let err = result.expect_err("a slow drip collector must not keep the export alive");
        assert!(
            err.contains("deadline"),
            "the export must fail on the total deadline: {err}"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "the export must stop at the deadline, took {elapsed:?}"
        );
    }

    /// 连接阶段同样受总时限约束：不可路由的采集端不得拖过总时限。
    ///
    /// 旧实现里每个地址的连接超时固定 5s（与总时限无关），且总时限在
    /// connect 之后才起算——采集端不可达时一次导出就会超出总时限。
    #[test]
    fn connect_phase_is_bounded_by_the_total_deadline() {
        use std::time::Duration;

        // RFC 5737 TEST-NET-1：保留的不可路由地址，连接会挂起直到超时。
        let started = Instant::now();
        let result = post_json_with_limits(
            "http://192.0.2.1:4318/v1/traces",
            &serde_json::json!({"spans": []}),
            Duration::from_secs(5),
            Duration::from_millis(300),
        );
        let elapsed = started.elapsed();

        let err = result.expect_err("an unreachable collector must not keep the export alive");
        assert!(
            err.contains("deadline"),
            "the export must fail on the total deadline: {err}"
        );
        assert!(
            elapsed < Duration::from_secs(3),
            "the connect phase must respect the total deadline, took {elapsed:?}"
        );
    }

    /// 预算耗尽时不得再发起连接：直接按总时限失败。
    #[test]
    fn an_exhausted_budget_fails_before_any_connection() {
        use std::time::Duration;

        let err = connect_with_timeout(
            "127.0.0.1",
            9,
            Instant::now() - Duration::from_millis(1),
            Duration::from_secs(5),
        )
        .expect_err("an exhausted budget must not open a connection");
        assert!(
            err.contains("deadline"),
            "an exhausted budget must report the total deadline: {err}"
        );
    }

    /// 解析阶段也受总时限约束：总时限已过即放弃，不得再等解析返回。
    #[test]
    fn an_expired_deadline_stops_the_resolve_phase() {
        use std::time::Duration;

        let started = Instant::now();
        let err = resolve_addrs_with_timeout(
            "collector.invalid",
            4318,
            Instant::now() - Duration::from_millis(1),
        )
        .expect_err("an expired deadline must not wait for the resolver");
        assert!(
            err.contains("deadline"),
            "an expired deadline must report the total deadline: {err}"
        );
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "an expired deadline must fail immediately, took {:?}",
            started.elapsed()
        );
    }

    /// 解析耗掉的时间必须计入总时限：连接阶段只能拿到剩余预算。
    ///
    /// 若解析独占整份预算、连接阶段再按整份预算重新计时，同一份预算会被计入
    /// 两次，总耗时可达 2 倍总时限。
    #[test]
    fn resolve_time_counts_against_the_total_deadline() {
        use std::time::Duration;

        let started = Instant::now();
        let deadline = started + Duration::from_secs(1);
        // 解析耗掉 700ms 后才返回一个不可路由地址：连接阶段只应拿到剩余的
        // 300ms，而不是重新拿到整份 1s 预算。
        // The connect phase uses an injected connector that simulates a
        // connection hanging until its timeout: a real unreachable address
        // gets answered under a full-tunnel VPN / transparent proxy (the
        // connect then succeeds at once), which would no longer exercise
        // the "remaining budget exhausted" path.
        let err = connect_with_resolver_and_connector(
            "collector.invalid",
            4318,
            deadline,
            Duration::from_secs(5),
            |_host, _port, _deadline| {
                std::thread::sleep(Duration::from_millis(700));
                Ok(vec![std::net::SocketAddr::from(([192, 0, 2, 1], 4318))])
            },
            |_addr, timeout| {
                std::thread::sleep(timeout);
                Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "simulated connect timeout",
                ))
            },
        )
        .expect_err("the export must fail once the total deadline is reached");
        let elapsed = started.elapsed();

        assert!(
            err.contains("deadline"),
            "the export must fail on the total deadline: {err}"
        );
        assert!(
            elapsed < Duration::from_millis(1400),
            "the resolve time must be charged to the total deadline, took {elapsed:?}"
        );
    }

    /// 写阶段同样受总时限约束：预算耗尽后不得再写任何字节。
    #[test]
    fn write_phase_stops_at_the_total_deadline() {
        use std::io::Read;
        use std::net::TcpListener;
        use std::time::Duration;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().expect("accept");
            socket
                .set_read_timeout(Some(Duration::from_millis(300)))
                .expect("set read timeout");
            let mut buf = [0u8; 64];
            // 对端不读：客户端预算耗尽时不得有字节落到线上。
            socket.read(&mut buf).ok()
        });

        let mut stream = std::net::TcpStream::connect(addr).expect("connect");
        let err = write_all_with_deadline(
            &mut stream,
            b"POST /v1/traces HTTP/1.1\r\n\r\n",
            Instant::now() - Duration::from_millis(1),
            Duration::from_secs(1),
        )
        .expect_err("an expired deadline must stop the write");
        assert!(
            err.contains("deadline"),
            "an expired deadline must abort the write: {err}"
        );

        let received = peer.join().expect("peer joined");
        assert!(
            received.is_none(),
            "no byte may reach the collector after the deadline expired"
        );
    }

    /// 状态行的 CRLF 跨读边界拆分时仍须被识别（扫描偏移不得漏判）。
    #[test]
    fn status_line_split_across_reads_is_recognized() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::Duration;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            if let Ok((mut socket, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf);
                // CRLF 拆成两次写：扫描偏移必须回退 1 字节才能识别。
                let _ = socket.write_all(b"HTTP/1.1 200 OK\r");
                std::thread::sleep(Duration::from_millis(50));
                let _ = socket.write_all(b"\n");
                std::thread::sleep(Duration::from_millis(50));
            }
        });

        let status = post_json_with_limits(
            &format!("http://{addr}/v1/traces"),
            &serde_json::json!({"spans": []}),
            Duration::from_secs(1),
            Duration::from_secs(5),
        )
        .expect("the status line must be parsed");
        assert_eq!(status, 200);
    }

    /// 导出失败时批次不得丢失：回填缓冲等待下一轮重试。
    #[test]
    fn failed_span_export_keeps_the_batch_buffered() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::Duration;

        let _guard = BUF_LOCK.lock().unwrap();
        take_spans();

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            if let Ok((mut socket, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = socket.read(&mut buf);
                let _ = socket
                    .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n");
                std::thread::sleep(Duration::from_millis(50));
            }
        });

        let span = start_span("unit.export.fail");
        finish_span(span);
        let before = buffered_span_count();
        assert!(before >= 1, "the span must be buffered before the export");

        let config = OtelConfig {
            endpoint: format!("http://{addr}"),
            service_name: "unit-under-test".to_string(),
        };
        let status = flush_spans(&config).expect("the collector answered");
        assert_eq!(
            status, 500,
            "a non-2xx status must be reported, not swallowed"
        );

        let after = buffered_span_count();
        assert!(
            after >= before,
            "the failed batch must stay buffered for the next round: before={before} after={after}"
        );
        let spans = take_spans();
        assert_eq!(
            spans
                .iter()
                .filter(|s| s.name == "unit.export.fail")
                .count(),
            1,
            "the failed batch must stay buffered for the next round"
        );
    }

    /// 非 2xx 必须可观测：状态码透传给调用方，日志文案带状态码。
    #[test]
    fn non_2xx_status_is_observable() {
        let span_message = crate::i18n::translate_for(
            "en",
            "otel-span-flush-http-error",
            &[("status", "500".to_string())],
        );
        assert!(
            span_message.contains("500"),
            "the span flush error message must carry the status code: {span_message}"
        );
        let metrics_message = crate::i18n::translate_for(
            "en",
            "otel-metrics-flush-http-error",
            &[("status", "503".to_string())],
        );
        assert!(
            metrics_message.contains("503"),
            "the metrics flush error message must carry the status code: {metrics_message}"
        );
    }

    /// 缓冲满时淘汰最旧的 span：先进先出，最新的必须留下。
    #[test]
    fn span_buffer_evicts_the_oldest_when_full() {
        // 局部缓冲：淘汰语义必须是先进先出（最旧的先走）。
        let mut buffer: VecDeque<SpanData> = VecDeque::new();
        for i in 0..MAX_BUFFERED_SPANS {
            let span = with_attr(start_span("unit.evict"), "i", serde_json::json!(i));
            buffer.push_back(span_data(span));
        }
        make_room(&mut buffer);
        buffer.push_back(span_data(start_span("unit.evict")));
        assert_eq!(
            buffer.len(),
            MAX_BUFFERED_SPANS,
            "the buffer must not grow beyond the cap"
        );
        let kept: Vec<u64> = buffer
            .iter()
            .filter_map(|s| {
                s.attributes
                    .iter()
                    .find(|(k, _)| k == "i")
                    .and_then(|(_, v)| v.as_u64())
            })
            .collect();
        assert_eq!(
            kept.first(),
            Some(&1),
            "the oldest span must be evicted first: {kept:?}"
        );
        assert_eq!(
            kept.last(),
            Some(&(MAX_BUFFERED_SPANS as u64 - 1)),
            "the newest span must be kept: {kept:?}"
        );

        // 进程级缓冲：收进超出上限的 span 后总量不得越过上限。
        let _guard = BUF_LOCK.lock().unwrap();
        take_spans();
        for _ in 0..MAX_BUFFERED_SPANS + 1 {
            finish_span(start_span("unit.evict"));
        }
        assert_eq!(
            buffered_span_count(),
            MAX_BUFFERED_SPANS,
            "the buffer must not grow beyond the cap"
        );
        take_spans();
    }
}
