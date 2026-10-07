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
//! 同源——注册即入包，无独立维护面。输出契约的数值（取值/默认/退出码）
//! 引用 `cli::output` 的公开常量，与实现保持编译期同源；`docs` 子命令的
//! 独立 `--format` 语义与自输出行为在 `output_contract.exceptions` 显式
//! 声明，解析方不会把它误当普通命令结果。

use serde_json::{Value, json};

use crate::cli::docs_subcommand::{DEFAULT_DOC_FORMAT, FORMAT_VALUES as DOC_FORMAT_VALUES};
use crate::cli::output::{
    ERROR_EXIT_CODE, FORMAT_ARG, FORMAT_DEFAULT, FORMAT_VALUES, SUCCESS_EXIT_CODE,
};

/// 生成 Agent 知识包 JSON（紧凑单行）。
///
/// `program.name` 为 sdforge 库名（编译期 `CARGO_PKG_NAME`）；宿主二进制
/// 名可能不同（`CliBuilder::with_name`），需要宿主标识的下游改用
/// [`generate_agent_knowledge_for_host`]。
pub fn generate_agent_knowledge() -> Value {
    generate_agent_knowledge_for_host(env!("CARGO_PKG_NAME"), "SDForge multi-protocol CLI")
}

/// 以宿主程序标识生成 Agent 知识包。
///
/// 宿主应用用 `CliBuilder::with_name` 自定义二进制名后，据此传入同一
/// 名字，使知识包 `program` 段与实际二进制一致。
pub fn generate_agent_knowledge_for_host(name: &str, description: &str) -> Value {
    json!({
        "schema": "sdforge.agent-knowledge/v1",
        "program": {
            "name": name,
            "description": description,
            "version": env!("CARGO_PKG_VERSION"),
            // name 语义：无参版本为 sdforge 库名；本变体为宿主传入的
            // 二进制名。声明字段避免消费方混淆。
            "name_semantics": "host binary name (or the sdforge library name when generated without arguments)",
        },
        "output_contract": output_contract(),
        "commands": cli_commands(),
        "mcp_tools": mcp_tools(),
    })
}

/// `--format` 输出契约（数值派生自 `cli::output` 公开常量——与实现
/// 编译期同源，无手写副本）。
fn output_contract() -> Value {
    json!({
        "format_flag": format!("--{FORMAT_ARG}"),
        "values": FORMAT_VALUES,
        "default_format": FORMAT_DEFAULT,
        "success_exit_code": SUCCESS_EXIT_CODE,
        "error_exit_code": ERROR_EXIT_CODE,
        "text_success_stream": "stdout",
        "text_error_stream": "stderr",
        "text_error_prefix": "error:",
        "json_success_stream": "stdout",
        "json_error_stream": "stdout",
        "json_error_shape": "UnifiedError",
        // handler 返回 null（或 docs 等自输出子命令的哨兵）时 stdout
        // 不产生任何输出——调用方按空输出处理，而非字面 "null"。
        "null_return": "no output on stdout",
        // 例外声明：docs 子命令拥有独立的 --format（文档格式，非本契约
        // 的取值集合），且其输出为生成的文档文本——即使全局 --format json
        // 也不会被 JSON 渲染包装；解析 `docs` 的 stdout 时按文档产物处理。
        "exceptions": [
            {
                "command": "docs",
                "independent_format_flag": true,
                "self_emitted_output": true,
                "note": "docs emits generated documentation text itself; global --format json does not wrap its output",
            }
        ],
    })
}

/// 全部注册的 CLI 子命令（`State` 参数不外露——它们由宿主注入）。
/// `docgen` feature 启用时追加内建 `docs` 子命令的静态条目（它不经
/// inventory 注册，但属于实际 CLI 面，Agent 必须可见）。
fn cli_commands() -> Value {
    let mut commands: Vec<Value> = inventory::iter::<crate::cli::CliCommandRegistration>()
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

    #[cfg(feature = "docgen")]
    commands.push(json!({
        "name": "docs",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Generate documentation files",
        "built_in": true,
        "args": [
            {
                "name": "format",
                "description": "Documentation format (independent of the global --format output contract)",
                "kind": "body",
                "required": false,
                "default": DEFAULT_DOC_FORMAT,
                "values": DOC_FORMAT_VALUES,
            },
            {
                "name": "output",
                "description": "Output file path (stdout if omitted)",
                "kind": "body",
                "required": false,
                "default": null,
            },
        ],
    }));

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
