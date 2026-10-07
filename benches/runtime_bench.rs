// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Runtime performance baselines.
//!
//! Run with:
//! ```text
//! cargo bench --bench runtime_bench --features http
//! cargo bench --bench runtime_bench --features serve-tls -- "tls_termination|plaintext_baseline|tls_cert_reloader"
//! ```
//!
//! Baselines are recorded in `docs/PERFORMANCE.md`. These cover the request
//! hot path (routing + middleware stack + handler dispatch), JSON
//! serialization, and (behind `serve-tls`) TLS termination round-trips plus
//! certificate-reloader costs; macro-expansion compile-time cost is
//! intentionally NOT measured here (see docs/PERFORMANCE.md).

use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;

/// A forge endpoint used for dispatch benchmarks (plain handler).
#[sdforge::forge(
    name = "bench_ping",
    version = "v1",
    path = "/bench/ping",
    method = "GET",
    description = "Bench ping"
)]
async fn bench_ping() -> serde_json::Value {
    json!({"ok": true})
}

/// A forge endpoint with a path parameter (exercises path extraction).
#[sdforge::forge(
    name = "bench_user",
    version = "v1",
    path = "/bench/user/:id",
    method = "GET",
    description = "Bench user"
)]
async fn bench_user(id: u64) -> serde_json::Value {
    json!({"id": id})
}

/// A lifecycle-annotated endpoint (exercises the per-route lifecycle header
/// layer on the dispatch hot path — annotated vs `plain_get` shows the
/// layer's marginal cost).
#[sdforge::forge(
    name = "bench_ping_lifecycle",
    version = "v1",
    path = "/bench/ping_lifecycle",
    method = "GET",
    description = "Bench ping with lifecycle annotation",
    deprecated,
    sunset = "2026-12-31",
    successor = "/api/v2/bench/ping"
)]
async fn bench_ping_lifecycle() -> serde_json::Value {
    json!({"ok": true})
}

fn bench_route_dispatch(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let router = sdforge::http::build();

    let mut group = c.benchmark_group("route_dispatch");
    group.throughput(criterion::Throughput::Elements(1));

    group.bench_function("plain_get", |b| {
        b.iter(|| {
            rt.block_on({
                let mut r = router.clone();
                async move {
                    use tower::Service;
                    let req = axum::http::Request::builder()
                        .uri("/api/v1/bench/ping")
                        .body(axum::body::Body::empty())
                        .unwrap();
                    let res = Service::call(&mut r, req).await.unwrap();
                    assert_eq!(res.status(), 200);
                }
            })
        });
    });

    group.bench_function("path_param_get", |b| {
        b.iter(|| {
            rt.block_on({
                let mut r = router.clone();
                async move {
                    use tower::Service;
                    let req = axum::http::Request::builder()
                        .uri("/api/v1/bench/user/12345")
                        .body(axum::body::Body::empty())
                        .unwrap();
                    let res = Service::call(&mut r, req).await.unwrap();
                    assert_eq!(res.status(), 200);
                }
            })
        });
    });

    // 生命周期注解端点：与 plain_get 对比得出 per-route lifecycle 层的
    // 边际成本（三个响应头的注入）。
    group.bench_function("lifecycle_annotated_get", |b| {
        b.iter(|| {
            rt.block_on({
                let mut r = router.clone();
                async move {
                    use tower::Service;
                    let req = axum::http::Request::builder()
                        .uri("/api/v1/bench/ping_lifecycle")
                        .body(axum::body::Body::empty())
                        .unwrap();
                    let res = Service::call(&mut r, req).await.unwrap();
                    assert_eq!(res.status(), 200);
                    assert_eq!(
                        res.headers().get("deprecation").unwrap(),
                        "true",
                        "annotated endpoint must stamp lifecycle headers"
                    );
                }
            })
        });
    });

    group.finish();
}

fn bench_unified_handler_dispatch(c: &mut Criterion) {
    // The protocol-unified handler path (gRPC/CLI): HandlerArgs map → typed
    // parse → handler → serialize. Measured via the CLI registration's
    // handler fn if available; exercised indirectly through JSON cost here.
    let mut group = c.benchmark_group("handler_args");

    group.bench_function("handler_args_build_5_params", |b| {
        b.iter(|| {
            let mut args = std::collections::HashMap::new();
            args.insert("a", "1".to_string());
            args.insert("b", "two".to_string());
            args.insert("c", "3.5".to_string());
            args.insert("d", "true".to_string());
            args.insert("e", "2026-09-11".to_string());
            std::hint::black_box(args)
        });
    });

    group.finish();
}

