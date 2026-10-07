// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! integration: gRPC server-streaming（`streaming` × `grpc` 组合）。
//!
//! `#[forge(grpc_method = "...", stream = true)]` 声明的 handler 返回
//! `StreamResponse<T>`；宏生成 `GrpcStreamHandlerRegistration`，服务端经
//! `CallStream` RPC 逐项回送 `CallResponse`。本文件走真实 tonic 网络链路
//!（绑定的 listener 经 `serve_with_incoming` + tonic client），验证端到
//! 端：多项流、项级错误不中断、unary/流式双向方向指引。

#![cfg(all(feature = "grpc", feature = "streaming"))]

use std::collections::HashMap;
use std::time::Duration;

use sdforge::core::ApiError;
#[allow(deprecated)]
use sdforge::grpc::SdForgeGrpcService;
use sdforge::grpc::sdforge_v1::{CallRequest, sd_forge_service_client::SdForgeServiceClient};
use sdforge::streaming::create_stream_channel;
use tokio_stream::StreamExt;

/// 流式端点（宏展开为 GrpcStreamHandlerRegistration；count 个计数项）。
#[sdforge::forge(
    name = "e2e_stream_counter",
    version = "v1",
    description = "streams count items",
    path = "/e2e/stream",
    method = "GET",
    grpc_method = "e2e_stream_counter",
    stream = true
)]
async fn e2e_stream_counter(
    count: Option<u64>,
) -> Result<sdforge::streaming::StreamResponse<String>, ApiError> {
    let count = count.unwrap_or(3);
    let (tx, response) = create_stream_channel::<String>(8);
    tokio::spawn(async move {
        for i in 0..count {
            if tx.send(Ok(format!("evt-{i}"))).await.is_err() {
                break;
            }
        }
    });
    Ok(response)
}

/// 项级错误流：中间项失败，流继续。
#[sdforge::forge(
    name = "e2e_stream_failing",
    version = "v1",
    description = "second item fails, stream continues",
    path = "/e2e/stream-fail",
    method = "GET",
    grpc_method = "e2e_stream_failing",
    stream = true
)]
async fn e2e_stream_failing() -> Result<sdforge::streaming::StreamResponse<String>, ApiError> {
    let (tx, response) = create_stream_channel::<String>(4);
    tokio::spawn(async move {
        let _ = tx.send(Ok("before".to_string())).await;
        let _ = tx.send(Err("boom-on-item-2".to_string())).await;
        let _ = tx.send(Ok("after".to_string())).await;
    });
    Ok(response)
}

/// unary 端点（验证 unary/流式两表互斥的方向指引）。
#[sdforge::forge(
    name = "e2e_unary_ping",
    version = "v1",
    description = "plain unary echo",
    path = "/e2e/ping",
    method = "GET",
    grpc_method = "e2e_unary_ping"
)]
async fn e2e_unary_ping() -> Result<String, ApiError> {
    Ok("pong".to_string())
}

