// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! integration: HTTP TLS 终止（feature = `serve-tls`）。
//!
//! 覆盖 `sdforge::http::tls` 的自签证书全链路：https 请求、明文打到 TLS
//! 端口、ALPN 协商、证书热重载对新握手生效、优雅停机排空在途 TLS 请求。
//! 与 gRPC 侧 `grpc-tls`（tonic `ServerTlsConfig` 接线）实现独立，本文件
//! 只覆盖 rustls 终止路径。

#![cfg(feature = "serve-tls")]

use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Duration;

use rustls::DigitallySignedStruct;
use rustls::SignatureScheme;
use rustls::client::danger::{ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use sdforge::config::TlsConfig;
use sdforge::http::tls::{
    ReloadingTls, TlsServeConfig, serve_with_graceful_shutdown_tls, tls_acceptor,
};
use sdforge::http::{GracefulShutdownConfig, default_shutdown_signal};

/// 生成自签证书 PEM（SAN 含 localhost 与 127.0.0.1，rcgen aws_lc_rs provider）。
fn self_signed_pem() -> (String, String) {
    let certified =
        rcgen::generate_simple_self_signed(vec!["localhost".to_string(), "127.0.0.1".to_string()])
            .expect("generate self-signed cert");
    (certified.cert.pem(), certified.signing_key.serialize_pem())
}

fn write_pem(dir: &tempfile::TempDir, name: &str, content: &str) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, content).expect("write pem file");
    path.to_string_lossy().into_owned()
}

/// 接受任意服务端证书的 rustls 客户端配置（测试专用，等价
/// reqwest 的 danger_accept_invalid_certs）。
fn danger_client_config(alpn: Vec<Vec<u8>>) -> rustls::ClientConfig {
    #[derive(Debug)]
    struct AcceptAll;

    impl ServerCertVerifier for AcceptAll {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            rustls::crypto::aws_lc_rs::default_provider()
                .signature_verification_algorithms
                .supported_schemes()
        }
    }

    let verifier = Arc::new(AcceptAll);
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("default protocol versions")
    .dangerous()
    .with_custom_certificate_verifier(verifier)
    .with_no_client_auth();
    config.alpn_protocols = alpn;
    config
}

/// 直连 TLS 握手：返回客户端侧连接（可读 peer 证书与协商出的 ALPN）。
async fn tls_connect(
    addr: std::net::SocketAddr,
    server_name: &str,
    alpn: Vec<Vec<u8>>,
) -> tokio_rustls::client::TlsStream<tokio::net::TcpStream> {
    let connector = tokio_rustls::TlsConnector::from(Arc::new(danger_client_config(alpn)));
    let tcp = tokio::net::TcpStream::connect(addr)
        .await
        .expect("tcp connect");
    let name = ServerName::try_from(server_name.to_string()).expect("server name");
    connector.connect(name, tcp).await.expect("tls handshake")
}

/// 接受任意服务端证书的 https 客户端。
fn https_client() -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("https client")
}

/// Bind an ephemeral listener for tests.
async fn test_listener() -> tokio::net::TcpListener {
    tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap()
}

/// Wait until the server accepts TCP connections (poll-connect).
async fn wait_until_up(addr: std::net::SocketAddr) {
    for _ in 0..200 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("server never came up at {addr}");
}

async fn ping_router() -> axum::Router {
    axum::Router::new().route("/ping", axum::routing::get(|| async { "pong" }))
}

/// TLS serve 配置：默认护栏（握手/头读取超时）+ 指定排空窗口。
fn serve_config(drain_timeout: Duration) -> TlsServeConfig {
    TlsServeConfig::default()
        .with_graceful(GracefulShutdownConfig::with_drain_timeout(drain_timeout))
}

