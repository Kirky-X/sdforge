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
//! let json = generate_docs(DocFormat::OpenApi);
//! let md = generate_docs(DocFormat::CliMarkdown);
//! ```

/// 文档输出格式枚举。
///
/// 每个变体对应一种文档生成策略，由 [`generate_docs`] 分发处理：
///
/// | 变体 | 输出格式 | 来源 |
/// |------|----------|------|
/// | `OpenApi` | OpenAPI 3.1 JSON | `openapi::generate_openapi_spec()` |
/// | `SwaggerUi` | HTML 入口页（指向 `/swagger-ui/`） | `utoipa-swagger-ui` |
/// | `CliMarkdown` | CLI 命令手册 Markdown | `clap_markdown::help_markdown` |
/// | `McpMarkdown` | MCP 工具列表 Markdown | `inventory::iter::<McpToolRegistration>` |
/// | `All` | 以上全部拼接 | 各格式组合 |
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

#[cfg(test)]
mod tests;
