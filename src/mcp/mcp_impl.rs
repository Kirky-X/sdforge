// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;

/// Get all registered MCP tools as runtime instances.
///
/// This function collects all `McpToolRegistration` entries from the
/// `inventory` registry and creates `McpToolInstance` objects with the
/// associated metadata.
#[cfg(feature = "mcp")]
pub fn get_mcp_tools() -> Vec<McpToolInstance> {
    inventory::iter::<McpToolRegistration>
        .into_iter()
        .map(|reg| {
            let tool = (reg.create_fn)();
            let reg_metadata = reg.metadata();
            McpToolInstance::new(
                tool,
                ApiMetadata::new(
                    reg.name().to_string(),
                    reg.version().to_string(),
                    reg.metadata().description().to_string(),
                    reg_metadata.cache_ttl(),
                    reg_metadata.is_streaming(),
                ),
            )
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
#[cfg(feature = "mcp")]
pub fn build() -> SdForgeMcpServer {
    SdForgeMcpServer::new()
}

/// Serve an MCP server over stdio transport.
///
/// This is a convenience wrapper that encapsulates `rmcp::transport::stdio()`
/// and `ServiceExt::serve()`, so downstream crates do not need to depend on
/// `rmcp` directly.
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
#[cfg(feature = "mcp")]
pub async fn serve_stdio(
    server: SdForgeMcpServer,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use rmcp::ServiceExt;
    let transport = rmcp::transport::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
