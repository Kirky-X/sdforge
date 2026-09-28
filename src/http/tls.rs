// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! HTTP TLS termination (feature = `serve-tls`).
//!
//! rustls（aws-lc-rs provider，与 `grpc-tls` 的 `tonic/tls-aws-lc` 同一密码学
//! 栈）在 sdforge 进程内终止 TLS：证书/密钥从 [`crate::config::TlsConfig`]
//! 指向的 PEM 文件加载，ALPN 列表可配置（缺省 `["h2", "http/1.1"]`），可选
//! [`ReloadingTls`] 运行时热重载。
//!
//! # 与 gRPC TLS 的关系
//!
//! gRPC 侧的对应物是 `crate::grpc::GrpcServerConfig::tls` 字段（feature =
//! `grpc-tls`，`tonic::transport::server::ServerTlsConfig` 接线）：只暴露
//! tonic 构建体，证书加载由调用方负责；HTTP 侧由本模块完成
//! 加载 → `ServerConfig` → serve 全链路。两者文档互链、实现独立，可各自
//! 单独启用。
//!
//! # Serve 语义（与明文路径的对应关系）
//!
//! [`serve_with_graceful_shutdown_tls`] 的停机编排与
//! `crate::http::graceful::serve_graceful` 同构（停止接新 → 排空在途 →
//! 停止钩子），两者必须同步演化——排空语义调整时须同时修改两处，差异仅在
//! 传输层（TLS accept 循环 vs `axum::serve`）。两个有意分叉：
//!
//! - **ConnectInfo 默认注入**：TLS 直连无前置代理，peer 地址即客户端来源，
//!   每个请求都注入 `ConnectInfo<SocketAddr>` 扩展（对齐明文家族的
//!   `_connect_info` 变体）。这是限流/审计取不可伪造客户端 IP 的前提
//!   （`crate::security::ip_util` 的安全模型），因此不设「不注入」的
//!   默认变体，也没有单独的 `_connect_info` 后缀变体。
//! - **TCP accept 错误不终止 serve**（镜像 `axum::serve` 的
//!   `handle_accept_error`）：连接类错误静默重试，其余错误（如 EMFILE）
//!   记 error 后退避 1s 继续——过载自愈，不停服。
//!
//! `after_drain` 钩子变体暂未提供：TLS 循环为自研 accept loop，需要
//! 排空后钩子的调用方可参照 `graceful.rs::serve_graceful` 的模式扩展本
//! 模块（接口面积控制，不预置未验证的组合）。
//!
//! drop/abort serve future 不会回收停机触发器的派生任务（持有关闭的
//! shutdown future 直至其完成）与已建立的连接任务（JoinSet drop 即
//! detach）——进程退出场景无影响，测试中 abort serve 时需自行容忍。
//!
//! # Example
//!
//! ```ignore
//! use sdforge::config::TlsConfig;
//! use sdforge::http::{default_shutdown_signal, GracefulShutdownConfig};
//! use sdforge::http::tls::{serve_with_graceful_shutdown_tls, tls_acceptor, TlsServeConfig};
//!
//! let tls = TlsConfig::new("cert.pem", "key.pem");
//! let acceptor = tls_acceptor(&tls)?;
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:8443").await?;
//! let config = TlsServeConfig::default()
//!     .with_graceful(GracefulShutdownConfig::default());
//! serve_with_graceful_shutdown_tls(
//!     router,
//!     listener,
//!     acceptor,
//!     default_shutdown_signal(),
//!     config,
//! ).await?;
//! ```

use std::sync::{Arc, RwLock};
use std::time::Duration;

use rustls::ServerConfig;
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use tower::ServiceExt;

use crate::config::TlsConfig;
use crate::http::graceful::{GracefulShutdownConfig, run_stop_phase};

/// tokio-rustls acceptor 再导出：调用方无需直接依赖 `tokio-rustls`。
pub use tokio_rustls::TlsAcceptor;

/// TLS 握手默认超时：半开连接/慢握手（TLS 层 slowloris）在窗口后关闭。
const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// HTTP 请求头读取默认超时（对齐 hyper-util 当前默认；覆盖建连后慢速
/// 发送请求头的在途连接——TimeoutLayer 只覆盖已到达的请求）。
const DEFAULT_HEADER_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// TCP accept 非连接类错误的退避间隔（镜像 axum `handle_accept_error`）。
const ACCEPT_RETRY_DELAY: Duration = Duration::from_secs(1);

