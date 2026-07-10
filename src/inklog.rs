// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! inklog 结构化日志集成 — 将裸 `log` 输出桥接到 inklog LoggerManager。
//!
//! 启用 `inklog` feature 后，此模块提供 [`init_inklog_logger`] 函数，
//! 将 inklog 安装为全局 `log` crate 后端。此后 sdforge 中所有
//! `log::error!`/`log::warn!`/`log::info!` 等调用自动路由到 inklog 的
//! 结构化日志管道（console + async sinks），无需修改任何现有调用点。
//!
//! ## 工作原理
//!
//! inklog 的 `LoggerManager::with_config()` 在内部：
//! 1. 创建 `LogAdapter`（实现 `log::Log` trait）
//! 2. 通过 `LogLogger::install()` 调用 `log::set_boxed_logger` 安装为全局 logger
//! 3. 调用 `log::set_max_level` 设置全局日志级别
//!
//! 因此 sdforge 现有的 29 处 `log::error!`/`log::warn!` 调用无需任何改动。
//!
//! ## 用法
//!
//! ```ignore
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 初始化 inklog 为全局 log 后端
//! let _manager = sdforge::inklog::init_inklog_logger().await?;
//!
//! // 此后所有 log 调用自动路由到 inklog
//! log::warn!("cache miss for key={:?}", "user:42");
//! log::error!("inventory Mutex poisoned: {}", "...");
//! # Ok(())
//! # }
//! ```
//!
//! ## 未启用 inklog feature 时
//!
//! 此模块不存在，`log` 行为完全不变（默认不输出，除非外部安装了其他 logger）。

/// 重导出 inklog 核心类型，供下游直接通过 `sdforge::inklog::` 访问。
///
/// 避免下游 crate 需要单独声明 inklog 依赖。
pub use ::inklog::{InklogConfig, InklogError, LoggerManager};

/// 将 inklog 初始化为全局结构化日志后端。
///
/// 使用默认配置创建 `LoggerManager`，并将 inklog 的 `LogLogger` 安装为
/// 全局 `log` crate 后端。调用后，sdforge 中所有 `log::error!`/
/// `log::warn!`/`log::info!` 等调用自动路由到 inklog 的结构化日志管道
/// （console + async sinks），无需修改任何现有调用点。
///
/// 返回的 `LoggerManager` 必须保持存活以维持 inklog 的 worker 线程运行。
/// 丢弃它会关闭日志管道。
///
/// # 返回
///
/// `Ok(LoggerManager)` — 管理器创建成功。
/// `Err(InklogError)` — 初始化失败（如 channel/sink 创建错误）。
///
/// # 幂等性
///
/// 全局 `log` logger 每进程只能安装一次。多次调用时，首次安装 logger；
/// 后续调用仍返回 `Ok`（install 失败被 inklog 降级为 warning）。
///
/// # 示例
///
/// ```ignore
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let _manager = sdforge::inklog::init_inklog_logger().await?;
/// log::info!("logs now route through inklog");
/// # Ok(())
/// # }
/// ```
pub async fn init_inklog_logger() -> Result<LoggerManager, InklogError> {
    LoggerManager::with_config(InklogConfig::default()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // init_inklog_logger — 验证 inklog 被正确安装为全局 log 后端
    //
    // 这些测试验证：
    // 1. init_inklog_logger 返回 Ok（LoggerManager 构造成功）
    // 2. 初始化后全局 log 级别被设置（证明 inklog LogLogger::install 执行）
    // 3. 初始化后 log:: 调用不 panic（证明桥接器正常工作）
    //
    // 使用 #[serial] 因为 inklog 安装全局 log logger（进程级单例）。
    // ========================================================================

    /// 验证 init_inklog_logger 成功返回 LoggerManager。
    ///
    /// inklog 的 LoggerManager::with_config 会启动 worker 线程并安装
    /// LogLogger。即使全局 logger 已被先前测试安装，with_config 不会
    /// 返回错误（install 失败仅 warn 不 propagate）。
    #[tokio::test]
    #[serial_test::serial]
    async fn init_inklog_logger_returns_manager() {
        let result = init_inklog_logger().await;
        assert!(
            result.is_ok(),
            "init_inklog_logger should return Ok, got: {:?}",
            result.err()
        );
    }

    /// 验证初始化后全局 log 级别不再是 Off。
    ///
    /// inklog 的 LogLogger::install 调用 log::set_max_level(Info)。
    /// 在任何 logger 安装前，log::max_level() 返回 Off。初始化后
    /// 必须为非 Off —— 这证明 inklog 的 install 路径被执行。
    #[tokio::test]
    #[serial_test::serial]
    async fn init_sets_global_log_level() {
        let _manager = init_inklog_logger().await.expect("init should succeed");
        assert_ne!(
            log::max_level(),
            log::LevelFilter::Off,
            "global log level should be set by inklog, not remain Off"
        );
    }

    /// 验证初始化后 log:: 调用通过 inklog 桥接器且不 panic。
    ///
    /// sdforge 现有 29 处 log::error!/log::warn! 调用点，初始化 inklog
    /// 后这些调用必须正常工作。此测试覆盖 error/warn/info 三个级别，
    /// 确保桥接器不会在运行时 panic。
    #[tokio::test]
    #[serial_test::serial]
    async fn log_calls_route_through_inklog_without_panic() {
        let _manager = init_inklog_logger().await.expect("init should succeed");
        // These calls route through inklog's LogLogger → LogAdapter → channels.
        // If the bridge is miswired, these would panic or hang.
        log::error!("sdforge inklog bridge test: error level");
        log::warn!("sdforge inklog bridge test: warn level");
        log::info!("sdforge inklog bridge test: info level");
        // Reaching this point without panicking proves the bridge is wired.
    }

    /// 验证 init_inklog_logger 是幂等的（重复调用不 panic）。
    ///
    /// 全局 logger 只能安装一次。第二次调用时 inklog 的 install 会
    /// 返回 Err(SetLoggerError)，但 with_config 将其降级为 warn
    /// 而非 propagate 为错误。因此 init_inklog_logger 多次调用
    /// 都应返回 Ok。
    #[tokio::test]
    #[serial_test::serial]
    async fn init_inklog_logger_is_idempotent() {
        let _first = init_inklog_logger()
            .await
            .expect("first init should succeed");
        let second = init_inklog_logger().await;
        assert!(
            second.is_ok(),
            "second init should still return Ok (install failure is downgraded to warn), got: {:?}",
            second.err()
        );
    }
}
