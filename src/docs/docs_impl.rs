// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;

/// 根据指定格式生成文档字符串。
///
/// 各变体分发到对应的生成函数：
/// - `OpenApi` → `crate::openapi::generate_openapi_spec()` 序列化为 pretty JSON
/// - `SwaggerUi` → HTML 入口页
/// - `CliMarkdown` → CLI 命令手册
/// - `McpMarkdown` → MCP 工具列表（需 `mcp` feature，未启用时返回占位提示）
/// - `All` → 全部格式拼接（OpenApi + CliMarkdown + McpMarkdown；SwaggerUi 为 HTML，需单独访问 /swagger-ui/）
///
/// 返回 `Result`，OpenAPI 序列化失败时以 [`DocError::Serialization`] 返回，
/// 不再 panic。
pub fn generate_docs(format: DocFormat) -> Result<String, DocError> {
    match format {
        DocFormat::OpenApi => {
            let spec = crate::openapi::generate_openapi_spec();
            Ok(serde_json::to_string_pretty(&spec)?)
        }
        DocFormat::SwaggerUi => {
            #[cfg(feature = "http")]
            {
                Ok(generate_swagger_html())
            }
            #[cfg(not(feature = "http"))]
            {
                Ok("<!-- Swagger UI requires the 'http' feature. Enable it to use interactive docs. -->\n".to_string())
            }
        }
        DocFormat::CliMarkdown => Ok(cli_markdown::generate_cli_docs()),
        DocFormat::McpMarkdown => Ok(generate_mcp_markdown()),
        DocFormat::All => {
            let mut out = String::new();
            out.push_str("# OpenAPI Specification\n\n");
            out.push_str(&generate_docs(DocFormat::OpenApi)?);
            out.push_str("\n\n");
            out.push_str("# CLI Documentation\n\n");
            out.push_str(&generate_docs(DocFormat::CliMarkdown)?);
            out.push_str("\n\n");
            out.push_str(&generate_docs(DocFormat::McpMarkdown)?);
            out.push_str("\n\n");
            #[cfg(feature = "http")]
            {
                out.push_str("<!-- Swagger UI: run with --format swagger or visit /swagger-ui/ for interactive docs -->\n");
            }
            #[cfg(not(feature = "http"))]
            {
                out.push_str("<!-- Swagger UI requires the 'http' feature. Enable it to use interactive docs. -->\n");
            }
            Ok(out)
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
#[cfg(feature = "http")]
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
/// 生成 / IO 错误均以 [`DocError`] 返回，不吞掉、不 panic。
pub fn write_docs(format: DocFormat, output_path: &std::path::Path) -> Result<(), DocError> {
    // 路径遍历防护：拒绝包含 `..` 的路径，防止写入到工作目录外的位置。
    if output_path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(DocError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "output path must not contain parent directory components (..)",
        )));
    }
    let content = generate_docs(format)?;
    std::fs::write(output_path, content)?;
    Ok(())
}