/// 启动真实 gRPC 服务器（后台任务），返回客户端。
async fn spawn_server() -> SdForgeServiceClient<tonic::transport::Channel> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");

    // 绑定的 listener 直接交给服务器（serve_with_incoming），与既有
    // grpc_tests setup 同模式：既避开端口释放竞态，也不经
    // build_server_with_config 的 require_auth fail-safe（security feature
    // 下默认拒绝无 auth 的服务器启动——那是面向部署的安全约束，本测试
    // 关注流式分发语义而非认证面）。
    tokio::spawn(async move {
        let service = SdForgeGrpcService::default();
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
        let _ = tonic::transport::Server::builder()
            .add_service(
                sdforge::grpc::sdforge_v1::sd_forge_service_server::SdForgeServiceServer::new(
                    service,
                )
                .max_decoding_message_size(4 * 1024 * 1024),
            )
            .serve_with_incoming(incoming)
            .await;
    });

    for _ in 0..100 {
        let endpoint = match tonic::transport::Endpoint::from_shared(format!("http://{addr}")) {
            Ok(endpoint) => endpoint,
            Err(_) => continue,
        };
        if let Ok(channel) = endpoint
            .connect_timeout(Duration::from_secs(1))
            .connect()
            .await
        {
            return SdForgeServiceClient::new(channel);
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("grpc server never became connectable at {addr}");
}

fn stream_request(method: &str, count: Option<u64>) -> tonic::Request<CallRequest> {
    let mut parameters = HashMap::new();
    if let Some(c) = count {
        parameters.insert("count".to_string(), c.to_string());
    }
    tonic::Request::new(CallRequest {
        method: method.to_string(),
        parameters,
        data: String::new(),
    })
}

#[tokio::test]
async fn e2e_stream_yields_all_items_over_real_network() {
    let mut client = spawn_server().await;

    let response = client
        .call_stream(stream_request("e2e_stream_counter", Some(4)))
        .await
        .expect("CallStream succeeds");
    let items: Vec<_> = response.into_inner().collect().await;

    assert_eq!(items.len(), 4, "all four items must arrive: {items:?}");
    for (i, item) in items.iter().enumerate() {
        let resp = item.as_ref().expect("item Ok");
        assert!(resp.success);
        assert_eq!(resp.data, format!("evt-{i}"));
    }
}

#[tokio::test]
async fn e2e_stream_item_error_reaches_client_and_stream_continues() {
    let mut client = spawn_server().await;

    let response = client
        .call_stream(stream_request("e2e_stream_failing", None))
        .await
        .expect("CallStream succeeds");
    let items: Vec<_> = response.into_inner().collect().await;

    assert_eq!(items.len(), 3, "error item must not abort: {items:?}");
    assert!(items[0].as_ref().expect("first").success);
    let failed = items[1].as_ref().expect("second arrives as message");
    assert!(!failed.success, "item error must carry success:false");
    assert_eq!(failed.error, "boom-on-item-2");
    assert_eq!(items[2].as_ref().expect("third").data, "after");
}

#[tokio::test]
async fn e2e_unary_call_rejects_streaming_method_with_direction() {
    let mut client = spawn_server().await;

    let err = match client
        .call(stream_request("e2e_stream_counter", None))
        .await
    {
        Ok(_) => panic!("unary Call must reject a streaming method"),
        Err(status) => status,
    };
    assert_eq!(err.code(), tonic::Code::FailedPrecondition);
    assert!(
        err.message().contains("CallStream"),
        "error must point at CallStream: {err}"
    );
}

#[tokio::test]
async fn e2e_stream_call_rejects_unary_method_with_direction() {
    let mut client = spawn_server().await;

    let err = match client
        .call_stream(stream_request("e2e_unary_ping", None))
        .await
    {
        Ok(_) => panic!("CallStream must reject a unary method"),
        Err(status) => status,
    };
    assert_eq!(err.code(), tonic::Code::FailedPrecondition);
    assert!(
        err.message().contains("Call"),
        "error must point at the unary Call RPC: {err}"
    );
}

#[tokio::test]
async fn e2e_unary_path_still_works_alongside_streaming() {
    let mut client = spawn_server().await;

    let response = client
        .call(stream_request("e2e_unary_ping", None))
        .await
        .expect("unary Call succeeds");
    let resp = response.into_inner();
    assert!(resp.success);
    assert_eq!(resp.data, "pong");
}

/// 生产者产出观测计数器（取消语义测试用，模块级 static 供宏生成的
/// handler 与测试断言共享）。
static CANCEL_PRODUCED: std::sync::LazyLock<std::sync::atomic::AtomicUsize> =
    std::sync::LazyLock::new(|| std::sync::atomic::AtomicUsize::new(0));

/// 取消语义（显性化既有契约）：客户端取得流后只消费首项即断开——
/// 服务端生产者的后续 send 因接收端 drop 而失败，生产者任务必须在有限
/// 时间内退出（不悬挂、不无限积压）。生产者侧以计数器观测实际产出量。
#[tokio::test]
async fn e2e_client_disconnect_stops_producer_within_bounds() {
    use std::sync::atomic::Ordering;

    // 专用 handler：产出 256 项，每项后递增计数器。
    #[sdforge::forge(
        name = "e2e_stream_cancellable",
        version = "v1",
        description = "long stream for cancel semantics",
        path = "/e2e/stream-cancel",
        method = "GET",
        grpc_method = "e2e_stream_cancellable",
        stream = true
    )]
    async fn e2e_stream_cancellable() -> Result<sdforge::streaming::StreamResponse<String>, ApiError>
    {
        let (tx, response) = create_stream_channel::<String>(2);
        tokio::spawn(async move {
            for i in 0..256u32 {
                if tx.send(Ok(format!("tick-{i}"))).await.is_err() {
                    break; // 客户端断开 → 生产者及时退出
                }
                CANCEL_PRODUCED.fetch_add(1, Ordering::SeqCst);
            }
        });
        Ok(response)
    }
    let _ = e2e_stream_cancellable; // 宏注册依赖符号存活

    let mut client = spawn_server().await;

    let response = client
        .call_stream(stream_request("e2e_stream_cancellable", None))
        .await
        .expect("CallStream succeeds");
    let mut stream = response.into_inner();

    // 只消费首项，然后断开（drop 整个响应流 → 客户端侧 RST/关闭）。
    let first = stream.next().await.expect("first item").expect("ok");
    assert_eq!(first.data, "tick-0");
    drop(stream);
    drop(client);

    // 生产者必须在有限时间内退出（远小于 256 项全部产出的量级）。
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
        // 稳态判定：两个采样间隔产出不再增长即认为生产者已停/阻塞。
        let a = CANCEL_PRODUCED.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(40)).await;
        let b = CANCEL_PRODUCED.load(Ordering::SeqCst);
        if a == b {
            break;
        }
    }
    // 观测窗口结束后，产出量必须远小于流全长（背压生效，生产者被卡在
    // channel 容量上或已因断开退出）。
    let observed = CANCEL_PRODUCED.load(Ordering::SeqCst);
    assert!(
        observed < 256,
        "producer must stop (or block) after client disconnect; produced {observed}/256"
    );
}