/// 握手失败日志聚合窗口：认证前可任意触发的事件不得以每连接一条的频率
/// 进 warn 通道；窗口内失败记 debug，窗口边界一条 warn 汇总被抑制条数。
const HANDSHAKE_FAILURE_LOG_WINDOW: Duration = Duration::from_secs(60);

/// TLS 装配错误。
///
/// 区分三类失败面：文件读取（IO）、PEM 解析（内容非证书/密钥）、
/// rustls 装配（证书与密钥不匹配等）。
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    /// 证书文件读取失败（缺失/无权限）。
    #[error("failed to read TLS certificate file {path}: {source}")]
    CertRead {
        /// 证书文件路径。
        path: String,
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },
    /// 私钥文件读取失败（缺失/无权限）。
    #[error("failed to read TLS private key file {path}: {source}")]
    KeyRead {
        /// 私钥文件路径。
        path: String,
        /// 底层 IO 错误。
        #[source]
        source: std::io::Error,
    },
    /// PEM 内容解析失败（不是合法 PEM / 编码损坏）。
    #[error("failed to parse TLS PEM data from {path}: {source}")]
    Pem {
        /// 出错的文件路径。
        path: String,
        /// 底层 PEM 解析错误。
        #[source]
        source: rustls_pki_types::pem::Error,
    },
    /// 证书文件中没有任何证书。
    #[error("no certificate found in {path}")]
    NoCertificate {
        /// 证书文件路径。
        path: String,
    },
    /// 私钥文件中没有受支持的私钥分节。
    #[error("no supported private key found in {path}")]
    NoPrivateKey {
        /// 私钥文件路径。
        path: String,
    },
    /// rustls 配置装配失败（证书与密钥不匹配、协议版本协商失败等）。
    #[error("failed to build rustls server config: {0}")]
    Config(#[from] rustls::Error),
    /// 重载任务执行失败（panic 或运行时关闭）。
    #[error("certificate reload task failed: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
}

/// 密码学 provider：aws-lc-rs，与 `grpc-tls`（`tonic/tls-aws-lc`）同栈。
/// 显式注入 builder，不依赖进程级 provider 全局解析。
fn tls_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::aws_lc_rs::default_provider())
}

/// 窗口判定（纯函数便于测试）：`last == 0` 视为首次（立即 warn），
/// 距上次 warn 超过一个窗口再次 warn。
fn handshake_failure_should_warn(now_secs: u64, last_warn_secs: u64) -> bool {
    last_warn_secs == 0
        || now_secs.saturating_sub(last_warn_secs) >= HANDSHAKE_FAILURE_LOG_WINDOW.as_secs()
}

/// 握手失败日志：窗口内降级 debug 并计数，窗口边界一条 warn 汇总抑制量。
fn log_handshake_failure(err: &std::io::Error) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST_WARN_SECS: AtomicU64 = AtomicU64::new(0);
    static SUPPRESSED: AtomicU64 = AtomicU64::new(0);

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let last = LAST_WARN_SECS.load(Ordering::Relaxed);
    if handshake_failure_should_warn(now_secs, last) {
        let suppressed = SUPPRESSED.swap(0, Ordering::Relaxed);
        if suppressed > 0 {
            log::warn!(
                "TLS handshake failed: {err} ({suppressed} similar failures suppressed in the last window)"
            );
        } else {
            log::warn!("TLS handshake failed: {err}");
        }
        LAST_WARN_SECS.store(now_secs, Ordering::Relaxed);
    } else {
        SUPPRESSED.fetch_add(1, Ordering::Relaxed);
        log::debug!("TLS handshake failed: {err}");
    }
}

/// unix 下私钥文件 group/other 权限位是否非零（仅告警不改行为，避免
/// 破坏挂载卷等合法场景）。误权限是 TLS 部署最常见事故之一，失败必须
/// 可见而非静默。
#[cfg(unix)]
fn key_file_permissions_exposed(path: &str) -> std::io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path)?.permissions().mode();
    Ok(mode & 0o077 != 0)
}

