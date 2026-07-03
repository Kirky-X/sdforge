// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//!
//! # confers `#[derive(Config)]` 宏完整示例
//!
//! 本示例展示 confers 的 `#[derive(Config)]` 过程宏及其自动生成的全部方法。
//!
//! 与 `app_config.rs`（仅展示 `AppConfig::default()` 和 Builder 模式）不同，
//! 本示例聚焦于 **derive 宏本身** 生成的配置加载能力。
//!
//! ## 涵盖的属性
//!
//! - `#[config(env_prefix = "APP_")]` — 结构体级环境变量前缀
//! - `#[config(default = ...)]` — 字段默认值表达式
//! - `#[config(name = "...")]` — 自定义配置键名（需配合 `#[serde(rename = "...")]`）
//! - `#[config(description = "...")]` — 字段描述（用于 CLI 帮助等）
//! - `#[config(skip)]` — 跳过环境变量加载，不出现在 `env_mapping()` 中
//!
//! ## 生成的 方法
//!
//! | 方法 | 说明 |
//! |------|------|
//! | `load_sync()` | 同步加载：默认值 + 环境变量 |
//! | `load()` | 异步加载（返回 Future） |
//! | `build_config()` | 从环境变量 + 默认值构建 |
//! | `load_file(path)` | 从 TOML/JSON 文件加载 |
//! | `load_file_with_env(path)` | 文件 + 环境变量合并加载 |
//! | `env_mapping()` | 返回 (字段名, 配置键, 环境变量名) 映射 |
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --features http_examples --example config/derive_config
//! ```

// 引入 confers crate 名称到作用域 —— #[derive(Config)] 宏生成的代码使用绝对路径
// `confers::ConfigBuilder` / `confers::ConfigValue` 等，需要 `confers` 名称可见。
// sdforge 通过 `pub use confers;` 重新导出 confers，因此可以从 sdforge 引入。
use sdforge::confers;
use sdforge::confers::Config;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// =============================================================================
// 配置结构定义 — 演示 #[derive(Config)] 的各种字段属性
// =============================================================================

/// 数据库配置结构
///
/// 使用 `#[derive(Config)]` 自动生成 `load_sync`、`load`、`build_config`、
/// `load_file`、`load_file_with_env`、`env_mapping` 方法以及 `Default` 实现。
///
/// # 属性说明
///
/// - `#[config(env_prefix = "APP_")]` — 所有环境变量自动添加 `APP_` 前缀
/// - `#[serde(default)]` — 结构体级 serde 默认值，保证 `load_file` 在文件
///   缺少部分字段时仍能反序列化成功（使用类型默认值填充缺失字段）
/// - `#[config(name = "db_host")]` + `#[serde(rename = "db_host")]` —
///   `name` 设置配置键名（用于默认值/环境变量映射），`serde(rename)` 使
///   serde 反序列化时使用相同的键名，二者必须配对使用
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[serde(default)]
#[config(env_prefix = "APP_")]
pub struct DatabaseConfig {
    /// 数据库主机地址
    ///
    /// 配置键 = `db_host`，环境变量 = `APP_DB_HOST`
    #[serde(rename = "db_host")]
    #[config(default = "localhost".to_string(), name = "db_host", description = "数据库服务器主机地址")]
    pub host: String,

    /// 数据库监听端口
    ///
    /// 环境变量 = `APP_PORT`
    #[config(default = 5432u16, description = "数据库监听端口")]
    pub port: u16,

    /// 数据库名称
    ///
    /// 环境变量 = `APP_DATABASE`
    #[config(default = "myapp".to_string(), description = "数据库名称")]
    pub database: String,

    /// 最大并发连接数
    ///
    /// 环境变量 = `APP_MAX_CONNECTIONS`
    #[config(default = 10u32, description = "最大并发连接数")]
    pub max_connections: u32,

    /// 连接超时（秒）
    ///
    /// 环境变量 = `APP_CONNECTION_TIMEOUT_SECS`
    #[config(default = 30u64, description = "连接超时秒数")]
    pub connection_timeout_secs: u64,

    /// 是否启用 SSL 加密连接
    ///
    /// 环境变量 = `APP_ENABLE_SSL`
    #[config(default = false, description = "是否启用 SSL 加密连接")]
    pub enable_ssl: bool,

    /// 调试模式标志
    ///
    /// `#[config(skip)]` 表示该字段不参与环境变量加载，
    /// 也不会出现在 `env_mapping()` 返回的映射列表中。
    /// `Default` 实现仍会使用 `#[config(default = false)]` 的值。
    #[config(default = false)]
    #[config(skip)]
    pub debug: bool,

