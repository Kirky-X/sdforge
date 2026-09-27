// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Agent 知识包生成——CLI 能力清单 / 用法契约的机器可读 JSON。
//!
//! 单个 JSON 文档（schema `sdforge.agent-knowledge/v1`）让 Agent/CI 一次
//! 拉取：程序标识、`--format` 输出契约（取值/退出码/流约定）、全部注册的
//! CLI 子命令（含参数元数据）与 MCP 工具（`mcp` feature 启用时）。
//!
//! 能力清单直接读 `inventory` 注册表（`CliCommandRegistration` /
//! `McpToolRegistration`），与 `CliBuilder::build()` / `get_mcp_tools()`
//! 同源——注册即入包，无独立维护面。

use serde_json::{Value, json};

/// 生成 Agent 知识包 JSON（紧凑单行）。
pub fn generate_agent_knowledge() -> Value {
    json!({
        "schema": "sdforge.agent-knowledge/v1",
        "program": {
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
            "description": "SDForge multi-protocol CLI",
        },
        "output_contract": output_contract(),
        "commands": cli_commands(),
        "mcp_tools": mcp_tools(),
    })
}

/// `--format` 输出契约（与 `crate::cli::output` 的实现一致——
/// 修改契约时同步更新该模块与文档）。
fn output_contract() -> Value {
    json!({
        "format_flag": "--format",
        "values": ["text", "json"],
        "default_format": "text",
        "success_exit_code": 0,
        "error_exit_code": 1,
        "text_success_stream": "stdout",
        "text_error_stream": "stderr",
        "text_error_prefix": "error:",
        "json_success_stream": "stdout",
        "json_error_stream": "stdout",
        "json_error_shape": "UnifiedError",
    })
}

/// 全部注册的 CLI 子命令（`State` 参数不外露——它们由宿主注入）。
fn cli_commands() -> Value {
    let commands: Vec<Value> = inventory::iter::<crate::cli::CliCommandRegistration>()
        .map(|reg| {
            let args: Vec<Value> = reg
                .args
                .iter()
                .filter(|a| !matches!(a.arg_type, crate::cli::CliArgType::State))
                .map(|a| {
                    json!({
                        "name": a.name,
                        "description": a.description,
                        "kind": match a.arg_type {
                            crate::cli::CliArgType::Path => "path",
                            crate::cli::CliArgType::Body => "body",
                            crate::cli::CliArgType::State => "state",
                        },
                        "required": a.required,
                        "default": a.default,
                    })
                })
                .collect();
            json!({
                "name": reg.name,
                "version": reg.version,
                "description": reg.description,
                "args": args,
            })
        })
        .collect();
    Value::Array(commands)
}

/// 全部注册的 MCP 工具（`mcp` feature 未启用时为空数组）。
fn mcp_tools() -> Value {
    #[cfg(feature = "mcp")]
    {
        let tools: Vec<Value> = crate::mcp::get_mcp_tools()
            .iter()
            .map(|instance| {
                let tool = instance.tool();
                json!({
                    "name": tool.name(),
                    "version": instance.metadata().version(),
                    "description": tool.description(),
                    "input_schema": tool.input_schema(),
                    "roles": instance.roles(),
                })
            })
            .collect();
        Value::Array(tools)
    }
    #[cfg(not(feature = "mcp"))]
    {
        Value::Array(Vec::new())
    }
}