/// 从 PEM 文件加载证书链；区分文件读取失败与内容解析失败。
fn load_certs(path: &str) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    fn map_pem(path: &str, err: rustls_pki_types::pem::Error) -> TlsError {
        match err {
            rustls_pki_types::pem::Error::Io(io) => TlsError::CertRead {
                path: path.to_string(),
                source: io,
            },
            other => TlsError::Pem {
                path: path.to_string(),
                source: other,
            },
        }
    }

    let iter = CertificateDer::pem_file_iter(path).map_err(|err| map_pem(path, err))?;
    let mut certs = Vec::new();
    for cert in iter {
        certs.push(cert.map_err(|err| map_pem(path, err))?);
    }
    if certs.is_empty() {
        return Err(TlsError::NoCertificate {
            path: path.to_string(),
        });
    }
    Ok(certs)
}

/// 从 PEM 文件加载私钥（自动识别 PKCS8 / PKCS1 / SEC1）。
///
/// unix 下额外检查文件权限：group/other 可读时打 warn（建议 `chmod 600`），
/// 不拒绝加载。
fn load_key(path: &str) -> Result<PrivateKeyDer<'static>, TlsError> {
    #[cfg(unix)]
    if let Ok(true) = key_file_permissions_exposed(path) {
        log::warn!("TLS private key file {path} is readable by group/other; `chmod 600` it");
    }

    PrivateKeyDer::from_pem_file(path).map_err(|err| match err {
        rustls_pki_types::pem::Error::Io(io) => TlsError::KeyRead {
            path: path.to_string(),
            source: io,
        },
        rustls_pki_types::pem::Error::NoItemsFound => TlsError::NoPrivateKey {
            path: path.to_string(),
        },
        other => TlsError::Pem {
            path: path.to_string(),
            source: other,
        },
    })
}

/// 按 `TlsConfig` 配置宣告 ALPN（TLS wire 为单字节长度前缀 + 协议名）。
fn apply_alpn(config: &mut ServerConfig, tls: &TlsConfig) {
    config.alpn_protocols = tls
        .alpn_protocols()
        .iter()
        .map(|protocol| protocol.as_bytes().to_vec())
        .collect();
}

/// 从 [`TlsConfig`] 一次性加载证书/密钥，构建含 ALPN 的 rustls
/// `ServerConfig`（静态证书解析）。
///
/// 证书与密钥在此处完成匹配校验：不匹配的密钥对在装配期即报
/// [`TlsError::Config`]，而不是等第一个客户端握手失败。
pub fn load_server_config(tls: &TlsConfig) -> Result<ServerConfig, TlsError> {
    let certs = load_certs(tls.cert_path())?;
    let key = load_key(tls.key_path())?;
    let mut config = ServerConfig::builder_with_provider(tls_provider())
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    apply_alpn(&mut config, tls);
    Ok(config)
}

/// [`TlsAcceptor`] 便捷构建：静态证书，内部复用 [`load_server_config`]。
///
/// 需要运行时热重载时改用 [`ReloadingTls::new`]。
pub fn tls_acceptor(tls: &TlsConfig) -> Result<TlsAcceptor, TlsError> {
    Ok(TlsAcceptor::from(Arc::new(load_server_config(tls)?)))
}

/// TLS serve 配置：预认证攻击面（握手 / 头读取）的超时护栏 + 复用
/// graceful 的排空参数。
#[derive(Debug, Clone)]
pub struct TlsServeConfig {
    /// TLS 握手超时（默认 10s）：不完成握手的连接在窗口后关闭，握手
    /// 成本型 DoS（慢握手占任务与缓冲、未认证 CPU 开销）被限制在窗口内。
    pub handshake_timeout: Duration,
    /// HTTP 请求头读取超时（默认 30s）：建连后慢速发送请求头的连接被
    /// 关闭（需向 hyper 连接构建器注册 timer 才生效，本模块已注册）。
    pub header_read_timeout: Duration,
    /// 优雅停机排空参数（与明文路径同语义）。
    pub graceful: GracefulShutdownConfig,
}

impl Default for TlsServeConfig {
    fn default() -> Self {
        Self {
            handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
            header_read_timeout: DEFAULT_HEADER_READ_TIMEOUT,
            graceful: GracefulShutdownConfig::default(),
        }
    }
}

