// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `docs` 模块测试入口。
//!
//! 按关注点拆分到各子文件：
//! - `mod_tests`: `DocFormat` / `generate_docs` / `write_docs`
//! - `swagger_tests`: Swagger UI Router（T014 添加）
//! - `cli_markdown_tests`: CLI Markdown 生成（T016 添加）
//! - `mcp_markdown_tests`: MCP Markdown 生成（T018 添加，`mcp` feature 门控）

mod cli_markdown_tests;
#[cfg(feature = "mcp")]
mod mcp_markdown_tests;
mod mod_tests;
mod swagger_tests;
