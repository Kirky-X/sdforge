// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `DocFormat::Agent` — Agent 知识包（能力清单/用法 JSON）测试。

use crate::docs::{DocFormat, generate_docs};

// 知识包 fixture：独立注册一条命令，断言其进入 commands 能力清单
// （不依赖其他测试模块的 fixture，避免耦合）。
inventory::submit!(
    crate::cli::CliCommandRegistration::new(
        "agent_knowledge_probe",
        "v9",
        "Agent knowledge probe command",
        "agent_knowledge_probe_handler",
    )
    .with_args(&[crate::cli::CliArgInfo::new(
        "id",
        "Resource id",
        crate::cli::CliArgType::Body,
        true,
        None,
    )])
);

/// Agent 知识包必须是合法 JSON，携带 schema 版本与程序标识。
#[test]
fn agent_knowledge_is_valid_json_with_schema_and_program() {
    let raw = generate_docs(DocFormat::Agent).expect("agent knowledge must generate");
    let v: serde_json::Value =
        serde_json::from_str(&raw).expect("agent knowledge output must be valid JSON");
    assert_eq!(v["schema"], serde_json::json!("sdforge.agent-knowledge/v1"));
    assert!(
        !v["program"]["name"].as_str().unwrap_or_default().is_empty(),
        "program name must be present"
    );
    assert!(
        v["program"]["version"].as_str().is_some(),
        "program version must be present"
    );
}

/// 输出契约段：--format 开关、取值、退出码与流约定全部机器可读。
#[test]
fn agent_knowledge_documents_output_contract() {
    let raw = generate_docs(DocFormat::Agent).expect("agent knowledge must generate");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let contract = &v["output_contract"];
    assert_eq!(contract["format_flag"], serde_json::json!("--format"));
    assert_eq!(
        contract["values"],
        serde_json::json!(["text", "json"]),
        "values must track the built-in --format flag"
    );
    assert_eq!(contract["default_format"], serde_json::json!("text"));
    assert_eq!(contract["success_exit_code"], serde_json::json!(0));
    assert_eq!(contract["error_exit_code"], serde_json::json!(1));
    assert_eq!(
        contract["text_error_stream"], "stderr",
        "text errors keep the stderr convention"
    );
    assert_eq!(
        contract["json_error_stream"], "stdout",
        "json errors move to stdout for machine parsing"
    );
    assert_eq!(
        contract["json_error_shape"], "UnifiedError",
        "json errors share the UnifiedError payload"
    );
}

/// CLI 能力清单：inventory 注册的命令（含参数元数据）全部进入 commands。
#[test]
fn agent_knowledge_lists_registered_cli_commands() {
    let raw = generate_docs(DocFormat::Agent).expect("agent knowledge must generate");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let commands = v["commands"].as_array().expect("commands must be an array");

    let probe = commands
        .iter()
        .find(|c| c["name"] == serde_json::json!("agent_knowledge_probe"))
        .expect("registered command must appear in commands");
    assert_eq!(probe["version"], serde_json::json!("v9"));
    assert_eq!(
        probe["description"],
        serde_json::json!("Agent knowledge probe command")
    );
    let args = probe["args"].as_array().expect("args must be an array");
    assert_eq!(args.len(), 1);
    assert_eq!(args[0]["name"], serde_json::json!("id"));
    assert_eq!(args[0]["kind"], serde_json::json!("body"));
    assert_eq!(args[0]["required"], serde_json::json!(true));
}

/// MCP 工具清单：`mcp` feature 启用时非空（coverage_test_tool 存在于
/// 测试构建），字段含 name/description/input_schema；未启用时为空数组。
#[test]
fn agent_knowledge_lists_mcp_tools() {
    let raw = generate_docs(DocFormat::Agent).expect("agent knowledge must generate");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let tools = v["mcp_tools"]
        .as_array()
        .expect("mcp_tools must be an array");

    #[cfg(feature = "mcp")]
    {
        assert!(
            !tools.is_empty(),
            "mcp feature is on: registered tools must be listed"
        );
        let coverage = tools
            .iter()
            .find(|t| t["name"] == serde_json::json!("coverage_test_tool"))
            .expect("coverage_test_tool must be listed");
        assert!(coverage["description"].as_str().is_some());
        assert!(
            coverage["input_schema"].is_object(),
            "input_schema must be embedded for agent consumption"
        );
    }
    #[cfg(not(feature = "mcp"))]
    {
        assert!(
            tools.is_empty(),
            "mcp feature is off: mcp_tools must be an empty array"
        );
    }
}
