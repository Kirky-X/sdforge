// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use super::*;

/// Get all registered MCP tools as runtime instances.
///
/// This function collects all `McpToolRegistration` entries from the
/// `inventory` registry and creates `McpToolInstance` objects with the
/// associated metadata.
pub fn get_mcp_tools() -> Vec<McpToolInstance> {
    inventory::iter::<McpToolRegistration>
        .into_iter()
        .map(|reg| {
            let tool = (reg.create_fn)();
            // metadata_fn 产出的完整元数据直通实例：重建会丢弃 i18n_key
            // 与 endpoint lifecycle（`#[forge(deprecated, sunset,
            // successor)]`），导致 MCP 消费点拿不到声明。
            McpToolInstance::new(tool, reg.metadata()).with_roles(reg.roles)
        })
        .collect()
}

/// Build a `SdForgeMcpServer` from registered tools.
///
/// This constructs a server that implements `rmcp::handler::server::ServerHandler`.
/// To start the server, use [`serve_stdio`] for stdio transport, or
/// `ServiceExt::serve()` on the returned server with a custom transport.
///
/// # Example
///
/// ```rust,ignore
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let server = sdforge::mcp::build();
/// sdforge::mcp::serve_stdio(server).await?;
/// # Ok(())
/// # }
/// ```
pub fn build() -> SdForgeMcpServer {
    SdForgeMcpServer::new()
}

/// Serve an MCP server over stdio transport.
///
/// This is a convenience wrapper that encapsulates `rmcp::transport::stdio()`
/// and `ServiceExt::serve()`, so downstream crates do not need to depend on
/// `rmcp` directly.
///
/// # Authentication caveat
///
/// The stdio transport carries no headers, so it has no channel for
/// [`McpCredentials`]: a server built with
/// `SdForgeMcpServer::with_auth_verifier` will reject **every** `call_tool`
/// / `list_tools` served here. This guards with a startup warning; for
/// authenticated MCP endpoints serve over a transport adapter that injects
/// credentials (see the [`McpCredentials`] docs).
///
/// # Errors
///
/// Returns an error if the server fails to start or the service encounters
/// an error during operation.
///
/// # Example
///
/// ```rust,ignore
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let server = sdforge::mcp::build();
/// sdforge::mcp::serve_stdio(server).await?;
/// # Ok(())
/// # }
/// ```
pub async fn serve_stdio(
    server: SdForgeMcpServer,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "security")]
    if server.auth_verifier.is_some() {
        log::warn!(
            "serve_stdio: an auth verifier is configured but the stdio transport cannot \
             carry transport credentials — every call_tool/list_tools will be rejected. \
             Serve authenticated MCP over a transport adapter that injects McpCredentials."
        );
    }
    use rmcp::ServiceExt;
    let transport = rmcp::transport::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
