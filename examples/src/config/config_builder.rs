// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//!
//! # confers `ConfigBuilder<T>` 完整示例
//!
//! 本示例展示 confers 的 `ConfigBuilder<T>` 流式 API 的全部用法。
//!
//! 与 `derive_config.rs`（演示 `#[derive(Config)]` 宏生成的便捷方法）不同，
//! 本示例聚焦于 **手动构建** 配置加载流程，适用于需要精细控制源优先级、
//! 合并策略、回退和弹性加载的场景。
//!
//! ## 涵盖的 API
//!
//! | 方法 | 说明 |
//! |------|------|
//! | `ConfigBuilder::<T>::new()` | 创建新的 builder |
//! | `.file(path)` | 添加文件源（TOML/JSON/YAML） |
//! | `.file_optional(path)` | 添加可选文件源（不存在不报错） |
//! | `.env()` | 添加环境变量源 |
//! | `.env_prefix(prefix)` | 添加带前缀的环境变量源 |
//! | `.default(key, value)` | 添加单个默认值 |
//! | `.defaults(map)` | 批量添加默认值 |
//! | `.memory(map)` | 添加内存值（最高优先级） |
//! | `.build()` | 构建配置（失败返回 Error） |
//! | `.build_with_fallback(fallback)` | 失败时使用回退配置 |
//! | `.build_resilient()` | 收集警告而非失败 |
//! | `.fail_fast(false)` | 非快速失败模式 |
//! | `.allow_absolute_paths()` | 允许绝对路径（测试用） |
//! | `confers::config::<T>()` | `ConfigBuilder::new()` 便捷函数 |
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --features http_examples --example config/config_builder
//! ```

use sdforge::confers;
use sdforge::confers::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// 引入 confers crate 名称（宏生成的代码使用 `confers::` 绝对路径）
use confers::{ConfigBuilder, ConfigValue};

// =============================================================================
// 配置结构定义
// =============================================================================

/// 服务器配置结构
///
/// 使用 `#[derive(Config)]` 提供 `Default` 实现和便捷方法，
/// 但本示例主要演示手动使用 `ConfigBuilder` 来加载它。
///
/// # 字段类型与 env var 覆盖
///
/// confers 将所有 env var 存为 `ConfigValue::String`，serde 反序列化时
/// 需要类型匹配。**`String` 类型字段可被 env var 覆盖**；`u16`/`usize`/`bool`
/// 等非字符串字段无法被 env var 直接覆盖（serde 会因类型不匹配报错）。
/// 因此 `host` 与 `level` 可被 env var 覆盖，`port` 与 `workers` 不能。
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "SRV_")]
pub struct ServerConfig {
    /// 监听地址（可被 `SRV_HOST` 覆盖）
    #[config(default = "127.0.0.1".to_string())]
    #[serde(default = "default_host")]
    pub host: String,

    /// 监听端口（不可被 env var 覆盖，因 confers 以 String 存储 env 值）
    #[config(default = 8080u16)]
    #[serde(default = "default_port")]
    pub port: u16,

    /// 工作线程数（不可被 env var 覆盖，因 confers 以 String 存储 env 值）
    #[config(default = 4usize)]
    #[serde(default = "default_workers")]
    pub workers: usize,