    // -----------------------------------------------------------------------
    // 敏感字段示例（需要 confers "encryption" 特性，此处以注释展示）
    // -----------------------------------------------------------------------
    // 注意：`#[config(sensitive = true)]` 要求字段类型为 `confers::SecretString`
    // 或 `confers::SecretBytes`（宏在编译期校验类型，不匹配则报错）。
    // `SecretString` 由 confers 的 `encryption` 特性提供，而 sdforge 当前仅
    // 启用了 confers 的 `watch` 特性，因此以下代码无法编译，仅作参考：
    //
    //   #[config(sensitive = true)]
    //   #[config(default = confers::SecretString::from("change-me"))]
    //   pub password: confers::SecretString,
    //
    // 启用方式：在 sdforge 的 Cargo.toml 中为 confers 依赖添加 "encryption" 特性，
    // 例如：confers = { path = "../confers", optional = true, features = ["watch", "encryption"] }
    // 启用后，敏感字段会触发 `_FILE` 后缀环境变量读取（Docker/K8s secrets 模式），
    // 并通过 PathValidator 防止目录遍历攻击。
}

// =============================================================================
// 辅助函数
// =============================================================================

/// 打印配置信息
fn print_config(label: &str, config: &DatabaseConfig) {
    println!("\n  [{}]", label);
    println!("    host (db_host)     : {}", config.host);
    println!("    port               : {}", config.port);
    println!("    database           : {}", config.database);
    println!("    max_connections    : {}", config.max_connections);
    println!("    connection_timeout : {}s", config.connection_timeout_secs);
    println!("    enable_ssl         : {}", config.enable_ssl);
    println!("    debug (skipped)    : {}", config.debug);
}

/// 创建临时 TOML 配置文件（相对路径）
///
/// confers 的文件源出于安全原因拒绝绝对路径，因此将文件创建在当前工作目录
/// 并返回相对路径（仅文件名）。使用 `std::env::temp_dir()` 会返回绝对路径
/// 导致 `Path validation failed: Absolute paths are not allowed`。
fn create_temp_toml(content: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let file_name = format!("sdforge_derive_config_{}_{}.toml", pid, id);

    // 在当前工作目录创建文件（cargo test 的工作目录为 crate root）
    std::fs::write(&file_name, content).expect("写入临时 TOML 文件失败");

    PathBuf::from(file_name)
}

/// 生成演示用 TOML 配置内容
fn sample_toml_content() -> &'static str {
    r#"
db_host = "db.prod.example.com"
port = 6543
database = "production_db"
max_connections = 50
connection_timeout_secs = 60
enable_ssl = true
"#
}