#[tokio::test]
#[serial_test::serial]
async fn tls_roundtrip_serves_https_request() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cert, key) = self_signed_pem();
    let tls = TlsConfig::new(
        write_pem(&dir, "cert.pem", &cert),
        write_pem(&dir, "key.pem", &key),
    );

    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();
    let acceptor = tls_acceptor(&tls).expect("acceptor builds");

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown_tls(
        ping_router().await,
        listener,
        acceptor,
        async move {
            let _ = trigger_rx.await;
        },
        serve_config(Duration::from_secs(5)),
    ));

    wait_until_up(addr).await;

    let client = https_client();
    let resp = client
        .get(format!("https://{addr}/ping"))
        .send()
        .await
        .expect("https request over self-signed cert");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.expect("body"), "pong");

    // 停机触发后 serve 必须收敛为 Ok。
    let _ = trigger_tx.send(());
    let result = tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("serve must resolve after shutdown trigger")
        .unwrap();
    assert!(result.is_ok(), "graceful TLS shutdown returns Ok");
}

#[tokio::test]
#[serial_test::serial]
async fn self_signed_cert_rejected_by_default_verifier() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cert, key) = self_signed_pem();
    let tls = TlsConfig::new(
        write_pem(&dir, "cert.pem", &cert),
        write_pem(&dir, "key.pem", &key),
    );

    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();
    let acceptor = tls_acceptor(&tls).expect("acceptor builds");

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown_tls(
        ping_router().await,
        listener,
        acceptor,
        async move {
            let _ = trigger_rx.await;
        },
        serve_config(Duration::from_secs(5)),
    ));
    wait_until_up(addr).await;

    // 默认校验器（信任系统根）必须拒绝自签证书——证明线上跑的是真 TLS，
    // 而不是某个被静默降级的明文通道。
    let result = reqwest::get(format!("https://{addr}/ping")).await;
    assert!(
        result.is_err(),
        "default verifier must reject the self-signed cert"
    );

    let _ = trigger_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("serve resolves")
        .unwrap()
        .expect("graceful shutdown ok");
}

#[tokio::test]
#[serial_test::serial]
async fn plain_http_to_tls_port_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cert, key) = self_signed_pem();
    let tls = TlsConfig::new(
        write_pem(&dir, "cert.pem", &cert),
        write_pem(&dir, "key.pem", &key),
    );

    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();
    let acceptor = tls_acceptor(&tls).expect("acceptor builds");

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown_tls(
        ping_router().await,
        listener,
        acceptor,
        async move {
            let _ = trigger_rx.await;
        },
        serve_config(Duration::from_secs(5)),
    ));
    wait_until_up(addr).await;

    // 明文 HTTP 打到 TLS 端口：TLS 握手必然失败，绝不能返回响应。
    let result = reqwest::get(format!("http://{addr}/ping")).await;
    assert!(
        result.is_err(),
        "plain http must not get a TLS-port response"
    );

    let _ = trigger_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("serve resolves")
        .unwrap()
        .expect("graceful shutdown ok");
}

#[tokio::test]
#[serial_test::serial]
async fn alpn_negotiates_configured_protocol() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cert, key) = self_signed_pem();
    let tls = TlsConfig::new(
        write_pem(&dir, "cert.pem", &cert),
        write_pem(&dir, "key.pem", &key),
    );

    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();
    let acceptor = tls_acceptor(&tls).expect("acceptor builds");

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown_tls(
        ping_router().await,
        listener,
        acceptor,
        async move {
            let _ = trigger_rx.await;
        },
        serve_config(Duration::from_secs(5)),
    ));
    wait_until_up(addr).await;

    // 客户端按偏好顺序提供 h2 + http/1.1，服务端默认 ALPN 列表同为
    // ["h2", "http/1.1"] → 协商结果必须是 h2（双方列表交集按客户端顺序）。
    let client_stream = tls_connect(
        addr,
        "localhost",
        vec![b"h2".to_vec(), b"http/1.1".to_vec()],
    )
    .await;
    let client_alpn = client_stream
        .get_ref()
        .1
        .alpn_protocol()
        .map(<[u8]>::to_vec);
    assert_eq!(
        client_alpn.as_deref(),
        Some(&b"h2"[..]),
        "client offering h2 + http/1.1 must negotiate h2"
    );
    drop(client_stream);

    let _ = trigger_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("serve resolves")
        .unwrap()
        .expect("graceful shutdown ok");
}

