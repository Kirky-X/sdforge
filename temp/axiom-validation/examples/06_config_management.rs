//! 06_config_management - 配置管理示例
//!
//! 这个示例演示如何使用 Axiom 框架的配置管理功能。

use axiom::config::{ConfigLoader, init_logging};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================");
    println!("Axiom 配置管理示例");
    println!("========================================");
    println!();

    // 从文件加载配置
    let config_path = "configs/full.toml";
    println!("📄 从文件加载配置: {}", config_path);

    let loader = ConfigLoader::new(config_path, "AXIOM");
    let config = loader.load()?;

    println!("✅ 配置加载成功");
    println!();
    println!("📋 配置信息:");
    println!("  服务器: {}:{}", config.server.host, config.server.port);
    println!("  API: {} v{}", config.api.name, config.api.version);
    println!();

    // 初始化日志
    if let Some(logging_config) = &config.logging {
        init_logging(logging_config);
        println!("✅ 日志系统已初始化");
    }

    println!();
    println!("按 Ctrl+C 退出");
    println!("========================================");

    tokio::signal::ctrl_c().await?;
    println!("\n👋 配置管理示例已停止");

    Ok(())
}