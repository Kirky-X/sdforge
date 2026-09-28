// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `--format` 机器可读输出契约测试。
//!
//! 契约（execute 的输出层，`-> !` 不可进程内测试，故全部收敛到纯函数）：
//! - `--format text`（默认）：成功 `extract_value` 到 stdout；错误
//!   `error: <e>` 到 stderr；退出码 成功 0 / 错误 1
//! - `--format json`：成功为 handler `Value` 的紧凑 JSON；错误为
//!   `UnifiedError` JSON（`code`/`message`/`trace_id`/`field`）——两者都走
//!   stdout；退出码契约同 text

use crate::cli::CliBuilder;
use crate::cli::output::OutputFormat;
use crate::core::ApiError;
use clap::Command;
use serde_json::json;

/// 构造带 `--format` 全局开关的 clap Command 并解析参数。
fn parse<const N: usize>(args: [&str; N]) -> Result<clap::ArgMatches, clap::Error> {
    let cmd = CliBuilder::new().with_name("fmt_prog").build();
    cmd.try_get_matches_from(args)
}

/// build() 必须内建挂载 `--format` 全局开关（text|json），
/// 下游无需自定义 GlobalArg 即得机器可读输出契约。
#[test]
fn build_mounts_format_flag_with_value_constraint() {
    let cmd: Command = CliBuilder::new().build();
    let arg = cmd
        .get_arguments()
        .find(|a| a.get_id() == "format")
        .expect("--format flag must be built in");
    assert!(
        arg.is_global_set(),
        "--format must propagate to subcommands"
    );
    let possible = arg.get_possible_values();
    let values: Vec<&str> = possible.iter().map(|v| v.get_name()).collect();
    assert_eq!(values, vec!["text", "json"]);
}

/// 省略 `--format` → Text（向后兼容默认）。
#[test]
fn format_defaults_to_text() {
    let matches = parse(["fmt_prog", "dispatch_test_greet"]).unwrap();
    assert!(matches!(
        OutputFormat::from_matches(&matches),
        OutputFormat::Text
    ));
}

/// `--format json` → Json。
#[test]
fn format_json_is_parsed() {
    let matches = parse(["fmt_prog", "--format", "json", "dispatch_test_greet"]).unwrap();
    assert!(matches!(
        OutputFormat::from_matches(&matches),
        OutputFormat::Json
    ));
}

/// 非法值被 clap 拒绝（value_parser 限定 text|json）。
#[test]
fn format_rejects_unknown_values() {
    assert!(parse(["fmt_prog", "--format", "yaml"]).is_err());
}

/// text 成功渲染 = extract_value 语义（字符串去引号、其余紧凑 JSON）。
#[test]
fn text_render_success_uses_extract_value() {
    let fmt = OutputFormat::Text;
    assert_eq!(fmt.render_success(&json!("hello")), "hello");
    assert_eq!(fmt.render_success(&json!({"a": 1})), r#"{"a":1}"#);
}

/// json 成功渲染 = Value 的紧凑 JSON 序列化（字符串含引号，合法 JSON）。
#[test]
fn json_render_success_serializes_value() {
    let fmt = OutputFormat::Json;
    assert_eq!(fmt.render_success(&json!("hello")), r#""hello""#);
    assert_eq!(fmt.render_success(&json!({"a": 1})), r#"{"a":1}"#);
}

/// json 错误渲染 = UnifiedError JSON 形状（code/message，可选字段缺省省略）。
#[test]
fn json_render_error_uses_unified_shape() {
    let fmt = OutputFormat::Json;
    let err = ApiError::AuthenticationFailed {
        reason: "missing bearer token".to_string(),
    };
    let rendered = fmt.render_error(&err);
    let parsed: serde_json::Value =
        serde_json::from_str(&rendered).expect("json error output must be valid JSON");
    assert_eq!(parsed["code"], serde_json::json!("UNAUTHORIZED"));
    assert!(
        parsed["message"]
            .as_str()
            .unwrap_or_default()
            .contains("missing bearer token")
    );
    assert!(parsed.get("trace_id").is_none() || parsed["trace_id"].is_null());
}

/// text 错误渲染保持既有 `error: <e>` 前缀（stderr 行）。
#[test]
fn text_render_error_keeps_error_prefix() {
    let fmt = OutputFormat::Text;
    let err = ApiError::NotFound {
        resource: "cli_command".to_string(),
        resource_id: Some("nope".to_string()),
    };
    let rendered = fmt.render_error(&err);
    assert!(rendered.starts_with("error: "), "got: {rendered}");
    assert!(rendered.contains("Resource not found"));
}

/// CLI text 认证失败文案保持历史小写形态（`error: authentication
/// failed: …`），与 HTTP 共用的 Display（大写 A）隔离——按旧字符串
/// 匹配的脚本不因本 feature 破坏。
#[cfg(feature = "security")]
#[test]
fn auth_failure_text_keeps_lowercase_compat() {
    assert_eq!(
        crate::cli::output::auth_failure_text("missing bearer token"),
        "error: authentication failed: missing bearer token"
    );
}
