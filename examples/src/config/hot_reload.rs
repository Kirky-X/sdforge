// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//!
//! # 配置热重载示例
//!
//! 本示例展示 SDForge 配置热重载 API 的使用方式：
//!
//! 1. **创建配置监听器** — `create_config_watcher(path)` 监听文件变更
//! 2. **处理重载事件** — 匹配 `ConfigEvent::Reloaded` 与 `ConfigEvent::Error`
//! 3. **使用 ConfigManager** — 创建、读取、更新共享配置
//! 4. **获取当前配置** — `watcher.get()` 从文件加载当前配置
//! 5. **后台事件循环** — 派生 tokio 任务处理事件
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --features http_examples --example config/hot_reload
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use sdforge::config::hot_reload::{
    create_config_watcher, ConfigEvent, ConfigManager, ConfigWatcherImpl,
};
use sdforge::config::AppConfig;

/// 示例使用的 TOML 配置内容（初始版本）。
const INITIAL_CONFIG_TOML: &str = r#"
[server]
host = "127.0.0.1"
port = 8080
request_timeout_secs = 30

[authentication]
type = "none"
"#;

/// 示例使用的 TOML 配置内容（重载版本）。
const RELOADED_CONFIG_TOML: &str = r#"
[server]
host = "0.0.0.0"
port = 9090
request_timeout_secs = 45

[authentication]
type = "none"
"#;

/// 在系统临时目录下创建一个唯一的 TOML 配置文件，返回其路径。
fn write_temp_config(content: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sdforge_hot_reload_example_{}.toml",
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, content).expect("写入临时配置文件失败");
    path
}

// =============================================================================
// 演示 1: ConfigManager — 共享配置状态管理
// =============================================================================

/// 演示 `ConfigManager` 的创建、读取与更新。
///
/// `ConfigManager` 内部使用 `RwLock` 保护共享配置，适合在多任务环境下
/// 作为配置状态的单一来源。
async fn demo_config_manager() {
    println!("📦 ConfigManager 演示");

    let initial = AppConfig::default();
    let manager = ConfigManager::new(initial.clone());

    let current = manager.get().await;
    println!("  初始 host: {}, port: {}", current.server.host, current.server.port);

    let mut updated = AppConfig::default();
    updated.server.host = "0.0.0.0".to_string();
    updated.server.port = 9090;
    manager.update(updated).await;

    let after = manager.get().await;
    println!(
        "  更新后 host: {}, port: {}",
        after.server.host, after.server.port
    );
    println!();
}

// =============================================================================
// 演示 2: 后台事件循环模式
// =============================================================================

/// 打印 `ConfigWatcherImpl` 监听器信息。
fn print_watcher_info(watcher: &ConfigWatcherImpl) {
    println!("  监听路径: {}", watcher.path().display());
}

/// 派生后台 tokio 任务接收 `ConfigEvent` 并更新共享的 `ConfigManager`。
///
/// 返回用 `Arc` 包裹的 `ConfigManager`（供调用方读取最新配置）和任务句柄。
async fn spawn_event_loop(
    mut rx: tokio::sync::mpsc::Receiver<ConfigEvent>,
    initial_config: AppConfig,
) -> (Arc<ConfigManager>, tokio::task::JoinHandle<()>) {
    let manager = Arc::new(ConfigManager::new(initial_config));
    let manager_clone = Arc::clone(&manager);

    let handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                ConfigEvent::Reloaded(new_config) => {
                    println!(
                        "  ✅ 配置已重载: {}:{} (timeout={}s)",
                        new_config.server.host,
                        new_config.server.port,
                        new_config.server.request_timeout_secs
                    );
                    // ConfigEvent::Reloaded 内部为 Box<AppConfig>，需解引用
                    manager_clone.update(*new_config).await;
                }
                ConfigEvent::Error(msg) => {
                    eprintln!("  ❌ 配置重载失败: {}", msg);
                }
            }
        }
    });

    (manager, handle)
}