// =============================================================================
// 主函数 — 演示所有 #[derive(Config)] 生成的方法
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================");
    println!("  confers #[derive(Config)] 宏示例");
    println!("========================================\n");

    // ------------------------------------------------------------------
    // 1. Default 值 — 由 #[config(default = ...)] 生成
    // ------------------------------------------------------------------
    println!("1. DatabaseConfig::default() — 使用 #[config(default = ...)] 值");
    let default_config = DatabaseConfig::default();
    print_config("默认配置", &default_config);

    // ------------------------------------------------------------------
    // 2. load_sync() — 默认值 + 环境变量覆盖
    // ------------------------------------------------------------------
    println!("\n2. load_sync() — 设置环境变量后加载");

    // 设置环境变量演示覆盖（env_prefix = "APP_"）
    // 注意：环境变量始终为字符串类型，confers 以 ConfigValue::String 存储。
    // 因此环境变量覆盖仅对 String 类型字段有效；bool/u16/u32 等非字符串
    // 字段的环境变量覆盖会导致 serde 反序列化类型不匹配错误。
    // host 字段配置键为 "db_host" → 环境变量 = "APP_DB_HOST"
    // database 字段 → 环境变量 = "APP_DATABASE"
    std::env::set_var("APP_DB_HOST", "10.0.0.1");
    std::env::set_var("APP_DATABASE", "env_override_db");
    println!("   已设置: APP_DB_HOST=10.0.0.1, APP_DATABASE=env_override_db");

    let loaded = DatabaseConfig::load_sync()
        .map_err(|e| format!("load_sync 失败: {:?}", e))?;
    print_config("环境变量覆盖后", &loaded);
    println!("   ✓ host 被覆盖为 10.0.0.1，database 被覆盖为 env_override_db");

    // 清理环境变量
    std::env::remove_var("APP_DB_HOST");
    std::env::remove_var("APP_DATABASE");

    // ------------------------------------------------------------------
    // 3. load() — 异步加载（返回 Future）
    // ------------------------------------------------------------------
    println!("\n3. load().await — 异步加载（内部调用 load_sync）");
    let async_loaded = DatabaseConfig::load()
        .await
        .map_err(|e| format!("load 失败: {:?}", e))?;
    print_config("异步加载", &async_loaded);

    // ------------------------------------------------------------------
    // 4. build_config() — 从环境变量 + 默认值构建
    // ------------------------------------------------------------------
    println!("\n4. build_config() — 从环境变量 + 默认值构建");
    let built = DatabaseConfig::build_config()
        .map_err(|e| format!("build_config 失败: {:?}", e))?;
    print_config("build_config", &built);

    // ------------------------------------------------------------------
    // 5. load_file(path) — 从 TOML 文件加载
    // ------------------------------------------------------------------
    println!("\n5. load_file(path) — 从临时 TOML 文件加载");
    let toml_path = create_temp_toml(sample_toml_content());
    println!("   临时文件: {:?}", toml_path);

    let file_config = DatabaseConfig::load_file(&toml_path)
        .map_err(|e| format!("load_file 失败: {:?}", e))?;
    print_config("TOML 文件加载", &file_config);
    println!("   ✓ 所有字段来自 TOML 文件");

    // ------------------------------------------------------------------
    // 6. load_file_with_env(path) — 文件 + 环境变量合并
    // ------------------------------------------------------------------
    println!("\n6. load_file_with_env(path) — 文件 + 环境变量合并");
    println!("   注意: load_file_with_env 使用 .env() 源（无前缀），");
    println!("   环境变量名需与 serde 键名（字段名或 rename）匹配才能覆盖。");
    println!("   同样，环境变量为字符串类型，仅对 String 字段有效。");

    // 设置与字段名匹配的环境变量（无前缀）—— database 是 String 字段
    std::env::set_var("database", "env_merged_db");
    println!("   已设置: database=env_merged_db (匹配 serde 键名)");

    let merged_config = DatabaseConfig::load_file_with_env(&toml_path)
        .map_err(|e| format!("load_file_with_env 失败: {:?}", e))?;
    print_config("文件 + 环境变量", &merged_config);
    println!("   ✓ database 被环境变量覆盖为 env_merged_db，其余来自文件");

    std::env::remove_var("database");
    let _ = std::fs::remove_file(&toml_path);

    // ------------------------------------------------------------------
    // 7. env_mapping() — 字段名 → 配置键 → 环境变量名 映射
    // ------------------------------------------------------------------
    println!("\n7. env_mapping() — (字段名, 配置键, 环境变量名) 映射");
    let mapping = DatabaseConfig::env_mapping();
    println!("   共 {} 条映射（debug 字段被 skip，不包含在内）:", mapping.len());
    for (field, key, env) in &mapping {
        println!("    {:<25} → config_key: {:<25} → env: {}", field, key, env);
    }

    println!("\n========================================");
    println!("  示例运行完成!");
    println!("========================================");
    Ok(())
}

