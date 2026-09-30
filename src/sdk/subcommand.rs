// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `sdk` 子命令处理（`sdk` feature；与 `docs` 同一保留子命令拦截范式）。
//!
//! `sdk --lang rust|typescript|all --output-dir <dir> [--reqwest]` 从全局
//! 注册表生成客户端产物文件（`sdforge_client.rs` / `sdforge_client.ts`）。
//! **`sdk` 是保留子命令名**：dispatch 先于用户注册拦截，下游注册同名命令
//! 不可达。

use std::path::PathBuf;

use crate::core::ApiError;
use crate::sdk::generator::{
    collect_grpc_methods, collect_routes, generate_rust_client, generate_typescript_client,
};

/// `sdk --lang` 支持的目标（单一事实源：clap 白名单与一致性测试由此派生）。
pub const SDK_LANG_VALUES: [&str; 3] = ["rust", "typescript", "all"];

/// Rust 产物文件名。
const RUST_FILE: &str = "sdforge_client.rs";
/// TypeScript 产物文件名。
const TS_FILE: &str = "sdforge_client.ts";

/// 构造 `sdk` 子命令的 `clap::Command` 定义。
pub fn sdk_subcommand_definition() -> clap::Command {
    clap::Command::new("sdk")
        .about("Generate client SDKs from the registered routes")
        .arg(
            clap::Arg::new("lang")
                .long("lang")
                .value_parser(clap::builder::PossibleValuesParser::new(SDK_LANG_VALUES))
                .default_value("all")
                .help("SDK target language"),
        )
        .arg(
            clap::Arg::new("output-dir")
                .long("output-dir")
                .value_parser(clap::value_parser!(PathBuf))
                .help("Output directory (defaults to current directory)"),
        )
        .arg(
            clap::Arg::new("reqwest")
                .long("reqwest")
                .action(clap::ArgAction::SetTrue)
                .help("Rust output: append the cfg(reqwest) ReqwestTransport impl"),
        )
}

/// 执行 `sdk` 子命令：收集注册表 → 渲染产物 → 写文件（stdout 不适用——
/// 产物是文件集合，路径经 `--output-dir`）。
///
/// # Errors
/// 目录创建或文件写入失败时返回 [`ApiError::Internal`]。
#[allow(clippy::result_large_err)]
pub fn sdk_subcommand(matches: &clap::ArgMatches) -> Result<(), ApiError> {
    let lang = matches
        .get_one::<String>("lang")
        .map(|s| s.as_str())
        .unwrap_or("all");
    let dir = matches
        .get_one::<PathBuf>("output-dir")
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));
    let reqwest = matches.get_flag("reqwest");

    let routes = collect_routes();
    let grpc_methods = collect_grpc_methods();
    std::fs::create_dir_all(&dir).map_err(|e| {
        ApiError::internal_with_source("sdk output dir creation failed", "sdk_subcommand", e)
    })?;

    let mut written = Vec::new();
    if lang == "rust" || lang == "all" {
        let path = dir.join(RUST_FILE);
        let content = generate_rust_client(&routes, &grpc_methods, reqwest);
        std::fs::write(&path, content).map_err(|e| {
            ApiError::internal_with_source("rust sdk write failed", "sdk_subcommand", e)
        })?;
        written.push(path.display().to_string());
    }
    if lang == "typescript" || lang == "all" {
        let path = dir.join(TS_FILE);
        let content = generate_typescript_client(&routes, &grpc_methods);
        std::fs::write(&path, content).map_err(|e| {
            ApiError::internal_with_source("typescript sdk write failed", "sdk_subcommand", e)
        })?;
        written.push(path.display().to_string());
    }

    println!(
        "generated {} sdk file(s): {}",
        written.len(),
        written.join(", ")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{SDK_LANG_VALUES, sdk_subcommand, sdk_subcommand_definition};

    /// 子命令定义：--lang 白名单与默认值、--output-dir、--reqwest 齐备。
    #[test]
    fn sdk_definition_carries_lang_output_and_reqwest_args() {
        let cmd = sdk_subcommand_definition();
        assert_eq!(cmd.get_name(), "sdk");
        let lang = cmd
            .get_arguments()
            .find(|a| a.get_id().as_str() == "lang")
            .expect("--lang must exist");
        let defaults: Vec<String> = lang
            .get_default_values()
            .iter()
            .map(|v| v.to_string_lossy().into_owned())
            .collect();
        assert_eq!(defaults, vec!["all".to_string()]);
        assert!(
            cmd.get_arguments()
                .any(|a| a.get_id().as_str() == "output-dir"),
            "--output-dir must exist"
        );
        assert!(
            cmd.get_arguments()
                .any(|a| a.get_id().as_str() == "reqwest")
        );
        assert_eq!(SDK_LANG_VALUES, ["rust", "typescript", "all"]);
    }

    /// 端到端：CliBuilder::build() 注入保留子命令 `sdk`；sdk_subcommand 落盘
    /// rust+typescript 两产物到临时目录。
    #[test]
    fn sdk_subcommand_writes_artifacts_to_output_dir() {
        let cmd = crate::cli::CliBuilder::new().build();
        assert!(
            cmd.find_subcommand("sdk").is_some(),
            "sdk feature 启用时 build() 必须包含保留子命令 sdk"
        );

        let dir = std::env::temp_dir().join(format!("sdforge-sdk-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let matches = sdk_subcommand_definition()
            .try_get_matches_from(["sdk", "--output-dir", dir.to_str().expect("utf8 temp dir")])
            .expect("valid args");
        sdk_subcommand(&matches).expect("sdk generation must succeed");

        let rust_artifact =
            std::fs::read_to_string(dir.join("sdforge_client.rs")).expect("rust artifact written");
        let ts_artifact = std::fs::read_to_string(dir.join("sdforge_client.ts"))
            .expect("typescript artifact written");
        assert!(rust_artifact.contains("pub trait Transport"));
        assert!(ts_artifact.contains("export class SdforgeClient"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