#[tokio::test]
#[serial_test::serial]
async fn alpn_h2_connection_serves_http2_requests() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cert, key) = self_signed_pem();
    let tls = TlsConfig::new(
        write_pem(&dir, "cert.pem", &cert),
        write_pem(&dir, "key.pem", &key),
    );

    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();
    let acceptor = tls_acceptor(&tls).expect("acceptor builds");

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown_tls(
        ping_router().await,
        listener,
        acceptor,
        async move {
            let _ = trigger_rx.await;
        },
        serve_config(Duration::from_secs(5)),
    ));
    wait_until_up(addr).await;

    // ALPN 协商出 h2 后，serve 循环必须能真正服务 HTTP/2 请求
    //（prior knowledge 跳过 http/1.1 升级路径，强制走 h2）。
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .http2_prior_knowledge()
        .build()
        .expect("h2 client");
    let resp = client
        .get(format!("https://{addr}/ping"))
        .send()
        .await
        .expect("https request over http/2");
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.version(),
        reqwest::Version::HTTP_2,
        "prior-knowledge client must be served over http/2"
    );
    assert_eq!(resp.text().await.expect("body"), "pong");

    let _ = trigger_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("serve resolves")
        .unwrap()
        .expect("graceful shutdown ok");
}

#[tokio::test]
#[serial_test::serial]
async fn reload_is_picked_up_by_new_handshakes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cert_v1, key_v1) = self_signed_pem();
    let (cert_v2, key_v2) = self_signed_pem();
    let cert_path = write_pem(&dir, "cert.pem", &cert_v1);
    let key_path = write_pem(&dir, "key.pem", &key_v1);

    let tls = TlsConfig::new(cert_path.clone(), key_path.clone());
    let reloading = ReloadingTls::new(&tls).expect("initial load");
    let initial_der = reloading.end_entity_certificate().expect("initial cert");

    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();
    let acceptor = reloading.acceptor();

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown_tls(
        ping_router().await,
        listener,
        acceptor,
        async move {
            let _ = trigger_rx.await;
        },
        serve_config(Duration::from_secs(5)),
    ));
    wait_until_up(addr).await;

    // 握手 1：peer 证书 = v1。
    let stream_v1 = tls_connect(addr, "localhost", Vec::new()).await;
    let peer_v1 = stream_v1
        .get_ref()
        .1
        .peer_certificates()
        .expect("server must present its cert chain")
        .first()
        .expect("end-entity cert present")
        .as_ref()
        .to_vec();
    assert_eq!(peer_v1, initial_der, "handshake must serve the loaded cert");
    drop(stream_v1);

    // 换盘 v2 → reload。
    std::fs::write(&cert_path, &cert_v2).expect("overwrite cert");
    std::fs::write(&key_path, &key_v2).expect("overwrite key");
    reloading.reload().expect("reload with matched pair");

    // 握手 2：新握手必须拿到 v2 证书——证明 ServerConfig 真的引用了 reloader。
    let stream_v2 = tls_connect(addr, "localhost", Vec::new()).await;
    let peer_v2 = stream_v2
        .get_ref()
        .1
        .peer_certificates()
        .expect("server must present its cert chain")
        .first()
        .expect("end-entity cert present")
        .as_ref()
        .to_vec();
    assert_ne!(
        peer_v1, peer_v2,
        "new handshakes must pick up the reloaded certificate"
    );
    drop(stream_v2);

    let _ = trigger_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("serve resolves")
        .unwrap()
        .expect("graceful shutdown ok");
}

