// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Streamable HTTP server-side mounting e2e（集成缺口修复实证）。
//!
//! 修复前：sdforge 的 rmcp 构建只开 `server + transport-io`（stdio 用），
//! 下游挂 `/mcp` 的 rmcp `StreamableHttpService` 必须直连 rmcp 并自担
//! 版本对齐。修复后：`mcp-http` feature（= `rmcp/transport-streamable-http-server`）
//! 叠加 `sdforge::rmcp` 再导出——本测试的全部 rmcp 类型经 `sdforge::rmcp`
//! 解析，**发布面（[dependencies]）零直连 rmcp**；dev-dependencies 里的
//! rmcp 仅用于开启客户端 transport feature（feature 无法经再导出开启），
//! 版本由 cargo 统一为单实例，即「类型合一」的直接证明。
//!
//! 真实往返：axum 挂载 `/mcp` → rmcp 客户端 initialize → tools/list →
//! tools/call → stateful 工具经 `ToolCallContext` 拿到 state（与 alphalloy
//! /mcp 场景同构）。

#![cfg(all(feature = "mcp", feature = "mcp-http"))]

use serde_json::json;
use std::sync::{Arc, Mutex};

/// Marker state for the stateful probe（alphalloy /mcp 场景同构）.
#[derive(Debug)]
struct HttpState {
    tag: &'static str,
}

/// Stateful probe tool: records what `ToolCallContext` carried.
struct HttpProbeTool {
    seen_state: Mutex<Vec<String>>,
}

impl sdforge::mcp::SdForgeTool for HttpProbeTool {
    fn name(&self) -> &str {
        "http_probe"
    }

    fn description(&self) -> &str {
        "stateful probe over streamable http"
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({"type": "object", "properties": {}})
    }

    fn call(
        &self,
        _input: Option<serde_json::Value>,
    ) -> Result<sdforge::rmcp::model::CallToolResult, sdforge::rmcp::model::ErrorData> {
        Err(sdforge::rmcp::model::ErrorData::internal_error(
            "http probe must be dispatched via call_with_ctx",
            None,
        ))
    }

    fn call_with_ctx(
        &self,
        _input: Option<serde_json::Value>,
        ctx: &sdforge::mcp::ToolCallContext,
    ) -> Result<sdforge::rmcp::model::CallToolResult, sdforge::rmcp::model::ErrorData> {
        let state: Arc<HttpState> = sdforge::core::downcast_state::<HttpState>(ctx.state())
            .map_err(|e| sdforge::rmcp::model::ErrorData::internal_error(format!("{e:?}"), None))?;
        self.seen_state
            .lock()
            .expect("mutex not poisoned")
            .push(state.tag.to_string());
        Ok(sdforge::rmcp::model::CallToolResult::success(vec![
            sdforge::rmcp::model::ContentBlock::text("probed-over-http"),
        ]))
    }
}

/// Spawn the axum server with `StreamableHttpService` mounted at `/mcp` —
/// every rmcp type here resolves through the `sdforge::rmcp` re-export.
async fn spawn_server() -> (std::net::SocketAddr, Arc<HttpProbeTool>) {
    use sdforge::rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };

    let probe = Arc::new(HttpProbeTool {
        seen_state: Mutex::new(Vec::new()),
    });
    let metadata = sdforge::core::ApiMetadata::new(
        "http_probe".to_string(),
        "v1".to_string(),
        "stateful probe".to_string(),
        None,
        false,
    );
    let server =
        sdforge::mcp::SdForgeMcpServer::with_tools(vec![sdforge::mcp::McpToolInstance::new(
            probe.clone(),
            metadata,
        )])
        .with_state(
            Arc::new(HttpState { tag: "over-http" }) as Arc<dyn std::any::Any + Send + Sync>
        );

    // Factory clones the wired server per request（rmcp 3.10：2026-07-28 协议
    // 无 session，仅 legacy 协议按 session 实例化）——克隆成本 = Vec<Arc> 原子
    // 递增 + 2 个 String，微秒级；state 经 HandlerState 的 Arc 随克隆共享。
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    let router = sdforge::axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind 127.0.0.1:0");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let _ = sdforge::axum::serve(listener, router).await;
    });
    (addr, probe)
}

/// Real MCP client over Streamable HTTP（dev rmcp client features；同一 crate，
/// 版本由 cargo 统一）。
async fn connect(
    addr: std::net::SocketAddr,
) -> sdforge::rmcp::service::RunningService<sdforge::rmcp::RoleClient, ()> {
    use sdforge::rmcp::transport::StreamableHttpClientTransport;
    use sdforge::rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;

    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(format!("http://{addr}/mcp")),
    );
    sdforge::rmcp::service::serve_client((), transport)
        .await
        .expect("client initialize must succeed")
}

#[tokio::test]
async fn streamable_http_mount_serves_tools_with_state() {
    let (addr, probe) = spawn_server().await;
    let client = connect(addr).await;

    // tools/list 走真实 HTTP 往返
    let tools = client
        .peer()
        .list_tools(None)
        .await
        .expect("tools/list must succeed over streamable http");
    assert!(
        tools.tools.iter().any(|t| t.name == "http_probe"),
        "registered probe tool must be listed, got: {:?}",
        tools.tools.iter().map(|t| &t.name).collect::<Vec<_>>()
    );

    // tools/call 走真实 HTTP 往返 → state 经 ToolCallContext 到达工具
    let result = client
        .peer()
        .call_tool(sdforge::rmcp::model::CallToolRequestParams::new(
            "http_probe",
        ))
        .await
        .expect("tools/call must succeed over streamable http");
    assert_ne!(result.is_error, Some(true), "probe must dispatch");
    assert_eq!(
        *probe.seen_state.lock().expect("mutex not poisoned"),
        vec!["over-http".to_string()],
        "state must reach the tool over streamable http"
    );
}
