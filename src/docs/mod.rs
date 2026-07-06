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

pub mod cli_markdown;
#[cfg(feature = "mcp")]
pub mod mcp_markdown;
pub mod swagger;

pub use swagger::swagger_ui_router;

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

/// 根据指定格式生成文档字符串。
///
/// 各变体分发到对应的生成函数：
/// - `OpenApi` → `crate::openapi::generate_openapi_spec()` 序列化为 pretty JSON
/// - `SwaggerUi` → HTML 入口页
/// - `CliMarkdown` → CLI 命令手册
/// - `McpMarkdown` → MCP 工具列表（需 `mcp` feature，未启用时返回占位提示）
/// - `All` → 全部格式拼接（OpenApi + CliMarkdown + McpMarkdown）
///
/// 序列化失败时降级为空字符串并 `log::warn!`，不 panic。
pub fn generate_docs(format: DocFormat) -> String {
    match format {
        DocFormat::OpenApi => {
            let spec = crate::openapi::generate_openapi_spec();
            serde_json::to_string_pretty(&spec).unwrap_or_else(|e| {
                log::warn!("OpenAPI 序列化失败: {}", e);
                String::new()
            })
        }
        DocFormat::SwaggerUi => generate_swagger_html(),
        DocFormat::CliMarkdown => cli_markdown::generate_cli_docs(),
        DocFormat::McpMarkdown => generate_mcp_markdown(),
        DocFormat::All => {
            let mut out = String::new();
            out.push_str("# OpenAPI Specification\n\n");
            out.push_str(&generate_docs(DocFormat::OpenApi));
            out.push_str("\n\n");
            out.push_str("# CLI Documentation\n\n");
            out.push_str(&generate_docs(DocFormat::CliMarkdown));
            out.push_str("\n\n");
            out.push_str(&generate_docs(DocFormat::McpMarkdown));
            out
        }
    }
}

/// 生成 MCP 工具列表 Markdown。
///
/// - `mcp` feature 启用时：委托给 [`mcp_markdown::generate_mcp_docs`]
/// - `mcp` feature 未启用时：返回占位提示（告知用户需启用 mcp feature）
fn generate_mcp_markdown() -> String {
    #[cfg(feature = "mcp")]
    {
        mcp_markdown::generate_mcp_docs()
    }
    #[cfg(not(feature = "mcp"))]
    {
        "<!-- MCP feature not enabled. Enable `mcp` feature to generate MCP tool documentation. -->\n"
            .to_string()
    }
}

/// 生成 Swagger UI HTML 入口页。
///
/// 返回一个简单的 HTML 页面，包含指向 `/swagger-ui/` 的链接和自动跳转脚本。
/// 实际的 Swagger UI 界面由 [`swagger_ui_router`] 挂载的 axum Router 提供。
fn generate_swagger_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <title>SDForge API Docs</title>
    <meta http-equiv="refresh" content="0; url=/swagger-ui/">
    <script>window.location.replace('/swagger-ui/');</script>
</head>
<body>
    <p>Redirecting to <a href="/swagger-ui/">Swagger UI</a>...</p>
</body>
</html>"#
        .to_string()
}

/// 将指定格式的文档写入文件。
///
/// 调用 [`generate_docs`] 生成内容，再用 `std::fs::write` 写入。
/// IO 错误直接返回，不吞掉。
pub fn write_docs(format: DocFormat, output_path: &std::path::Path) -> Result<(), std::io::Error> {
    let content = generate_docs(format);
    std::fs::write(output_path, content)
}

#[cfg(test)]
mod tests;