    /// 日志级别（可被 `SRV_LEVEL` 覆盖）
    ///
    /// **命名约束**：confers 将 env var 名通过 lowercase + `_`→`.` 转换为配置键。
    /// 多词字段（如 `log_level`）对应的 `SRV_LOG_LEVEL` 会被转换为嵌套键
    /// `log.level`，无法匹配平铺字段 `log_level`。因此本示例使用单词字段
    /// `level` 以演示 env var 覆盖能力。
    #[config(default = "info".to_string())]
    #[serde(default = "default_level")]
    pub level: String,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_workers() -> usize {
    4
}

fn default_level() -> String {
    "info".to_string()
}

// =============================================================================
// 辅助函数
// =============================================================================

/// 创建临时 TOML 配置文件用于演示
///
/// 返回文件路径。文件创建在当前目录下（相对路径）以避免 confers
/// 的路径遍历防护拒绝绝对路径。
fn write_temp_toml(filename: &str, content: &str) -> PathBuf {
    let path = PathBuf::from(filename);
    std::fs::write(&path, content).expect("failed to write temp config");
    path
}

/// 清理临时文件
fn cleanup_temp(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
}

/// 打印配置信息
fn print_config(label: &str, config: &ServerConfig) {
    println!(
        "  [{}] host={}, port={}, workers={}, level={}",
        label, config.host, config.port, config.workers, config.level
    );
}

// =============================================================================
// 场景演示函数
// =============================================================================

/// 场景 1: 基础文件加载
///
/// `ConfigBuilder::<T>::new().file("config.toml").build()`
pub fn demo_basic_file() -> ServerConfig {
    let path = write_temp_toml(
        "config_builder_basic.toml",
        r#"host = "0.0.0.0"
port = 3000
workers = 8
level = "debug"
"#,
    );

    let config = ConfigBuilder::<ServerConfig>::new()
        .file(&path)
        .build()
        .expect("basic file build should succeed");

    cleanup_temp(&path);
    config
}

/// 场景 2: 多文件合并（后加载优先级高）
///
/// `.file("base.toml").file("override.toml").build()`
pub fn demo_multi_file_merge() -> ServerConfig {
    let base = write_temp_toml(
        "config_builder_base.toml",
        r#"host = "10.0.0.1"
port = 8000
workers = 2
level = "info"
"#,
    );
    let override_file = write_temp_toml(
        "config_builder_override.toml",
        r#"port = 9000
workers = 16
level = "warn"
"#,
    );

    let config = ConfigBuilder::<ServerConfig>::new()
        .file(&base)
        .file(&override_file)
        .build()
        .expect("multi-file merge should succeed");

    cleanup_temp(&base);
    cleanup_temp(&override_file);
    config
}

/// 场景 3: 文件 + 环境变量（带前缀）
///
/// `.file("config.toml").env_prefix("SRV_").build()`
///
/// 环境变量 `SRV_HOST` 覆盖文件中的 `host` 字段。
/// 注意: `.env()` (无前缀) 会读取所有环境变量，键名经 lowercase + 分隔符
/// 替换后可能不匹配字段名。生产环境推荐使用 `.env_prefix(...)` 限定范围。
pub fn demo_file_with_env() -> ServerConfig {
    // 设置环境变量（覆盖文件中的 host）
    std::env::set_var("SRV_HOST", "env.override.local");

    let path = write_temp_toml(
        "config_builder_env.toml",
        r#"host = "file.default"
port = 4000
workers = 4
level = "info"
"#,
    );

    let config = ConfigBuilder::<ServerConfig>::new()
        .file(&path)
        .env_prefix("SRV_")
        .build()
        .expect("file + env build should succeed");

    cleanup_temp(&path);
    std::env::remove_var("SRV_HOST");
    config
}

/// 场景 4: 带前缀的环境变量（覆盖 String 字段）
///
/// `.file("config.toml").env_prefix("SRV_").build()`
///
/// 环境变量 `SRV_LEVEL` 覆盖文件中的 `level` 字段。
///
/// **重要**：confers 将 env var 存为 `ConfigValue::String`，serde 反序列化
/// 时需类型匹配。只有 `String` 类型字段可被 env var 覆盖；`u16`/`usize`/`bool`
/// 等非字符串字段会被 serde 拒绝（"invalid type: string \"32\", expected usize"）。
/// 因此这里用 `level` (String) 演示，而不是 `workers` (usize)。
pub fn demo_env_prefix() -> ServerConfig {
    std::env::set_var("SRV_LEVEL", "debug");

    let path = write_temp_toml(
        "config_builder_prefix.toml",
        r#"host = "file.host"
port = 5000
workers = 2
level = "info"
"#,
    );

    let config = ConfigBuilder::<ServerConfig>::new()
        .file(&path)
        .env_prefix("SRV_")
        .build()
        .expect("env_prefix build should succeed");

    cleanup_temp(&path);
    std::env::remove_var("SRV_LEVEL");
    config
}

/// 场景 5: 可选文件（不存在不报错）
///
/// `.file_optional("local.toml").file("config.toml").build()`
pub fn demo_optional_file() -> ServerConfig {
    let path = write_temp_toml(
        "config_builder_required.toml",
        r#"host = "required.host"
port = 6000
workers = 3
level = "info"
"#,
    );

    // local.toml 不存在，但 file_optional 不会报错
    let config = ConfigBuilder::<ServerConfig>::new()
        .file_optional("config_builder_local_nonexistent.toml")
        .file(&path)
        .build()
        .expect("optional file build should succeed");

    cleanup_temp(&path);
    config
}

/// 场景 6: 默认值
///
/// `.default("key", value).build()`
pub fn demo_defaults() -> ServerConfig {
    let config = ConfigBuilder::<ServerConfig>::new()
        .default("host", ConfigValue::string("default.host"))
        .default("port", ConfigValue::uint(7777))
        .default("workers", ConfigValue::uint(6))
        .default("level", ConfigValue::string("trace"))
        .build()
        .expect("defaults build should succeed");

    config
}

/// 场景 7: 内存值（最高优先级）
///
/// `.memory(map).build()` — 内存值覆盖文件和默认值
pub fn demo_memory_values() -> ServerConfig {
    let path = write_temp_toml(
        "config_builder_mem_file.toml",
        r#"host = "file.host"
port = 1111
workers = 1
level = "info"
"#,
    );

    let mut mem = HashMap::new();
    mem.insert("host".to_string(), ConfigValue::string("memory.override"));
    mem.insert("workers".to_string(), ConfigValue::uint(99));
    mem.insert("level".to_string(), ConfigValue::string("error"));

    let config = ConfigBuilder::<ServerConfig>::new()
        .file(&path)
        .memory(mem)
        .build()
        .expect("memory values build should succeed");

    cleanup_temp(&path);
    config
}

/// 场景 8: 回退配置
///
/// `.build_with_fallback(fallback)` — 失败时返回回退配置
pub fn demo_build_with_fallback() -> confers::BuildResult<ServerConfig> {
    let fallback = ServerConfig::default();

    // 引用一个不存在的文件路径，触发 fallback
    let result = ConfigBuilder::<ServerConfig>::new()
        .file("/nonexistent/path/config.toml")
        .allow_absolute_paths()
        .build_with_fallback(fallback);

    result
}

/// 场景 9: 弹性构建
///
/// `.build_resilient()` — 收集警告而非失败
pub fn demo_build_resilient() -> confers::BuildResult<ServerConfig> {
    let result = ConfigBuilder::<ServerConfig>::new()
        .file_optional("config_builder_nonexistent_optional.toml")
        .default("host", ConfigValue::string("resilient.host"))
        .default("port", ConfigValue::uint(3333))
        .default("workers", ConfigValue::uint(7))
        .default("level", ConfigValue::string("warn"))
        .build_resilient()
        .expect("resilient build should not fail on optional missing file");

    result
}

/// 场景 10: 便捷函数
///
/// `confers::config::<T>()` 等价于 `ConfigBuilder::<T>::new()`
pub fn demo_convenience_function() -> ServerConfig {
    let path = write_temp_toml(
        "config_builder_convenience.toml",
        r#"host = "convenience.host"
port = 2222
workers = 5
level = "info"
"#,
    );

    let config = confers::config::<ServerConfig>()
        .file(&path)
        .build()
        .expect("convenience function build should succeed");

    cleanup_temp(&path);
    config
}

// =============================================================================
// Main Entry Point
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚙️  confers ConfigBuilder<T> 示例");
    println!("==================================\n");

