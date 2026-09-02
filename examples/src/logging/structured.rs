// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 结构化日志示例
//!
//! 本示例展示 SDForge 结构化日志的使用方式：
//!
//! 1. 创建 `StructuredLogger` 实例（独立使用）
//! 2. 配置 `LoggerConfig`（日志级别、格式、颜色）
//! 3. 构建带上下文字段的 `LogEntry`
//! 4. 初始化全局日志器并通过便捷宏记录日志
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --features logging_examples --example logging/structured
//! ```

use sdforge::logging::{
    get_global_logger, init_global_logger, LogEntry, LogFormat, LogLevel, LoggerConfig,
    StructuredLogger,
};
use serde_json::json;

// =============================================================================
// 日志器配置
// =============================================================================

/// 默认日志器配置 — Info 级别，JSON 格式，启用颜色。
pub fn default_config() -> LoggerConfig {
    LoggerConfig::default()
}

/// 开发环境配置 — Debug 级别，文本格式，启用颜色。
pub fn dev_config() -> LoggerConfig {
    LoggerConfig {
        min_level: LogLevel::Debug,
        format: LogFormat::Text,
        colored: true,
        ..Default::default()
    }
}

/// 生产环境配置 — Warn 级别，JSON 格式，禁用颜色。
pub fn production_config() -> LoggerConfig {
    LoggerConfig {
        min_level: LogLevel::Warn,
        format: LogFormat::Json,
        colored: false,
        max_file_size: 50 * 1024 * 1024, // 50 MB
        max_files: 10,
    }
}

// =============================================================================
// 日志条目构建
// =============================================================================

/// 演示构建带上下文字段的日志条目。
///
/// `LogEntry` 支持链式调用添加字段，字段以 `BTreeMap` 存储（有序输出）。
pub fn build_log_entry() -> LogEntry {
    LogEntry::new(
        LogLevel::Info,
        "user_service",
        "User logged in successfully",
    )
    .with_field("user_id", "12345")
    .with_field("username", "alice")
    .with_field("ip_address", "192.168.1.100")
    .with_field("method", "password")
}

/// 演示多字段批量添加。
pub fn build_log_entry_with_fields() -> LogEntry {
    let fields = vec![
        ("request_id".to_string(), json!("req-abc-123")),
        ("duration_ms".to_string(), json!(42)),
        ("status_code".to_string(), json!(200)),
        ("endpoint".to_string(), json!("/api/v1/users")),
    ];

    LogEntry::new(LogLevel::Info, "http", "Request completed").with_fields(fields)
}

// =============================================================================
// 独立日志器使用（不依赖全局状态）
// =============================================================================

/// 创建一个独立的 `StructuredLogger` 并记录各级别日志。
///
/// 独立日志器适用于组件级日志记录，不需要全局状态。
pub fn create_standalone_logger() -> StructuredLogger {
    let config = LoggerConfig {
        min_level: LogLevel::Trace,
        format: LogFormat::Json,
        colored: false,
        ..Default::default()
    };

    let logger = StructuredLogger::new(config);

    // 各级别日志方法
    logger.trace(
        "trace_module",
        "Detailed trace information",
        vec![("step".to_string(), json!(1))],
    );

    logger.debug(
        "debug_module",
        "Debug information for development",
        vec![("var".to_string(), json!("value"))],
    );

    logger.info(
        "app",
        "Application initialized",
        vec![
            ("version".to_string(), json!("0.1.0")),
            ("env".to_string(), json!("production")),
        ],
    );

    logger.warn(
        "cache",
        "Cache hit rate below threshold",
        vec![
            ("hit_rate".to_string(), json!(0.65)),
            ("threshold".to_string(), json!(0.8)),
        ],
    );

    logger.error(
        "database",
        "Connection pool exhausted",
        vec![
            ("active".to_string(), json!(100)),
            ("max".to_string(), json!(100)),
        ],
    );

    logger
}

// =============================================================================
// 全局日志器
// =============================================================================