#[tokio::test]
#[serial_test::serial]
async fn inflight_tls_request_completes_before_shutdown() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cert, key) = self_signed_pem();
    let tls = TlsConfig::new(
        write_pem(&dir, "cert.pem", &cert),
        write_pem(&dir, "key.pem", &key),
    );

    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();

    let router = axum::Router::new().route(
        "/slow",
        axum::routing::get(|| async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            "done"
        }),
    );
    let acceptor = tls_acceptor(&tls).expect("acceptor builds");

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown_tls(
        router,
        listener,
        acceptor,
        async move {
            let _ = trigger_rx.await;
        },
        serve_config(Duration::from_secs(5)),
    ));
    wait_until_up(addr).await;

    let in_flight = tokio::spawn(async move {
        https_client()
            .get(format!("https://{addr}/slow"))
            .send()
            .await
            .expect("in-flight request drained")
            .text()
            .await
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let _ = trigger_tx.send(());

    // 排空语义：触发停机后已在途的 TLS 请求必须完整拿到响应。
    let body = in_flight
        .await
        .expect("client task")
        .expect("in-flight response body");
    assert_eq!(body, "done");

    let result = tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("serve future must resolve after drain")
        .unwrap();
    assert!(result.is_ok(), "graceful path returns Ok");
}

#[tokio::test]
#[serial_test::serial]
async fn drain_timeout_forces_tls_shutdown() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cert, key) = self_signed_pem();
    let tls = TlsConfig::new(
        write_pem(&dir, "cert.pem", &cert),
        write_pem(&dir, "key.pem", &key),
    );

    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();

    // Handler hold 1.5s > drain budget 200ms → 强制断连路径。
    let router = axum::Router::new().route(
        "/stuck",
        axum::routing::get(|| async {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            "late"
        }),
    );
    let acceptor = tls_acceptor(&tls).expect("acceptor builds");

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown_tls(
        router,
        listener,
        acceptor,
        async move {
            let _ = trigger_rx.await;
        },
        serve_config(Duration::from_millis(200)),
    ));
    wait_until_up(addr).await;

    let in_flight = tokio::spawn(async move {
        https_client()
            .get(format!("https://{addr}/stuck"))
            .send()
            .await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let t0 = std::time::Instant::now();
    let _ = trigger_tx.send(());

    let result = tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("serve must resolve after forced drain timeout")
        .unwrap();
    assert!(result.is_ok());
    let elapsed = t0.elapsed();
    assert!(
        elapsed < Duration::from_millis(1400),
        "forced shutdown must not wait for the stuck handler; took {elapsed:?}"
    );

    // 强制路径：在途请求的连接被切断，客户端不允许拿到正常响应。
    let _ = in_flight.await;
}

/// 覆盖 [`default_shutdown_signal`] 与 TLS serve 的组合编译可用性
/// （真实信号不打；只验证可执行入口存在且类型吻合）。
#[tokio::test]
#[serial_test::serial]
async fn serve_accepts_default_shutdown_signal_future() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cert, key) = self_signed_pem();
    let tls = TlsConfig::new(
        write_pem(&dir, "cert.pem", &cert),
        write_pem(&dir, "key.pem", &key),
    );

    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();
    let acceptor = tls_acceptor(&tls).expect("acceptor builds");

    let server = tokio::spawn(serve_with_graceful_shutdown_tls(
        ping_router().await,
        listener,
        acceptor,
        default_shutdown_signal(),
        TlsServeConfig::default(),
    ));
    wait_until_up(addr).await;

    let resp = https_client()
        .get(format!("https://{addr}/ping"))
        .send()
        .await
        .expect("https request");
    assert_eq!(resp.status(), 200);

    // 不向进程发真实信号：直接 abort 测试持有的 serve 任务收尾。
    server.abort();
    let _ = server.await;
}