    // 场景 1: 基础文件加载
    println!("📄 场景 1: 基础文件加载 (.file(path).build())");
    let config = demo_basic_file();
    print_config("文件", &config);
    println!();

    // 场景 2: 多文件合并
    println!("📄📄 场景 2: 多文件合并 (后加载优先级高)");
    let config = demo_multi_file_merge();
    print_config("合并后", &config);
    println!("  注: port 和 workers 来自 override 文件\n");

    // 场景 3: 文件 + 环境变量
    println!("📄🌍 场景 3: 文件 + 环境变量");
    let config = demo_file_with_env();
    print_config("文件+env", &config);
    println!("  注: host 来自环境变量 SRV_HOST\n");

    // 场景 4: 带前缀的环境变量
    println!("🌍 场景 4: 带前缀的环境变量 (.env_prefix(\"SRV_\"))");
    let config = demo_env_prefix();
    print_config("env_prefix", &config);
    println!();

    // 场景 5: 可选文件
    println!("📄❓ 场景 5: 可选文件 (.file_optional)");
    let config = demo_optional_file();
    print_config("可选文件", &config);
    println!("  注: local.toml 不存在但未报错\n");

    // 场景 6: 默认值
    println!("⚙️  场景 6: 默认值 (.default(key, value))");
    let config = demo_defaults();
    print_config("默认值", &config);
    println!();

    // 场景 7: 内存值
    println!("💾 场景 7: 内存值 (.memory(map) — 最高优先级)");
    let config = demo_memory_values();
    print_config("内存覆盖", &config);
    println!("  注: host 和 workers 被内存值覆盖\n");

    // 场景 8: 回退配置
    println!("🔄 场景 8: 回退配置 (.build_with_fallback)");
    let result = demo_build_with_fallback();
    print_config("回退", &result.config);
    println!("  degraded={}, 警告数={}", result.degraded, result.warnings.len());
    if let Some(reason) = &result.degraded_reason {
        println!("  原因: {}", reason);
    }
    println!();