impl TlsServeConfig {
    /// 覆盖优雅停机排空参数（builder style）。
    pub fn with_graceful(mut self, graceful: GracefulShutdownConfig) -> Self {
        self.graceful = graceful;
        self
    }

    /// 覆盖 TLS 握手超时（builder style）。
    pub fn with_handshake_timeout(mut self, timeout: Duration) -> Self {
        self.handshake_timeout = timeout;
        self
    }
}

/// 支持运行时热重载的服务端证书解析器（内部类型，经 [`ReloadingTls`] 使用）。
struct CertReloader {
    cert_path: String,
    key_path: String,
    current: RwLock<Arc<CertifiedKey>>,
}

impl std::fmt::Debug for CertReloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // CertifiedKey 无 Debug 实现：只暴露可诊断的路径与证书链长度。
        let chain_len = self
            .current
            .read()
            .map(|certified| certified.cert.len())
            .unwrap_or_default();
        f.debug_struct("CertReloader")
            .field("cert_path", &self.cert_path)
            .field("key_path", &self.key_path)
            .field("cert_chain_len", &chain_len)
            .finish()
    }
}

/// 加载证书/密钥为 [`CertifiedKey`]（含证书-密钥匹配校验）。
fn load_certified_key(cert_path: &str, key_path: &str) -> Result<Arc<CertifiedKey>, TlsError> {
    let certs = load_certs(cert_path)?;
    let key = load_key(key_path)?;
    Ok(Arc::new(CertifiedKey::from_der(
        certs,
        key,
        &tls_provider(),
    )?))
}

impl CertReloader {
    /// 加载证书/密钥并构建接线好的 `(ServerConfig, CertReloader)`。
    fn build(tls: &TlsConfig) -> Result<(ServerConfig, Arc<Self>), TlsError> {
        let current = load_certified_key(tls.cert_path(), tls.key_path())?;
        let reloader = Arc::new(Self {
            cert_path: tls.cert_path().to_string(),
            key_path: tls.key_path().to_string(),
            current: RwLock::new(current),
        });
        let mut config = ServerConfig::builder_with_provider(tls_provider())
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_cert_resolver(reloader.clone());
        apply_alpn(&mut config, tls);
        Ok((config, reloader))
    }

    /// 重读磁盘证书/密钥并原子换入。
    ///
    /// 返回 [`TlsError`] 时当前证书保持不变。
    fn reload(&self) -> Result<(), TlsError> {
        let fresh = load_certified_key(&self.cert_path, &self.key_path)?;
        let mut current = match self.current.write() {
            Ok(guard) => guard,
            // 写侧 panic 过的锁数据依然可用：Arc 换入是单条赋值，取回即可。
            Err(poisoned) => poisoned.into_inner(),
        };
        *current = fresh;
        Ok(())
    }

    /// 当前终端实体证书的 DER 字节（重载结果观测/测试用）。
    fn end_entity_certificate(&self) -> Result<Vec<u8>, TlsError> {
        let guard = self.current.read().map_err(|poisoned| {
            TlsError::Config(rustls::Error::General(format!(
                "certificate resolver lock poisoned: {poisoned}"
            )))
        })?;
        Ok(guard.end_entity_cert()?.as_ref().to_vec())
    }
}

impl ResolvesServerCert for CertReloader {
    fn resolve(&self, _client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        let guard = match self.current.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        Some(guard.clone())
    }
}

/// 可热重载的 TLS 服务端：acceptor 与证书重载器**编译期绑定**。
///
/// serve 用 [`ReloadingTls::acceptor`] 取 acceptor，换盘后调
/// [`ReloadingTls::reload`]（或异步场景的 [`ReloadingTls::reload_async`]）——
/// acceptor 内部的 rustls config 持有 reloader 引用，不存在「config 与
/// reloader 拆开导致热重载静默失效」的错误形态。
///
/// 重载语义：原子换入当前证书，未完成的握手继续用旧证书，新握手立即用
/// 新证书；重载读到的密钥对必须与证书匹配，否则保留旧证书并返回错误。
pub struct ReloadingTls {
    acceptor: TlsAcceptor,
    reloader: Arc<CertReloader>,
}

impl std::fmt::Debug for ReloadingTls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReloadingTls")
            .field("reloader", &self.reloader)
            .finish()
    }
}