fn bench_json_serialization(c: &mut Criterion) {
    let payload = json!({
        "id": 12345u64,
        "name": "benchmark-user",
        "email": "user@example.com",
        "active": true,
        "tags": ["alpha", "beta", "gamma"],
        "profile": {
            "level": 3,
            "score": 98.5,
            "joined": "2026-09-11T00:00:00Z",
        },
    });

    let mut group = c.benchmark_group("json");

    group.bench_function("serialize_nested_object", |b| {
        b.iter(|| {
            let s = serde_json::to_string(&payload).unwrap();
            std::hint::black_box(s)
        });
    });

    group.bench_function("deserialize_nested_object", |b| {
        let text = serde_json::to_string(&payload).unwrap();
        b.iter(|| {
            let v: serde_json::Value = serde_json::from_str(&text).unwrap();
            std::hint::black_box(v)
        });
    });

    group.finish();
}

/// TLS 终止热路径基线（`serve-tls`）：环回 https 往返 vs 明文对照、
/// 证书重载器读路径与换盘成本。数据记录在 `docs/PERFORMANCE.md`。
/// `serve-tls` 未启用时空实现占位——本 bench 的 `required-features` 只含
/// `http`，文档化的 `--features http` 调用路径必须可编译。
#[cfg(not(feature = "serve-tls"))]
fn bench_tls_termination(_: &mut Criterion) {}