    // 场景 9: 弹性构建
    println!("🛡️  场景 9: 弹性构建 (.build_resilient)");
    let result = demo_build_resilient();
    print_config("弹性", &result.config);
    println!("  degraded={}, 警告数={}", result.degraded, result.warnings.len());
    println!();

    // 场景 10: 便捷函数
    println!("⚡ 场景 10: 便捷函数 (confers::config::<T>())");
    let config = demo_convenience_function();
    print_config("便捷", &config);
    println!();

    println!("✓ ConfigBuilder 示例完成");

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 串行化所有涉及环境变量的测试。
    ///
    /// `env_prefix("SRV_")` 会读取所有 `SRV_*` 环境变量；并行运行时一个测试
    /// 设置的 `SRV_HOST`/`SRV_LEVEL` 会被另一个测试的 env 源意外读取，
    /// 导致竞态条件。此 Mutex 强制所有 env-touching 测试串行执行。
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// 获取 env 测试串行锁。在测试开始处调用 `let _guard = env_lock();`
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        // 若锁被 poisoned（其他测试 panic），仍获取以继续测试
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn test_demo_basic_file() {
        let config = demo_basic_file();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.workers, 8);
        assert_eq!(config.level, "debug");
    }

    #[test]
    fn test_demo_multi_file_merge() {
        let config = demo_multi_file_merge();
        // base: host=10.0.0.1, port=8000, workers=2, level=info
        // override: port=9000, workers=16, level=warn
        // 合并后: host 来自 base, port/workers/level 来自 override
        assert_eq!(config.host, "10.0.0.1");
        assert_eq!(config.port, 9000);
        assert_eq!(config.workers, 16);
        assert_eq!(config.level, "warn");
    }

    #[test]
    fn test_demo_file_with_env() {
        let _guard = env_lock();
        let config = demo_file_with_env();
        // SRV_HOST 环境变量覆盖文件中的 host
        assert_eq!(config.host, "env.override.local");
        // port 和 workers 来自文件
        assert_eq!(config.port, 4000);
        assert_eq!(config.workers, 4);
        // level 未被 env 覆盖（未设置 SRV_LEVEL），来自文件
        assert_eq!(config.level, "info");
    }

    #[test]
    fn test_demo_env_prefix() {
        let _guard = env_lock();
        let config = demo_env_prefix();
        // SRV_LEVEL 覆盖文件中的 level
        assert_eq!(config.level, "debug");
        // host 和 port 来自文件（未被 env 覆盖）
        assert_eq!(config.host, "file.host");
        assert_eq!(config.port, 5000);
    }

    #[test]
    fn test_demo_optional_file() {
        let config = demo_optional_file();
        // 可选文件不存在，使用 required 文件的值
        assert_eq!(config.host, "required.host");
        assert_eq!(config.port, 6000);
        assert_eq!(config.workers, 3);
        assert_eq!(config.level, "info");
    }

    #[test]
    fn test_demo_defaults() {
        let config = demo_defaults();
        assert_eq!(config.host, "default.host");
        assert_eq!(config.port, 7777);
        assert_eq!(config.workers, 6);
        assert_eq!(config.level, "trace");
    }

    #[test]
    fn test_demo_memory_values() {
        let config = demo_memory_values();
        // 内存值覆盖文件值
        assert_eq!(config.host, "memory.override");
        assert_eq!(config.workers, 99);
        assert_eq!(config.level, "error");
        // port 未被内存覆盖，来自文件
        assert_eq!(config.port, 1111);
    }

    #[test]
    fn test_demo_build_with_fallback() {
        let result = demo_build_with_fallback();
        assert!(result.degraded, "should be degraded when file doesn't exist");
        // 应回退到默认值
        assert_eq!(result.config.port, 8080);
        assert_eq!(result.config.level, "info");
        assert!(!result.warnings.is_empty(), "should have warnings");
    }

    #[test]
    fn test_demo_build_resilient() {
        let result = demo_build_resilient();
        // 可选文件不存在，但默认值提供配置
        assert_eq!(result.config.host, "resilient.host");
        assert_eq!(result.config.port, 3333);
        assert_eq!(result.config.workers, 7);
        assert_eq!(result.config.level, "warn");
    }

    #[test]
    fn test_demo_convenience_function() {
        let config = demo_convenience_function();
        assert_eq!(config.host, "convenience.host");
        assert_eq!(config.port, 2222);
        assert_eq!(config.workers, 5);
        assert_eq!(config.level, "info");
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.workers, 4);
        assert_eq!(config.level, "info");
    }
}