impl ReloadingTls {
    /// 加载证书/密钥并构建可热重载的 TLS 服务端。
    pub fn new(tls: &TlsConfig) -> Result<Self, TlsError> {
        let (config, reloader) = CertReloader::build(tls)?;
        Ok(Self {
            acceptor: TlsAcceptor::from(Arc::new(config)),
            reloader,
        })
    }

    /// serve 用的 acceptor 句柄（克隆；多实例共享同一重载状态）。
    pub fn acceptor(&self) -> TlsAcceptor {
        self.acceptor.clone()
    }

    /// 重读磁盘证书/密钥并原子换入。
    ///
    /// **阻塞当前线程**（两次文件读取 + PEM 解析 + 密钥装配）：适合
    /// 定时器线程/专用阻塞线程；tokio worker 上请用 [`ReloadingTls::reload_async`]。
    pub fn reload(&self) -> Result<(), TlsError> {
        self.reloader.reload()
    }

    /// 异步重载：`spawn_blocking` 包装同步 reload，不阻塞 tokio worker。
    pub async fn reload_async(&self) -> Result<(), TlsError> {
        let reloader = self.reloader.clone();
        tokio::task::spawn_blocking(move || reloader.reload())
            .await
            .map_err(TlsError::from)?
    }

    /// 当前终端实体证书的 DER 字节（重载结果观测/测试用）。
    pub fn end_entity_certificate(&self) -> Result<Vec<u8>, TlsError> {
        self.reloader.end_entity_certificate()
    }
}

/// 在 `listener` 上以 TLS 终止方式服务 `router`，复用优雅停机三阶段时序
/// （停止接新 → 排空在途 → 停止钩子），模块文档记载了与明文路径的
/// 语义对应关系与两处有意分叉。
///
/// `acceptor` 来自 [`tls_acceptor`]（静态证书）或 [`ReloadingTls::acceptor`]
/// （可热重载）。`shutdown` 为停机触发器（如
/// [`crate::http::default_shutdown_signal`]）；触发后停止 accept，在途
/// HTTP 请求排空至 `config.graceful.drain_timeout`，超时强制断连。
pub async fn serve_with_graceful_shutdown_tls(
    router: axum::Router,
    listener: tokio::net::TcpListener,
    acceptor: TlsAcceptor,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    config: TlsServeConfig,
) -> std::io::Result<()> {
    use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
    use hyper_util::server::conn::auto::Builder as HttpConnBuilder;

    // 停机触发经 watch 通道扇出：accept 循环与每条连接任务共用同一信号。
    let (trigger_tx, trigger_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        shutdown.await;
        let _ = trigger_tx.send(true);
    });

    // run on_start hooks before accepting connections.
    super::graceful::run_lifecycle_start_hooks().await;

    let mut connections = tokio::task::JoinSet::new();
    let mut accept_trigger = trigger_rx.clone();
    let handshake_timeout = config.handshake_timeout;
    let header_read_timeout = config.header_read_timeout;

    loop {
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((tcp_stream, peer_addr)) => {
                    // 小响应（<MSS）免 Nagle+延迟 ACK 的 ~40ms 尾延迟；
                    // 失败仅影响性能，不放大为错误。
                    let _ = tcp_stream.set_nodelay(true);
                    let acceptor = acceptor.clone();
                    let router = router.clone();
                    let mut shutdown_rx = trigger_rx.clone();
                    connections.spawn(async move {
                        // 预认证窗口护栏：慢握手（TLS 层 slowloris）在
                        // 窗口后关闭，任务与缓冲不被无限占用。
                        let tls_stream =
                            match tokio::time::timeout(handshake_timeout, acceptor.accept(tcp_stream))
                                .await
                            {
                                Ok(Ok(stream)) => stream,
                                Ok(Err(err)) => {
                                    log_handshake_failure(&err);
                                    return;
                                }
                                Err(_) => {
                                    log_handshake_failure(&std::io::Error::new(
                                        std::io::ErrorKind::TimedOut,
                                        "TLS handshake timed out",
                                    ));
                                    return;
                                }
                            };
                        // TLS 直连无前置代理，peer 地址即不可伪造的客户端
                        // 来源：每请求注入 ConnectInfo（限流/审计的 IP 依赖）。
                        let service = hyper::service::service_fn(
                            move |req: hyper::Request<hyper::body::Incoming>| {
                                let router = router.clone();
                                async move {
                                    let (parts, body) = req.into_parts();
                                    let mut req = hyper::Request::from_parts(
                                        parts,
                                        axum::body::Body::new(body),
                                    );
                                    req.extensions_mut()
                                        .insert(axum::extract::ConnectInfo(peer_addr));
                                    router.oneshot(req).await
                                }
                            },
                        );
                        // timer + header_read_timeout 覆盖建连后慢速发送
                        // 请求头的在途连接（h1/h2 各自注册 timer）。
                        let mut conn_builder = HttpConnBuilder::new(TokioExecutor::new());
                        let mut http1 = conn_builder.http1();
                        http1
                            .timer(TokioTimer::new())
                            .header_read_timeout(Some(header_read_timeout));
                        let mut http2 = http1.http2();
                        http2.timer(TokioTimer::new());
                        let conn =
                            http2.serve_connection_with_upgrades(TokioIo::new(tls_stream), service);
                        tokio::pin!(conn);
                        let stop = async {
                            loop {
                                if *shutdown_rx.borrow_and_update() {
                                    break;
                                }
                                if shutdown_rx.changed().await.is_err() {
                                    break;
                                }
                            }
                        };
                        tokio::select! {
                            result = &mut conn => {
                                if let Err(err) = result {
                                    log::debug!("TLS connection ended: {err}");
                                }
                            }
                            _ = stop => {
                                // 停机先到：让在途请求跑完、keep-alive 连接收尾。
                                conn.as_mut().graceful_shutdown();
                                if let Err(err) = conn.await {
                                    log::debug!("TLS connection during drain: {err}");
                                }
                            }
                        }
                    });
                }
                Err(err) => {
                    // 镜像 axum::serve 的 handle_accept_error：任何 accept
                    // 错误都不终止 serve——连接类错误静默重试，其余（如
                    // EMFILE，正是过载场景）error 后退避重试自愈。
                    if is_connection_error(&err) {
                        log::debug!("TCP accept connection error: {err}");
                    } else {
                        log::error!("TCP accept failed: {err}; retrying in 1s");
                        tokio::time::sleep(ACCEPT_RETRY_DELAY).await;
                    }
                }
            },
            _ = wait_for_trigger(&mut accept_trigger) => break,
        }
    }

    // 排空窗口：在途连接自然完成 vs deadline 到期强制断连。
    let drain = async { while connections.join_next().await.is_some() {} };
    tokio::select! {
        _ = drain => {}
        _ = tokio::time::sleep(config.graceful.drain_timeout) => {
            connections.abort_all();
        }
    }

    run_stop_phase(None).await;
    Ok(())
}