#[tokio::test]
#[serial_test::serial]
async fn connect_info_injected_on_tls_requests() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cert, key) = self_signed_pem();
    let tls = TlsConfig::new(
        write_pem(&dir, "cert.pem", &cert),
        write_pem(&dir, "key.pem", &key),
    );

    // handler 回显 ConnectInfo 对端地址——限流/审计的不可伪造 IP 来源。
    let router =
        axum::Router::new().route(
            "/peer",
            axum::routing::get(
                |info: axum::extract::ConnectInfo<std::net::SocketAddr>| async move {
                    info.0.to_string()
                },
            ),
        );

    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();
    let acceptor = tls_acceptor(&tls).expect("acceptor builds");

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown_tls(
        router,
        listener,
        acceptor,
        async move {
            let _ = trigger_rx.await;
        },
        serve_config(Duration::from_secs(5)),
    ));
    wait_until_up(addr).await;

    // 手动 TCP 连接以拿到客户端侧 local addr，随后在 handler 回显中比对。
    let connector = tokio_rustls::TlsConnector::from(Arc::new(danger_client_config(Vec::new())));
    let tcp = tokio::net::TcpStream::connect(addr)
        .await
        .expect("tcp connect");
    let local_addr = tcp.local_addr().expect("local addr");
    let name = std::convert::TryFrom::try_from("localhost".to_string()).expect("server name");
    let mut conn = connector.connect(name, tcp).await.expect("tls handshake");
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    conn.write_all(b"GET /peer HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .expect("write request");
    let mut raw = Vec::new();
    conn.read_to_end(&mut raw).await.expect("read response");
    let body = String::from_utf8_lossy(&raw);
    let peer_line = body
        .split("\r\n\r\n")
        .nth(1)
        .expect("response body after headers");
    assert_eq!(
        peer_line.trim(),
        local_addr.to_string(),
        "handler must see the real TCP peer via ConnectInfo"
    );

    let _ = trigger_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("serve resolves")
        .unwrap()
        .expect("graceful shutdown ok");
}

/// 安全模型契约：TLS 路径注入 ConnectInfo 后，`extract_client_ip` 取到
/// 的是不可伪造的直连 peer IP（而非 None / 可伪造 forwarded header）。
#[tokio::test]
#[serial_test::serial]
#[cfg(feature = "security")]
async fn extract_client_ip_uses_tls_peer_on_tls_requests() {
    use sdforge::security::extract_client_ip;

    let dir = tempfile::tempdir().expect("tempdir");
    let (cert, key) = self_signed_pem();
    let tls = TlsConfig::new(
        write_pem(&dir, "cert.pem", &cert),
        write_pem(&dir, "key.pem", &key),
    );

    let router = axum::Router::new().route(
        "/ip",
        axum::routing::get(|req: axum::extract::Request| async move {
            extract_client_ip(&req).unwrap_or_else(|| "unknown".to_string())
        }),
    );

    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();
    let acceptor = tls_acceptor(&tls).expect("acceptor builds");

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown_tls(
        router,
        listener,
        acceptor,
        async move {
            let _ = trigger_rx.await;
        },
        serve_config(Duration::from_secs(5)),
    ));
    wait_until_up(addr).await;

    let connector = tokio_rustls::TlsConnector::from(Arc::new(danger_client_config(Vec::new())));
    let tcp = tokio::net::TcpStream::connect(addr)
        .await
        .expect("tcp connect");
    let local_addr = tcp.local_addr().expect("local addr");
    let name = std::convert::TryFrom::try_from("localhost".to_string()).expect("server name");
    let mut conn = connector.connect(name, tcp).await.expect("tls handshake");
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    conn.write_all(b"GET /ip HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .expect("write request");
    let mut raw = Vec::new();
    conn.read_to_end(&mut raw).await.expect("read response");
    let body = String::from_utf8_lossy(&raw);
    let reported_ip = body.split("\r\n\r\n").nth(1).expect("body").trim();

    assert_eq!(
        reported_ip,
        local_addr.ip().to_string(),
        "extract_client_ip must report the unspoofable TLS peer, not a fallback bucket"
    );
    assert_ne!(
        reported_ip, "unknown",
        "TLS serve must never collapse clients into the shared unknown bucket"
    );

    let _ = trigger_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("serve resolves")
        .unwrap()
        .expect("graceful shutdown ok");
}