// =============================================================================
// 单元测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // 测试 1: Default 值正确性
    // ------------------------------------------------------------------

    #[test]
    fn test_default_values() {
        let config = DatabaseConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.database, "myapp");
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.connection_timeout_secs, 30);
        assert!(!config.enable_ssl);
        assert!(!config.debug);
    }

    // ------------------------------------------------------------------
    // 测试 2: 环境变量覆盖
    // ------------------------------------------------------------------

    #[test]
    fn test_env_var_override() {
        // 环境变量始终为字符串类型（confers 以 ConfigValue::String 存储），
        // 因此仅对 String 类型字段测试环境变量覆盖。
        // 对 bool/u16/u32 等非字符串字段，环境变量覆盖会导致 serde
        // 反序列化类型不匹配错误（这是 confers 的已知行为）。
        std::env::set_var("APP_DB_HOST", "10.0.0.1");
        std::env::set_var("APP_DATABASE", "env_test_db");

        let config = DatabaseConfig::load_sync().expect("load_sync 应成功");

        // 清理环境变量
        std::env::remove_var("APP_DB_HOST");
        std::env::remove_var("APP_DATABASE");

        // 验证 String 字段的环境变量覆盖
        assert_eq!(config.host, "10.0.0.1", "APP_DB_HOST 应覆盖 host");
        assert_eq!(config.database, "env_test_db", "APP_DATABASE 应覆盖 database");

        // 未设置环境变量的字段仍使用默认值
        assert_eq!(config.port, 5432);
        assert_eq!(config.max_connections, 10);
        assert!(!config.enable_ssl);
    }

    // ------------------------------------------------------------------
    // 测试 3: env_mapping() 返回预期映射
    // ------------------------------------------------------------------

    #[test]
    fn test_env_mapping() {
        let mapping = DatabaseConfig::env_mapping();

        // debug 字段被 skip，不应出现在映射中
        let has_debug = mapping.iter().any(|(f, _, _)| f == "debug");
        assert!(!has_debug, "debug 字段被 skip，不应出现在 env_mapping 中");

        // 应有 6 条映射（7 个字段减去 1 个 skip）
        assert_eq!(mapping.len(), 6, "应有 6 条映射（排除 debug）");

        // 验证 host 字段映射（name = "db_host"）
        let host_mapping = mapping.iter().find(|(f, _, _)| f == "host");
        assert!(host_mapping.is_some(), "应包含 host 字段映射");
        let (_, key, env) = host_mapping.unwrap();
        assert_eq!(*key, "db_host", "host 的配置键应为 db_host");
        assert_eq!(*env, "APP_DB_HOST", "host 的环境变量应为 APP_DB_HOST");

        // 验证 port 字段映射（无 name 覆盖）
        let port_mapping = mapping.iter().find(|(f, _, _)| f == "port");
        assert!(port_mapping.is_some(), "应包含 port 字段映射");
        let (_, key, env) = port_mapping.unwrap();
        assert_eq!(*key, "port", "port 的配置键应为 port");
        assert_eq!(*env, "APP_PORT", "port 的环境变量应为 APP_PORT");

        // 验证 enable_ssl 字段映射
        let ssl_mapping = mapping.iter().find(|(f, _, _)| f == "enable_ssl");
        assert!(ssl_mapping.is_some(), "应包含 enable_ssl 字段映射");
        let (_, _, env) = ssl_mapping.unwrap();
        assert_eq!(*env, "APP_ENABLE_SSL", "enable_ssl 的环境变量应为 APP_ENABLE_SSL");
    }

    // ------------------------------------------------------------------
    // 测试 4: load_file() 正确解析 TOML
    // ------------------------------------------------------------------

    #[test]
    fn test_load_file_toml() {
        let toml_content = r#"
db_host = "db.test.example.com"
port = 9999
database = "test_db"
max_connections = 5
connection_timeout_secs = 15
enable_ssl = true
"#;
        let path = create_temp_toml(toml_content);

        let config = DatabaseConfig::load_file(&path)
            .expect("load_file 应成功解析 TOML");

        // 清理临时文件
        let _ = std::fs::remove_file(&path);

        // 验证 TOML 文件中的值被正确解析
        assert_eq!(config.host, "db.test.example.com", "host 应来自 TOML");
        assert_eq!(config.port, 9999, "port 应来自 TOML");
        assert_eq!(config.database, "test_db", "database 应来自 TOML");
        assert_eq!(config.max_connections, 5, "max_connections 应来自 TOML");
        assert_eq!(config.connection_timeout_secs, 15, "connection_timeout_secs 应来自 TOML");
        assert!(config.enable_ssl, "enable_ssl 应来自 TOML");

        // debug 字段被 skip，文件中未指定，使用 serde 默认值 (false)
        assert!(!config.debug, "debug 应为 serde 默认值 false");
    }

    // ------------------------------------------------------------------
    // 测试 5: skip 字段不参与环境变量加载
    // ------------------------------------------------------------------

    #[test]
    fn test_skip_field_not_loaded_from_env() {
        // 设置 APP_DEBUG 环境变量
        std::env::set_var("APP_DEBUG", "true");

        let config = DatabaseConfig::load_sync().expect("load_sync 应成功");

        std::env::remove_var("APP_DEBUG");

        // debug 字段被 skip，环境变量 APP_DEBUG 不应影响它
        assert!(!config.debug, "debug 被 skip，APP_DEBUG 不应覆盖它");
    }

    // ------------------------------------------------------------------
    // 测试 6: load() 异步加载与 load_sync() 结果一致
    // ------------------------------------------------------------------

    #[test]
    fn test_load_async_matches_sync() {
        // load() 内部调用 load_sync()，两者代码路径相同。
        // 仅比较不受环境变量影响的 skip 字段（debug），避免并行测试
        // 设置环境变量导致的竞态条件。
        let sync_config = DatabaseConfig::load_sync().expect("load_sync 应成功");

        let rt = tokio::runtime::Runtime::new().expect("创建 runtime 失败");
        let async_config = rt.block_on(async {
            DatabaseConfig::load().await.expect("load 应成功")
        });

        // debug 是 skip 字段，不受环境变量影响，两者必须一致
        assert_eq!(sync_config.debug, async_config.debug);
        assert!(!sync_config.debug, "debug 应为 false（serde 默认值）");
        assert!(!async_config.debug, "debug 应为 false（serde 默认值）");
    }
}