// =============================================================================
// Main Entry Point
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔥 SDForge 配置热重载示例");
    println!("============================\n");

    // 1. ConfigManager 演示
    demo_config_manager().await;

    // 2. 创建临时配置文件并启动监听器
    println!("👀 创建配置监听器 (create_config_watcher)");
    let config_path = write_temp_config(INITIAL_CONFIG_TOML);
    println!("  配置文件: {}", config_path.display());

    let (watcher, rx) = create_config_watcher(config_path.to_str().unwrap())
        .await
        .expect("创建监听器失败");
    print_watcher_info(&watcher);

    // 3. 通过 watcher.get() 获取当前配置（从文件加载）
    let initial_config = watcher.get().await.expect("读取当前配置失败");
    println!(
        "  当前配置: {}:{} (timeout={}s)\n",
        initial_config.server.host,
        initial_config.server.port,
        initial_config.server.request_timeout_secs
    );

    // 4. 启动后台事件循环
    println!("🚀 启动后台事件循环 (tokio::spawn)");
    let (manager, _handle) = spawn_event_loop(rx, initial_config).await;

    // 5. 修改配置文件触发热重载
    println!("✏️  修改配置文件以触发热重载...");
    std::fs::write(&config_path, RELOADED_CONFIG_TOML)?;

    // 等待监听器检测变更（confers FsWatcher 防抖约 200ms，留足时间）
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // 6. 从 ConfigManager 读取热重载后的配置
    let final_config = manager.get().await;
    println!("\n📊 最终配置（经热重载更新）:");
    println!(
        "  host: {}, port: {}, timeout: {}s",
        final_config.server.host,
        final_config.server.port,
        final_config.server.request_timeout_secs
    );

    // 清理临时文件
    let _ = std::fs::remove_file(&config_path);

    println!("\n✓ 热重载示例完成");
    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 清理临时文件（best-effort）。
    fn cleanup(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
    }

    /// 验证 `ConfigManager::new`、`get`、`update` 的正确性。
    #[tokio::test]
    async fn test_config_manager_new_get_update() {
        let config = AppConfig::default();
        let manager = ConfigManager::new(config.clone());

        // get 返回初始配置
        let retrieved = manager.get().await;
        assert_eq!(retrieved.server.host, config.server.host);
        assert_eq!(retrieved.server.port, config.server.port);

        // update 后 get 返回新配置
        let mut new_config = AppConfig::default();
        new_config.server.host = "127.0.0.1".to_string();
        new_config.server.port = 3000;
        manager.update(new_config).await;

        let updated = manager.get().await;
        assert_eq!(updated.server.host, "127.0.0.1");
        assert_eq!(updated.server.port, 3000);
    }

    /// 验证 `ConfigEvent` 两个变体可被构造并正确匹配。
    #[test]
    fn test_config_event_variants_construct_and_match() {
        // Reloaded 变体
        let config = AppConfig::default();
        let reloaded = ConfigEvent::Reloaded(Box::new(config.clone()));
        match reloaded {
            ConfigEvent::Reloaded(c) => {
                assert_eq!(c.server.host, config.server.host);
                assert_eq!(c.server.port, config.server.port);
            }
            ConfigEvent::Error(_) => panic!("期望 Reloaded 变体"),
        }

        // Error 变体
        let error = ConfigEvent::Error("解析失败".to_string());
        match error {
            ConfigEvent::Reloaded(_) => panic!("期望 Error 变体"),
            ConfigEvent::Error(msg) => {
                assert_eq!(msg, "解析失败");
            }
        }
    }

    /// 验证 `ConfigWatcherImpl::new` 对有效文件可正常工作，
    /// 且 `path()` 与 `get()` 行为符合预期。
    #[tokio::test]
    async fn test_config_watcher_new_with_valid_file() {
        let path = write_temp_config(INITIAL_CONFIG_TOML);
        let (watcher, _rx) = ConfigWatcherImpl::new(path.clone())
            .await
            .expect("ConfigWatcherImpl::new 应当成功");

        // path() 返回传入的路径
        assert_eq!(watcher.path(), &path);

        // get() 能从文件加载并解析配置
        let config = watcher.get().await.expect("get() 应当解析有效配置");
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.request_timeout_secs, 30);

        cleanup(&path);
    }

    /// 验证 `create_config_watcher` 对不存在的路径返回错误
    /// （`ConfigError::FileNotFound`）。
    #[tokio::test]
    async fn test_create_config_watcher_nonexistent_path_returns_error() {
        let result = create_config_watcher("/nonexistent/sdforge/path.toml").await;
        assert!(result.is_err(), "不存在的路径应返回错误");
    }
}
