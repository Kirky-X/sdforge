// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 统一文档输出模块 — Swagger UI + CLI/MCP Markdown。
//!
//! 仅当 `docs` feature 启用时可用。依赖 `openapi`（复用 `generate_openapi_spec`）、
//! `cli`（复用 `CliBuilder`）和 `clap-markdown`/`utoipa-swagger-ui`。
//!
//! # 示例
//!
//! ```ignore
//! use sdforge::docs::{generate_docs, DocFormat};
//!
//! let json = generate_docs(DocFormat::OpenApi).expect("OpenAPI generation");
//! let md = generate_docs(DocFormat::CliMarkdown).expect("CLI docs generation");
//! ```

pub mod cli_markdown;
#[cfg(feature = "mcp")]
pub mod mcp_markdown;
#[cfg(feature = "http")]
pub mod swagger;

#[cfg(feature = "http")]
pub use swagger::swagger_ui_router;

pub use cli_markdown::generate_cli_docs;
#[cfg(feature = "mcp")]
pub use mcp_markdown::generate_mcp_docs;

/// 文档输出格式枚举。
///
/// 每个变体对应一种文档生成策略，由 [`generate_docs`] 分发处理：
///
/// | 变体 | 输出格式 | 来源 |
/// |------|----------|------|
/// | `OpenApi` | OpenAPI 3.1 JSON | `openapi::generate_openapi_spec()` |
/// | `SwaggerUi` | HTML 入口页（指向 `/swagger-ui/`） | `utoipa-swagger-ui` |
/// | `CliMarkdown` | CLI 命令手册 Markdown | `clap_markdown::help_markdown_command` |
/// | `McpMarkdown` | MCP 工具列表 Markdown | `inventory::iter::<McpToolRegistration>` |
/// | `All` | OpenApi + CliMarkdown + McpMarkdown 拼接（SwaggerUi 为 HTML，需单独访问 /swagger-ui/） | 各格式组合 |
///
/// 默认值为 [`DocFormat::OpenApi`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocFormat {
    /// OpenAPI 3.1 JSON 规范文档。
    #[default]
    OpenApi,
    /// Swagger UI HTML 入口页。
    SwaggerUi,
    /// CLI 命令手册（Markdown）。
    CliMarkdown,
    /// MCP 工具列表（Markdown，需 `mcp` feature）。
    McpMarkdown,
    /// 全部格式拼接输出。
    All,
}

/// 文档生成 / 写入过程中可能发生的错误。
#[derive(Debug, thiserror::Error)]
pub enum DocError {
    /// OpenAPI JSON 序列化失败（通常表示框架 bug）。
    #[error("OpenAPI serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    /// `write_docs` 文件 I/O 失败。
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

mod docs_impl;
pub use docs_impl::{generate_docs, write_docs};

#[cfg(test)]
mod tests;
