// Copyright (c) 2026 Kirky.X
//! Configuration hot reload support
//!
//! This module provides configuration hot-reload functionality using the Confers library.

use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::config::ConfigError;
use crate::AppConfig;

/// Configuration event type for hot reload
#[derive(Debug, Clone)]
pub enum ConfigEvent {
    /// Configuration was successfully reloaded
    ///
    /// Contains the new configuration that was loaded.
    Reloaded(Box<AppConfig>),
    /// An error occurred during reload
    ///
    /// Contains the error message describing what went wrong.
    Error(String),
}

/// File watcher for configuration files using Confers FsWatcher
pub struct ConfigWatcherImpl {
    path: PathBuf,
}

impl ConfigWatcherImpl {
    /// Create a new configuration watcher
    ///
    /// Returns a tuple of the watcher and a receiver for config events.
    pub async fn new(path: PathBuf) -> Result<(Self, mpsc::Receiver<ConfigEvent>), ConfigError> {
        let (tx, rx) = mpsc::channel(1);

        let mut watcher = confers::watcher::fs_watcher::FsWatcher::new(&path, 200)
            .await
            .map_err(|e| ConfigError::WatchError(e.to_string()))?;

        let path_clone = path.clone();
        tokio::spawn(async move {
            while let Some(_changed_path) = watcher.recv().await {
                if let Ok(content) = tokio::fs::read_to_string(&path_clone).await {
                    match toml::from_str::<AppConfig>(&content) {
                        Ok(config) => {
                            let _ = tx.send(ConfigEvent::Reloaded(Box::new(config))).await;
                        }
                        Err(e) => {
                            let _ = tx.send(ConfigEvent::Error(e.to_string())).await;
                        }
                    }
                }
            }
        });

        Ok((Self { path }, rx))
    }

    /// Get the watched configuration path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get the current configuration (loads from file)
    pub async fn get(&self) -> Result<AppConfig, ConfigError> {
        let content = tokio::fs::read_to_string(&self.path)
            .await
            .map_err(|e| ConfigError::LoadError(e.to_string()))?;
        toml::from_str(&content).map_err(|e| ConfigError::ParseError {
            message: e.to_string(),
        })
    }
}

/// Create a configuration watcher for the specified path
///
/// This is a wrapper that provides backward compatibility with the existing API.
pub async fn create_config_watcher(
    path: &str,
) -> Result<(ConfigWatcherImpl, mpsc::Receiver<ConfigEvent>), ConfigError> {
    if !PathBuf::from(path).exists() {
        return Err(ConfigError::FileNotFound {
            path: path.to_string(),
        });
    }

    ConfigWatcherImpl::new(path.into()).await
}

/// Configuration manager for hot reload
pub struct ConfigManager {
    config: tokio::sync::RwLock<AppConfig>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: tokio::sync::RwLock::new(config),
        }
    }

    /// Get current configuration
    pub async fn get(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    pub async fn update(&self, new_config: AppConfig) {
        *self.config.write().await = new_config;
    }
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Test ConfigEvent variants
    #[test]
    fn test_config_event_variants() {
        let config = AppConfig::default();
        let reloaded_event = ConfigEvent::Reloaded(Box::new(config.clone()));

        assert!(
            matches!(reloaded_event, ConfigEvent::Reloaded(_)),
            "Expected Reloaded event, got {:?}",
            reloaded_event
        );

        let error_event = ConfigEvent::Error("Test error message".to_string());

        assert!(
            matches!(error_event, ConfigEvent::Error(ref msg) if msg == "Test error message"),
            "Expected Error event with correct message, got {:?}",
            error_event
        );
    }

    /// Test ConfigManager creation and operations
    #[tokio::test]
    async fn test_config_manager_operations() {
        let config = AppConfig::default();
        let manager = ConfigManager::new(config.clone());

        let retrieved = manager.get().await;
        assert_eq!(retrieved.server.host, config.server.host);

        let mut new_config = AppConfig::default();
        new_config.server.host = "127.0.0.1".to_string();
        manager.update(new_config.clone()).await;

        let updated = manager.get().await;
        assert_eq!(updated.server.host, "127.0.0.1");
    }

    /// Test ConfigManager with RwLock operations
    #[tokio::test]
    async fn test_config_manager_rwlock_operations() {
        let config = AppConfig::default();
        let manager = ConfigManager::new(config);

        let _retrieved = manager.get().await;

        let mut new_config = AppConfig::default();
        new_config.server.host = "127.0.0.1".to_string();
        manager.update(new_config.clone()).await;

        let updated = manager.get().await;
        assert_eq!(updated.server.host, "127.0.0.1");
    }

    /// Test create_config_watcher with non-existent path
    #[tokio::test]
    async fn test_create_config_watcher_nonexistent() {
        let result = create_config_watcher("/nonexistent/path.toml").await;
        assert!(result.is_err());
    }

    /// Test create_config_watcher with valid path
    #[tokio::test]
    async fn test_create_config_watcher_valid() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
            [server]
            host = "localhost"
            port = 8080
            request_timeout_secs = 30

            [authentication]
            type = "none"

            [logging]
            level = "info"
            format = "json"
        "#;
        std::fs::write(&config_path, config_content).unwrap();

        let result = create_config_watcher(config_path.to_str().unwrap()).await;
        assert!(result.is_ok());
    }
}