#[cfg(feature = "serve-tls")]
fn bench_tls_termination(c: &mut Criterion) {
    use sdforge::config::TlsConfig;
    use sdforge::http::tls::{ReloadingTls, TlsServeConfig, serve_with_graceful_shutdown_tls};
    use sdforge::http::{GracefulShutdownConfig, serve_with_graceful_shutdown};
    use std::time::Duration;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cert_path = dir.path().join("cert.pem");
    let key_path = dir.path().join("key.pem");
    std::fs::write(&cert_path, certified.cert.pem()).unwrap();
    std::fs::write(&key_path, certified.signing_key.serialize_pem()).unwrap();
    let tls = TlsConfig::new(
        cert_path.to_string_lossy().into_owned(),
        key_path.to_string_lossy().into_owned(),
    );

    let ping_router =
        || axum::Router::new().route("/ping", axum::routing::get(|| async { "pong" }));

    let wait_until_up = |rt: &tokio::runtime::Runtime, addr: std::net::SocketAddr| {
        rt.block_on(async {
            for _ in 0..200 {
                if tokio::net::TcpStream::connect(addr).await.is_ok() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("bench server never came up at {addr}");
        });
    };

    let https_client = || {
        reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap()
    };

    // TLS serve（停机触发器在 bench 结束前保持未触发——tx 存活即不关停；
    // runtime drop 收敛一切）。
    let (tls_trigger_tx, tls_addr) = rt.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(serve_with_graceful_shutdown_tls(
            ping_router(),
            listener,
            sdforge::http::tls::tls_acceptor(&tls).unwrap(),
            async move {
                let _ = trigger_rx.await;
            },
            TlsServeConfig::default(),
        ));
        (trigger_tx, addr)
    });
    wait_until_up(&rt, tls_addr);

    // 明文对照 serve（同一 router / 同一排空配置，仅传输层不同）。
    let (plain_trigger_tx, plain_addr) = rt.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(serve_with_graceful_shutdown(
            ping_router(),
            listener,
            async move {
                let _ = trigger_rx.await;
            },
            GracefulShutdownConfig::default(),
        ));
        (trigger_tx, addr)
    });
    wait_until_up(&rt, plain_addr);
    // 显式保活：触发器只随 bench 作用域结束而释放。
    let _keepalive = (tls_trigger_tx, plain_trigger_tx);

    let mut group = c.benchmark_group("tls_termination");
    group.throughput(criterion::Throughput::Elements(1));

    // 复用连接池的纯请求成本（TLS 记录层加解密 + HTTP 往返）。
    group.bench_function("https_request_pooled_conn", |b| {
        let client = https_client();
        b.iter(|| {
            let status = rt.block_on(async {
                client
                    .get(format!("https://{tls_addr}/ping"))
                    .send()
                    .await
                    .unwrap()
                    .status()
            });
            assert_eq!(status, 200);
        });
    });

    // 每 iter 新建连接：TLS 握手 + 请求往返（部署面上每新连接的进入成本）。
    group.bench_function("https_handshake_and_request", |b| {
        b.iter(|| {
            let status = rt.block_on(async {
                https_client()
                    .get(format!("https://{tls_addr}/ping"))
                    .send()
                    .await
                    .unwrap()
                    .status()
            });
            assert_eq!(status, 200);
        });
    });

    group.finish();

    let mut group = c.benchmark_group("plaintext_baseline");
    group.throughput(criterion::Throughput::Elements(1));

    group.bench_function("http_request_pooled_conn", |b| {
        let client = reqwest::Client::new();
        b.iter(|| {
            let status = rt.block_on(async {
                client
                    .get(format!("http://{plain_addr}/ping"))
                    .send()
                    .await
                    .unwrap()
                    .status()
            });
            assert_eq!(status, 200);
        });
    });

    group.finish();

    // 证书重载器：读路径与换盘成本。resolve 读路径以
    // `end_entity_certificate()` 为代理（rustls `ClientHello` 无法在
    // bench 内构造；两者同为 RwLock 读 + 终端实体 DER 访问）。
    let reloading = ReloadingTls::new(&tls).expect("reloading tls builds");

    let mut group = c.benchmark_group("tls_cert_reloader");
    group.throughput(criterion::Throughput::Elements(1));

    group.bench_function("resolve_read_path", |b| {
        b.iter(|| {
            let der = reloading.end_entity_certificate().unwrap();
            std::hint::black_box(der);
        });
    });

    group.bench_function("reload_from_disk", |b| {
        b.iter(|| {
            reloading.reload().unwrap();
        });
    });

    // 换盘与并发进入交叠：4 个新连接握手 + 1 次 reload 的批次耗时。
    group.bench_function("reload_plus_4_concurrent_handshakes", |b| {
        b.iter(|| {
            rt.block_on(async {
                let handshake = || async {
                    https_client()
                        .get(format!("https://{tls_addr}/ping"))
                        .send()
                        .await
                        .unwrap()
                        .status()
                };
                let (r, s0, s1, s2, s3) = tokio::join!(
                    async { reloading.reload() },
                    handshake(),
                    handshake(),
                    handshake(),
                    handshake()
                );
                r.expect("reload with matched pair");
                for status in [s0, s1, s2, s3] {
                    assert_eq!(status, 200);
                }
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_route_dispatch,
    bench_unified_handler_dispatch,
    bench_json_serialization,
    bench_tls_termination,
    bench_grpc_streaming,
    bench_i18n_translation,
    bench_simd_json,
    bench_middleware_tiers
);
criterion_main!(benches);

/// simd-json 与 serde_json 的序列化/反序列化对比（`simd-json` feature）。
/// 无 `simd-json` 时空存根——`--features http` 调用路径必须可编译（同
/// TLS/grpc 组的存根约定）。`simd_from_str` 按 facade 口径测量（含输入
/// 字节拷贝，simd-json 需要 mutable buffer）。
#[cfg(feature = "simd-json")]
fn bench_simd_json(c: &mut Criterion) {
    let payload = serde_json::json!({
        "id": 12345u64,
        "name": "benchmark-user",
        "email": "user@example.com",
        "active": true,
        "tags": ["alpha", "beta", "gamma"],
        "profile": {
            "level": 3,
            "score": 98.5,
            "joined": "2026-09-11T00:00:00Z",
        },
    });
    let text = serde_json::to_string(&payload).unwrap();

    let mut group = c.benchmark_group("simd_json");

    group.bench_function("serialize_serde_json", |b| {
        b.iter(|| {
            let s = serde_json::to_string(&payload).unwrap();
            std::hint::black_box(s)
        });
    });

    group.bench_function("serialize_simd", |b| {
        b.iter(|| {
            let s = sdforge::core::json::simd_to_string(&payload).unwrap();
            std::hint::black_box(s)
        });
    });

    group.bench_function("deserialize_serde_json", |b| {
        b.iter(|| {
            let v: serde_json::Value = serde_json::from_str(&text).unwrap();
            std::hint::black_box(v)
        });
    });

    group.bench_function("deserialize_simd", |b| {
        b.iter(|| {
            let v: serde_json::Value = sdforge::core::json::simd_from_str(&text).unwrap();
            std::hint::black_box(v)
        });
    });

    // 大 payload（~64 KiB，512 个 item 的数组）：simd-json 的 SIMD 解析器
    // 主场——小 payload 下 SIMD 启动/分配成本占主导（见上方反直觉结果）。
    let large = serde_json::json!({
        "batch": "bench",
        "items": (0..512)
            .map(|i| serde_json::json!({
                "id": i,
                "name": format!("item-{i}"),
                "score": i as f64 * 1.5,
                "tags": ["a", "b", "c"],
            }))
            .collect::<Vec<_>>(),
    });
    let large_text = serde_json::to_string(&large).unwrap();

    group.bench_function("deserialize_large_serde_json", |b| {
        b.iter(|| {
            let v: serde_json::Value = serde_json::from_str(&large_text).unwrap();
            std::hint::black_box(v)
        });
    });

    group.bench_function("deserialize_large_simd", |b| {
        b.iter(|| {
            let v: serde_json::Value = sdforge::core::json::simd_from_str(&large_text).unwrap();
            std::hint::black_box(v)
        });
    });

    group.finish();
}

#[cfg(not(feature = "simd-json"))]
fn bench_simd_json(_: &mut Criterion) {}

/// `build_with_config` 逐中间件累加分档（审查欠账收口）：每档一个
/// Router，`Service::call` GET /api/v1/bench/ping 的完整请求处理耗时。
/// 分档按 `build_with_config` 实际装配面（body-limit / compression /
/// timeout / security-headers 恒装；auth/etag/metrics/context/cors/
/// idempotency 按 feature + config 门控逐级叠加），feature 未启用时该
/// 分档不编译（cfg 各自独立）。HTTP 侧限流由调用方以 `rate_limit_layer`
/// 自装（不在 build_with_config 装配面内），故不设限流分档。
fn bench_middleware_tiers(c: &mut Criterion) {
    #[cfg(feature = "security")]
    use sdforge::config::ApiKeySeed;
    use sdforge::config::{AuthConfig, SdForgeConfig};

    // 共享 runtime 外提：每迭代不含 tokio 构建常数（各档绝对值纯度）。
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let call_ping = |rt: &tokio::runtime::Runtime, router: &mut axum::Router| {
        rt.block_on(async {
            use tower::Service;
            let req = axum::http::Request::builder()
                .uri("/api/v1/bench/ping")
                .body(axum::body::Body::empty())
                .unwrap();
            let res = Service::call(router, req).await.unwrap();
            assert_eq!(res.status(), 200);
        });
    };

    // 认证档驱动：请求带合法 key（bench- 前缀）。仅在 security 特性下被
    // t1/t6 档位消费，无 security 时允许空置。
    #[cfg_attr(not(feature = "security"), allow(unused_variables))]
    let authed_ping = |rt: &tokio::runtime::Runtime, router: &mut axum::Router| {
        rt.block_on(async {
            use tower::Service;
            let req = axum::http::Request::builder()
                .uri("/api/v1/bench/ping")
                .header("x-api-key", "bench-bench-key")
                .body(axum::body::Body::empty())
                .unwrap();
            let res = Service::call(router, req).await.unwrap();
            assert_eq!(res.status(), 200);
        });
    };

    fn base_config() -> SdForgeConfig {
        SdForgeConfig {
            server: sdforge::config::ServerConfig::default(),
            authentication: AuthConfig::None,
            timeout: None,
            #[cfg(feature = "cache")]
            cache: sdforge::config::CacheConfig::default(),
            #[cfg(feature = "security")]
            security: sdforge::config::SecurityConfig::default(),
        }
    }

    let mut group = c.benchmark_group("middleware_tiers");

    // T0：恒装层（RequestBodyLimit + Compression + Timeout + 安全响应头）。
    #[cfg_attr(not(feature = "security"), allow(unused_mut))]
    let mut base_router = sdforge::http::build_with_config(&base_config()).unwrap();
    group.bench_function("t0_base_config", |b| {
        b.iter(|| call_ping(&rt, &mut base_router));
    });

    // T1：+ ApiKey 认证（合法 key 请求 → 200）。
    #[cfg(feature = "security")]
    {
        let mut config = base_config();
        config.authentication = AuthConfig::ApiKey {
            header_name: "x-api-key".to_string(),
            // 前缀必须非空：认证层对空前缀 fail-closed（防认证绕过的安全修复）。
            prefix: "bench-".to_string(),
            keys: vec![ApiKeySeed {
                key: "bench-key".to_string(),
                // 空 permissions 会被 validate_key 判为无效（None）→ 401。
                permissions: vec!["bench".to_string()],
            }],
        };
        let mut router = sdforge::http::build_with_config(&config).unwrap();
        group.bench_function("t1_auth_api_key", |b| {
            b.iter(|| authed_ping(&rt, &mut router));
        });
    }

    // T2：+ ETag 条件请求中间件。
    #[cfg(feature = "etag")]
    {
        let mut router = sdforge::http::build_with_config(&base_config()).unwrap();
        group.bench_function("t2_etag", |b| {
            b.iter(|| call_ping(&rt, &mut router));
        });
    }

    // T3：+ 指标中间件。
    #[cfg(feature = "metrics")]
    {
        let mut router = sdforge::http::build_with_config(&base_config()).unwrap();
        group.bench_function("t3_metrics", |b| {
            b.iter(|| call_ping(&rt, &mut router));
        });
    }

    // T4：+ context 中间件（request_id + trace_id task-local）。
    #[cfg(feature = "context")]
    {
        let mut router = sdforge::http::build_with_config(&base_config()).unwrap();
        group.bench_function("t4_context", |b| {
            b.iter(|| call_ping(&rt, &mut router));
        });
    }

    // T5：+ CORS。
    {
        let mut config = base_config();
        config.server.cors = Some(sdforge::config::CorsConfig {
            allowed_origins: vec!["https://bench.example".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["content-type".to_string()],
        });
        let mut router = sdforge::http::build_with_config(&config).unwrap();
        group.bench_function("t5_cors", |b| {
            b.iter(|| call_ping(&rt, &mut router));
        });
    }

    // T6：全叠加（feature 齐全时 = auth + etag + metrics + context + cors
    // + idempotency enabled 于恒装层之上）。
    #[cfg(all(
        feature = "security",
        feature = "etag",
        feature = "metrics",
        feature = "context",
        feature = "idempotency"
    ))]
    {
        let mut config = base_config();
        config.authentication = AuthConfig::ApiKey {
            header_name: "x-api-key".to_string(),
            prefix: "bench-".to_string(),
            keys: vec![ApiKeySeed {
                key: "bench-key".to_string(),
                // 空 permissions 会被 validate_key 判为无效（None）→ 401。
                permissions: vec!["bench".to_string()],
            }],
        };
        config.server.cors = Some(sdforge::config::CorsConfig {
            allowed_origins: vec!["https://bench.example".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["content-type".to_string()],
        });
        config.server.idempotency.enabled = true;
        let mut router = sdforge::http::build_with_config(&config).unwrap();
        group.bench_function("t6_all_layers", |b| {
            b.iter(|| authed_ping(&rt, &mut router));
        });
    }

    group.finish();
}

/// i18n 翻译查表微基准（`i18n` 模块无条件编译，无 feature 门控）：
/// `translate_or_fallback` 每次调用为 get_locale 快照 + 注册表查表（两次
/// Mutex 临界区）+ 分配——被 MCP 工具列表 / CLI help / OpenAPI 每路由 /
/// gRPC GetInfo 按描述条目调用。数据记录在 `docs/PERFORMANCE.md`。
fn bench_i18n_translation(c: &mut Criterion) {
    sdforge::i18n::clear_translations();

    // 回退臂：未注册键 → 纯锁 + 分配（多数 OpenAPI 路由无 i18n_key 时的
    // 实际形态）。
    let mut group = c.benchmark_group("i18n_translation");
    group.throughput(criterion::Throughput::Elements(1));

    group.bench_function("fallback_unregistered_key", |b| {
        b.iter(|| {
            let s = sdforge::i18n::translate_or_fallback(
                "Static English description",
                Some("bench.unregistered.key"),
            );
            std::hint::black_box(s);
        });
    });

    // 命中臂：注册表非空（1e4 宿主注册项，模拟大路由量）+ 命中键。
    for i in 0..10_000u32 {
        sdforge::i18n::register_translation("en", &format!("bench.hit.key.{i}"), "translated");
    }
    sdforge::i18n::register_translation("en", "bench.hit.key", "translated description");

    group.bench_function("registry_hit_10k_entries", |b| {
        b.iter(|| {
            let s = sdforge::i18n::translate_or_fallback(
                "Static English description",
                Some("bench.hit.key"),
            );
            std::hint::black_box(s);
        });
    });

    group.finish();
    sdforge::i18n::clear_translations();
}

/// gRPC server-streaming 基线（`grpc` + `streaming`）：分发 + 逐项映射的
/// 端到端耗时与每项序列化映射开销。`grpc`/`streaming` 未启用时空实现
/// 占位——`--features http` 调用路径必须可编译（同 TLS 组的存根约定）。
#[cfg(not(all(feature = "grpc", feature = "streaming")))]
fn bench_grpc_streaming(_: &mut Criterion) {}

#[cfg(all(feature = "grpc", feature = "streaming"))]
fn bench_grpc_streaming(c: &mut Criterion) {
    use sdforge::grpc::sdforge_v1::sd_forge_service_server::SdForgeService;
    use sdforge::grpc::stream_output_from;
    use sdforge::streaming::create_stream_channel;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    fn count_stream_handler(
        args: sdforge::core::HandlerArgs,
        _state: sdforge::core::HandlerState,
    ) -> sdforge::grpc::GrpcStreamHandlerFuture {
        let count: u64 = args.get("count").and_then(|s| s.parse().ok()).unwrap_or(10);
        let (tx, response) = create_stream_channel::<serde_json::Value>(32);
        tokio::spawn(async move {
            for i in 0..count {
                if tx
                    .send(Ok(serde_json::json!({ "seq": i, "payload": "bench-item" })))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        Box::pin(async move { Ok(stream_output_from(response)) })
    }

    fn stream_request(count: u64) -> tonic::Request<sdforge::grpc::sdforge_v1::CallRequest> {
        let mut parameters = std::collections::HashMap::new();
        parameters.insert("count".to_string(), count.to_string());
        tonic::Request::new(sdforge::grpc::sdforge_v1::CallRequest {
            method: "bench_stream".to_string(),
            parameters,
            data: String::new(),
        })
    }

    // 流式 handler 手工注册（bench 内不经宏，直接对齐 inventory 契约）。
    inventory::submit! {
        sdforge::grpc::GrpcStreamHandlerRegistration {
            method: "bench_stream",
            handler: count_stream_handler,
            body_param: None,
            default_status: None,
            roles: &[],
            i18n_key: None,
            deprecated: false,
            sunset: None,
            successor: None,
        }
    }

    let service = sdforge::grpc::SdForgeGrpcService::default();
    #[allow(deprecated)]
    let _ = &service;

    let mut group = c.benchmark_group("grpc_stream");
    group.throughput(criterion::Throughput::Elements(1));

    // 端到端分发：call_stream 守卫链 + handler + 逐项映射 + 收流。
    for items in [10u64, 100, 1000] {
        group.bench_function(format!("call_stream_{items}_items"), |b| {
            b.iter(|| {
                rt.block_on(async {
                    let response = service
                        .call_stream(stream_request(items))
                        .await
                        .expect("dispatch");
                    use tokio_stream::StreamExt;
                    let mut n = 0u64;
                    let mut stream = response.into_inner();
                    while let Some(item) = stream.next().await {
                        let resp = item.expect("item ok");
                        assert!(resp.success);
                        n += 1;
                    }
                    assert_eq!(n, items);
                });
            });
        });
    }

    // 每项映射开销（stream_output_from 消费 1e4 项，不经 gRPC 分发层）。
    group.bench_function("item_mapping_10k", |b| {
        b.iter(|| {
            let mut total = 0usize;
            rt.block_on(async {
                use tokio_stream::StreamExt;
                // 独立构造 1e4 项的流并经 stream_output_from 逐项消费。
                let (tx, rx) =
                    tokio::sync::mpsc::channel::<Result<serde_json::Value, String>>(1024);
                tokio::spawn(async move {
                    for i in 0..10_000u64 {
                        let _ = tx.send(Ok(serde_json::json!({ "seq": i }))).await;
                    }
                });
                let mapped =
                    sdforge::grpc::stream_output_from(sdforge::streaming::StreamResponse::new(
                        tokio_stream::wrappers::ReceiverStream::new(rx),
                    ));
                let mut stream = mapped.stream;
                while let Some(item) = stream.next().await {
                    let _ = item.expect("item ok");
                    total += 1;
                }
            });
            assert_eq!(total, 10_000);
        });
    });

    group.finish();
}
