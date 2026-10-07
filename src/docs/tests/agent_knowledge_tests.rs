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

// ============================================================================
// 契约一致性锁定（审查修复）：知识包 output_contract 与 cli::output 实现
// 同源，任一侧漂移都会在此失败。
// ============================================================================

/// output_contract 的 values/default 必须与 format_arg() 的 clap 白名单、
/// OutputFormat::default() 完全一致（防手写副本静默漂移）。
#[test]
fn output_contract_matches_format_arg_whitelist_and_default() {
    let raw = generate_docs(DocFormat::Agent).expect("agent knowledge must generate");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let contract = &v["output_contract"];

    // values 与 format_arg() 的 clap possible values 一致
    let arg = crate::cli::output::format_arg();
    let possible = arg.get_possible_values();
    let mut whitelist: Vec<&str> = possible.iter().map(|pv| pv.get_name()).collect();
    whitelist.sort_unstable();
    // 常量单一事实源：declared 与 FORMAT_VALUES 常量严格一致（声明序）
    assert_eq!(
        contract["values"],
        serde_json::json!(crate::cli::output::FORMAT_VALUES)
    );
    let mut declared: Vec<&str> = contract["values"]
        .as_array()
        .expect("values must be an array")
        .iter()
        .map(|v| v.as_str().expect("values entries must be strings"))
        .collect();
    declared.sort_unstable();
    assert_eq!(
        declared, whitelist,
        "contract values must track the clap whitelist"
    );

    // default_format 与 OutputFormat::default() 一致
    let default_str = match crate::cli::output::OutputFormat::default() {
        crate::cli::output::OutputFormat::Text => "text",
        crate::cli::output::OutputFormat::Json => "json",
    };
    assert_eq!(contract["default_format"], serde_json::json!(default_str));
    assert_eq!(
        contract["default_format"],
        serde_json::json!(crate::cli::output::FORMAT_DEFAULT)
    );

    // 退出码与常量一致
    assert_eq!(
        contract["success_exit_code"],
        serde_json::json!(crate::cli::output::SUCCESS_EXIT_CODE)
    );
    assert_eq!(
        contract["error_exit_code"],
        serde_json::json!(crate::cli::output::ERROR_EXIT_CODE)
    );

    // format_flag 与 FORMAT_ARG 派生一致
    assert_eq!(
        contract["format_flag"],
        serde_json::json!(format!("--{}", crate::cli::output::FORMAT_ARG))
    );
}

/// docs 子命令的例外必须显式声明：独立 --format 语义、自输出行为、
/// 内建条目进入 commands 清单——机器消费方不会误当普通命令结果解析。
#[test]
fn output_contract_declares_docs_exceptions() {
    let raw = generate_docs(DocFormat::Agent).expect("agent knowledge must generate");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let contract = &v["output_contract"];

    // null 哨兵行为登记
    assert_eq!(
        contract["null_return"],
        serde_json::json!("no output on stdout")
    );

    // exceptions 声明 docs 的独立 --format 与自输出
    let exceptions = contract["exceptions"].as_array().expect("exceptions array");
    let docs_exc = exceptions
        .iter()
        .find(|e| e["command"] == serde_json::json!("docs"))
        .expect("docs exception must be declared");
    assert_eq!(docs_exc["independent_format_flag"], serde_json::json!(true));
    assert_eq!(docs_exc["self_emitted_output"], serde_json::json!(true));
}

/// docgen feature 下内建 docs 子命令进入 commands 清单（built_in 标记 +
/// 独立 format 参数取值），Agent 可见实际 CLI 面。
#[test]
#[cfg(feature = "docgen")]
fn commands_include_builtin_docs_entry() {
    let raw = generate_docs(DocFormat::Agent).expect("agent knowledge must generate");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let docs_entry = v["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == serde_json::json!("docs"))
        .expect("built-in docs command must be listed");
    assert_eq!(docs_entry["built_in"], serde_json::json!(true));
    let args = docs_entry["args"].as_array().unwrap();
    // values/default 清单与 FORMAT_VALUES 常量编译期同源（同一常量派生
    // 两侧），自反断言无信息量——映射正确性由
    // parse_format_maps_every_declared_doc_format 独立锁定。
    let format_arg_entry = args.iter().find(|a| a["name"] == "format").unwrap();
    assert_eq!(format_arg_entry["default"], serde_json::json!("all"));
}

/// 声明的每个文档格式必须命中 `parse_format` 的显式映射分支：
/// 新增 FORMAT_VALUES 值而漏加 match 分支时，该测试以 panic 失败
/// （unreachable! loud-fail），而不是等到运行时文档请求才暴露。
#[test]
fn parse_format_maps_every_declared_doc_format() {
    for value in crate::cli::docs_subcommand::FORMAT_VALUES {
        let format = crate::cli::docs_subcommand::parse_format(value);
        match value {
            "openapi" => assert!(matches!(format, DocFormat::OpenApi)),
            "swagger" => assert!(matches!(format, DocFormat::SwaggerUi)),
            "cli-markdown" => assert!(matches!(format, DocFormat::CliMarkdown)),
            "mcp-markdown" => assert!(matches!(format, DocFormat::McpMarkdown)),
            "all" => assert!(matches!(format, DocFormat::All)),
            "agent" => assert!(matches!(format, DocFormat::Agent)),
            // FORMAT_VALUES 中新出现、parse_format 未显式映射的值——
            // 全局 --format 穿透值（text/json）已在 parse_format 显式
            // 回落 All，其余即扩展遗漏，立即失败。
            other => panic!(
                "FORMAT_VALUES 值 {other:?} 在 parse_format 中缺少显式映射（新增格式需同步 match 分支）"
            ),
        }
    }
}

/// generate_agent_knowledge_for_host：宿主标识进入 program 段，
/// name_semantics 字段声明名字来源语义。
#[test]
fn host_variant_uses_provided_identity() {
    let v = crate::docs::generate_agent_knowledge_for_host("my-tool", "My tool description");
    assert_eq!(v["program"]["name"], serde_json::json!("my-tool"));
    assert_eq!(
        v["program"]["description"],
        serde_json::json!("My tool description")
    );
    assert!(v["program"]["name_semantics"].as_str().is_some());
}