/// 初始化全局日志器并验证可用性。
///
/// `init_global_logger` 只能调用一次（使用 `OnceCell` 保证）。
/// 重复调用返回 `LoggerError::AlreadyInitialized`。
pub fn setup_global_logger() -> Result<(), Box<dyn std::error::Error>> {
    let config = production_config();
    init_global_logger(config).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// 通过全局日志器记录日志（如果已初始化）。
pub fn log_via_global(message: &str) {
    if let Some(logger) = get_global_logger() {
        logger.info("global_demo", message, vec![]);
    } else {
        eprintln!("[fallback] global logger not initialized: {}", message);
    }
}

// =============================================================================
// Main Entry Point
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 SDForge Structured Logging Example");
    println!("======================================\n");

    // 配置展示
    println!("⚙️  日志器配置:");
    println!(
        "  default:    level={}, format={:?}, colored={}",
        default_config().min_level,
        default_config().format,
        default_config().colored
    );
    println!(
        "  dev:        level={}, format={:?}, colored={}",
        dev_config().min_level,
        dev_config().format,
        dev_config().colored
    );
    println!(
        "  production: level={}, format={:?}, colored={}",
        production_config().min_level,
        production_config().format,
        production_config().colored
    );
    println!();

    // 日志条目展示
    println!("📋 LogEntry 构建:");
    let entry = build_log_entry();
    println!("  level: {}", entry.level);
    println!("  target: {}", entry.target);
    println!("  message: {}", entry.message);
    println!("  fields ({} 个):", entry.fields.len());
    for (key, value) in &entry.fields {
        println!("    {} = {}", key, value);
    }

    println!();
    let multi_entry = build_log_entry_with_fields();
    println!("  多字段条目: {} 个字段", multi_entry.fields.len());
    let json = serde_json::to_string(&multi_entry)?;
    println!("  JSON: {}", json);
    println!();

    // 独立日志器
    println!("🔌 独立日志器 (StructuredLogger::new):");
    println!("  (日志输出到 stdout)");
    let _logger = create_standalone_logger();
    println!();

    // 全局日志器
    println!("🌍 全局日志器:");
    match setup_global_logger() {
        Ok(()) => {
            println!("  ✓ 全局日志器已初始化");
            log_via_global("Global logger is working");
        }
        Err(e) => {
            println!("  ⚠️  全局日志器已初始化（重复调用返回错误）: {}", e);
        }
    }

    println!();
    println!("💡 便捷宏 (可在任意位置使用):");
    println!("  sdforge::log_info!(\"module\", \"message\", \"key\" => \"value\");");
    println!("  sdforge::log_error!(\"module\", \"error\", \"code\" => 500);");
    println!("  sdforge::log_debug!(\"module\", \"debug info\");");

    println!();
    println!("✓ 日志示例完成");

    // 给异步日志器一点时间刷新输出
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_info_json() {
        let config = default_config();
        assert_eq!(config.min_level, LogLevel::Info);
        assert_eq!(config.format, LogFormat::Json);
        assert!(config.colored);
    }

    #[test]
    fn test_dev_config_is_debug_text() {
        let config = dev_config();
        assert_eq!(config.min_level, LogLevel::Debug);
        assert_eq!(config.format, LogFormat::Text);
        assert!(config.colored);
    }

    #[test]
    fn test_production_config_is_warn_json_nocolor() {
        let config = production_config();
        assert_eq!(config.min_level, LogLevel::Warn);
        assert_eq!(config.format, LogFormat::Json);
        assert!(!config.colored);
        assert_eq!(config.max_files, 10);
    }

    #[test]
    fn test_build_log_entry_has_fields() {
        let entry = build_log_entry();
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.target, "user_service");
        assert!(entry.message.contains("User logged in"));
        assert_eq!(entry.fields.len(), 4, "should have 4 context fields");
        assert!(entry.fields.contains_key("user_id"));
        assert!(entry.fields.contains_key("username"));
    }

    #[test]
    fn test_build_log_entry_with_fields_batch() {
        let entry = build_log_entry_with_fields();
        assert_eq!(entry.fields.len(), 4, "should have 4 batch fields");
        assert!(entry.fields.contains_key("request_id"));
        assert!(entry.fields.contains_key("duration_ms"));
    }

    #[test]
    fn test_log_entry_serializes_to_json() {
        let entry = build_log_entry();
        let json = serde_json::to_string(&entry).expect("serialization should succeed");
        assert!(
            json.contains("\"level\":\"info\""),
            "JSON should contain level"
        );
        assert!(
            json.contains("\"target\":\"user_service\""),
            "JSON should contain target"
        );
        assert!(
            json.contains("\"user_id\""),
            "JSON should contain user_id field"
        );
    }

    #[test]
    fn test_log_level_ordering() {
        // LogLevel derives Ord — verify ordering is correct
        assert!(LogLevel::Error > LogLevel::Warn);
        assert!(LogLevel::Warn > LogLevel::Info);
        assert!(LogLevel::Info > LogLevel::Debug);
        assert!(LogLevel::Debug > LogLevel::Trace);
    }

    #[tokio::test]
    async fn test_create_standalone_logger_does_not_panic() {
        // StructuredLogger::new spawns a tokio task, so this must run in a runtime.
        let logger = create_standalone_logger();
        logger.flush().await;
        logger.shutdown().await;
    }
}