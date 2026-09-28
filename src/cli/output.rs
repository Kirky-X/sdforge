// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `--format` 机器可读输出契约。
//!
//! [`CliBuilder`](super::CliBuilder) 内建挂载全局 `--format text|json` 开关，
//! [`OutputFormat`] 承载两种模式下的渲染与流约定（`execute` 为 `-> !` 不可
//! 进程内测试，全部逻辑收敛为这里的纯函数）：
//!
//! | 模式 | 成功 | 错误 | 退出码 |
//! |------|------|------|--------|
//! | `text`（默认） | `extract_value` → stdout | `error: <e>` → stderr | 0 / 1 |
//! | `json` | handler `Value` 紧凑 JSON → stdout | `UnifiedError` JSON → stdout | 0 / 1 |
//!
//! json 错误载荷复用 [`crate::error::UnifiedError`]（HTTP/gRPC 同一形状：
//! `{"code","message","trace_id"?,"field"?}`），Agent/CI 按单一契约解析。

use clap::ArgMatches;
use serde_json::Value;

use crate::core::ApiError;
use crate::error::UnifiedError;

/// `--format` 参数在 clap 中的 arg id / long 名。
pub const FORMAT_ARG: &str = "format";

/// `--format` 的全部合法取值（单一事实源：`format_arg()` 的白名单与
/// Agent 知识包的 `output_contract.values` 都从这里派生）。
pub const FORMAT_VALUES: [&str; 2] = ["text", "json"];

/// 默认取值（[`OutputFormat::default`] 对应的命令行字符串）。
pub const FORMAT_DEFAULT: &str = "text";

/// 成功退出码。
pub const SUCCESS_EXIT_CODE: i32 = 0;

/// 错误退出码。
pub const ERROR_EXIT_CODE: i32 = 1;

/// `--format` 的帮助文案（`build()` 挂载时使用）。
pub const FORMAT_HELP: &str = "Output format: text (human-readable, default) or json (machine-readable; errors as UnifiedError JSON on stdout)";

/// CLI 输出模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// 人类可读（默认，向后兼容）。
    #[default]
    Text,
    /// 机器可读 JSON。
    Json,
}

impl OutputFormat {
    /// 从顶层 `ArgMatches` 解析输出模式。
    ///
    /// `--format` 由 `build()` 内建挂载且带 `default_value("text")`，
    /// 缺省即 [`OutputFormat::Text`]；未知值已在 clap `value_parser` 层拒绝。
    pub fn from_matches(matches: &ArgMatches) -> Self {
        match matches.get_one::<String>(FORMAT_ARG).map(String::as_str) {
            Some("json") => Self::Json,
            _ => Self::Text,
        }
    }

    /// 渲染成功结果（调用方写入 stdout）。
    ///
    /// `Text` 保持 `crate::core::extract_value` 语义（字符串去引号、其余
    /// 紧凑 JSON）；`Json` 为 `Value` 的紧凑 JSON 序列化。
    pub fn render_success(&self, value: &Value) -> String {
        match self {
            Self::Text => crate::core::extract_value(value),
            Self::Json => value.to_string(),
        }
    }

    /// 渲染错误（`Json` 模式含 `error: ` 无前缀的 UnifiedError JSON 行）。
    pub fn render_error(&self, err: &ApiError) -> String {
        match self {
            Self::Text => format!("error: {err}"),
            Self::Json => UnifiedError::from(err).to_json().to_string(),
        }
    }

    /// 按契约输出错误并返回应使用的目标流提示。
    ///
    /// `Text` → stderr（既有惯例）；`Json` → stdout（机器可读单流）。
    pub fn emit_error(&self, err: &ApiError) {
        match self {
            Self::Text => eprintln!("{}", self.render_error(err)),
            Self::Json => println!("{}", self.render_error(err)),
        }
    }
}

/// 构造内建的 `--format` 全局 clap 参数（`build()` 挂载）。
///
/// `global(true)` 传播到全部子命令；取值限定 `text|json`，默认 `text`。
/// `docs` 子命令的同名 `--format`（文档格式）由 clap 的子命令作用域遮蔽，
/// 两者语义独立、可同时使用（`prog --format json docs --format all`）。
pub(crate) fn format_arg() -> clap::Arg {
    clap::Arg::new(FORMAT_ARG)
        .long(FORMAT_ARG)
        .global(true)
        .value_parser(FORMAT_VALUES)
        .default_value(FORMAT_DEFAULT)
        .help(FORMAT_HELP)
}

/// CLI text 模式的认证失败文案（历史兼容）。
///
/// `ApiError::AuthenticationFailed` 的 `Display` 是大写
/// `Authentication failed: …`（HTTP 共用，不可改）；CLI 历史输出为小写
/// `authentication failed: …`，按旧字符串匹配的脚本依赖该形态，故 CLI
/// text 路径显式构造小写文案（json 模式走 UnifiedError，不受影响）。
///
/// 唯一调用点是 `execute` 的凭据预检（`security` 特性），定义随之同门
/// 编译，避免 `cli` 单独启用时产生 dead_code。
#[cfg(feature = "security")]
pub(crate) fn auth_failure_text(reason: &str) -> String {
    format!("error: authentication failed: {reason}")
}
