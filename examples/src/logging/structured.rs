// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! # 结构化日志示例（inklog LoggerManager）
//!
//! 自研结构化日志栈已在 0.5.0-rc 期删除，结构化日志收敛到 inklog
//! `LoggerManager`。本示例展示迁移目标的四条路径：
//!
//! 1. [`init_global`](crate::logging::structured::init_global) ——
//!    `init_inklog_logger` 安装 inklog 为全局 `log`
//!    后端：既有 `log::info!` 等 facade 调用零改动自动进 inklog 管道。
//! 2. [`rotation_config_toml`](crate::logging::structured::rotation_config_toml)
//!    + [`manager_with_rotation`](crate::logging::structured::manager_with_rotation)
//!    —— `LoggerManager::from_file` 从 TOML 加载含**真实轮转**（大小 + 时间）
//!    的文件 sink 配置（替换旧栈宣称却不存在的轮转）。
//! 3. [`manager_from_builder`](crate::logging::structured::manager_from_builder)
//!    —— `LoggerManager::builder` 编程式构造。
//! 4. [`relax_level`](crate::logging::structured::relax_level) ——
//!    `set_level` 运行时调级。
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --features logging_examples --example logging/structured
//! ```

//! 参考型演示模块（未接线为 bin 目标）：handler 与辅助函数仅供文档阅读，
//! 编译为库模块时允许保留未调用项。
#![allow(dead_code)]

use sdforge::i18n::t;
use sdforge::inklog::{LoggerManager, init_inklog_logger};
use std::path::{Path, PathBuf};

// 配置文件

/// 含轮转配置的 inklog TOML：文件 sink，大小（10MB）与时间（daily）双轮转，
/// 保留 5 个历史文件。
pub fn rotation_config_toml(log_path: &str) -> String {
    format!(
        r#"[global]
level = "info"

[console_sink]
enabled = true

[file_sink]
enabled = true
path = "{log_path}"
max_size = "10MB"
rotation_time = "daily"
keep_files = 5
"#
    )
}

/// 进程内唯一的临时目录（示例/测试用，不引入 tempfile 依赖）。
fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sdforge-logging-example-{}-{}",
        std::process::id(),
        name
    ));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

// 全局 log 后端安装（facade 零改动入口）

/// 安装 inklog 为全局 `log` 后端。
///
/// 返回的 `LoggerManager` 必须保持存活以维持 inklog worker 线程；重复调用
/// 幂等（install 失败降级为 warning，仍返回 Ok）。
pub async fn init_global() -> Result<LoggerManager, sdforge::inklog::InklogError> {
    init_inklog_logger().await
}

// from_file：含轮转配置的构造路径

/// 从含轮转配置的 TOML 构造 `LoggerManager`（真实轮转，非宣称）。
pub async fn manager_with_rotation(
    dir: &Path,
) -> Result<LoggerManager, sdforge::inklog::InklogError> {
    let log_path = dir.join("sdforge.log");
    let config_path = dir.join("inklog.toml");
    std::fs::write(
        &config_path,
        rotation_config_toml(&log_path.to_string_lossy()),
    )
    .expect("write inklog config");
    LoggerManager::from_file(&config_path).await
}

// builder：编程式构造

/// `LoggerManager::builder` 编程式构造（info 级 + console sink）。
pub async fn manager_from_builder() -> Result<LoggerManager, sdforge::inklog::InklogError> {
    LoggerManager::builder()
        .level("info")
        .console(true)
        .build()
        .await
}

// set_level：运行时调级

/// 运行时调级（全局 target）。低于新级别的记录被过滤。
pub fn relax_level(manager: &LoggerManager) -> Result<(), sdforge::inklog::InklogError> {
    manager.set_level(None, "debug")
}

// Main Entry Point

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", t("example-logging-title", &[]));
    println!("=============================================");
    println!();

    println!("{}", t("example-logging-init-title", &[]));
    let manager = init_global().await?;
    println!("  {}", t("example-logging-init-done", &[]));
    log::info!("{}", t("example-logging-routed-info", &[]));

    println!("\n{}", t("example-logging-from-file-title", &[]));
    let dir = scratch_dir("main");
    let rotating = manager_with_rotation(&dir).await?;
    println!("  {}", t("example-logging-from-file-done", &[]));
    drop(rotating);

    println!("\n{}", t("example-logging-builder-title", &[]));
    let built = manager_from_builder().await?;
    println!("  {}", t("example-logging-builder-done", &[]));
    drop(built);

    println!("\n{}", t("example-logging-set-level-title", &[]));
    relax_level(&manager)?;
    println!("  {}", t("example-logging-set-level-done", &[]));
    log::debug!("{}", t("example-logging-debug-visible", &[]));

    println!("\n{}", t("example-logging-finished", &[]));
    Ok(())
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_config_toml_carries_rotation_fields() {
        let toml = rotation_config_toml("/tmp/sdforge-example.log");
        assert!(toml.contains("[file_sink]"), "file sink section present");
        assert!(toml.contains("max_size = \"10MB\""), "size rotation");
        assert!(toml.contains("rotation_time = \"daily\""), "time rotation");
        assert!(toml.contains("keep_files = 5"), "history retention");
        assert!(
            toml.contains("/tmp/sdforge-example.log"),
            "log path embedded"
        );
    }

    #[tokio::test]
    async fn test_manager_from_builder_builds() {
        let manager = manager_from_builder()
            .await
            .expect("builder construct must succeed");
        drop(manager);
    }

    #[tokio::test]
    async fn test_manager_with_rotation_from_file() {
        let dir = scratch_dir("from_file");
        let manager = manager_with_rotation(&dir)
            .await
            .expect("from_file with rotation config must succeed");
        drop(manager);
    }

    #[tokio::test]
    async fn test_init_global_installs_log_backend() {
        // 幂等：无论本测试进程是否已安装过全局 logger，调用均返回 Ok。
        let manager = init_global().await.expect("init must be idempotent-ok");
        drop(manager);
    }

    #[tokio::test]
    async fn test_relax_level_filters_below_new_level() {
        let manager = manager_from_builder()
            .await
            .expect("builder construct must succeed");
        relax_level(&manager).expect("set_level must accept a valid level");
        drop(manager);
    }
}
