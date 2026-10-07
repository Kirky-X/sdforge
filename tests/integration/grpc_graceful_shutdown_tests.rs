// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! gRPC 优雅停机（`build_server_with_graceful_shutdown`）集成测试。
//!
//! 与 HTTP 侧 `serve_with_graceful_shutdown`（axum）对齐的语义：signal
//! future 完成后停止接受新连接、等待 in-flight 请求完成，`serve` 调用
//! 返回 `Ok(())`。全部用例经真实网络（tonic Channel）验证，口径与
//! grpc_extra_services_tests 一致。

#![cfg(feature = "grpc")]

use std::time::Duration;

use sdforge::grpc::sdforge_v1::{InfoRequest, sd_forge_service_client::SdForgeServiceClient};
use sdforge::grpc::{GrpcServerConfig, build_server_with_graceful_shutdown};
use tonic::Request;
use tonic::transport::Channel;

/// 预 bind 127.0.0.1:0 拿可用端口后 drop（与既有 gRPC 集成测试同口径）。
fn free_addr() -> std::net::SocketAddr {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe listener");
    let addr = probe.local_addr().expect("probe local_addr");
    drop(probe);
    addr
}

/// 轮询等待端口可连（服务器 bind 完成）。
async fn wait_ready(addr: std::net::SocketAddr) {
    for _ in 0..200 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("gRPC server did not become ready within 5s at {addr}");
}

/// 轮询等待端口不可连（服务器已释放监听）。
async fn wait_closed(addr: std::net::SocketAddr) {
    for _ in 0..200 {
        if tokio::net::TcpStream::connect(addr).await.is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("gRPC server still listening within 5s after shutdown at {addr}");
}

/// 建立已连接的 client（5s 超时，与 grpc_tests setup 同口径）。
async fn connect_client(addr: std::net::SocketAddr) -> SdForgeServiceClient<Channel> {
    let channel = tokio::time::timeout(
        Duration::from_secs(5),
        Channel::builder(format!("http://{addr}").parse().unwrap()).connect(),
    )
    .await
    .expect("client connect timeout")
    .expect("client connect failed");
    SdForgeServiceClient::new(channel)
}

/// 优雅停机主链路：signal 触发后 serve 返回 Ok、监听释放、新连接被拒。
#[tokio::test]
async fn graceful_shutdown_stops_serve_and_releases_listener() {
    let addr = free_addr();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_addr = addr.to_string();
    let server = tokio::spawn(async move {
        build_server_with_graceful_shutdown(
            &server_addr,
            GrpcServerConfig {
                require_auth: false,
                ..Default::default()
            },
            async {
                let _ = shutdown_rx.await;
            },
        )
        .await
        .expect("graceful server should not fail");
    });

    wait_ready(addr).await;

    // 停机前：门面可正常服务
    let mut client = connect_client(addr).await;
    let info = client
        .get_info(Request::new(InfoRequest {
            version: String::new(),
        }))
        .await
        .expect("get_info before shutdown");
    assert!(
        !info.into_inner().name.is_empty(),
        "GetInfo 应返回非空服务名"
    );

    // 触发停机 signal
    shutdown_tx
        .send(())
        .expect("server task should still be alive");

    // serve 返回（JoinHandle 完成）且监听释放
    tokio::time::timeout(Duration::from_secs(10), server)
        .await
        .expect("serve should return after shutdown signal")
        .expect("server task join");
    wait_closed(addr).await;

    // 停机后：新连接失败（旧 Channel 的懒重连不可靠，以全新连接为准）
    let reconnect = tokio::time::timeout(
        Duration::from_secs(5),
        Channel::builder(format!("http://{addr}").parse().unwrap()).connect(),
    )
    .await;
    match reconnect {
        Err(_) => panic!("停机后 connect 不应超时（监听已释放应立即拒绝）"),
        Ok(Ok(_)) => panic!("shutdown 后新连接应失败；成功说明监听未释放"),
        Ok(Err(_)) => {}
    }
}

/// signal 触发时无 in-flight 请求 → serve 立即返回（不等待 timeout_seconds）。
#[tokio::test]
async fn graceful_shutdown_returns_promptly_when_idle() {
    let addr = free_addr();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_addr = addr.to_string();
    let server = tokio::spawn(async move {
        build_server_with_graceful_shutdown(
            &server_addr,
            GrpcServerConfig {
                require_auth: false,
                timeout_seconds: 30,
                ..Default::default()
            },
            async {
                let _ = shutdown_rx.await;
            },
        )
        .await
        .expect("graceful server should not fail");
    });

    wait_ready(addr).await;
    shutdown_tx
        .send(())
        .expect("server task should still be alive");

    // 空闲态停机：远小于 timeout_seconds(30s)，10s 内必须返回
    tokio::time::timeout(Duration::from_secs(10), server)
        .await
        .expect("idle graceful shutdown should return well before timeout_seconds")
        .expect("server task join");
}

/// signal 未触发 → serve 永驻：sender 被 forget（永不触发）后服务持续可用。
/// 语义与 axum `with_graceful_shutdown` 一致——以 future 完成为准。
#[tokio::test]
async fn serve_keeps_running_without_signal() {
    let addr = free_addr();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    std::mem::forget(shutdown_tx);

    let server_addr = addr.to_string();
    let _server = tokio::spawn(async move {
        let _ = build_server_with_graceful_shutdown(
            &server_addr,
            GrpcServerConfig {
                require_auth: false,
                ..Default::default()
            },
            async {
                let _ = shutdown_rx.await;
            },
        )
        .await;
    });

    wait_ready(addr).await;

    let mut client = connect_client(addr).await;
    let call = client
        .get_info(Request::new(InfoRequest {
            version: String::new(),
        }))
        .await;
    assert!(
        call.is_ok(),
        "未触发 signal 时服务应持续可用（GetInfo 正常响应）"
    );
}
