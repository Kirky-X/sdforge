// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! MCP 工具列表 Markdown 生成。
//!
//! 整个文件需 `mcp` feature 门控 — 只有同时启用 `docs` + `mcp` 时才编译。
//! 迭代 `inventory::iter::<McpToolRegistration>`，每个工具输出 Markdown 段落。

use crate::core::Registration;
use crate::mcp::McpToolRegistration;

/// 生成 MCP 工具列表 Markdown 文档。
///
/// 迭代所有 `inventory` 注册的 `McpToolRegistration`，每个工具输出：
/// ```markdown
/// ## {tool.name}
///
/// {tool.metadata.description}
///
/// Version: {tool.version}
/// ```
///
/// 顶部加 `# MCP Tools` 标题。
pub fn generate_mcp_docs() -> String {
    let mut out = format!("# {}\n\n", crate::i18n::t("docs-mcp-tools-heading", &[]));

    for reg in inventory::iter::<McpToolRegistration> {
        let metadata = reg.metadata();
        let version = crate::i18n::t(
            "docs-mcp-version",
            &[("version", reg.version().to_string())],
        );
        out.push_str(&format!(
            "## {}\n\n{}\n\n{}\n\n",
            reg.name(),
            metadata.description(),
            version
        ));
    }

    out
}
