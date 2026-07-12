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
/// To start the server, use `ServiceExt::serve()` on the returned server with
/// a transport (e.g., `rmcp::transport::stdio()`).
///
/// # Example
///
/// ```rust,ignore
/// use rmcp::{ServiceExt, transport::stdio};
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let server = sdforge::mcp::build();
/// let service = server.serve(stdio()).await?;
/// service.waiting().await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "mcp")]
pub fn build() -> SdForgeMcpServer {
    SdForgeMcpServer::new()
}
