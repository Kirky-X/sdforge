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
/// - `SwaggerUi` → HTML 入口页（T015 填充）
/// - `CliMarkdown` → CLI 命令手册（T017 填充）
/// - `McpMarkdown` → MCP 工具列表（T019 填充，需 `mcp` feature）
/// - `All` → 全部格式拼接（T019 填充）
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
        DocFormat::SwaggerUi => String::new(),
        DocFormat::CliMarkdown => String::new(),
        DocFormat::McpMarkdown => String::new(),
        DocFormat::All => String::new(),
    }
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