/// TCP accept 错误分类（镜像 axum `is_connection_error`）：对端侧抖动
/// （拒连/中断/重置）静默重试，其余进入退避。
fn is_connection_error(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
    )
}

/// 等待停机触发置位（值翻转或发送端消失都视为停机）。
async fn wait_for_trigger(rx: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        if *rx.borrow_and_update() {
            return;
        }
        if rx.changed().await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TlsConfig;

    /// 生成自签证书 PEM（rcgen aws_lc_rs provider，与运行时同栈）。
    fn self_signed_pem(common_name: &str) -> (String, String) {
        let certified = rcgen::generate_simple_self_signed(vec![common_name.to_string()])
            .expect("generate self-signed cert");
        (certified.cert.pem(), certified.signing_key.serialize_pem())
    }

    fn write_pem(dir: &tempfile::TempDir, name: &str, content: &str) -> String {
        let path = dir.path().join(name);
        std::fs::write(&path, content).expect("write pem file");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn load_server_config_reads_cert_key_and_applies_default_alpn() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (cert, key) = self_signed_pem("localhost");
        let tls = TlsConfig::new(
            write_pem(&dir, "cert.pem", &cert),
            write_pem(&dir, "key.pem", &key),
        );
        let config = load_server_config(&tls).expect("loads cert + key");
        assert_eq!(
            config.alpn_protocols,
            vec![b"h2".to_vec(), b"http/1.1".to_vec()],
            "default ALPN must be h2 + http/1.1"
        );
    }

    #[test]
    fn load_server_config_applies_custom_alpn_as_wire_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (cert, key) = self_signed_pem("localhost");
        let tls = TlsConfig::new(
            write_pem(&dir, "cert.pem", &cert),
            write_pem(&dir, "key.pem", &key),
        )
        .with_alpn_protocols(vec!["http/1.1".to_string()]);
        let config = load_server_config(&tls).expect("loads");
        assert_eq!(
            config.alpn_protocols,
            vec![b"http/1.1".to_vec()],
            "custom ALPN list must map to wire bytes verbatim"
        );
    }

    #[test]
    fn load_server_config_no_alpn_when_list_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (cert, key) = self_signed_pem("localhost");
        let tls = TlsConfig::new(
            write_pem(&dir, "cert.pem", &cert),
            write_pem(&dir, "key.pem", &key),
        )
        .with_alpn_protocols(Vec::new());
        let config = load_server_config(&tls).expect("loads");
        assert!(
            config.alpn_protocols.is_empty(),
            "empty ALPN list must disable ALPN advertisement"
        );
    }

    #[test]
    fn missing_cert_file_is_cert_read_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_, key) = self_signed_pem("localhost");
        let tls = TlsConfig::new(
            dir.path().join("absent.pem").to_string_lossy().into_owned(),
            write_pem(&dir, "key.pem", &key),
        );
        let err = load_server_config(&tls).expect_err("missing cert must fail");
        assert!(matches!(err, TlsError::CertRead { .. }), "got: {err:?}");
        assert!(err.to_string().contains("certificate"));
    }

    #[test]
    fn missing_key_file_is_key_read_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (cert, _) = self_signed_pem("localhost");
        let tls = TlsConfig::new(
            write_pem(&dir, "cert.pem", &cert),
            dir.path().join("absent.pem").to_string_lossy().into_owned(),
        );
        let err = load_server_config(&tls).expect_err("missing key must fail");
        assert!(matches!(err, TlsError::KeyRead { .. }), "got: {err:?}");
        assert!(err.to_string().contains("private key"));
    }

    #[test]
    fn non_pem_cert_file_is_no_certificate_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_, key) = self_signed_pem("localhost");
        let tls = TlsConfig::new(
            write_pem(&dir, "cert.pem", "definitely not a pem file"),
            write_pem(&dir, "key.pem", &key),
        );
        let err = load_server_config(&tls).expect_err("garbage cert must fail");
        assert!(
            matches!(err, TlsError::NoCertificate { .. }),
            "got: {err:?}"
        );
    }

    #[test]
    fn key_without_pem_sections_is_no_private_key_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (cert, _) = self_signed_pem("localhost");
        let tls = TlsConfig::new(
            write_pem(&dir, "cert.pem", &cert),
            write_pem(&dir, "key.pem", "not a key either"),
        );
        let err = load_server_config(&tls).expect_err("garbage key must fail");
        assert!(matches!(err, TlsError::NoPrivateKey { .. }), "got: {err:?}");
    }

    #[test]
    fn mismatched_cert_key_pair_is_config_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (cert_a, _) = self_signed_pem("a.example");
        let (_, key_b) = self_signed_pem("b.example");
        let tls = TlsConfig::new(
            write_pem(&dir, "cert_a.pem", &cert_a),
            write_pem(&dir, "key_b.pem", &key_b),
        );
        let err = load_server_config(&tls).expect_err("mismatched pair must fail");
        assert!(matches!(err, TlsError::Config(_)), "got: {err:?}");
    }

    #[test]
    fn reloader_serves_initial_cert_and_swap_on_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (cert_v1, key_v1) = self_signed_pem("v1.example");
        let (cert_v2, key_v2) = self_signed_pem("v2.example");
        let cert_path = write_pem(&dir, "cert.pem", &cert_v1);
        let key_path = write_pem(&dir, "key.pem", &key_v1);

        let tls = TlsConfig::new(cert_path.clone(), key_path.clone());
        let reloading = ReloadingTls::new(&tls).expect("initial load");
        let before = reloading.end_entity_certificate().expect("initial cert");

        // 换盘为 v2 后 reload：终端实体证书必须变化。
        std::fs::write(&cert_path, &cert_v2).expect("overwrite cert");
        std::fs::write(&key_path, &key_v2).expect("overwrite key");
        reloading.reload().expect("reload with matched pair");
        let after = reloading.end_entity_certificate().expect("reloaded cert");
        assert_ne!(before, after, "reload must swap the served certificate");
    }

    #[tokio::test]
    async fn reload_async_swaps_cert_without_blocking_caller() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (cert_v1, key_v1) = self_signed_pem("v1.example");
        let (cert_v2, key_v2) = self_signed_pem("v2.example");
        let cert_path = write_pem(&dir, "cert.pem", &cert_v1);
        let key_path = write_pem(&dir, "key.pem", &key_v1);

        let reloading = ReloadingTls::new(&TlsConfig::new(cert_path.clone(), key_path.clone()))
            .expect("initial load");
        let before = reloading.end_entity_certificate().expect("initial cert");

        std::fs::write(&cert_path, &cert_v2).expect("overwrite cert");
        std::fs::write(&key_path, &key_v2).expect("overwrite key");
        reloading.reload_async().await.expect("async reload");
        assert_ne!(
            before,
            reloading.end_entity_certificate().expect("reloaded cert"),
            "reload_async must swap the served certificate"
        );
    }

    #[test]
    fn reload_with_mismatched_pair_keeps_old_cert() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (cert_v1, key_v1) = self_signed_pem("v1.example");
        let (_, key_other) = self_signed_pem("other.example");
        let cert_path = write_pem(&dir, "cert.pem", &cert_v1);
        let key_path = write_pem(&dir, "key.pem", &key_v1);

        let reloading = ReloadingTls::new(&TlsConfig::new(cert_path.clone(), key_path.clone()))
            .expect("initial load");
        let before = reloading.end_entity_certificate().expect("initial cert");

        // 证书换新、私钥不换 → 密钥对不匹配 → reload 失败且旧证书保留。
        std::fs::write(&cert_path, self_signed_pem("v2.example").0).expect("overwrite cert");
        std::fs::write(&key_path, key_other).expect("overwrite key with foreign key");
        let err = reloading.reload().expect_err("mismatched reload must fail");
        assert!(matches!(err, TlsError::Config(_)), "got: {err:?}");
        assert_eq!(
            reloading
                .end_entity_certificate()
                .expect("cert still there"),
            before,
            "failed reload must keep the previous certificate"
        );
    }

    #[test]
    fn reload_with_missing_file_keeps_old_cert() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (cert_v1, key_v1) = self_signed_pem("v1.example");
        let cert_path = write_pem(&dir, "cert.pem", &cert_v1);
        let key_path = write_pem(&dir, "key.pem", &key_v1);

        let reloading = ReloadingTls::new(&TlsConfig::new(cert_path.clone(), key_path.clone()))
            .expect("initial load");
        let before = reloading.end_entity_certificate().expect("initial cert");

        std::fs::remove_file(&key_path).expect("remove key file");
        let err = reloading.reload().expect_err("missing key file must fail");
        assert!(matches!(err, TlsError::KeyRead { .. }), "got: {err:?}");
        assert_eq!(
            reloading
                .end_entity_certificate()
                .expect("cert still there"),
            before,
            "failed reload must keep the previous certificate"
        );
    }

    #[test]
    fn handshake_failure_log_window_limits_warn_rate() {
        // 首次立即 warn；窗口内不 warn；窗口边界再次 warn。
        assert!(
            handshake_failure_should_warn(1000, 0),
            "first failure warns"
        );
        assert!(
            !handshake_failure_should_warn(1059, 1000),
            "within the 60s window must not warn again"
        );
        assert!(
            handshake_failure_should_warn(1060, 1000),
            "at window expiry must warn again"
        );
    }

    #[cfg(unix)]
    #[test]
    fn key_file_permissions_detection_matches_file_mode() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("key.pem");
        std::fs::write(&path, b"not really a key").expect("write key file");

        let make_mode = |mode: u32| {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        };

        make_mode(0o600);
        assert!(
            !key_file_permissions_exposed(path.to_str().unwrap()).expect("stat 0600"),
            "0600 must not be flagged as exposed"
        );
        make_mode(0o644);
        assert!(
            key_file_permissions_exposed(path.to_str().unwrap()).expect("stat 0644"),
            "group/other-readable key must be flagged"
        );
        make_mode(0o660);
        assert!(
            key_file_permissions_exposed(path.to_str().unwrap()).expect("stat 0660"),
            "group-readable key must be flagged"
        );
    }
}
